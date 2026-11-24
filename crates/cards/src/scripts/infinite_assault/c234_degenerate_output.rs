//! IA #234 Degenerate Output (SPEC §8.13 row 234), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Transform every enemy Unit into a random Unit whose Cost is ({step}) lower." (1)
//!   Radiant: "Transform every enemy Unit into a random Unit whose Cost is ({step}) lower." (2)
//!
//! Each enemy Unit a Spell may reach is transformed, in lane order, into a random Unit costing its
//! printed cost minus the declared `step` — drawn from the shipped sets' Units (R380) — keeping
//! its face (R4577006). Below (0) is no pool, so the Unit stays (R61, R129, R4594012).
//!
//! Engine: `transformRandom` (R4594012). Built by C10 (#4594).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-234";

fn degenerate() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            let enemy = opponent_of(ctx.controller);
            let source = ctx.self_.clone();
            let step = param(&*ctx, "step");
            active_units_of(&*ctx.state, enemy)
                .into_iter()
                .filter(|card| !spell_cannot_reach(&*ctx.state, source.as_ref(), card))
                .filter_map(|card| {
                    // The played card's printed cost minus the step; below (0) is no pool.
                    let printed = query_cost(def_of(Some(&*ctx.state), &card.def_id));
                    let cost = printed - step;
                    if cost < 0 {
                        return None;
                    }
                    Some(transform_random(json_as(json!({
                        "instanceId": card.id,
                        "query": { "type": "Unit", "cost": cost },
                        "radiant": "keep",
                    }))))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The same script: the Radiant face's (2) is its declared `step`.
    let base = degenerate();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #234 Degenerate Output — SPEC §8.13 row 234 (R4594012).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P2;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const TWO: &str = "stand-in (2) 3/3";
    const ZERO: &str = "stand-in (0) 1/1";
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn cost_of(s: &Scenario, lane: i32) -> i32 {
        let unit = s.unit_at(P2, lane);
        query_cost(def_of(Some(s.state()), &unit.def_id))
    }

    fn cast(seed: &str, radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, STOCKPILE], "library": crate::filler(4) },
            "p2": { "field": [TWO, ZERO], "hand": [STOCKPILE], "library": crate::filler(4) },
        }));
        s.play(ID, json!({}));
        s
    }

    mod base {
        use super::*;

        #[test]
        fn r4594012_each_unit_costs_1_less_a_0_unit_stays() {
            let s = cast("degenerate-base", false);
            assert_eq!(cost_of(&s, 1), 2 - 1, "the (2) is a (1) now");
            assert_eq!(s.unit_at(P2, 2).def_id, ZERO, "the (0) stays");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4594012_each_unit_costs_2_less() {
            let s = cast("degenerate-radiant", true);
            assert_eq!(cost_of(&s, 1), 2 - 2, "the (2) is a (0) now");
            assert_eq!(s.unit_at(P2, 2).def_id, ZERO, "the (0) stays");
        }
    }
}
