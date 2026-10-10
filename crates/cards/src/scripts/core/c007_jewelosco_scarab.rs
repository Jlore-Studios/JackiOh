//! #7 Jewelosco Scarab (SPEC §8.1): a 1-cost 1/1 → 2/2 Unit, "Cry: Discover a 2-cost card", radiant
//! "Cry: Discover a 3-cost card; it costs 1 less". The radiant cell changes the number and ADDS a
//! clause (§8 Conventions), so the faces differ in the cost bracket and in the permanent discount.
//!
//! §8.1's Engine cell is `catalog.query({cost, notTags:["Token"], excludeIndex:7})`:
//!   - the cost bracket is read per R65 by `catalog::query_cost`, so this file passes a plain number;
//!   - `notTags: ["Token"]` is stated because §8 states it, though §5.1 already keeps tokens out;
//!   - #7's own exclusion is not passed: `discover_from_catalog` adds the running card's id (R387,
//!     §5.1). The pool reaches every set (R380).
//!
//! The Cry opens a `discover` prompt (§10.6, R81); the answer re-enters `resume.chosen` with the
//! pick in `ctx.targets`. An empty pool opens no prompt: the effect fizzles (§8 Conventions).

use jackioh_engine::effects::{add_to_hand, chosen_options, discover_from_catalog};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-007";

/// The cost bracket each face Discovers from (§8.1).
const BASE_COST: i32 = 2;
const RADIANT_COST: i32 = 3;
// "It costs (1) less": a permanent `costMod` on the card the radiant face found (§8.1, R65), by the
// declared number `discount` (R386).

fn discover(cost: i32) -> Vec<Effect> {
    vec![discover_from_catalog(json_as(json!({
        "step": "chosen",
        "query": { "cost": cost, "notTags": ["Token"] },
        "count": 3
    })))]
}

