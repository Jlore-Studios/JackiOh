//! #62 Friend of Felinors (SPEC §8.3, §7, R64).
//!
//! Base: "Fill your board with Felinor Tokens". Radiant: "Then your units get +2/+2", which §8's
//! Conventions read as an addition ("Then") rather than a replacement, so the radiant face fills the
//! board first and buffs afterwards.
//!
//! R64 defines "fill your board": every empty, unlocked unit zone, left to right — so an occupied
//! zone is untouched and a full board produces nothing. `fill_board` is that verb; this file never
//! counts zones itself.
//!
//! §8.3's Engine cell for the radiant face is "Permanent buff on those instances", i.e. layer 4 of
//! §10.4 on each unit rather than an aura, which is what `buff_all_units` writes. Ordering is the
//! whole of the radiant clause: the tokens are summoned by the effect BEFORE the buff effect in the
//! same list, and `apply_effects` runs a list in order, so the new tokens are units the controller
//! has when the buff lands and they get +2/+2 too (BUILD M4-T4 #62).

use jackioh_engine::prelude::*;

pub const ID: &str = "core-062";

/// §7's shared Felinor Token. Read through `card_def` so a wrong id fails as the script is built, not
/// in play.
fn felinor_token() -> String {
    crate::card_def("core-t-felinor").id
}

pub fn script() -> CardScripts {
    let base_token = felinor_token();
    let radiant_token = felinor_token();
    CardScripts {
        base: Script {
            cry: Some(hook(move |_ctx| vec![fill_board(json_as(json!({ "defId": base_token })))])),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(move |ctx| {
                // The +2/+2 is the declared number `buff` (R386).
                let buff = param(&*ctx, "buff");
                vec![
                    fill_board(json_as(json!({ "defId": radiant_token }))),
                    // "Then your units get +2/+2": after the fill, so the new tokens are included.
                    buff_all_units(json_as(json!({ "side": "self", "attack": buff, "health": buff }))),
                ]
            })),
            ..Script::default()
        },
    }
}

