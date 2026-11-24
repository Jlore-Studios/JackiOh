//! TEMPORARY debug probe (removed before delivery): dump Afterburner's events.

#[test]
fn tmp_debug_afterburner_events() {
    jackioh_cards::register_all();
    let mut s = crate::testkit::scenario(serde_json::json!({
        "p1": { "hand": ["infiniteassault-056"] },
    }));
    eprintln!(
        "BEFORE p1hero={} p2hero={} p1hand={} p2hand={}",
        s.state().players[crate::PlayerId::P1].hero.health,
        s.state().players[crate::PlayerId::P2].hero.health,
        s.state().players[crate::PlayerId::P1].hand.len(),
        s.state().players[crate::PlayerId::P2].hand.len(),
    );
    for (i, c) in s.state().players[crate::PlayerId::P1].hand.iter().enumerate() {
        eprintln!("p1hand[{i}] = {} {}", c.def_id, c.id);
    }
    for (i, c) in s.state().players[crate::PlayerId::P2].hand.iter().enumerate() {
        eprintln!("p2hand[{i}] = {} {}", c.def_id, c.id);
    }
    s.play("infiniteassault-056", serde_json::json!({}));
    for e in s.events() {
        let v = serde_json::to_value(e).unwrap_or(serde_json::Value::Null);
        let kind = v.get("kind").cloned().unwrap_or(serde_json::Value::Null);
        if kind == "damage" || kind == "cardPlayed" || kind == "cardResolved" || kind == "hookResolving" {
            eprintln!("EVENT {v}");
        }
    }
    eprintln!(
        "AFTER p1hero={} p2hero={}",
        s.state().players[crate::PlayerId::P1].hero.health,
        s.state().players[crate::PlayerId::P2].hero.health,
    );
    panic!("dump only");
}
