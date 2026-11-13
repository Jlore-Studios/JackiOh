//! The Warhero Token's worth (SPEC §6.3, Stinkyscape+ E5; R3548002): one game-wide value in
//! Credits every Warhero Token is worth. It starts at 0 once the first Warhero Token has been
//! minted anywhere, and grows by a random 0 to `WARHERO_GROWTH_MAX` at the end of every turn after
//! one has. A Radiant one is worth double. "They do nothing when played other than give credits":
//! the worth is paid through the Resource's own rules (`resources.rs`, `marketplace.rs`).

use crate::config::{
    WARHERO_GROWTH_MAX, WARHERO_GROWTH_MIN, WARHERO_RADIANT_MULTIPLIER, WARHERO_TOKEN_DEF_ID,
};
use crate::script::EngineSink;
use crate::state::{CardInstance, GameState};
use crate::wire::{CounterKind, GameEvent};

/// The `counterChanged` id the worth travels under: a game-wide count, not a card's.
pub const WARHERO_WORTH_COUNTER_ID: &str = "warhero";

/// R3548002: whether this definition is a Warhero Token.
pub fn is_warhero_token(def_id: &str) -> bool {
    def_id == WARHERO_TOKEN_DEF_ID
}

/// R3548002: the first Warhero Token minted starts the game-wide worth at 0. A later mint, or a
/// mint while it is already up, changes nothing.
pub fn note_minted(state: &mut GameState, def_id: &str) {
    if is_warhero_token(def_id) && state.warhero_worth.is_none() {
        state.warhero_worth = Some(0);
    }
}

/// R3548002: what this card pays in Credits: the game-wide worth, doubled on its Radiant face, or
/// 0 when no Warhero Token has been minted yet. `None` for a card that is no Warhero Token.
pub fn worth_of(state: &GameState, card: &CardInstance) -> Option<i32> {
    if !is_warhero_token(&card.def_id) {
        return None;
    }
    let worth = state.warhero_worth.unwrap_or(0);
    Some(if card.radiant {
        worth * WARHERO_RADIANT_MULTIPLIER
    } else {
        worth
    })
}

/// R3548002: at the end of every turn, once any Warhero Token has been minted, the game-wide worth
/// grows by a random `WARHERO_GROWTH_MIN` to `WARHERO_GROWTH_MAX` from the match rng, reported as a
/// `warhero` `counterChanged` with the growth it added. With no Warhero Token minted, nothing is
/// drawn and nothing is reported.
pub fn grow_at_cleanup(sink: &mut EngineSink<'_>) {
    let Some(worth) = sink.state.warhero_worth else {
        return;
    };
    let growth = WARHERO_GROWTH_MIN + sink.rng.int(WARHERO_GROWTH_MAX - WARHERO_GROWTH_MIN + 1);
    let value = worth + growth;
    sink.state.warhero_worth = Some(value);
    sink.events.push(GameEvent::CounterChanged {
        instance_id: WARHERO_WORTH_COUNTER_ID.to_string(),
        counter: CounterKind::Warhero,
        value,
        placed: Some(growth),
    });
}
