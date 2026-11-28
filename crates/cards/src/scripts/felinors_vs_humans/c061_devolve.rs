//! FvH #61 Devolve (SPEC §8.14 row 61): (1) Spell, Rare, of the Felinors vs Humans set.
//!   Base:    "Transform all enemy Units into ones that cost (1) less."
//!   Radiant: "Transform all enemy Units into ones that cost (2) less."
//!
//! `transform_cost_step` over the enemy's Units on the field with a negative step, Hearthstone's Devolve
//! (FV2 #61): each is replaced in place by a Unit of the shipped sets' non-token cards at its printed
//! price less the step, else the closest cheaper price, a (0) becoming a random (0) (R5238010, the mirror
//! of R3557004's closest higher), keeping its battle position and face and running no Cry (R691, R1). An
//! Immutable Unit stays (R23), a Unit Immune to Spells is passed by (R4453000) and a Dormant card is not
//! reached (R1839000). Each replace is a `Transformed` event. The (1) and (2) are prices, declared
//! nowhere (R1748003).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-061";

/// "(1) less" on the base face, "(2) less" on the Radiant face.
const BASE_STEP: i32 = -1;
const RADIANT_STEP: i32 = -2;

fn devolve(step: i32) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![transform_cost_step(json_as(json!({
                "scope": { "side": "enemy", "zones": ["field"], "types": ["Unit"] },
                "step": step,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: devolve(BASE_STEP),
        radiant: devolve(RADIANT_STEP),
    }
}

// FvH #61 Devolve — SPEC §8.14 row 61 (R5238010).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    /// Stockpile: a Spell each hand keeps.
    const FILLER: &str = "core-005";
    /// Midrange Menace: the Unit each side stands in lane 1.
    const MENACE: &str = "core-019";

    /// A card's printed price, the one the pool of a cost-step Transform reads (R3557004, R5238010).
    fn printed(def_id: &str) -> i32 {
        jackioh_engine::catalog::query_cost(catalog::def_of(None, def_id))
    }

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "devolve",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "field": [MENACE], "library": filler(4) },
            "p2": { "hand": [FILLER], "field": [MENACE, { "def": MENACE, "radiant": true }], "library": filler(4) },
        }))
    }

    /// p1 plays Devolve on `radiant`'s face: each enemy Unit stands replaced by one `less` cheaper, its face
    /// kept, and p1's own Unit is left alone.
    fn devolves_by(radiant: bool, less: i32) {
        let mut s = board(radiant);
        let mine = s.unit_at(P1, 1).id;
        let before: Vec<(String, i32, bool)> = [1, 2]
            .map(|lane| s.unit_at(P2, lane))
            .into_iter()
            .map(|unit| {
                let cost = printed(&unit.def_id);
                (unit.id, cost, unit.radiant)
            })
            .collect();
        s.play(ID, json!({}));
        for (lane, (old, cost, face)) in [1, 2].into_iter().zip(before) {
            let new = s.unit_at(P2, lane);
            assert_ne!(new.id, old, "replaced in place");
            assert_eq!(printed(&new.def_id), cost - less, "a Unit that costs ({less}) less");
            assert_eq!(card_type_of(s.state(), &new), CardType::Unit);
            assert_eq!(new.radiant, face, "its face is kept");
        }
        assert_eq!(s.unit_at(P1, 1).id, mine, "the caster's own Unit is left alone");
    }

    #[test]
    fn r5238010_base_transforms_each_enemy_unit_into_one_that_costs_1_less() {
        devolves_by(false, 1);
    }

    #[test]
    fn r5238010_radiant_transforms_each_enemy_unit_into_one_that_costs_2_less() {
        devolves_by(true, 2);
    }
}
