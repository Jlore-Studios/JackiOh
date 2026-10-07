//! #37 Gravedigger (SPEC §8.2): 4/5 → 8/10, "Start of turn: add a random card from your GY to your
//! hand", radiant "Discover one from your GY; it costs 1 less". The radiant cell restates the whole
//! clause — the random add becomes a Discover — and adds the discount, so the radiant face is a
//! prompt where the base face is a seeded pick (§8 Conventions).
//!
//! Base is one verb: `addRandomFromGraveyard` already draws from `ctx.rng` (CLAUDE.md rule 4) and
//! returns without doing anything when the graveyard is empty (§8.2 Engine: "Empty GY → nothing").
//! `startTurn` (engine/src/turn.ts) runs the start-of-turn hooks BEFORE `draw(sink, player, 1)`, so
//! "before the draw" is the turn loop's order and not a clause here.
//!
//! Radiant is a genuine RESOLUTION prompt, not a play-time choice: the card is already on the field
//! and the Discover happens at the start of a later turn, so R81's "travels in the play action" does
//! not apply and it opens a `PendingChoice` through `discoverFromGraveyard` (R50: the options come
//! from the actual graveyard, so spell tokens there are eligible). An empty graveyard opens no
//! prompt at all, which is the same "nothing" the base face does.
//!
//! The answer re-enters this script through the continuation `prompts.ts` documents: `resumeSelf`
//! records `hook: "resume"` (`RESUME_HOOK`) and `step: "picked"`, and `runResume` looks up
//! `script.resume.picked`. So the resume step is a TABLE on the `Script` and NOT the `startOfTurn`
//! hook that opened the prompt — which is exactly what makes a start-of-turn Discover land in the
//! right place even though a `Resume` cannot say which hook asked for it.
//!
//! The pick arrives in `ctx.targets` as `{ pick: "instance", instanceId }` (that is what
//! `discoverFromGraveyard` offers, R50), so `{ of: "chosen" }` names it. Two verbs finish the job:
//!   - `bounce` moves the named card to its controller's hand — its owner's, out of a graveyard
//!     (R747). §6.3 Bounce is "return to controller's hand" with no zone restriction, the hand cap
//!     burns it when the hand is full (§2.4, R4), and #72
//!     Reminisce's Engine cell prints this same move as "chosen card moves GY → hand"; #23
//!     Reoccurring Dream already uses it to come back out of the graveyard. There is no
//!     `moveToHand` verb and there should not be: §6.3's Add to hand row is one verb that "Creates
//!     OR MOVES the card", so `addToHand({ instance })` moves the chosen instance while
//!     `addToHand({ defId })` creates a fresh one. #72 Reminisce uses the `instance` form.
//!   - `setCostMod({ amount: -1 })` is R65's "costs 1 less": it adds to the instance's `costMod`,
//!     which R78 keeps in every zone, so the discount survives the card's next trip to the
//!     graveyard. `cost.ts` names this card as the reason that verb exists.
//!
//! The order is bounce-then-discount so the discount is the price of a card that reached the hand:
//! `inHandOnly` skips it for a pick a full hand burned straight back to the graveyard (§2.4, R4),
//! which would otherwise keep a discount for a return it never made (R78). The `costChanged` event
//! then reports the card as it now stands, in hand.

use jackioh_engine::effects::{add_random_from_graveyard, bounce, discover_from_graveyard, set_cost_mod};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-037";

/// The step name the Discover's continuation carries (`prompts.ts`: `script.resume[step]`).
const PICKED: &str = "picked";

/// R65: "costs 1 less" is a −1 `costMod` on the chosen instance, permanent and zone-proof (R78).
const DISCOUNT: i32 = -1;

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            start_of_turn: Some(hook(|_ctx| {
                vec![add_random_from_graveyard(json_as(json!({ "player": "self" })))]
            })),
            ..Script::default()
        },
        radiant: Script {
            start_of_turn: Some(hook(|_ctx| {
                vec![discover_from_graveyard(json_as(json!({
                    "step": PICKED,
                    "prompt": "Discover a card from your graveyard",
                })))]
            })),
            resume: IndexMap::from([(
                PICKED,
                hook(|_ctx| {
                    vec![
                        bounce(json_as(json!({ "target": { "of": "chosen" } }))),
                        // R4: "it costs 1 less" is its price in the hand, so a pick a full hand burns keeps its cost.
                        set_cost_mod(json_as(json!({
                            "target": { "of": "chosen" },
                            "amount": DISCOUNT,
                            "inHandOnly": true,
                        }))),
                    ]
                }),
            )]),
            ..Script::default()
        },
    }
}

