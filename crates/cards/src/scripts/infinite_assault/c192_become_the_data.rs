//! SPEC §8.13 IA #192 Become the Data ((4) Spell, Mythic), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Swap decks with your opponent. Draw two cards."
//!   Radiant: "Swap decks with your opponent. Draw three cards."
//!
//! Built by C8 (#4592). R4592000, R4592002.

use jackioh_engine::effects::{draw, swap_library};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-192";

fn become_the_data(radiant: bool) -> Script {
    let count = if radiant { 3 } else { 2 };
    Script {
        cry: Some(hook(move |_ctx| {
            vec![
                swap_library(),
                draw(json_as(json!({ "count": count }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: become_the_data(false),
        radiant: become_the_data(true),
    }
}

// IA #192 Become the Data — SPEC §8.13 row 192.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const CARD_P1: &str = "core-001";
    const CARD_P2: &str = "core-016";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }],
                "library": [CARD_P1, CARD_P1, CARD_P1, CARD_P1],
                "mana": 10,
            },
            "p2": {
                "library": [CARD_P2, CARD_P2, CARD_P2, CARD_P2],
            },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn swaps_decks_and_draws_two_from_new_deck() {
            let mut s = board(false);
            s.play(ID, json!({}));
            // P1 drew 2 cards from what was P2's deck
            let hand = s.hand(P1);
            assert_eq!(hand.len(), 2);
            assert_eq!(hand[0].def_id, CARD_P2);
            assert_eq!(hand[1].def_id, CARD_P2);

            // Cards drawn started in P2's deck
            assert!(jackioh_engine::origin::started_in_opponents_deck(&hand[0]));

            // P2's library now has P1's original cards
            assert_eq!(s.state().players[P2].library[0].def_id, CARD_P1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn swaps_decks_and_draws_three() {
            let mut s = board(true);
            s.play(ID, json!({}));
            assert_eq!(s.hand(P1).len(), 3);
            assert_eq!(s.hand(P1)[0].def_id, CARD_P2);
        }
    }
}
