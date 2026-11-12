//! SPEC §8.12 S+ #5 Dingleberry Droat (2) Unit, Human, Acclaimed, Legendary, 2/1 → 4/2, a card of the
//! Stinkyscape+ set (R1420; R3558004).
//!   Base:    "Cry: Fuse Fruits together until you make one that costs at least (5). Add it to your hand.
//!            It costs (2)."
//!   Radiant: "Cry: Fuse Radiant Fruits together until you make one that costs at least (5). Add it to
//!            your hand. It costs (2)."
//!
//! Engine: E8's `fuseGenerated` with `untilCost` (R3558004): random Fruits of the shipped sets (R1420),
//! never this card (R387), are drawn one by one until their printed costs reach 5 — at least two picks,
//! at most `FUSED_CARDS_CAP` — and fused into one new card (R77) that goes to its controller's hand
//! costing 2 there (`costOverride`, as NA Tech's, R1183001); a full hand burns it (§2.4). The Radiant
//! face fuses the Radiant faces (R469). The numbers are printed, so the script names them.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-005";

/// "until you make one that costs at least (5)".
const UNTIL_COST: i32 = 5;
/// "It costs (2)".
const HAND_COST: i32 = 2;

fn droat(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![fuse_generated(json_as(json!({
                "query": { "tags": ["Fruit"] },
                "untilCost": UNTIL_COST,
                "costOverride": HAND_COST,
                "toHand": "self",
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: droat(false),
        radiant: droat(true),
    }
}

// S+ #5 Dingleberry Droat — SPEC §8.12 row 5 (R3558004).
#[cfg(test)]
mod tests {
    use jackioh_engine::subsystems::fuse::fused_ingredients;
    use jackioh_engine::testkit::*;

    use super::{HAND_COST, ID, UNTIL_COST};
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    /// p1 plays the Droat (`radiant` its face): the fused card it adds, and the game.
    fn played(seed: &str, radiant: bool) -> (Scenario, CardInstance) {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, "core-005"], "library": filler(4), "mana": 10 },
            "p2": { "hand": ["core-005"], "library": filler(4) },
        }));
        s.play(ID, json!({ "zone": 1 }));
        let fused = s
            .hand(P1)
            .into_iter()
            .find(|card| fused_ingredients(s.state(), &card.def_id).is_some())
            .expect("the fused card");
        (s, fused)
    }

    fn fruits(s: &Scenario, fused: &CardInstance) -> Vec<String> {
        let ingredients = fused_ingredients(s.state(), &fused.def_id).expect("a fusion");
        assert!(ingredients.len() >= FUSE_MIN_INGREDIENTS, "{ingredients:?}");
        for id in &ingredients {
            let def = def_of(Some(s.state()), id);
            assert!(def.tags.contains(&Tag::Fruit), "{id}");
            assert!(set_ships(def.set), "{id}");
            assert_ne!(id, ID);
        }
        ingredients
    }

    #[test]
    fn r3558004_its_cry_fuses_fruits_until_they_cost_5_and_the_card_costs_2() {
        for n in 0..6 {
            let (s, fused) = played(&format!("r3558004-droat-{n}"), false);
            let ingredients = fruits(&s, &fused);
            let sum: i32 = ingredients
                .iter()
                .map(|id| query_cost(def_of(Some(s.state()), id)))
                .sum();
            assert!(
                sum >= UNTIL_COST || ingredients.len() == FUSED_CARDS_CAP,
                "{n}: printed costs reach 5: {ingredients:?}"
            );
            assert_eq!(fused.cost_override, Some(HAND_COST));
            assert_eq!(effective_cost(s.state(), &fused, Default::default()), HAND_COST);
            assert!(!fused.radiant);
        }
    }

    #[test]
    fn r3558004_radiant_fuses_radiant_fruits() {
        let (s, fused) = played("r3558004-droat-radiant", true);
        let fused_def = def_of(Some(s.state()), &fused.def_id).clone();
        for id in fruits(&s, &fused) {
            assert!(fused_def.base.text.contains(&crate::card_def(&id).radiant.text), "{id}");
        }
        assert_eq!(effective_cost(s.state(), &fused, Default::default()), HAND_COST);
    }
}
