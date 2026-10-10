//! #25 4-mana 7/7 (SPEC §8.2): a 7/7 → 14/14 Unit, base "Armor 7", radiant "Indestructible" (R275:
//! the Radiant body doubles, and the keyword is the stronger one). The Engine cell is "Keywords
//! only", so there is no script: both keywords are printed on the catalog faces and §10.4 layer 1
//! reads them from the def; granting them here would double the Armor, which sums across sources.
//!
//! Where the behaviour lives instead:
//!   Armor 7        — §4.4 step 2 subtracts the unit's total Armor from every incoming hit, so a
//!                    7-damage hit is reduced to 0 and the zero rule (R63) makes it a non-event.
//!   Indestructible — §4.4 step 4 (takes no damage), §4.5 step 1 and R46 (destroy marks are
//!                    ignored; a marked unit switches to Attack Position and loses Taunt for the
//!                    turn), R69 (it still dies if its max health falls to 0 or less). §6.1: "can be
//!                    exiled or sacrificed", so Tribute, Sacrifice and Exile still remove it.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-025";

pub fn script() -> CardScripts {
    CardScripts {
        base: Script::default(),
        radiant: Script::default(),
    }
}

// #25 4-mana 7/7 — SPEC §8.2, BUILD M4-T4 row 25: "Armor 7 zeroes a 7 hit; radiant 14/14 Armor 7,
// Reborn: it comes back once at 1 health, from combat or a Tribute, and an exile removes it for good".
// Both scripts are empty, so these fixtures prove the catalog faces' keywords do the work (§4.4, §4.5).
// The Tribute is #22 Carnivorous Cube (§6.3 Tribute, R428), the exile #34 Collateral Damage (both
// fixtures depend on those scripts too).
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BIG: &str = "core-025";
    const CUBE: &str = "core-022"; // Cry: Tribute one of your other Units (R428).
    const EXILER: &str = "core-034"; // Collateral Damage: exile target permanent, cost 3.
    const FILLER: &str = "core-005";

    fn unit_view_of(s: &Scenario, player: PlayerId, lane: usize) -> Option<jackioh_engine::wire::view::UnitView> {
        let view = s.view(P1);
        let side = if player == P1 { view.you } else { view.opponent };
        side.units.get(lane - 1).cloned().flatten()
    }

    /// A script with no hook, no declaration and no flag: every field of `Script` absent or empty.
    fn is_empty_script(script: &Script) -> bool {
        script.cost.is_none()
            && script.cry.is_none()
            && script.death.is_none()
            && script.start_of_game.is_none()
            && script.resume.is_empty()
            && script.delayed.is_none()
            && script.set_stat.is_none()
            && script.start_of_turn.is_none()
            && script.end_of_turn.is_none()
            && script.aura.is_none()
            && script.triggers.is_empty()
            && script.on_play_hook.is_none()
            && script.hand_triggers.is_empty()
            && script.static_flags.is_none()
            && script.targets.is_empty()
            && script.modes.is_empty()
            && script.condition_met.is_none()
            && script.preview.is_none()
            && script.activations.is_empty()
            && script.target_checks.is_empty()
            && script.cost_aura.is_none()
            && script.graveyard_play.is_none()
            && script.targeting_discards.is_none()
            && script.records_play_as.is_none()
            && script.draw_limit.is_none()
            && script.replacements.is_empty()
            && script.hero_guard.is_none()
            && script.conditional_keywords.is_none()
            && script.after_attack.is_none()
            && script.plague_multiplier.is_none()
            && script.deck_triggers.is_empty()
            && script.graveyard_triggers.is_empty()
            && script.quests.is_none()
            && script.tribute_when.is_none()
            && script.would_counter.is_none()
            && script.start_of_opponent_turn.is_none()
    }

    mod n25_4_mana_7_7 {
        use super::*;

        #[test]
        fn the_keywords_are_printed_on_the_catalog_faces_so_neither_script_grants_anything() {
            // §10.4: Armor sums across sources, so a script that re-granted Armor 7 would show 14.
            let def = crate::card_def(ID);
            assert_eq!(serde_json::to_value(&def.base.keywords).unwrap(), json!([{ "kind": "Armor", "n": 7 }]));
            assert_eq!(
                serde_json::to_value(&def.radiant.keywords).unwrap(),
                json!([{ "kind": "Armor", "n": 7 }, { "kind": "Reborn" }])
            );
            let scripts = script();
            assert!(is_empty_script(&scripts.base));
            assert!(is_empty_script(&scripts.radiant));
        }

        mod base {
            use super::*;

            #[test]
            fn s4_4_step_2_armor_7_zeroes_a_7_hit() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "big-armor",
                    "p1": { "hand": [FILLER], "field": [BIG], "library": [FILLER] },
                    "p2": { "hand": [FILLER], "field": [BIG], "library": [FILLER] },
                }));
                let (Some(mine), Some(theirs)) = (s.unit(P1, 1), s.unit(P2, 1)) else {
                    panic!("both 7/7s should be on the board");
                };

                s.end_turn(); // p2 attacks into p1.
                s.attack(&theirs, &mine);

                // §4.3 step 2 is simultaneous, and 7 − 7 is 0 on each side, which R63 makes a non-event.
                s.expect_stats(&mine, json!({ "attack": 7, "maxHealth": 7, "health": 7 }));
                s.expect_stats(&theirs, json!({ "attack": 7, "maxHealth": 7, "health": 7 }));
                assert_eq!(unit_view_of(&s, P1, 1).map(|unit| unit.armor), Some(7));
                // R63: a hit reduced to 0 emits nothing, so neither unit ever took a damage instance.
                let hits = s
                    .events()
                    .iter()
                    .filter(|event| {
                        matches!(event, GameEvent::Damage { target_id, .. }
                            if *target_id == mine.id || *target_id == theirs.id)
                    })
                    .count();
                assert_eq!(hits, 0);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r275_the_radiant_face_is_a_14_14_with_armor_7_and_reborn() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "big-radiant-face",
                    "p1": { "hand": [FILLER], "field": [{ "def": BIG, "radiant": true }] },
                    "p2": { "hand": [FILLER] },
                }));

                s.expect_stats(BIG, json!({ "attack": 14, "health": 14, "maxHealth": 14 }));
                assert_eq!(
                    unit_view_of(&s, P1, 1).map(|unit| serde_json::to_value(&unit.keywords).unwrap()),
                    Some(json!([{ "kind": "Armor", "n": 7 }, { "kind": "Reborn" }]))
                );
                assert_eq!(unit_view_of(&s, P1, 1).map(|unit| unit.armor), Some(7));
            }

            #[test]
            fn s4_4_step_2_its_armor_7_zeroes_a_7_hit_and_its_14_back_kills_a_base_7_7_through_that_one_s_armor() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "big-radiant-armor",
                    "p1": { "hand": [FILLER], "field": [{ "def": BIG, "radiant": true }] },
                    "p2": { "hand": [FILLER], "field": [BIG] },
                }));
                let (Some(mine), Some(theirs)) = (s.unit(P1, 1), s.unit(P2, 1)) else {
                    panic!("both 7/7s should be on the board");
                };

                s.end_turn();
                s.attack(&theirs, &mine);

                s.expect_stats(&mine, json!({ "health": 14, "maxHealth": 14 }));
                s.expect_in_zone(&theirs, "graveyard");
            }

            #[test]
            fn s4_5_step_4_reborn_brings_it_back_once_at_1_health_and_without_reborn_when_it_dies_in_combat() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "big-radiant-reborn",
                    "active": "p2",
                    "p1": { "hand": [FILLER], "field": [{ "def": BIG, "radiant": true, "damage": 10 }], "library": [FILLER] },
                    // A radiant 18/18 Midrange Menace hits for 18: 11 through the Armor, which kills the 4 left.
                    "p2": { "hand": [FILLER], "field": [{ "def": "core-019", "radiant": true }], "library": [FILLER] },
                }));
                let big = s.card(BIG).clone();

                s.attack("core-019", &big);

                s.expect_in_zone(&big, "field");
                s.expect_stats(&big, json!({ "health": 1, "maxHealth": 14 }));
                assert_eq!(
                    unit_view_of(&s, P1, 1).map(|unit| serde_json::to_value(&unit.keywords).unwrap()),
                    Some(json!([{ "kind": "Armor", "n": 7 }]))
                );
                s.expect_events(json!(["destroyed", "summoned"]));
            }

            #[test]
            fn s6_3_a_tribute_is_a_death_too_so_reborn_brings_it_back_from_that_as_well() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "big-sacrifice",
                    "p1": { "hand": [CUBE, FILLER], "field": [{ "def": BIG, "radiant": true }] },
                    "p2": { "hand": [FILLER] },
                }));
                let big = s.card(BIG).clone();
                s.play(CUBE, json!({ "targets": [{ "pick": "instance", "instanceId": big.id }] }));

                // §6.3 Sacrifice counts as a death, and §6.1 Reborn answers it (R64).
                s.expect_events(json!(["destroyed", "summoned"]));
                s.expect_in_zone(&big, "field");
                s.expect_stats(&big, json!({ "health": 1 }));
            }

            #[test]
            fn s6_1_s6_3_an_exile_removes_it_for_good_no_death_so_no_reborn() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "big-exile",
                    "p1": { "hand": [EXILER, FILLER], "field": ["core-008"] },
                    "p2": { "hand": [FILLER], "field": [{ "def": BIG, "radiant": true }] },
                }));
                let big = s.card(BIG).clone();
                s.play(EXILER, json!({ "targets": [{ "pick": "instance", "instanceId": big.id }] }));

                // §6.3 Exile: from anywhere to the exile pile, with no Death trigger (R55 counts it).
                s.expect_in_zone(&big, "exile");
                assert!(!s.pile(P2, "graveyard").iter().any(|card| card.def_id == BIG));
                assert!(s.unit(P2, 1).is_none());
            }
        }
    }
}
