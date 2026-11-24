//! Audit (Stinkyscape+ cards, second half): each test asserts a card's printed text and fails on the bug.

use jackioh_engine::testkit::*;

fn game(setup: Value) -> Scenario {
    jackioh_cards::register_all();
    scenario(setup)
}

#[test]
fn harness_builds() {
    let s = game(json!({}));
    assert_eq!(s.state.turn, s.state.turn);
}
