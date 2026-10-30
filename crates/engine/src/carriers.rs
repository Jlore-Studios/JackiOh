//! A backrow card that carries a Unit (docs/classic-sets.md B5 E21, Classic+ #33 Ivory Tower; R446).
//!
//! The Unit stands in the carrier's backrow zone (`PlayerState.carried`) as a Unit for every rule
//! (`zones::active_units_of`) but can neither attack nor be attacked (`combat.rs`); the carrier still
//! acts beneath it, and backrow effects find the carrier, never the Unit (`zones::card_at`). The zone
//! takes nothing more meanwhile (`zones::accepts_stack_card`). When the zone stops carrying (the
//! carrier left, or a Vanilla took its text), the Unit steps down into a unit zone without leaving the
//! field, or is destroyed for want of one; §4.5's check runs this first in every pass (`state_check`).

use serde_json::Value;

use crate::animated::FieldSink;
use crate::state::find_instance_mut;
use crate::wire::{GameEvent, PLAYER_IDS, Row};
use crate::zones::{ZoneSlot, first_entry_zone, is_open, slots_of, step_into_unit_zone};

/// R446: set in a stranded Unit's memory when it was destroyed for want of a unit zone and survived it
/// (Indestructible, R46), so it is destroyed once and not again at every check while it waits for one.
/// Memory, so R78 clears it when the card leaves the field.
const STRANDED_KEY: &str = "__stranded";

/// R446: every Unit whose backrow zone no longer carries it steps down into its side's unit zone in the
/// same lane when open, else R64's leftmost open one: a move, not leaving the field, so no Cry, no R78
/// reset and no summoning sickness, reported as `animated` with `carried`. With none open it is
/// destroyed (§6.3 Destroy, no killer); an Indestructible one waits where it is for a zone to open.
pub fn settle_carried(sink: &mut FieldSink<'_>) {
    for player in PLAYER_IDS {
        if sink.state.players[player].carried.is_none() {
            continue;
        }
        for slot in slots_of(player, Row::Backrow) {
            let Some(unit) = crate::zones::carried_at(sink.state, slot).cloned() else {
                continue;
            };
            if let Some(top) = crate::zones::card_at(sink.state, slot).cloned()
                && crate::zones::is_carrier(sink.state, &top)
            {
                continue;
            }
            let same = ZoneSlot {
                player,
                row: Row::Units,
                lane: slot.lane,
            };
            let to = if is_open(sink.state, same) {
                Some(same)
            } else {
                first_entry_zone(sink.state, player, Row::Units)
            };
            let Some(to) = to else {
                if unit.memory.get(STRANDED_KEY) == Some(&Value::Bool(true)) {
                    continue;
                }
                if let Some(stranded) = find_instance_mut(sink.state, &unit.id) {
                    stranded
                        .memory
                        .insert(STRANDED_KEY.to_string(), Value::Bool(true));
                    stranded.marked_destroyed = Some(true);
                    // R42: a destroy names no killer, and clears an older hit's credit as it marks.
                    stranded.last_damaged_by = None;
                }
                continue;
            };
            if !step_into_unit_zone(sink.state, &unit, to) {
                continue;
            }
            if let Some(moved) = find_instance_mut(sink.state, &unit.id) {
                // The other memory keys keep their order.
                moved.memory.shift_remove(STRANDED_KEY);
            }
            sink.events.push(GameEvent::Animated {
                player,
                instance_id: unit.id.clone(),
                def_id: unit.def_id.clone(),
                backrow_lane: slot.lane,
                unit_lane: to.lane,
                carried: Some(true),
            });
        }
    }
}
