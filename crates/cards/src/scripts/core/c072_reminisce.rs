//! #72 Reminisce (SPEC §8.3): "Discover a card from your GY; it costs 1 less; exile this", radiant
//! "It costs 0" — the radiant cell restates only the cost clause, so the Discover and the self-exile
//! are kept and only the discount changes (§8 Conventions).
//!
//! R50 is the whole of the Discover: the options come from the ACTUAL graveyard, not from a pool, so
//! spell tokens sitting there are eligible and are drawn without replacement. `discover_from_graveyard`
//! (effects/choose.rs) is exactly that — it shuffles the real graveyard pile and offers instance
//! selections — so this card names the step and nothing else. A pool-based Discover
//! (`discover_from_catalog`) would exclude tokens and would be wrong here.
//!
//! The two-step shape, per §10.6 and prompts.rs:
//!   cry   -> [discover_from_graveyard({ step: "chosen" }), exile self]
//!   resume.chosen -> [chosen card to hand, its cost changed]
//!
//! `apply_resumable` parks the tail of an effect list as a `WorkItem` the moment a prompt opens, and
//! `answer_prompt` runs the resume step FIRST and then drains the parked tail. So one array covers
//! both branches of "exile this":
//!   - graveyard non-empty: the prompt opens, `exile` is parked, and it runs after the pick has
//!     reached the hand — the order §8 writes (Discover, cost, exile);
//!   - graveyard empty: `discover_from_graveyard` returns without opening anything (§6.3: an effect
//!     with no options fizzles and the card still resolves), no prompt, so `exile` runs straight
//!     through in the same pass and `resume.chosen` never runs at all.
//!
//! The resume step therefore reads only `{ of: "chosen" }` and never `ctx.self`: a resumed step may
//! find its instance gone (§10.6), and a Spell mid-resolution is in no pile for `find_instance` to
//! find anyway.
//!
//! Cost: R65 starts the calculation from `costOverride`, else the printed cost, then adds `costMod`,
//! then the player's discounts. So "costs 1 less" is `costMod -1` (it stacks and it travels with the
//! card between zones, R78), while "it costs 0" is a `costOverride` of 0 — the same reading R77 gives
//! Craft a Card's "0-cost hand card". Reported as a ruling to settle: a `costOverride` of 0 still has
//! `costMod` added after it, so a radiant Reminisce on a card KY's Math Equation has bumped to +1
//! leaves it at 1 rather than 0.
//!
//! R4 is the engine's: `add_to_hand({ instance })` puts the card in the hand through the same pipeline a
//! draw uses, so a card picked into a full hand is burned to the graveyard. §6.3's Add to hand row is
//! "Creates OR MOVES the card", so moving the Discover's pick is that verb and not one of its own.

use jackioh_engine::effects::{add_to_hand, discover_from_graveyard, exile, set_cost_mod, set_cost_override};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-072";

/// The name the prompt's continuation is filed under, in `resume` and in `step` (§10.6).
const STEP_CHOSEN: &str = "chosen";

