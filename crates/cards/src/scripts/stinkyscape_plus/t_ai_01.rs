//! SPEC §8.12 S+ #T-AI-1 Hallucinated Kiwi, a card of the Stinkyscape+ set (R1420, R3535000). Built by S25 (#3571).
//!   Base:    "Tipsy 1
//!            Draw a card."
//!   Radiant: "Draw 2 cards."
//! Engine: a (0) Spell, Fruit, AI, Token (no printed rarity, R482). Tipsy 1 is catalog data (§6.1): the engine
//!   raises its controller's Tipsy counter as the card is played, and the Radiant face prints none. The Cry (R1)
//!   draws (§2.4): 1, Radiant 2. Declares no numbers (R482: its numbers are the script's).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-t-ai-01";

/// "Draw a card" (§2.4).
const DRAWS: i32 = 1;

/// "Draw 2 cards".
const RADIANT_DRAWS: i32 = 2;

fn face(draws: i32) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![draw(DrawArgs {
                count: draws,
                player: None,
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(DRAWS),
        radiant: face(RADIANT_DRAWS),
    }
}

// S+ #T-AI-1 Hallucinated Kiwi — SPEC §8.12 row T-AI-1.
#[cfg(test)]
mod tests {
    use super::ID;
    use crate::weapon_board;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    /// A textless (0) Spell: what the hand holds beside the Kiwi.
    const BLANK: &str = "stand-in (0) Spell";

    fn cast(radiant: bool) -> Scenario {
        let mut s = weapon_board(
            "s-t-ai-01",
            json!({ "hand": [{ "def": ID, "radiant": radiant }, BLANK], "mana": 10 }),
            json!({}),
        );
        s.play(ID, json!({}));
        s
    }

    #[test]
    fn base_adds_tipsy_1_and_draws_one() {
        let s = cast(false);
        assert_eq!(s.hand(P1).len(), 2, "the blank Spell and one draw");
        assert_eq!(s.state().players[P1].tipsy.map(|counter| counter.count), Some(1));
    }

    #[test]
    fn radiant_draws_two_and_adds_no_tipsy() {
        let s = cast(true);
        assert_eq!(s.hand(P1).len(), 3, "the blank Spell and two draws");
        assert_eq!(
            s.state().players[P1]
                .tipsy
                .map(|counter| counter.count)
                .unwrap_or(0),
            0
        );
    }
}
