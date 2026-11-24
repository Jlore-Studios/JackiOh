//! SPEC §8.13 IA #178 Dataset Thief ((3) Unit, Rare, 4/5 → 8/10), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Cry: Steal a random card from your opponent's deck."
//!   Radiant: "Cry: Steal two random cards from your opponent's deck."
//!
//! Built by C8 (#4592). R4592000, R4592002.

use jackioh_engine::effects::{TakeFromLibraryArgs, TakeFromLibraryPick, take_from_library};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-178";

fn dataset_thief(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![take_from_library(TakeFromLibraryArgs {
                from: Some(PlayerSpec::Enemy),
                pick: Some(TakeFromLibraryPick::Random),
                count: Some(if radiant { 2 } else { 1 }),
                ..Default::default()
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: dataset_thief(false),
        radiant: dataset_thief(true),
    }
}

// IA #178 Dataset Thief — SPEC §8.13 row 178.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "core-005";
    const TARGET1: &str = "core-016";
    const TARGET2: &str = "core-019";

    fn board(radiant: bool, opp_library: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "mana": 10 },
            "p2": { "hand": [FILLER], "library": opp_library },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn steals_one_card_from_opponents_deck() {
            let mut s = board(false, json!([TARGET1]));
            s.play(ID, json!({ "zone": 1 }));
            s.expect_stats(s.unit_at(P1, 1), json!({ "attack": 4, "health": 5 }));
            assert_eq!(s.state().players[P2].library.len(), 0);
            let stolen = s.hand(P1).into_iter().find(|c| c.def_id == TARGET1).expect("stolen card");
            assert_eq!(stolen.owner, P1);
            // Started in opponent's deck
            assert!(jackioh_engine::origin::started_in_opponents_deck(&stolen));
        }

        #[test]
        fn with_empty_deck_steals_nothing() {
            let mut s = board(false, json!([]));
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(s.hand(P1).len(), 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_8_10_and_steals_two_cards() {
            let mut s = board(true, json!([TARGET1, TARGET2]));
            s.play(ID, json!({ "zone": 1 }));
            s.expect_stats(s.unit_at(P1, 1), json!({ "attack": 8, "health": 10 }));
            assert_eq!(s.state().players[P2].library.len(), 0);
            assert_eq!(s.hand(P1).len(), 2);
        }
    }
}
