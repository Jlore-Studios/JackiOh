//! Stinkyscape+ S14 (issue #3565) meets the AI: the only prompt its cards open is #140 Automa
//! Reintegration Plan's Discover of three Automa Units (R3565005). It needs nothing new: `legal_actions`
//! lists every answer, the beam scores each one through `reduce`, and `decide` answers within its budget
//! with reason "prompt". The random policy and the timeout answer it too, in the card's own tests
//! (`crates/cards/src/scripts/stinkyscape_plus/c140_automa_reintegration_plan.rs`).

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, ai_options, clone, is_legal, register_cards, scenario};

/// S+ #140 Automa Reintegration Plan: (1) Spell, "Discover an Automa Unit, it gains Starship Piece".
const REINTEGRATION_PLAN: &str = "stinkyscapeplus-140";
/// Core #8 Mr. Vanilla: a (1) 4/4 with no text, the library and hand filler.
const VANILLA: &str = "core-008";

fn library() -> Value {
    json!([VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA])
}

/// The AI's Discover of the plan, open: the plan is played from its hand.
fn discovering(seed: &str, radiant: bool) -> GameState {
    register_cards();
    let mut s = scenario(json!({
        "seed": seed,
        "p1": { "hand": [{ "def": REINTEGRATION_PLAN, "radiant": radiant }], "mana": 1, "library": library() },
        "p2": { "hand": [VANILLA], "library": library() },
    }));
    s.play(REINTEGRATION_PLAN, json!({}));
    s.state().clone()
}

/// decide on the Discover the AI holds: a legal answer, reason "prompt", within the budget.
fn answered(state: &GameState, seed: &str) -> ActionBody {
    assert_eq!(state.pending.as_ref().map(|pending| pending.player_id), Some(AI));
    assert_eq!(
        state.pending.as_ref().map(|pending| pending.kind),
        Some(PromptKind::Discover)
    );
    let decision = decide(&clone(state), AI, &mut ai_options(seed, AI_BUDGET)).expect("a decision");
    assert_eq!(decision.reason, DecisionReason::Prompt);
    assert!(decision.stats.nodes <= AI_BUDGET.nodes);
    assert_eq!(decision.stats.sim_errors, 0);
    assert!(matches!(decision.action, ActionBody::Answer { .. }));
    assert!(is_legal(state, AI, &decision.action));
    decision.action
}

#[test]
fn r3565005_the_ai_answers_the_discover() {
    for radiant in [false, true] {
        let seed = format!("ai-s14-discover-{radiant}");
        let state = discovering(&seed, radiant);
        let action = answered(&state, &seed);
        // The answer resolves the prompt and the pick reaches the hand given Starship Piece.
        let after = reduce(&state, &Action::new(action, AI, "ai-s14-answer".to_string()));
        assert!(after.error.is_none(), "{:?}", after.error);
        assert!(after.state.pending.is_none(), "radiant {radiant}");
        let held = after.state.players[AI]
            .hand
            .iter()
            .find(|card| card.granted_keywords.contains(&Keyword::StarshipPiece))
            .expect("the pick is in the hand");
        assert_eq!(held.radiant, radiant);
    }
}
