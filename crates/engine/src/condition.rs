//! Hearthstone's yellow glow (SPEC §10.8, §10.9, R195). A playable card lights up yellow when its
//! printed condition is met — Combo, "if your hero is below N" — and the engine decides that, never
//! the client (CLAUDE.md rule 7; SURFACE §4.1). `view_for` asks only about the viewer's own hand, the
//! top of a unit pile and a public backrow card.
//!
//! The first rule that applies decides: game over, a field card the viewer does not control, or a hand
//! card outside the viewer's own main phase or with a prompt open -> false, hook not called (a hook
//! runs only for its controller, who may read all the Core hooks read, §9.1); the running face's
//! `conditionMet` strictly true -> true (a fused def carries its ingredients' hooks, or-ed, R196);
//! a hand card granted a condition that holds now (R662) -> true; else false. The grant is the
//! engine's, not a card's, since the card being played takes the branch; it reads the facts §10.5
//! steps 3 and 5 decide with (`query.rs`).

use crate::query::{gifted_would_make_radiant, granted_combo_live};
use crate::script::{ConditionContext, ConditionZone};
use crate::state::{CardInstance, GameState};
use crate::subsystems::copied_text::{running_script_of, text_face_of};
use crate::wire::{Phase, PlayerId};

/// R195: whether `viewer` sees `card` glowing yellow. Pure; calls the hook at most once.
pub fn condition_active(
    state: &GameState,
    card: &CardInstance,
    viewer: PlayerId,
    zone: ConditionZone,
) -> bool {
    if state.result.is_some() {
        return false;
    }
    if zone == ConditionZone::Field && card.controller != viewer {
        return false;
    }
    if zone == ConditionZone::Hand
        && !(state.phase == Phase::Main && state.active == viewer && state.pending.is_none())
    {
        return false;
    }
    // B5 E14, R547: a copier (Classic #57 Echo) glows for the condition of the text it has.
    let hook = running_script_of(state, card).condition_met.clone();
    if let Some(hook) = hook {
        let face: CardInstance = text_face_of(state, card).clone();
        let met = hook(ConditionContext {
            state,
            self_: &face,
            controller: viewer,
            radiant: face.radiant,
            zone,
            your_turn: state.active == viewer,
        });
        if met {
            return true;
        }
    }
    zone == ConditionZone::Hand && granted_condition_holds(state, card, viewer)
}

/// R662: a condition another card grants this hand card holds now — playing it would take the
/// Combo branch a Quickstriker or a Radiant /fullsend gives "your cards", or Gifted Program would make
/// it Radiant as it is played.
fn granted_condition_holds(state: &GameState, card: &CardInstance, viewer: PlayerId) -> bool {
    granted_combo_live(state, viewer) || gifted_would_make_radiant(state, viewer, card)
}
