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
//! Since issue #3139 it is its own card in earnest: "Aura: Your Units have +{attack} Attack and Rush."
//! at 4 (Radiant 8), with no First Strike on either face, where Core #14 moved on its own way (issue
//! #3137, part 3). `script_v0_5_11o` keeps the +3 / +6 aura with Rush and First Strike for the replays of
//! v0.5.11r and before (R767000). Since issue #3996 it gives no Rush either: "Aura: Your Units have
//! +{attack} Attack."; `script_v0_6_5b` keeps the Rush for the replays of v0.6.5b and before (R767000).
//!
//! An aura is §10.4 layer 5, gathered from every permanent in play by `aura_mods` in
//! `engine/src/layers.rs` and recomputed on every read, so it covers units summoned later (BUILD
//! M3-T2) and vanishes the instant the card stops being an aura source. `applies` reads instance data
//! only and never calls back into `unit_view`, or the layers would recurse.
//!
//! "Your units": the controller's units in the `units` row or standing in the backrow (R1081000), so the
//! aura skips its own backrow card and a stolen unit stops matching because `controller` is compared
//! (R78 resets it on the way out). Rush from this aura, through v0.6.5b, is what R83's "a Reborn body the
//! board has granted Rush may attack again" refers to.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-045";

/// +`attack` attack and `keywords`, if any, to the controller's units on the field (§10.4 layer 5);
/// `attack` is the declared number (R386), 4 and 8 on the Radiant face.
fn weapons_aura(keywords: Vec<Keyword>) -> AuraHook {
    aura_hook(move |args: HookArgs<'_>| {
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
                keywords: (!keywords.is_empty()).then(|| keywords.clone()),
                ..StatMod::default()
            },
        }]
    })
}

