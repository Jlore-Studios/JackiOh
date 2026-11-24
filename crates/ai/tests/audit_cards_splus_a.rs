//! Audit (Stinkyscape+ cards, first half: S+ #1 to #100).

use jackioh_engine::testkit::*;

fn scenario_with_cards(opts: serde_json::Value) -> Scenario {
    jackioh_cards::register_all();
    scenario(opts)
}

#[test]
fn smoke_builds() {
    let s = scenario_with_cards(json!({}));
    assert!(s.state.winner.is_none());
}
