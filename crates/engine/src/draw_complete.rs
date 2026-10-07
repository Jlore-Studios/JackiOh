//! The "draw complete" point of a cast-on-draw draw (SPEC §2.4, R58, R70; Classic #9 Income Tax).
//!
//! R58: a cast-on-draw card is cast the moment it is drawn, and the draw is complete once that cast
//! has resolved — so a trap or trigger answering the draw (its `drawn` event) answers it then, after
//! the cast, and never inside it. The cast is a play with windows of its own (its announce, its step
//! 4), and each window offers every event so far to the traps (R70), which would hand them the
//! `drawn` that found the card before the card had resolved. So the draw holds its `drawn` back from
//! every dispatch (`triggers::dispatch_new_events` passes over a held one and keeps its place) until the
//! cast's pipeline has finished (`play_steps::drive`), and the loop that runs next delivers it.
//!
//! A leaf over `state.held_draws`: the instance ids the drawn cards had when they were drawn (a cast
//! Trap set face-down takes a fresh id, R227, and a Devil's Pact replacement is a new card, R449 —
//! neither moves the hold). Plain JSON, so a cast that pauses keeps its draw held across the answer.
//!
//! Port of `packages/engine/src/drawComplete.ts`.

use crate::state::GameState;
use crate::wire::GameEvent;

/// Where a cast-on-draw cast carries the id its card was drawn under (`resolve::cast_card`'s options
/// data), so the play pipeline knows which draw its finish completes.
pub const CAST_ON_DRAW_KEY: &str = "__castOnDraw";

/// R58: hold this draw's `drawn` until the cast of the card it drew has resolved.
pub fn hold_draw(state: &mut GameState, instance_id: &str) {
    let mut held = state.held_draws.clone().unwrap_or_default();
    if !held.iter().any(|id| id == instance_id) {
        held.push(instance_id.to_string());
        state.held_draws = Some(held);
    }
}

/// R58: the cast has resolved, so the draw is complete and its `drawn` may be answered.
pub fn release_draw(state: &mut GameState, instance_id: &str) {
    let Some(held) = state.held_draws.as_ref() else {
        return;
    };
    if !held.iter().any(|id| id == instance_id) {
        return;
    }
    let rest: Vec<String> = held.iter().filter(|id| *id != instance_id).cloned().collect();
    if rest.is_empty() {
        state.held_draws = None;
    } else {
        state.held_draws = Some(rest);
    }
}

/// Whether a dispatch must pass over this event for now: the `drawn` of a draw not yet complete.
pub fn held_back(state: &GameState, event: &GameEvent) -> bool {
    match event {
        GameEvent::Drawn { instance_id, .. } => state
            .held_draws
            .as_ref()
            .is_some_and(|held| held.iter().any(|id| id == instance_id)),
        _ => false,
    }
}