fn weapons(keywords: Vec<Keyword>) -> CardScripts {
    let base = Script {
        aura: Some(weapons_aura(keywords)),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

/// Attack alone since issue #3996: the aura grants no keyword.
pub fn script() -> CardScripts {
    weapons(vec![])
}

/// R767000: through v0.6.5b the aura granted Rush (issue #3996), and no First Strike since issue #3139;
/// `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_5b() -> CardScripts {
    weapons(vec![Keyword::Rush])
}

/// R767000: through v0.5.11r the aura granted Rush and First Strike (+3, Radiant +6); `KEPT_SCRIPTS` folds
/// those replays under it.
pub fn script_v0_5_11o() -> CardScripts {
    weapons(vec![Keyword::Rush, Keyword::FirstStrike])
}

// TG #45 Legacy: Jlockeed's Weapons (SPEC §8.11, BUILD M13 row TG 45, §4.1, §4.3 step 1, R1, #36, §8
// Conventions). Allies gain +4 attack (Radiant +8) while present, no First Strike since issue #3139 and no
// Rush since issue #3996; removed via #36 Magic Jammed.
#[cfg(test)]
mod tests {
    use super::{script_v0_5_11o, script_v0_6_5b};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    /// The enemy Unit the allies meet, a 4/1.
    const ENEMY: &str = "stand-in (2) 4/1 R8/2";

    fn lane(s: &Scenario, player: PlayerId, at: i32) -> CardInstance {
        match s.unit(player, at) {
            Some(unit) => unit,
            None => panic!("no unit in {player} lane {at}"),
        }
    }

    /// The keywords Mr. Vanilla shows.
    fn kinds(s: &Scenario) -> Vec<KeywordKind> {
        s.stats("core-008").keywords.iter().map(|keyword| keyword.kind()).collect()
    }

    mod n14_jlockeed_s_weapons {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn gives_allied_units_4_attack_while_it_is_on_the_field() {
                let mut s = scenario(json!({
                    "p1": { "backrow": ["traininggrounds-045"], "field": ["core-008"], "hand": ["core-005"], "library": ["core-005"] },
                    "p2": { "field": [ENEMY], "hand": ["core-005"], "library": ["core-005"] }
                }));

                // Mr. Vanilla is printed 4/4.
                s.expect_stats("core-008", json!({ "attack": 8, "health": 4, "maxHealth": 4 }));
                // "Your units": the enemy 4/1 is untouched.
                s.expect_stats(ENEMY, json!({ "attack": 4, "health": 1, "maxHealth": 1 }));
            }

            /// Issue #3996: no keyword is left, so the ally keeps only what it prints; issue #3139 took
            /// First Strike, so an exchange is simultaneous (§4.3) and the enemy 4/1's 4 kills the
            /// 4-health ally it dies to.
            #[test]
            fn grants_no_rush_and_no_first_strike_so_a_4_health_ally_trades_with_a_4_1_s4_3() {
                let mut s = scenario(json!({
                    "p1": { "backrow": ["traininggrounds-045"], "field": ["core-008"], "hand": ["core-005"], "library": ["core-005"] },
                    "p2": { "field": [ENEMY], "hand": ["core-005"], "library": ["core-005"] }
                }));
                assert!(!kinds(&s).contains(&KeywordKind::Rush));
                assert!(!kinds(&s).contains(&KeywordKind::FirstStrike));
                let enemy = s.card(ENEMY).clone();
                let ally = s.card("core-008").clone();

                s.attack("core-008", ENEMY);

                s.expect_in_zone(&enemy, "graveyard");
                s.expect_in_zone(&ally, "graveyard");
            }

            #[test]
            fn covers_a_unit_summoned_after_it_and_grants_it_no_rush_s10_4_layer_5() {
                let mut s = scenario(json!({
                    "p1": { "backrow": ["traininggrounds-045"], "hand": ["core-015", "core-005"], "library": ["core-005"] },
                    "p2": { "field": [ENEMY], "hand": ["core-005"], "library": ["core-005"] }
                }));

                // #15 Me and Mr Token is printed 1/1 and its Cry summons a 3/3 Rush Token: both land
                // after the Field Spell did, and both are covered.
                s.play("core-015", json!({}));
                let first = lane(&s, PlayerId::P1, 1);
                let second = lane(&s, PlayerId::P1, 2);
                s.expect_stats(&first, json!({ "attack": 5, "health": 1 }));
                s.expect_stats(&second, json!({ "attack": 7, "health": 3 }));

                // Summoning sick this turn, and the aura gives no Rush: it cannot attack.
                s.expect_refused(|s| s.attack("core-015", ENEMY));
                // The Rush Token prints its own Rush, so it may: it kills the 4/1 and dies to its 4, a
                // token gone.
                let enemy = s.card(ENEMY).clone();
                s.attack(&second, ENEMY);
                s.expect_in_zone(&enemy, "graveyard");
                s.expect_in_zone(&second, "gone");
            }

            #[test]
            fn takes_the_whole_aura_away_when_it_is_destroyed() {
                let mut s = scenario(json!({
                    "p1": {
                        "backrow": ["traininggrounds-045"],
                        "field": ["core-008"],
                        "hand": ["core-036", "core-005"],
                        "library": ["core-005"]
                    },
                    "p2": { "field": [ENEMY], "hand": ["core-005"], "library": ["core-005"] }
                }));
                let weapons = s.card("traininggrounds-045").clone();
                s.expect_stats("core-008", json!({ "attack": 8 }));

                // #36 Magic Jammed, aimed at the ally Field Spell (see the header).
                s.play("core-036", json!({ "targets": [{ "pick": "instance", "instanceId": weapons.id }] }));

                s.expect_in_zone(&weapons, "graveyard");
                s.expect_stats("core-008", json!({ "attack": 4, "health": 4, "maxHealth": 4 }));
                // The ally is summoning-sick-free and has no First Strike, so the enemy's 4 kills it in
                // the simultaneous step.
                let ally = s.card("core-008").clone();
                s.attack("core-008", ENEMY);
                s.expect_in_zone(&ally, "graveyard");
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn gives_8_attack_and_no_keyword_s8_conventions_only_the_number_changes() {
                let mut s = scenario(json!({
                    "p1": {
                        "backrow": [{ "def": "traininggrounds-045", "radiant": true }],
                        "field": ["core-008"],
                        "hand": ["core-005"],
                        "library": ["core-005"]
                    },
                    "p2": { "field": [ENEMY], "hand": ["core-005"], "library": ["core-005"] }
                }));

                s.expect_stats("core-008", json!({ "attack": 12, "health": 4, "maxHealth": 4 }));
                assert!(!kinds(&s).contains(&KeywordKind::Rush));
                assert!(!kinds(&s).contains(&KeywordKind::FirstStrike));

                let enemy = s.card(ENEMY).clone();
                let ally = s.card("core-008").clone();
                s.attack("core-008", ENEMY);

                s.expect_in_zone(&enemy, "graveyard");
                s.expect_in_zone(&ally, "graveyard"); // no First Strike: the exchange is simultaneous
            }

            #[test]
            fn covers_a_unit_summoned_after_it_at_8_and_grants_no_rush() {
                let mut s = scenario(json!({
                    "p1": {
                        "backrow": [{ "def": "traininggrounds-045", "radiant": true }],
                        "hand": ["core-015", "core-005"],
                        "library": ["core-005"]
                    },
                    "p2": { "field": [ENEMY], "hand": ["core-005"], "library": ["core-005"] }
                }));

                s.play("core-015", json!({}));

                let first = lane(&s, PlayerId::P1, 1);
                let second = lane(&s, PlayerId::P1, 2);
                s.expect_stats(&first, json!({ "attack": 9, "health": 1 }));
                s.expect_stats(&second, json!({ "attack": 11, "health": 3 }));
                s.expect_refused(|s| s.attack("core-015", ENEMY));
                let enemy = s.card(ENEMY).clone();
                s.attack(&second, ENEMY);
                s.expect_in_zone(&enemy, "graveyard");
                assert!(s.unit(PlayerId::P2, 1).is_none());
            }
        }

        #[test]
        fn r386_an_upgrade_gives_5_attack_and_a_degrade_3() {
            for (upgrade, attack) in [(true, 5), (false, 3)] {
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

    /// R1081000: a Unit standing in the backrow is one of "your units": it gets the +4 attack, and no Rush.
    #[test]
    fn r1081000_a_unit_standing_in_the_backrow_is_one_of_your_units() {
        let mut s = scenario(json!({
            "p1": { "backrow": [{ "def": "traininggrounds-045", "lane": 1 }], "field": ["core-008"], "hand": ["core-005"] },
            "p2": { "hand": ["core-005"] }
        }));
        let vanilla = s.card("core-008").id.clone();
        crate::stand_in_backrow(&mut s, &vanilla, 2);
        s.expect_stats(vanilla.as_str(), json!({ "attack": 8, "health": 4 }));
        assert!(!s.stats(vanilla.as_str()).keywords.iter().any(|keyword| keyword.kind() == KeywordKind::Rush));
    }

    /// R2195003, issues #3139 and #3996: the snapshot is its own copy of Core #14, frozen against its later
    /// patches, and now moved by its own: it adds 4 (Radiant 8) and no keyword, where Core #14 still adds
    /// Rush and First Strike.
    #[test]
    fn r2195003_the_legacy_card_is_its_own_copy_and_no_longer_plays_as_core_014() {
        crate::register_all();
        let aura = |id: &str, radiant: bool| -> Vec<KeywordKind> {
            let s = scenario(json!({
                "p1": { "backrow": [{ "def": id, "radiant": radiant }], "field": ["core-008"], "hand": ["core-005"] },
                "p2": { "hand": ["core-005"] }
            }));
            s.stats("core-008").keywords.iter().map(|keyword| keyword.kind()).collect()
        };
        for radiant in [false, true] {
            let legacy = aura("traininggrounds-045", radiant);
            let core = aura("core-014", radiant);
            assert!(!legacy.contains(&KeywordKind::Rush), "radiant {radiant}");
            assert!(!legacy.contains(&KeywordKind::FirstStrike), "radiant {radiant}");
            assert!(core.contains(&KeywordKind::Rush), "radiant {radiant}");
            assert!(core.contains(&KeywordKind::FirstStrike), "radiant {radiant}");
        }
    }

    /// R767000: through v0.6.5b the aura gave Rush too (issue #3996), on both faces.
    #[test]
    fn r767000_the_script_kept_for_v0_6_5b_grants_rush_and_no_first_strike() {
        crate::under_kept("traininggrounds-045", script_v0_6_5b(), || {
            for radiant in [false, true] {
                let mut s = scenario(json!({
                    "p1": {
                        "backrow": [{ "def": "traininggrounds-045", "radiant": radiant }],
                        "hand": ["core-015", "core-005"],
                        "library": ["core-005"]
                    },
                    "p2": { "field": [ENEMY], "hand": ["core-005"], "library": ["core-005"] }
                }));
                s.play("core-015", json!({}));
                let first = lane(&s, PlayerId::P1, 1);
                let kinds: Vec<KeywordKind> = s.stats(&first).keywords.iter().map(|keyword| keyword.kind()).collect();
                assert!(kinds.contains(&KeywordKind::Rush), "radiant {radiant}");
                assert!(!kinds.contains(&KeywordKind::FirstStrike), "radiant {radiant}");
                // The granted Rush: summoned this turn, it attacks all the same.
                let enemy = s.card(ENEMY).clone();
                s.attack(&first, ENEMY);
                s.expect_in_zone(&enemy, "graveyard");
            }
        });
    }

    /// R767000: through v0.5.11r the aura gave +3 (Radiant +6), Rush and First Strike.
    #[test]
    fn r767000_the_script_kept_for_v0_5_11o_grants_rush_and_first_strike_as_before() {
        crate::under_kept("traininggrounds-045", script_v0_5_11o(), || {
            let mut s = scenario(json!({
                "p1": { "backrow": ["traininggrounds-045"], "field": ["core-008"], "hand": ["core-005"], "library": ["core-005"] },
                "p2": { "field": [ENEMY], "hand": ["core-005"], "library": ["core-005"] }
            }));
            let kinds: Vec<KeywordKind> =
                s.stats("core-008").keywords.iter().map(|keyword| keyword.kind()).collect();
            assert!(kinds.contains(&KeywordKind::Rush));
            assert!(kinds.contains(&KeywordKind::FirstStrike));
            // First Strike: the 4-health ally kills the 4/1 and takes nothing back.
            let enemy = s.card(ENEMY).clone();
            s.attack("core-008", ENEMY);
            s.expect_in_zone(&enemy, "graveyard");
            s.expect_stats("core-008", json!({ "health": 4 }));
        });
    }
}
