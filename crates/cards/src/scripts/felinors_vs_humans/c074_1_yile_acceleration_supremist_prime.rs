//! FvH #74.1 Yile, Acceleration Supremist Prime (SPEC §8.14 row 74.1): (2) Unit, Human, Prime, Acclaimed,
//! Token (printed Mythic), 9/5 → 18/10; the card FvH #74's Death shuffles in. Built by S08 (#5258).
//!   Base:    "Cry: Overload ({mana}). Discover 2 Human Units and 2 Felinor Units. Fuse the four and add
//!            the result to your hand. It costs ({setCost})." — mana 4, setCost 2
//!   Radiant: the same with "Discover 2 Radiant Human Units and 2 Radiant Felinor Units".
//!
//! The list's "Overload 4" line is folded into the Cry (FV2, R4353021, R4751000), paid first, whatever
//! the Discovers find: `next_turn_mana` of −`mana`. Then four Discovers in turn (Core #99 Craft a Card's
//! chain, the picks carried in the step data): two of non-token Human Units, then two of non-token
//! Felinor Units, of the sets that ship (R5235000); a Unit with both tags may be offered in either, and
//! repeats across the Discovers are allowed (R60). The last answer fuses the four into one hand card
//! (`fuse_cards` to its controller's hand, R77), the ingredients on their Radiant faces on the Radiant
//! face (R469), the product itself not Radiant; then its price is set to `setCost` (`set_cost_override`
//! on the card the fusion put into the hand, read with `added_to_hand_so_far`, R5239000). A full hand
//! burns it and no price is set (R4).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-074-1";

/// The resume step each Discover's answer re-enters (§10.6).
const PICKED: &str = "picked";

/// The data key the picks so far travel in.
const PICKS_KEY: &str = "picks";

/// The four Discovers in order, each its tag and its prompt.
const DISCOVERS: [(Tag, &str); 4] = [
    (Tag::Human, "Discover a Human Unit"),
    (Tag::Human, "Discover a Human Unit"),
    (Tag::Felinor, "Discover a Felinor Unit"),
    (Tag::Felinor, "Discover a Felinor Unit"),
];

/// The Discover after `picks`, or none once all four are made.
fn next_discover(picks: &[String]) -> Option<Effect> {
    let (tag, prompt) = DISCOVERS.get(picks.len())?;
    Some(discover_from_catalog(json_as(json!({
        "step": PICKED,
        "query": { "type": "Unit", "tags": [tag] },
        "prompt": prompt,
        "data": { PICKS_KEY: picks },
    }))))
}

/// The picks a paused Cry has made so far, read back out of the prompt's data.
fn picks_so_far(ctx: &EffectContext<'_>) -> Vec<String> {
    match ctx.data.get(PICKS_KEY).and_then(Value::as_array) {
        Some(stored) => stored.iter().filter_map(|pick| pick.as_str().map(str::to_string)).collect(),
        None => Vec::new(),
    }
}

