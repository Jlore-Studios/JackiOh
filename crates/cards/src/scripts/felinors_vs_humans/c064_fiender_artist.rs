//! FvH #64 Fiender Artist (SPEC §8.14 row 64): (2) Unit, Human, Common, 3/4 → 6/8. Built by S08 (#5258).
//!   Base:    "Cry: Discover a (3)+ Cost Unit. Give it the Felinor tag and cat ears."
//!   Radiant: "Cry: Discover a Radiant (3)+ Cost Unit. Give it the Felinor tag and cat ears."
//!
//! Core #7 Jewelosco Scarab's Discover-then-add: `discover_from_catalog` of three non-token Units whose
//! printed cost (R65) is (3) or more, from the sets that ship (R5235000, §6.3 Discover), never this card
//! (R387); the answer is added to its controller's hand, Radiant on the Radiant face. Then the card the
//! add put into the hand (`added_to_hand_so_far`, R5239000, read as the effect applies) is given the
//! Felinor tag (`grant_tag`): a granted tag every instance tag read sees and no catalog pool does (R923).
//! A full hand burns the pick and nothing is tagged (R4). The cat ears are the client's, drawn over a
//! card whose granted tags hold Felinor; no rule reads them. Declares no numbers: the (3) is a price.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-064";

/// The resume step the Discover's answer re-enters (§10.6).
const CHOSEN: &str = "chosen";

/// The printed "(3)+ Cost".
const MIN_COST: i32 = 3;

fn discover(radiant: bool) -> Effect {
    let prompt = if radiant {
        "Discover a Radiant (3)+ Cost Unit"
    } else {
        "Discover a (3)+ Cost Unit"
    };
    discover_from_catalog(json_as(json!({
        "step": CHOSEN,
        "query": { "type": "Unit", "costRange": { "min": MIN_COST } },
        "prompt": prompt,
    })))
}

/// The Felinor tag on every card this step has put into a hand: the one the add just landed.
fn tag_the_added() -> Effect {
    for_each_card(ForEachCardArgs {
        cards: Arc::new(|ctx: &mut EffectContext<'_>| added_to_hand_so_far(ctx)),
        each: Arc::new(|instance_id: &str| {
            grant_tag(GrantTagArgs {
                target: None,
                instance_id: Some(instance_id.to_string()),
                tag: Tag::Felinor,
            })
        }),
    })
}

fn fiender(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| vec![discover(radiant)])),
        resume: IndexMap::from([(
            CHOSEN,
            hook(move |ctx| {
                let Some(def_id) = chosen_options(ctx).into_iter().next() else {
                    return vec![];
                };
                vec![
                    add_to_hand(json_as(json!({ "defId": def_id, "radiant": radiant }))),
                    tag_the_added(),
                ]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: fiender(false),
        radiant: fiender(true),
    }
}

// FvH #64 Fiender Artist — SPEC §8.14 row 64.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-005"; // Stockpile: a card in hand

    fn artist(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "fvh-064",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER] },
            "p2": { "hand": [FILLER] },
        }))
    }

    /// Plays the Artist and answers its Discover with the first option: that option's def id.
    fn discover_first(s: &mut Scenario) -> String {
        s.play(ID, json!({}));
        let first = must(s.prompt_modes().into_iter().next(), "a Discover option");
        s.answer(pick::mode(&first));
        first
    }

    /// The cards of `def_id` in p1's hand.
    fn held(s: &Scenario, def_id: &str) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id == def_id).collect()
    }

    fn is_felinor(s: &Scenario, card: &CardInstance) -> bool {
        tags_of(s.state(), s.card(card)).contains(&Tag::Felinor)
    }

    mod base {
        use super::*;

        #[test]
        fn offers_three_non_token_units_costing_3_or_more() {
            let mut s = artist(false);
            s.play(ID, json!({}));
            let offered = nonempty(s.prompt_modes(), "the Discover's options");
            assert_eq!(offered.len(), 3);
            for id in &offered {
                let def = crate::card_def(id);
                assert_eq!(def.type_, CardType::Unit, "{id}");
                assert!(query_cost(&def) >= MIN_COST, "{id}");
                assert!(!def.token, "{id}");
                assert_ne!(def.id, ID);
            }
        }

        #[test]
        fn the_pick_lands_in_your_hand_as_a_felinor() {
            let mut s = artist(false);
            let picked = discover_first(&mut s);
            let added = nonempty(held(&s, &picked), "the discovered card in hand");
            assert_eq!(added.len(), 1);
            assert!(is_felinor(&s, &added[0]));
            assert!(!added[0].radiant);
            assert!(s.state().pending.is_none());
        }

        #[test]
        fn the_cards_already_in_hand_are_not_tagged() {
            let mut s = artist(false);
            discover_first(&mut s);
            let filler = nonempty(held(&s, FILLER), "the filler in hand");
            assert!(!is_felinor(&s, &filler[0]));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_pick_lands_radiant_as_a_felinor() {
            let mut s = artist(true);
            let picked = discover_first(&mut s);
            let added = nonempty(held(&s, &picked), "the discovered card in hand");
            assert_eq!(added.len(), 1);
            assert!(added[0].radiant);
            assert!(is_felinor(&s, &added[0]));
        }

        #[test]
        fn offers_units_costing_3_or_more() {
            let mut s = artist(true);
            s.play(ID, json!({}));
            for id in nonempty(s.prompt_modes(), "the Discover's options") {
                let def = crate::card_def(&id);
                assert_eq!(def.type_, CardType::Unit, "{id}");
                assert!(query_cost(&def) >= MIN_COST, "{id}");
            }
        }
    }
}
