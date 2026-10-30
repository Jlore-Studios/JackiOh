//! The one-instance `Selection[]` a test hands to a play's `targets` or a prompt's `answer`. Not
//! globbed in: a test writes `pick::<builder>`. crates/tools/tests/acceptance.rs refuses a test's own
//! copy (#1826).

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
