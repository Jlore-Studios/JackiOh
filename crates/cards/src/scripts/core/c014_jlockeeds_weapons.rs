//! #14 Jlockeed's Weapons (SPEC §8.1): Field Spell, cost 4. Base "Aura: your units have +4 attack,
//! Rush, First Strike", radiant "+10 attack". §8's Engine cell: "Aura grants keywords; removed when
//! it leaves".
//!
//! The radiant cell changes only a number, so it changes only that number (§8 Conventions): the two
//! keywords are kept and the attack bonus becomes +10.
//!
//! Nothing here removes anything. An aura is §10.4 layer 5, gathered from every permanent in play by
//! `aura_mods` in `engine/src/layers.rs` and recomputed on every read, so the bonus covers units
//! summoned after the Field Spell landed (BUILD M3-T2's acceptance names this card for exactly that)
//! and vanishes the instant the card stops being an aura source — destroyed, exiled, stolen or
//! bounced. `applies` reads instance data only and never calls back into `unit_view`, or the layers
//! would recurse.
//!
//! "Your units": the controller's units on the field, in the `units` row. The Field Spell itself sits
//! in the backrow, so the row test also keeps the aura off its own card, and a unit the opponent
//! steals stops matching because `controller` is what is compared (R78 resets it on the way out).
//! Rush from this aura is what R83's "a Reborn body the board has granted Rush may attack again"
//! refers to; granting the keyword is all this card does about it.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-014";

/// +`attack` attack, Rush and First Strike to the controller's units on the field (§10.4 layer 5);
/// `attack` is the declared number (R386), 4 and 10 on the Radiant face.
fn weapons_aura() -> AuraHook {
    aura_hook(|args: HookArgs<'_>| {
        let attack = param(&args, "attack");
        let controller = args.self_.controller;
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| {
                unit.controller == controller && matches!(unit.zone, Zone::Field { row: Row::Units, .. })
            }),
            mod_: StatMod {
                attack: Some(attack),
                keywords: Some(vec![Keyword::Rush, Keyword::FirstStrike]),
                ..StatMod::default()
            },
        }]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        aura: Some(weapons_aura()),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// #14 Jlockeed's Weapons (SPEC §8.1, BUILD M4-T4 row 14): "Allies +4 attack, Rush, First Strike
