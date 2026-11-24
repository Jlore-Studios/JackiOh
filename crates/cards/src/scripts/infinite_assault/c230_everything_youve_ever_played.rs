//! IA #230 Everything You've Ever Played (SPEC §8.13 row 230), a card of the held Infinite
//! Assault set (R4577000).
//!   Base:    "Fill your board with copies of random Units your opponent played this game."
//!   Radiant: "Fill your board with Radiant copies of random Units your opponent played this game."
//!
//! The copies come from the game history (`summonPlayedCopies` with `fill`): one unique pick per
//! open unit zone among the Units the opponent played this game — distinct, uniform and balanced
//! (R4594009) — fresh on their printed face, Radiant ones on the Radiant face. With nothing played
//! or nowhere to go it draws nothing (R129).
//!
//! Engine: `summonPlayedCopies` (R4594009). Built by C10 (#4594).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-230";

fn everything(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_| {
            vec![summon_played_copies(json_as(json!({ "fill": true, "radiant": radiant })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts { base: everything(false), radiant: everything(true) }
}

// IA #230 Everything You've Ever Played — SPEC §8.13 row 230 (R4594009).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const FIRST: &str = "stand-in (1) 2/2";
    const SECOND: &str = "stand-in (3) 5/5";
    const FILLER: &str = "core-008"; // Mr. Vanilla: holds a zone without joining the pool.
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn played_copies(s: &Scenario) -> Vec<(String, bool)> {
        s.state().players[P1]
            .units
            .iter()
            .flatten()
            .flatten()
            .filter(|card| card.def_id == FIRST || card.def_id == SECOND)
            .map(|card| (card.def_id.clone(), card.radiant))
            .collect()
    }

    fn filled(radiant: bool, seed: &str, p1_field: Value, p2_plays: &[&str]) -> Scenario {
        let mut p2_hand = vec![json!(STOCKPILE)];
        for play in p2_plays {
            p2_hand.push(json!(play));
        }
        let mut s = scenario(json!({
            "seed": seed,
            "active": "p1",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, STOCKPILE], "field": p1_field, "library": crate::filler(4) },
            "p2": { "hand": p2_hand, "library": crate::filler(4) },
        }));
        s.end_turn();
        for (lane, play) in p2_plays.iter().enumerate() {
            s.play(*play, json!({ "zone": lane as i32 + 1 }));
        }
        s.end_turn();
        s.play(ID, json!({}));
        s
    }

    mod base {
        use super::*;

        #[test]
        fn r4594009_an_empty_board_fills_with_the_one_def_played() {
            let s = filled(false, "everything-fill", json!([]), &[FIRST]);
            let copies = played_copies(&s);
            assert_eq!(copies.len(), 5, "every zone");
            assert!(copies.iter().all(|(def, radiant)| def == FIRST && !radiant));
        }

        #[test]
        fn r4594009_two_defs_over_four_zones_come_twice_each() {
            let s = filled(false, "everything-balanced", json!([FILLER]), &[FIRST, SECOND]);
            let copies = played_copies(&s);
            assert_eq!(copies.len(), 4, "the four open zones");
            assert_eq!(copies.iter().filter(|(def, _)| def == FIRST).count(), 2);
            assert_eq!(copies.iter().filter(|(def, _)| def == SECOND).count(), 2);
        }

        #[test]
        fn r4594009_nothing_played_nothing_summoned() {
            let s = filled(false, "everything-none", json!([]), &[]);
            assert!(played_copies(&s).is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4594009_the_copies_are_radiant() {
            let s = filled(true, "everything-radiant", json!([]), &[FIRST]);
            let copies = played_copies(&s);
            assert_eq!(copies.len(), 5);
            assert!(copies.iter().all(|(_, radiant)| *radiant));
        }
    }
}
