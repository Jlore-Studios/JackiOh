//! IA #206 Thought Accelerator (SPEC §8.13 row 206), a card of the held Infinite Assault
//! set (R4577000).
//!   Base:    "Cry: Your Dormant Units awaken."
//!   Radiant: "Cry: Your Dormant Units awaken. They gain +{buff}/+{buff}."
//!
//! Engine: a Cry (R1): each Dormant card its controller has on the field, in lane order,
//! `awaken`s — each "When this awakens" answering (R1839002) — and on the Radiant face each
//! takes a permanent `buff` of the declared `buff` after it wakes (R4593011). Built by C9
//! (#4593).

use jackioh_engine::effects::{awaken, buff};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-206";

fn accelerator(radiant_face: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let sleepers: Vec<String> = units_on_field_of(ctx.state, ctx.controller)
                .into_iter()
                .filter(|card| is_slumbering(card))
                .map(|card| card.id.clone())
                .collect();
            let gain = if radiant_face { param(&*ctx, "buff") } else { 0 };
            let mut effects = Vec::new();
            for id in sleepers {
                effects.push(awaken(json_as(json!({
                    "target": { "of": "instance", "instanceId": id },
                }))));
                if radiant_face {
                    effects.push(buff(json_as(json!({
                        "target": { "of": "instance", "instanceId": id },
                        "attack": gain,
                        "health": gain,
                    }))));
                }
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: accelerator(false),
        radiant: accelerator(true),
    }
}

// IA #206 Thought Accelerator — SPEC §8.13 row 206 (R4593011).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const SLEEPER: &str = "infiniteassault-199"; // Patient Machine: Dormant for 1 turn.
    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    fn accelerated(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "field": [{ "def": SLEEPER, "lane": 1 }],
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "library": crate::filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        s.play(ID, json!({ "zone": 2 }));
        s
    }

    mod base {
        use super::*;

        #[test]
        fn r4593011_a_dormant_unit_wakes_with_no_buff() {
            let s = accelerated(false, "accelerator-base");
            let unit = s.unit_at(P1, 1);
            assert!(!is_slumbering(&unit));
            let view = unit_view(s.state(), &unit);
            assert_eq!((view.attack, view.health), (5, 6));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4593011_a_dormant_unit_wakes_with_2_2() {
            let s = accelerated(true, "accelerator-radiant");
            let unit = s.unit_at(P1, 1);
            assert!(!is_slumbering(&unit));
            let view = unit_view(s.state(), &unit);
            assert_eq!((view.attack, view.health), (7, 8));
        }
    }
}
