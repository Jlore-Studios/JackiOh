//! SPEC §8.12 S+ #165 Shadiris (§6.3 Cast, R70, R113, R129, R1420, R1840002): (3) Unit, Epic, 3/13 → 6/26, a
//! card of the Stinkyscape+ set. Built by S20 (#3568).
//!   Base:    "Cry: Cast Shadiris Visions twice and add three random Stinkyscape Resources to your hand."
//!   Radiant: "Cry: Cast Radiant Shadiris Visions twice and add three random Radiant Stinkyscape Resources
//!            to your hand."
//!
//! "Stinkscape" in the list is Stinkyscape (SP5), the six Resources `gainResources` rolls from (R1840002).
//! The two casts are two `castNew` effects of S+ #68 Shadiris Visions, free and in order (R70), each
//! opening its own Discover over the Spells of the caster's library and resolving, the pick copied into the
//! hand, before the next begins (R113: each effect pauses the list on its own, so each cast asks for its
//! own pick); a deck with no Spell offers nothing (R129) and the Cry goes on. Only then, the Resources: each
//! of the three rolled as it is gained, a full hand burning it (§2.4). The Radiant face casts the Radiant
//! Visions and gains Radiant Resources. The cast and the gain are printed in the words, so they are named
//! constants here, not declared numbers (R1748003).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-165";

/// S+ #68 Shadiris Visions: the Spell this Cry casts.
const VISIONS: &str = "stinkyscapeplus-068";
/// "Cast Shadiris Visions twice".
const CASTS: i32 = 2;
/// "add three random Stinkyscape Resources".
const RESOURCES: i32 = 3;

fn shadiris(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            let visions = CastNewArgs {
                def: CastNewDef::Def(CastDef {
                    def_id: VISIONS.to_string(),
                    radiant: Some(radiant),
                }),
                radiant: None,
                how: CastHow::default(),
            };
            // R3558008: one `castNew` per cast, never one cast of doubled strength: each asks its own targets.
            let mut effects = cast_new_times(visions, CASTS);
            effects.push(gain_resources(GainResourcesArgs {
                count: RESOURCES,
                radiant: radiant.then_some(true),
                ..GainResourcesArgs::default()
            }));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: shadiris(false),
        radiant: shadiris(true),
    }
}

// S+ #165 Shadiris — SPEC §8.12 row 165 (R113, R129, R1840002).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{resources_in_hand, scenario};

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-005"; // Stockpile: a Spell, so no turn ends by itself
    const SPELL_A: &str = "core-088"; // Twisting Nether: a Spell for Visions to find
    const SPELL_B: &str = "core-035"; // Lunar Eclipse: another
    const UNIT: &str = "stand-in (1) 1/1"; // a card that is no Spell
    const VISIONS: &str = "stinkyscapeplus-068"; // Shadiris Visions: the Spell the Cry casts

    fn board(radiant: bool, library: Value) -> Scenario {
        scenario(json!({
            "seed": "shadiris",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": library, "mana": 10 },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }))
    }

    /// The library cards the open Discover shows.
    fn offered(s: &Scenario) -> Vec<String> {
        must(s.state().pending.clone(), "an open prompt")
            .options
            .iter()
            .filter_map(|option| match &option.selection {
                Selection::Instance { instance_id } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    /// The copies of Spells Visions has added to p1's hand, by def, in hand order.
    fn copies(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1)
            .into_iter()
            .filter(|card| card.def_id == SPELL_A || card.def_id == SPELL_B)
            .collect()
    }

    /// How many casts of Visions the game has seen so far.
    fn visions_casts(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::CardPlayed { def_id, .. } if def_id == VISIONS))
            .count()
    }

    #[test]
    fn casts_visions_twice_each_with_its_own_discover_and_gains_three_resources() {
        crate::register_all();
        let mut s = board(false, json!([SPELL_A, SPELL_B, UNIT, UNIT]));
        s.play(ID, json!({}));
        // The first cast's Discover: both Spells of the deck.
        let first = offered(&s);
        assert_eq!(first.len(), 2, "both Spells of the deck are offered");
        s.answer(json!(first[0]));
        // Its copy is in the hand, and the second cast now asks for its own pick before anything else.
        assert_eq!(copies(&s).len(), 1);
        assert!(
            resources_in_hand(&s, P1).is_empty(),
            "the Resources wait for both casts"
        );
        let second = offered(&s);
        assert_eq!(
            second.len(),
            2,
            "the pick stays in the deck, so the second Discover offers it again"
        );
        s.answer(json!(second[1]));
        assert!(s.state().pending.is_none());
        assert_eq!(copies(&s).len(), 2, "two casts, two copies");
        assert_eq!(visions_casts(&s), 2);
        // The originals stay in the deck (R57).
        let library = s.pile(P1, "library");
        assert_eq!(library.iter().filter(|card| card.def_id == SPELL_A).count(), 1);
        assert_eq!(library.iter().filter(|card| card.def_id == SPELL_B).count(), 1);
        // Three Resources, none Radiant.
        let resources = resources_in_hand(&s, P1);
        assert_eq!(resources.len(), 3);
        assert!(resources.iter().all(|card| !card.radiant));
        assert!(copies(&s).iter().all(|card| !card.radiant && card.cost_mod == 0));
    }

    #[test]
    fn the_same_spell_may_be_picked_twice() {
        crate::register_all();
        let mut s = board(false, json!([SPELL_A, UNIT, UNIT]));
        s.play(ID, json!({}));
        let first = offered(&s);
        assert_eq!(first.len(), 1);
        s.answer(json!(first[0]));
        let second = offered(&s);
        assert_eq!(second, first, "the one Spell is offered again");
        s.answer(json!(second[0]));
        assert_eq!(copies(&s).iter().filter(|card| card.def_id == SPELL_A).count(), 2);
    }

    #[test]
    fn radiant_casts_radiant_visions_twice_and_gains_radiant_resources() {
        crate::register_all();
        let mut s = board(true, json!([SPELL_A, SPELL_B, UNIT]));
        s.play(ID, json!({}));
        let first = offered(&s);
        s.answer(json!(first[0]));
        let second = offered(&s);
        s.answer(json!(second[0]));
        assert!(s.state().pending.is_none());
        // Radiant Visions: each copy is Radiant and costs (1) less.
        let made = copies(&s);
        assert_eq!(made.len(), 2);
        assert!(made.iter().all(|card| card.radiant && card.cost_mod == -1));
        // Three Radiant Resources.
        let resources = resources_in_hand(&s, P1);
        assert_eq!(resources.len(), 3);
        assert!(resources.iter().all(|card| card.radiant));
    }

    #[test]
    fn a_spell_less_deck_still_gains_resources() {
        crate::register_all();
        let mut s = board(false, json!([UNIT, UNIT, UNIT]));
        s.play(ID, json!({}));
        // R129: nothing to discover, so no prompt opens and the Cry goes on to the Resources.
        assert!(s.state().pending.is_none());
        assert!(copies(&s).is_empty());
        assert_eq!(resources_in_hand(&s, P1).len(), 3);
    }

    #[test]
    fn radiant_in_a_spell_less_deck_still_gains_radiant_resources() {
        crate::register_all();
        let mut s = board(true, json!([UNIT, UNIT, UNIT]));
        s.play(ID, json!({}));
        assert!(s.state().pending.is_none());
        let resources = resources_in_hand(&s, P1);
        assert_eq!(resources.len(), 3);
        assert!(resources.iter().all(|card| card.radiant));
    }
}
