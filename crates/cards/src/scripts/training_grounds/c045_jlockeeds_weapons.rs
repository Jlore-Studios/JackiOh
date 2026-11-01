//! SPEC §8.11 Training Grounds #45 Legacy: Jlockeed's Weapons,
//! the Legacy card of Core #14 Jlockeed's Weapons (`core-014`), a card of the held Training Grounds set (R1420,
//! R2194000), built by #2195.
//!
//! The code is a copy of the Core card's at the foundation's commit, not a re-export, so a later Core
//! patch does not move it (R2195003); it is named `Legacy: <the Core card's name>`, tagged Legacy
//! (R2511000), and is its own card, which a deck holds beside the Core card (R2511001). The Core
//! file's notes, as they stood:
//!
//! #14 Jlockeed's Weapons (SPEC §8.1): Field Spell, cost 4. Base "Aura: your units have +3 attack,
//! Rush, First Strike", radiant "+6 attack": only the number changes (§8 Conventions).
//!
//! An aura is §10.4 layer 5, gathered from every permanent in play by `aura_mods` in
//! `engine/src/layers.rs` and recomputed on every read, so it covers units summoned later (BUILD
//! M3-T2) and vanishes the instant the card stops being an aura source. `applies` reads instance data
//! only and never calls back into `unit_view`, or the layers would recurse.
//!
//! "Your units": the controller's units in the `units` row or standing in the backrow (R1081000), so the
//! aura skips its own backrow card and a stolen unit stops matching because `controller` is compared
//! (R78 resets it on the way out). Rush from this aura is what R83's "a Reborn body the board has
//! granted Rush may attack again" refers to.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-045";

