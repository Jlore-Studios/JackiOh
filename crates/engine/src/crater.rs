//! Crater (#783, R783010, R783011): a Unit that Locks its own zone as it arrives on the field, and a
//! Trap or Field Trap that Locks its backrow zone as it is revealed. The Lock is R688's: it refuses
//! plays into the zone, a Stack play included, and never a summon or a move; it outlasts the card and
//! stays until an Unlock.
//!
//! "Arrives" is every way a Unit gets onto the field — a play or a cast (`play_steps::place_card`), a
//! summon of any kind, a Recruit's included (`effects/summon`), a Reborn body (`state_check`), Joro's
//! summon (`replacements`), a Flicker (`effects/flicker`) and a Cloaked Unit's reveal (`alt_play`) —
//! each of which calls `lock_on_arrival` right after the event that put it there, so the zone is
//! Locked before its Cry. A control change, a board swap, a rotation, an animation, a Transform and a
//! restored board move a card that is already on the field and are no arrival. Crater on a Spell or a
//! Field Spell does nothing.

use crate::script::EngineSink;
use crate::state::{CardInstance, find_instance};
use crate::wire::{CardType, GameEvent, KeywordKind, Row};
use crate::zones::{ZoneSlot, is_locked, lock_zone, slot_of};

/// Lock one zone, reported as every Lock is (`locked`); a zone Locked already stays as it is.
fn lock(sink: &mut EngineSink<'_>, slot: ZoneSlot) {
    if is_locked(sink.state, slot) {
        return;
    }
    lock_zone(sink.state, slot);
    sink.events.push(GameEvent::Locked {
        player: slot.player,
        row: slot.row,
        lane: slot.lane,
    });
}

/// R783010: the card of this id, standing in a unit zone with Crater (read through the layers, §10.4),
/// Locks that zone. Called as it arrives on the field, before its Cry. A Flying Unit standing in a
/// backrow zone Locks that one (R1081000).
pub fn lock_on_arrival(sink: &mut EngineSink<'_>, id: &str) {
    let Some(card) = find_instance(sink.state, id) else {
        return;
    };
    if !crate::zones::in_units_row_or_standing(sink.state, card) {
        return;
    }
    if !crate::layers::unit_has(sink.state, card, KeywordKind::Crater) {
        return;
    }
    let Some(at) = slot_of(sink.state, card) else {
        return;
    };
    lock(sink, at);
}

/// R783011: a Trap or Field Trap with Crater Locks the backrow zone it stood in as it is revealed —
/// as it fires face-down, `trap` being the card as the firing matched it and `at` its zone then
/// (`traps::fire_trap`, and `replacements` for one that fires as it replaces, §5.1). A Field Trap
/// firing again is revealed already and Locks nothing more, and a card set face-down by its Cloaked is
/// not a Trap by its face.
pub fn lock_on_firing(sink: &mut EngineSink<'_>, trap: &CardInstance, at: Option<ZoneSlot>) {
    let Some(at) = at else {
        return;
    };
    if at.row != Row::Backrow || trap.face_up == Some(true) || trap.set_as.is_some() {
        return;
    }
    let printed = crate::faces::card_type_of_face(sink.state, &trap.def_id, trap.radiant);
    if printed != CardType::Trap && printed != CardType::FieldTrap {
        return;
    }
    if !crate::layers::unit_has(sink.state, trap, KeywordKind::Crater) {
        return;
    }
    lock(sink, at);
}
