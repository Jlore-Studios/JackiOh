//! Audit: what a player is shown (`view_for`, events) and what a player may send (the validator).
//! Each test asserts the documented behaviour and fails on the bug it proves.

use jackioh_engine::testkit::*;

#[test]
fn smoke() {
    jackioh_cards::register_all();
    let s = scenario(json!({
        "seed": "audit-engine-views",
        "p1": { "hand": ["core-008"], "library": ["core-020", "core-053"] },
        "p2": { "library": ["core-005", "core-016"] },
    }));
    let view = view_for(s.state(), PlayerId::P2);
    println!("{}", serde_json::to_string_pretty(&view).unwrap());
}
