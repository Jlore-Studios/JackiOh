//! R429: how many times a card has been played (SPEC §10.1, §10.5 step 4), for the one Core card that
//! counts its own plays, #31 KY's Math Equation — "Deal Fib(times played + 1) damage".
//!
//! The count lives on the instance, like `costMod`, so it rides the card through every zone and
//! through leaving the field (R78 leaves it alone), and a copy or a Transform — a new instance — starts
//! its own (R57). It is written in one place, §10.5 step 4, where every play is counted (the turn log,
//! the game's `played` counter): a cast is a play there too (R70), and a countered play never reaches
//! that step, so it is never counted. Only a card whose script sets `StaticFlags.countsPlays` carries
//! the field at all, so a game without one hashes as it did before the field existed.
//!
//! Port of `packages/engine/src/timesPlayed.ts`.

use crate::state::{CardInstance, GameState, find_instance_mut};

/// R429: the plays this card has had so far, the one under way included once step 4 has run.
pub fn times_played_of(card: &CardInstance) -> i32 {
    match card.times_played {
        Some(count) if count > 0 => count,
        _ => 0,
    }
}

/// R429, §10.5 step 4: one more play of this card, when its script counts them.
///
/// TS wrote through the live card; here `card` is the card as the caller holds it (its flags are
/// read off it) and the count is written on the instance of that id in `state`.
pub fn count_play(state: &mut GameState, card: &CardInstance) {
    if crate::scripts::script_of(state, card).flags().counts_plays != Some(true) {
        return;
    }
    if let Some(live) = find_instance_mut(state, &card.id) {
        live.times_played = Some(times_played_of(live) + 1);
    }
}
