//! #7 Jewelosco Scarab (SPEC §8.1): a 1-cost 1/1 → 2/2 Unit, "Cry: Discover a 2-cost card", radiant
//! "Cry: Discover a 3-cost card; it costs 1 less". The radiant cell restates the Discover with a new
//! number and ADDS a clause (§8 Conventions: "a cell that changes only a number changes only that
//! number", and "Also"/"Then" add effects), so the two faces differ in the cost bracket and in the
//! permanent discount the radiant face puts on what it finds.
//!
//! §8.1's Engine cell is `catalog.query({cost, notTags:["Token"], excludeIndex:7})`:
//!   - the cost bracket is read per R65, which `catalog::query_cost` already owns: outside play an
//!     embiggen card counts at its base price and an X-cost card as 0, so this file passes a plain
//!     number and nothing here reads a cost;
//!   - `notTags: ["Token"]` is stated because §8 states it, even though §5.1 already keeps tokens
//!     out of every pool that does not name them (`catalog::asks_for_tokens`);
//!   - the exclusion of #7 itself is NOT passed here on purpose: `discover_from_catalog` adds the
//!     running card's own id (`excludeDefId`, R387) to every query it builds (§5.1, "a random pool
//!     never offers the card that generated it"), so repeating it would be duplicated rules, not
//!     safety. The pool reaches every set (R380).
//!
//! The prompt and the continuation (§10.6, R81): the Cry opens a `discover` prompt whose answer is a
//! `mode` selection carrying the chosen DEF ID, and the answer re-enters `resume.chosen` with that
//! selection in `ctx.targets`, which is what `chosen_options` reads. An empty pool never opens a
//! prompt (the effect fizzles, the unit still enters, §8 Conventions), and then no step runs at all.

use jackioh_engine::effects::{add_to_hand, chosen_options, discover_from_catalog};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-007";

/// The cost bracket each face Discovers from (§8.1).
const BASE_COST: i32 = 2;
const RADIANT_COST: i32 = 3;
/// "It costs 1 less": a permanent −1 `costMod` on the card the radiant face found (§8.1, R65).
const RADIANT_DISCOUNT: i32 = 1;

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
                // §8.1 asks for a permanent `costMod`, not a `costOverride`: R65 starts from the override
                // in place of the printed cost, which would also erase any other discount the card
                // carries.
                vec![add_to_hand(json_as(json!({ "defId": def_id, "costMod": -RADIANT_DISCOUNT })))]
            }),
        )]),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// #7 Jewelosco Scarab — SPEC §8.1 row 7, BUILD M4-T4 must-pass: "Discover offers 3 distinct 2-cost
// non-token cards, never #7; radiant 3-cost pick costs 2".
//
// This is the one prompting card in #1-20, so every test here answers the prompt and asserts what
// follows: the pick lands in hand, the prompt closes, and the game carries on (§10.6's re-entrant
// reducer, R81). The offered options are read out of `view_for` rather than out of `state.pending`,
// because §10.8 is what a player actually sees and only the chooser sees the options.
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
                // R65: an embiggen card's cost outside play is its base price, so "a 2-cost card"
                // includes one.
                assert_eq!(crate::query::query_cost(&def), 2);
                assert!(!def.token);
                assert!(!def.tags.contains(&Tag::Token));
                // §5.1: a random pool never offers the card that generated it. `discover_from_catalog`
                // adds that exclusion itself from the running instance, so the card file does not
                // repeat it.
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
                // #26 Glowy Jelly Bean makes a chosen hand card Radiant, which is the only way to hold a
                // Radiant card in hand: `SideSetup.hand` takes def ids only (see the report's harness gap).
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
    }
}
