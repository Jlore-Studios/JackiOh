//! #36 Magic Jammed (SPEC §8.2): "Destroy target backrow card; Lock its zone", radiant "Steal
//! target backrow card; Lock its original zone". The radiant cell restates BOTH base clauses, so it
//! replaces both and nothing base-only survives (§8 Conventions).
//! The target is declared, so it travels in the `play` action and resolution never pauses (R81).
//! Unnarrowed means either side: an ALLY backrow card is a legal pick (unlike #49), and stealing it
//! does nothing (`steal` refuses your own card, R76) while the Lock still lands (§3.2).
//! EFFECT ORDER is load-bearing: `lock` reads the zone where the chosen card sits now and `steal`
//! MOVES it (R15), so the lock runs FIRST. `destroy` only marks the card (§4.5), so the base order
//! is not load-bearing. R33's visibility needs no line here: `viewFor` keys it on `controller`.
//! Any face-down backrow card is a target, a set Cloaked card included (R3777000); a client clicks its
//! zone (R3777001).

use jackioh_engine::effects::{destroy, lock, steal};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-036";

/// R81: one backrow card, picked with the play. Unnarrowed, so either side (§8 Conventions).
fn backrow_target() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["backrow"] }))]
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            targets: backrow_target(),
            // Lock before Destroy: §6.3 Destroy only marks, but the two faces stay in one order.
            cry: Some(hook(|_ctx| {
                vec![
                    lock(json_as(json!({ "zone": { "of": "chosen" } }))),
                    destroy(json_as(json!({ "target": { "of": "chosen" } }))),
                ]
            })),
            ..Script::default()
        },
        radiant: Script {
            targets: backrow_target(),
            // Lock strictly before Steal, so "its original zone" is the zone the card is still sitting in.
            cry: Some(hook(|_ctx| {
                vec![
                    lock(json_as(json!({ "zone": { "of": "chosen" } }))),
                    steal(json_as(json!({ "target": { "of": "chosen" } }))),
                ]
            })),
            ..Script::default()
        },
    }
}

// #36 Magic Jammed (SPEC §8.2, BUILD M4-T4) (R33).
#[cfg(test)]
mod tests {
    use super::script;
    use crate::js;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const MAGIC_JAMMED: &str = "core-036";
    /// A Field Spell: a public backrow card (§3.2).
    const MANA_WELL: &str = "core-006";
    /// A Trap: face-down in the backrow, so R33 has something to hide (§3.2).
    const SHEEPISH: &str = "core-041";
    /// A spare hand card, so no side runs out of meaningful actions and auto-ends its turn.
    const SPARE: &str = "core-005";
    /// (1) 4/4 Unit with no text.
    const VANILLA: &str = "core-008";
    /// C #6 Cloaked Toe Cracker: a (2) Unit with Cloaked, so it may be set face-down as a Trap (§6.1).
    const CRACKER: &str = "classic-006";
    /// M #99 Paranoia: a Field Spell whose Aura gives its controller's Spells Cloaked (R1044).
    const PARANOIA: &str = "meditative-099";

    /// Registers the catalog first: the engine's testkit cannot name the cards crate.
    fn scn(opts: Value) -> Scenario {
        scenario(opts)
    }

    fn view_json(s: &Scenario, seat: &str) -> Value {
        js(&s.view(seat))
    }

    /// Every key of `expected` is in `actual` with that value.
    fn assert_matches_object(actual: &Value, expected: &Value) {
        let expected = expected.as_object().expect("toMatchObject takes an object");
        for (key, value) in expected {
            assert_eq!(&actual[key.as_str()], value, "key {key} of {actual}");
        }
    }

    /// R783004: on p2's turn, p2 sets `def` face-down by its Cloaked in backrow lane 2, beside
    /// `p2_backrow`; then p1's turn begins with `jammed` in hand. The set card's fresh id (R227).
    fn set_by_p2(def: &str, p2_backrow: Value, jammed: Value) -> (Scenario, String) {
        let mut s = scn(json!({
            "active": "p2",
            "p1": { "hand": [jammed, SPARE], "library": [SPARE, SPARE] },
            "p2": { "hand": [def, SPARE], "backrow": p2_backrow, "library": [SPARE, SPARE] },
        }));
        s.play(def, json!({ "zone": 2, "row": "backrow", "faceDown": "cloaked" }));
        let set = s.backrow("p2", 2).expect("the set card in p2's backrow lane 2");
        assert!(set.set_as.is_some(), "set by its Cloaked");
        s.end_turn();
        (s, set.id)
    }

