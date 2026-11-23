//! Counter a Ranged attack or an Activate (Stinkyscape+ #43 Segment, SPEC §6.3 Counter, R3550005).
//!
//! A declared Ranged attack is canceled before §4.2 step 4's trap window, and an `activate` action's
//! effect never runs — the attack deals no damage, the Activate does nothing, and the Activate's use
//! and costs stay spent. One budget a turn is shared by both. A forced attack opens no window (R121)
//! and is never countered.

use serde_json::Value;

use crate::script::EngineSink;
use crate::state::{CardInstance, find_instance_mut};
use crate::wire::{GameEvent, KeywordKind, PlayerId, Row, has_keyword};

/// Where a counter trap notes what it has countered this turn, `{ turn, count }`, as an Activate
/// ability notes its uses (`subsystems::activate`).
const COUNTERED_KEY: &str = "@countered";

/// How much `card` has countered on the running turn; a note from another turn is 0.
fn countered_this_turn(state: &crate::state::GameState, card: &CardInstance) -> i64 {
    card.memory
        .get(COUNTERED_KEY)
        .filter(|record| record.get("turn").and_then(Value::as_i64) == Some(i64::from(state.turn)))
        .and_then(|record| record.get("count").and_then(Value::as_i64))
        .unwrap_or(0)
}

/// The first trap that may counter for anyone but `against`, in R68's order: on the field, held by
/// another side, with no per-turn budget (`counters_ranged_all`) or one it has not spent this turn
/// (its declared `perTurn`, R386).
fn counter_for(state: &crate::state::GameState, against: PlayerId) -> Option<CardInstance> {
    crate::traps::traps_in_order(state)
        .into_iter()
        .filter(|card| card.zone.z() == crate::wire::ZoneName::Field && card.controller != against)
        .find(|card| {
            let flags = crate::scripts::flags_of(state, card);
            if flags.counters_ranged_all == Some(true) {
                return true;
            }
            let Some(budget) = flags.counters_ranged else {
                return false;
            };
            let budget = crate::params::declared_or(state, card, "perTurn", budget);
            countered_this_turn(state, card) < i64::from(budget)
        })
}

/// Spend one of `card`'s counters: bump its note, announce the firing and turn it face-up, as
/// `traps::fire_trap` does.
fn spend(sink: &mut EngineSink<'_>, card: &CardInstance) {
    let turn = sink.state.turn;
    let count = countered_this_turn(sink.state, card) + 1;
    let at = crate::zones::slot_of(sink.state, card);
    let again = crate::traps::fires_again(sink.state, card);
    if let Some(live) = find_instance_mut(sink.state, &card.id) {
        live.memory.insert(
            COUNTERED_KEY.to_string(),
            serde_json::json!({ "turn": turn, "count": count }),
        );
    }
    sink.events.push(GameEvent::TrapFired {
        instance_id: card.id.clone(),
        def_id: card.def_id.clone(),
        controller: card.controller,
        row: at.map_or(Row::Backrow, |slot| slot.row),
        lane: at.map_or(0, |slot| slot.lane),
        again,
    });
    // §5.1: it has fired, and it is face-up from this moment (R33).
    if let Some(live) = find_instance_mut(sink.state, &card.id) {
        live.face_up = Some(true);
    }
}

/// R3550005: counter a declared Ranged attack — the attack deals no damage. Only a Ranged attacker
/// is countered, and a forced attack never reaches here (R121). True when a trap countered it.
pub(crate) fn counter_declared_attack(sink: &mut EngineSink<'_>, attacker: &CardInstance) -> bool {
    if !has_keyword(
        &crate::layers::unit_view(sink.state, attacker).keywords,
        KeywordKind::Ranged,
    ) {
        return false;
    }
    let Some(card) = counter_for(sink.state, attacker.controller) else {
        return false;
    };
    spend(sink, &card);
    if let Some(open) = sink.state.declared_attack.as_mut() {
        open.cancelled = true;
        let attacker_id = open.attacker_id.clone();
        let target_id = open.target_id.clone();
        sink.events.push(GameEvent::AttackCancelled {
            attacker_id,
            target_id,
            by_instance_id: card.id.clone(),
        });
    }
    crate::traps::consume_trap(sink, &card);
    true
}

/// R3550005: counter an `activate` action — the Activate does nothing, its use and costs staying
/// spent by the caller. True when a trap countered it.
pub(crate) fn counter_activation(sink: &mut EngineSink<'_>, player: PlayerId) -> bool {
    let Some(card) = counter_for(sink.state, player) else {
        return false;
    };
    spend(sink, &card);
    crate::traps::consume_trap(sink, &card);
    true
}
