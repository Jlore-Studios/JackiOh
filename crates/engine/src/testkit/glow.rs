//! R195, R662: reading the yellow glow (`conditionActive`) off a viewer's own `view_for`, for the
//! card tests that prove it in their own file (README §5). The key is present and `true`, or absent:
//! a key present with any other value fails here.
//!
//! Port of `packages/cards/test/_glow.ts` (part 5). The view is read as its JSON, as TS read the
//! object: "is the key there" is a question about the wire, not about a Rust `Option`.

use serde::Serialize;
use serde_json::Value;

use crate::testkit::scenario::Scenario;
use crate::wire::PlayerId;

/// `card` as its JSON, `None` for no card at all (TS `null` or `undefined`).
fn as_json<T: Serialize>(card: Option<&T>) -> Option<Value> {
    let value = serde_json::to_value(card?).expect("a view serialises");
    if value.is_null() { None } else { Some(value) }
}

/// `true` when the view carries the key (which must then be exactly `true`), `false` when absent.
pub fn glows<T: Serialize>(card: Option<&T>) -> bool {
    let Some(card) = as_json(card) else {
        panic!("no card at that place in the view");
    };
    let Some(active) = card.get("conditionActive") else {
        return false;
    };
    assert_eq!(
        active,
        &Value::Bool(true),
        "conditionActive is present and not true"
    );
    true
}

/// The viewer's own view, as its JSON.
fn view_json(s: &Scenario, viewer: PlayerId) -> Value {
    serde_json::to_value(s.view(viewer)).expect("a view serialises")
}

/// Does this hand card glow in its holder's own view right now?
pub fn hand_glows(s: &Scenario, instance_id: &str, viewer: PlayerId) -> bool {
    let view = view_json(s, viewer);
    let Some(hand) = view["you"]["hand"].as_array() else {
        panic!("the viewer's own hand must travel in full (§10.8)");
    };
    glows(
        hand.iter()
            .find(|card| card.get("instanceId").and_then(Value::as_str) == Some(instance_id)),
    )
}

/// Does the card in this backrow lane (1-based) glow in its controller's own view right now?
pub fn backrow_glows(s: &Scenario, lane: usize, viewer: PlayerId) -> bool {
    let view = view_json(s, viewer);
    glows(
        view["you"]["backrow"]
            .as_array()
            .and_then(|row| row.get(lane.wrapping_sub(1))),
    )
}

/// The other seat's view of that backrow lane: a face-down trap is a bare back, with no glow on it.
pub fn opponent_sees_glow(s: &Scenario, lane: usize, owner: PlayerId) -> bool {
    let other = owner.opponent();
    let view = view_json(s, other);
    let zone = view["opponent"]["backrow"]
        .as_array()
        .and_then(|row| row.get(lane.wrapping_sub(1)))
        .filter(|zone| !zone.is_null());
    let Some(zone) = zone else {
        panic!("no card at that place in the view");
    };
    zone.get("conditionActive").is_some()
}
