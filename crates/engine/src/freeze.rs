//! Frozen (SPEC §6.1, §6.3 Freeze, R1174000): a Hearthstone-style status. A Frozen Unit loses its
//! next attack or position switch, and thaws at the end of the turn in which it could have made one.
//! The status is `CardInstance.frozen`, the turn it thaws by, read only here and by
//! `combat::has_exertion_in`, so `legal_actions`, the refusals, the view's `canAct` and the AI's lethal
//! solver agree (§4.1); an effect's attack or switch spends no exertion (R20, R53) and is not stopped.
//!
//! The thaw is stamped as the freeze lands, never worked out later (`thaw_turn`, §4.1, R49, R636), and
//! read against its controller's cleanups only (`thaw_at_cleanup`), so an extra turn (R845) moves
//! nothing; a change of control (R171) stamps it again (`restamp`). Every freeze and thaw is R437's
//! `marked` (`FROZEN_MARK`), so the client needs no new event; leaving the field clears it silently
//! with R78's reset (`zones::reset_instance`). R1848000 (Stinkyscape #2 Absolute Zero): the stamp
//! `FROZEN_FOREVER`, a turn no cleanup reaches, which a change of control leaves as it is.

use crate::combat::{ExertionKind, exertion_unspent};
use crate::config::{FROZEN_FOREVER, FROZEN_MARK};
use crate::script::EngineSink;
use crate::state::{CardInstance, GameState, find_instance, find_instance_mut};
use crate::wire::{GameEvent, Phase, PlayerId};

/// R1174000: whether a Unit has an exertion left to spend this turn, frozen or not: an attack, a
/// switch, Deft's other half (R49), Windfury's second attack (R636), a summoning-sick Unit's switch
/// (§4.1). The exertion is the opportunity, not the legality of an action, so a Unit that could
/// never attack or switch has one all the same.
fn exertion_left(state: &GameState, unit: &CardInstance) -> bool {
    exertion_unspent(state, unit, ExertionKind::Attack) || exertion_unspent(state, unit, ExertionKind::Switch)
}

/// R1174000: the thaw a freeze landing on `unit` now is stamped with — the turn at whose cleanup, its
/// controller's, it thaws. This turn's, when it is the controller's turn, in its Start or Main phase,
/// and the Unit still has an exertion left; otherwise its controller's next turn's (frozen on the
/// opponent's turn, after it spent its exertion, or in its controller's End phase).
pub fn thaw_turn(state: &GameState, unit: &CardInstance) -> i32 {
    let own_turn = state.active == unit.controller && matches!(state.phase, Phase::Start | Phase::Main);
    if own_turn && exertion_left(state, unit) {
        state.turn
    } else {
        state.turn + 1
    }
}

fn frozen_event(instance_id: &str, added: bool) -> GameEvent {
    GameEvent::Marked {
        instance_id: instance_id.to_string(),
        mark: FROZEN_MARK.mark.to_string(),
        color: FROZEN_MARK.color.to_string(),
        added,
    }
}

/// §6.3 Freeze: the Unit is Frozen, stamped with `thaw_turn`. A Unit already Frozen keeps the later of
/// its two thaws. The caller has checked it is a Unit on the field; a hero is never Frozen.
pub fn freeze_unit(sink: &mut EngineSink<'_>, instance_id: &str) {
    freeze_stamped(sink, instance_id, false);
}

/// R1848000 (Stinkyscape #2 Absolute Zero): "Freeze a Unit forever" — Frozen as `freeze_unit` makes
/// it, its thaw `FROZEN_FOREVER`, which no cleanup reaches and a later freeze never lowers, since a
/// Unit keeps the later of two thaws. Announced with the same `marked` "frozen" as any freeze.
pub fn freeze_unit_forever(sink: &mut EngineSink<'_>, instance_id: &str) {
    freeze_stamped(sink, instance_id, true);
}

/// R1848000: whether the Unit is Frozen forever (`freeze_unit_forever`), which only leaving the field
/// and a Vanilla end.
pub fn frozen_forever(unit: &CardInstance) -> bool {
    unit.frozen == Some(FROZEN_FOREVER)
}

fn freeze_stamped(sink: &mut EngineSink<'_>, instance_id: &str, forever: bool) {
    let Some(unit) = find_instance(sink.state, instance_id) else {
        return;
    };
    let thaw = if forever {
        FROZEN_FOREVER
    } else {
        thaw_turn(sink.state, unit)
    };
    let Some(live) = find_instance_mut(sink.state, instance_id) else {
        return;
    };
    live.frozen = Some(live.frozen.map_or(thaw, |was| was.max(thaw)));
    sink.events.push(frozen_event(instance_id, true));
}

/// R1174000: the status comes off a Unit that stays on the field (a Vanilla), with its `marked` thaw.
/// Nothing happens to a Unit that is not Frozen.
pub fn unfreeze(sink: &mut EngineSink<'_>, instance_id: &str) {
    let Some(live) = find_instance_mut(sink.state, instance_id) else {
        return;
    };
    if live.frozen.take().is_some() {
        sink.events.push(frozen_event(instance_id, false));
    }
}

/// R1174000, R171: a Frozen card whose controller changed stays Frozen, stamped again for its new
/// controller as if the freeze landed now (`combat::enter_new_side`, after its exertion is fresh).
pub fn restamp(state: &mut GameState, instance_id: &str) {
    let Some(unit) = find_instance(state, instance_id) else {
        return;
    };
    // R1848000: a forever freeze has no thaw to stamp again, so it stays forever on the new side.
    if unit.frozen.is_none() || frozen_forever(unit) {
        return;
    }
    let thaw = thaw_turn(state, unit);
    if let Some(live) = find_instance_mut(state, instance_id) {
        live.frozen = Some(thaw);
    }
}

/// §2.2 cleanup, beside `modifiers::expire_modifiers`: every Unit `player` controls on the field —
/// one dormant under a Stack included — whose thaw is this turn or earlier thaws, in lane order.
pub fn thaw_at_cleanup(sink: &mut EngineSink<'_>, player: PlayerId) {
    let turn = sink.state.turn;
    let thawing: Vec<String> = crate::zones::active_units_of(sink.state, player)
        .into_iter()
        .chain(crate::zones::dormant_units_of(sink.state, player))
        .filter(|unit| unit.controller == player && unit.frozen.is_some_and(|thaw| thaw <= turn))
        .map(|unit| unit.id.clone())
        .collect();
    for id in thawing {
        unfreeze(sink, &id);
    }
}

/// R1174000: whether the Unit is still Frozen through its controller's next turn — every Frozen Unit
/// but one that thaws at the cleanup of the turn under way, its controller's. The AI's evaluation
/// reads it (`face_threat_with`, `unit_worth`), which counts a Unit's swing next turn.
pub fn frozen_next_turn(state: &GameState, unit: &CardInstance) -> bool {
    match unit.frozen {
        None => false,
        Some(thaw) => !(state.active == unit.controller && thaw <= state.turn),
    }
}
