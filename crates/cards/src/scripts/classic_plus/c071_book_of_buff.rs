//! C+ #71 Book of Buff (SPEC §8.7 row 71, R386; BUILD M9 row C+ 71). (1) Spell, Book, Epic.
//!   Both faces: "Buff a card {times|time|times}. It may be a card in your hand." — times 5, Radiant 10
//!
//! A declared target (R81): a permanent on either side or a card in your own hand. `times` separate
//! Upgrades, each its own draw (R386); an Immutable card is left alone. The Radiant ten is the declared
//! `times`, so both faces run this one script.

use jackioh_engine::effects::upgrade;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-071";

pub fn script() -> CardScripts {
    let base = Script {
        // R656: an Upgrade helps, so a random cast that targets enemies aims this at friends.
        targets: vec![TargetDecl {
            aim: Some(TargetAim::Help),
            ..TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "backrow", "hand"] }))
        }],
        cry: Some(hook(|ctx| vec![upgrade(json_as(json!({ "target": { "of": "chosen" }, "times": param(ctx, "times") })))])),
        ..Script::default()
    };

    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C+ #71 Book of Buff — SPEC §8.7 row 71, BUILD M9 row C+ 71: Upgrades one card 5 times, the target
// chosen with the play (R81) — a permanent on either side (an enemy face-down trap offered by id only,
// R177) or a card in your hand; each Upgrade its own draw (R386); an Immutable target is unchanged; a
// hand card's changes are hidden from the opponent (R97); the count reads through `param()`; radiant
// 10 times.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BOOK: &str = "classicplus-071";
    const UNIT: &str = "core-008"; // Mr. Vanilla 4/4.
    const MENACE: &str = "core-019"; // Midrange Menace; Radiant it is Immutable.
    const TRAP: &str = "core-041"; // Sheepish, set face-down.
    const FIELD: &str = "core-006"; // Mana Well.
    const FILLER: &str = "core-005";

    use crate::js;

    /// The `upgraded` events, as their JSON (`instanceId`, `defId`, `change`).
    fn upgrades(events: &[GameEvent]) -> Vec<Value> {
        events.iter().filter(|event| matches!(event, GameEvent::Upgraded { .. })).map(js).collect()
    }

    fn pick(id: &str) -> Value {
        json!([{ "pick": "instance", "instanceId": id }])
    }

    /// TS `book({ radiant? })`.
    fn book(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": BOOK, "radiant": radiant }, UNIT, FILLER], "field": [UNIT], "backrow": [FIELD] },
            "p2": { "hand": [FILLER], "field": [UNIT], "backrow": [TRAP] }
        }))
    }

    #[test]
    fn is_a_1_spell_book_with_one_script_on_both_faces() {
        crate::register_all();
        let def = js(&crate::card_def(ID));
        assert_eq!(def["cost"], json!(1));
        assert_eq!(def["type"], json!("Spell"));
        assert_eq!(def["tags"], json!(["Book"]));
        let CardScripts { base, radiant } = script();
        assert!(Arc::ptr_eq(base.cry.as_ref().unwrap(), radiant.cry.as_ref().unwrap()));
        assert_eq!(js(&base.targets), js(&radiant.targets));
    }

    mod base {
        use super::*;

        #[test]
        fn r81_r177_the_target_is_a_permanent_on_either_side_or_a_card_in_your_hand_an_enemy_face_down_trap_is_offered_by_id_only() {
            crate::register_all();
            let s = book(false);
            let own = s.card(BOOK).id.clone();
            let offered: Vec<Value> = legal_actions(s.state(), P1)
                .iter()
                .map(js)
                .filter(|action| action["type"] == json!("play") && action["instanceId"] == json!(own))
                .flat_map(|action| action["targets"].as_array().cloned().unwrap_or_default())
                .collect();
            let ids: Vec<String> = offered
                .iter()
                .map(|selection| {
                    if selection["pick"] == json!("instance") {
                        selection["instanceId"].as_str().unwrap_or("").to_string()
                    } else {
                        selection["pick"].as_str().unwrap_or("").to_string()
                    }
                })
                .collect();
            let mut expected: Vec<String> = [
                s.unit(P1, 1).expect("p1's unit"),
                s.backrow(P1, 1).expect("p1's Mana Well"),
                s.unit(P2, 1).expect("p2's unit"),
                s.backrow(P2, 1).expect("p2's trap"),
            ]
            .into_iter()
            .map(|card| card.id)
            .collect();
            expected.extend(s.hand(P1).into_iter().filter(|card| card.id != own).map(|card| card.id));
            let got: IndexSet<String> = ids.iter().cloned().collect();
            let want: IndexSet<String> = expected.into_iter().collect();
            assert_eq!(got.len(), want.len());
            assert!(got.iter().all(|id| want.contains(id)));
            for selection in &offered {
                let mut keys: Vec<&String> = selection.as_object().map(|object| object.keys().collect()).unwrap_or_default();
                keys.sort();
                assert_eq!(keys, vec!["instanceId", "pick"]);
            }
            assert!(!ids.contains(&s.hand(P2)[0].id));
        }

        #[test]
        fn r386_upgrades_a_unit_5_times_each_its_own_draw() {
            crate::register_all();
            let mut s = book(false);
            let unit = s.unit(P2, 1).expect("p2's unit");
            s.play(BOOK, json!({ "targets": pick(&unit.id) }));
            let events = upgrades(s.events());
            assert_eq!(events.len(), 5);
            assert!(events.iter().all(|event| event["instanceId"] == json!(unit.id)));
            assert!(events.iter().all(|event| event["change"]["kind"] != json!("none")));
        }

        #[test]
        fn r97_upgrades_a_card_in_your_hand_and_the_opponent_reads_only_that_a_card_of_your_hand_changed() {
            crate::register_all();
            let hidden_id = jackioh_engine::view_for::HIDDEN_ID;
            let mut s = book(false);
            let held = s.hand(P1).into_iter().find(|card| card.def_id == UNIT).expect("Mr. Vanilla in hand");
            s.play(BOOK, json!({ "targets": pick(&held.id) }));
            assert_eq!(upgrades(s.events()).len(), 5);
            let live = s.card(&held.id);
            assert!(live.tuning.is_some() || live.cost_mod != 0);
            let theirs = upgrades(&s.view(P2).events);
            assert_eq!(theirs.len(), 5);
            assert!(theirs.iter().all(|event| event["instanceId"] == json!(hidden_id) && event["defId"] == json!(hidden_id)));
            assert!(upgrades(&s.view(P1).events).iter().all(|event| event["instanceId"] == json!(held.id)));
        }

        #[test]
        fn r177_an_enemy_face_down_trap_may_be_its_target_its_changes_are_its_controller_s_to_read() {
            crate::register_all();
            let hidden_id = jackioh_engine::view_for::HIDDEN_ID;
            let mut s = book(false);
            let trap = s.backrow(P2, 1).expect("p2's trap");
            s.play(BOOK, json!({ "targets": pick(&trap.id) }));
            assert_eq!(upgrades(s.events()).len(), 5);
            assert!(upgrades(&s.view(P1).events).iter().all(|event| event["instanceId"] == json!(hidden_id)));
            assert!(upgrades(&s.view(P2).events).iter().all(|event| event["instanceId"] == json!(trap.id)));
        }

        #[test]
        fn r386_an_immutable_target_is_unchanged_with_no_draw() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [BOOK, FILLER] }, "p2": { "hand": [FILLER], "field": [{ "def": MENACE, "radiant": true }] } }));
            let menace = s.unit(P2, 1).expect("Midrange Menace");
            s.play(BOOK, json!({ "targets": pick(&menace.id) }));
            assert!(upgrades(s.events()).is_empty());
            assert!(s.card(&menace.id).tuning.is_none());
            s.expect_stats(&menace.id, json!({ "attack": 18, "maxHealth": 18 }));
        }

        #[test]
        fn r90_with_nothing_to_target_the_play_is_still_legal_and_upgrades_nothing() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [BOOK] }, "p2": { "hand": [FILLER] } }));
            s.play(BOOK, json!({}));
            assert!(upgrades(s.events()).is_empty());
            s.expect_in_zone(BOOK, "graveyard");
        }

        #[test]
        fn r386_the_count_reads_through_param_an_upgrade_of_times_makes_6_a_degrade_4() {
            crate::register_all();
            for (steps, count) in [(1, 6), (-1, 4)] {
                let mut s = book(false);
                step_param(s.card_mut(BOOK), "times", steps);
                let target = s.unit(P1, 1).expect("p1's unit").id;
                s.play(BOOK, json!({ "targets": pick(&target) }));
                assert_eq!(upgrades(s.events()).len(), count);
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r386_upgrades_its_target_10_times() {
            crate::register_all();
            let mut s = book(true);
            let unit = s.unit(P1, 1).expect("p1's unit");
            s.play(BOOK, json!({ "targets": pick(&unit.id) }));
            assert_eq!(upgrades(s.events()).len(), 10);
            assert!(upgrades(s.events()).iter().all(|event| event["instanceId"] == json!(unit.id)));
        }

        #[test]
        fn may_still_target_a_card_in_your_hand() {
            crate::register_all();
            let mut s = book(true);
            let held = s.hand(P1).into_iter().find(|card| card.def_id == UNIT).expect("Mr. Vanilla in hand");
            s.play(BOOK, json!({ "targets": pick(&held.id) }));
            assert_eq!(upgrades(s.events()).len(), 10);
        }
    }
}
