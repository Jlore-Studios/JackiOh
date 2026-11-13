//! The one-pick `Selection[]` a test hands to a play's `targets` or a prompt's `answer` (an instance,
//! a hero or a Mode option), and `targets`, the play's `{ "targets": … }` around one. Not globbed in:
//! a test writes `pick::<builder>`. crates/tools/tests/acceptance.rs refuses a test's own copy of one,
//! whatever its name (#1826, #3827).

use std::fmt::Display;

use serde_json::{Value, json};

use crate::state::CardInstance;
use crate::testkit::scenario::{CardRef, Scenario, SeatRef};

/// The pick of the instance whose id is `instance_id`.
pub fn id(instance_id: &str) -> Value {
    json!([{ "pick": "instance", "instanceId": instance_id }])
}

/// The pick of `card`.
pub fn instance(card: &CardInstance) -> Value {
    id(&card.id)
}

/// The pick of the card `card` names in `s` (`Scenario::card`'s lookup).
pub fn card(s: &Scenario, card: impl Into<CardRef>) -> Value {
    instance(s.card(card))
}

/// The pick of the unit in `player`'s lane `lane`, which must hold one (`Scenario::unit_at`).
pub fn unit_at(s: &Scenario, player: impl SeatRef, lane: i32) -> Value {
    instance(&s.unit_at(player, lane))
}

/// The pick of the Mode option `option` (a Discover, Mode or Answer prompt's option string).
pub fn mode(option: impl AsRef<str>) -> Value {
    json!([{ "pick": "mode", "option": option.as_ref() }])
}

/// The pick of `player`'s hero (`"p1"`, `"p2"` or a `PlayerId`).
pub fn hero(player: impl Display) -> Value {
    json!([{ "pick": "hero", "player": player.to_string() }])
}

/// The `targets` of a play: `picks` (a builder above, or any `Selection[]`) as its only key.
pub fn targets(picks: Value) -> Value {
    json!({ "targets": picks })
}
