//! Crater (#783, R783010, R783011): a Unit that Locks its own zone as it arrives on the field, and a
//! Trap or Field Trap that Locks its backrow zone as it is revealed. The Lock is R688's: it refuses
//! plays into the zone, a Stack play included, and never a summon or a move; it outlasts the card and
//! stays until an Unlock.
//!
//! Every arrival calls `lock_on_arrival` right after its event, so the zone is Locked before its Cry:
//! a play or cast (`play_steps::place_card`), any summon (`effects/summon`), a Reborn body
//! (`state_check`), Joro's summon (`replacements`), a Flicker (`effects/flicker`), a Cloaked reveal
//! (`alt_play`). A control change, swap, rotation, animation, Transform or restored board is no
//! arrival. Crater on a Spell does nothing; a Field Spell with it Locks its backrow zone (R1850006).

use crate::script::EngineSink;
use crate::state::{CardInstance, GameState, find_instance};
use crate::wire::{CardType, GameEvent, KeywordKind, Row, Zone};
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
/// backrow zone Locks that one (R1081000), and so does a face-up Field Spell entering a backrow zone
/// (R1850006).
pub fn lock_on_arrival(sink: &mut EngineSink<'_>, id: &str) {
    let Some(card) = find_instance(sink.state, id) else {
        return;
    };
    if field_spell_in_backrow(sink.state, card) {
        if crate::layers::unit_has(sink.state, card, KeywordKind::Crater)
            && let Some(at) = slot_of(sink.state, card)
        {
            lock(sink, at);
        }
        return;
    }
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

/// R1850006: a face-up Field Spell in a backrow zone of its own, not a Unit standing there.
fn field_spell_in_backrow(state: &GameState, card: &CardInstance) -> bool {
    matches!(
        card.zone,
        Zone::Field {
            row: Row::Backrow,
            ..
        }
    ) && !crate::zones::stands_in_backrow(state, card)
        && !crate::preview::is_face_down(state, card)
        && crate::faces::card_type_of(state, card) == CardType::FieldSpell
}

/// R783011: a Trap or Field Trap with Crater Locks the backrow zone it stood in as it is revealed, as
/// it fires face-down: `trap` as the firing matched it, `at` its zone then (`traps::fire_trap`, and
/// `replacements` for one firing as it replaces, §5.1). A Field Trap firing again Locks nothing more,
/// and a card set face-down by its Cloaked is not a Trap by its face.
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
