//! SPEC §7 Stinkyscape T-narcor Narcor, a Resource token of the Stinkyscape set (§7's token table).
//!   Base:    "Add a random Legendary card to your hand."
//!   Radiant: "Add a random Radiant Legendary card to your hand."
//!
//! A Legendary Resource (R1840000), tagged Resource and Token (§5's tags, issue #2521), played from its
//! owner's hand at (0), which pays its quote in Credits (R2250000, R2250001); S #39 The Narcor Project
//! makes them. The pool is every non-token Legendary card of the sets that ship (R380, R1420): the
//! query's `rarity` is that rarity alone, so never a Mythic, and a pool that names no token never
//! holds one (§5.1). Drawn uniformly (R60); a full hand burns the card (§2.4, R4). Radiant: the card is
//! added on its Radiant face. Issue #2521 raised the pool from Rare to Legendary; the Rare pool v0.5.11
//! and every version before it drew from is `script_v0_5_11`'s (R767000).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-narcor";

fn face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "rarity": "Legendary" },
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

/// R767000: a face as v0.5.11 ran it, adding a random Rare card.
fn face_v0_5_11(radiant: bool) -> Script {
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

/// R767000: through v0.5.11 it added a random Rare card; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_11() -> CardScripts {
    CardScripts {
        base: face_v0_5_11(false),
        radiant: face_v0_5_11(true),
    }
}

// S T-narcor — SPEC §7, BUILD M12 row S T-narcor (issue #2521): played from the hand, adds one random
// non-token Legendary card of a set that ships to the hand, never a Mythic, a full hand burning it;
// radiant a Radiant one. Tagged Resource and Token.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{card_def, filler, play_resource, scenario};

    const P1: PlayerId = PlayerId::P1;

    /// The Narcor first in a hand of it and `hand` fillers (R2250000), played.
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

    /// The one card the play added, after checking it is a non-token Legendary of a set that ships.
    fn added(seed: &str, s: &Scenario) -> CardInstance {
        let hand = s.hand(P1);
        assert_eq!(hand.len(), 2, "{seed}: one card added");
        let added = hand[1].clone();
        let def = card_def(&added.def_id);
        assert_eq!(def.rarity, Rarity::Legendary, "{seed}: {} is Legendary", def.id);
        assert_ne!(def.rarity, Rarity::Mythic, "{seed}: {} is no Mythic", def.id);
        assert!(!def.token, "{seed}: {} is no token", def.id);
        assert!(!def.tags.contains(&Tag::Token), "{seed}: {} has no Token tag", def.id);
        assert!(set_is_open(def.set), "{seed}: {} is of a set that ships", def.id);
        added
    }

    #[test]
    fn tagged_resource_and_token() {
        assert_eq!(card_def(ID).tags, vec![Tag::Resource, Tag::Token]);
    }

    #[test]
    fn a_legendary_non_token() {
        let mut seen = Vec::new();
        for seed in (0..20).map(|n| format!("narcor-{n}")) {
            let s = played(&seed, false, 1);
            let card = added(&seed, &s);
            assert!(!card.radiant, "{seed}: the base face adds a base card");
            seen.push(card.def_id);
        }
        seen.sort();
        seen.dedup();
        assert!(seen.len() > 1, "a random pick, not one card: {seen:?}");
    }

    #[test]
    fn full_hand_burns() {
        // A hand past its cap, which a setup may hold: the Narcor played, it is still full.
        let s = played("narcor-full", false, HAND_CAP as usize);
        assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
        let burned: Vec<CardInstance> = s
            .pile(P1, "graveyard")
            .into_iter()
            .filter(|card| card.def_id != ID)
            .collect();
        assert_eq!(burned.len(), 1, "the added card is burned: {burned:?}");
        assert_eq!(card_def(&burned[0].def_id).rarity, Rarity::Legendary);
    }

    /// R767000: the script kept for the replays of v0.5.11 and before added a non-token Rare card.
    #[test]
    fn r767000_the_script_kept_for_v0_5_11_adds_a_rare() {
        crate::under_kept(ID, super::script_v0_5_11(), || {
            let s = played("narcor-kept", false, 1);
            let hand = s.hand(P1);
            assert_eq!(hand.len(), 2, "one card added");
            let def = card_def(&hand[1].def_id);
            assert_eq!(def.rarity, Rarity::Rare, "{} is Rare", def.id);
            assert!(!def.token, "{} is no token", def.id);
        });
    }

    mod radiant {
        use super::*;

        #[test]
        fn a_radiant_legendary_non_token() {
            for seed in (0..10).map(|n| format!("narcor-radiant-{n}")) {
                let s = played(&seed, true, 1);
                let card = added(&seed, &s);
                assert!(card.radiant, "{seed}: added on its Radiant face");
            }
        }

        #[test]
        fn full_hand_burns_a_legendary() {
            let s = played("narcor-radiant-full", true, HAND_CAP as usize);
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            let burned: Vec<CardInstance> = s
                .pile(P1, "graveyard")
                .into_iter()
                .filter(|card| card.def_id != ID)
                .collect();
            assert_eq!(burned.len(), 1, "the added card is burned: {burned:?}");
            assert_eq!(card_def(&burned[0].def_id).rarity, Rarity::Legendary);
        }
    }
}
