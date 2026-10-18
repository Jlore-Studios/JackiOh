//! Frozen (SPEC §6.1, §6.3 Freeze, R1174000): a Hearthstone-style status. A Frozen Unit loses its
//! next attack or position switch, and thaws at the end of the turn in which it could have made one.
//!
//! The status is `CardInstance.frozen`, the turn it thaws by, and nothing else reads a freeze but
//! through this module and `combat::has_exertion_in`, which answers false for both halves of a
//! Frozen Unit's exertion, so `legal_actions`, the reducer's refusals, the view's `canAct` and the
//! AI's lethal solver all agree (§4.1). An effect's attack or switch spends no exertion (R20, R53)
//! and is not stopped.
//!
//! The thaw is stamped as the freeze lands and never worked out later (`thaw_turn`): a Unit frozen
//! during its controller's Start or Main phase while it still has an exertion to spend (§4.1, R49,
//! R636) has lost that opportunity and thaws at this turn's cleanup; any other Unit stays Frozen
//! through its controller's next turn and thaws at that turn's cleanup. The stamp is a turn number
//! read against its controller's cleanups only (`thaw_at_cleanup`), so an extra turn (R845) of
//! either player moves nothing: a Unit thaws at the cleanup of its controller's first turn numbered
//! its stamp or later. A change of control (R171) stamps it again for its new controller
//! (`restamp`).
//!
//! Every freeze and thaw is said with R437's reusable mark (`FROZEN_MARK`): `marked` with `added:
//! true` as it lands and `added: false` as it thaws or a Vanilla removes it, so the client's brand
//! flourish, its sound cue and its log need no new event. A Unit leaving the field loses the status
//! with R78's reset (`zones::reset_instance`), silently, as Berserk does.

use crate::combat::{ExertionKind, exertion_unspent};
use crate::config::FROZEN_MARK;
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
    let Some(unit) = find_instance(sink.state, instance_id) else {
        return;
    };
    let thaw = thaw_turn(sink.state, unit);
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
    if unit.frozen.is_none() {
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
