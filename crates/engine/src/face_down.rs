//! The face-down board facts the Hallucination cards read (R4586001, R4586004).
//!
//! A "face-down card" is `preview::is_face_down`: a backrow Trap or Field Trap neither flipped
//! face-up nor revealed, read by whoever holds it. A card set by its Cloaked counts (it is the
//! Trap it was set as); a Revealed one does not. The ward is Paranoia Engine's: while it acts,
//! `destroy`, `destroyAll` and `reveal` run by an effect the opponent controls pass over its
//! controller's face-down cards.

use crate::preview::{is_face_down, lasting_text_off};
use crate::state::{CardInstance, GameState};
use crate::wire::PlayerId;

/// `player`'s backrow cards that are face-down, in lane order.
pub fn face_down_cards(state: &GameState, player: PlayerId) -> Vec<CardInstance> {
    state.players[player]
        .backrow
        .iter()
        .filter_map(|slot| slot.as_ref())
        .filter(|card| is_face_down(state, card))
        .cloned()
        .collect()
}

/// How many face-down cards `player` holds.
pub fn face_down_count(state: &GameState, player: PlayerId) -> i32 {
    face_down_cards(state, player).len() as i32
}

/// Whether `card`, a face-down card of its controller, is warded against an effect `by` runs:
/// `by` is the opponent and the controller has a card acting on the field whose lasting text is
/// on and whose static flags carry the ward.
pub fn face_down_warded(state: &GameState, card: &CardInstance, by: PlayerId) -> bool {
    if !is_face_down(state, card) || by == card.controller {
        return false;
    }
    crate::layers::aura_sources(state).iter().any(|ward| {
        ward.controller == card.controller
            && !lasting_text_off(state, ward)
            && crate::scripts::flags_of(state, ward).face_down_ward == Some(true)
    })
}
