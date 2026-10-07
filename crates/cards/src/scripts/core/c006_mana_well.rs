//! #6 Mana Well (SPEC §8.1): a 3-cost Field Spell, "Start of turn: gain 1 mana", radiant "Gain 2" —
//! a Radiant cell that changes only a number changes only that number (§8 Conventions), so the two
//! faces are the same hook with a different amount.
//!
//! Everything else in the §8.1 Engine cell ("Temporary mana, may exceed 4") is already the engine's:
//!   - `gain_mana` (effects/mana.rs) calls `mana::gain_mana`, which adds to `mana.current` and never
//!     touches `mana.max`, so §2.3's "temporary mana adds to current mana and can exceed 4" holds
//!     without this file saying anything. A turn-4 player refreshes to 4 and then sits at 5.
//!   - it is temporary because nothing stores it: `refresh_mana` sets `current = max` at every start
//!     of turn (§2.3), so the gain never accumulates across turns — the Well grants it again.
//!   - `start_of_turn` fires for the controller only, on their own turn, because `turn::start_turn`
//!     asks `run_hooks_in_trigger_order(sink, "startOfTurn", player)` for that one player (§2.2,
//!     §6.2, R68), and it fires before the draw, which is the same §2.2 ordering.
//!   - a Field Spell that has left the field answers no start-of-turn hook (R153's filter in
//!     `triggers::trigger_holders_with_hook`), so "leaves → back to 4" needs nothing here either.
//!
//! The gain lands on the controller because `gain_mana` defaults its `player` to "self" (§6.3).

use jackioh_engine::prelude::*;

pub const ID: &str = "core-006";

/// The only difference between the two faces is how much mana the start of the turn gives.
fn mana_well(amount: i32) -> Script {
    Script {
        start_of_turn: Some(hook(move |_ctx| vec![gain_mana(json_as(json!({ "amount": amount })))])),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = mana_well(1);
    let radiant = mana_well(2);
    CardScripts { base, radiant }
}

// #6 Mana Well — SPEC §8.1 row 6, BUILD M4-T4 must-pass: "Turn-4 player has 5 mana; leaves → back
// to 4; radiant 6".
//
// The harness's default `turn: 9` gives the active side five started turns, so `refresh_mana`
// computes §2.3's max of min(turnsStarted, MAX_MANA) = 4: every scenario here is already a
// "turn-4 player". `start_turn()` is the engine's own start of turn for the player who is active
// now — refresh, then the start-of-turn triggers, then the draw — which is the moment this card
// acts. Each side keeps a small library so that draw is a draw and not fatigue (§2.4, R3).
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
    }
}
