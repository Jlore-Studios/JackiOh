//! SPEC §8.1 #1 Big D-fender — 0/8 → 0/16 Unit, Human, cost 2.
//! Base: "Aura: your units in Defense Position have +2 Armor". The radiant cell is "+4 Armor", and
//! per §8's Conventions a cell that changes only a number changes only that number, so the radiant
//! form is the same aura at 4.
//!
//! Engine cell: an aura layer keyed on `position == DEF` (§10.4 layer 5); 0 attack, so R7 stops it
//! from ever declaring an attack — that is the combat validator reading the printed stats, nothing
//! for this file to script. Armor sums across every source (§10.4), so a Defense-Position ally ends
//! up with this aura's 2 on top of Defense Position's own +1 (§4.1) and takes 3 less (BUILD M4-T4).

use std::sync::Arc;

use jackioh_engine::prelude::*;

pub const ID: &str = "core-001";

/// Armor +n on the controller's own units that are on the field in Defense Position (§4.1, §10.4).
/// "Your units" is the controller's side, so an enemy unit in Defense Position gets nothing, and the
/// aura reads the position off the instance on every read rather than storing a total.
fn defense_armor_aura(n: i32) -> AuraHook {
    Arc::new(move |args: &AuraArgs| {
        let controller = args.self_.controller;
        vec![AuraEntry {
            applies: Arc::new(move |unit: &CardInstance| {
                unit.controller == controller
                    && matches!(unit.zone, Zone::Field { row: Row::Units, .. })
                    && unit.position.unwrap_or(Position::Atk) == Position::Def
            }),
            mod_: json_as(json!({ "keywords": [{ "kind": "Armor", "n": n }] })),
        }]
    })
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script { aura: Some(defense_armor_aura(2)), ..Script::default() },
        radiant: Script { aura: Some(defense_armor_aura(4)), ..Script::default() },
    }
}

// SPEC §8.1 #1 Big D-fender. BUILD M4-T4 row 1: "DEF ally takes 3 less (1 position + 2 aura), ATK
// ally unaffected; it never attacks; radiant +4".
//
// The aura is only observable through the damage pipeline (§4.4 step 2 subtracts total Armor), so
// every case here is one combat and one health assertion.
//   attacker  #25 "4-mana 7/7" — 7 attack, 7 health, printed Armor 7, no hooks at all. Its own
//             Armor keeps it alive through the retaliation, so nothing else moves.
//   ally      #19 Midrange Menace — 9/9 with printed Taunt, so the attack MUST go to it (§4.2
//             step 3) whatever position it is in, and 9 health is enough to survive every case.
// 7 damage less (1 Defense Position + 2 aura) = 4, so a 9/9 ends at 5; without the aura, 2; with
// the radiant aura's 4, 7.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;
    use serde_json::json;

    mod c1_big_d_fender {
        use super::*;

        #[test]
        fn a_defense_position_ally_takes_3_less_1_from_the_position_4_1_and_2_from_the_aura() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "field": ["core-001", { "def": "core-019", "position": "DEF" }] },
                "p2": { "field": ["core-025"] }
            }));
            s.attack("core-025", "core-019");
            s.expect_stats("core-019", json!({ "health": 5, "maxHealth": 9 }));
        }

        #[test]
        fn an_attack_position_ally_is_unaffected() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "field": ["core-001", "core-019"] },
                "p2": { "field": ["core-025"] }
            }));
            s.attack("core-025", "core-019");
            s.expect_stats("core-019", json!({ "health": 2, "maxHealth": 9 }));
        }

        #[test]
        fn radiant_gives_4_armor_instead_so_a_defense_position_ally_takes_5_less() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": {
                    "field": [{ "def": "core-001", "radiant": true }, { "def": "core-019", "position": "DEF" }]
                },
                "p2": { "field": ["core-025"] }
            }));
            s.attack("core-025", "core-019");
            s.expect_stats("core-019", json!({ "health": 7, "maxHealth": 9 }));
        }

        #[test]
        fn r7_never_attacks_a_0_attack_unit_cannot_declare_an_attack_on_a_unit_or_the_hero() {
            let mut s = scenario(json!({
                "p1": { "field": ["core-001", "core-019"] },
                "p2": { "field": ["core-025"] }
            }));
            s.expect_refused(|s| {
                s.attack("core-001", "hero");
            });
            s.expect_refused(|s| {
                s.attack("core-001", "core-025");
            });
            // The board is untouched: no exertion was spent and nothing was damaged.
            s.expect_stats("core-025", json!({ "health": 7 }));
        }

        #[test]
        fn the_aura_is_your_units_an_enemy_unit_in_defense_position_gets_nothing() {
            let mut s = scenario(json!({
                "p1": { "field": ["core-001", "core-019"] },
                "p2": { "field": [{ "def": "core-025", "position": "DEF" }] }
            }));
            s.attack("core-019", "core-025");
            // 9 attack − (7 printed Armor + 1 Defense Position) = 1. If the aura reached across the
            // board it would be 9 − 10 = 0 and the enemy would be untouched.
            s.expect_stats("core-025", json!({ "health": 6, "maxHealth": 7 }));
        }
    }
}
