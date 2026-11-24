//! Audit of the opponent model (`crates/ai/src/reply.rs`): each test asserts the documented behaviour.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

const AI: PlayerId = PlayerId::P1;
const HUMAN: PlayerId = PlayerId::P2;

#[test]
fn warm_up() {
    jackioh_cards::register_all();
    let s = scenario(json!({ "seed": "audit-reply", "p1": { "field": ["core-008"] }, "p2": { "field": ["core-008"] } }));
    let state = s.state().clone();
    assert_eq!(state.active, AI);
    let _ = HUMAN;
    let _ = hidden_card_ids(&state, AI);
}