/// "Fuse the four and add the result to your hand. It costs ({setCost})."
fn fuse_to_hand(picks: &[String], radiant: bool, set_cost: i32) -> Vec<Effect> {
    vec![
        fuse_cards(json_as(json!({
            "defIds": picks,
            "toHand": "self",
            "radiantIngredients": radiant,
        }))),
        for_each_card(ForEachCardArgs {
            cards: Arc::new(|ctx: &mut EffectContext<'_>| added_to_hand_so_far(ctx)),
            each: Arc::new(move |instance_id: &str| {
                set_cost_override(json_as(json!({
                    "target": { "of": "instance", "instanceId": instance_id },
                    "cost": set_cost,
                })))
            }),
        }),
    ]
}

fn prime(radiant: bool) -> Script {
    Script {
        cry: Some(hook(|ctx| {
            let mut effects = vec![next_turn_mana(json_as(json!({ "amount": -param(ctx, "mana") })))];
            effects.extend(next_discover(&[]));
            effects
        })),
        resume: IndexMap::from([(
            PICKED,
            hook(move |ctx| {
                // A Discover is answered with one of its options (§10.6); naming none adds nothing.
                let Some(picked) = chosen_options(ctx).into_iter().next() else {
                    return vec![];
                };
                let mut picks = picks_so_far(ctx);
                picks.push(picked);
                if let Some(discover) = next_discover(&picks) {
                    return vec![discover];
                }
                fuse_to_hand(&picks, radiant, param(ctx, "setCost"))
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: prime(false),
        radiant: prime(true),
    }
}

// FvH #74.1 Yile, Acceleration Supremist Prime — SPEC §8.14 row 74.1.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-005"; // Stockpile: a card in hand

    fn prime_game(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "fvh-074-1",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER] },
            "p2": { "hand": [FILLER] },
        }))
    }

    /// Answers the open Discover with its first option, checking each offered card holds `tag`.
    fn answer_first(s: &mut Scenario, tag: Tag) -> String {
        let offered = nonempty(s.prompt_modes(), "the Discover's options");
        for id in &offered {
            let def = crate::card_def(id);
            assert_eq!(def.type_, CardType::Unit, "{id}");
            assert!(def.tags.contains(&tag), "{id} is offered as a {tag:?}");
            assert!(!def.token, "{id}");
        }
        let first = offered[0].clone();
        s.answer(pick::mode(&first));
        first
    }

    /// Plays the Prime and answers the four Discovers: the picks, and the hand card made.
    fn crafted(radiant: bool) -> (Scenario, Vec<String>, CardInstance) {
        let mut s = prime_game(radiant);
        s.play(ID, json!({}));
        let held: Vec<String> = s.hand(P1).into_iter().map(|card| card.id).collect();
        let mut picks = Vec::new();
        for (tag, _) in DISCOVERS {
            picks.push(answer_first(&mut s, tag));
        }
        assert!(s.state().pending.is_none(), "four Discovers, no more");
        let made: Vec<CardInstance> =
            s.hand(P1).into_iter().filter(|card| !held.contains(&card.id)).collect();
        assert_eq!(made.len(), 1, "one hand card");
        let card = must(made.into_iter().next(), "the fused card");
        (s, picks, card)
    }

    mod base {
        use super::*;

        #[test]
        fn its_cry_overloads_4_before_the_discovers() {
            let mut s = prime_game(false);
            s.play(ID, json!({}));
            assert!(s.state().pending.is_some(), "the first Discover is open");
            assert_eq!(s.state().players.p1.mana.next_turn_mod, -4);
        }

        #[test]
        fn discovers_two_humans_then_two_felinors_and_fuses_them_into_a_hand_card_costing_2() {
            let (s, picks, card) = crafted(false);
            let parts = must(fused_id_parts(Some(s.state()), &card.def_id), "a fused card");
            assert_eq!(parts.len(), 4);
            for pick in &picks {
                assert!(parts.contains(pick), "{pick} is fused in");
            }
            assert!(!card.radiant, "the product is not Radiant");
            assert_eq!(s.card(&card).cost_override, Some(2));
        }

        #[test]
        fn the_ingredients_go_in_on_their_base_faces() {
            let (s, _picks, card) = crafted(false);
            let specs = nonempty(
                must(fused_id_specs(Some(s.state()), &card.def_id), "a fused card"),
                "the ingredients",
            );
            for spec in &specs {
                assert_eq!(spec.radiant, None, "{}", spec.def_id);
            }
        }

        #[test]
        fn the_numbers_tune() {
            let mut s = prime_game(false);
            set_param(s.card_mut(ID), "mana", 3);
            set_param(s.card_mut(ID), "setCost", 1);
            s.play(ID, json!({}));
            assert_eq!(s.state().players.p1.mana.next_turn_mod, -3);
            let held: Vec<String> = s.hand(P1).into_iter().map(|card| card.id).collect();
            for (tag, _) in DISCOVERS {
                answer_first(&mut s, tag);
            }
            let made = nonempty(
                s.hand(P1).into_iter().filter(|card| !held.contains(&card.id)).collect(),
                "the fused card",
            );
            assert_eq!(made[0].cost_override, Some(1));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_ingredients_go_in_radiant_and_the_product_is_not() {
            let (s, _picks, card) = crafted(true);
            let specs = nonempty(
                must(fused_id_specs(Some(s.state()), &card.def_id), "a fused card"),
                "the ingredients",
            );
            assert_eq!(specs.len(), 4);
            for spec in &specs {
                assert_eq!(spec.radiant, Some(true), "{}", spec.def_id);
            }
            assert!(!card.radiant);
            assert_eq!(s.card(&card).cost_override, Some(2));
        }

        #[test]
        fn its_cry_overloads_4() {
            let mut s = prime_game(true);
            s.play(ID, json!({}));
            assert_eq!(s.state().players.p1.mana.next_turn_mod, -4);
        }
    }
}
