//! #45 Deft Duelist (SPEC §8.2). Unit 4/3 → 8/6, Human, cost 2, Rare.
//!   Base:    "Charge, Deft"
//!   Radiant: "Charge, Armor 1, Deft" — §8 Conventions: a keyword cell without "Plus" gives the
//!            radiant face's COMPLETE keyword list.
//!
//! Charge, Armor 1 and Deft are PRINTED keywords: `catalog.json` carries them on `base.keywords`
//! and `radiant.keywords`, and §10.4 layer 1 reads them off the def. Granting any of them here
//! would be a second source — for Armor it would literally double it, because §10.4 sums Armor
//! across sources. So both scripts are empty.
//!
//! R49 (two exertions: one attack plus one switch in a turn) is the Deft keyword, which
//! `combat.ts` reads through §10.4's layers, so a granted Deft works the same as the printed one.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-045";

pub fn script() -> CardScripts {
    let base = Script::default();
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// #45 Deft Duelist (SPEC §8.2, §4.1, §4.2, §4.4; R6, R49, R63).
//
// The must-pass row (BUILD M4-T4 #45): "Charge; attack then switch and switch then attack in one
// turn (R49); radiant Armor 1."
//
// R6 fixes what "switch then attack" has to mean: "attacking from Defense is not allowed and
// switching to Attack spends the turn's exertion — #45 is the exception". So the second order is a
// unit that starts in Defense, switches to Attack and then attacks: for every other unit the switch
// has spent the turn, and each control case below proves that on #20 Pointmaster.
//
// Charge and Armor 1 are printed keywords, so the tests read them through their effect (a summon
// turn attack; a 1-damage hit reduced to nothing) rather than off the def.
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    /// TS's harness registered the real catalog and every script on import (`registerAll()`); the
    /// Rust testkit cannot name this crate, so the scenario builder registers first.
    fn scenario(opts: Value) -> Scenario {
        crate::register_all();
        jackioh_engine::testkit::scenario(opts)
    }

    /// TS's `def` (`cardDef("core-045")`): the catalog card this file scripts.
    fn def() -> CardDef {
        crate::register_all();
        crate::card_def(ID)
    }

    /// Deft Duelist in hand, an enemy hero to hit; #21 Hinder keeps the turn alive (R82). TS flagged a
    /// radiant hand card after the build; the setup entry's own `radiant` sets the same flag on the
    /// same freshly created hand card.
    fn from_hand(radiant_duelist: bool) -> Scenario {
        scenario(json!({
            "seed": "deft-duelist",
            "p1": { "hand": [{ "def": "core-045", "radiant": radiant_duelist }, "core-021"] },
            "p2": { "health": 30 },
        }))
    }

    /// Already on the field in Defense Position, so the "switch then attack" order can be taken.
    fn in_defense(def: &str, radiant_unit: bool) -> Scenario {
        scenario(json!({
            "seed": "deft-duelist",
            "p1": {
                "hand": ["core-021"],
                "field": [{ "def": def, "radiant": radiant_unit, "position": "DEF", "lane": 1 }],
            },
            "p2": { "health": 30 },
        }))
    }

    fn position_at(s: &Scenario, player: PlayerId, lane: i32) -> Option<Position> {
        s.unit(player, lane).and_then(|card| card.position)
    }

    /// TS's `staticFlags?.deftDuelist`: the legacy flag (read nowhere, not ported, SURFACE §7.2) is
    /// absent from a face's static flags as they are written out.
    fn has_deft_duelist_flag(face: &Script) -> bool {
        serde_json::to_value(face.flags()).unwrap().get("deftDuelist").is_some()
    }

    /// "#45 Deft Duelist — base"
    mod base {
        use super::*;

        /// "R49 the two exertions are the Deft keyword on both faces, which `combat.ts` reads"
        #[test]
        fn r49_the_two_exertions_are_the_deft_keyword_on_both_faces_which_combat_reads() {
            let scripts = script();
            assert!(!has_deft_duelist_flag(&scripts.base));
            assert!(!has_deft_duelist_flag(&scripts.radiant));
            let def = def();
            assert_eq!(
                serde_json::to_value(&def.base.keywords).unwrap(),
                json!([{ "kind": "Charge" }, { "kind": "Deft" }]),
            );
            assert_eq!(
                serde_json::to_value(&def.radiant.keywords).unwrap(),
                json!([{ "kind": "Charge" }, { "kind": "Armor", "n": 1 }, { "kind": "Deft" }]),
            );
        }

        /// "§6.1 Charge lets it attack a unit on its summon turn"
        #[test]
        fn s6_1_charge_lets_it_attack_a_unit_on_its_summon_turn() {
            let mut s = scenario(json!({
                "seed": "deft-duelist",
                "p1": { "hand": ["core-045", "core-021"] },
                "p2": { "field": [{ "def": "core-t-felinor", "lane": 1 }], "health": 30 },
            }));
            s.play("core-045", json!({ "zone": 1 }));
            s.attack("core-045", "core-t-felinor");

            // 4 into a 1/1 kills it; the 1 back leaves the 4/3 at 2 health.
            assert!(s.unit(PlayerId::P2, 1).is_none());
            s.expect_stats("core-045", json!({ "attack": 4, "health": 2, "maxHealth": 3 }));
        }

        /// "§6.1 Charge lets it attack the enemy hero on its summon turn"
        #[test]
        fn s6_1_charge_lets_it_attack_the_enemy_hero_on_its_summon_turn() {
            let mut s = from_hand(false);
            s.play("core-045", json!({ "zone": 1 }));
            s.attack("core-045", "hero");

            s.expect_health(PlayerId::P2, 26);
        }

        /// "R49 attacks and then switches position in the same turn"
        #[test]
        fn r49_attacks_and_then_switches_position_in_the_same_turn() {
            let mut s = from_hand(false);
            s.play("core-045", json!({ "zone": 1 }));
            s.attack("core-045", "hero");
            s.switch_position("core-045");

            s.expect_health(PlayerId::P2, 26);
            assert_eq!(position_at(&s, PlayerId::P1, 1), Some(Position::Def));
            s.expect_events(json!(["cardPlayed", "attackDeclared", "positionSwitched"]));
        }

        /// "R49 switches position and then attacks in the same turn, which R6 forbids every other unit"
        #[test]
        fn r49_r6_switches_position_and_then_attacks_in_the_same_turn_which_r6_forbids_every_other_unit() {
            let mut s = in_defense("core-045", false);
            s.switch_position("core-045");
            s.attack("core-045", "hero");

            assert_eq!(position_at(&s, PlayerId::P1, 1), Some(Position::Atk));
            s.expect_health(PlayerId::P2, 26);
            s.expect_events(json!(["positionSwitched", "attackDeclared"]));
        }

        /// "R6 a plain unit that switched to Attack Position has spent its exertion and cannot attack"
        #[test]
        fn r6_a_plain_unit_that_switched_to_attack_position_has_spent_its_exertion_and_cannot_attack() {
            let mut s = in_defense("core-020", false);
            s.switch_position("core-020");

            s.expect_refused_with(|s| s.attack("core-020", "hero"), "already acted");
            s.expect_health(PlayerId::P2, 30);
        }

        /// "§4.1 a plain unit that attacked cannot then switch: one exertion, not two"
        #[test]
        fn s4_1_a_plain_unit_that_attacked_cannot_then_switch_one_exertion_not_two() {
            let mut s = scenario(json!({
                "seed": "deft-duelist",
                "p1": { "hand": ["core-021"], "field": [{ "def": "core-020", "lane": 1 }] },
                "p2": { "health": 30 },
            }));
            s.attack("core-020", "hero");

            s.expect_health(PlayerId::P2, 23);
            s.expect_refused_with(|s| s.switch_position("core-020"), "already acted");
        }

        /// "R49 is one attack and one switch, not two of either"
        #[test]
        fn r49_is_one_attack_and_one_switch_not_two_of_either() {
            let mut s = from_hand(false);
            s.play("core-045", json!({ "zone": 1 }));
            s.attack("core-045", "hero");
            s.switch_position("core-045");

            s.expect_refused_with(|s| s.attack("core-045", "hero"), "already acted");
            s.expect_refused_with(|s| s.switch_position("core-045"), "already acted");
            s.expect_health(PlayerId::P2, 26);
        }

        /// "§8.2 the base stats are 4/3"
        #[test]
        fn s8_2_the_base_stats_are_4_3() {
            let mut s = from_hand(false);
            s.play("core-045", json!({ "zone": 1 }));

            s.expect_stats("core-045", json!({ "attack": 4, "health": 3, "maxHealth": 3 }));
        }
    }

    /// "#45 Deft Duelist — radiant"
    mod radiant {
        use super::*;

        /// "§8.2 the radiant stats are 8/6 and Charge still lets it hit the hero on its summon turn"
        #[test]
        fn s8_2_the_radiant_stats_are_8_6_and_charge_still_lets_it_hit_the_hero_on_its_summon_turn() {
            let mut s = from_hand(true);
            s.play("core-045", json!({ "zone": 1 }));
            s.attack("core-045", "hero");

            s.expect_stats("core-045", json!({ "attack": 8, "health": 6, "maxHealth": 6 }));
            s.expect_health(PlayerId::P2, 22);
        }

        /// "R49 'same': the radiant face also attacks and then switches in one turn"
        #[test]
        fn r49_same_the_radiant_face_also_attacks_and_then_switches_in_one_turn() {
            let mut s = from_hand(true);
            s.play("core-045", json!({ "zone": 1 }));
            s.attack("core-045", "hero");
            s.switch_position("core-045");

            assert_eq!(position_at(&s, PlayerId::P1, 1), Some(Position::Def));
            s.expect_health(PlayerId::P2, 22);
        }

        /// "R49 'same': and switches and then attacks in one turn"
        #[test]
        fn r49_same_and_switches_and_then_attacks_in_one_turn() {
            let mut s = in_defense("core-045", true);
            s.switch_position("core-045");
            s.attack("core-045", "hero");

            assert_eq!(position_at(&s, PlayerId::P1, 1), Some(Position::Atk));
            s.expect_health(PlayerId::P2, 22);
        }

        /// "R63 Armor 1 reduces an incoming 1-damage hit to nothing, so no damage lands"
        #[test]
        fn r63_armor_1_reduces_an_incoming_1_damage_hit_to_nothing_so_no_damage_lands() {
            let mut s = scenario(json!({
                "seed": "deft-duelist",
                "p1": {
                    "hand": ["core-021"],
                    "field": [{ "def": "core-045", "radiant": true, "lane": 1 }],
                    "health": 30,
                },
                "p2": {
                    "hand": ["core-021"],
                    "field": [{ "def": "core-t-felinor", "lane": 1 }],
                    "health": 30,
                },
            }));
            s.end_turn();
            s.attack("core-t-felinor", "core-045");

            // 1 − Armor 1 = 0, and R63 makes a hit that is 0 after Armor a non-event.
            s.expect_stats("core-045", json!({ "attack": 8, "health": 6, "maxHealth": 6 }));
            // The 8 it strikes back with kills the 1/1 token, which ceases to exist (R11).
            assert!(s.unit(PlayerId::P2, 1).is_none());
        }
    }
}
