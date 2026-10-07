//! C+ #72 Book of Nerf (SPEC §8.7 row 72, R386; BUILD M9 row C+ 72). (1) Spell, Book, Epic.
//!   Base:    "Degrade a permanent {times|time|times}." — times 5
//!   Radiant: "Degrade a card {times|time|times}. It may be a card in your hand." — times 10
//!
//! A declared target (R81): a permanent on either side, and on the Radiant face also a card in your own
//! hand (a broader scope, R275). `times` separate Degrades, each its own draw (R386): attack floors at
//! 0, current health at 1, cost stops at (4), no harmful keyword goes; an Immutable card is left alone.

use jackioh_engine::effects::degrade;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-072";

pub fn script() -> CardScripts {
    let cry: Hook = hook(|ctx| vec![degrade(json_as(json!({ "target": { "of": "chosen" }, "times": param(ctx, "times") })))]);

    let base = Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "backrow"] }))],
        cry: Some(cry.clone()),
        ..Script::default()
    };

    let radiant = Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "backrow", "hand"] }))],
        cry: Some(cry),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C+ #72 Book of Nerf — SPEC §8.7 row 72, BUILD M9 row C+ 72: Degrades a target permanent on either
// side 5 times, each its own draw (R386): attack floors at 0, current health never below 1 (it never
// kills), cost never above (4), no harmful keyword removed, what the floors refuse lost; an Immutable
// target is unchanged; the count reads through `param()`; radiant 10 times on a permanent or a card in
// your hand (the hand clause is the Radiant face's alone).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BOOK: &str = "classicplus-072";
    const UNIT: &str = "core-008"; // Mr. Vanilla 4/4.
    const MROW: &str = "core-086"; // "Miss" Mrow: (1) 1/1, Can't attack.
    const MENACE: &str = "core-019"; // Midrange Menace; Radiant it is Immutable.
    const TRAP: &str = "core-041";
    const FILLER: &str = "core-005";

    /// An engine value as the JSON TS compares it with.
    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialisable")
    }

    /// The `degraded` events, as their JSON (`instanceId`, `defId`, `change`).
    fn degrades(events: &[GameEvent]) -> Vec<Value> {
        events.iter().filter(|event| matches!(event, GameEvent::Degraded { .. })).map(|event| js(event)).collect()
    }

    fn pick(id: &str) -> Value {
        json!([{ "pick": "instance", "instanceId": id }])
    }

    /// The instance ids a play of the Book is offered (`legalActions`).
    fn offered(s: &Scenario) -> Vec<String> {
        let own = s.card(BOOK).id.clone();
        legal_actions(s.state(), P1)
            .iter()
            .map(|action| js(action))
            .filter(|action| action["type"] == json!("play") && action["instanceId"] == json!(own))
            .flat_map(|action| action["targets"].as_array().cloned().unwrap_or_default())
            .filter(|selection| selection["pick"] == json!("instance"))
            .map(|selection| selection["instanceId"].as_str().unwrap_or("").to_string())
            .collect()
    }

    /// TS `book({ radiant?, seed? })`.
    fn book(radiant: bool, seed: Option<&str>) -> Scenario {
        let mut options = json!({
            "p1": { "hand": [{ "def": BOOK, "radiant": radiant }, UNIT, FILLER], "field": [UNIT] },
            "p2": { "hand": [FILLER], "field": [MROW, UNIT], "backrow": [TRAP] }
        });
        if let Some(seed) = seed {
            options["seed"] = json!(seed);
        }
        scenario(options)
    }

    /// TS `stepParam(s.card(ref), key, steps)`: the live card in the state, tuned in place.
    fn step(s: &mut Scenario, card: &str, key: &str, steps: i32) {
        let id = s.card(card).id.clone();
        let instance = find_instance_mut(s.state_mut(), &id).expect("the card is in the state");
        step_param(instance, key, steps);
    }

    #[test]
    fn is_a_1_spell_book_the_radiant_face_declares_a_wider_target() {
        crate::register_all();
        let def = js(&crate::card_def(ID));
        assert_eq!(def["cost"], json!(1));
        assert_eq!(def["type"], json!("Spell"));
        assert_eq!(def["tags"], json!(["Book"]));
        let CardScripts { base, radiant } = script();
        assert_eq!(js(&base.targets)[0]["filter"]["of"], json!(["unit", "backrow"]));
        assert_eq!(js(&radiant.targets)[0]["filter"]["of"], json!(["unit", "backrow", "hand"]));
    }

    mod base {
        use super::*;

        #[test]
        fn r81_its_target_is_a_permanent_on_either_side_never_a_card_in_a_hand() {
            crate::register_all();
            let s = book(false, None);
            let expected: IndexSet<String> = [
                s.unit(P1, 1).expect("p1's unit"),
                s.unit(P2, 1).expect("Miss Mrow"),
                s.unit(P2, 2).expect("p2's unit"),
                s.backrow(P2, 1).expect("p2's trap"),
            ]
            .into_iter()
            .map(|card| card.id)
            .collect();
            let got: IndexSet<String> = offered(&s).into_iter().collect();
            assert_eq!(got.len(), expected.len());
            assert!(got.iter().all(|id| expected.contains(id)));
        }

        #[test]
        fn r386_degrades_a_unit_5_times_each_its_own_draw() {
            crate::register_all();
            let mut s = book(false, None);
            let unit = s.unit(P2, 2).expect("p2's unit");
            s.play(BOOK, json!({ "targets": pick(&unit.id) }));
            let events = degrades(s.events());
            assert_eq!(events.len(), 5);
            assert!(events.iter().all(|event| event["instanceId"] == json!(unit.id)));
        }

        #[test]
        fn r386_it_never_kills_attack_floors_at_0_current_health_at_1_what_the_floors_refuse_is_lost_the_harmful_keyword_stays_cost_stops_at_4() {
            crate::register_all();
            for n in 0..8 {
                let seed = format!("nerf-floors-{n}");
                let mut s = book(false, Some(&seed));
                let mrow = s.unit(P2, 1).expect("Miss Mrow");
                s.play(BOOK, json!({ "targets": pick(&mrow.id) }));
                assert_eq!(s.unit(P2, 1).map(|unit| unit.id), Some(mrow.id.clone()));
                let stats = s.stats(&mrow.id);
                assert!(stats.attack >= 0);
                assert_eq!(stats.health, 1);
                let kinds: Vec<Value> = stats.keywords.iter().map(|keyword| js(keyword)["kind"].clone()).collect();
                assert!(kinds.contains(&json!("Can't attack")));
                assert!(cost_now(s.state(), s.card(&mrow.id)) <= 4);
                for event in degrades(s.events()) {
                    assert_ne!(event["change"]["kind"], json!("keyword"));
                    if event["change"]["kind"] == json!("stats") {
                        assert_eq!(event["change"]["health"], json!(0));
                        assert!(event["change"]["attack"].as_i64().unwrap_or(i64::MIN) >= -1);
                    }
                }
            }
        }

        #[test]
        fn r386_it_may_degrade_your_own_permanent_or_an_enemy_face_down_trap() {
            crate::register_all();
            let hidden_id = jackioh_engine::view_for::HIDDEN_ID;
            let mut own = book(false, None);
            let target = own.unit(P1, 1).expect("p1's unit").id;
            own.play(BOOK, json!({ "targets": pick(&target) }));
            assert_eq!(degrades(own.events()).len(), 5);
            let mut trap = book(false, None);
            let set = trap.backrow(P2, 1).expect("p2's trap");
            trap.play(BOOK, json!({ "targets": pick(&set.id) }));
            assert_eq!(degrades(trap.events()).len(), 5);
            assert!(degrades(&trap.view(P1).events).iter().all(|event| event["instanceId"] == json!(hidden_id)));
        }

        #[test]
        fn r386_an_immutable_target_is_unchanged() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [BOOK, FILLER] }, "p2": { "hand": [FILLER], "field": [{ "def": MENACE, "radiant": true }] } }));
            let menace = s.unit(P2, 1).expect("Midrange Menace");
            s.play(BOOK, json!({ "targets": pick(&menace.id) }));
            assert!(degrades(s.events()).is_empty());
            s.expect_stats(&menace.id, json!({ "attack": 18, "maxHealth": 18 }));
        }

        #[test]
        fn r386_the_count_reads_through_param_an_upgrade_of_times_makes_6_a_degrade_4() {
            crate::register_all();
            for (steps, count) in [(1, 6), (-1, 4)] {
                let mut s = book(false, None);
                step(&mut s, BOOK, "times", steps);
                let target = s.unit(P2, 2).expect("p2's unit").id;
                s.play(BOOK, json!({ "targets": pick(&target) }));
                assert_eq!(degrades(s.events()).len(), count);
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r275_may_also_target_a_card_in_your_hand() {
            crate::register_all();
            let s = book(true, None);
            let hand: Vec<String> = s.hand(P1).into_iter().filter(|card| card.def_id != BOOK).map(|card| card.id).collect();
            let offered = offered(&s);
            assert!(hand.iter().all(|id| offered.contains(id)));
        }

        #[test]
        fn r386_r97_degrades_a_hand_card_10_times_which_the_opponent_reads_only_as_a_card_of_your_hand_changing() {
            crate::register_all();
            let hidden_id = jackioh_engine::view_for::HIDDEN_ID;
            let mut s = book(true, None);
            let held = s.hand(P1).into_iter().find(|card| card.def_id == UNIT).expect("Mr. Vanilla in hand");
            s.play(BOOK, json!({ "targets": pick(&held.id) }));
            assert_eq!(degrades(s.events()).len(), 10);
            assert!(degrades(&s.view(P2).events).iter().all(|event| event["instanceId"] == json!(hidden_id)));
            // Cost never above (4): Mr. Vanilla costs (1).
            assert!(s.card(&held.id).cost_mod <= 3);
        }

        #[test]
        fn r386_r440_degrades_a_permanent_10_times_an_enemy_face_down_trap_is_cued_each_time() {
            crate::register_all();
            let mut s = book(true, None);
            let trap = s.backrow(P2, 1).expect("p2's trap");
            s.play(BOOK, json!({ "targets": pick(&trap.id) }));
            assert_eq!(degrades(s.events()).len(), 10);
        }

        #[test]
        fn r386_r129_a_public_card_the_floors_have_emptied_is_left_alone_with_no_event_mr_vanilla_bottoms_out_at_0_attack_1_health_and_4() {
            crate::register_all();
            let mut emptied = 0;
            for n in 0..8 {
                let mut s = scenario(json!({
                    "seed": format!("nerf-empty-{n}"),
                    "p1": { "hand": [{ "def": BOOK, "radiant": true }, { "def": BOOK, "radiant": true }, FILLER], "mana": 10 },
                    "p2": { "hand": [FILLER], "field": [UNIT] }
                }));
                let unit = s.unit(P2, 1).expect("Mr. Vanilla");
                let hand = s.hand(P1);
                let (first, second) = (hand[0].id.clone(), hand[1].id.clone());
                s.play(&first, json!({ "targets": pick(&unit.id) }));
                let stats = s.stats(&unit.id);
                assert_eq!(s.unit(P2, 1).map(|card| card.id), Some(unit.id.clone()));
                assert!(stats.attack >= 0);
                assert!(stats.health >= 1);
                let cost = cost_now(s.state(), s.card(&unit.id));
                assert!(cost <= 4);
                if stats.attack != 0 || stats.health != 1 || cost != 4 {
                    continue;
                }
                emptied += 1;
                let cursor = s.state().rng_cursor;
                s.play(&second, json!({ "targets": pick(&unit.id) }));
                assert!(degrades(s.last_events()).is_empty());
                assert_eq!(s.state().rng_cursor, cursor);
            }
            assert!(emptied > 0);
        }
    }
}