/// +`attack` attack, Rush and First Strike to the controller's units on the field (§10.4 layer 5);
/// `attack` is the declared number (R386), 3 and 6 on the Radiant face.
fn weapons_aura() -> AuraHook {
    aura_hook(|args: HookArgs<'_>| {
        let attack = param(&args, "attack");
        let controller = args.self_.controller;
        let state = args.state;
        vec![AuraEntry {
            // R1081000: a Unit standing in the backrow is one of "your units".
            applies: Box::new(move |unit: &CardInstance| {
                unit.controller == controller && in_units_row_or_standing(state, unit)
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

// #14 Jlockeed's Weapons (SPEC §8.1, BUILD M4-T4 row 14, §4.1, §4.3 step 1, R1, #36, §8 Conventions).
// Allies gain attack, Rush, First Strike while present; removed via #36 Magic Jammed.
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
            fn gives_allied_units_3_attack_while_it_is_on_the_field() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "backrow": ["traininggrounds-045"], "field": ["core-008"], "hand": ["core-005"], "library": ["core-005"] },
                    "p2": { "field": ["core-002"], "hand": ["core-005"], "library": ["core-005"] }
                }));

                // Mr. Vanilla is printed 4/4.
                s.expect_stats("core-008", json!({ "attack": 7, "health": 4, "maxHealth": 4 }));
                // "Your units": the enemy's Bigot (4/1) is untouched.
                s.expect_stats("core-002", json!({ "attack": 4, "health": 1, "maxHealth": 1 }));
            }

            #[test]
            fn grants_first_strike_so_a_4_health_ally_kills_bigot_and_takes_nothing_back_s4_3() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "backrow": ["traininggrounds-045"], "field": ["core-008"], "hand": ["core-005"], "library": ["core-005"] },
                    "p2": { "field": ["core-002"], "hand": ["core-005"], "library": ["core-005"] }
                }));
                let bigot = s.card("core-002").clone();

                s.attack("core-008", "core-002");

                s.expect_in_zone(&bigot, "graveyard");
                // Without the granted First Strike the exchange is simultaneous and Bigot's 4 kills the
                // ally.
                s.expect_stats("core-008", json!({ "health": 4 }));
            }

            #[test]
            fn covers_a_unit_summoned_after_it_rush_included_s10_4_layer_5_r83() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "backrow": ["traininggrounds-045"], "hand": ["core-015", "core-005"], "library": ["core-005"] },
                    "p2": { "field": ["core-002"], "hand": ["core-005"], "library": ["core-005"] }
                }));

                // #15 Me and Mr Token is printed 1/1 and its Cry summons a 3/3 Rush Token: both land
                // after the Field Spell did, and both are covered.
                s.play("core-015", json!({}));
                let first = lane(&s, PlayerId::P1, 1);
                let second = lane(&s, PlayerId::P1, 2);
                s.expect_stats(&first, json!({ "attack": 4, "health": 1 }));
                s.expect_stats(&second, json!({ "attack": 6, "health": 3 }));

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
                        "backrow": ["traininggrounds-045"],
                        "field": ["core-008"],
                        "hand": ["core-036", "core-005"],
                        "library": ["core-005"]
                    },
                    "p2": { "field": ["core-002"], "hand": ["core-005"], "library": ["core-005"] }
                }));
                let weapons = s.card("traininggrounds-045").clone();
                s.expect_stats("core-008", json!({ "attack": 7 }));

                // #36 Magic Jammed, aimed at the ally Field Spell (see the header).
                s.play("core-036", json!({ "targets": [{ "pick": "instance", "instanceId": weapons.id }] }));

                s.expect_in_zone(&weapons, "graveyard");
                s.expect_stats("core-008", json!({ "attack": 4, "health": 4, "maxHealth": 4 }));
                // The keywords went with it: the ally is summoning-sick-free but has no First Strike, so
                // Bigot's 4 now kills it in the simultaneous step.
                let ally = s.card("core-008").clone();
                s.attack("core-008", "core-002");
                s.expect_in_zone(&ally, "graveyard");
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn gives_6_attack_and_keeps_both_keywords_s8_conventions_only_the_number_changes() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "backrow": [{ "def": "traininggrounds-045", "radiant": true }],
                        "field": ["core-008"],
                        "hand": ["core-005"],
                        "library": ["core-005"]
                    },
                    "p2": { "field": ["core-002"], "hand": ["core-005"], "library": ["core-005"] }
                }));

                s.expect_stats("core-008", json!({ "attack": 10, "health": 4, "maxHealth": 4 }));

                let bigot = s.card("core-002").clone();
                s.attack("core-008", "core-002");

                s.expect_in_zone(&bigot, "graveyard");
                s.expect_stats("core-008", json!({ "health": 4 })); // First Strike still granted
            }

            #[test]
            fn covers_a_unit_summoned_after_it_at_6_rush_included() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "backrow": [{ "def": "traininggrounds-045", "radiant": true }],
                        "hand": ["core-015", "core-005"],
                        "library": ["core-005"]
                    },
                    "p2": { "field": ["core-002"], "hand": ["core-005"], "library": ["core-005"] }
                }));

                s.play("core-015", json!({}));

                let first = lane(&s, PlayerId::P1, 1);
                let second = lane(&s, PlayerId::P1, 2);
                s.expect_stats(&first, json!({ "attack": 7, "health": 1 }));
                s.expect_stats(&second, json!({ "attack": 9, "health": 3 }));
                let bigot = s.card("core-002").clone();
                s.attack("core-015", "core-002");
                s.expect_in_zone(&bigot, "graveyard");
                assert!(s.unit(PlayerId::P2, 1).is_none());
            }
        }

        #[test]
        fn r386_an_upgrade_gives_4_attack_and_a_degrade_2() {
            for (upgrade, attack) in [(true, 4), (false, 2)] {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "backrow": ["traininggrounds-045"], "field": ["core-008"] } }));
                let moved = if upgrade {
                    crate::upgrade_number(&mut s, "traininggrounds-045", "attack")
                } else {
                    crate::degrade_number(&mut s, "traininggrounds-045", "attack")
                };
                assert_eq!(moved, attack);
                // Mr. Vanilla is a 4/4.
                s.expect_stats("core-008", json!({ "attack": 4 + attack, "health": 4 }));
            }
        }
    }

    /// R1081000: a Unit standing in the backrow is one of "your units": it gets the +3 attack.
    #[test]
    fn r1081000_a_unit_standing_in_the_backrow_is_one_of_your_units() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "backrow": [{ "def": "traininggrounds-045", "lane": 1 }], "field": ["core-008"], "hand": ["core-005"] },
            "p2": { "hand": ["core-005"] }
        }));
        let vanilla = s.card("core-008").id.clone();
        crate::stand_in_backrow(&mut s, &vanilla, 2);
        s.expect_stats(vanilla.as_str(), json!({ "attack": 7, "health": 4 }));
        assert!(s.stats(vanilla.as_str()).keywords.iter().any(|keyword| keyword.kind() == KeywordKind::Rush));
    }

    /// R2195003: the snapshot plays exactly as core-014, on both faces: the aura reaches a Unit on the field and one summoned after.
    #[test]
    fn r2195003_plays_exactly_as_core_014() {
        for radiant in [false, true] {
            crate::expect_plays_as(
                "core-014",
                super::ID,
                |id| json!({ "p1": { "hand": [{ "def": id, "radiant": radiant }, "core-011"], "field": ["core-008"], "mana": 5 } }),
                |s, id| {
                    s.play(id, json!({}));
                    s.play("core-011", json!({}));
                },
            );
        }
    }
}
