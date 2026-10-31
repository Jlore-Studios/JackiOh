//! SPEC §8.11 Training Grounds #44 Legacy: Duplicating Felinors,
//! the Legacy card of Core #12 Duplicating Felinors (`core-012`), a card of the held Training Grounds set (R1420,
//! R2194000), built by #2195.
//!
//! The code is a copy of the Core card's at the foundation's commit, not a re-export, so a later Core
//! patch does not move it (R2195003); it is named `Legacy: <the Core card's name>`, tagged Legacy
//! (R2511000), and is its own card, which a deck holds beside the Core card (R2511001). The Core
//! file's notes, as they stood:
//!
//! #12 Duplicating Felinors (SPEC §8.1): 3/4 → 6/8 Unit, Felinor, cost 2, Rare. Base "Cry: summon a
//! copy of this unit"; the radiant cell says "Same", so the radiant face runs the same text (§8
//! Conventions) on the 6/8 body, and its copy is a Radiant 6/8 too (R275). §8's Engine cell: "Copy
//! per R57, placed per R64; the copy's Cry does not fire (§6.2)".
//!
//! R57: the copy keeps the radiant flag, buffs, granted keywords, Vanilla state and `statsOverride`
//! and resets damage, exertion and counters; as a new instance it takes the current turn as its
//! `summonedTurn` (§4.1). R64: with no zone named it takes the leftmost free zone and fizzles
//! silently when the row has none. R1 / §6.2: copies fire no Cry, so the chain ends after one copy.
//!
//! The clone is the engine's `summon_copy` (engine/src/effects/summon.rs). A card file may not
//! rebuild buffs, keywords and Vanilla state off the instance (CLAUDE.md rule 5), hence a verb.

use jackioh_engine::effects::summon_copy;
use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-044";

