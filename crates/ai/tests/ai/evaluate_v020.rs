//! What `evaluate` sees of patch v0.2.0's mechanics (SPEC §9.9 "Evaluation"; docs/classic-sets.md B3.1,
//! B3.3, B3.4, B5 E6, E35). Each term reads the engine's own helper — `active_brittle_count`,
//! `animated_kind_of`, `cannot_attack`, `own_cost` and the layers' `unit_view` — and is a named weight in
//! AI_EVAL that GREEDY_EVAL, frozen with the gates, sets to nothing. As in evaluate.rs, nothing
//! here pins a weight's value: the tests pin orderings, and where a term is exactly one weight, that
//! the weight is all it adds.
//!
//! Port of `packages/ai/test/evaluate-v020.test.ts`. TS registered the test-only card for the whole
//! file at import; the registries' override is per thread here, so every test installs it first
//! (`install_idle`). A TS helper that changed a live card (`giveBrittleCount`, `tuningOf`) changes the
//! state's copy of it here, found by id.

use jackioh_ai::{AI_EVAL, EvalWeights, GREEDY_EVAL, NextSwing, evaluate, face_threat, unit_worth};
use jackioh_engine::testkit::{
    AnimateOptions, CardDef, CardInstance, CardScripts, EngineSink, GameEvent, GameState, Keyword, KeywordKind,
    PlayerId, Position, Script, StaticFlags, Value, active_units_of, animate_card, cannot_attack, create_rng,
    find_instance_mut, give_brittle_count, json, json_as, register_catalog_as, register_scripts, tuning_of,
    unit_view,
};

use super::support::{AI, HUMAN, clone, register_cards, scenario};

const TESLA: &str = "classic-005"; // Field Trap (2), Animated, its unit face 1/4 Lifesteal
const FROSTSPATULA: &str = "classicplus-012-8"; // Field Spell token (2), Animated on your turn, 10/3 Rush
const SAME_COST_TRAP: &str = "core-085"; // Unlicensed Experimentation: a Trap (2) with no unit face
const SOLARIUS: &str = "classicplus-038"; // Unit 3/2, Spell Damage +2 (Radiant +5)
const TOP_LOSER: &str = "classicplus-019-1"; // Unit token; only its Radiant face prints Immune to Spells
const VANILLA: &str = "core-008"; // Mr. Vanilla, 4/4, no keywords

/// A test-only 4/4 whose text bars it from attacking: E35's static flag, no keyword.
const IDLE: &str = "ai-test-idle";

fn idle_def() -> CardDef {
    json_as(json!({
        "id": IDLE,
        "index": "ai-test-2",
        "name": "Idle Wall (AI test)",
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Token",
        "token": true,
        "cost": 1,
        "base": { "attack": 4, "health": 4, "keywords": [], "text": "Can't attack or be attacked." },
        "radiant": { "attack": 8, "health": 8, "keywords": [], "text": "Can't attack or be attacked." },
    }))
}

fn idle() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            cant_attack_or_be_attacked: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

/// A token, so no deck, pool or determinization ever deals it; registered for this file only.
fn install_idle() {
    register_cards();
    let mut catalog = (*jackioh_cards::CATALOG).clone();
    catalog.insert(IDLE.to_string(), idle_def());
    register_catalog_as(catalog, jackioh_cards::catalog_version());
    let mut scripts = jackioh_cards::scripts_of();
    scripts.insert(
        IDLE.to_string(),
        CardScripts {
            base: idle(),
            radiant: idle(),
        },
    );
    register_scripts(scripts);
}

fn board(opts: Value) -> GameState {
    install_idle();
    let mut options = json!({ "seed": "evaluate-v020" });
    if let (Some(base), Value::Object(extra)) = (options.as_object_mut(), opts) {
        for (key, value) in extra {
            base.insert(key, value);
        }
    }
    scenario(options).state().clone()
}

