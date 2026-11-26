//! SPEC §8.13 IA #235.1 The Prime Generator, Iteration Two (token, (4) Unit, 12/16 → 24/32),
//! a token of the held Infinite Assault set (R4577000).
//!   Base:    "Immune to Spells, Rush. Start of turn: Summon a random Radiant Infinite Assault
//!            Unit."
//!   Radiant: "Immune to Spells, Rush. Start of turn: Summon two random Radiant Infinite Assault
//!            Units."
//!
//! Immune to Spells and Rush are catalog data. The Start of turn summons from the set-named pool
//! (R1846001, R4577000): a random Unit of the set, Radiant ones, itself never offered (R387,
//! §5.1). With no open zone it draws nothing (R129).
//!
//! Engine: `summonRandom` (R4594003 is the Drone's; this token's reading is §7). Built by C10
//! (#4594).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-235-1";

/// The set the pool names (R4577000).
const SET: &str = "Infinite Assault";

/// How many Units the Start of turn summons.
const SUMMONS: i32 = 1;
const RADIANT_SUMMONS: i32 = 2;

fn iteration_two(summons: i32) -> Script {
    Script {
        start_of_turn: Some(hook(move |_| {
            (0..summons.max(0))
                .map(|_| {
                    summon_random(json_as(json!({
                        "query": { "set": SET, "type": "Unit" },
                        "radiant": true,
                    })))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts { base: iteration_two(SUMMONS), radiant: iteration_two(RADIANT_SUMMONS) }
}

// IA #235.1 The Prime Generator, Iteration Two — SPEC §8.13 row 235.1, §7.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::{ID, SET};
    use crate::scenario;

    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.
    const FILLER_UNIT: &str = "core-008"; // Mr. Vanilla: holds a zone in the full-board test.

    fn summoned(s: &Scenario) -> Vec<CardInstance> {
        s.state().players[P1]
            .units
            .iter()
            .flatten()
            .flatten()
            .filter(|card| card.def_id != ID)
            .cloned()
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn start_of_turn_summons_an_ia_radiant_unit() {
            let mut s = scenario(json!({
                "seed": "iteration-two",
                "active": "p1",
                "p1": { "field": [{ "def": ID }], "hand": [STOCKPILE], "library": crate::filler(4) },
                "p2": { "hand": [STOCKPILE], "library": crate::filler(4) },
            }));
            s.start_turn();
            let made = summoned(&s);
            assert_eq!(made.len(), 1, "one Unit");
            for card in nonempty(made, "the summoned Unit") {
                let def = def_of(Some(s.state()), &card.def_id);
                assert_eq!(def.set, SetName::InfiniteAssault, "{}", SET);
                assert_eq!(def.type_, CardType::Unit);
                assert!(card.radiant, "a Radiant Unit");
            }
        }

        #[test]
        fn a_full_board_leaves_the_cursor_unchanged() {
            let mut s = scenario(json!({
                "seed": "iteration-full",
                "active": "p1",
                "p1": {
                    "field": [{ "def": ID }, FILLER_UNIT, FILLER_UNIT, FILLER_UNIT, FILLER_UNIT],
                    "hand": [STOCKPILE],
                    "library": crate::filler(4),
                },
                "p2": { "hand": [STOCKPILE], "library": crate::filler(4) },
            }));
            let held: Vec<String> = s.state().players[P1]
                .units
                .iter()
                .flatten()
                .flatten()
                .map(|card| card.id.clone())
                .collect();
            let before = s.state().rng_cursor;
            s.start_turn();
            let now: Vec<String> = s.state().players[P1]
                .units
                .iter()
                .flatten()
                .flatten()
                .map(|card| card.id.clone())
                .collect();
            assert_eq!(now, held, "nothing summoned");
            assert_eq!(s.state().rng_cursor, before, "no number drawn");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn start_of_turn_summons_two_radiant_units() {
            let mut s = scenario(json!({
                "seed": "iteration-radiant",
                "active": "p1",
                "p1": { "field": [{ "def": ID, "radiant": true }], "hand": [STOCKPILE], "library": crate::filler(4) },
                "p2": { "hand": [STOCKPILE], "library": crate::filler(4) },
            }));
            s.start_turn();
            let made = summoned(&s);
            assert_eq!(made.len(), 2, "two Units");
            assert!(made.iter().all(|card| card.radiant), "Radiant ones");
        }
    }
}