/// "A copy of this unit": the Cry's own instance is the source, so `{ of: "self" }` names it without
/// the card ever touching state. Both faces run this — the radiant cell is "Same".
fn cry() -> Hook {
    hook(|_ctx| vec![summon_copy(json_as(json!({ "of": { "of": "self" }, "player": "self" })))])
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(cry()),
        ..Script::default()
    };
    let radiant = Script {
        cry: Some(cry()),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// #12 Duplicating Felinors (SPEC §8.1, BUILD M4-T4 row 12): "Copy lands in the leftmost free zone
// (R64), its Cry does not fire, buffs copied, damage not (R57); board full → no copy".
//
// Radiant (R275): the cell is "Same", so the copy keeps the radiant flag (R57): a Radiant 6/8 (6/9 before
// patch v0.3.10).
//
// HARNESS GAP: layer-4 buffs and damage cannot be seeded, nor reached through play (a Cry fires the
// instant the unit enters). R78 resets both on leaving the field and the play path resets neither on
// entering, so seeding the hand instance is the one route to "buffs copied, damage not".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    /// The controller's unit row as def ids, lane 1 to 5, with `None` for an empty zone.
    fn row(s: &Scenario, player: PlayerId) -> Vec<Option<String>> {
        (1..=5).map(|lane| s.unit(player, lane).map(|unit| unit.def_id)).collect()
    }

    fn lane(s: &Scenario, player: PlayerId, at: i32) -> CardInstance {
        match s.unit(player, at) {
            Some(unit) => unit,
            None => panic!("no unit in {player} lane {at}"),
        }
    }

    /// `["core-008", null, …]` as the row's Rust shape.
    fn ids(row: &[Option<&str>]) -> Vec<Option<String>> {
        row.iter().map(|id| id.map(str::to_string)).collect()
    }

    /// HARNESS GAP (see the header): layer-4 buffs and damage, seeded on the hand instance.
    fn seed_history(s: &mut Scenario, id: &str, attack: i32, health: i32, damage: i32) {
        match find_instance_mut(s.state_mut(), id) {
            Some(felinors) => {
                felinors.buffs.attack = attack;
                felinors.buffs.health = health;
                felinors.damage = damage;
            }
            None => panic!("no instance {id}"),
        }
    }

    mod n12_duplicating_felinors {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r64_puts_the_copy_in_the_leftmost_free_unit_zone_not_the_lane_next_to_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    // Lanes 1 and 3 are taken: the Felinors takes 2 and its copy 4, "leftmost free",
                    // never "adjacent" (§3.1 lanes).
                    "p1": {
                        "field": [
                            { "def": "core-008", "lane": 1 },
                            { "def": "core-020", "lane": 3 }
                        ],
                        "hand": ["traininggrounds-044", "core-005"],
                        "library": ["core-005"]
                    }
                }));

                s.play("traininggrounds-044", json!({}));

                assert_eq!(
                    row(&s, PlayerId::P1),
                    ids(&[Some("core-008"), Some("traininggrounds-044"), Some("core-020"), Some("traininggrounds-044"), None])
                );
            }

            #[test]
            fn r1_the_copy_s_cry_does_not_fire_so_exactly_one_copy_is_made() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": ["traininggrounds-044", "core-005"], "library": ["core-005"] } }));

                s.play("traininggrounds-044", json!({}));

                // Two bodies, not a full board: the copy is a summon, and a summon fires no Cry (§6.2).
                assert_eq!(
                    row(&s, PlayerId::P1),
                    ids(&[Some("traininggrounds-044"), Some("traininggrounds-044"), None, None, None])
                );
            }

            #[test]
            fn r57_copies_buffs_and_does_not_copy_damage_exertion_or_counters() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": ["traininggrounds-044", "core-005"], "library": ["core-005"] } }));
                // HARNESS GAP (see the header).
                let felinors = s.card("traininggrounds-044").id.clone();
                seed_history(&mut s, &felinors, 2, 2, 3);

                s.play("traininggrounds-044", json!({}));

                // The original: printed 3/4 plus the +2/+2 buff, three damage still on it.
                let original = lane(&s, PlayerId::P1, 1);
                s.expect_stats(&original, json!({ "attack": 5, "health": 3, "maxHealth": 6 }));
                // The copy: the same buff, no damage.
                let copy = lane(&s, PlayerId::P1, 2);
                s.expect_stats(&copy, json!({ "attack": 5, "health": 6, "maxHealth": 6 }));
                assert_eq!(copy.damage, 0);
                assert_eq!(copy.exertion, Exertion::default());
                assert_ne!(copy.id, felinors);
            }

            #[test]
            fn r64_makes_no_copy_when_the_board_is_full() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "field": ["core-008", "core-008", "core-008", "core-008"],
                        "hand": ["traininggrounds-044", "core-005"],
                        "library": ["core-005"]
                    }
                }));

                s.play("traininggrounds-044", json!({}));

                // The Felinors took the last zone itself, so the copy found none and fizzled silently.
                assert_eq!(
                    row(&s, PlayerId::P1),
                    ids(&[Some("core-008"), Some("core-008"), Some("core-008"), Some("core-008"), Some("traininggrounds-044")])
                );
            }
        }

        mod radiant {
            use super::*;

            fn radiant_hand() -> Value {
                json!([{ "def": "traininggrounds-044", "radiant": true }, "core-005"])
            }

            #[test]
            fn r275_r57_the_radiant_body_is_a_6_8_and_so_is_its_copy_which_keeps_the_radiant_flag() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": radiant_hand(), "library": ["core-005"] } }));

                s.play("traininggrounds-044", json!({}));

                assert_eq!(
                    row(&s, PlayerId::P1),
                    ids(&[Some("traininggrounds-044"), Some("traininggrounds-044"), None, None, None])
                );
                let first = lane(&s, PlayerId::P1, 1);
                let second = lane(&s, PlayerId::P1, 2);
                s.expect_stats(&first, json!({ "attack": 6, "health": 8, "maxHealth": 8 }));
                s.expect_stats(&second, json!({ "attack": 6, "health": 8, "maxHealth": 8 }));
                assert!(first.radiant);
                assert!(second.radiant);
                assert_ne!(second.id, first.id);
            }

            #[test]
            fn r1_the_radiant_copy_s_cry_does_not_fire_either() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": radiant_hand(), "library": ["core-005"] } }));

                s.play("traininggrounds-044", json!({}));

                assert_eq!(
                    row(&s, PlayerId::P1)
                        .iter()
                        .filter(|def_id| def_id.as_deref() == Some("traininggrounds-044"))
                        .count(),
                    2
                );
            }

            #[test]
            fn r57_the_radiant_copy_keeps_buffs_and_not_damage_on_the_6_8_face() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": radiant_hand(), "library": ["core-005"] } }));
                // HARNESS GAP (see the header): seeded on the hand instance.
                let felinors = s.card("traininggrounds-044").id.clone();
                seed_history(&mut s, &felinors, 1, 1, 4);

                s.play("traininggrounds-044", json!({}));

                let original = lane(&s, PlayerId::P1, 1);
                let copy = lane(&s, PlayerId::P1, 2);
                s.expect_stats(&original, json!({ "attack": 7, "health": 5, "maxHealth": 9 }));
                s.expect_stats(&copy, json!({ "attack": 7, "health": 9, "maxHealth": 9 }));
            }

            #[test]
            fn r64_the_radiant_face_makes_no_copy_on_a_full_board_either() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "field": ["core-008", "core-008", "core-008", "core-008"],
                        "hand": radiant_hand(),
                        "library": ["core-005"]
                    }
                }));

                s.play("traininggrounds-044", json!({}));

                assert_eq!(
                    row(&s, PlayerId::P1),
                    ids(&[Some("core-008"), Some("core-008"), Some("core-008"), Some("core-008"), Some("traininggrounds-044")])
                );
            }
        }
    }

    /// R2195003: the snapshot plays exactly as core-012, on both faces: the Cry summons a copy.
    #[test]
    fn r2195003_plays_exactly_as_core_012() {
        for radiant in [false, true] {
            crate::expect_plays_as(
                "core-012",
                super::ID,
                |id| json!({ "p1": { "hand": [{ "def": id, "radiant": radiant }] } }),
                |s, id| {
                    s.play(id, json!({}));
                },
            );
        }
    }
}