fn units_of(state: &GameState, player: PlayerId) -> Vec<CardInstance> {
    active_units_of(state, player).into_iter().cloned().collect()
}

fn first_unit(state: &GameState, player: PlayerId) -> CardInstance {
    units_of(state, player).into_iter().next().expect("setup: nothing there")
}

fn first_backrow(state: &GameState) -> CardInstance {
    state.players.p1.backrow.iter().flatten().next().cloned().expect("setup: nothing there")
}

fn first_hand(state: &GameState) -> CardInstance {
    state.players.p1.hand.first().cloned().expect("setup: nothing there")
}

/// A clone of `state` with `change` applied to it.
fn changed(state: &GameState, change: impl FnOnce(&mut GameState)) -> GameState {
    let mut out = clone(state);
    change(&mut out);
    out
}

/// The state's copy of `card`, to change in place.
fn live<'a>(state: &'a mut GameState, card: &CardInstance) -> &'a mut CardInstance {
    find_instance_mut(state, &card.id).expect("the card is in the state")
}

/// TS `giveBrittleCount(out, card, count)` on the state's copy of the card.
fn give_brittle(out: &mut GameState, card: &CardInstance, count: i32) {
    let mut held = card.clone();
    give_brittle_count(out, &mut held, count);
    *live(out, card) = held;
}

fn eval(state: &GameState, seat: PlayerId) -> f64 {
    evaluate(state, seat, NextSwing::Enemy, &AI_EVAL)
}

fn worth(state: &GameState, unit: &CardInstance) -> f64 {
    unit_worth(state, unit, &AI_EVAL)
}

/// Jest's `toBeCloseTo(expected, digits)`: within half a unit of the last digit.
fn assert_close(received: f64, expected: f64, digits: i32, label: &str) {
    assert!(
        (expected - received).abs() < 10f64.powi(-digits) / 2.0,
        "{label}: {received} is not close to {expected}"
    );
}

mod evaluate_patch_v0_2_0s_mechanics {
    use super::*;

    #[test]
    fn b3_3_a_brittle_card_is_worth_less_the_nearer_it_is_to_crumbling_on_the_field_in_the_backrow_and_in_hand() {
        let base = board(json!({
            "p1": { "field": [VANILLA], "backrow": ["core-006"], "hand": ["core-053"] },
            "p2": { "hand": ["core-005"] },
        }));
        type Pick = fn(&GameState) -> CardInstance;
        let picks: [(&str, Pick); 3] = [
            ("unit", |state| first_unit(state, AI)),
            ("backrow", first_backrow),
            ("hand", first_hand),
        ];
        for (place, pick) in picks {
            let brittle = |count: i32| {
                changed(&base, |out| {
                    let card = pick(out);
                    give_brittle(out, &card, count);
                })
            };
            let one = eval(&brittle(1), AI);
            let three = eval(&brittle(3), AI);
            assert!(one < three, "{place}");
            assert!(three < eval(&base, AI), "{place}");
        }
        // On the field the count is the unit's own: unit_worth carries it.
        let unit = |count: i32| {
            let out = changed(&base, |state| {
                let card = first_unit(state, AI);
                give_brittle(state, &card, count);
            });
            worth(&out, &first_unit(&out, AI))
        };
        assert!(unit(1) < unit(3));
        assert!(unit(3) < worth(&base, &first_unit(&base, AI)));
    }

