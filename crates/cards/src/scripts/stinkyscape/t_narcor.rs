//! SPEC §7 Stinkyscape T-narcor Narcor, a Resource token of the Stinkyscape set.
//!   Base:    "Add a random Rare card to your hand."
//!   Radiant: "Add a random Radiant Rare card to your hand."
//!
//! A Legendary Resource (R1840000), played from its owner's hand at (0), which pays its quote in Credits
//! (R2250000, R2250001);
//! S #39 The Narcor Project makes them. The pool is every non-token Rare card of the sets that ship
//! (R380, R1420), drawn uniformly (R60); a full hand burns the card (§2.4). Radiant: the card is Radiant.

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-narcor";

fn face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "rarity": "Rare" },
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// S T-narcor — SPEC §7, BUILD M12 row S T-narcor.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{card_def, filler, play_resource, scenario};

    const P1: PlayerId = PlayerId::P1;

    fn played(seed: &str, radiant: bool, hand: usize) -> Scenario {
        let mut cards = vec![json!({ "def": ID, "radiant": radiant })];
        cards.extend(filler(hand).as_array().cloned().unwrap_or_default());
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": cards },
        }));
        play_resource(&mut s, P1, ID, Value::Null);
        s
    }

    #[test]
    fn a_rare_non_token() {
        for seed in (0..10).map(|n| format!("narcor-{n}")) {
            let s = played(&seed, false, 1);
            let hand = s.hand(P1);
            assert_eq!(hand.len(), 2, "{seed}: one card added");
            let added = &hand[1];
            let def = card_def(&added.def_id);
            assert_eq!(def.rarity, Rarity::Rare, "{seed}: {} is Rare", def.id);
            assert!(!def.token, "{seed}: {} is no token", def.id);
            assert!(set_is_open(def.set), "{seed}: {} is of a set that ships", def.id);
            assert!(!added.radiant, "{seed}: the base face adds a base card");
        }
    }

    #[test]
    fn full_hand_burns() {
        let s = played("narcor-full", false, HAND_CAP as usize);
        assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
        let burned: Vec<CardInstance> = s
            .pile(P1, "graveyard")
            .into_iter()
            .filter(|card| card.def_id != ID)
            .collect();
        assert_eq!(burned.len(), 1, "the added card is burned: {burned:?}");
        assert_eq!(card_def(&burned[0].def_id).rarity, Rarity::Rare);
    }

    #[test]
    fn radiant_a_radiant_one() {
        let s = played("narcor-radiant", true, 1);
        let added = &s.hand(P1)[1];
        assert_eq!(card_def(&added.def_id).rarity, Rarity::Rare);
        assert!(added.radiant);
    }
}
