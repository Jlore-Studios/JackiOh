//! IA #30 Smoke and Mirrors (SPEC §8.13 row 30), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Set two Figments. Draw a card."
//!   Radiant: "Set three Figments. Draw two cards."
//!
//! Face-down Figments first (`c025_1_figment::set_figments`), then the draw; with no open backrow
//! zone a set fizzles and the draw still happens (R4586000, R61). Plain Figments on both faces (§7).
//!
//! Engine: `summon` of a Trap, `setCostOverrideRandom`, `draw` (R4586000). Built by C2 (#4586).

use jackioh_engine::prelude::*;

use crate::scripts::infinite_assault::c025_1_figment::set_figments;

pub const ID: &str = "infiniteassault-030";

fn mirrors(figments: i32, cards: i32) -> Script {
    Script {
        cry: Some(hook(move |_| {
            let mut effects = set_figments(figments, false);
            effects.push(draw(json_as(json!({ "count": cards }))));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: mirrors(2, 1),
        radiant: mirrors(3, 2),
    }
}

// IA #30 Smoke and Mirrors — SPEC §8.13 row 30 (R4586000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const FIGMENT: &str = "infiniteassault-025-1";
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn cast(radiant: bool, seed: &str) -> (Scenario, usize) {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, STOCKPILE], "library": crate::filler(6) },
            "p2": { "hand": [STOCKPILE], "library": crate::filler(6) },
        }));
        let before = s.hand(P1).len();
        s.play(ID, json!({}));
        (s, before)
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
        fn r4586000_two_figments_and_a_card() {
            let (s, before) = cast(false, "mirrors-base");
            assert_eq!(figments(&s).len(), 2);
            assert_eq!(s.hand(P1).len(), before, "played one, drew one");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4586000_three_figments_and_two_cards() {
            let (s, before) = cast(true, "mirrors-radiant");
            assert_eq!(figments(&s).len(), 3);
            assert_eq!(s.hand(P1).len(), before + 1, "played one, drew two");
        }
    }
}
