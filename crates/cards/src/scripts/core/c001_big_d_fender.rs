//! SPEC §8.1 #1 Big D-fender — 0/8 → 0/16 Unit, Human, cost 2.
//! Base: "Aura: your units in Defense Position have +2 Armor". The radiant cell is "+4 Armor", and
//! per §8's Conventions a cell that changes only a number changes only that number, so the radiant
//! form is the same aura at 4.
//!
//! Engine cell: an aura layer keyed on `position == DEF` (§10.4 layer 5); 0 attack, so R7 stops it
//! from ever declaring an attack — that is the combat validator reading the printed stats, nothing
//! for this file to script. Armor sums across every source (§10.4), so a Defense-Position ally ends
//! up with this aura's 2 on top of Defense Position's own +1 (§4.1) and takes 3 less (BUILD M4-T4).

use jackioh_engine::prelude::*;

pub const ID: &str = "core-001";

/// Armor +n on the controller's own units that are on the field in Defense Position (§4.1, §10.4).
/// "Your units" is the controller's side, so an enemy unit in Defense Position gets nothing, and the
/// aura reads the position off the instance on every read rather than storing a total. n is the
/// declared number `armor` (R386): 2, 4 on the Radiant face.
fn defense_armor_aura() -> AuraHook {
    aura_hook(|args: HookArgs<'_>| {
        let n = param(&args, "armor");
        let controller = args.self_.controller;
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| {
                unit.controller == controller
                    && matches!(unit.zone, Zone::Field { row: Row::Units, .. })
                    && unit.position.unwrap_or(Position::Atk) == Position::Def
            }),
            mod_: StatMod {
                keywords: Some(vec![Keyword::Armor { n }]),
                ..StatMod::default()
            },
        }]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        aura: Some(defense_armor_aura()),
        ..Script::default()
    };
    // The same aura: the Radiant face's 4 is its declared `armor`, read off the face that is up.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// BUILD M4-T4 row 1: "DEF ally takes 3 less (1 position + 2 aura), ATK ally unaffected; it never
// attacks; radiant +4". The aura shows only in damage (§4.4 step 2 subtracts total Armor), so each
// case is one combat and one health assertion: #25 (7/7, Armor 7) attacks #19 (9/9, Taunt, so the
// attack must go to it, §4.2 step 3).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    mod n1_big_d_fender {
        use super::*;

        #[test]
        fn a_defense_position_ally_takes_3_less_1_from_the_position_s4_1_and_2_from_the_aura() {
            crate::register_all();
            scenario(json!({
                "active": "p2",
                "p1": { "field": ["core-001", { "def": "core-019", "position": "DEF" }] },
                "p2": { "field": ["core-025"] }
            }))
            .attack("core-025", "core-019")
            .expect_stats("core-019", json!({ "health": 5, "maxHealth": 9 }));
        }

        #[test]
        fn an_attack_position_ally_is_unaffected() {
            crate::register_all();
            scenario(json!({
                "active": "p2",
                "p1": { "field": ["core-001", "core-019"] },
                "p2": { "field": ["core-025"] }
            }))
            .attack("core-025", "core-019")
            .expect_stats("core-019", json!({ "health": 2, "maxHealth": 9 }));
        }

        #[test]
        fn radiant_gives_4_armor_instead_so_a_defense_position_ally_takes_5_less() {
            crate::register_all();
            scenario(json!({
                "active": "p2",
                "p1": {
                    "field": [{ "def": "core-001", "radiant": true }, { "def": "core-019", "position": "DEF" }]
                },
                "p2": { "field": ["core-025"] }
            }))
            .attack("core-025", "core-019")
            .expect_stats("core-019", json!({ "health": 7, "maxHealth": 9 }));
        }

        #[test]
        fn r7_never_attacks_a_0_attack_unit_cannot_declare_an_attack_on_a_unit_or_the_hero() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "field": ["core-001", "core-019"] },
                "p2": { "field": ["core-025"] }
            }));
            s.expect_refused(|s| s.attack("core-001", "hero"));
            s.expect_refused(|s| s.attack("core-001", "core-025"));
            // The board is untouched: no exertion was spent and nothing was damaged.
            s.expect_stats("core-025", json!({ "health": 7 }));
        }

        #[test]
        fn the_aura_is_your_units_an_enemy_unit_in_defense_position_gets_nothing() {
            crate::register_all();
            scenario(json!({
                "p1": { "field": ["core-001", "core-019"] },
                "p2": { "field": [{ "def": "core-025", "position": "DEF" }] }
            }))
            .attack("core-019", "core-025")
            // 9 attack − (7 printed Armor + 1 Defense Position) = 1. If the aura reached across the
            // board it would be 9 − 10 = 0 and the enemy would be untouched.
            .expect_stats("core-025", json!({ "health": 6, "maxHealth": 7 }));
        }

        #[test]
        fn r386_an_upgrade_moves_the_armor_to_3_and_a_degrade_to_1_and_the_aura_gives_that() {
            for (upgrade, armor, health) in [(true, 3, 6), (false, 1, 4)] {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "field": ["core-001", { "def": "core-019", "position": "DEF" }] },
                    "p2": { "field": ["core-025"] }
                }));
                let moved = if upgrade {
                    crate::upgrade_number(&mut s, "core-001", "armor")
                } else {
                    crate::degrade_number(&mut s, "core-001", "armor")
                };
                assert_eq!(moved, armor);
                // 7 attack − (1 Defense Position + `armor`).
                s.attack("core-025", "core-019").expect_stats("core-019", json!({ "health": health }));
            }
        }
    }
}
