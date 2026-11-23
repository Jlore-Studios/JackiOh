//! R3575603: a decision keeps the expensive steps it simulated (`SimMemo`, the ones it pays more than a
//! node for) and answers the same step from the same state again without the engine, as the engine
//! would answer it, its nonce record rebuilt, and charging the same nodes. Zephyrs (core-097) is an
//! expensive card: its play ranks every shipped card (R29).

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, register_cards};

const ZEPHYRS: &str = "core-097";
const STOCKPILE: &str = "core-005";
const DECK: [&str; 4] = [STOCKPILE, STOCKPILE, STOCKPILE, STOCKPILE];

fn zephyrs_board() -> (GameState, ActionBody, ActionBody) {
    register_cards();
    let s = scenario(json!({
        "seed": "ai-sim-memo",
        "p1": { "hand": [ZEPHYRS, STOCKPILE], "library": DECK, "mana": 5 },
        "p2": { "hand": [STOCKPILE], "library": DECK },
    }));
    let play_of = |def_id: &str| {
        let id = s.card(def_id).id.clone();
        legal_actions(s.state(), AI)
            .into_iter()
            .find(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == id))
            .expect("playable")
    };
    (s.state().clone(), play_of(ZEPHYRS), play_of(STOCKPILE))
}

#[test]
fn r3575603_an_expensive_step_taken_again_is_answered_as_the_engine_answers_it() {
    let (state, zephyrs, _) = zephyrs_board();
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let first = simulate(&state, AI, &zephyrs, &counter, Prices::NONE)
        .expect("a node")
        .expect("accepted");
    let after_first = counter.used();
    assert_eq!(
        counter.sim_memo().map(SimMemo::len),
        Some(1),
        "the expensive step is kept"
    );
    let second = simulate(&state, AI, &zephyrs, &counter, Prices::NONE)
        .expect("a node")
        .expect("accepted");
    assert_eq!(counter.used() - after_first, after_first, "the same nodes again");
    assert_eq!(counter.sim_memo().map(SimMemo::len), Some(1), "kept once");

    // The same step on a fresh decision's counter at the same count is what the engine answers.
    let fresh = create_node_counter(AI_BUDGET.nodes, None);
    for _ in 0..after_first {
        assert!(fresh.take());
    }
    let engine = simulate(&state, AI, &zephyrs, &fresh, Prices::NONE)
        .expect("a node")
        .expect("accepted");
    assert_eq!(second, engine, "the memo's answer, nonce record included");
    assert_eq!(first.players, second.players);
}

#[test]
fn r3575603_a_cheap_step_is_not_kept() {
    let (state, _, stockpile) = zephyrs_board();
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    simulate(&state, AI, &stockpile, &counter, Prices::NONE)
        .expect("a node")
        .expect("accepted");
    assert_eq!(counter.sim_memo().map(SimMemo::len), Some(0));
}

#[test]
fn r3575603_the_same_step_from_a_state_differing_only_in_its_nonces_is_answered_again() {
    let (state, zephyrs, _) = zephyrs_board();
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    simulate(&state, AI, &zephyrs, &counter, Prices::NONE)
        .expect("a node")
        .expect("accepted");
    let mut renamed = state.clone();
    renamed.applied.push(AppliedAction {
        nonce: "an earlier step".to_string(),
        events: Vec::new(),
    });
    let before = counter.used();
    let memo = simulate(&renamed, AI, &zephyrs, &counter, Prices::NONE)
        .expect("a node")
        .expect("accepted");
    let fresh = create_node_counter(AI_BUDGET.nodes, None);
    for _ in 0..before {
        assert!(fresh.take());
    }
    let engine = simulate(&renamed, AI, &zephyrs, &fresh, Prices::NONE)
        .expect("a node")
        .expect("accepted");
    assert_eq!(memo, engine);
    assert_eq!(
        counter.sim_memo().map(SimMemo::len),
        Some(1),
        "answered from the memo"
    );
}