// #37 Gravedigger (SPEC §8.2, BUILD M4-T4): "Random GY card to hand at start of turn before the
// draw; empty GY nothing; radiant Discover at −1."
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    const GRAVEDIGGER: &str = "core-037";
    /// Three distinct graveyard cards, so a random pick and a Discover are both readable by defId.
    const STOCKPILE: &str = "core-005"; // cost 1
    const HIT_JOB: &str = "core-016"; // cost 2
    const MANA_WELL: &str = "core-006"; // cost 3
    /// What the turn's draw puts in hand, so "before the draw" has something to be before.
    const DRAWN: &str = "core-011";

    const SEED: &str = "gravedigger-37";
    /// The card that seed picks out of the three-card graveyard; pinned per CLAUDE.md rule 4.
    const SEEDED_PICK: &str = MANA_WELL;

    /// The harness's import-time `registerAll()`: the engine's testkit cannot name the cards crate.
    fn scn(opts: Value) -> Scenario {
        crate::register_all();
        scenario(opts)
    }

    fn def_ids_in_hand(s: &Scenario) -> Vec<String> {
        s.hand("p1").into_iter().map(|card| card.def_id).collect()
    }

    fn sorted(mut ids: Vec<String>) -> Vec<String> {
        ids.sort();
        ids
    }

    fn strings(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    mod gravedigger {
        use super::*;

        #[test]
        fn is_sec8_2s_37_a_2_cost_unit_4_5_to_8_10() {
            crate::register_all();
            let def = crate::card_def(ID);
            assert_eq!(def.index, "37");
            assert_eq!(def.type_, CardType::Unit);
            assert_eq!(def.cost, CardCost::Fixed(2));
            assert_eq!((def.base.attack, def.base.health), (Some(4), Some(5)));
            assert_eq!((def.radiant.attack, def.radiant.health), (Some(8), Some(10)));
        }

        #[test]
        fn base_hangs_its_clause_on_start_of_turn_and_radiant_answers_through_a_resume_step_sec10_6() {
            let scripts = script();
            assert!(scripts.base.start_of_turn.is_some());
            assert!(scripts.base.resume.is_empty());
            assert!(scripts.radiant.start_of_turn.is_some());
            // `prompts.ts`: `resumeSelf` files the continuation under `hook: "resume"`, `step: "picked"`.
            assert!(scripts.radiant.resume.contains_key("picked"));
        }

        #[test]
        fn base_adds_a_graveyard_card_to_your_hand_at_the_start_of_your_turn_before_the_draw() {
            let mut s = scn(json!({
                "seed": SEED,
                "p1": { "field": [GRAVEDIGGER], "graveyard": [STOCKPILE], "library": [DRAWN] },
                "p2": { "hand": [STOCKPILE], "library": [STOCKPILE] },
            }));

            s.start_turn();

            // One card in the graveyard, so the seeded pick is forced: Stockpile moves GY → hand.
            assert!(s.pile("p1", "graveyard").is_empty());
            assert_eq!(sorted(def_ids_in_hand(&s)), sorted(strings(&[STOCKPILE, DRAWN])));
            // §2.2/R62: the start-of-turn hooks run before the draw, so the add is logged first.
            s.expect_events(json!(["addedToHand", "drawn"]));
        }

        #[test]
        fn base_picks_with_the_seeded_rng_so_a_fixed_seed_gives_a_fixed_card() {
            let build = || -> Scenario {
                let mut s = scn(json!({
                    "seed": SEED,
                    "p1": {
                        "field": [GRAVEDIGGER],
                        "graveyard": [STOCKPILE, HIT_JOB, MANA_WELL],
                        "library": [DRAWN],
                    },
                    "p2": { "hand": [STOCKPILE], "library": [STOCKPILE] },
                }));
                s.start_turn();
                s
            };

            let first = build();
            let second = build();

            let moved = |s: &Scenario| -> Vec<String> {
                def_ids_in_hand(s).into_iter().filter(|id| id != DRAWN).collect()
            };

            // The seed above picks Mana Well out of the three; the point of the assertion is that the
            // outcome is pinned, not which card it is.
            assert_eq!(moved(&first), strings(&[SEEDED_PICK]));
            assert_eq!(moved(&second), strings(&[SEEDED_PICK]));
            // It MOVED: one card left the graveyard rather than being copied out of it (R78, §6.3).
            assert_eq!(first.pile("p1", "graveyard").len(), 2);
            assert!(
                !first
                    .pile("p1", "graveyard")
                    .iter()
                    .any(|card| card.def_id == SEEDED_PICK)
            );
        }

        #[test]
        fn base_does_nothing_with_an_empty_graveyard_sec8_2_engine() {
            let mut s = scn(json!({
                "seed": SEED,
                "p1": { "field": [GRAVEDIGGER], "library": [DRAWN] },
                "p2": { "hand": [STOCKPILE], "library": [STOCKPILE] },
            }));

            s.start_turn();

            // Only the turn's draw reached the hand.
            assert_eq!(def_ids_in_hand(&s), strings(&[DRAWN]));
            assert!(s.pile("p1", "graveyard").is_empty());
        }

        #[test]
        fn radiant_discovers_from_the_graveyard_the_pick_lands_in_hand_at_1_less_r50_r65_r78() {
            let mut s = scn(json!({
                "seed": SEED,
                "p1": {
                    "field": [{ "def": GRAVEDIGGER, "radiant": true }],
                    "graveyard": [STOCKPILE, HIT_JOB, MANA_WELL],
                    "library": [DRAWN],
                },
                "p2": { "hand": [STOCKPILE], "library": [STOCKPILE] },
            }));

            s.start_turn();

            // A genuine resolution prompt, not a play-time choice (R81): it opened a PendingChoice.
            let pending = s
                .state()
                .pending
                .clone()
                .expect("the radiant face should have opened a Discover");
            assert_eq!(pending.kind, PromptKind::Discover);
            assert_eq!(pending.player_id, PlayerId::P1);
            // R50: the options are the real graveyard instances, not a catalog pool.
            let offered: Vec<Selection> = pending.options.iter().map(|option| option.selection.clone()).collect();
            assert_eq!(offered.len(), 3);
            for selection in &offered {
                assert!(matches!(selection, Selection::Instance { .. }));
            }

            s.answer(json!(MANA_WELL));

            let picked = s.card(MANA_WELL).clone();
            s.expect_in_zone(&picked, "hand");
            // R65: "costs 1 less" is a −1 costMod, and R78 keeps it in every zone.
            assert_eq!(picked.cost_mod, -1);
            assert_eq!(effective_cost(s.state(), s.card(&picked.id), Default::default()), 2);
            // The other two are still in the graveyard: Discover moves the pick alone.
            let left: Vec<String> = s.pile("p1", "graveyard").into_iter().map(|card| card.def_id).collect();
            assert_eq!(sorted(left), sorted(strings(&[HIT_JOB, STOCKPILE])));
        }

        /// KNOWN FAILING, and deliberately so — the assertion states R62's order, not the engine's.
        ///
        /// R62: "Refresh → start-of-turn delayed effects → start-of-turn triggers → draw". So a
        /// start-of-turn trigger that opens a prompt must hold the draw until the prompt is answered and
        /// the trigger has run to the end. The engine draws first: the log today is
        ///     turnStarted, manaChanged, promptOpened, drawn, addedToHand, promptAnswered, …
        /// with the turn's draw landing INSIDE the open prompt. That is the same bug class R62/R113 just
        /// fixed at the end of a turn — the end of turn now parks its remainder in `state.work` and the
        /// answer finishes it (R122) — still present at the start of one.
        ///
        /// THE FIX IS NOT IN THIS DIRECTORY: `packages/engine/src/turn.ts`'s `startTurn` runs
        /// `queueHooksInTriggerOrder` + `settle` and then `draw` straight through, so a pause inside the
        /// trigger queue leaves the draw to run under the prompt. It needs the end of turn's shape: a
        /// start-of-turn work item that owes the draw, parked when the triggers pause and resumed by the
        /// action that answers (R113, R117, R122). Everything else in this test passes — the resume does
        /// happen, the draw does happen and the turn does land in `main`; only the ORDER is wrong.
        #[test]
        fn r62_radiant_resumes_across_the_prompt_the_draw_comes_after_the_start_of_turn_trigger() {
            let mut s = scn(json!({
                "seed": SEED,
                "p1": {
                    "field": [{ "def": GRAVEDIGGER, "radiant": true }],
                    "graveyard": [STOCKPILE, HIT_JOB, MANA_WELL],
                    "library": [DRAWN],
                },
                "p2": { "hand": [STOCKPILE], "library": [STOCKPILE] },
            }));

            s.start_turn().answer(json!(MANA_WELL));

            assert!(s.state().pending.is_none());
            assert!(s.state().work.is_empty());
            // The draw that follows the start-of-turn hook happened, and the picked card is there too.
            assert_eq!(sorted(def_ids_in_hand(&s)), sorted(strings(&[DRAWN, MANA_WELL])));
            assert_eq!(s.state().phase, Phase::Main);
            // R62's order, as a subsequence: the Discover opens, is answered and puts its pick in hand,
            // and only then does the turn draw. Red until `turn.ts` owes the draw to `state.work`.
            s.expect_events(json!(["promptOpened", "promptAnswered", "addedToHand", "drawn", "addedToHand"]));
        }

        #[test]
        fn radiant_does_nothing_with_an_empty_graveyard_no_prompt_at_all_sec6_3() {
            let mut s = scn(json!({
                "seed": SEED,
                "p1": { "field": [{ "def": GRAVEDIGGER, "radiant": true }], "library": [DRAWN] },
                "p2": { "hand": [STOCKPILE], "library": [STOCKPILE] },
            }));

            s.start_turn();

            assert!(s.state().pending.is_none());
            assert_eq!(def_ids_in_hand(&s), strings(&[DRAWN]));
        }
    }
}