/// `price` is the whole of the radiant text: −1 on the base face, a flat 0 on it.
fn reminisce(price: impl Fn() -> Effect + Send + Sync + 'static) -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![
                discover_from_graveyard(json_as(json!({
                    "step": STEP_CHOSEN,
                    "prompt": "Discover a card from your graveyard",
                }))),
                // Parked while the prompt is open; runs straight through on an empty graveyard.
                exile(json_as(json!({ "target": { "of": "self" } }))),
            ]
        })),
        resume: IndexMap::from([(
            // The answered selection arrives in `ctx.targets`, which `{ of: "chosen" }` reads (§10.6).
            STEP_CHOSEN,
            hook(move |_ctx| vec![add_to_hand(json_as(json!({ "instance": { "of": "chosen" } }))), price()]),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        // R4: the price is the card's in the hand it reached, so a pick a full hand burns keeps its cost.
        base: reminisce(|| {
            set_cost_mod(json_as(json!({ "target": { "of": "chosen" }, "amount": -1, "inHandOnly": true })))
        }),
        radiant: reminisce(|| {
            set_cost_override(json_as(json!({ "target": { "of": "chosen" }, "cost": 0, "inHandOnly": true })))
        }),
    }
}

// #72 Reminisce — SPEC §8.3, BUILD M4-T4: "Discover from the GY including spell tokens (R50);
// chosen card −1 (radiant 0); exiled; empty GY → nothing".
//
// The card under test is a two-step script (§10.6): the Cry opens the Discover and parks "exile
// this" as a work item, and the answer runs the `chosen` resume step and then drains the parked
// tail. So every test here asserts BOTH halves — where the picked card went and what its cost is,
// and that Reminisce itself ended in exile — and one test covers each of the two paths through the
// Cry's single effect list: the prompt opened (tail parked), and the graveyard was empty (tail ran
// straight through, resume step never reached).
//
// core-013 Jlockeed Shredder-10 (printed cost 3) is the Discover subject in the cost tests on both
// faces, so the base "-1" (cost 2) and the radiant "costs 0" are the same card read two ways.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    /// `scenario(opts)` with the shipped cards registered first (the TS globalSetup's `registerAll()`).
    fn setup(opts: Value) -> Scenario {
        crate::register_all();
        scenario(opts)
    }

    use crate::js;

    /// Every `costChanged` event, as TS's object literal compares it.
    fn costs_changed(s: &Scenario) -> Vec<Value> {
        s.events().iter().map(js).filter(|event| event["type"] == "costChanged").collect()
    }

    /// The open prompt, as JSON (`null` when none).
    fn pending(s: &Scenario) -> Value {
        js(&s.state().pending)
    }

    fn hand_def_ids(s: &Scenario) -> Vec<String> {
        s.hand(P1).into_iter().map(|card| card.def_id).collect()
    }

    mod reminisce_base {
        use super::*;

        #[test]
        fn r50_discovers_from_the_actual_graveyard_so_a_spell_token_there_is_an_eligible_option() {
            let mut s = setup(json!({
                "seed": "core-072-r50",
                "p1": {
                    "hand": ["core-072"],
                    // core-051-1 is KY's Empty Notebook, a spell token (§7). A pool-based Discover would
                    // exclude it; R50 says this one does not, because it discovers "a card in your GY".
                    "graveyard": ["core-051-1", "core-005", "core-023"],
                    "library": ["core-035"],
                },
                "p2": { "hand": ["core-005"] },
            }));
            let graveyard = s.pile(P1, "graveyard");
            let token = graveyard[0].clone();
            assert_eq!(token.def_id, "core-051-1");

            s.play("core-072", json!({}));

            let pending = pending(&s);
            assert!(!pending.is_null());
            assert_eq!(pending["kind"], "discover");
            assert_eq!(pending["playerId"], "p1");
            // Drawn without replacement from a graveyard of exactly three: all three, each once.
            let options = pending["options"].as_array().cloned().unwrap_or_default();
            assert_eq!(options.len(), 3);
            let offered: Vec<Value> = options
                .iter()
                .map(|option| {
                    if option["selection"]["pick"] == "instance" {
                        option["selection"]["instanceId"].clone()
                    } else {
                        Value::Null
                    }
                })
                .collect();
            assert!(offered.contains(&json!(token.id)));
            let distinct: IndexSet<String> = offered.iter().map(|id| id.to_string()).collect();
            assert_eq!(distinct.len(), 3);

            s.answer(json!(token.id));

            s.expect_in_zone(&token, "hand");
            assert!(hand_def_ids(&s).contains(&"core-051-1".to_string()));
        }

        #[test]
        fn the_chosen_card_moves_gy_hand_and_costs_1_less_and_reminisce_is_exiled() {
            let mut s = setup(json!({
                "seed": "core-072-base-cost",
                "p1": { "hand": ["core-072"], "graveyard": ["core-013"], "library": ["core-035"] },
                "p2": { "hand": ["core-005"] },
            }));
            let spell = s.hand(P1)[0].clone();
            let picked = s.pile(P1, "graveyard")[0].clone();
            assert_eq!(spell.def_id, "core-072");
            assert_eq!(picked.def_id, "core-013");

            s.play("core-072", json!({})).answer(json!(picked.id));

            // GY → hand (§8.3 Engine cell).
            s.expect_in_zone(&picked, "hand");
            // R65: printed 3 plus a costMod of −1 is 2, and the mod travels with the card (R78).
            assert_eq!(s.card(&picked).cost_mod, -1);
            assert_eq!(s.card(&picked).cost_override, None);
            assert_eq!(
                costs_changed(&s).last().cloned(),
                Some(json!({ "type": "costChanged", "instanceId": picked.id, "cost": 2 }))
            );
            // "Exile this" — not the graveyard every other Spell goes to (§10.5 step 7).
            s.expect_in_zone(&spell, "exile");
        }

        #[test]
        fn s10_6_the_sequence_spanning_the_prompt_resumes_in_order_pick_to_hand_cost_changed_then_the_parked_exile() {
            let mut s = setup(json!({
                "seed": "core-072-resume-order",
                "p1": { "hand": ["core-072"], "graveyard": ["core-013"], "library": ["core-035"] },
                "p2": { "hand": ["core-005"] },
            }));
            let picked = s.pile(P1, "graveyard")[0].clone();

            s.play("core-072", json!({}));
            // The prompt paused the Cry's list: "exile this" has NOT happened yet. (A Spell mid-resolution
            // sits in the `resolving` zone, which `ZoneName` does not name, so this reads the exile pile.)
            assert!(s.state().pending.is_some());
            assert_eq!(s.pile(P1, "exile").len(), 0);

            s.answer(json!(picked.id));

            s.expect_events(json!([
                "cardPlayed",
                "promptOpened",
                "promptAnswered",
                "addedToHand",
                "costChanged",
                "exiled"
            ]));
        }

        #[test]
        fn an_empty_graveyard_opens_no_prompt_and_does_nothing_and_the_spell_is_still_exiled() {
            let mut s = setup(json!({
                "seed": "core-072-empty-gy",
                "p1": { "hand": ["core-072", "core-005"], "graveyard": [], "library": ["core-035"] },
                "p2": { "hand": ["core-005"] },
            }));
            let spell = s.hand(P1)[0].clone();

            s.play("core-072", json!({}));

            // §6.3: an effect with nothing to offer fizzles and the card still resolves.
            assert!(s.state().pending.is_none());
            // The resume step never ran, so nothing reached the hand; only core-005 is left there.
            assert_eq!(hand_def_ids(&s), ["core-005"]);
            // The parked tail was never parked: it ran straight through in the same pass.
            s.expect_in_zone(&spell, "exile").expect_events(json!(["cardPlayed", "exiled"]));
        }

        #[test]
        fn r4_a_card_the_discover_picks_into_a_full_hand_is_burned_to_the_graveyard() {
            // Eleven cards in hand, so playing Reminisce leaves exactly HAND_CAP (10) behind it.
            let mut s = setup(json!({
                "seed": "core-072-hand-cap",
                "p1": {
                    "hand": [
                        "core-072",
                        "core-005",
                        "core-023",
                        "core-031",
                        "core-035",
                        "core-036",
                        "core-013",
                        "core-019",
                        "core-043",
                        "core-054",
                        "core-066",
                    ],
                    "graveyard": ["core-002"],
                    "library": ["core-088"],
                },
                "p2": { "hand": ["core-005"] },
            }));
            let picked = s.pile(P1, "graveyard")[0].clone();
            assert_eq!(s.hand(P1).len(), 11);

            s.play("core-072", json!({})).answer(json!(picked.id));

            assert_eq!(s.hand(P1).len(), 10);
            s.expect_events(json!(["burned"]));
            // Burned means back to the graveyard it came from, not the hand.
            s.expect_in_zone(&picked, "graveyard");
        }
    }

    mod reminisce_radiant {
        use super::*;

        #[test]
        fn radiant_the_chosen_card_costs_0_not_one_less() {
            let mut s = setup(json!({
                "seed": "core-072-radiant-cost",
                "p1": {
                    "hand": [{ "def": "core-072", "radiant": true }],
                    "graveyard": ["core-013"],
                    "library": ["core-035"],
                },
                "p2": { "hand": ["core-005"] },
            }));
            let spell = s.hand(P1)[0].clone();
            let picked = s.pile(P1, "graveyard")[0].clone();
            assert!(spell.radiant);

            s.play("core-072", json!({})).answer(json!(picked.id));

            s.expect_in_zone(&picked, "hand");
            // R65/R77: "it costs 0" is a costOverride, the same reading Craft a Card's 0-cost card gets.
            assert_eq!(s.card(&picked).cost_override, Some(0));
            assert_eq!(s.card(&picked).cost_mod, 0);
            assert_eq!(
                costs_changed(&s).last().cloned(),
                Some(json!({ "type": "costChanged", "instanceId": picked.id, "cost": 0 }))
            );
            s.expect_in_zone(&spell, "exile");
        }

        #[test]
        fn radiant_keeps_the_discover_r50s_token_eligibility_and_the_self_exile_s8_conventions() {
            let mut s = setup(json!({
                "seed": "core-072-radiant-r50",
                "p1": {
                    "hand": [{ "def": "core-072", "radiant": true }],
                    "graveyard": ["core-051-1"],
                    "library": ["core-035"],
                },
                "p2": { "hand": ["core-005"] },
            }));
            let spell = s.hand(P1)[0].clone();
            let token = s.pile(P1, "graveyard")[0].clone();

            s.play("core-072", json!({}));
            assert_eq!(pending(&s)["kind"], "discover");
            assert_eq!(pending(&s)["options"].as_array().map(Vec::len), Some(1));

            s.answer(json!(token.id));

            s.expect_in_zone(&token, "hand");
            assert_eq!(s.card(&token).cost_override, Some(0));
            s.expect_in_zone(&spell, "exile");
        }

        #[test]
        fn radiant_on_an_empty_graveyard_also_does_nothing_but_still_exiles_the_spell() {
            let mut s = setup(json!({
                "seed": "core-072-radiant-empty",
                "p1": {
                    "hand": [{ "def": "core-072", "radiant": true }, "core-005"],
                    "graveyard": [],
                    "library": ["core-035"]
                },
                "p2": { "hand": ["core-005"] },
            }));
            let spell = s.hand(P1)[0].clone();

            s.play("core-072", json!({}));

            assert!(s.state().pending.is_none());
            s.expect_in_zone(&spell, "exile");
            assert_eq!(hand_def_ids(&s), ["core-005"]);
        }
    }
}
