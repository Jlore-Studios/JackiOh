//! Redacting a redacted state changes nothing (R185, SURFACE §14.1). The training arena's referee
//! sends each agent `redact(state, seat)`, and `decide` redacts what it is given, so every arena
//! decision redacts twice. That second pass has to be the identity: on placeholders, which have no
//! definition to read (an opponent's face-down trap keeps the price R351 showed), and on everything
//! else, so the AI an arena plays decides exactly as the AI of the quality gates does on the true
//! state.
//!
//! Rust only: no TypeScript test ports to this, since the agent protocol is v0.3.0's (part 29). Found
//! by part 34's `promote --dry-run`, where every decision facing a face-down trap panicked with
//! `unknown defId "ai:hidden"`.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, card_by_id, random_policy_states};

fn text(state: &GameState) -> String {
    serde_json::to_string(state).expect("a GameState serialises")
}

/// p1 (the AI) faces p2's face-down Siphon Squad, whose aura is live (R602) and whose cost both
/// players see (R351).
fn facing_a_face_down_trap() -> GameState {
    jackioh_cards::register_all();
    scenario(json!({
        "seed": "redact-twice",
        "active": "p1",
        "turn": 9,
        "p1": { "field": ["core-019", "core-011"], "hand": ["core-008"], "mana": 3, "library": ["core-020", "core-053"] },
        "p2": { "backrow": [{ "def": "classic-088", "faceUp": false, "radiant": true }], "library": ["core-005", "core-016"] },
    }))
    .state()
    .clone()
}

fn ai_options(seed: &str) -> AiOptions<'static> {
    AiOptions {
        rng: create_rng(seed, 0),
        budget: AI_BUDGET,
        should_stop: None,
    }
}

/// A face-down trap's placeholder keeps the price it shows, and the second pass is the identity
#[test]
fn a_face_down_traps_placeholder_keeps_its_shown_price_when_redacted_again() {
    let state = facing_a_face_down_trap();
    let trap = state.players[HUMAN]
        .backrow
        .iter()
        .flatten()
        .next()
        .cloned()
        .expect("a face-down trap");
    let once = redact(&state, AI);
    let twice = redact(&once, AI);
    let placeholder = card_by_id(&twice, &trap.id).expect("the trap stays on the board");
    assert_eq!(placeholder.def_id, HIDDEN_DEF_ID);
    assert_eq!(
        placeholder.cost_override,
        card_by_id(&once, &trap.id).and_then(|card| card.cost_override)
    );
    assert!(placeholder.cost_override.is_some());
    assert_eq!(text(&twice), text(&once));
}

/// The AI decides the same on the redacted state as on the true one
#[test]
fn the_ai_decides_the_same_on_the_redacted_state_as_on_the_true_one() {
    let state = facing_a_face_down_trap();
    let on_truth = decide(&state, AI, &mut ai_options("redact-twice-decide"));
    let on_view = decide(&redact(&state, AI), AI, &mut ai_options("redact-twice-decide"));
    assert!(on_truth.is_some());
    assert_eq!(on_view, on_truth);
}

/// Redacting twice is redacting once, for both seats, all through random-policy games
#[test]
fn redacting_twice_is_redacting_once_through_random_policy_games() {
    let mut face_down_seen = 0;
    for k in 1..=6 {
        for state in random_policy_states(&format!("redact-twice-{k}"), 2, 400) {
            for seat in PLAYER_IDS {
                let once = redact(&state, seat);
                face_down_seen += once.players[seat.opponent()]
                    .backrow
                    .iter()
                    .flatten()
                    .filter(|card| card.def_id == HIDDEN_DEF_ID)
                    .count();
                assert_eq!(
                    text(&redact(&once, seat)),
                    text(&once),
                    "redact-twice-{k}, turn {}, {}",
                    state.turn,
                    seat.as_str()
                );
            }
        }
    }
    assert!(face_down_seen > 0, "the games set no face-down card");
}
