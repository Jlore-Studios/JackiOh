//! IA #193 Seedling Mind (SPEC §8.13 row 193), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "End of turn: Gain +{buff}/+{buff}."
//!   Radiant: "End of turn: Gain +{buff}/+{buff}."
//!
//! Engine: an `end_of_turn` hook (its controller's, R68): a permanent `buff` of the declared
//! `buff` on itself. Built by C9 (#4593).

use jackioh_engine::effects::buff;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-193";

fn seedling() -> Script {
    Script {
        end_of_turn: Some(hook(|ctx| {
            let gain = param(&*ctx, "buff");
            vec![buff(json_as(json!({
                "target": { "of": "self" },
                "attack": gain,
                "health": gain,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The same script: the Radiant face's +2/+2 is its declared `buff`.
    let radiant = seedling();
    CardScripts {
        base: seedling(),
        radiant,
    }
}

// IA #193 Seedling Mind — SPEC §8.13 row 193.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    fn grown(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "field": [{ "def": ID, "lane": 1, "radiant": radiant }],
                "hand": [FILLER],
                "library": crate::filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        s.end_turn();
        s
    }

    fn stats(s: &Scenario) -> (i32, i32) {
        let unit = s.unit_at(P1, 1);
        let view = unit_view(s.state(), &unit);
        (view.attack, view.health)
    }

    mod base {
        use super::*;

        #[test]
        fn gains_1_1_at_the_end_of_its_controllers_turn() {
            let s = grown(false, "seedling-base");
            assert_eq!(stats(&s), (2, 3));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn gains_2_2_at_the_end_of_its_controllers_turn() {
            let s = grown(true, "seedling-radiant");
            assert_eq!(stats(&s), (4, 6));
        }
    }
}