    #[test]
    fn b3_1_a_readable_animated_backrow_card_is_partly_a_unit_already_worth_animated_share_of_its_unit_face() {
        let mut no_share: EvalWeights = AI_EVAL;
        no_share.animated_share = 0.0;
        const { assert!(AI_EVAL.animated_share > 0.0) };
        for def_id in [TESLA, FROSTSPATULA] {
            let state = board(json!({ "p1": { "backrow": [def_id], "hand": ["core-053"] }, "p2": { "hand": ["core-005"] } }));
            let card = first_backrow(&state);
            let gain = eval(&state, AI) - evaluate(&state, AI, NextSwing::Enemy, &no_share);
            assert_close(gain, AI_EVAL.animated_share * worth(&state, &card), 10, def_id);
            assert!(gain > 0.0, "{def_id}");
        }
        // Tesla against a Trap of the same cost with no unit face: the same backrow value, plus the share.
        let tesla = board(json!({ "p1": { "backrow": [TESLA], "hand": ["core-053"] }, "p2": { "hand": ["core-005"] } }));
        let trap = board(json!({ "p1": { "backrow": [SAME_COST_TRAP], "hand": ["core-053"] }, "p2": { "hand": ["core-005"] } }));
        assert_close(
            evaluate(&tesla, AI, NextSwing::Enemy, &no_share),
            evaluate(&trap, AI, NextSwing::Enemy, &no_share),
            10,
            "no share",
        );
        assert!(eval(&tesla, AI) > eval(&trap, AI));
    }

    #[test]
    fn b3_1_an_animated_card_standing_in_a_unit_zone_is_a_unit_counted_once() {
        let state = changed(
            &board(json!({ "p1": { "backrow": [TESLA], "hand": ["core-053"] }, "p2": { "hand": ["core-005"] } })),
            |out| {
                let tesla = first_backrow(out);
                let mut events: Vec<GameEvent> = Vec::new();
                let mut rng = create_rng(&out.seed, out.rng_cursor);
                let mut sink = EngineSink::new(out, &mut events, &mut rng);
                assert!(animate_card(
                    &mut sink,
                    &tesla,
                    AnimateOptions {
                        position: Some(Position::Def)
                    }
                ));
            },
        );
        let tesla = first_unit(&state, AI);
        assert_eq!(tesla.def_id, TESLA);
        assert!(state.players.p1.backrow.iter().all(Option::is_none));
        // The backrow term is gone with it: the share no longer moves the score.
        let mut no_share: EvalWeights = AI_EVAL;
        no_share.animated_share = 0.0;
        assert_eq!(eval(&state, AI), evaluate(&state, AI, NextSwing::Enemy, &no_share));
    }

    #[test]
    fn e6_spell_damage_is_worth_ai_eval_spell_damage_per_point_radiants_5_too() {
        let state = board(json!({ "p1": { "field": [SOLARIUS, { "def": SOLARIUS, "radiant": true }] } }));
        let units = units_of(&state, AI);
        let (Some(base), Some(radiant)) = (units.first(), units.get(1)) else {
            panic!("setup");
        };
        let mut none: EvalWeights = AI_EVAL;
        none.spell_damage = 0.0;
        const { assert!(AI_EVAL.spell_damage > 0.0) };
        assert_close(
            worth(&state, base) - unit_worth(&state, base, &none),
            2.0 * AI_EVAL.spell_damage,
            10,
            "base",
        );
        assert_close(
            worth(&state, radiant) - unit_worth(&state, radiant, &none),
            5.0 * AI_EVAL.spell_damage,
            10,
            "radiant",
        );
    }

    #[test]
    fn e35_immune_to_spells_is_worth_its_keyword_weight() {
        let state = board(json!({ "p1": { "field": [{ "def": TOP_LOSER, "radiant": true }, TOP_LOSER] } }));
        let units = units_of(&state, AI);
        let (Some(radiant), Some(base)) = (units.first(), units.get(1)) else {
            panic!("setup");
        };
        let weight = AI_EVAL.keyword.immune_to_spells;
        let mut none: EvalWeights = AI_EVAL;
        none.keyword.immune_to_spells = 0.0;
        assert!(weight > 0.0);
        assert_close(worth(&state, radiant) - unit_worth(&state, radiant, &none), weight, 10, "radiant");
        // The base face prints no immunity.
        assert_eq!(worth(&state, base), unit_worth(&state, base, &none));
    }

