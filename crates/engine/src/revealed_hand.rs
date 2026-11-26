//! R4592003: revealed hand state (Infinite Assault #191 Ground Truth).
//!
//! While Ground Truth acts on a player's side, their opponent plays with their hand revealed to
//! that player.

use crate::state::GameState;
use crate::wire::{PlayerId, opponent_of};

/// Whether `holder`'s hand is revealed to `viewer` by a card acting with `reveals_opponents_hand`.
pub fn hand_revealed_to(state: &GameState, holder: PlayerId, viewer: PlayerId) -> bool {
    holder == opponent_of(viewer)
        && crate::query::acting_with_flag(state, viewer, |f| f.reveals_opponents_hand)
}
