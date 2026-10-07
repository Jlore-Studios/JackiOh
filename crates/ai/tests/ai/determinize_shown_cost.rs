//! R762: the AI samples an unseen face-down trap at the cost the board shows.
//!
//! Since R351 every face-down Trap shows its cost to both players, and `redact` keeps that number
//! on the placeholder — but `determinize` filled each placeholder from every Trap and Field Trap,
//! so a sampled world could put a cost-1 trap where the board shows 3. The lethal solver accepts a
//! line that wins on all 3 determinizations, so it swung at the face into a shown cost-3 trap most
//! of the time, where a human reading the same cost would not.
//!
//! Port of `packages/ai/test/determinize-shown-cost.test.ts`. TS's `{ timeout: 60_000 }` has no
//! `cargo test` twin and is dropped.

use indexmap::IndexSet;
use jackioh_ai::{AiOptions, DecisionReason, DeterminizeOptions, decide, determinize, redact};
use jackioh_engine::testkit::{
    CardCost, CardInstance, CostOptions, GameState, create_rng, def_of, effective_cost, json,
};

use super::support::{AI, HUMAN, card_by_id, is_legal, register_cards, scenario};

const DOOM_SHROOM: &str = "classicplus-001";
const GROOM_SHROOM: &str = "classicplus-002";

fn one_of_each_cost() -> GameState {
    register_cards();
    scenario(json!({
        "seed": "shown-cost-lanes",
        "active": AI,
        "turn": 9,
        "p1": { "field": ["core-008"], "library": ["core-020"] },
        "p2": {
            "backrow": [
                { "def": "core-041", "faceUp": false },
                { "def": "classic-017", "faceUp": false },
                { "def": DOOM_SHROOM, "faceUp": false },
            ],
            "library": ["core-005"],
        },
    }))
    .state()
    .clone()
}

/// Vanilla 4 + Timmy 3 is exactly 7, so the two swings are lethal. Doom Shroom exiles both attackers
/// on the first swing. Groom Shroom lets the first swing hit (R405) and walls the second behind
/// Taunts. So no world at (3) has a lethal. Do not cut this to one attacker: a Groom world would
/// then still be lethal.
fn shroomed(trap: bool) -> GameState {
    register_cards();
    let backrow = if trap {
        json!([{ "def": DOOM_SHROOM, "faceUp": false }])
    } else {
        json!([])
    };
    scenario(json!({
        "seed": "shown-cost-doom",
        "active": AI,
        "turn": 9,
        "p1": { "field": ["core-008", "core-011"], "library": ["core-020", "core-053"] },
        "p2": {
            "health": 7,
            "backrow": backrow,
            "library": ["core-005", "core-016"],
        },
    }))
    .state()
    .clone()
}

fn face_down(state: &GameState) -> Vec<CardInstance> {
    state.players[HUMAN].backrow.iter().flatten().cloned().collect()
}

mod determinize_at_the_shown_cost_r762 {
    use super::*;

    #[test]
    fn r762_every_face_down_card_showing_a_cost_is_sampled_as_a_trap_of_that_cost_over_100_seeds() {
        let state = one_of_each_cost();
        let lanes = face_down(&state);
        let shown: Vec<i32> = lanes
            .iter()
            .map(|card| effective_cost(&state, card, CostOptions::default()))
            .collect();
        assert_eq!(shown, vec![1, 2, 3]);
        let mut picked: IndexSet<String> = IndexSet::new();
        for k in 0..100 {
            let world = determinize(
                &redact(&state, AI),
                AI,
                &mut create_rng(&format!("shown-cost-lanes:{k}"), 0),
                DeterminizeOptions::default(),
            );
            for (at, lane) in lanes.iter().enumerate() {
                let def_id = card_by_id(&world, &lane.id)
                    .map(|card| card.def_id.clone())
                    .unwrap_or_default();
                assert_eq!(
                    def_of(Some(&world), &def_id).cost,
                    CardCost::Fixed(shown[at]),
                    "seed {k}, lane {}: {def_id}",
                    at + 1
                );
                picked.insert(def_id);
            }
        }
        assert!(picked.contains(DOOM_SHROOM));
        assert!(picked.contains(GROOM_SHROOM));
    }

    #[test]
    fn r762_the_greedy_baselines_sampler_match_shown_cost_false_still_fills_a_back_with_a_trap_of_any_cost() {
        let state = one_of_each_cost();
        let lanes = face_down(&state);
        let seen = redact(&state, AI);
        let mut costs: IndexSet<CardCost> = IndexSet::new();
        for k in 0..100 {
            let world = determinize(
                &seen,
                AI,
                &mut create_rng(&format!("shown-cost-greedy:{k}"), 0),
                DeterminizeOptions {
                    match_shown_cost: Some(false),
                },
            );
            let def_id = card_by_id(&world, &lanes[2].id)
                .map(|card| card.def_id.clone())
                .unwrap_or_default();
            costs.insert(def_of(Some(&world), &def_id).cost);
        }
        assert!(costs.len() > 1);
    }

    #[test]
    fn r762_with_no_face_down_card_the_two_swings_are_a_lethal_the_ai_takes() {
        let decision = decide(
            &shroomed(false),
            AI,
            &mut AiOptions::new(create_rng("shown-cost-doom:control", 0)),
        );
        assert_eq!(decision.map(|d| d.reason), Some(DecisionReason::Lethal));
    }

    #[test]
    fn r762_never_returns_the_swing_into_a_face_down_doom_shroom_showing_3_as_lethal_on_ai_seeds_1_20() {
        let state = shroomed(true);
        for seed in 1..=20 {
            let decision = decide(
                &state,
                AI,
                &mut AiOptions::new(create_rng(&format!("shown-cost-doom:{seed}"), 0)),
            );
            // TS's `decision?.action ?? { type: "endTurn" }` never falls back: a null decision fails here.
            let decision = decision.unwrap_or_else(|| panic!("seed {seed}: no decision"));
            assert_ne!(
                decision.reason,
                DecisionReason::Lethal,
                "seed {seed}: {}",
                serde_json::to_string(&decision.line).unwrap()
            );
            assert!(is_legal(&state, AI, &decision.action), "seed {seed}");
        }
    }
}
