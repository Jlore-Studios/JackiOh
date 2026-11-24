//! IA #41 Phantom Army (SPEC §8.13 row 41), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Fill your board with Phantoms."
//!   Radiant: "Fill your board with Radiant Phantoms."
//!
//! Every empty unit zone of its controller's takes a Phantom (§3.2); a full board summons
//! none (R129). Radiant ones on the Radiant face.
//!
//! Engine: `fill_board` (R4586009). Built by C2 (#4586).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-041";

/// What the board fills with (§7).
const PHANTOM: &str = "infiniteassault-041-1";

fn army(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_| {
            vec![fill_board(json_as(json!({ "defId": PHANTOM, "radiant": radiant })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: army(false),
        radiant: army(true),
    }
}

// IA #41 Phantom Army — SPEC §8.13 row 41.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::{ID, PHANTOM};
    use crate::scenario;

    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.
    const UNIT: &str = "core-008"; // Mr. Vanilla: a 4/4.

    fn cast(radiant: bool, seed: &str, field: Value) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, STOCKPILE],
                "field": field,
                "library": crate::filler(4),
            },
            "p2": { "hand": [STOCKPILE], "library": crate::filler(4) },
        }));
        s.play(ID, json!({}));
        s
    }

    fn phantoms(s: &Scenario) -> Vec<CardInstance> {
        s.state().players[P1]
            .units
            .iter()
            .flatten()
            .flatten()
            .filter(|card| card.def_id == PHANTOM)
            .cloned()
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn every_empty_zone_takes_a_plain_phantom() {
            let s = cast(false, "army-base", json!([{ "def": UNIT, "lane": 1 }]));
            let landed = phantoms(&s);
            assert_eq!(landed.len(), 4, "lanes 2 to 5");
            assert!(landed.iter().all(|card| !card.radiant));
            assert!(landed.iter().all(|card| card.brittle.is_some()), "printed Brittle 1");
        }

        #[test]
        fn a_full_board_summons_none() {
            let s = cast(
                false,
                "army-full",
                json!([
                    { "def": UNIT, "lane": 1 },
                    { "def": UNIT, "lane": 2 },
                    { "def": UNIT, "lane": 3 },
                    { "def": UNIT, "lane": 4 },
                    { "def": UNIT, "lane": 5 },
                ]),
            );
            assert!(phantoms(&s).is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_phantoms_are_radiant() {
            let s = cast(true, "army-radiant", json!([]));
            let landed = phantoms(&s);
            assert_eq!(landed.len(), 5, "every zone");
            assert!(landed.iter().all(|card| card.radiant), "Radiant ones");
        }
    }
}