    #[test]
    fn e35_a_unit_a_status_bars_from_attacking_counts_no_attack_and_threatens_nothing_exactly_as_cant_attack() {
        let barred = board(json!({ "p1": { "hand": ["core-053"] }, "p2": { "field": [IDLE] } }));
        let plain = board(json!({ "p1": { "hand": ["core-053"] }, "p2": { "field": [VANILLA] } }));
        let idle_unit = first_unit(&barred, HUMAN);
        let vanilla = first_unit(&plain, HUMAN);
        assert!(cannot_attack(&barred, &idle_unit));
        assert_eq!(unit_view(&barred, &idle_unit).keywords, Vec::<Keyword>::new());

        assert_eq!(face_threat(&plain, HUMAN), 4);
        assert_eq!(face_threat(&barred, HUMAN), 0);
        assert_close(
            worth(&barred, &idle_unit),
            worth(&plain, &vanilla) - AI_EVAL.attack * 4.0,
            10,
            "barred",
        );

        // The same 4/4 with the "Can't attack" keyword instead: the same worth, the same threat.
        let keyworded = changed(&plain, |out| {
            let unit = first_unit(out, HUMAN);
            live(out, &unit).granted_keywords.push(Keyword::CantAttack);
        });
        assert_close(
            worth(&keyworded, &first_unit(&keyworded, HUMAN)),
            worth(&barred, &idle_unit),
            10,
            "keyworded",
        );
        assert_eq!(face_threat(&keyworded, HUMAN), 0);
    }

    #[test]
    fn b3_4_a_degrade_or_upgrade_on_the_field_reaches_the_evaluation_through_the_layers() {
        // Mr. Vanilla 4/4 and Midrange Menace 9/9 Taunt on p1's field.
        let base = board(json!({ "p1": { "field": [VANILLA, "core-019"] }, "p2": { "hand": ["core-005"] } }));
        let tune = |change: &dyn Fn(&mut GameState, &CardInstance, &CardInstance)| {
            changed(&base, |out| {
                let units = units_of(out, AI);
                let (Some(vanilla), Some(menace)) = (units.first(), units.get(1)) else {
                    panic!("setup");
                };
                change(out, vanilla, menace);
            })
        };
        let degraded = tune(&|out, vanilla, _| {
            let tuning = tuning_of(live(out, vanilla));
            tuning.attack = Some(-1);
            tuning.health = Some(-3);
        });
        let upgraded = tune(&|out, vanilla, _| {
            let tuning = tuning_of(live(out, vanilla));
            tuning.attack = Some(2);
            tuning.health = Some(2);
        });
        let stripped = tune(&|out, _, menace| {
            tuning_of(live(out, menace)).remove_keywords = Some(vec![KeywordKind::Taunt]);
        });
        let shielded = tune(&|out, vanilla, _| {
            tuning_of(live(out, vanilla)).add_keywords = Some(vec![Keyword::DivineShield]);
        });

        assert_eq!(unit_view(&degraded, &first_unit(&degraded, AI)).attack, 3);
        let score = eval(&base, AI);
        assert!(eval(&degraded, AI) < score);
        assert!(eval(&stripped, AI) < score);
        assert!(eval(&upgraded, AI) > score);
        assert!(eval(&shielded, AI) > score);

        // A numbered keyword's step (B3.4 rule 3's X row) moves Spell Damage's worth by one point.
        let solarius = board(json!({ "p1": { "field": [SOLARIUS] } }));
        let stepped = changed(&solarius, |out| {
            let unit = first_unit(out, AI);
            tuning_of(live(out, &unit)).x = Some([("Spell Damage".to_string(), 1)].into_iter().collect());
        });
        assert_close(
            worth(&stepped, &first_unit(&stepped, AI)) - worth(&solarius, &first_unit(&solarius, AI)),
            AI_EVAL.spell_damage,
            10,
            "stepped",
        );
    }