// while present; later summons get it; gone when destroyed; radiant +10".
//
// The keywords are tested through the rules rather than through a keyword list, which is what the
// row is really about:
//   Rush         — §4.1: a unit summoned this turn attacking a unit at all.
//   First Strike — §4.3 step 1: the ally kills Bigot (6/1) and takes nothing back, where without
//                  the keyword the simultaneous step would kill a 4-health ally outright.
// Bigot is seeded on the field, so its own Cry never fires (R1: only when played).
//
// The aura is §10.4 layer 5, recomputed on every read from the permanents in play, so "gone when it
// leaves" is not a clause this card scripts — the test proves it by removing the Field Spell.
//
// CROSS-CARD DEPENDENCY: the removal test plays #36 Magic Jammed ("Destroy target backrow card"),
// whose target is unnarrowed and may therefore be an ally card (§8 Conventions). It is the only
// in-game way a test can take a Field Spell off the board; a harness step that removed a permanent
// would make this test self-contained.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    fn lane(s: &Scenario, player: PlayerId, at: i32) -> CardInstance {
        match s.unit(player, at) {
            Some(unit) => unit,
            None => panic!("no unit in {player} lane {at}"),
        }
    }

    mod n14_jlockeed_s_weapons {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn gives_allied_units_4_attack_while_it_is_on_the_field() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "backrow": ["core-014"], "field": ["core-008"], "hand": ["core-005"], "library": ["core-005"] },
                    "p2": { "field": ["core-002"], "hand": ["core-005"], "library": ["core-005"] }
                }));

                // Mr. Vanilla is printed 4/4.
                s.expect_stats("core-008", json!({ "attack": 8, "health": 4, "maxHealth": 4 }));
                // "Your units": the enemy's Bigot (6/1) is untouched.
                s.expect_stats("core-002", json!({ "attack": 6, "health": 1, "maxHealth": 1 }));
            }

            #[test]
            fn grants_first_strike_so_a_4_health_ally_kills_bigot_and_takes_nothing_back_s4_3() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "backrow": ["core-014"], "field": ["core-008"], "hand": ["core-005"], "library": ["core-005"] },
                    "p2": { "field": ["core-002"], "hand": ["core-005"], "library": ["core-005"] }
                }));
                let bigot = s.card("core-002").clone();

                s.attack("core-008", "core-002");

                s.expect_in_zone(&bigot, "graveyard");
                // Without the granted First Strike the exchange is simultaneous and Bigot's 6 kills the
                // ally.
                s.expect_stats("core-008", json!({ "health": 4 }));
            }

            #[test]
            fn covers_a_unit_summoned_after_it_rush_included_s10_4_layer_5_r83() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "backrow": ["core-014"], "hand": ["core-015", "core-005"], "library": ["core-005"] },
                    "p2": { "field": ["core-002"], "hand": ["core-005"], "library": ["core-005"] }
                }));

                // #15 Me and Mr Token is printed 1/1 and its Cry summons a 3/3 Rush Token: both land
                // after the Field Spell did, and both are covered.
                s.play("core-015", json!({}));
                let first = lane(&s, PlayerId::P1, 1);
                let second = lane(&s, PlayerId::P1, 2);
                s.expect_stats(&first, json!({ "attack": 5, "health": 1 }));
                s.expect_stats(&second, json!({ "attack": 7, "health": 3 }));

                // Summoning sick this turn, and it attacks a unit anyway: that is the granted Rush.
                let bigot = s.card("core-002").clone();
                s.attack("core-015", "core-002");

                s.expect_in_zone(&bigot, "graveyard");
                s.expect_stats("core-015", json!({ "health": 1 })); // and the granted First Strike kept it alive
            }

            #[test]
            fn takes_the_whole_aura_away_when_it_is_destroyed() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "backrow": ["core-014"],
                        "field": ["core-008"],
                        "hand": ["core-036", "core-005"],
                        "library": ["core-005"]
                    },
                    "p2": { "field": ["core-002"], "hand": ["core-005"], "library": ["core-005"] }
                }));
                let weapons = s.card("core-014").clone();
                s.expect_stats("core-008", json!({ "attack": 8 }));

                // #36 Magic Jammed, aimed at the ally Field Spell (see the header).
                s.play("core-036", json!({ "targets": [{ "pick": "instance", "instanceId": weapons.id }] }));

                s.expect_in_zone(&weapons, "graveyard");
                s.expect_stats("core-008", json!({ "attack": 4, "health": 4, "maxHealth": 4 }));
                // The keywords went with it: the ally is summoning-sick-free but has no First Strike, so
                // Bigot's 6 now kills it in the simultaneous step.
                let ally = s.card("core-008").clone();
                s.attack("core-008", "core-002");
                s.expect_in_zone(&ally, "graveyard");
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn gives_10_attack_and_keeps_both_keywords_s8_conventions_only_the_number_changes() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "backrow": [{ "def": "core-014", "radiant": true }],
                        "field": ["core-008"],
                        "hand": ["core-005"],
                        "library": ["core-005"]
                    },
                    "p2": { "field": ["core-002"], "hand": ["core-005"], "library": ["core-005"] }
                }));

                s.expect_stats("core-008", json!({ "attack": 14, "health": 4, "maxHealth": 4 }));

                let bigot = s.card("core-002").clone();
                s.attack("core-008", "core-002");

                s.expect_in_zone(&bigot, "graveyard");
                s.expect_stats("core-008", json!({ "health": 4 })); // First Strike still granted
            }

            #[test]
            fn covers_a_unit_summoned_after_it_at_10_rush_included() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "backrow": [{ "def": "core-014", "radiant": true }],
                        "hand": ["core-015", "core-005"],
                        "library": ["core-005"]
                    },
                    "p2": { "field": ["core-002"], "hand": ["core-005"], "library": ["core-005"] }
                }));

                s.play("core-015", json!({}));

                let first = lane(&s, PlayerId::P1, 1);
                let second = lane(&s, PlayerId::P1, 2);
                s.expect_stats(&first, json!({ "attack": 11, "health": 1 }));
                s.expect_stats(&second, json!({ "attack": 13, "health": 3 }));
                let bigot = s.card("core-002").clone();
                s.attack("core-015", "core-002");
                s.expect_in_zone(&bigot, "graveyard");
                assert!(s.unit(PlayerId::P2, 1).is_none());
            }
        }

        #[test]
        fn r386_an_upgrade_gives_5_attack_and_a_degrade_3() {
            for (upgrade, attack) in [(true, 5), (false, 3)] {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "backrow": ["core-014"], "field": ["core-008"] } }));
                let moved = if upgrade {
                    crate::upgrade_number(&mut s, "core-014", "attack")
                } else {
                    crate::degrade_number(&mut s, "core-014", "attack")
                };
                assert_eq!(moved, attack);
                // Mr. Vanilla is a 4/4.
                s.expect_stats("core-008", json!({ "attack": 4 + attack, "health": 4 }));
            }
        }
    }
}
