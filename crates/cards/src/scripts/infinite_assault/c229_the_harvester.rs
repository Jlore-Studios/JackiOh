//! IA #229 The Harvester (SPEC §8.13 row 229), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Cry: Steal three random cards from your opponent's deck. Start of turn: Summon a copy
//!            of a random Unit your opponent played this game."
//!   Radiant: "Cry: Steal three random cards from your opponent's deck. Start of turn: Summon two
//!            copies of a random Unit your opponent played this game."
//!
//! The Cry takes three random cards of the enemy library into its controller's hand (R60, R12),
//! hidden per zone (R97); with fewer in the deck it takes what there is (R129). The Start of turn
//! summons from the game history (`summonPlayedCopies`): one pick among the Units the opponent
//! played this game — every resolved Unit play (casts count, R70; countered plays and Cloaked sets
//! never do, R451) — the Radiant face summoning the one pick twice (R4594009). With nothing played
//! it draws nothing and the cursor stands.
//!
//! Engine: `takeFromLibrary`, `summonPlayedCopies` (R4594009). Built by C10 (#4594).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-229";

/// How many cards the Cry steals.
const STEALS: i32 = 3;

/// How many copies the Start of turn summons.
const COPIES: i32 = 1;
const RADIANT_COPIES: i32 = 2;

fn harvester(copies: i32) -> Script {
    Script {
        cry: Some(hook(|_| {
            vec![take_from_library(json_as(json!({
                "from": "enemy",
                "pick": "random",
                "count": STEALS,
            })))]
        })),
        start_of_turn: Some(hook(move |_| {
            vec![summon_played_copies(json_as(json!({ "copies": copies })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts { base: harvester(COPIES), radiant: harvester(RADIANT_COPIES) }
}

// IA #229 The Harvester — SPEC §8.13 row 229 (R4594009).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const PLAYED: &str = "core-008"; // Mr. Vanilla: the Unit p2 plays.
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn standing(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "active": "p1",
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, STOCKPILE],
                "library": crate::filler(4),
            },
            "p2": { "hand": [PLAYED, STOCKPILE], "library": [STOCKPILE, STOCKPILE, STOCKPILE] },
        }));
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    fn copies(s: &Scenario) -> Vec<CardInstance> {
        s.state().players[P1]
            .units
            .iter()
            .flatten()
            .flatten()
            .filter(|card| card.def_id == PLAYED)
            .cloned()
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r4594009_the_cry_steals_three_hidden_from_the_victim() {
            let s = standing(false, "harvester-cry");
            // The three-card library is empty now, the hand holds them.
            assert_eq!(s.hand(P1).len(), 1 + 3, "stockpile plus three steals");
            assert!(s.state().players[P2].library.is_empty());
            let theirs = crate::js(&s.view(P2));
            assert_eq!(theirs["opponent"]["hand"], json!({ "count": 4 }), "no defs leak");
        }

        #[test]
        fn r4594009_the_start_summons_a_copy_of_p2s_play() {
            let mut s = standing(false, "harvester-copy");
            assert!(copies(&s).is_empty());
            s.end_turn();
            s.play(PLAYED, json!({ "zone": 1 }));
            s.end_turn();
            let summoned = copies(&s);
            assert_eq!(summoned.len(), 1, "one copy of p2's play");
            assert!(!summoned[0].radiant, "a plain copy");
        }

        #[test]
        fn r4594009_nothing_played_nothing_summoned() {
            let mut s = standing(false, "harvester-none");
            let before = s.state().rng_cursor;
            s.end_turn();
            s.end_turn();
            assert!(copies(&s).is_empty());
            assert_eq!(s.state().rng_cursor, before, "no number drawn");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4594009_the_start_summons_two_of_the_one_pick() {
            let mut s = standing(true, "harvester-radiant");
            s.end_turn();
            s.play(PLAYED, json!({ "zone": 1 }));
            s.end_turn();
            assert_eq!(copies(&s).len(), 2, "the one pick twice");
        }
    }
}
