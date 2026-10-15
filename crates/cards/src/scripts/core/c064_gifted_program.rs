//! #64 Gifted Program (SPEC §8.3, §10.1, §10.5 step 3, §5.2, R56, R70, R74, R213, R214).
//!
//! Base: "The first card costing 1 or less you play each turn becomes Radiant as it is played".
//! Radiant: "2 or less" — §8's Conventions make that a change to the threshold only.
//!
//! §8.3's Engine cell puts it before resolution with "cost = cost paid": §10.5 step 3, after the
//! cost is paid and before the card moves or resolves. The card is a static flag carrying its
//! threshold, read by `play_steps::gifted_program_step` through `play_choices::gifted_makes_radiant`.
//! Three things decide it, all the engine's:
//!   - "you play": only the playing player's permanents count (§8 Conventions), so a stolen Gifted
//!     Program works for its thief;
//!   - R56: the cost compared is the one ACTUALLY PAID after every modifier; a cast pays 0 (R70);
//!   - "the first ... each turn": R213 counts plays off the turn log, not a flag on this instance,
//!     so a Gifted Program that changes hands, or leaves and comes back, can neither use up another
//!     player's first cheap card nor hand its own player a second one.
//!
//! A flag, not a hook, because step 1 must know the face before it reads targets and modes (R214).
//! It cannot catch its own play: step 3 runs before step 4 puts it on the board (R119).
//! THE GLOW (R662): `condition.rs` asks `query::gifted_would_make_radiant`, which is
//! `gifted_makes_radiant` at the cost a play pays now (R56, R213), so glow and step 3 agree.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-064";

/// The printed thresholds: "1 or less", and "2 or less" on the radiant face. The card declares them as
/// `giftLimit` (R386), and the engine reads the flag through `params::declared_or`, so a Degrade or an
/// Upgrade moves the threshold step 3 uses.
const BASE_THRESHOLD: i32 = 1;
const RADIANT_THRESHOLD: i32 = 2;

fn gifted(threshold: i32) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            gifted_program: Some(threshold),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: gifted(BASE_THRESHOLD),
        radiant: gifted(RADIANT_THRESHOLD),
    }
}

// #64 Gifted Program — SPEC §8.3, §10.5 step 3, BUILD M4-T4 row 64.
//
// Must-pass: "First ≤1-cost card each turn is radiant before it resolves (its Cry uses radiant
// text); second is not; radiant threshold 2 (R56)."
//
// The card is its `giftedProgram` flag, read as §10.5 step 3 (R213, R214); `backrow: [GIFTED]` arms
// it. R662's yellow glow is tested at the end of this file.
#[cfg(test)]
mod tests {
    use super::script;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const GIFTED: &str = "core-064"; // Field Spell, 2
    const FRIEND: &str = "core-062"; // Spell, 1 — radiant adds +2/+2, so the face it ran is visible
    const SURGERY: &str = "core-063"; // Spell, 1 — +3/+3 base, +6/+6 radiant
    const POINTMASTER: &str = "core-020"; // Unit, 2 — 7/1 base, 14/2 radiant: the threshold probe
    const TIMMY: &str = "core-011"; // Unit, 3/3
    const MENACE: &str = "core-019"; // Unit, 9/9 — filler so a turn never auto-ends (§2.5, R82)
    const FELINOR: &str = "core-t-felinor";

    fn unit_at(s: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
        match s.unit(player, lane) {
            Some(found) => found,
            None => panic!("expected a unit in {player} lane {lane}, found none"),
        }
    }

    fn sel(card: &CardInstance) -> Value {
        json!({ "pick": "instance", "instanceId": card.id })
    }

    /// Whether `card` glows in p1's hand.
    fn glows_in_hand(s: &Scenario, card: &str) -> bool {
        let id = s.card(card).id.clone();
        hand_glows(s, &id, P1)
    }

