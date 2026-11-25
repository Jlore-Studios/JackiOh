//! IA #197 Long Context (SPEC §8.13 row 197), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Cry: Draw a card. For the rest of the game, your hand size is {handSize}."
//!   Radiant: "Cry: Draw two cards. For the rest of the game, your hand size is {handSize}."
//!
//! Engine: a Cry (R1): `set_hand_cap` to the declared `handSize` for the rest of the game (R1143,
//! the latest setting wins), then `draw` — one card, two on the Radiant face — so the draws use
//! the new room. Built by C9 (#4593).

use jackioh_engine::effects::{draw, set_hand_cap};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-197";

/// The base face's draws; the Radiant face draws twice.
const DRAWS: i32 = 1;
const RADIANT_DRAWS: i32 = 2;

fn long_context(draws: i32) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![
                set_hand_cap(json_as(json!({ "cap": param(&*ctx, "handSize") }))),
                draw(json_as(json!({ "count": draws }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: long_context(DRAWS),
        radiant: long_context(RADIANT_DRAWS),
    }
}

// IA #197 Long Context — SPEC §8.13 row 197.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    fn cried(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "library": crate::filler(6),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    mod base {
        use super::*;

        #[test]
        fn the_cry_draws_one_and_sets_the_hand_cap_to_12() {
            let s = cried(false, "long-context-base");
            assert_eq!(hand_cap_of(s.state(), P1), 12);
            assert_eq!(s.hand(P1).len(), 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_cry_draws_two_and_sets_the_hand_cap_to_12() {
            let s = cried(true, "long-context-radiant");
            assert_eq!(hand_cap_of(s.state(), P1), 12);
            assert_eq!(s.hand(P1).len(), 3);
        }
    }
}
