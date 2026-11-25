//! IA #226 Escalated Privileges (SPEC §8.13 row 226), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Steal two random cards from your opponent's hand."
//!   Radiant: "Steal three random cards from your opponent's hand."
//!
//! `give_from_hand` with random cards: `count` distinct random cards (R60) become the stealer's
//! (R12), hidden per zone (R97), the hand cap burning the overflow into the stealer's graveyard
//! (§2.4); an empty or short hand takes what there is and draws no number for what there is not
//! (R129).
//!
//! Engine: `giveFromHand`. Built by C10 (#4594).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-226";

/// How many cards the faces steal.
const COUNT: i32 = 2;
const RADIANT_COUNT: i32 = 3;

fn privileges(count: i32) -> Script {
    Script {
        cry: Some(hook(move |_| {
            vec![give_from_hand(json_as(json!({
                "from": "enemy",
                "cards": "random",
                "count": count,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts { base: privileges(COUNT), radiant: privileges(RADIANT_COUNT) }
}

// IA #226 Escalated Privileges — SPEC §8.13 row 226.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn played(radiant: bool, seed: &str, p1_rest: Value, p2_hand: Value) -> Scenario {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend(p1_rest.as_array().cloned().unwrap_or_default());
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": json!(hand), "library": crate::filler(10) },
            "p2": { "hand": p2_hand, "library": crate::filler(10) },
        }));
        s.play(ID, json!({}));
        s
    }

    mod base {
        use super::*;

        #[test]
        fn two_random_cards_change_hands() {
            let s = played(
                false,
                "privileges-base",
                json!([STOCKPILE]),
                json!([STOCKPILE, STOCKPILE, STOCKPILE, STOCKPILE]),
            );
            assert_eq!(s.hand(P1).len(), 1 + 2, "played one, stole two");
            assert_eq!(s.hand(P2).len(), 4 - 2);
            assert!(s.hand(P1).iter().all(|card| card.controller == P1));
        }

        #[test]
        fn a_short_hand_gives_what_there_is() {
            let s = played(false, "privileges-short", json!([]), json!([STOCKPILE]));
            assert_eq!(s.hand(P1).len(), 1);
            assert!(s.hand(P2).is_empty());
        }

        #[test]
        fn a_full_hand_burns_the_overflow() {
            let mut hand = vec![];
            hand.extend((0..HAND_CAP - 1).map(|_| json!(STOCKPILE)));
            let s = played(
                false,
                "privileges-burn",
                json!(hand),
                json!([STOCKPILE, STOCKPILE, STOCKPILE]),
            );
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            // The cast Privileges rests there too: only the burned Stockpile counts.
            let burned = s.pile(P1, "graveyard").into_iter().filter(|card| card.def_id == STOCKPILE).count();
            assert_eq!(burned, 1, "the burned card");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn three_random_cards_change_hands() {
            let s = played(
                true,
                "privileges-radiant",
                json!([STOCKPILE]),
                json!([STOCKPILE, STOCKPILE, STOCKPILE, STOCKPILE]),
            );
            assert_eq!(s.hand(P1).len(), 1 + 3);
            assert_eq!(s.hand(P2).len(), 4 - 3);
        }
    }
}
