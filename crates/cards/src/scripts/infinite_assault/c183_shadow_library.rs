//! SPEC §8.13 IA #183 Shadow Library ((2) Field Spell, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Start of turn: Add a copy of a random card your opponent played last turn to your hand."
//!   Radiant: "Start of turn: Add two copies of a random card your opponent played last turn to your hand."
//!
//! Built by C8 (#4592). R4592000, R4592002.

use jackioh_engine::effects::{AddPlayCopiesArgs, add_play_copies};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-183";

fn shadow_library(radiant: bool) -> Script {
    Script {
        start_of_turn: Some(hook(move |_ctx| {
            vec![add_play_copies(AddPlayCopiesArgs {
                of: PlayerSpec::Enemy,
                last: None,
                last_turn: Some(true),
                copies: Some(if radiant { 2 } else { 1 }),
                cost_mod: None,
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: shadow_library(false),
        radiant: shadow_library(true),
    }
}

// IA #183 Shadow Library — SPEC §8.13 row 183.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const SPELL: &str = "core-016"; // Hit Job

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "active": "p2",
            "p1": {
                "backrow": [{ "def": ID, "radiant": radiant, "faceUp": true, "lane": 1 }],
                "mana": 10,
            },
            "p2": { "hand": [SPELL], "mana": 10 },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn adds_one_copy_of_card_played_last_turn_at_start_of_turn() {
            let mut s = board(false);
            s.play(SPELL, json!({}));
            s.end_turn(); // p1's turn starts -> trigger fires
            let hand = s.hand(P1);
            assert_eq!(hand.len(), 1);
            assert_eq!(hand[0].def_id, SPELL);
        }

        #[test]
        fn with_no_plays_last_turn_adds_nothing() {
            let mut s = board(false);
            s.end_turn();
            assert_eq!(s.hand(P1).len(), 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn adds_two_copies_at_start_of_turn() {
            let mut s = board(true);
            s.play(SPELL, json!({}));
            s.end_turn();
            let hand = s.hand(P1);
            assert_eq!(hand.len(), 2);
            assert_eq!(hand[0].def_id, SPELL);
            assert_eq!(hand[1].def_id, SPELL);
        }
    }
}
