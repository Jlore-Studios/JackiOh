//! R429: how many times a card has been played (SPEC §10.1, §10.5 step 4), for #31 KY's Math
//! Equation ("Deal Fib(times played + 1) damage").
//!
//! The count lives on the instance, so it rides the card through every zone and off the field (R78
//! leaves it alone), and a copy or a Transform, a new instance, starts its own (R57). Only §10.5 step
//! 4 counts a play: a cast is one there too (R70), and a countered play never reaches it. Only a card
//! whose script sets `counts_plays` carries the field, so a game without one never serializes it.

use crate::state::{CardInstance, GameState, find_instance_mut};

/// R429: the plays this card has had so far, the one under way included once step 4 has run.
pub fn times_played_of(card: &CardInstance) -> i32 {
    match card.times_played {
        Some(count) if count > 0 => count,
        _ => 0,
    }
}

/// R429, §10.5 step 4: one more play of this card, when its script counts them. Its flags are
/// read off `card` as the caller holds it; the count goes on the live instance of that id.
pub fn count_play(state: &mut GameState, card: &CardInstance) {
    if crate::scripts::script_of(state, card).flags().counts_plays != Some(true) {
        return;
    }
    if let Some(live) = find_instance_mut(state, &card.id) {
        live.times_played = Some(times_played_of(live) + 1);
    }
}
