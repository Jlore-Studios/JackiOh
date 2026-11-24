//! IA #38 Imaginary Friend (SPEC §8.13 row 38), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Divine Shield. Death: Set a Figment."
//!   Radiant: "Divine Shield. Death: Set two Figments."
//!
//! Divine Shield is catalog data. The Death sets face-down Figments
//! (`c025_1_figment::set_figments`); with no open backrow zone, nothing happens (R4586000).
//! Radiant ones on the Radiant face (§7).
//!
//! Engine: `summon` of a Trap, `setCostOverrideRandom` (R4586000). Built by C2 (#4586).

use jackioh_engine::prelude::*;

use crate::scripts::infinite_assault::c025_1_figment::set_figments;

pub const ID: &str = "infiniteassault-038";

fn friend(figments: i32, radiant: bool) -> Script {
    Script {
        death: Some(hook(move |_| set_figments(figments, radiant))),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: friend(1, false),
        radiant: friend(2, true),
    }
}

// IA #38 Imaginary Friend — SPEC §8.13 row 38 (R4586000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const FIGMENT: &str = "infiniteassault-025-1";
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.
    const BIG: &str = "stand-in (3) 5/5"; // A body its attack survives but the Friend does not.

    fn killed(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, STOCKPILE],
                "library": crate::filler(4),
            },
            "p2": { "field": [BIG], "hand": [STOCKPILE], "library": crate::filler(4) },
        }));
        s.play(ID, json!({}));
        // Twice: the first hit spends its Divine Shield, the second kills it.
        for _ in 0..2 {
            let friend = s.unit(P1, 1).expect("the Friend").id.clone();
            s.attack(&friend, s.unit_at(PlayerId::P2, 1));
            s.end_turn();
            s.end_turn();
        }
        s
    }

    fn figments(s: &Scenario) -> Vec<CardInstance> {
        s.state().players[P1]
            .backrow
            .iter()
            .filter_map(|slot| slot.clone())
            .filter(|card| card.def_id == FIGMENT)
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r4586000_the_death_sets_one_plain_figment() {
            let mut s = killed(false, "friend-base");
            s.expect_in_zone(ID, "graveyard");
            let set = figments(&s);
            assert_eq!(set.len(), 1);
            assert!(!set[0].radiant);
            assert!(is_face_down(s.state(), &set[0]));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4586000_the_death_sets_two_radiant_figments() {
            let mut s = killed(true, "friend-radiant");
            s.expect_in_zone(ID, "graveyard");
            let set = figments(&s);
            assert_eq!(set.len(), 2);
            assert!(set.iter().all(|card| card.radiant), "Radiant ones (§7)");
        }
    }
}
