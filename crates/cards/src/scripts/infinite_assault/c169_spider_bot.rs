//! SPEC §8.13 IA #169 Spider Bot ((1) Unit, Common, 2/2 → 4/4), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Cry: Add a copy of a random card in your opponent's deck to your hand."
//!   Radiant: "Cry: Add a copy of a random card in your opponent's deck to your hand. It costs ({discount}) less."
//!
//! Built by C8 (#4592). R4592000, R4592002, R4592005.

use jackioh_engine::effects::{AddLibraryCopiesArgs, add_library_copies};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-169";

fn spider(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let cost_mod = if radiant {
                Some(-param(&*ctx, "discount"))
            } else {
                None
            };
            vec![add_library_copies(AddLibraryCopiesArgs {
                of: PlayerSpec::Enemy,
                count: 1,
                brittle: None,
                cost_mod,
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: spider(false),
        radiant: spider(true),
    }
}

// IA #169 Spider Bot — SPEC §8.13 row 169.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "core-005";
    const TARGET: &str = "core-016";

    fn board(radiant: bool, opp_library: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "mana": 10 },
            "p2": { "hand": [FILLER], "library": opp_library },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn cry_adds_a_copy_of_random_card_in_opponents_deck() {
            let mut s = board(false, json!([TARGET]));
            s.play(ID, json!({ "zone": 1 }));
            s.expect_stats(s.unit_at(P1, 1), json!({ "attack": 2, "health": 2 }));
            let added = s.hand(P1).into_iter().find(|c| c.def_id == TARGET).expect("added card");
            assert!(!added.radiant);
            assert_eq!(added.cost_mod, 0);
            assert_eq!(s.state().players[P2].library.len(), 1);
            assert!(!jackioh_engine::origin::started_in_opponents_deck(&added));
        }

        #[test]
        fn with_empty_deck_nothing_is_added() {
            let mut s = board(false, json!([]));
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(s.hand(P1).len(), 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_4_4_and_adds_copy_with_discount() {
            let mut s = board(true, json!([TARGET]));
            s.play(ID, json!({ "zone": 1 }));
            s.expect_stats(s.unit_at(P1, 1), json!({ "attack": 4, "health": 4 }));
            let added = s.hand(P1).into_iter().find(|c| c.def_id == TARGET).expect("added card");
            assert_eq!(added.cost_mod, -1);
        }
    }
}