    #[test]
    fn r386_an_upgrade_reaches_a_2_cost_card_and_a_radiant_degrade_stops_at_1() {
        for (radiant, upgrade, limit, made_radiant) in [(false, true, 2, true), (true, false, 1, false)] {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "backrow": [{ "def": GIFTED, "radiant": radiant }], "hand": [POINTMASTER, MENACE], "mana": 4 }
            }));
            let moved = if upgrade {
                crate::upgrade_number(&mut s, GIFTED, "giftLimit")
            } else {
                crate::degrade_number(&mut s, GIFTED, "giftLimit")
            };
            assert_eq!(moved, limit);
            s.play(POINTMASTER, json!({}));
            assert_eq!(s.card(POINTMASTER).radiant, made_radiant);
        }
        crate::register_all();
        let s = scenario(json!({ "p1": { "backrow": [GIFTED] } }));
        assert!(!crate::can_degrade_number(&s, GIFTED, "giftLimit"));
    }

    mod gifted_program {
        use super::*;

        #[test]
        fn s10_5_step_3_both_faces_carry_the_pre_resolution_threshold_and_nothing_else() {
            // The card is its flag: it has no Cry, no trigger, no hook and no aura, so if step 3 did not read
            // `giftedProgram` the card would do nothing at all. The faces differ only in the threshold.
            crate::register_all();
            let faces = script();
            assert_eq!(
                serde_json::to_value(&faces.base.static_flags).expect("flags serialise"),
                json!({ "giftedProgram": 1 })
            );
            assert_eq!(
                serde_json::to_value(&faces.radiant.static_flags).expect("flags serialise"),
                json!({ "giftedProgram": 2 })
            );
            assert!(faces.base.on_play_hook.is_none());
            assert!(faces.base.cry.is_none());
            assert!(faces.base.start_of_turn.is_none());
            assert!(faces.base.aura.is_none());
        }

        #[test]
        fn it_enters_the_backrow_and_never_makes_its_own_play_radiant() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [GIFTED], "mana": 4 } }));

            s.play(GIFTED, json!({}));

            s.expect_in_zone(GIFTED, "field");
            // Step 3 runs before step 4 puts the card on the board, so it is not among its own hooks —
            // which matters for the radiant face, whose own cost of 2 is inside its own threshold.
            assert!(!s.card(GIFTED).radiant);
        }

        #[test]
        fn r56_a_card_the_opponent_plays_is_never_made_radiant() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [GIFTED], "field": [MENACE] },
                "p2": { "hand": [FRIEND], "field": [TIMMY], "mana": 4 },
            }));
            let timmy = s.card(TIMMY).clone();

            s.play(FRIEND, json!({}));

            // "you play": p1's Gifted Program ignores p2's play, so p2's spell runs its base text and
            // p2's units are not buffed.
            s.expect_stats(&timmy, json!({ "attack": 3, "maxHealth": 3 }));
        }

        #[test]
        fn r56_the_first_card_costing_1_or_less_becomes_radiant_before_it_resolves() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "backrow": [GIFTED], "hand": [FRIEND], "field": [TIMMY], "mana": 4 } }));
            let timmy = s.card(TIMMY).clone();

            s.play(FRIEND, json!({}));

            // §10.5 step 3 sets the flag before step 5 runs the script, so the RADIANT text is what ran:
            // "Then your units get +2/+2" on top of the fill.
            s.expect_stats(&timmy, json!({ "attack": 5, "maxHealth": 5 }));
            let token = unit_at(&s, P1, 2);
            s.expect_stats(&token, json!({ "attack": 3, "maxHealth": 3 }));
        }

        #[test]
        fn r56_the_second_cheap_card_of_the_same_turn_is_not_made_radiant() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "backrow": [GIFTED], "hand": [SURGERY, FRIEND], "field": [TIMMY], "mana": 4 },
            }));
            let timmy = s.card(TIMMY).clone();

            s.play(SURGERY, json!({ "targets": [sel(&timmy)] }));
            s.expect_stats(&timmy, json!({ "attack": 9, "maxHealth": 9 }));

            // The turn's first cheap card has been played (R213), so this one resolves its base text: fill
            // only, no +2/+2.
            s.play(FRIEND, json!({}));

            s.expect_stats(&timmy, json!({ "attack": 9, "maxHealth": 9 }));
            let token = unit_at(&s, P1, 2);
            assert_eq!(token.def_id, FELINOR);
            s.expect_stats(&token, json!({ "attack": 1, "maxHealth": 1 }));
        }

        #[test]
        fn r56_a_card_above_the_threshold_is_left_alone() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "backrow": [GIFTED], "hand": [POINTMASTER], "mana": 4 }, "p2": {} }));

            s.play(POINTMASTER, json!({}));

            // Pointmaster costs 2, over the base threshold of 1, so it enters on its base face.
            s.expect_stats(POINTMASTER, json!({ "attack": 7, "maxHealth": 1 }));
            assert!(!s.card(POINTMASTER).radiant);
        }

        #[test]
        fn r56_radiant_s_threshold_of_2_catches_a_2_cost_card_the_base_one_does_not() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "backrow": [{ "def": GIFTED, "radiant": true }], "hand": [POINTMASTER], "mana": 4 },
                "p2": {},
            }));

            s.play(POINTMASTER, json!({}));

            assert!(s.card(POINTMASTER).radiant);
            s.expect_stats(POINTMASTER, json!({ "attack": 14, "maxHealth": 2 }));
        }

        #[test]
        fn r56_the_count_starts_again_each_turn_so_the_next_turn_s_first_cheap_card_is_radiant_too() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "backrow": [GIFTED],
                    "hand": [SURGERY, FRIEND],
                    "field": [TIMMY],
                    "library": [MENACE, MENACE],
                    "mana": 4,
                },
                "p2": { "hand": [MENACE], "field": [MENACE], "library": [MENACE] },
            }));
            let timmy = s.card(TIMMY).clone();

            s.play(SURGERY, json!({ "targets": [sel(&timmy)] }));
            s.expect_stats(&timmy, json!({ "attack": 9, "maxHealth": 9 }));

            // Over to p2 and back: one endTurn hands the turn over, the second comes back around.
            s.end_turn();
            s.end_turn();

            s.play(FRIEND, json!({}));

            // A fresh turn, so Friend of Felinors is the first cheap card again and runs its radiant text.
            s.expect_stats(&timmy, json!({ "attack": 11, "maxHealth": 11 }));
            let token = unit_at(&s, P1, 2);
            s.expect_stats(&token, json!({ "attack": 3, "maxHealth": 3 }));
        }
    }

    mod lights_the_hand_card_it_would_make_radiant_r662_r213 {
        use super::*;

        #[test]
        fn r662_base_the_1_cost_card_glows_and_the_2_cost_one_does_not_and_the_glowing_one_is_played_radiant() {
            crate::register_all();
            let mut s = scenario(json!({ "seed": "r662-064-base", "p1": { "backrow": [GIFTED], "hand": [FRIEND, POINTMASTER, MENACE] } }));
            assert!(glows_in_hand(&s, FRIEND));
            assert!(!glows_in_hand(&s, POINTMASTER));

            s.play(POINTMASTER, json!({}));
            assert!(!s.card(POINTMASTER).radiant);
        }

        #[test]
        fn r662_radiant_the_2_cost_card_glows_too_and_once_one_cheap_card_is_played_nothing_does() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "r662-064-radiant",
                "p1": { "backrow": [{ "def": GIFTED, "radiant": true }], "hand": [FRIEND, POINTMASTER, MENACE] },
            }));
            assert!(glows_in_hand(&s, FRIEND));
            assert!(glows_in_hand(&s, POINTMASTER));

            s.play(POINTMASTER, json!({}));
            assert!(s.card(POINTMASTER).radiant);
            // "The first … each turn" (R213): the cheap play has been made.
            assert!(!glows_in_hand(&s, FRIEND));
        }

        #[test]
        fn r662_a_card_already_radiant_gains_nothing_so_it_does_not_glow() {
            crate::register_all();
            let s = scenario(json!({
                "seed": "r662-064-already",
                "p1": { "backrow": [GIFTED], "hand": [{ "def": FRIEND, "radiant": true }, MENACE] },
            }));
            assert!(!glows_in_hand(&s, FRIEND));
        }

        #[test]
        fn r662_without_it_on_the_field_or_on_the_opponent_s_side_nothing_glows() {
            crate::register_all();
            let s = scenario(json!({
                "seed": "r662-064-none",
                "p1": { "hand": [FRIEND, MENACE] },
                "p2": { "backrow": [GIFTED] },
            }));
            assert!(!glows_in_hand(&s, FRIEND));
        }
    }
}
