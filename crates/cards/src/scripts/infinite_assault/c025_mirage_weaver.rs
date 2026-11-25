//! IA #25 Mirage Weaver (SPEC §8.13 row 25), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Cry: Set a Figment."
//!   Radiant: "Cry: Set two Figments."
//!
//! The Cry sets face-down Figments (`c025_1_figment::set_figments`), fresh summons into the
//! leftmost open backrow zones; with none open, nothing happens (R4586000). On its Radiant face
//! the Figments are Radiant ones (§7).
//!
//! Engine: `summon` of a Trap, `setCostOverrideRandom` (R4586000). Built by C2 (#4586).

use jackioh_engine::prelude::*;

use crate::scripts::infinite_assault::c025_1_figment::set_figments;

pub const ID: &str = "infiniteassault-025";

fn weaver(figments: i32, radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_| set_figments(figments, radiant))),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: weaver(1, false),
        radiant: weaver(2, true),
    }
}

// IA #25 Mirage Weaver — SPEC §8.13 row 25 (R4586000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const FIGMENT: &str = "infiniteassault-025-1";
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn played(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, STOCKPILE],
                "library": crate::filler(4),
            },
            "p2": { "hand": [STOCKPILE], "library": crate::filler(4) },
        }));
        s.play(ID, json!({}));
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
        fn r4586000_the_cry_sets_one_face_down_figment() {
            let s = played(false, "weaver-base");
            let set = figments(&s);
            assert_eq!(set.len(), 1);
            assert!(is_face_down(s.state(), &set[0]));
            assert!(!set[0].radiant, "a plain Figment");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4586000_the_cry_sets_two_radiant_figments() {
            let s = played(true, "weaver-radiant");
            let set = figments(&s);
            assert_eq!(set.len(), 2);
            assert!(set.iter().all(|card| card.radiant), "Radiant ones (§7)");
            assert!(set.iter().all(|card| is_face_down(s.state(), card)));
        }
    }
}