/// §10.6: a Discover's answer is the def id it picked, or nothing when the prompt never opened.
fn picked(ctx: &mut EffectContext<'_>) -> Option<String> {
    chosen_options(ctx).into_iter().next()
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|_ctx| discover(BASE_COST))),
        resume: IndexMap::from([(
            "chosen",
            hook(|ctx| match picked(ctx) {
                None => vec![],
                Some(def_id) => vec![add_to_hand(json_as(json!({ "defId": def_id })))],
            }),
        )]),
        ..Script::default()
    };
    let radiant = Script {
        cry: Some(hook(|_ctx| discover(RADIANT_COST))),
        resume: IndexMap::from([(
            "chosen",
            hook(|ctx| {
                let Some(def_id) = picked(ctx) else {
                    return vec![];
                };
                // A permanent `costMod`, not a `costOverride` (§8.1): R65 starts from the override in
                // place of the printed cost, which would erase any other discount the card carries.
                vec![add_to_hand(json_as(json!({ "defId": def_id, "costMod": -param(&*ctx, "discount") })))]
            }),
        )]),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// #7 Jewelosco Scarab — SPEC §8.1 row 7, BUILD M4-T4 must-pass: "Discover offers 3 distinct 2-cost
// non-token cards, never #7; radiant 3-cost pick costs 2".
//
// The one prompting card in #1-20: each test answers the prompt (§10.6, R81). Options are read out of
// `view_for`, not `state.pending`, because only the chooser sees them (§10.8).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    /// The open Discover prompt as its owner sees it (§10.8).
    fn discover(s: &Scenario) -> PendingPromptView {
        match s.view(PlayerId::P1).pending {
            None => panic!("no prompt is open for p1"),
            Some(PendingView::Elsewhere(elsewhere)) => {
                panic!("the open prompt belongs to {}", elsewhere.pending_for)
            }
            Some(PendingView::ForYou(prompt)) => prompt,
        }
    }

    /// §10.8: a viewer's own hand is full cards, the opponent's a count — narrow to the cards.
    fn hand_view(s: &Scenario) -> Vec<CardView> {
        match s.view(PlayerId::P1).you.hand {
            HandView::Cards(cards) => cards,
            HandView::Count { .. } => panic!("p1's own hand should be full cards (§10.8)"),
        }
    }

    /// The def ids a Discover put on offer, in the order they were offered.
    fn offered(s: &Scenario) -> Vec<String> {
        discover(s)
            .options
            .into_iter()
            .map(|option| match option.def_id {
                Some(def_id) => def_id,
                None => panic!("option {} carries no def id", option.key),
            })
            .collect()
    }

    mod n7_jewelosco_scarab_s8_1_row_7 {
        use super::*;

        #[test]
        fn base_cry_offers_3_distinct_2_cost_non_token_cards_of_every_set_r380_and_never_n7_itself_s5_1() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-007-offer",
                "p1": { "hand": ["core-007"], "mana": 4, "library": ["core-020"] },
                "p2": { "field": ["core-020"] }
            }));

            s.play("core-007", json!({}));

            let prompt = discover(&s);
            assert_eq!(prompt.kind, PromptKind::Discover);
            assert_eq!(prompt.min, 1);
            assert_eq!(prompt.max, 1);

            let ids = offered(&s);
            assert_eq!(ids.len(), 3);
            assert_eq!(ids.iter().collect::<IndexSet<_>>().len(), 3);
            for id in &ids {
                let def = crate::card_def(id);
                // R65: an embiggen card's cost outside play is its base price.
                assert_eq!(crate::query::query_cost(&def), 2);
                assert!(!def.token);
                assert!(!def.tags.contains(&Tag::Token));
                // §5.1: a random pool never offers the card that generated it; the engine adds that.
                assert_ne!(def.index, "7");
                assert_ne!(def.id, "core-007");
            }

            // The unit is on the field while its Cry is still resolving (§10.5 step 4 before step 5).
            s.expect_stats("core-007", json!({ "attack": 1, "health": 1, "maxHealth": 1 }));
        }

        #[test]
        fn r81_the_answer_puts_the_chosen_card_in_hand_and_the_sequence_resumes() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-007-answer",
                "p1": { "hand": ["core-007", "core-011"], "mana": 4, "library": ["core-020"] },
                "p2": { "field": ["core-020"] }
            }));

            s.play("core-007", json!({}));
            let chosen = offered(&s)[1].clone();
            s.answer(json!(chosen));

            // The prompt is closed and the picked definition is a new card in the chooser's hand.
            assert!(s.view(PlayerId::P1).pending.is_none());
            let added: Vec<CardInstance> =
                s.hand(PlayerId::P1).into_iter().filter(|card| card.def_id == chosen).collect();
            assert_eq!(added.len(), 1);
            s.expect_in_zone(&added[0], "hand");
            s.expect_events(json!(["cardPlayed", "promptOpened", "promptAnswered", "addedToHand"]));

            // Base takes no discount: the card is in hand at its printed 2 (R65).
            let picked = &added[0];
            assert_eq!(
                hand_view(&s).iter().find(|card| card.instance_id == picked.id).map(|card| card.cost),
                Some(2)
            );

            // The turn carries on after the answer: the rest of the hand is still playable.
            s.play("core-011", json!({}));
            s.expect_stats("core-011", json!({ "attack": 3, "health": 3 }));
        }

        #[test]
        fn an_empty_pool_would_fizzle_and_the_answer_only_ever_adds_one_card() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-007-single",
                "p1": { "hand": ["core-007"], "mana": 4, "library": ["core-020"] }
            }));

            s.play("core-007", json!({}));
            let ids = offered(&s);
            s.answer(json!(ids[0]));

            // Exactly one card entered the hand: the one that was picked, not the three on offer.
            assert_eq!(s.hand(PlayerId::P1).len(), 1);
            assert_eq!(s.hand(PlayerId::P1)[0].def_id, ids[0]);
        }

        #[test]
        fn radiant_cry_discovers_a_3_cost_card_and_it_costs_2_s8_1_r65() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-007-radiant",
                // #26 Glowy Jelly Bean makes a chosen hand card Radiant, the only way to hold one in hand:
                // `SideSetup.hand` takes def ids only.
                "p1": { "hand": ["core-026", "core-007"], "mana": 8, "library": ["core-020"] },
                "p2": { "field": ["core-020"] }
            }));
            let scarab = s.card("core-007").clone();

            s.play("core-026", json!({ "targets": [{ "pick": "instance", "instanceId": scarab.id }] }));
            s.play(&scarab, json!({}));

            // The radiant face is a 2/2 (printed, §10.4 layer 1).
            s.expect_stats(&scarab, json!({ "attack": 2, "health": 2, "maxHealth": 2 }));

            let ids = offered(&s);
            assert_eq!(ids.len(), 3);
            for id in &ids {
                let def = crate::card_def(id);
                assert_eq!(def.cost, CardCost::Fixed(3));
                assert!(!def.token);
                assert_ne!(def.index, "7");
            }

            let chosen = ids[0].clone();
            s.answer(json!(chosen));

            let added = s.hand(PlayerId::P1).into_iter().find(|card| card.def_id == chosen);
            assert!(added.is_some());
            let added_id = added.map(|card| card.id);
            let in_hand = hand_view(&s)
                .into_iter()
                .find(|card| Some(&card.instance_id) == added_id.as_ref());
            // "It costs 1 less": a permanent −1 costMod on the chosen card, so 3 reads as 2 (R65, R78).
            assert_eq!(in_hand.map(|card| card.cost), Some(2));
        }

        #[test]
        fn r386_an_upgrade_makes_the_radiant_discount_2_and_a_degrade_finds_it_at_its_floor_of_1() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-007-radiant",
                "p1": { "hand": [{ "def": "core-007", "radiant": true }], "library": ["core-020"] },
                "p2": { "field": ["core-020"] }
            }));
            assert!(!crate::can_degrade_number(&s, "core-007", "discount"));
            assert_eq!(crate::upgrade_number(&mut s, "core-007", "discount"), 2);
            s.play("core-007", json!({}));
            let chosen = offered(&s)[0].clone();
            s.answer(json!(chosen));
            let added_id = s.hand(PlayerId::P1).into_iter().find(|card| card.def_id == chosen).map(|card| card.id);
            let in_hand = hand_view(&s).into_iter().find(|card| Some(&card.instance_id) == added_id.as_ref());
            assert_eq!(in_hand.map(|card| card.cost), Some(1));
        }
    }
}
