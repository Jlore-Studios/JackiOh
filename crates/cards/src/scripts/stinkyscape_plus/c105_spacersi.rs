//! SPEC §8.12 S+ #105 Spacersi, a card of the Stinkyscape+ set (R1420, R3535000). (3) Unit,
//! Common, 5/8 → 10/16.
//!   Base:    "Cry: Transform your other Units into ones that cost (1) more."
//!   Radiant: "Cry: Transform your other Units into ones that cost (2) more."
//!
//! Cry (R1): once Spacersi stands, every other Unit of its controller's is replaced in place by a Unit
//! that costs `STEP` more (`transform_cost_step`, R3557004). Spacersi spares itself (`excludeSelf`); a
//! Unit standing in a backrow zone is a Unit (R1081000). The stats are catalog data.

use jackioh_engine::effects::transform_cost_step;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-105";

/// "(1) more" on the base face, "(2) more" on the Radiant face.
const BASE_STEP: i32 = 1;
const RADIANT_STEP: i32 = 2;

fn spacersi(step: i32) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![transform_cost_step(json_as(json!({
                "scope": { "side": "self", "zones": ["field"], "types": ["Unit"], "excludeSelf": true },
                "step": step,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: spacersi(BASE_STEP),
        radiant: spacersi(RADIANT_STEP),
    }
}

// S+ #105 Spacersi — SPEC §8.12 row 105 (R3557004).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005";

    /// A card's printed price, the one the pool of a cost-step Transform reads (R3557004).
    fn printed(def_id: &str) -> i32 {
        jackioh_engine::catalog::query_cost(catalog::def_of(None, def_id))
    }

    fn played(radiant: bool) -> (Scenario, Vec<(String, i32)>, String) {
        let mut s = scenario(json!({
            "seed": "spacersi",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "field": ["core-008", "core-019"], "library": filler(3), "mana": 10 },
            "p2": { "hand": [FILLER], "field": ["core-008"], "library": filler(4) },
        }));
        let before = [1, 2]
            .map(|lane| s.unit_at(P1, lane).id)
            .into_iter()
            .map(|id| {
                let cost = printed(&s.card(&id).def_id);
                (id, cost)
            })
            .collect();
        let enemy = s.unit_at(P2, 1).id;
        s.play(ID, json!({ "zone": 3 }));
        (s, before, enemy)
    }

    #[test]
    fn r3557004_each_other_unit_of_yours_becomes_one_that_costs_one_more_and_it_spares_itself() {
        crate::register_all();
        let (s, before, enemy) = played(false);
        for (lane, (old, cost)) in [1, 2].into_iter().zip(before) {
            let new = s.unit(P1, lane).expect("a Unit stands there");
            assert_ne!(new.id, old);
            assert_eq!(printed(&new.def_id), cost + 1);
        }
        assert_eq!(s.unit(P1, 3).map(|card| card.def_id), Some(ID.to_string()), "Spacersi itself stays");
        assert_eq!(s.unit_at(P2, 1).id, enemy);
    }

    #[test]
    fn r3557004_radiant_costs_two_more() {
        crate::register_all();
        let (s, before, _) = played(true);
        for (lane, (_, cost)) in [1, 2].into_iter().zip(before) {
            let new = s.unit(P1, lane).expect("a Unit stands there");
            assert_eq!(printed(&new.def_id), cost + 2);
        }
    }
}
