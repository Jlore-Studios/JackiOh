//! IA #34 Mass Hallucination (SPEC §8.13 row 34), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Transform every enemy (1) Cost or less Unit into a Husk."
//!   Radiant: "Transform every enemy (2) Cost or less Unit into a Husk."
//!
//! Every enemy Unit it can reach (R4453000) costing the cap or less as it resolves (R65) becomes
//! a plain Husk on both faces; costlier Units and unreachable ones are passed by (R61, R129).
//! (The wording is C2's tune: "(N) Cost or less" for the pool the price names, R1748003.)
//!
//! Engine: `transform` (R4586009). Built by C2 (#4586).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-034";

/// What the transformed Units become (§7).
const HUSK: &str = "infiniteassault-034-1";

/// The cap the pool's price filter names.
/// A Spell declares no number it prints unless the cross test names why it is fixed
/// (`tests/cross/params.rs`, R1748003): the two caps are that file's `FIXED` price rows.
const BASE_CAP: i32 = 1;
const RADIANT_CAP: i32 = 2;

fn hallucination(cap: i32) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let enemy = opponent_of(ctx.controller);
            let source = ctx.self_.clone();
            active_units_of(&*ctx.state, enemy)
                .into_iter()
                .filter(|card| {
                    cost_now(&*ctx.state, card) <= cap
                        && !spell_cannot_reach(&*ctx.state, source.as_ref(), card)
                })
                .map(|card| {
                    transform(json_as(json!({ "instanceId": card.id, "defId": HUSK })))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: hallucination(BASE_CAP),
        radiant: hallucination(RADIANT_CAP),
    }
}

// IA #34 Mass Hallucination — SPEC §8.13 row 34 (R4586009).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P2;
    use jackioh_engine::testkit::*;

    use super::{HUSK, ID};
    use crate::scenario;

    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.
    const CHEAP: &str = "core-008"; // Mr. Vanilla: (1).
    const PRICEY: &str = "core-001"; // Big D-fender: (2).

    fn cast(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, STOCKPILE],
                "library": crate::filler(4),
            },
            "p2": { "field": [CHEAP, PRICEY], "hand": [STOCKPILE], "library": crate::filler(4) },
        }));
        s.play(ID, json!({}));
        s
    }

    fn defs(s: &Scenario) -> Vec<String> {
        (1..=2)
            .filter_map(|lane| s.unit(P2, lane).map(|unit| unit.def_id))
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r4586009_every_enemy_unit_costing_1_or_less_becomes_a_plain_husk() {
            let s = cast(false, "hallucination-base");
            assert_eq!(defs(&s), vec![HUSK, PRICEY]);
            let husk = s.unit(P2, 1).expect("the Husk");
            assert!(!husk.radiant, "a plain Husk");
            assert_eq!(unit_view(s.state(), &husk).attack, 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4586009_the_cap_is_2() {
            let s = cast(true, "hallucination-radiant");
            assert_eq!(defs(&s), vec![HUSK, HUSK]);
            assert!(s.state().players[P2].units.iter().flatten().flatten().all(|card| !card.radiant));
        }
    }
}
