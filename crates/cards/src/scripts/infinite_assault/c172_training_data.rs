//! SPEC §8.13 IA #172 Training Data ((2) Spell, Common), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Add copies of the last two cards your opponent played to your hand."
//!   Radiant: "Add copies of the last three cards your opponent played to your hand."
//!
//! Built by C8 (#4592). R4592000.

use jackioh_engine::effects::{AddPlayCopiesArgs, add_play_copies};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-172";

fn training_data(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![add_play_copies(AddPlayCopiesArgs {
                of: PlayerSpec::Enemy,
                last: Some(if radiant { 3 } else { 2 }),
                last_turn: None,
                copies: None,
                cost_mod: None,
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: training_data(false),
        radiant: training_data(true),
    }
}

// IA #172 Training Data — SPEC §8.13 row 172.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "core-005";
    const SPELL1: &str = "core-016"; // Hit Job
    const SPELL2: &str = "core-088"; // Twisting Nether
    const SPELL3: &str = "core-014"; // Shield Slam

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "mana": 10 },
            "p2": { "hand": [SPELL1, SPELL2, SPELL3], "mana": 10 },
            "active": "p2",
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn adds_copies_of_last_two_cards_opponent_played() {
            let mut s = board(false);
            s.play(SPELL1, json!({}));
            s.play(SPELL2, json!({}));
            s.end_turn();
            s.play(ID, json!({}));
            let hand = s.hand(P1);
            assert!(hand.iter().any(|c| c.def_id == SPELL1));
            assert!(hand.iter().any(|c| c.def_id == SPELL2));
        }

        #[test]
        fn with_fewer_plays_adds_available() {
            let mut s = board(false);
            s.play(SPELL1, json!({}));
            s.end_turn();
            s.play(ID, json!({}));
            let hand = s.hand(P1);
            assert_eq!(hand.iter().filter(|c| c.def_id == SPELL1).count(), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn adds_copies_of_last_three_cards_opponent_played() {
            let mut s = board(true);
            s.play(SPELL1, json!({}));
            s.play(SPELL2, json!({}));
            s.play(SPELL3, json!({}));
            s.end_turn();
            s.play(ID, json!({}));
            let hand = s.hand(P1);
            assert!(hand.iter().any(|c| c.def_id == SPELL1));
            assert!(hand.iter().any(|c| c.def_id == SPELL2));
            assert!(hand.iter().any(|c| c.def_id == SPELL3));
        }
    }
}
