//! SPEC §8.14 FvH #103 Frantic Forager ((1) Unit, Human, Common, 2/2 → 4/4), a card of the Felinors vs
//! Humans set (R5235000).
//!   Base:    "Cry: Add a random Spell you can afford to your hand. Give it Temporary {temporary}." (1)
//!   Radiant: "Cry: Add a random Radiant Spell you can afford to your hand. Give it Temporary
//!            {temporary}." (1)
//!
//! Cry (R1): `add_random_from_catalog` of a non-token Spell of the sets that ship whose printed cost
//! (R65, an X as 0) is at most the mana its controller has left as the Cry resolves (`unspent_mana_of`),
//! the Forager's own price already paid; the designer's "playable" is that price alone (R5261001), so a
//! Spell with no legal target now is still offered. The card is Temporary `temporary` in the hand
//! (`temporary_count`, R5238000): discarded at the end of its owner's turn, this one for Temporary 1. A
//! full hand burns it (§2.4). Radiant on the Radiant face. Built by S11 (#5261).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-103";

fn forager(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            // R5261001: what "you can afford" reads, the mana left as the Cry resolves.
            let mana = unspent_mana_of(ctx.state, ctx.controller).max(0);
            vec![add_random_from_catalog(json_as(json!({
                "query": { "type": "Spell", "costRange": { "max": mana } },
                "radiant": radiant,
                "temporaryCount": param(&*ctx, "temporary"),
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: forager(false),
        radiant: forager(true),
    }
}

// FvH #103 Frantic Forager — SPEC §8.14 row 103 (R5261001, R5238000).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself
    /// A play open at any mana, so p1's turn never ends by itself; a Unit, so never the Spell added.
    const KEEPER: &str = "stand-in (0) 1/1";

    /// Plays the Forager with `mana` in the pool and answers the card it added.
    fn forage(radiant: bool, mana: i32, seed: &str) -> (Scenario, CardInstance) {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, KEEPER], "mana": mana, "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(ID, json!({ "zone": 1 }));
        let added = must(s.hand(P1).into_iter().find(|card| card.def_id != KEEPER), "the added Spell");
        (s, added)
    }

    /// Over a few seeds, the added card is a non-token Spell printed at most at the mana left.
    fn adds_an_affordable_spell(radiant: bool, mana: i32) {
        let left = mana - 1;
        for seed in 0..8 {
            let (s, added) = forage(radiant, mana, &format!("frantic-forager-{radiant}-{mana}-{seed}"));
            let def = def_of(Some(s.state()), &added.def_id);
            assert_eq!(def.type_, CardType::Spell, "{}", def.id);
            assert!(query_cost(def) <= left, "{} costs at most the {left} mana left", def.id);
            assert!(!def.is_token(), "{}", def.id);
            assert_eq!(added.radiant, radiant, "{}", def.id);
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r5261001_it_adds_a_random_spell_you_can_afford_with_the_mana_left() {
            adds_an_affordable_spell(false, 3);
        }

        #[test]
        fn r5261001_with_no_mana_left_the_spell_costs_0() {
            adds_an_affordable_spell(false, 1);
        }

        #[test]
        fn r5238000_the_spell_is_temporary_1_discarded_at_the_end_of_this_turn() {
            let (mut s, added) = forage(false, 3, "frantic-forager-temporary");
            s.expect_in_zone(&added, "hand");
            s.end_turn();
            s.expect_in_zone(&added, "graveyard");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5261001_it_adds_a_random_radiant_spell_you_can_afford() {
            adds_an_affordable_spell(true, 4);
        }

        #[test]
        fn r5238000_the_radiant_spell_is_temporary_1_too() {
            let (mut s, added) = forage(true, 4, "frantic-forager-radiant-temporary");
            s.end_turn();
            s.expect_in_zone(&added, "graveyard");
        }
    }
}