// #62 Friend of Felinors — SPEC §8.3, BUILD M4-T4 row 62.
//
// Must-pass: "Fills empty zones only; radiant then +2/+2 to every unit you control including the
// new tokens."
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const FRIEND: &str = "core-062"; // Spell, 1
    const FELINOR: &str = "core-t-felinor"; // §7's shared Felinor Token, 1/1
    const TIMMY: &str = "core-011"; // Unit, 3/3 — the unit already on the board
    const LANES: [i32; 5] = [1, 2, 3, 4, 5];

    fn unit_at(s: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
        match s.unit(player, lane) {
            Some(found) => found,
            None => panic!("expected a unit in {player} lane {lane}, found none"),
        }
    }

    fn defs_of(s: &Scenario, player: PlayerId) -> Vec<Option<String>> {
        LANES.iter().map(|&lane| s.unit(player, lane).map(|u| u.def_id)).collect()
    }

    /// A row of def ids (or empty zones) as `defs_of` reads one.
    fn row(ids: &[Option<&str>]) -> Vec<Option<String>> {
        ids.iter().map(|id| id.map(str::to_string)).collect()
    }

    mod friend_of_felinors {
        use super::*;

        #[test]
        fn r64_fills_every_empty_unit_zone_left_to_right_and_leaves_the_occupied_one_alone() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [FRIEND], "field": [TIMMY], "mana": 4 } }));

            s.play(FRIEND, json!({}));

            assert_eq!(
                defs_of(&s, PlayerId::P1),
                row(&[Some(TIMMY), Some(FELINOR), Some(FELINOR), Some(FELINOR), Some(FELINOR)])
            );
            let token = unit_at(&s, PlayerId::P1, 2);
            s.expect_stats(&token, json!({ "attack": 1, "maxHealth": 1 }));
            // The pre-existing unit is untouched by the base face.
            let timmy = unit_at(&s, PlayerId::P1, 1);
            s.expect_stats(&timmy, json!({ "attack": 3, "maxHealth": 3 }));
        }

        #[test]
        fn fills_only_your_own_board() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [FRIEND], "mana": 4 }, "p2": { "field": [TIMMY] } }));

            s.play(FRIEND, json!({}));

            assert_eq!(
                defs_of(&s, PlayerId::P1),
                row(&[Some(FELINOR), Some(FELINOR), Some(FELINOR), Some(FELINOR), Some(FELINOR)])
            );
            assert_eq!(defs_of(&s, PlayerId::P2), row(&[Some(TIMMY), None, None, None, None]));
        }

        #[test]
        fn r64_a_full_board_takes_no_tokens_and_the_spell_still_resolves() {
            crate::register_all();
            let full = [TIMMY, "core-t-rush", "core-t-sheep", "core-019", "core-020"];
            let mut s = scenario(json!({ "p1": { "hand": [FRIEND], "field": full, "mana": 4 } }));

            s.play(FRIEND, json!({}));

            assert_eq!(defs_of(&s, PlayerId::P1), row(&full.map(Some)));
            s.expect_in_zone(FRIEND, "graveyard");
        }

        #[test]
        fn radiant_gives_plus_2_plus_2_to_every_unit_you_control_the_new_tokens_included() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [{ "def": FRIEND, "radiant": true }], "field": [TIMMY], "mana": 4 } }));

            s.play(FRIEND, json!({}));

            // "Then": the buff lands after the fill, so the Felinors it just made are units you control.
            let timmy = unit_at(&s, PlayerId::P1, 1);
            s.expect_stats(&timmy, json!({ "attack": 5, "maxHealth": 5 }));
            for lane in [2, 3, 4, 5] {
                let token = unit_at(&s, PlayerId::P1, lane);
                assert_eq!(token.def_id, FELINOR);
                s.expect_stats(&token, json!({ "attack": 3, "maxHealth": 3 }));
            }
            s.expect_events(json!(["cardPlayed", "summoned", "buffed"]));
        }

        #[test]
        fn radiant_buffs_your_units_only() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": FRIEND, "radiant": true }], "mana": 4 },
                "p2": { "field": [TIMMY] },
            }));

            s.play(FRIEND, json!({}));

            let token = unit_at(&s, PlayerId::P1, 1);
            s.expect_stats(&token, json!({ "attack": 3, "maxHealth": 3 }));
            let timmy = unit_at(&s, PlayerId::P2, 1);
            s.expect_stats(&timmy, json!({ "attack": 3, "maxHealth": 3 }));
        }

        #[test]
        fn radiant_still_buffs_your_board_when_it_is_full_and_no_token_can_enter() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": FRIEND, "radiant": true }], "field": [TIMMY, TIMMY, TIMMY, TIMMY, TIMMY], "mana": 4 },
            }));

            s.play(FRIEND, json!({}));

            for lane in LANES {
                let unit = unit_at(&s, PlayerId::P1, lane);
                s.expect_stats(&unit, json!({ "attack": 5, "maxHealth": 5 }));
            }
        }

        #[test]
        fn r386_an_upgrade_gives_3_3_and_a_degrade_1_1() {
            for (upgrade, buff) in [(true, 3), (false, 1)] {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": FRIEND, "radiant": true }], "field": [TIMMY, TIMMY, TIMMY, TIMMY, TIMMY], "mana": 4 },
                }));
                let moved = if upgrade {
                    crate::upgrade_number(&mut s, FRIEND, "buff")
                } else {
                    crate::degrade_number(&mut s, FRIEND, "buff")
                };
                assert_eq!(moved, buff);
                s.play(FRIEND, json!({}));
                let unit = unit_at(&s, PlayerId::P1, 1);
                s.expect_stats(&unit, json!({ "attack": 3 + buff, "maxHealth": 3 + buff }));
            }
        }
    }
}
