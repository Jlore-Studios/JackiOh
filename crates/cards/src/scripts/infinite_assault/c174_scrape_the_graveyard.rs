//! SPEC §8.13 IA #174 Scrape the Graveyard ((2) Spell, Common), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Add copies of two random cards in your opponent's graveyard to your hand."
//!   Radiant: "Add copies of three random cards in your opponent's graveyard to your hand."
//!
//! Built by C8 (#4592). R4592000, R4592002.

use jackioh_engine::effects::add_to_hand;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-174";

fn scrape(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let count = if radiant { 3 } else { 2 };
            let opp = opponent_of(ctx.controller);
            let graveyard: Vec<String> = ctx.state.players[opp]
                .graveyard
                .iter()
                .map(|c| c.id.clone())
                .collect();
            let to_take = if graveyard.len() <= count {
                graveyard
            } else {
                ctx.rng.shuffle(&graveyard).into_iter().take(count).collect()
            };
            to_take
                .into_iter()
                .map(|id| add_to_hand(json_as(json!({ "copyOf": id }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: scrape(false),
        radiant: scrape(true),
    }
}

// IA #174 Scrape the Graveyard — SPEC §8.13 row 174.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "core-005";
    const SPELL1: &str = "core-016";
    const SPELL2: &str = "core-014";
    const SPELL3: &str = "core-088";

    fn board(radiant: bool, opp_graveyard: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "mana": 10 },
            "p2": { "hand": [FILLER], "graveyard": opp_graveyard },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn adds_copies_of_two_cards_from_graveyard() {
            let mut s = board(false, json!([SPELL1, SPELL2]));
            s.play(ID, json!({}));
            let hand = s.hand(P1);
            assert!(hand.iter().any(|c| c.def_id == SPELL1));
            assert!(hand.iter().any(|c| c.def_id == SPELL2));
            assert_eq!(s.state().players[P2].graveyard.len(), 2);
        }

        #[test]
        fn with_empty_graveyard_adds_nothing() {
            let mut s = board(false, json!([]));
            s.play(ID, json!({}));
            assert_eq!(s.hand(P1).len(), 1); // just filler
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn adds_copies_of_three_cards_from_graveyard() {
            let mut s = board(true, json!([SPELL1, SPELL2, SPELL3]));
            s.play(ID, json!({}));
            let hand = s.hand(P1);
            assert!(hand.iter().any(|c| c.def_id == SPELL1));
            assert!(hand.iter().any(|c| c.def_id == SPELL2));
            assert!(hand.iter().any(|c| c.def_id == SPELL3));
        }
    }
}