    /// Whether `legalActions` lists p1's Magic Jammed aimed at `target` alone.
    fn listed(s: &Scenario, target: &str) -> bool {
        let jammed = s
            .hand("p1")
            .into_iter()
            .find(|card| card.def_id == MAGIC_JAMMED)
            .expect("Magic Jammed in p1's hand")
            .id;
        let aimed = vec![Selection::Instance {
            instance_id: target.to_string(),
        }];
        s.offered("p1").iter().any(|action| {
            matches!(action, ActionBody::Play { instance_id, targets: Some(targets), .. }
                if *instance_id == jammed && *targets == aimed)
        })
    }

    mod magic_jammed {
        use super::*;

        #[test]
        fn r81_declares_its_backrow_target_as_a_play_time_choice_unnarrowed_by_side() {
            // §8 Conventions: "target" is either side unless the cell narrows it, and neither cell does.
            let scripts = script();
            for face in [&scripts.base, &scripts.radiant] {
                assert_eq!(
                    js(&face.targets),
                    json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["backrow"] } }])
                );
            }
        }

        #[test]
        fn base_destroys_the_target_backrow_card_and_locks_its_zone_sec3_2_lock() {
            let mut s = scn(json!({
                "p1": { "hand": [MAGIC_JAMMED, SPARE], "library": [SPARE] },
                "p2": { "backrow": [{ "def": MANA_WELL, "lane": 3 }], "hand": [SPARE], "library": [SPARE] },
            }));
            let target = s
                .backrow("p2", 3)
                .expect("setup should have put Mana Well in p2's backrow lane 3");

            s.play(MAGIC_JAMMED, json!({ "targets": pick::id(&target.id) }));

            s.expect_in_zone(&target, "graveyard");
            assert!(s.backrow("p2", 3).is_none());
            // §3.2: the lock lives on the zone and outlives its occupant.
            assert!(s.state().players.p2.locks.backrow[2]);
            assert!(s.state().pending.is_none());
        }

        /// R1081000: a Unit standing in a backrow zone is no backrow card (§6.1 Animated): no target.
        #[test]
        fn r1081000_a_unit_standing_in_the_backrow_is_no_backrow_card_to_target() {
            let mut s = scn(json!({
                "p1": { "hand": [MAGIC_JAMMED, SPARE], "library": [SPARE] },
                "p2": { "field": [{ "def": VANILLA, "lane": 1 }], "hand": [SPARE], "library": [SPARE] },
            }));
            let unit = s.unit("p2", 1).expect("a Unit in lane 1").id;
            crate::stand_in_backrow(&mut s, &unit, 3);

            s.expect_refused(|s| s.play(MAGIC_JAMMED, json!({ "targets": pick::id(&unit) })));

            s.expect_in_zone(unit.as_str(), "field");
        }

        /// R3777000: a face-down Trap is a backrow card to target, offered by its bare id (R177).
        #[test]
        fn r3777000_base_destroys_an_enemy_face_down_trap_listed_and_locks_its_zone() {
            let mut s = scn(json!({
                "p1": { "hand": [MAGIC_JAMMED, SPARE], "library": [SPARE] },
                "p2": { "backrow": [{ "def": SHEEPISH, "lane": 3 }], "hand": [SPARE], "library": [SPARE] },
            }));
            let trap = s.backrow("p2", 3).expect("Sheepish in p2's backrow lane 3");
            let before = view_json(&s, "p1");
            assert_eq!(before["opponent"]["backrow"][2], json!({ "faceDown": true, "cost": 1 }));
            assert!(!before.to_string().contains(SHEEPISH), "R33: p1 reads nothing of it");
            assert!(listed(&s, &trap.id));

            s.play(MAGIC_JAMMED, json!({ "targets": pick::id(&trap.id) }));

            s.expect_in_zone(&trap, "graveyard");
            assert!(s.backrow("p2", 3).is_none());
            assert!(s.state().players.p2.locks.backrow[2]);
        }

        /// R3777000, R783008: a set Cloaked Unit is a backrow card, no Unit; destroyed unrevealed, it is
        /// its printed card again in its owner's graveyard.
        #[test]
        fn r3777000_base_destroys_a_set_cloaked_unit_which_reaches_the_graveyard_as_its_printed_card() {
            let (mut s, id) = set_by_p2(CRACKER, json!([]), json!(MAGIC_JAMMED));
            assert!(listed(&s, &id));

            s.play(MAGIC_JAMMED, json!({ "targets": pick::id(&id) }));

            let dead = s
                .pile("p2", "graveyard")
                .into_iter()
                .find(|card| card.id == id)
                .expect("the set Toe Cracker in p2's graveyard");
            assert_eq!(dead.def_id, CRACKER);
            assert_eq!(dead.set_as, None);
            assert!(s.backrow("p2", 2).is_none());
            assert!(s.state().players.p2.locks.backrow[1]);
        }

        /// R3777000: Radiant steals a set Cloaked card as the set card it is, and its owner stops reading it.
        #[test]
        fn r3777000_radiant_steals_a_set_cloaked_unit_still_set_and_locks_its_original_zone() {
            let (mut s, id) = set_by_p2(CRACKER, json!([]), json!({ "def": MAGIC_JAMMED, "radiant": true }));
            assert!(listed(&s, &id));

            s.play(MAGIC_JAMMED, json!({ "targets": pick::id(&id) }));

            assert_eq!(s.backrow("p1", 2).map(|card| card.id), Some(id.clone()));
            let stolen = s.card(id.as_str()).clone();
            assert_eq!(stolen.set_as.map(|set| set.reveal), Some(RevealAt::Cloaked));
            assert_eq!(stolen.controller, PlayerId::P1);
            assert_eq!(stolen.owner, PlayerId::P2);
            assert!(s.state().players.p2.locks.backrow[1]);
            let owners = view_json(&s, "p2")["opponent"]["backrow"][1].clone();
            assert_eq!(owners["faceDown"], true);
            assert!(owners.get("defId").is_none(), "{owners}");
        }

        /// R3777000: a Spell set by a lent Cloaked (M #99 Paranoia, R1044) is a backrow card too.
        #[test]
        fn r3777000_a_set_cloaked_spell_is_a_backrow_card_too() {
            let (mut s, id) = set_by_p2(SPARE, json!([{ "def": PARANOIA, "lane": 5 }]), json!(MAGIC_JAMMED));
            assert_eq!(s.card(id.as_str()).def_id, SPARE);
            assert!(listed(&s, &id));

            s.play(MAGIC_JAMMED, json!({ "targets": pick::id(&id) }));

            assert!(
                s.pile("p2", "graveyard")
                    .iter()
                    .any(|card| card.id == id && card.def_id == SPARE && card.set_as.is_none())
            );
            assert!(s.state().players.p2.locks.backrow[1]);
        }

        /// R3777000, R783006: revealed, a Cloaked Unit stands in a unit zone and is no backrow card.
        #[test]
        fn r3777000_a_cloaked_unit_revealed_into_a_unit_zone_is_no_backrow_target() {
            let mut s = scn(json!({
                "active": "p2",
                "p1": { "hand": [MAGIC_JAMMED, SPARE], "library": [SPARE, SPARE] },
                "p2": { "hand": [CRACKER, SPARE], "library": [SPARE, SPARE] },
            }));
            s.play(CRACKER, json!({ "zone": 2, "row": "backrow", "faceDown": "cloaked" }));
            let id = s.backrow("p2", 2).expect("the set Toe Cracker").id;
            s.activate(id.as_str(), json!({ "ability": jackioh_engine::alt_play::CLOAKED_REVEAL }));
            assert_eq!(s.unit("p2", 2).map(|unit| unit.id), Some(id.clone()));
            s.end_turn();

            assert!(!listed(&s, &id));
            s.expect_refused(|s| s.play(MAGIC_JAMMED, json!({ "targets": pick::id(&id) })));
            assert_eq!(s.unit("p2", 2).map(|unit| unit.id), Some(id));
        }

        /// R3777001: the targeter's view names the zone of the set card its play may target, by the id
        /// `legalActions` offers, and nothing else of it (R33, R177); no other view names it.
        #[test]
        fn r3777001_the_targeter_s_view_names_the_set_card_s_zone_and_nothing_of_it() {
            let (mut s, id) = set_by_p2(CRACKER, json!([]), json!(MAGIC_JAMMED));

            let view = view_json(&s, "p1");
            assert_eq!(view["opponent"]["backrow"][1], json!({ "faceDown": true, "cost": 2 }));
            assert_eq!(
                view["hiddenTargets"],
                json!([{ "instanceId": id, "zone": { "player": "p2", "row": "backrow", "lane": 2 } }])
            );
            assert!(!view.to_string().contains(CRACKER), "R33: p1 reads nothing of it");
            assert!(view_json(&s, "p2").get("hiddenTargets").is_none());

            s.play(MAGIC_JAMMED, json!({ "targets": pick::id(&id) }));

            assert!(view_json(&s, "p1").get("hiddenTargets").is_none());
        }

        #[test]
        fn bases_locked_zone_rejects_a_later_play_into_it_sec3_2() {
            let mut s = scn(json!({
                "p1": { "hand": [MAGIC_JAMMED, SPARE], "library": [SPARE, SPARE] },
                "p2": {
                    "backrow": [{ "def": MANA_WELL, "lane": 3 }],
                    "hand": [MANA_WELL, SPARE],
                    "library": [SPARE, SPARE],
                },
            }));
            let target = s.backrow("p2", 3).expect("Mana Well in p2's backrow lane 3");

            s.play(MAGIC_JAMMED, json!({ "targets": pick::id(&target.id) })).end_turn();

            s.expect_refused_with(|s| s.play(MANA_WELL, json!({ "zone": 3 })), "not open");
            s.play(MANA_WELL, json!({ "zone": 4 }));
            assert!(s.backrow("p2", 4).is_some());
        }

        #[test]
        fn radiant_steals_the_target_into_the_same_lane_and_locks_its_original_zone_r15_sec3_2() {
            let mut s = scn(json!({
                "p1": { "hand": [{ "def": MAGIC_JAMMED, "radiant": true }, SPARE], "library": [SPARE] },
                "p2": { "backrow": [{ "def": SHEEPISH, "lane": 2 }], "hand": [SPARE], "library": [SPARE] },
            }));
            let target = s.backrow("p2", 2).expect("Sheepish in p2's backrow lane 2");

            s.play(MAGIC_JAMMED, json!({ "targets": pick::id(&target.id) }));

            assert_eq!(s.backrow("p1", 2).map(|card| card.id), Some(target.id.clone()));
            assert!(s.backrow("p2", 2).is_none());
            assert_eq!(s.card(&target.id).controller, PlayerId::P1);
            // R12: control moved, ownership did not.
            assert_eq!(s.card(&target.id).owner, PlayerId::P2);
            assert!(s.state().players.p2.locks.backrow[1]);
            assert!(!s.state().players.p1.locks.backrow[1]);
            s.expect_in_zone(&target.id, "field");
        }

        #[test]
        fn radiant_steals_into_the_first_free_zone_when_the_same_lane_is_taken_r15() {
            let mut s = scn(json!({
                "p1": {
                    "hand": [{ "def": MAGIC_JAMMED, "radiant": true }, SPARE],
                    "backrow": [
                        { "def": MANA_WELL, "lane": 1 },
                        { "def": MANA_WELL, "lane": 2 },
                    ],
                    "library": [SPARE],
                },
                "p2": { "backrow": [{ "def": SHEEPISH, "lane": 2 }], "hand": [SPARE], "library": [SPARE] },
            }));
            let target = s.backrow("p2", 2).expect("Sheepish in p2's backrow lane 2");

            s.play(MAGIC_JAMMED, json!({ "targets": pick::id(&target.id) }));

            // Lane 2 is occupied on p1's side, so R15 falls through to the leftmost free zone.
            assert_eq!(s.backrow("p1", 3).map(|card| card.id), Some(target.id.clone()));
            assert!(s.state().players.p2.locks.backrow[1]);
        }

        #[test]
        fn r33_the_thief_reads_a_stolen_face_down_trap_and_its_owner_stops_reading_it() {
            let mut s = scn(json!({
                "p1": { "hand": [{ "def": MAGIC_JAMMED, "radiant": true }, SPARE], "library": [SPARE] },
                "p2": { "backrow": [{ "def": SHEEPISH, "lane": 2 }], "hand": [SPARE], "library": [SPARE] },
            }));
            let target = s.backrow("p2", 2).expect("Sheepish in p2's backrow lane 2");

            // Before: p2 controls it and reads it; p1 sees a face-down marker and nothing else (§10.8).
            assert_matches_object(
                &view_json(&s, "p2")["you"]["backrow"][1],
                &json!({ "faceDown": false, "defId": SHEEPISH }),
            );
            assert_eq!(view_json(&s, "p1")["opponent"]["backrow"][1], json!({ "faceDown": true, "cost": 1 }));

            s.play(MAGIC_JAMMED, json!({ "targets": pick::id(&target.id) }));

            // After: R33 keys readability on the CONTROLLER, so the thief reads it even though p2 owns it.
            assert_matches_object(
                &view_json(&s, "p1")["you"]["backrow"][1],
                &json!({
                    "faceDown": false,
                    "defId": SHEEPISH,
                    "owner": "p2",
                    "controller": "p1",
                }),
            );
            assert_eq!(view_json(&s, "p2")["opponent"]["backrow"][1], json!({ "faceDown": true, "cost": 1 }));
        }
    }
}
