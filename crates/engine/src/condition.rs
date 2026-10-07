//! Hearthstone's yellow glow (SPEC §10.8, §10.9, R195). A playable card lights up yellow when its
//! printed condition is met — Combo, "if your hero is below N" — and the engine decides that, never
//! the client (CLAUDE.md rule 7). `view_for` asks this module about three kinds of card only: the
//! viewer's own hand, the top of a unit pile and a public backrow card. Everything else (graveyard,
//! exile, resolving, buried cards, the opponent's hand) is never asked.
//!
//! The rules, in order; the first that applies decides:
//!   1. the game is over                                            -> false
//!   2. a field card the viewer does not control                    -> false, hook not called
//!   3. a hand card outside the viewer's own main phase, or with a
//!      prompt open (the only time it could not be played now)      -> false, hook not called
//!   4. the running face's `conditionMet` answers strictly `=== true`  -> true
//!      (a transient def with no registered script gets EMPTY_SCRIPT
//!      from `scriptsFor`; a fused def carries its ingredients' hooks,
//!      or-ed, R196)
//!   5. a hand card a permanent or a modifier grants a condition that
//!      holds now (R662: #38's and #78's Combo, #64's Radiant)       -> true
//!   6. otherwise                                                    -> false
//!
//! Rule 2 is what keeps this from leaking: a hook only ever runs for its controller, who may read
//! everything the Core hooks read (hero health, library counts, plays this turn, a grade, §9.1).
//!
//! Rule 5 is the engine's and no card's: the condition is printed on the card that grants it, but it
//! is the card being played that takes the branch, so the granting card's script cannot be asked
//! about it. It reads the same facts §10.5 steps 3 and 5 decide with (`query.rs`), and only the
//! viewer's own plays, modifiers and permanents, which are public.
//!
//! Port of `packages/engine/src/condition.ts` (SURFACE §4.1).

use crate::query::{gifted_would_make_radiant, granted_combo_live};
use crate::script::{ConditionContext, ConditionZone};
use crate::state::{CardInstance, GameState};
use crate::subsystems::copied_text::{running_script_of, text_face_of};
use crate::wire::{Phase, PlayerId};

/// R195: whether `viewer` sees `card` glowing yellow. Pure; calls the hook at most once.
pub fn condition_active(state: &GameState, card: &CardInstance, viewer: PlayerId, zone: ConditionZone) -> bool {
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
