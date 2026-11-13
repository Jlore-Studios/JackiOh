//! SPEC §8.12 S+ #103 Transcend, a card of the Stinkyscape+ set (R1420, R3535000). (1) Spell, Common.
//!   Base:    "Transform your permanents into ones that cost (1) more."
//!   Radiant: "Transform your permanents into ones that cost (2) more."
//!
//! Each permanent of the caster's is a card on the field, a Unit or a backrow card (§5.1). `transform_cost_step`
//! replaces each in place by a card of its own type that costs `STEP` more, its face kept (R3557004): one
//! drawn with the match rng from the shipped sets' non-token cards of that type and price, the closest
//! higher price when there is none, nothing when there is none dearer (R129); no Cry (R691), an Immutable
//! card stays as it is (R23), a Dormant one is passed by (R1839000). The step is the printed (1) and (2)
//! (R1748003: declared nowhere).

use jackioh_engine::effects::transform_cost_step;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-103";

/// "(1) more" on the base face, "(2) more" on the Radiant face.
const BASE_STEP: i32 = 1;
const RADIANT_STEP: i32 = 2;

fn transcend(step: i32) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![transform_cost_step(json_as(json!({
                "scope": { "side": "self", "zones": ["field"] },
                "step": step,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: transcend(BASE_STEP),
        radiant: transcend(RADIANT_STEP),
    }
}

// S+ #103 Transcend — SPEC §8.12 row 103 (R3557004).
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

    fn board(radiant: bool, field: Value) -> Scenario {
        scenario(json!({
            "seed": "transcend",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "field": field, "library": filler(3), "mana": 10 },
            "p2": { "hand": [FILLER], "field": ["core-019"], "library": filler(4) },
        }))
    }

    fn cost_of(s: &Scenario, id: &str) -> i32 {
        printed(&s.card(id).def_id)
    }

    #[test]
    fn r3557004_each_of_your_permanents_becomes_one_that_costs_one_more() {
        crate::register_all();
        let mut s = board(false, json!(["core-019", "core-008"]));
        let before: Vec<(String, i32)> = [1, 2]
            .map(|lane| s.unit_at(P1, lane).id)
            .into_iter()
            .map(|id| (id.clone(), cost_of(&s, &id)))
            .collect();
        let enemy = s.unit_at(P2, 1).id;
        s.play(ID, json!({}));
        for (lane, (old, cost)) in [1, 2].into_iter().zip(before) {
            let new = s.unit(P1, lane).expect("a Unit stands there");
            assert_ne!(new.id, old, "replaced in place");
            assert_eq!(printed(&new.def_id), cost + 1);
            assert_eq!(card_type_of(s.state(), &new), CardType::Unit);
        }
        assert_eq!(s.unit_at(P2, 1).id, enemy, "the opponent's Unit is left alone");
    }

    #[test]
    fn r3557004_radiant_costs_two_more_and_keeps_the_face() {
        crate::register_all();
        let mut s = board(true, json!([{ "def": "core-008", "radiant": true }]));
        let old = s.unit_at(P1, 1).id;
        let cost = cost_of(&s, &old);
        s.play(ID, json!({}));
        let new = s.unit(P1, 1).expect("a Unit stands there");
        assert_eq!(printed(&new.def_id), cost + 2);
        assert!(new.radiant, "its face is kept");
    }

    /// The distinct printed prices of the Units the pool reaches, ascending.
    fn unit_prices() -> Vec<i32> {
        let mut prices: Vec<i32> = jackioh_engine::catalog::query(&json_as(json!({ "type": CardType::Unit })))
            .into_iter()
            .map(jackioh_engine::catalog::query_cost)
            .collect();
        prices.sort_unstable();
        prices.dedup();
        prices
    }

    /// A shipped Unit of this printed price.
    fn unit_costing(price: i32) -> String {
        jackioh_engine::catalog::query(&json_as(json!({ "type": CardType::Unit, "cost": price })))
            .first()
            .map(|def| def.id.clone())
            .expect("a Unit of that price")
    }

    #[test]
    fn r3557004_with_nothing_at_the_price_the_closest_higher_one_is_used() {
        crate::register_all();
        let prices = unit_prices();
        let below_gap = *prices
            .iter()
            .find(|price| !prices.contains(&(**price + 1)) && prices.iter().any(|other| other > *price))
            .expect("a gap in the curve");
        let next = *prices.iter().find(|price| **price > below_gap).expect("a dearer price");
        let mut s = board(false, json!([{ "def": unit_costing(below_gap) }]));
        s.play(ID, json!({}));
        let new = s.unit(P1, 1).expect("a Unit stands there");
        assert_eq!(printed(&new.def_id), next, "nothing costs {} more than {below_gap}", 1);
    }

    #[test]
    fn r3557004_with_nothing_dearer_the_unit_stays_as_it_is() {
        crate::register_all();
        let top = *unit_prices().last().expect("a Unit");
        let mut s = board(false, json!([{ "def": unit_costing(top) }]));
        let old = s.unit_at(P1, 1).id;
        s.play(ID, json!({}));
        assert_eq!(s.unit_at(P1, 1).id, old, "the top of the curve is unchanged");
    }

    #[test]
    fn r23_an_immutable_unit_is_not_transformed() {
        crate::register_all();
        let mut s = board(false, json!([{ "def": "core-066", "radiant": true }]));
        let old = s.unit_at(P1, 1).id;
        s.play(ID, json!({}));
        assert_eq!(s.unit_at(P1, 1).id, old);
    }
}
