//! The Hallucination prompts the AI must answer (R4586002, R4586005): Unreliable Narrator's
//! pick and Phantom Lancer's target, each open for the AI seat with a legal answer.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, ai_options, is_legal, register_cards};

const NARRATOR: &str = "infiniteassault-031";
const LANCER: &str = "infiniteassault-026";
const LIBRARY: &[&str] = &[
    "core-008", "core-005", "core-001", "core-020", "core-011", "core-019",
];

/// Narrator played by the AI seat, its look's pick open.
fn narrator_open(seed: &str) -> GameState {
    register_cards();
    let mut s = scenario(json!({
        "seed": seed,
        "p1": { "hand": [NARRATOR, "core-005"], "library": LIBRARY },
        "p2": { "hand": ["core-005"], "library": LIBRARY },
    }));
    s.play(NARRATOR, json!({ "zone": 1 }));
    assert!(s.state().pending.is_some(), "the look's pick is open");
    s.state().clone()
}

/// The AI seat's Lancer set face-down, revealed, its target prompt open.
fn lancer_open(seed: &str) -> GameState {
    register_cards();
    let mut s = scenario(json!({
        "seed": seed,
        "p1": { "hand": [LANCER, "core-005"], "library": LIBRARY },
        "p2": { "field": ["core-008"], "hand": ["core-005"], "library": LIBRARY },
    }));
    s.play(
        LANCER,
        json!({ "zone": 1, "row": "backrow", "faceDown": "cloaked" }),
    );
    let set = s.state().players[AI]
        .backrow
        .iter()
        .filter_map(|slot| slot.clone())
        .find(|card| card.def_id == LANCER)
        .expect("the set Lancer")
        .id;
    s.activate(
        &set,
        json!({ "ability": jackioh_engine::alt_play::CLOAKED_REVEAL }),
    );
    assert!(s.state().pending.is_some(), "the reveal's pick is open");
    s.state().clone()
}

#[test]
fn r4586005_the_ai_answers_unreliable_narrators_pick() {
    for k in ["ai-narrator-k1", "ai-narrator-k2"] {
        let state = narrator_open(k);
        let decision = decide(&state, AI, &mut ai_options(k, AI_BUDGET)).expect("a decision");
        assert!(
            is_legal(&state, AI, &decision.action),
            "{k}: {:?}",
            decision.action
        );
        assert_ne!(decision.reason, DecisionReason::Fallback, "{k}");
        assert_eq!(decision.stats.sim_panics, 0, "{k}");
    }
}

#[test]
fn r4586002_the_ai_answers_phantom_lancers_target() {
    for k in ["ai-lancer-k1", "ai-lancer-k2"] {
        let state = lancer_open(k);
        let decision = decide(&state, AI, &mut ai_options(k, AI_BUDGET)).expect("a decision");
        assert!(
            is_legal(&state, AI, &decision.action),
            "{k}: {:?}",
            decision.action
        );
        assert_ne!(decision.reason, DecisionReason::Fallback, "{k}");
        assert_eq!(decision.stats.sim_panics, 0, "{k}");
    }
}
