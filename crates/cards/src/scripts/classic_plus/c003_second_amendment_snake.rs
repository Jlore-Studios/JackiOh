//! C+ #3 Second Amendment Snake (SPEC §8.7 row 3). (2) Unit, Rare, 1/6 → 2/12.
//! End of turn: one placement of {tokens} Plague Counters on itself (E19). Death: its last-known tokens
//! (R78) become that many hits of {damage}, each on a random enemy still standing (E37, R59). Both faces
//! run this script; the numbers are the face's declared `params` (R386). Preview: the hits (R280).

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-003";

/// R280: the words the preview's value follows, on both faces.
const HITS_LABEL: &str = "for each Plague Counter on this";

/// One hit per Plague Counter on the card — the Death's count and the preview's (R78: last-known).
fn hits(self_: Option<&CardInstance>) -> i32 {
    self_.and_then(|card| card.counters.plague).unwrap_or(0)
}

pub fn script() -> CardScripts {
    let base = Script {
        end_of_turn: Some(hook(|ctx| vec![place_plague(json_as(json!({ "amount": param(&*ctx, "tokens") })))])),
        death: Some(hook(|ctx| {
            let per_hit = param(&*ctx, "damage");
            vec![damage_split(json_as(json!({
                "amount": hits(ctx.self_.as_ref()) * per_hit,
                "perHit": per_hit,
                "among": "enemies",
            })))]
        })),
        preview: Some(condition_hook(|c| {
            vec![json_as::<PreviewValue>(json!({ "label": HITS_LABEL, "value": hits(Some(c.self_)) }))]
        })),
        ..Script::default()
    };
    // The Radiant face differs only in its stats and its `tokens` value (3), both catalog data.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// SPEC §8.7 row 3: Death reads last-known tokens (R78) and deals that many hits of 1,
// each to a random enemy still standing, all before the state check (R59); 0 tokens deals
// nothing and draws nothing (R129); preview is the hits its Death would deal now (R280).
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;
    use std::sync::Arc;

    const SNAKE: &str = "classicplus-003";
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy target Unit.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const MR_TOKEN: &str = "core-015"; // (1) 1/1.
    const SOLARIUS: &str = "classicplus-038"; // Spell Damage +2.
    const FILLER: &str = "core-010";
    const STOCKPILE: &str = "core-005";
    const HITS_LABEL: &str = "for each Plague Counter on this";

    /// One of the Snake's `damage` events: where it sits in the event list, what it hit, how hard.
    struct Hit {
        index: usize,
        target_id: String,
        amount: i32,
    }

    fn snake_hits(s: &Scenario, snake_id: &str) -> Vec<Hit> {
        s.events()
            .iter()
            .enumerate()
            .filter_map(|(index, event)| match event {
                GameEvent::Damage { source_id: Some(source), target_id, amount, .. } if source == snake_id => {
                    Some(Hit { index, target_id: target_id.clone(), amount: *amount })
                }
                _ => None,
            })
            .collect()
    }

    use crate::merged;

    #[derive(Default)]
    struct Extra {
        radiant: bool,
        p1: Option<Value>,
        seed: Option<String>,
    }

    fn with_snake(tokens: i32, p2: Value, extra: Extra) -> Scenario {
        crate::register_all();
        let p1 = merged(
            json!({
                "hand": [HIT_JOB, FILLER],
                "library": [STOCKPILE, STOCKPILE],
                "field": [{ "def": SNAKE, "radiant": extra.radiant, "counters": { "plague": tokens } }],
            }),
            extra.p1.unwrap_or_else(|| json!({})),
        );
        let mut options = json!({
            "p1": p1,
            "p2": merged(json!({ "hand": [FILLER], "library": [STOCKPILE, STOCKPILE] }), p2),
        });
        if let Some(seed) = extra.seed {
            options["seed"] = json!(seed);
        }
        scenario(options)
    }

    fn kill_snake(s: &mut Scenario) -> &mut Scenario {
        let id = s.card(SNAKE).id.clone();
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": id }] }))
    }

    /// The Snake's `preview` as `viewer` reads it on the board (`None`: no Snake, or no preview).
    fn snake_preview(s: &Scenario, viewer: PlayerId) -> Option<Vec<PreviewValue>> {
        let view = s.view(viewer);
        let side = if viewer == P1 { view.you } else { view.opponent };
        side.units.into_iter().flatten().find(|unit| unit.def_id == SNAKE).and_then(|unit| unit.preview)
    }

    use crate::matches_object;

    fn preview_json(shown: &Option<Vec<PreviewValue>>) -> Value {
        serde_json::to_value(shown).unwrap()
    }

    #[test]
    fn is_a_2_1_6_unit_declaring_tokens_2_3_and_damage_1_one_script_runs_both_faces() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.cost, CardCost::Fixed(2));
        assert_eq!(
            [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
            [Some(1), Some(6), Some(2), Some(12)]
        );
        assert_eq!(
            def.params.unwrap_or_default().iter().map(|p| (p.key.clone(), p.base, p.radiant)).collect::<Vec<_>>(),
            vec![("tokens".to_string(), 2, 3), ("damage".to_string(), 1, 1)]
        );
        // The Radiant face is the base face's very script.
        let scripts = script();
        let same = |a: &Option<Hook>, b: &Option<Hook>| match (a, b) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            _ => false,
        };
        assert!(same(&scripts.base.end_of_turn, &scripts.radiant.end_of_turn));
        assert!(same(&scripts.base.death, &scripts.radiant.death));
        assert!(matches!(
            (&scripts.base.preview, &scripts.radiant.preview),
            (Some(a), Some(b)) if Arc::ptr_eq(a, b)
        ));
    }

    mod base {
        use super::*;

        #[test]
        fn e19_at_its_controller_s_end_of_turn_places_2_plague_counters_on_itself_in_one_placement_not_at_the_opponent_s()
        {
            crate::register_all();
            let mut s = with_snake(0, json!({}), Extra::default());
            let snake = s.card(SNAKE).clone();

            s.end_turn();

            assert_eq!(s.card(&snake).counters.plague, Some(2));
            let changes: Vec<Value> = s
                .events()
                .iter()
                .filter(|event| matches!(event, GameEvent::CounterChanged { instance_id, .. } if *instance_id == snake.id))
                .map(|event| serde_json::to_value(event).unwrap())
                .collect();
            assert_eq!(changes.len(), 1);
            assert!(matches_object(&changes[0], &json!({ "counter": "plague", "value": 2 })));

            s.end_turn();
            assert_eq!(s.state().active, P1);
            assert_eq!(s.card(&snake).counters.plague, Some(2));
        }

        #[test]
        fn r78_e37_its_death_reads_its_last_known_tokens_that_many_hits_of_1_each_on_a_random_enemy() {
            crate::register_all();
            let mut s = with_snake(3, json!({ "field": [VANILLA, VANILLA] }), Extra::default());
            let snake = s.card(SNAKE).clone();

            kill_snake(&mut s);

            let hits = snake_hits(&s, &snake.id);
            assert_eq!(hits.len(), 3);
            assert!(hits.iter().all(|hit| hit.amount == 1));
            let mut enemies: IndexSet<String> = IndexSet::new();
            enemies.insert("hero-p2".to_string());
            for lane in [1, 2] {
                if let Some(unit) = s.unit(P2, lane) {
                    enemies.insert(unit.id);
                }
            }
            assert!(hits.iter().all(|hit| enemies.contains(&hit.target_id)));
            let dealt = 30 - s.state().players.p2.hero.health
                + [1, 2].iter().map(|&lane| 4 - s.stats(s.unit(P2, lane).unwrap()).health).sum::<i32>();
            assert_eq!(dealt, 3);
            s.expect_health(P1, 30);
        }

        #[test]
        fn r59_a_unit_an_earlier_hit_brought_to_0_is_not_picked_again_and_it_dies_after_the_last_hit() {
            crate::register_all();
            let mut deaths = 0;
            for seed in 1..=8 {
                let mut s = with_snake(
                    5,
                    json!({ "field": [MR_TOKEN] }),
                    Extra { seed: Some(format!("snake-{seed}")), ..Extra::default() },
                );
                let token = s.card(MR_TOKEN).clone();

                kill_snake(&mut s);

                let snake_id = s.card(SNAKE).id.clone();
                let hits = snake_hits(&s, &snake_id);
                assert_eq!(hits.len(), 5);
                assert!(hits.iter().filter(|hit| hit.target_id == token.id).count() <= 1);
                let died = s.events().iter().position(
                    |event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == token.id),
                );
                let last = hits[4].index;
                if let Some(died) = died {
                    deaths += 1;
                    assert!(died > last);
                }
            }
            assert!(deaths > 0);
        }

        #[test]
        fn r129_with_no_tokens_it_deals_nothing_and_draws_no_random_number() {
            crate::register_all();
            let mut s = with_snake(0, json!({ "field": [VANILLA] }), Extra::default());
            let cursor = s.state().rng_cursor;

            kill_snake(&mut s);

            s.expect_in_zone(SNAKE, "graveyard");
            let snake_id = s.card(SNAKE).id.clone();
            assert_eq!(snake_hits(&s, &snake_id).len(), 0);
            assert_eq!(s.state().rng_cursor, cursor);
            s.expect_health(P2, 30);
        }

        #[test]
        fn e6_its_hits_are_no_spell_s_spell_damage_on_its_side_never_raises_them() {
            crate::register_all();
            let mut s = with_snake(
                2,
                json!({}),
                Extra {
                    p1: Some(json!({ "field": [{ "def": SNAKE, "counters": { "plague": 2 } }, SOLARIUS] })),
                    ..Extra::default()
                },
            );

            kill_snake(&mut s);

            let snake_id = s.card(SNAKE).id.clone();
            assert_eq!(snake_hits(&s, &snake_id).iter().map(|hit| hit.amount).collect::<Vec<_>>(), vec![1, 1]);
            s.expect_health(P2, 28);
        }

        #[test]
        fn dies_in_combat_too_the_death_fires_off_the_tokens_it_had() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": [FILLER], "field": [{ "def": SNAKE, "counters": { "plague": 2 } }] },
                "p2": { "hand": [FILLER], "field": [{ "def": "core-013" }] }, // Jlockeed Shredder-10, 8/10
            }));
            let snake = s.card(SNAKE).clone();

            let attacker = s.unit(P2, 1).unwrap();
            s.attack(&attacker, &snake);

            s.expect_in_zone(&snake, "graveyard");
            // Its strike back is its own hit of 1; the Death's two come after it dies.
            let died = s
                .events()
                .iter()
                .position(|event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == snake.id))
                .expect("the Snake died");
            assert_eq!(snake_hits(&s, &snake.id).iter().filter(|hit| hit.index > died).count(), 2);
        }

        #[test]
        fn r386_tokens_per_turn_and_damage_per_token_read_through_param_an_upgrade_of_each_moves_what_resolves() {
            crate::register_all();
            let mut s = with_snake(0, json!({}), Extra::default());
            step_param(s.card_mut(SNAKE), "tokens", 1);
            s.end_turn();
            assert_eq!(s.card(SNAKE).counters.plague, Some(3));

            let mut t = with_snake(2, json!({}), Extra::default());
            step_param(t.card_mut(SNAKE), "damage", 1);
            kill_snake(&mut t);
            let snake_id = t.card(SNAKE).id.clone();
            assert_eq!(snake_hits(&t, &snake_id).iter().map(|hit| hit.amount).collect::<Vec<_>>(), vec![2, 2]);
            t.expect_health(P2, 26);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn places_3_plague_counters_at_its_controller_s_end_of_turn_and_its_death_deals_3_hits() {
            crate::register_all();
            let mut s = with_snake(0, json!({}), Extra { radiant: true, ..Extra::default() });
            let snake = s.card(SNAKE).clone();

            s.end_turn();
            assert_eq!(s.card(&snake).counters.plague, Some(3));
            s.end_turn();

            kill_snake(&mut s);
            assert_eq!(snake_hits(&s, &snake.id).len(), 3);
            s.expect_health(P2, 27);
        }
    }

    mod r280_preview {
        use super::*;

        #[test]
        fn is_the_hits_its_death_would_deal_now_for_both_viewers_and_the_label_is_in_each_face_s_text() {
            crate::register_all();
            let def = crate::card_def(ID);
            for face in [false, true] {
                let mut s = with_snake(4, json!({}), Extra { radiant: face, ..Extra::default() });
                let shown = snake_preview(&s, P1);
                assert_eq!(preview_json(&shown), json!([{ "label": HITS_LABEL, "value": 4 }]));
                assert_eq!(preview_json(&snake_preview(&s, P2)), preview_json(&shown));
                assert!((if face { &def.radiant } else { &def.base }).text.contains(HITS_LABEL));

                kill_snake(&mut s);
                let snake_id = s.card(SNAKE).id.clone();
                assert_eq!(snake_hits(&s, &snake_id).len(), 4);
            }
        }

        #[test]
        fn follows_the_tokens_0_on_a_fresh_snake_2_after_one_end_of_turn() {
            crate::register_all();
            let mut s = with_snake(0, json!({}), Extra::default());
            assert_eq!(preview_json(&snake_preview(&s, P1)), json!([{ "label": HITS_LABEL, "value": 0 }]));
            s.end_turn();
            assert_eq!(preview_json(&snake_preview(&s, P2)), json!([{ "label": HITS_LABEL, "value": 2 }]));
        }

        #[test]
        fn reads_only_its_own_counters_the_hook_answers_off_a_state_with_no_libraries_or_hands() {
            crate::register_all();
            let s = with_snake(3, json!({}), Extra::default());
            let mut bare: GameState = s.state().clone();
            for player in [P1, P2] {
                bare.players[player].library = vec![];
                bare.players[player].hand = vec![];
            }
            let self_ = bare.players.p1.units[0]
                .as_ref()
                .and_then(|pile| pile.first())
                .cloned()
                .expect("no Snake in lane 1");
            let preview = script().base.preview.expect("the base face has a preview");
            let shown = preview(ConditionContext {
                state: &bare,
                self_: &self_,
                controller: P1,
                radiant: false,
                zone: ConditionZone::Field,
                your_turn: true,
            });
            assert_eq!(serde_json::to_value(&shown).unwrap(), json!([{ "label": HITS_LABEL, "value": 3 }]));
        }
    }
}
