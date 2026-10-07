//! Call to Chaos (BUILD M3-T7): SPEC §8 #95, §7's tokens and R28's capped recursion. The card file
//! arrives in M4; these tests pin the machinery it will call — each of the ten effects, the base and
//! radiant rolls, the chain cap, and what the ten do against a full board, a full hand and an empty
//! library.
//!
//! Port of `packages/engine/test/callToChaos.test.ts`.

use jackioh_engine::subsystems::call_to_chaos::{
    CHAOS_CHAIN_KEY, CHAOS_EFFECTS, CHAOS_RECURSION, CallToChaosArgs, ChaosEffectDef, add_random_zero_cost_cards, call_to_chaos,
    cast_random_call_to_chaos, chaos_chain_cap_reached, chaos_chain_of, discount_hand_and_library,
    draw_library_and_gain_mana, heal_hero_thirty, make_hand_radiant, roll_chaos_effects, summon_chaos_golem,
    summon_random_backrow, summon_random_three_cost_units, summon_rush_tokens,
};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, put, set_library, slot};

/// TS's `def(name, type, extra)`, its running index written out (TS counted from 900 in file order).
fn def(name: &str, index: i32, type_: &str, extra: Value) -> CardDef {
    let mut out = json!({
        "id": format!("cc-{name}"),
        "index": index.to_string(),
        "name": name,
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    });
    if let (Some(into), Some(from)) = (out.as_object_mut(), extra.as_object()) {
        for (key, value) in from {
            into.insert(key.clone(), value.clone());
        }
    }
    json_as(out)
}

/// #95 itself: the Spell the recursion's pool is drawn from (§8, §5).
fn chaos() -> CardDef {
    def(
        "chaos",
        901,
        "Spell",
        json!({
            "index": "95",
            "name": "Call to Chaos (fixture)",
            "tags": ["Call to Chaos"],
            "rarity": "Legendary",
            "cost": 4,
        }),
    )
}

/// §7: the 10/10 token index 95.1.
fn golem() -> CardDef {
    let face = json!({
        "attack": 10,
        "health": 10,
        "keywords": [{ "kind": "Rush" }, { "kind": "Lifesteal" }, { "kind": "Divine Shield" }, { "kind": "First Strike" }],
        "text": "Chaos Golem",
    });
    def(
        "golem",
        902,
        "Unit",
        json!({
            "index": "95.1",
            "name": "Chaos Golem (fixture)",
            "tags": ["Token"],
            "rarity": "Token",
            "token": true,
            "cost": 4,
            "base": face,
            "radiant": face,
        }),
    )
}

/// Three 3-cost Units and one 5-cost Unit, so the cost filter of effect 1 has something to refuse.
fn three_a() -> CardDef {
    def(
        "three-a",
        903,
        "Unit",
        json!({ "cost": 3, "base": { "attack": 3, "health": 3, "keywords": [], "text": "3a" }, "radiant": { "attack": 6, "health": 6, "keywords": [], "text": "3a+" } }),
    )
}
fn three_b() -> CardDef {
    def(
        "three-b",
        904,
        "Unit",
        json!({ "cost": 3, "base": { "attack": 2, "health": 4, "keywords": [], "text": "3b" }, "radiant": { "attack": 4, "health": 8, "keywords": [], "text": "3b+" } }),
    )
}
fn three_c() -> CardDef {
    def(
        "three-c",
        905,
        "Unit",
        json!({ "cost": 3, "base": { "attack": 4, "health": 2, "keywords": [], "text": "3c" }, "radiant": { "attack": 8, "health": 4, "keywords": [], "text": "3c+" } }),
    )
}
fn five_cost() -> CardDef {
    def(
        "five",
        906,
        "Unit",
        json!({ "cost": 5, "base": { "attack": 5, "health": 5, "keywords": [], "text": "5" }, "radiant": { "attack": 5, "health": 5, "keywords": [], "text": "5" } }),
    )
}

/// The backrow pool of effect 9: a Trap, a Field Trap and a Field Spell (§3.2, R33).
fn trap() -> CardDef {
    def("trap", 907, "Trap", json!({ "cost": 2 }))
}
fn field_trap() -> CardDef {
    def("field-trap", 908, "Field Trap", json!({ "cost": 2 }))
}
fn field_spell() -> CardDef {
    def("field-spell", 909, "Field Spell", json!({ "cost": 2 }))
}

fn defs() -> Vec<CardDef> {
    vec![
        chaos(),
        golem(),
        three_a(),
        three_b(),
        three_c(),
        five_cost(),
        trap(),
        field_trap(),
        field_spell(),
    ]
}

/// `chaos_cry`: what the cast copy of #95 does when it resolves; the default is the real card (§8).
fn game(seed: &str, chaos_cry: Option<Hook>) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);

    let cry: Hook = chaos_cry.unwrap_or_else(|| hook(|_ctx| vec![call_to_chaos(CallToChaosArgs::default())]));
    let scripts = CardScripts {
        base: Script {
            cry: Some(cry.clone()),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(cry),
            ..Script::default()
        },
    };
    let mut registered = registered_scripts().clone();
    registered.insert(chaos().id, scripts);
    register_scripts(registered);

    state.turn = 4;
    state.active = P1;
    state.phase = Phase::Main;
    // Every test sets the library it needs; the dealt deck would only add noise to the draw effects.
    set_library(&mut state, P1, &[] as &[&str]);
    state
}

