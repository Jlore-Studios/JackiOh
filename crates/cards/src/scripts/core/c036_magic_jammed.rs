//! #36 Magic Jammed (SPEC §8.2): "Destroy target backrow card; Lock its zone", radiant "Steal
//! target backrow card; Lock its original zone". The radiant cell restates BOTH base clauses, so it
//! replaces both and nothing base-only survives (§8 Conventions).
//!
//! The target is declared, so it travels in the `play` action and resolution never pauses (R81).
//! Unnarrowed means either side: an ALLY backrow card is a legal pick (unlike #49), and stealing it
//! does nothing (`steal` refuses your own card, R76) while the Lock still lands (§3.2).
//!
//! EFFECT ORDER is load-bearing: `lock` reads the zone where the chosen card sits now and `steal`
//! MOVES it (R15), so the lock runs FIRST. `destroy` only marks the card (§4.5), so the base order
//! is not load-bearing. R33's visibility needs no line here: `viewFor` keys it on `controller`.

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

// #36 Magic Jammed (SPEC §8.2, BUILD M4-T4): "Destroy backrow and lock zone, locked zone rejects
// play; radiant steals into same-lane zone else first free, original zone locked, trap identity
// visible to thief (R33)."
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    const MAGIC_JAMMED: &str = "core-036";
    /// A Field Spell: a public backrow card (§3.2).
    const MANA_WELL: &str = "core-006";
    /// A Trap: face-down in the backrow, so R33 has something to hide (§3.2).
    const SHEEPISH: &str = "core-041";
    /// A spare hand card, so no side runs out of meaningful actions and auto-ends its turn.
    const SPARE: &str = "core-005";

    /// Registers the catalog first: the engine's testkit cannot name the cards crate.
    fn scn(opts: Value) -> Scenario {
        crate::register_all();
        scenario(opts)
    }

    fn pick(instance_id: &str) -> Value {
        json!([{ "pick": "instance", "instanceId": instance_id }])
    }

    fn view_json(s: &Scenario, seat: &str) -> Value {
        serde_json::to_value(s.view(seat)).expect("a view is JSON")
    }

    /// Every key of `expected` is in `actual` with that value.
    fn assert_matches_object(actual: &Value, expected: &Value) {
        let expected = expected.as_object().expect("toMatchObject takes an object");
        for (key, value) in expected {
            assert_eq!(&actual[key.as_str()], value, "key {key} of {actual}");
        }
    }

    mod magic_jammed {
        use super::*;

        #[test]
        fn is_sec8_2s_36_a_1_cost_spell() {
            crate::register_all();
            let def = crate::card_def(ID);
            assert_eq!(def.index, "36");
            assert_eq!(def.type_, CardType::Spell);
            assert_eq!(def.cost, CardCost::Fixed(1));
        }

        #[test]
        fn r81_declares_its_backrow_target_as_a_play_time_choice_unnarrowed_by_side() {
            // §8 Conventions: "target" is either side unless the cell narrows it, and neither cell does.
            let scripts = script();
            for face in [&scripts.base, &scripts.radiant] {
                assert_eq!(
                    serde_json::to_value(&face.targets).expect("declarations are JSON"),
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

            s.play(MAGIC_JAMMED, json!({ "targets": pick(&target.id) }));

            s.expect_in_zone(&target, "graveyard");
            assert!(s.backrow("p2", 3).is_none());
            // §3.2: the lock lives on the zone and outlives its occupant.
            assert!(s.state().players.p2.locks.backrow[2]);
            // R81: the target travelled with the play, so resolution never paused.
            assert!(s.state().pending.is_none());
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

            s.play(MAGIC_JAMMED, json!({ "targets": pick(&target.id) })).end_turn();

            // p2 is active now and holds a Field Spell; lane 3 is the zone Magic Jammed locked.
            s.expect_refused_with(|s| s.play(MANA_WELL, json!({ "zone": 3 })), "not open");
            // Every other backrow zone still takes it.
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

            s.play(MAGIC_JAMMED, json!({ "targets": pick(&target.id) }));

            // R15: the same lane on the thief's side, because it was free.
            assert_eq!(s.backrow("p1", 2).map(|card| card.id), Some(target.id.clone()));
            assert!(s.backrow("p2", 2).is_none());
            assert_eq!(s.card(&target.id).controller, PlayerId::P1);
            // R12: control moved, ownership did not.
            assert_eq!(s.card(&target.id).owner, PlayerId::P2);
            // "Lock its original zone": the zone it came from, not the one it landed in.
            assert!(s.state().players.p2.locks.backrow[1]);
            assert!(!s.state().players.p1.locks.backrow[1]);
            // Steal, not destroy: the card is still on the field.
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

            s.play(MAGIC_JAMMED, json!({ "targets": pick(&target.id) }));

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

            s.play(MAGIC_JAMMED, json!({ "targets": pick(&target.id) }));

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
