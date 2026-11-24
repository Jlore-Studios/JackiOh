//! Audit of how a card is played (`play_choices.rs`, `play_steps.rs` and their neighbours). Temporary.

use jackioh_engine::testkit::*;

#[test]
fn builds() {
    jackioh_cards::register_all();
    let s = scenario(json!({}));
    assert!(!legal_actions(s.state(), PlayerId::P1).is_empty());
}
