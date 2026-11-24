//! IA #48 Ego Death (SPEC §8.13 row 48), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Swap hands with your opponent. The cards you get cost ({discount}) less." (1)
//!   Radiant: "Swap hands with your opponent. The cards you get cost ({discount}) less." (2)
//!
//! Both hands change holders at once: every card is owned and controlled by its new holder, and
//! each card the caster takes carries `costMod` minus the declared `discount` on top of what it
//! holds. No hand cap burns a card (§2.4's cap is a draw's) and no `entersHand` hook runs: the
//! hands move, nothing is drawn (R4586008).
//!
//! Engine: `swapHands` (R4586008). Built by C2 (#4586).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-048";

fn ego_death() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            let discount = param(&*ctx, "discount");
            vec![swap_hands(json_as(json!({ "costMod": -discount })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: ego_death(),
        // The same script: the Radiant face's 2 is its declared `discount`.
        radiant: ego_death(),
    }
}

// IA #48 Ego Death — SPEC §8.13 row 48 (R4586008).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const MINE: &str = "core-005"; // Stockpile: held by p1.
    const THEIRS: &str = "core-010"; // Rapid Replenish: held by p2.
    const FILLER: &str = "core-008"; // Mr. Vanilla: played to keep the turn alive.

    fn swapped(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, MINE, MINE, FILLER],
                "library": crate::filler(10),
            },
            "p2": { "hand": [THEIRS, THEIRS, THEIRS], "library": crate::filler(10) },
        }));
        s.play(ID, json!({}));
        s
    }

    mod base {
        use super::*;

        #[test]
        fn r4586008_both_hands_change_holders_the_taken_cards_cost_1_less() {
            // TEMP-C9-VERIFY: reverted before delivery.
            let s = swapped(false, "ego-base");
            let p1 = s.hand(P1);
            let mine: Vec<&str> = p1.iter().map(|card| card.def_id.as_str()).collect();
            assert_eq!(mine, vec![THEIRS, THEIRS, THEIRS]);
            let p2 = s.hand(P2);
            let theirs: Vec<&str> =
                p2.iter().map(|card| card.def_id.as_str()).collect();
            assert_eq!(theirs, vec![MINE, MINE, FILLER]);
            for card in s.hand(P1) {
                assert_eq!(card.controller, P1);
                assert_eq!(card.owner, P1);
                assert_eq!(card.cost_mod, -1, "the cards taken cost 1 less");
            }
            for card in s.hand(P2) {
                assert_eq!(card.controller, P2);
                assert_eq!(card.owner, P2);
                assert_eq!(card.cost_mod, 0, "the other side keeps its price");
            }
        }

        #[test]
        fn r4586008_full_hands_swap_without_burning() {
            let mut s = scenario(json!({
                "seed": "ego-full",
                "p1": {
                    "hand": [ID, MINE, MINE, MINE, MINE, MINE, MINE, MINE, MINE, MINE],
                    "library": crate::filler(10),
                },
                "p2": {
                    "hand": [THEIRS, THEIRS, THEIRS, THEIRS, THEIRS, THEIRS, THEIRS, THEIRS, THEIRS, THEIRS],
                    "library": crate::filler(10),
                },
            }));
            s.play(ID, json!({}));
            // Nothing burned: every card changes sides, ten over and nine back.
            assert_eq!(s.hand(P1).len(), 10);
            assert_eq!(s.hand(P2).len(), 9);
            assert!(s.hand(P1).iter().all(|card| card.def_id == THEIRS));
            assert!(s.hand(P2).iter().all(|card| card.def_id == MINE));
            assert!(s.hand(P1).iter().all(|card| card.cost_mod == -1));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4586008_the_taken_cards_cost_2_less() {
            // TEMP-C9-VERIFY: reverted before delivery.
            let s = swapped(true, "ego-radiant");
            let p1 = s.hand(P1);
            let mine: Vec<&str> = p1.iter().map(|card| card.def_id.as_str()).collect();
            assert_eq!(mine, vec![THEIRS, THEIRS, THEIRS]);
            assert!(p1.iter().all(|card| card.cost_mod == -2));
        }
    }
}
