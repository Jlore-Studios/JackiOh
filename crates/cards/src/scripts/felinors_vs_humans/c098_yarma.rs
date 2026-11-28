//! SPEC §8.14 FvH #98 Yarma ((1) Unit, Felinor, Catalyst, Legendary, 1/3 → 2/6), a card of the held
//! Felinors vs Humans set. Built by S10 (#5260).
//!   Base:    "Cry: Shuffle a Dream Portal into your deck. / Death: Shuffle a Yarma Prime into your deck."
//!   Radiant: "Cry: Shuffle a Radiant Dream Portal into your deck. / Death: Shuffle a Radiant Yarma Prime
//!            into your deck."
//!
//! A Catalyst, in Meditative #45 Knowledge Breaker's shape. Cry (R1): `shuffle_into` of one FvH #98.2
//! Dream Portal; Death: `shuffle_into` of one FvH #98.1 Yarma Prime; each Radiant on the Radiant face, a
//! full deck refusing it (R80, R316). Yarma Prime carries Prime, so it is in the Prime pool. Declares no
//! numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-098";

/// FvH #98.1 Yarma Prime and #98.2 Dream Portal.
const PRIME: &str = "felinorsvshumans-098-1";
const DREAM_PORTAL: &str = "felinorsvshumans-098-2";

/// One of each the text names.
const SHUFFLED: i32 = 1;

fn shuffle_one(def_id: &'static str, radiant: bool) -> Hook {
    hook(move |_ctx| {
        vec![shuffle_into(json_as(json!({
            "defId": def_id,
            "count": SHUFFLED,
            "radiant": radiant,
        })))]
    })
}

fn yarma(radiant: bool) -> Script {
    Script {
        cry: Some(shuffle_one(DREAM_PORTAL, radiant)),
        death: Some(shuffle_one(PRIME, radiant)),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: yarma(false),
        radiant: yarma(true),
    }
}

// FvH #98 Yarma — SPEC §8.14 row 98 (R1, R80, R316).
#[cfg(test)]
mod tests {
    use super::{DREAM_PORTAL, ID, PRIME};
    use jackioh_engine::testkit::*;

    use crate::{filler, weapon_board};

    const P1: PlayerId = PlayerId::P1;

    const HIT_JOB: &str = "core-016"; // Hit Job: the removal

    /// p1's library cards of `def_id`, each one's Radiant flag.
    fn in_library(s: &Scenario, def_id: &str) -> Vec<bool> {
        s.pile(P1, "library")
            .into_iter()
            .filter(|card| card.def_id == def_id)
            .map(|card| card.radiant)
            .collect()
    }

    fn played(radiant: bool) -> Scenario {
        let mut s = weapon_board(
            "fvh098-cry",
            json!({ "hand": [{ "def": ID, "radiant": radiant }], "library": filler(4), "mana": 10 }),
            json!({}),
        );
        s.play(ID, json!({}));
        s
    }

    fn killed(radiant: bool) -> Scenario {
        let mut s = weapon_board(
            "fvh098-death",
            json!({
                "field": [{ "def": ID, "radiant": radiant }],
                "hand": [HIT_JOB],
                "library": filler(4),
                "mana": 10,
            }),
            json!({}),
        );
        let yarma = s.unit_at(P1, 1);
        s.play(HIT_JOB, pick::targets(pick::instance(&yarma)));
        s.expect_in_zone(&yarma, "graveyard");
        s
    }

    mod base {
        use super::*;

        #[test]
        fn its_cry_shuffles_a_dream_portal_into_your_deck() {
            let s = played(false);
            assert_eq!(in_library(&s, DREAM_PORTAL), [false]);
            assert!(in_library(&s, PRIME).is_empty());
        }

        #[test]
        fn its_death_shuffles_a_yarma_prime_into_your_deck() {
            let s = killed(false);
            assert_eq!(in_library(&s, PRIME), [false]);
            assert!(in_library(&s, DREAM_PORTAL).is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn its_cry_shuffles_a_radiant_dream_portal_into_your_deck() {
            let s = played(true);
            assert_eq!(in_library(&s, DREAM_PORTAL), [true]);
        }

        #[test]
        fn its_death_shuffles_a_radiant_yarma_prime_into_your_deck() {
            let s = killed(true);
            assert_eq!(in_library(&s, PRIME), [true]);
        }
    }
}
