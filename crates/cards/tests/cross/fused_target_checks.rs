//! R102, §10.6: a fused card carries every ingredient's declared targets, and a declaration's filter
//! may name a predicate (`TargetFilter.check`) in its card's `targetChecks`. Those are pure reads asked
//! with the candidate, not hooks that return lists, so the fused card must answer them with a boolean.
//!
//! Found by the fuzz gate: a fused card of two ingredients that each define `targetChecks` (here Classic
//! #48 Hired Shrimp and a fusion holding C+ #41 KY's Constant) made `legalActions` throw "Cannot read
//! properties of undefined (reading '__partDepth')" for as long as it sat in a hand, and `reduce` with
//! it, because every action ends by asking whether the turn is over (R82). The combiner wrapped each
//! predicate as a fused Cry; one ingredient defining the object was never wrapped, so no test saw it.
//!
//! Port of `packages/cards/test/fused-target-checks.test.ts` (SURFACE §4.1, §8). TS's
//! `expect(() => legalActions(…)).not.toThrow()` is the call itself: a panic fails the test.

use jackioh_engine::PlayerId::P1;
use jackioh_engine::subsystems::fuse::FuseArgs;
use jackioh_engine::testkit::*;

const KYS_CONSTANT: &str = "classicplus-041"; // (1) Spell: Cry, pick a hand card (check: "number").
const REWIND: &str = "classic-054"; // (1) Spell: Cry, pick an ally Unit or graveyard card with a Cry (check: "hasCry").
const SHRIMP: &str = "classic-048"; // (2) Unit with a Cry: a card Rewind's predicate admits.
const HIT_JOB: &str = "core-016"; // a hand card for KY's Constant's pick to name.
const ANCHOR: &str = "core-010"; // a free Spell that keeps the turn open; never played.

use super::scenario;

/// TS `craft(s, ingredients)`: `subsystems.fuse` over a sink on the scenario's state, with an event
/// list of its own and an rng at the state's cursor that nothing writes back.
fn craft(s: &mut Scenario, ingredients: [CardInstance; 2]) -> CardInstance {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
    let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
    let fused = subsystems::fuse::fuse(
        &mut sink,
        FuseArgs {
            ingredients: ingredients.to_vec(),
            to_hand: Some(P1),
            ..Default::default()
        },
    );
    match fused {
        Some(card) => card,
        None => panic!("the fusion did not happen"),
    }
}

/// The `play` actions `legalActions` offers p1 for this card.
fn plays_of(s: &Scenario, card: &CardInstance) -> Vec<ActionBody> {
    legal_actions(s.state(), P1)
        .into_iter()
        .filter(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == card.id))
        .collect()
}

mod r102_a_fused_cards_declared_targets_keep_working_when_two_ingredients_define_targetchecks {
    use super::*;

    #[test]
    fn r102_a_crafted_rewind_kys_constant_offers_plays_that_pick_for_both_declarations_instead_of_throwing_from_legalactions()
     {
        let mut s = scenario(json!({
            "seed": "fused-target-checks",
            "p1": { "hand": [REWIND, KYS_CONSTANT, HIT_JOB, ANCHOR], "field": [{ "def": SHRIMP, "lane": 1 }], "mana": 10 },
        }));
        let rewind = s.card(REWIND).clone();
        let constant = s.card(KYS_CONSTANT).clone();
        let fused = craft(&mut s, [rewind, constant]);

        // not.toThrow(): listing the legal actions must not panic.
        let _listed = legal_actions(s.state(), P1);
        // Rewind's pick (an ally with a Cry) and KY's Constant's (a hand card), in ingredient order.
        assert!(plays_of(&s, &fused).iter().any(
            |play| matches!(play, ActionBody::Play { targets: Some(targets), .. } if targets.len() == 2)
        ));
    }

    #[test]
    fn r102_ingredients_that_name_the_same_predicate_must_each_admit_the_candidate_a_crafted_kys_constant_kys_constant()
     {
        let mut s = scenario(json!({
            "seed": "fused-target-checks",
            "p1": { "hand": [KYS_CONSTANT, KYS_CONSTANT, HIT_JOB, ANCHOR], "mana": 10 },
        }));
        let constants: Vec<CardInstance> = s
            .hand(P1)
            .iter()
            .filter(|card| card.def_id == KYS_CONSTANT)
            .cloned()
            .collect();
        let (Some(first), Some(second)) = (constants.first().cloned(), constants.get(1).cloned()) else {
            panic!("setup: two KY's Constant in hand");
        };
        let fused = craft(&mut s, [first, second]);

        // not.toThrow(): listing the legal actions must not panic.
        let _listed = legal_actions(s.state(), P1);
        assert!(!plays_of(&s, &fused).is_empty());
    }
}
