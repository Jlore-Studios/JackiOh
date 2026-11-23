//! SPEC §8.12 S+ #69 Exotic Jellybean, a card of the Stinkyscape+ set (R1420, R3535000). (2) Spell, Jelly Bean, Epic.
//!   Base:    "Add a Sweet Essence to your hand."
//!   Radiant: "Add a Radiant Sweet Essence to your hand."
//!
//! One `add_to_hand` of S+ #69.1 Sweet Essence, Radiant on the Radiant face. A named card, so no pool.
//! Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-069";

fn jellybean(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![add_to_hand(json_as(json!({
                "defId": super::c069_1_sweet_essence::ID,
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: jellybean(false),
        radiant: jellybean(true),
    }
}

// S+ #69 Exotic Jellybean — SPEC §8.12 row 69.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const ESSENCE: &str = super::super::c069_1_sweet_essence::ID;
    const FILLER: &str = "core-005";

    fn cast(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        s.play(ID, json!({}));
        s
    }

    #[test]
    fn adds_a_sweet_essence() {
        let s = cast(false);
        let made: Vec<CardInstance> = s
            .hand(P1)
            .into_iter()
            .filter(|card| card.def_id == ESSENCE)
            .collect();
        assert_eq!(made.len(), 1);
        assert!(!made[0].radiant);
    }

    #[test]
    fn radiant_adds_a_radiant_one() {
        let s = cast(true);
        let made: Vec<CardInstance> = s
            .hand(P1)
            .into_iter()
            .filter(|card| card.def_id == ESSENCE)
            .collect();
        assert_eq!(made.len(), 1);
        assert!(made[0].radiant);
    }
}
