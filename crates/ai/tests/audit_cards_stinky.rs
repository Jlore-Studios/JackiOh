//! Audit (Stinkyscape cards, S #1 to #177 and the set's tokens).

use jackioh_engine::testkit::*;

#[allow(dead_code)]
const P1: PlayerId = PlayerId::P1;
#[allow(dead_code)]
const P2: PlayerId = PlayerId::P2;

#[allow(dead_code)]
fn scenario_with_cards(opts: serde_json::Value) -> Scenario {
    jackioh_cards::register_all();
    scenario(opts)
}

#[test]
fn harness_builds() {
    let s = scenario_with_cards(json!({
        "p1": { "hand": ["core-005"], "library": ["core-005", "core-005", "core-005"] },
        "p2": { "hand": ["core-005"], "library": ["core-005", "core-005", "core-005"] },
    }));
    assert!(s.state().pending.is_none());
}