/// A #95 instance mid-resolution, which is what `self` is while its spell script runs (§10.5).
fn chaos_card(state: &mut GameState, radiant: bool, chain: Option<i32>) -> CardInstance {
    let mut card = new_instance(&mut *state, &chaos().id, P1, Zone::Resolving { player: P1 });
    if radiant {
        card.radiant = true;
    }
    if let Some(chain) = chain {
        card.memory.insert(CHAOS_CHAIN_KEY.to_string(), json!(chain));
    }
    card
}

fn run(sink: &mut EngineSink<'_>, effect: Effect, self_: Option<&CardInstance>, controller: PlayerId) {
    let options = HookOptions {
        controller: Some(controller),
        ..HookOptions::default()
    };
    let mut ctx = make_context(sink, self_, options);
    apply_effects(&[effect], &mut ctx);
}

fn summoned_defs(state: &GameState, events: &[GameEvent]) -> Vec<CardDef> {
    events_of_type(events, GameEventType::Summoned)
        .iter()
        .map(|event| {
            let def_id = json_of(event)["defId"].as_str().unwrap_or_default().to_string();
            def_of(Some(state), &def_id).clone()
        })
        .collect()
}

fn json_of<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).unwrap()
}

/// One field of every event of a type, as JSON.
fn field_of(events: &[GameEvent], kind: GameEventType, field: &str) -> Vec<Value> {
    events_of_type(events, kind)
        .iter()
        .map(|event| json_of(event)[field].clone())
        .collect()
}

/// TS `Array.prototype.indexOf`: -1 when absent.
fn index_of(types: &[GameEventType], kind: GameEventType) -> i64 {
    types
        .iter()
        .position(|each| *each == kind)
        .map(|at| at as i64)
        .unwrap_or(-1)
}

