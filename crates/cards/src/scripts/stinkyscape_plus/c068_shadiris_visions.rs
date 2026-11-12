//! SPEC §8.12 S+ #68 Shadiris Visions, a card of the Stinkyscape+ set (R1420, R3535000). (1) Spell,
//! Rare.
//!   Base:    "Discover a Spell in your deck; add a copy of it to your hand."
//!   Radiant: "Discover a Spell in your deck; add a Radiant copy of it to your hand, it costs (1) less."
//!
//! One Discover of three (§6.3) over the Spells of the caster's library (`discover_from_library`): its
//! options are library cards shown to their owner alone (§10.8), the count of them the owner's secret
//! (R2739000). The chosen card stays in the deck and a fresh copy of it reaches the hand (`add_to_hand`
//! with `copyOf`, R57's riders, a full hand burning it, §2.4). The Radiant face's copy is Radiant and
//! carries a cost change of `DISCOUNT` less, never below (0) (R65). A deck with no Spell offers
//! nothing (R129).

use jackioh_engine::effects::{add_to_hand, discover_from_library};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-068";

/// The resume step the Discover's answer comes back to (§10.6).
const PICKED: &str = "picked";
/// §6.3 Discover: 1 of 3, so three Spells of the deck are shown.
const REVEAL_COUNT: i32 = 3;
/// "it costs (1) less" on the Radiant face (R1748003: printed, declared nowhere).
const DISCOUNT: i32 = 1;

fn visions(radiant: bool) -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![discover_from_library(json_as(json!({
                "step": PICKED,
                "count": REVEAL_COUNT,
                "player": "self",
                "filter": { "type": [CardType::Spell] },
                "prompt": "Discover a Spell in your deck",
            })))]
        })),
        resume: IndexMap::from([(
            PICKED,
            hook(move |ctx| {
                let Some(Selection::Instance { instance_id }) = ctx.targets.first() else {
                    return vec![];
                };
                let mut args = json!({ "copyOf": instance_id });
                if radiant {
                    args["radiant"] = json!(true);
                    args["costMod"] = json!(-DISCOUNT);
                }
                vec![add_to_hand(json_as(args))]
            }),
        )]),
        secret_option_counts: true,
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: visions(false),
        radiant: visions(true),
    }
}

// S+ #68 Shadiris Visions — SPEC §8.12 row 68.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";
    const SPELL: &str = "core-088"; // Twisting Nether
    const UNIT: &str = "stand-in (1) 1/1"; // a card that is no Spell

    fn board(radiant: bool, library: Value) -> Scenario {
        scenario(json!({
            "seed": "shadiris",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": library, "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    fn visions(s: &mut Scenario) {
        s.play(ID, json!({}));
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

    #[test]
    fn base_adds_a_plain_copy_and_the_original_stays_in_the_deck() {
        crate::register_all();
        let mut s = board(false, json!([SPELL, UNIT, UNIT]));
        visions(&mut s);
        let options = offered(&s);
        assert_eq!(options.len(), 1, "one Spell in the deck: one option");
        s.answer(json!(options[0]));
        let copy = s.hand(P1).into_iter().find(|card| card.def_id == SPELL).expect("the copy");
        assert!(!copy.radiant);
        assert_eq!(copy.cost_mod, 0);
        assert_eq!(s.state().players[P1].library.iter().filter(|card| card.def_id == SPELL).count(), 1);
    }

    #[test]
    fn radiant_adds_a_radiant_copy_that_costs_one_less() {
        crate::register_all();
        let mut s = board(true, json!([SPELL, UNIT, UNIT]));
        visions(&mut s);
        let options = offered(&s);
        s.answer(json!(options[0]));
        let copy = s.hand(P1).into_iter().find(|card| card.def_id == SPELL).expect("the copy");
        assert!(copy.radiant);
        assert_eq!(copy.cost_mod, -1);
        assert_eq!(s.state().players[P1].library.iter().filter(|card| card.def_id == SPELL).count(), 1);
    }

    #[test]
    fn a_deck_without_a_spell_offers_nothing() {
        crate::register_all();
        let mut s = board(false, json!([UNIT, UNIT, UNIT]));
        let hand = s.hand(P1).len();
        visions(&mut s);
        assert!(s.state().pending.is_none());
        assert_eq!(s.hand(P1).len(), hand - 1);
    }
}