    #[test]
    fn b3_4_r65_an_own_hand_card_made_dearer_scores_lower_and_one_made_cheaper_higher_x_cost_and_enemy_cards_do_not_move()
    {
        let hand = |cost_mod: i32, extra: Value| {
            let mut p2 = json!({ "hand": ["core-005"] });
            if let (Some(base), Value::Object(more)) = (p2.as_object_mut(), extra) {
                for (key, value) in more {
                    base.insert(key, value);
                }
            }
            board(json!({ "p1": { "hand": [{ "def": "core-053", "costMod": cost_mod }] }, "p2": p2 }))
        };
        let plain = eval(&hand(0, json!({})), AI);
        const { assert!(AI_EVAL.hand_cost_delta > 0.0) };
        assert_close(plain - eval(&hand(1, json!({})), AI), AI_EVAL.hand_cost_delta, 10, "dearer");
        assert_close(eval(&hand(-1, json!({})), AI) - plain, AI_EVAL.hand_cost_delta, 10, "cheaper");

        // A price floors at 0: Mr. Vanilla (1) two crystals cheaper is one crystal cheaper.
        let vanilla = |cost_mod: i32| {
            eval(
                &board(json!({ "p1": { "hand": [{ "def": VANILLA, "costMod": cost_mod }] }, "p2": { "hand": ["core-005"] } })),
                AI,
            )
        };
        assert_close(vanilla(-2), vanilla(-1), 10, "floored");
        // R65: no modifier reaches an X-cost card, so its costMod is no change.
        let dividend = |cost_mod: i32| {
            eval(
                &board(json!({ "p1": { "hand": [{ "def": "core-024", "costMod": cost_mod }] }, "p2": { "hand": ["core-005"] } })),
                AI,
            )
        };
        assert_eq!(dividend(1), dividend(0));
        // The opponent's hand is placeholders to the AI: its cards' costs are no term.
        let enemy = |cost_mod: i32| {
            eval(
                &board(json!({ "p1": { "hand": ["core-053"] }, "p2": { "hand": [{ "def": "core-005", "costMod": cost_mod }] } })),
                AI,
            )
        };
        assert_eq!(enemy(1), enemy(0));
    }

    #[test]
    fn greedy_eval_gives_every_v0_2_0_term_nothing_so_the_frozen_baseline_scores_these_boards_as_before() {
        let greedy = |state: &GameState| evaluate(state, AI, NextSwing::Enemy, &GREEDY_EVAL);
        let field = board(json!({
            "p1": { "field": [VANILLA, SOLARIUS, { "def": TOP_LOSER, "radiant": true }], "hand": ["core-053"] },
        }));
        let plain_field = changed(&field, |out| {
            let units = units_of(out, AI);
            let (Some(solarius), Some(top_loser)) = (units.get(1), units.get(2)) else {
                panic!("setup");
            };
            tuning_of(live(out, solarius)).remove_keywords = Some(vec![KeywordKind::SpellDamage]);
            tuning_of(live(out, top_loser)).remove_keywords = Some(vec![KeywordKind::ImmuneToSpells]);
        });
        assert_eq!(greedy(&field), greedy(&plain_field));
        assert!(eval(&field, AI) > eval(&plain_field, AI));

        let brittle = changed(&field, |out| {
            let unit = first_unit(out, AI);
            give_brittle(out, &unit, 1);
            let card = first_hand(out);
            give_brittle(out, &card, 1);
        });
        assert_eq!(greedy(&brittle), greedy(&field));

        let dearer = changed(&field, |out| {
            let card = first_hand(out);
            live(out, &card).cost_mod = 1;
        });
        assert_eq!(greedy(&dearer), greedy(&field));

        let tesla = board(json!({ "p1": { "backrow": [TESLA], "hand": ["core-053"] }, "p2": { "hand": ["core-005"] } }));
        let trap = board(json!({ "p1": { "backrow": [SAME_COST_TRAP], "hand": ["core-053"] }, "p2": { "hand": ["core-005"] } }));
        assert_eq!(greedy(&tesla), greedy(&trap));
    }
}
