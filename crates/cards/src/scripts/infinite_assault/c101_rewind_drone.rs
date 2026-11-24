//! SPEC §8.13 IA #101 Rewind Drone ((2) Unit, Common, 2/4 → 4/8), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Cry: Return a random Spell from your graveyard to hand."
//!   Radiant: "Cry: Return two random Spells from your graveyard to hand."
//!
//! One `add_random_from_graveyard` filtered to Spells (`card_type`, R4589008), each pick uniform
//! over the graveyard's Spells (R60); fewer Spells than asked ends it. "To hand" is R746's wording:
//! a card that returns from anywhere but the field (a Spell after it resolves, a card from the
//! graveyard) says so, while a bounce off the field says "to its owner's hand" (R692).

use jackioh_engine::effects::add_random_from_graveyard;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-101";

/// "a random Spell", the base face's.
const COUNT: i32 = 1;
/// "two random Spells", the Radiant face's.
const RADIANT_COUNT: i32 = 2;

fn drone(count: i32) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![add_random_from_graveyard(json_as(json!({
                "player": "self",
                "count": count,
                "cardType": "Spell",
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: drone(COUNT),
        radiant: drone(RADIANT_COUNT),
    }
}

// IA #101 Rewind Drone — SPEC §8.13 row 101 (R4589008).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const SPELL: &str = "stand-in (0) Spell";
    const UNIT: &str = "stand-in (1) 2/2";

    /// p1's graveyard holds two Spells and a Unit; the Drone is played from hand.
    fn rewound(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }],
                "graveyard": [SPELL, SPELL, UNIT],
                "library": crate::filler(2),
            },
            "p2": { "library": crate::filler(2) },
        }));
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    fn spells_back(s: &Scenario) -> usize {
        s.hand(P1).iter().filter(|card| card.def_id == SPELL).count()
    }

    mod base {
        use super::*;

        #[test]
        fn r4589008_returns_spells_only() {
            let s = rewound(false);
            assert_eq!(spells_back(&s), 1, "one random Spell returns to hand");
            assert!(
                s.hand(P1).iter().all(|card| card.def_id != UNIT),
                "no Unit returns"
            );
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4589008_returns_two_spells_only() {
            let s = rewound(true);
            assert_eq!(spells_back(&s), 2, "two random Spells return to hand");
            assert!(
                s.hand(P1).iter().all(|card| card.def_id != UNIT),
                "no Unit returns"
            );
        }
    }
}
