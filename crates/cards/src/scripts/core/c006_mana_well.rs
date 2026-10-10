//! #6 Mana Well (SPEC §8.1): a 3-cost Field Spell, "Start of turn: gain 1 mana", radiant "Gain 2" —
//! a Radiant cell that changes only a number changes only that number (§8 Conventions), so the two
//! faces are the same hook with a different amount.
//!
//! The rest of the §8.1 Engine cell ("Temporary mana, may exceed 4") is the engine's already:
//!   - `gain_mana` adds to `mana.current`, never `mana.max`, so it can exceed 4 (§2.3); `refresh_mana`
//!     resets `current = max` each turn, so the gain never accumulates.
//!   - `start_of_turn` fires for the controller only, before the draw (§2.2, §6.2, R68), and a Field
//!     Spell that has left the field answers no hook (R153), so "leaves → back to 4" needs nothing here.
//!
//! The gain lands on the controller because `gain_mana` defaults its `player` to "self" (§6.3).

use jackioh_engine::prelude::*;

pub const ID: &str = "core-006";

/// The only difference between the two faces is how much mana the start of the turn gives: the
/// declared number `mana` (R386), 1 and 2 on the Radiant face, read off the face that is up.
fn mana_well() -> Script {
    Script {
        start_of_turn: Some(hook(|ctx| vec![gain_mana(json_as(json!({ "amount": param(&*ctx, "mana") })))])),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = mana_well();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// #6 Mana Well — SPEC §8.1 row 6, BUILD M4-T4 must-pass: "Turn-4 player has 5 mana; leaves → back
// to 4; radiant 6".
//
// The default `turn: 9` makes every scenario here a "turn-4 player" (§2.3's max is 4), and each side
// keeps a small library so `start_turn()`'s draw is a draw, not fatigue (§2.4, R3).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const LIBRARY: [&str; 4] = ["core-011", "core-011", "core-011", "core-011"];

    mod n6_mana_well_s8_1_row_6 {
        use super::*;

        #[test]
        fn a_turn_4_player_has_5_mana_at_the_start_of_the_turn_above_a_max_of_4() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-006-base",
                "p1": { "backrow": ["core-006"], "hand": ["core-011"], "library": LIBRARY },
                "p2": { "hand": ["core-011"], "library": LIBRARY }
            }));

            // §2.3: the refresh at setup is the cap alone.
            s.expect_mana(PlayerId::P1, 4);
            assert_eq!(s.view(PlayerId::P1).you.mana.max, 4);

            s.start_turn();

            // 4 refreshed + 1 from the Well, and the max is untouched: temporary mana may exceed 4
            // (§2.3).
            s.expect_mana(PlayerId::P1, 5);
            assert_eq!(s.view(PlayerId::P1).you.mana.max, 4);
        }

        #[test]
        fn the_gain_is_temporary_not_cumulative_the_next_start_of_turn_is_5_again() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-006-temporary",
                "p1": { "backrow": ["core-006"], "hand": ["core-011"], "library": LIBRARY },
                "p2": { "hand": ["core-011"], "library": LIBRARY }
            }));

            s.start_turn();
            s.expect_mana(PlayerId::P1, 5);
            s.start_turn();

            // `refresh_mana` set current back to max (4) before the Well gave its 1 again (§2.3).
            s.expect_mana(PlayerId::P1, 5);
            assert_eq!(s.view(PlayerId::P1).you.mana.max, 4);
        }

        #[test]
        fn only_the_controller_gains_and_only_on_their_own_turn_s2_2_r68() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-006-controller",
                "p1": { "backrow": ["core-006"], "hand": ["core-011"], "library": LIBRARY },
                "p2": { "hand": ["core-011"], "library": LIBRARY },
                "active": "p2"
            }));

            // p2 is active, so p2's start of turn runs; a Field Spell fires for its controller alone.
            s.start_turn();
            s.expect_mana(PlayerId::P2, 4);
            s.expect_mana(PlayerId::P1, 4);
        }

        #[test]
        fn the_field_spell_leaves_back_to_4() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-006-leaves",
                // #34 Collateral Damage exiles a chosen permanent on either side (§8.2 row 34).
                "p1": { "backrow": ["core-006"], "hand": ["core-034", "core-011"], "library": LIBRARY },
                "p2": { "hand": ["core-011"], "library": LIBRARY }
            }));
            let well = s.card("core-006").clone();

            s.start_turn();
            s.expect_mana(PlayerId::P1, 5);

            s.play("core-034", json!({ "targets": [{ "pick": "instance", "instanceId": well.id }] }));
            s.expect_in_zone(&well, "exile");

            s.start_turn();
            s.expect_mana(PlayerId::P1, 4);
        }

        #[test]
        fn radiant_gains_2_a_turn_4_player_has_6() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-006-radiant",
                "p1": {
                    "backrow": [{ "def": "core-006", "radiant": true }],
                    "hand": ["core-011"],
                    "library": LIBRARY
                },
                "p2": { "hand": ["core-011"], "library": LIBRARY }
            }));

            s.start_turn();

            s.expect_mana(PlayerId::P1, 6);
            assert_eq!(s.view(PlayerId::P1).you.mana.max, 4);
        }

        #[test]
        fn r386_an_upgrade_gains_2_a_turn_and_a_radiant_degrade_1() {
            for (radiant, upgrade, mana) in [(false, true, 6), (true, false, 5)] {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "core-006-tuned",
                    "p1": {
                        "backrow": [{ "def": "core-006", "radiant": radiant }],
                        "hand": ["core-011"],
                        "library": LIBRARY
                    },
                    "p2": { "hand": ["core-011"], "library": LIBRARY }
                }));
                let moved = if upgrade {
                    crate::upgrade_number(&mut s, "core-006", "mana")
                } else {
                    crate::degrade_number(&mut s, "core-006", "mana")
                };
                assert_eq!(moved, mana - 4);
                s.start_turn();
                s.expect_mana(PlayerId::P1, mana);
            }
        }
    }
}
