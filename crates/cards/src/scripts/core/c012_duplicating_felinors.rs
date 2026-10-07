//! #12 Duplicating Felinors (SPEC §8.1): 3/4 → 6/9 Unit, Felinor, cost 2, Rare. Base "Cry: summon a
//! copy of this unit"; the radiant cell says "Same", so the radiant face runs the very same text
//! (§8 Conventions) on the 6/9 body — and its copy is a Radiant 6/9 too, so the effect doubles with
//! the stats (R275). §8's Engine cell: "Copy per R57, placed per R64; the copy's Cry does not fire
//! (§6.2)".
//!
//! R57 (copy semantics): "A copy of a unit on the field keeps its radiant flag, buffs, granted
//! keywords, Vanilla state and `statsOverride` and resets damage, exertion and counters." The copy is
//! a new instance, so it also takes the current turn as its `summonedTurn` (§4.1 summoning sickness)
//! and carries no memory of its own.
//!
//! R64 (placement): with no zone named the copy takes the leftmost empty, unlocked, unreserved unit
//! zone, and the summon fizzles silently when the row has none — which is the row's "board full → no
//! copy" with no check of its own.
//!
//! R1 / §6.2 (Cry): "Only when played from hand or cast by an effect. Copies, Recruit, Reborn, tokens,
//! Transform never fire it" — the cited reason being this very card, which "would fill the board for
//! 2 mana otherwise". A summon fires no Cry (engine/src/effects/summon.rs), so the chain ends after
//! one copy on its own.
//!
//! The clone is the engine's `summon_copy` (engine/src/effects/summon.rs), which resolves `of` to a
//! unit, clones it per R57 and places it per R64 through the same path as `summon`. A card file may
//! not read the buffs, keywords and Vanilla state off the instance and rebuild them (CLAUDE.md
//! rule 5), which is why the copy is a verb and not a `summon({ defId, radiant })`.

use jackioh_engine::effects::summon_copy;
use jackioh_engine::prelude::*;

pub const ID: &str = "core-012";

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
// R57: a copy of a unit on the field keeps its radiant flag, buffs, granted keywords, Vanilla state
// and `statsOverride`, and resets damage, exertion and counters.
// R64: with no zone named the copy takes the leftmost empty, unlocked, unreserved unit zone, and
// the summon fizzles when the row has none.
// R1 / §6.2: a copy fires no Cry — the ruling names this very card ("#12 would fill the board for
// 2 mana otherwise").
//
// Radiant (R275): the body is a 6/9 and the cell is "Same", so the copy — which keeps the radiant
// flag (R57) — is a Radiant 6/9 as well: the effect doubles with the stats.
//
// HARNESS GAP, worked around here and reported: there is no way to seed layer-4 buffs or damage.
// They cannot be reached through play either: a Cry fires the instant the unit enters, so the played
// body has no history yet. R78 resets both on leaving the field and `reduce`'s play path resets
// neither on entering, so seeding them on the hand instance is the one route to the "buffs copied,
// damage not" clause.
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

    /// HARNESS GAP (see the header): layer-4 buffs and damage, seeded on the hand instance because a
    /// Cry leaves no window to apply them on the field.
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
                    // Lanes 1 and 3 are taken, so lane 2 is the leftmost free zone for the Felinors
                    // itself and lane 4 for its copy: placement is "leftmost free", never "adjacent"
                    // (§3.1 lanes).
                    "p1": {
                        "field": [
                            { "def": "core-008", "lane": 1 },
                            { "def": "core-020", "lane": 3 }
                        ],
                        "hand": ["core-012", "core-005"],
                        "library": ["core-005"]
                    }
                }));

                s.play("core-012", json!({}));

                assert_eq!(
                    row(&s, PlayerId::P1),
                    ids(&[Some("core-008"), Some("core-012"), Some("core-020"), Some("core-012"), None])
                );
            }

            #[test]
            fn r1_the_copy_s_cry_does_not_fire_so_exactly_one_copy_is_made() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": ["core-012", "core-005"], "library": ["core-005"] } }));

                s.play("core-012", json!({}));

                // Two bodies, not a full board: the copy is a summon, and a summon fires no Cry (§6.2).
                assert_eq!(
                    row(&s, PlayerId::P1),
                    ids(&[Some("core-012"), Some("core-012"), None, None, None])
                );
            }

            #[test]
            fn r57_copies_buffs_and_does_not_copy_damage_exertion_or_counters() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": ["core-012", "core-005"], "library": ["core-005"] } }));
                // HARNESS GAP (see the header): layer-4 buffs and damage, seeded on the hand instance
                // because a Cry leaves no window to apply them on the field.
                let felinors = s.card("core-012").id.clone();
                seed_history(&mut s, &felinors, 2, 2, 3);

                s.play("core-012", json!({}));

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
                        "hand": ["core-012", "core-005"],
                        "library": ["core-005"]
                    }
                }));

                s.play("core-012", json!({}));

                // The Felinors took the last zone itself, so the copy found none and fizzled silently.
                assert_eq!(
                    row(&s, PlayerId::P1),
                    ids(&[Some("core-008"), Some("core-008"), Some("core-008"), Some("core-008"), Some("core-012")])
                );
            }
        }

        mod radiant {
            use super::*;

            fn radiant_hand() -> Value {
                json!([{ "def": "core-012", "radiant": true }, "core-005"])
            }

            #[test]
            fn r275_r57_the_radiant_body_is_a_6_9_and_so_is_its_copy_which_keeps_the_radiant_flag() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": radiant_hand(), "library": ["core-005"] } }));

                s.play("core-012", json!({}));

                assert_eq!(
                    row(&s, PlayerId::P1),
                    ids(&[Some("core-012"), Some("core-012"), None, None, None])
                );
                let first = lane(&s, PlayerId::P1, 1);
                let second = lane(&s, PlayerId::P1, 2);
                s.expect_stats(&first, json!({ "attack": 6, "health": 9, "maxHealth": 9 }));
                s.expect_stats(&second, json!({ "attack": 6, "health": 9, "maxHealth": 9 }));
                assert!(first.radiant);
                assert!(second.radiant);
                assert_ne!(second.id, first.id);
            }

            #[test]
            fn r1_the_radiant_copy_s_cry_does_not_fire_either() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": radiant_hand(), "library": ["core-005"] } }));

                s.play("core-012", json!({}));

                assert_eq!(
                    row(&s, PlayerId::P1)
                        .iter()
                        .filter(|def_id| def_id.as_deref() == Some("core-012"))
                        .count(),
                    2
                );
            }

            #[test]
            fn r57_the_radiant_copy_keeps_buffs_and_not_damage_on_the_6_9_face() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": radiant_hand(), "library": ["core-005"] } }));
                // HARNESS GAP (see the header): seeded on the hand instance.
                let felinors = s.card("core-012").id.clone();
                seed_history(&mut s, &felinors, 1, 1, 4);

                s.play("core-012", json!({}));

                let original = lane(&s, PlayerId::P1, 1);
                let copy = lane(&s, PlayerId::P1, 2);
                s.expect_stats(&original, json!({ "attack": 7, "health": 6, "maxHealth": 10 }));
                s.expect_stats(&copy, json!({ "attack": 7, "health": 10, "maxHealth": 10 }));
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

                s.play("core-012", json!({}));

                assert_eq!(
                    row(&s, PlayerId::P1),
                    ids(&[Some("core-008"), Some("core-008"), Some("core-008"), Some("core-008"), Some("core-012")])
                );
            }
        }
    }
}