fn sink_events(state: &mut GameState, events: &mut Vec<GameEvent>, f: impl FnOnce(&mut EngineSink<'_>)) {
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut sink = EngineSink::new(state, events, &mut rng);
    f(&mut sink);
}

fn names_of<D: std::borrow::Borrow<ChaosEffectDef>>(effects: &[D]) -> Vec<String> {
    effects.iter().map(|effect| effect.borrow().name.to_string()).collect()
}

fn labels_of<D: std::borrow::Borrow<ChaosEffectDef>>(effects: &[D]) -> Vec<String> {
    effects.iter().map(|effect| effect.borrow().label.to_string()).collect()
}


mod r28_call_to_chaos_s8_95_m3_t7 {
    use super::*;

    #[test]
    fn r60_r64_s8_95_summons_3_random_3_cost_units_into_the_leftmost_free_zones() {
        let mut state = game("chaos-units", None);
        let mut events: Vec<GameEvent> = Vec::new();
        sink_events(&mut state, &mut events, |sink| {
            run(sink, summon_random_three_cost_units(), None, P1);
        });

        let summoned = events_of_type(&events, GameEventType::Summoned);
        assert_eq!(summoned.len(), 3);
        // R64: no zone is named, so they take the leftmost empty unlocked zones in order.
        assert_eq!(field_of(&events, GameEventType::Summoned, "lane"), vec![json!(1), json!(2), json!(3)]);
        assert!(field_of(&events, GameEventType::Summoned, "row").iter().all(|row| *row == json!("units")));
        for card_def in summoned_defs(&state, &events) {
            assert_eq!(card_def.type_, CardType::Unit);
            assert_eq!(query_cost(&card_def), 3);
            assert!(!card_def.token);
        }
    }

    #[test]
    fn r60_lets_the_three_summoned_units_repeat_since_they_are_generated_from_the_catalog() {
        // Over many seeds the three picks are independent draws, so a repeat must be reachable.
        let repeats: Vec<bool> = ["r1", "r2", "r3", "r4", "r5", "r6", "r7", "r8"]
            .iter()
            .map(|seed| {
                let mut state = game(&format!("chaos-repeat-{seed}"), None);
                let mut events: Vec<GameEvent> = Vec::new();
                sink_events(&mut state, &mut events, |sink| {
                    run(sink, summon_random_three_cost_units(), None, P1);
                });
                let ids = field_of(&events, GameEventType::Summoned, "defId");
                let distinct: IndexSet<String> = ids.iter().map(|id| id.to_string()).collect();
                distinct.len() < ids.len()
            })
            .collect();
        assert!(repeats.contains(&true));
    }

    #[test]
    fn s6_3_heal_heals_your_hero_30_with_no_cap_on_a_heros_health() {
        let mut state = game("chaos-heal", None);
        state.players.p1.hero.health = 12;
        sink_events(&mut state, &mut Vec::new(), |sink| {
            run(sink, heal_hero_thirty(), None, P1);
        });
        assert_eq!(state.players.p1.hero.health, 42);
        assert_eq!(state.players.p2.hero.health, 30); // "your hero" only
    }

    #[test]
    fn r58_draws_the_library_it_had_when_the_effect_started_and_gains_4_mana() {
        let mut state = game("chaos-draw", None);
        set_library(&mut state, P1, &["fx-1", "fx-2", "fx-3", "fx-4"]);
        sink_events(&mut state, &mut Vec::new(), |sink| {
            run(sink, draw_library_and_gain_mana(), None, P1);
        });

        assert!(state.players.p1.library.is_empty());
        assert_eq!(state.players.p1.hand.len(), 4);
        assert_eq!(state.players.p1.mana.current, 4);
        assert_eq!(state.players.p1.fatigue_count, 0);
    }

    #[test]
    fn r60_r65_adds_3_random_non_token_cards_to_hand_each_costing_0() {
        let mut state = game("chaos-add", None);
        sink_events(&mut state, &mut Vec::new(), |sink| {
            run(sink, add_random_zero_cost_cards(), None, P1);
        });

        let hand = &state.players.p1.hand;
        assert_eq!(hand.len(), 3);
        for card in hand {
            // §5.1: `query` keeps tokens out of a pool that does not ask for them.
            assert!(!def_of(Some(&state), &card.def_id).token);
            assert_eq!(card.cost_override, Some(0));
            assert_eq!(effective_cost(&state, card, CostOptions::default()), 0);
        }
    }

    #[test]
    fn s5_2_makes_every_card_in_your_hand_radiant_and_leaves_an_already_radiant_card_alone() {
        let mut state = game("chaos-radiant", None);
        let mut events: Vec<GameEvent> = Vec::new();
        let hand = in_hand(&mut state, "fx-1", P1, 3);
        let (first, second, third) = (hand[0].clone(), hand[1].clone(), hand[2].clone());
        find_instance_mut(&mut state, &third.id).expect("in hand").radiant = true;
        let enemy = in_hand(&mut state, "fx-1", P2, 1).remove(0);

        sink_events(&mut state, &mut events, |sink| {
            run(sink, make_hand_radiant(), None, P1);
        });

        assert!(state.players.p1.hand.iter().all(|card| card.radiant));
        // §6.3: the flag is set once and never unset; the cue goes out for all three, because a hand
        // card is hidden from the opponent and a cue only for the changed ones would give away its face
        // (R177, R97).
        assert_eq!(
            field_of(&events, GameEventType::RadiantSet, "instanceId"),
            vec![json!(first.id), json!(second.id), json!(third.id)]
        );
        assert!(!find_instance(&state, &enemy.id).expect("in hand").radiant);
    }

    #[test]
    fn s7_summons_five_radiant_rush_tokens_whose_6_6_comes_from_the_catalog() {
        let mut state = game("chaos-tokens", None);
        let mut events: Vec<GameEvent> = Vec::new();
        sink_events(&mut state, &mut events, |sink| {
            run(sink, summon_rush_tokens(), None, P1);
        });

        let summoned = events_of_type(&events, GameEventType::Summoned);
        assert_eq!(summoned.len(), 5);
        assert_eq!(
            field_of(&events, GameEventType::Summoned, "lane"),
            vec![json!(1), json!(2), json!(3), json!(4), json!(5)]
        );
        for pile in &state.players.p1.units {
            let token = pile.as_ref().and_then(|pile| pile.first());
            assert!(token.is_some());
            let Some(token) = token else { continue };
            assert_eq!(def_of(Some(&state), &token.def_id).index, "T-rush");
            // No `statsOverride`: #95 was the only card that invented a Rush Token size, and it now
            // summons the token's own Radiant face instead, so the 6/6 is the catalog's.
            assert!(token.radiant);
            assert_eq!(token.stats_override, None);
            let view = unit_view(&state, token);
            assert_eq!((view.attack, view.max_health), (6, 6));
        }
    }

    #[test]
    fn r78_s6_3_cost_every_card_in_your_hand_and_library_costs_2_less_floored_at_0() {
        let mut state = game("chaos-discount", None);
        let cheap = in_hand(&mut state, "fx-1", P1, 1).remove(0); // printed cost 1
        let pricey = in_hand(&mut state, &chaos().id, P1, 1).remove(0); // printed cost 4
        let in_library = set_library(&mut state, P1, &[chaos().id.as_str()]).remove(0);
        let enemy = in_hand(&mut state, &chaos().id, P2, 1).remove(0);

        sink_events(&mut state, &mut Vec::new(), |sink| {
            run(sink, discount_hand_and_library(), None, P1);
        });

        let now = |id: &str| find_instance(&state, id).expect("still there").clone();
        assert_eq!(now(&cheap.id).cost_mod, -2);
        assert_eq!(now(&pricey.id).cost_mod, -2);
        assert_eq!(now(&in_library.id).cost_mod, -2);
        assert_eq!(now(&enemy.id).cost_mod, 0); // "your" hand and library only
        // R65 floors the result at 0, and the discount travels with the card between zones (R78).
        assert_eq!(effective_cost(&state, &now(&cheap.id), CostOptions::default()), 0);
        assert_eq!(effective_cost(&state, &now(&pricey.id), CostOptions::default()), 2);
        assert_eq!(effective_cost(&state, &now(&in_library.id), CostOptions::default()), 2);
    }

    #[test]
    fn s7_summons_a_chaos_golem_the_10_10_token_of_index_95_1() {
        let mut state = game("chaos-golem", None);
        let mut events: Vec<GameEvent> = Vec::new();
        sink_events(&mut state, &mut events, |sink| {
            run(sink, summon_chaos_golem(), None, P1);
        });

        assert_eq!(events_of_type(&events, GameEventType::Summoned).len(), 1);
        let card = state.players.p1.units[0].as_ref().and_then(|pile| pile.first());
        assert!(card.is_some());
        let Some(card) = card else { return };
        assert_eq!(def_of(Some(&state), &card.def_id).index, "95.1");
        let view = unit_view(&state, card);
        assert_eq!((view.attack, view.max_health), (10, 10));
    }

    #[test]
    fn r33_summons_5_random_field_spells_or_traps_into_your_backrow_traps_face_down() {
        let mut seen: IndexSet<CardType> = IndexSet::new();

        for seed in ["b1", "b2", "b3", "b4", "b5", "b6"] {
            let mut state = game(&format!("chaos-backrow-{seed}"), None);
            let mut events: Vec<GameEvent> = Vec::new();
            sink_events(&mut state, &mut events, |sink| {
                run(sink, summon_random_backrow(), None, P1);
            });

            assert_eq!(events_of_type(&events, GameEventType::Summoned).len(), 5);
            assert!(field_of(&events, GameEventType::Summoned, "row").iter().all(|row| *row == json!("backrow")));
            assert_eq!(
                field_of(&events, GameEventType::Summoned, "lane"),
                vec![json!(1), json!(2), json!(3), json!(4), json!(5)]
            );

            for card in &state.players.p1.backrow {
                assert!(card.is_some());
                let Some(card) = card else { continue };
                let card_def = def_of(Some(&state), &card.def_id);
                seen.insert(card_def.type_);
                assert!([CardType::FieldSpell, CardType::Trap, CardType::FieldTrap].contains(&card_def.type_));
                // §3.2 and R33: a Field Spell is public, a Trap or Field Trap stays face-down.
                let expected = if card_def.type_ == CardType::FieldSpell { Some(true) } else { None };
                assert_eq!(card.face_up, expected);
            }
        }

        // The pool really does include all three types, Field Traps among them (§8 #95).
        assert!(seen.contains(&CardType::FieldSpell));
        assert!(seen.contains(&CardType::Trap));
        assert!(seen.contains(&CardType::FieldTrap));
    }

    #[test]
    fn r70_casts_a_random_call_to_chaos_free_counted_as_a_play_and_its_script_resolves() {
        // The cast card's script is one of #95's own effects, so its resolution is visible.
        let mut state = game("chaos-cast", Some(hook(|_ctx| vec![heal_hero_thirty()])));
        let mut events: Vec<GameEvent> = Vec::new();
        state.players.p1.mana.current = 2;
        let me = chaos_card(&mut state, true, None);

        sink_events(&mut state, &mut events, |sink| {
            run(sink, cast_random_call_to_chaos(), Some(&me), P1);
        });

        let played = events_of_type(&events, GameEventType::CardPlayed);
        assert_eq!(played.len(), 1);
        let first = json_of(&played[0]);
        assert_eq!(
            (first["player"].clone(), first["defId"].clone(), first["costPaid"].clone()),
            (json!("p1"), json!(chaos().id), json!(0))
        );
        // R70: a cast counts as a play for everything that counts plays, and pays nothing.
        assert_eq!(state.counters.played, 1);
        assert_eq!(state.players.p1.turn_log.cards_played, 1);
        assert_eq!(state.players.p1.mana.current, 2);
        // The script ran: heal 30 on the caster's hero.
        assert_eq!(state.players.p1.hero.health, 60);

        let cast = state.players.p1.graveyard.first();
        assert!(cast.is_some());
        assert_eq!(cast.map(|card| card.def_id.clone()), Some(chaos().id));
        // R28: a Radiant Call still casts the *base* card.
        assert_eq!(cast.map(|card| card.radiant), Some(false));
        // R215: it was the chain's first cast while it resolved, and it has landed (R87) as the printed
        // card again, carrying no link of that chain into whatever brings it back.
        assert_eq!(chaos_chain_of(cast), 0);
    }

    #[test]
    fn r28_the_base_form_rolls_exactly_one_of_the_ten_effects_and_all_ten_are_reachable() {
        let mut rolled: IndexSet<String> = IndexSet::new();
        for seed in 0..200 {
            let effects = roll_chaos_effects(&mut Rng::new(&format!("base-{seed}"), 0), false, Some(CHAOS_EFFECTS));
            assert_eq!(effects.len(), 1);
            let name = names_of(&effects).first().cloned();
            assert!(CHAOS_EFFECTS.iter().any(|effect| Some(effect.name.to_string()) == name));
            if let Some(name) = name {
                rolled.insert(name);
            }
        }
        assert_eq!(rolled.len(), CHAOS_EFFECTS.len());
        assert_eq!(CHAOS_EFFECTS.len(), 10);
    }

    #[test]
    fn r423_the_radiant_form_rolls_three_different_effects_in_the_order_the_list_writes_them() {
        let mut reached: IndexSet<String> = IndexSet::new();
        let mut with_recursion = 0;
        for seed in 0..300 {
            let effects = roll_chaos_effects(&mut Rng::new(&format!("radiant-{seed}"), 0), true, Some(CHAOS_EFFECTS));
            assert_eq!(effects.len() as i32, CALL_TO_CHAOS_RADIANT_EFFECTS);
            let names = names_of(&effects);
            // Three different entries: none comes up twice.
            assert_eq!(names.iter().collect::<IndexSet<_>>().len() as i32, CALL_TO_CHAOS_RADIANT_EFFECTS);
            // In list order, whatever order they were drawn in.
            let order: Vec<i64> = names
                .iter()
                .map(|name| {
                    CHAOS_EFFECTS
                        .iter()
                        .position(|effect| effect.name == name.as_str())
                        .map(|at| at as i64)
                        .unwrap_or(-1)
                })
                .collect();
            let mut sorted = order.clone();
            sorted.sort();
            assert_eq!(order, sorted);
            for name in &names {
                reached.insert(name.clone());
            }
            if names.iter().any(|name| name == CHAOS_RECURSION.as_str()) {
                with_recursion += 1;
            }
        }
        // Every entry can be rolled, the recursion included — and it is no longer guaranteed (R423
        // rewrites R28's radiant pair): about three rolls in ten hold it.
        assert_eq!(reached.len(), CHAOS_EFFECTS.len());
        assert!(with_recursion > 0);
        assert!(with_recursion < 300);
    }

    #[test]
    fn r423_a_list_shorter_than_three_rolls_all_of_it_once_each() {
        let short = &CHAOS_EFFECTS[..2];
        let rolled = roll_chaos_effects(&mut Rng::new("short-list", 0), true, Some(short));
        assert_eq!(names_of(&rolled), names_of(short));
    }

    #[test]
    fn r423_a_radiant_call_to_chaos_runs_all_three_rolled_effects_the_recursion_only_when_rolled() {
        // A seed whose three are the hero heal, the Chaos Golem and the recursion: all three visible at once.
        let seed = (0..3000).map(|i| format!("chaos-radiant-run-{i}")).find(|candidate| {
            let names = names_of(&roll_chaos_effects(&mut Rng::new(candidate, 0), true, Some(CHAOS_EFFECTS)));
            names.join(",") == ["heal", "golem", CHAOS_RECURSION.as_str()].join(",")
        });
        assert!(seed.is_some());
        let Some(seed) = seed else { return };

        // The cast copy resolves into nothing, so every change left belongs to the card that cast it.
        let mut state = game(&seed, Some(hook(|_ctx| vec![])));
        let mut events: Vec<GameEvent> = Vec::new();
        let me = chaos_card(&mut state, true, None);

        sink_events(&mut state, &mut events, |sink| {
            run(sink, call_to_chaos(CallToChaosArgs::default()), Some(&me), P1);
        });

        assert_eq!(state.players.p1.hero.health, 60);
        assert_eq!(field_of(&events, GameEventType::Summoned, "defId"), vec![json!(golem().id)]);
        assert_eq!(events_of_type(&events, GameEventType::CardPlayed).len(), 1);
        assert_eq!(
            state.players.p1.graveyard.iter().map(|card| card.def_id.clone()).collect::<Vec<String>>(),
            vec![chaos().id]
        );
        // In list order: the heal, then the Golem, then the recursion's cast (R87's "where it falls").
        let types: Vec<GameEventType> = events.iter().map(|event| event.event_type()).collect();
        assert!(index_of(&types, GameEventType::Healed) < index_of(&types, GameEventType::Summoned));
        assert!(index_of(&types, GameEventType::Summoned) < index_of(&types, GameEventType::CardPlayed));

        // A seed whose three leave the recursion out casts nothing at all.
        let no_recursion = (0..400).map(|i| format!("chaos-radiant-none-{i}")).find(|candidate| {
            !names_of(&roll_chaos_effects(&mut Rng::new(candidate, 0), true, Some(CHAOS_EFFECTS)))
                .iter()
                .any(|name| name == CHAOS_RECURSION.as_str())
        });
        assert!(no_recursion.is_some());
        let Some(no_recursion) = no_recursion else { return };
        let mut quiet = game(&no_recursion, Some(hook(|_ctx| vec![])));
        let mut quiet_events: Vec<GameEvent> = Vec::new();
        let quiet_card = chaos_card(&mut quiet, true, None);
        sink_events(&mut quiet, &mut quiet_events, |sink| {
            run(sink, call_to_chaos(CallToChaosArgs::default()), Some(&quiet_card), P1);
        });
        assert!(events_of_type(&quiet_events, GameEventType::CardPlayed).is_empty());
    }

    #[test]
    fn r436_names_what_it_rolled_to_both_players_by_the_printed_clauses_before_any_of_it_resolves() {
        let seed = (0..400).map(|i| format!("chaos-announce-{i}")).find(|candidate| {
            let names = names_of(&roll_chaos_effects(&mut Rng::new(candidate, 0), true, Some(CHAOS_EFFECTS)));
            names.iter().any(|name| name == "heal") && !names.iter().any(|name| name == CHAOS_RECURSION.as_str())
        });
        assert!(seed.is_some());
        let Some(seed) = seed else { return };
        let mut state = game(&seed, None);
        let mut events: Vec<GameEvent> = Vec::new();
        let me = chaos_card(&mut state, true, None);
        let expected = labels_of(&roll_chaos_effects(&mut Rng::new(&seed, 0), true, Some(CHAOS_EFFECTS)));

        sink_events(&mut state, &mut events, |sink| {
            run(sink, call_to_chaos(CallToChaosArgs::default()), Some(&me), P1);
        });

        assert_eq!(
            json_of(events_of_type(&events, GameEventType::ChaosRolled)),
            json!([{ "type": "chaosRolled", "player": "p1", "instanceId": me.id, "defId": chaos().id, "effects": expected }])
        );
        // First, before the first rolled effect lands anything.
        assert_eq!(events.first().map(|event| event.event_type()), Some(GameEventType::ChaosRolled));
        // Every label is a clause of the card's printed list, readable as it stands.
        assert!(expected.iter().all(|label| CHAOS_EFFECTS.iter().any(|effect| effect.label == label.as_str())));
        assert!(expected.contains(&"Heal your hero 30".to_string()));

        // The base face names its one.
        let mut base_state = game("chaos-announce-base", None);
        let mut base_events: Vec<GameEvent> = Vec::new();
        let base_card = chaos_card(&mut base_state, false, None);
        sink_events(&mut base_state, &mut base_events, |sink| {
            run(sink, call_to_chaos(CallToChaosArgs::default()), Some(&base_card), P1);
        });
        let one = events_of_type(&base_events, GameEventType::ChaosRolled);
        assert_eq!(one.len(), 1);
        let base_label = labels_of(&roll_chaos_effects(&mut Rng::new("chaos-announce-base", 0), false, Some(CHAOS_EFFECTS)))
            .first()
            .cloned();
        assert_eq!(json_of(&one[0])["effects"], json!([base_label]));
    }

    #[test]
    fn r28_caps_the_recursion_at_call_to_chaos_chain_cap_casts() {
        // Every cast copy recurses again, which is the worst case the cap has to stop.
        // Each cast reports the link it is as its own script runs; a test builder, not a card file.
        // (TS pushed into a captured array; a hook is `Fn + Send + Sync`, so it reports down a channel.)
        let (sender, receiver) = std::sync::mpsc::channel::<i32>();
        let mut state = game(
            "chaos-chain",
            Some(hook(move |ctx| {
                sender.send(chaos_chain_of(ctx.self_.as_ref())).expect("the test is listening");
                vec![cast_random_call_to_chaos()]
            })),
        );
        let mut events: Vec<GameEvent> = Vec::new();
        let played = chaos_card(&mut state, false, None); // the card the player played: chain 0

        sink_events(&mut state, &mut events, |sink| {
            run(sink, cast_random_call_to_chaos(), Some(&played), P1);
        });
        let depths: Vec<i32> = receiver.try_iter().collect();

        assert_eq!(events_of_type(&events, GameEventType::CardPlayed).len() as i32, CALL_TO_CHAOS_CHAIN_CAP);
        assert_eq!(state.counters.played, CALL_TO_CHAOS_CHAIN_CAP);
        assert_eq!(state.players.p1.graveyard.len() as i32, CALL_TO_CHAOS_CHAIN_CAP);

        // The player's card cast link 1, which cast link 2, and so on to the cap.
        assert_eq!(depths, (1..=CALL_TO_CHAOS_CHAIN_CAP).collect::<Vec<i32>>());
        // R215: each landed in the graveyard (§10.5 step 7, R87) as the printed card again.
        assert!(state.players.p1.graveyard.iter().all(|card| chaos_chain_of(Some(card)) == 0));
        assert_eq!(depths.iter().copied().max(), Some(CALL_TO_CHAOS_CHAIN_CAP));
        assert!(chaos_chain_cap_reached(CALL_TO_CHAOS_CHAIN_CAP));
        assert!(!chaos_chain_cap_reached(CALL_TO_CHAOS_CHAIN_CAP - 1));
    }

    #[test]
    fn r28_the_cap_is_a_hard_stop_a_cast_at_the_cap_resolves_into_nothing_at_all() {
        let mut state = game("chaos-cap", Some(hook(|_ctx| vec![cast_random_call_to_chaos()])));
        let mut events: Vec<GameEvent> = Vec::new();
        let at_cap = chaos_card(&mut state, false, Some(CALL_TO_CHAOS_CHAIN_CAP));
        let before = clone_state(&state);

        sink_events(&mut state, &mut events, |sink| {
            run(sink, cast_random_call_to_chaos(), Some(&at_cap), P1);
        });

        assert!(events.is_empty());
        assert_eq!(state.counters.played, 0);
        assert!(state.players.p1.graveyard.is_empty());
        // Not even an instance was created, so the id counter did not move either.
        assert_eq!(state.next_id, before.next_id);
    }

    #[test]
    fn r28_the_chain_counter_is_instance_state_so_two_calls_in_one_turn_do_not_share_it() {
        let mut state = game("chaos-two-calls", Some(hook(|_ctx| vec![cast_random_call_to_chaos()])));
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        {
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            let first = chaos_card(sink.state, false, None);
            run(&mut sink, cast_random_call_to_chaos(), Some(&first), P1);
            let second = chaos_card(sink.state, false, None);
            run(&mut sink, cast_random_call_to_chaos(), Some(&second), P1);
        }

        assert_eq!(state.counters.played, CALL_TO_CHAOS_CHAIN_CAP * 2);
        // §9.3: the counter lives on the instance, so a serialized game resumes with the same chain.
        assert_eq!(serde_json::from_value::<GameState>(json_of(&state)).unwrap(), state);
    }

    #[test]
    fn s10_7_the_same_seed_rolls_the_same_effects_and_different_seeds_roll_different_ones() {
        let mut first = game("chaos-determinism", None);
        let mut second = game("chaos-determinism", None);
        let self_a = chaos_card(&mut first, false, None);
        let self_b = chaos_card(&mut second, false, None);

        sink_events(&mut first, &mut Vec::new(), |sink| {
            run(sink, call_to_chaos(CallToChaosArgs::default()), Some(&self_a), P1);
        });
        sink_events(&mut second, &mut Vec::new(), |sink| {
            run(sink, call_to_chaos(CallToChaosArgs::default()), Some(&self_b), P1);
        });
        assert_eq!(clone_state(&first), clone_state(&second));

        let names: IndexSet<Option<String>> = (0..20)
            .map(|i| {
                names_of(&roll_chaos_effects(&mut Rng::new(&format!("spread-{i}"), 0), false, Some(CHAOS_EFFECTS)))
                    .first()
                    .cloned()
            })
            .collect();
        assert!(names.len() > 1);
        assert_eq!(
            names_of(&roll_chaos_effects(&mut Rng::new("one", 0), false, Some(CHAOS_EFFECTS))).first(),
            names_of(&roll_chaos_effects(&mut Rng::new("one", 0), false, Some(CHAOS_EFFECTS))).first()
        );
    }

    #[test]
    fn r4_a_full_hand_burns_what_the_draw_and_the_added_cards_cannot_fit() {
        let mut state = game("chaos-full-hand", None);
        let mut events: Vec<GameEvent> = Vec::new();
        in_hand(&mut state, "fx-1", P1, HAND_CAP);
        set_library(&mut state, P1, &["fx-2", "fx-3", "fx-4"]);

        sink_events(&mut state, &mut events, |sink| {
            run(sink, draw_library_and_gain_mana(), None, P1);
            run(sink, add_random_zero_cost_cards(), None, P1);
        });

        assert_eq!(state.players.p1.hand.len() as i32, HAND_CAP);
        assert!(state.players.p1.library.is_empty());
        // §2.4, R4: three drawn and three added, all burned to the graveyard.
        assert_eq!(events_of_type(&events, GameEventType::Burned).len(), 6);
        assert_eq!(state.players.p1.graveyard.len(), 6);
        assert_eq!(state.players.p1.mana.current, 4);
    }

    #[test]
    fn r80_no_chaos_effect_creates_a_library_card_so_a_full_library_only_ever_shrinks() {
        let mut state = game("chaos-library-cap", None);
        let full: Vec<&str> = (0..LIBRARY_CAP).map(|_| "fx-1").collect();
        set_library(&mut state, P1, &full);

        sink_events(&mut state, &mut Vec::new(), |sink| {
            for entry in CHAOS_EFFECTS {
                if entry.name == CHAOS_RECURSION.as_str() {
                    continue; // its own test; nothing here reaches a library
                }
                run(sink, (entry.build)(), None, P1);
                assert!(sink.state.players.p1.library.len() as i32 <= LIBRARY_CAP);
            }
        });
        assert!(state.players.p1.library.is_empty());
    }

    #[test]
    fn r3_r58_an_empty_library_draws_nothing_and_takes_no_fatigue() {
        let mut state = game("chaos-empty-library", None);
        let mut events: Vec<GameEvent> = Vec::new();

        sink_events(&mut state, &mut events, |sink| {
            run(sink, draw_library_and_gain_mana(), None, P1);
        });

        assert!(state.players.p1.hand.is_empty());
        assert_eq!(state.players.p1.fatigue_count, 0);
        assert_eq!(state.players.p1.hero.health, 30);
        assert!(events_of_type(&events, GameEventType::Drawn).is_empty());
        // The mana half of the effect still happens.
        assert_eq!(state.players.p1.mana.current, 4);
    }

    #[test]
    fn r64_a_full_board_fizzles_the_token_golem_and_backrow_summons_without_touching_the_enemy() {
        let mut state = game("chaos-full-board", None);
        let mut events: Vec<GameEvent> = Vec::new();
        for lane in 1..=5 {
            put(&mut state, "fx-1", slot(P1, Row::Units, lane), json!({}));
            put(&mut state, &trap().id, slot(P1, Row::Backrow, lane), json!({}));
        }

        sink_events(&mut state, &mut events, |sink| {
            run(sink, summon_rush_tokens(), None, P1);
            run(sink, summon_chaos_golem(), None, P1);
            run(sink, summon_random_three_cost_units(), None, P1);
            run(sink, summon_random_backrow(), None, P1);
        });

        assert!(events_of_type(&events, GameEventType::Summoned).is_empty());
        assert!(state.players.p2.units.iter().all(|pile| pile.is_none()));
        assert!(state.players.p2.backrow.iter().all(|card| card.is_none()));
    }

    #[test]
    fn r688_a_reserved_zone_is_skipped_but_a_locked_zone_takes_an_unaimed_summon_once_no_unlocked_zone_is_open() {
        let mut state = game("chaos-locked", None);
        let mut events: Vec<GameEvent> = Vec::new();
        state.players.p1.locks.units[0] = true; // lane 1 Locked (§3.2)
        state.reserved.push(ZoneRef {
            player: P1,
            row: Row::Units,
            lane: 2,
        }); // lane 2 held by a Reborn unit

        sink_events(&mut state, &mut events, |sink| {
            run(sink, summon_rush_tokens(), None, P1);
        });

        // Five were asked for: lanes 3, 4 and 5 take the unlocked ones, then the Locked lane 1;
        // the reserved lane 2 is still skipped, so the fifth summon fizzles.
        assert_eq!(
            field_of(&events, GameEventType::Summoned, "lane"),
            vec![json!(3), json!(4), json!(5), json!(1)]
        );
        assert!(state.players.p1.units[0].is_some());
        assert!(state.players.p1.units[1].is_none());
    }
}

mod r28_r87_r423_what_r28_leaves_open_m3_t7 {
    use super::*;

    #[test]
    fn r87_r423_resolves_the_rolled_effects_in_written_order_leaves_a_cast_card_in_the_graveyard_and_rolls_no_substitute_at_the_cap()
     {
        // 1. Written order: the recursion is the list's last entry, so when it is rolled it resolves last.
        for seed in 0..50 {
            let names = names_of(&roll_chaos_effects(&mut Rng::new(&format!("r87-{seed}"), 0), true, Some(CHAOS_EFFECTS)));
            if names.iter().any(|name| name == CHAOS_RECURSION.as_str()) {
                assert_eq!(names.last().map(String::as_str), Some(CHAOS_RECURSION.as_str()));
            }
        }

        // 2. A card cast from no zone ends in the caster's graveyard (§10.5 step 7), so a chain feeds
        // Gravedigger and friends rather than vanishing.
        let mut state = game("r87-graveyard", Some(hook(|_ctx| vec![heal_hero_thirty()])));
        let cast = chaos_card(&mut state, false, None);
        sink_events(&mut state, &mut Vec::new(), |sink| {
            run(sink, cast_random_call_to_chaos(), Some(&cast), P1);
        });
        assert_eq!(
            state.players.p1.graveyard.iter().map(|card| card.def_id.clone()).collect::<Vec<String>>(),
            vec![chaos().id]
        );

        // 3. At the cap the recursion does nothing and nothing is rolled in its place: a radiant Call
        // at the cap runs only its other two effects.
        let seed = (0..400).map(|i| format!("r87-cap-{i}")).find(|candidate| {
            names_of(&roll_chaos_effects(&mut Rng::new(candidate, 0), true, Some(CHAOS_EFFECTS)))
                .iter()
                .any(|name| name == CHAOS_RECURSION.as_str())
        });
        assert!(seed.is_some());
        let Some(seed) = seed else { return };
        let radiant_call = || {
            hook(|_ctx| {
                vec![call_to_chaos(CallToChaosArgs {
                    radiant: Some(true),
                    ..CallToChaosArgs::default()
                })]
            })
        };
        let mut capped = game(&seed, Some(radiant_call()));
        let me = chaos_card(&mut capped, true, Some(CALL_TO_CHAOS_CHAIN_CAP));
        assert!(chaos_chain_cap_reached(chaos_chain_of(Some(&me))));

        let rolled = roll_chaos_effects(&mut Rng::new(&capped.seed, 0), true, Some(CHAOS_EFFECTS));
        let others: Vec<&ChaosEffectDef> = CHAOS_EFFECTS
            .iter()
            .filter(|effect| names_of(&rolled).contains(&effect.name.to_string()) && effect.name != CHAOS_RECURSION.as_str())
            .collect();
        assert_eq!(others.len() as i32, CALL_TO_CHAOS_RADIANT_EFFECTS - 1);

        let mut capped_events: Vec<GameEvent> = Vec::new();
        sink_events(&mut capped, &mut capped_events, |sink| {
            run(
                sink,
                call_to_chaos(CallToChaosArgs {
                    radiant: Some(true),
                    ..CallToChaosArgs::default()
                }),
                Some(&me),
                P1,
            );
        });
        assert!(events_of_type(&capped_events, GameEventType::CardPlayed).is_empty());
        assert_eq!(capped.counters.played, 0);
        // R436: the announcement still names all three: the recursion was rolled, and resolved into nothing.
        assert_eq!(
            events_of_type(&capped_events, GameEventType::ChaosRolled)
                .first()
                .map(|event| json_of(event)["effects"].clone()),
            Some(json!(labels_of(&rolled)))
        );

        // What is left is exactly the other two: the same run on a twin state, with those two alone,
        // produces the same events in the same order after the announcement.
        let mut alone = game(&seed, Some(radiant_call()));
        let mut alone_events: Vec<GameEvent> = Vec::new();
        sink_events(&mut alone, &mut alone_events, |sink| {
            // The twin takes the roll's draws first, so the two effects meet the rng exactly where they did.
            roll_chaos_effects(sink.rng, true, Some(CHAOS_EFFECTS));
            let twin = chaos_card(sink.state, true, Some(CALL_TO_CHAOS_CHAIN_CAP));
            for effect in &others {
                run(sink, (effect.build)(), Some(&twin), P1);
            }
        });

        assert_eq!(
            capped_events[1..].iter().map(|event| event.event_type()).collect::<Vec<GameEventType>>(),
            alone_events.iter().map(|event| event.event_type()).collect::<Vec<GameEventType>>()
        );
    }
}
