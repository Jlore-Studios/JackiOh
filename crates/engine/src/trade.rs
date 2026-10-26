//! Tradable and the Trade (§6.1, R1838000–R1838003), Hearthstone's Tradeable as the designer wrote it:
//! in their own main phase a player may put a Tradable card from their hand into their deck at a
//! random position and draw a card, at `TRADE_COST` (no mana), up to their Trades a turn
//! (`TRADES_PER_TURN`, or the figure an effect set for them), counted over all their Tradable cards.
//!
//! One function says whether a Trade is legal, `why_trade_refused`: `legal_actions` lists a `trade`
//! for each hand card it passes, the reducer refuses what it refuses in its words, and the view gives
//! the viewer's own Tradable hand card its refusal as `tradeReason` and their Trades as `trades`
//! (`view_for.rs`), so a greyed-out control and a refusal cannot disagree (§9.3).
//!
//! A card is Tradable when its running face prints the keyword or an effect granted it — the set a card
//! in a hand is made of (`card_keywords`), as Temporary is read (`temporary.rs`).
//!
//! The Trade is hidden information like a draw (R1838001): `traded` names only the player, the card
//! goes in through the one shuffle-in the engine has (`draw::shuffle_into_library`: `shuffledIn`, its
//! slot hidden from both and the card from the other player, R97, shown to its owner, R311) and the card
//! comes out through §2.4's draw (`draw::draw`: `drawn`, hidden from the other player). It is no play,
//! no cast and no discard (R1838002).

use crate::config::{LIBRARY_CAP, TRADE_COST, TRADES_PER_TURN};
use crate::layers::card_keywords;
use crate::script::EngineSink;
use crate::state::{CardInstance, DrawCount, EngineError, GameState};
use crate::wire::{GameEvent, KeywordKind, Phase, PlayerId, has_keyword};

/// R1838000: whether this card may be Traded from a hand: its running face prints Tradable or an effect
/// gave it the keyword.
pub fn is_tradable(state: &GameState, card: &CardInstance) -> bool {
    has_keyword(&card_keywords(state, card), KeywordKind::Tradable)
}

/// R1838000: the Trades `player` may make each turn — `TRADES_PER_TURN`, or their own figure once an
/// effect has set one. Every rule that reads the limit reads it here.
pub fn trades_per_turn_of(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].trades_per_turn.unwrap_or(TRADES_PER_TURN)
}

/// R1838000: the Trades `player` has made this turn. `PlayerState.trades` is kept per turn, so a count
/// from an earlier turn reads as 0, as a draw count does (`draw::draws_this_turn`).
pub fn trades_this_turn(state: &GameState, player: PlayerId) -> i32 {
    match state.players[player].trades {
        Some(trades) if trades.turn == state.turn => trades.count,
        _ => 0,
    }
}

/// R1838000: the Trades `player` has left this turn, never below 0.
pub fn trades_left(state: &GameState, player: PlayerId) -> i32 {
    (trades_per_turn_of(state, player) - trades_this_turn(state, player)).max(0)
}

/// R1838000, R1853003: why `player` may not take a main-phase deal now — a Trade, or a buy or a sale
/// at the Marketplace (`marketplace.rs`) — or `Ok`: a live game in their own main phase with no
/// prompt open.
pub fn why_not_in_main_phase(state: &GameState, player: PlayerId) -> Result<(), EngineError> {
    if state.result.is_some() {
        return Err(EngineError::new("the game is over"));
    }
    if state.pending.is_some() || state.mulligan.is_some() {
        return Err(EngineError::new("a prompt is open: answer it first"));
    }
    if state.active != player || state.phase != Phase::Main {
        return Err(EngineError::new("it is not your turn"));
    }
    Ok(())
}

/// R1838000: why `player` may not Trade the card `instance_id` names now, or `Ok`. The one rule:
/// `legal_actions`, the reducer and the view all ask it. In order: a live game in `player`'s own main
/// phase with no prompt open, the card in their hand and Tradable, a Trade left this turn, the mana for
/// `TRADE_COST`, and room in their deck for the card (R80's cap, which would otherwise turn it away).
pub fn why_trade_refused(state: &GameState, player: PlayerId, instance_id: &str) -> Result<(), EngineError> {
    why_not_in_main_phase(state, player)?;
    let side = &state.players[player];
    let Some(card) = side.hand.iter().find(|card| card.id == instance_id) else {
        return Err(EngineError::new(format!("no card {instance_id} in your hand")));
    };
    if !is_tradable(state, card) {
        let name = &crate::catalog::def_of(Some(state), &card.def_id).name;
        return Err(EngineError::new(format!("{name} is not Tradable")));
    }
    if trades_left(state, player) <= 0 {
        let limit = trades_per_turn_of(state, player);
        let noun = if limit == 1 { "Trade" } else { "Trades" };
        return Err(EngineError::new(format!(
            "you have made this turn's {limit} {noun}"
        )));
    }
    let mana = side.mana.current;
    if mana < TRADE_COST {
        return Err(EngineError::new(format!(
            "a Trade costs {TRADE_COST} mana and you have {mana}"
        )));
    }
    if side.library.len() as i32 >= LIBRARY_CAP {
        return Err(EngineError::new(format!(
            "your deck is full ({LIBRARY_CAP} cards)"
        )));
    }
    Ok(())
}

/// R1838000, R1838001: the Trade. Refused as `why_trade_refused` refuses it; otherwise it pays
/// `TRADE_COST`, counts the Trade, reports `traded`, shuffles the card into its owner's deck at a
/// random position and draws one card for `player`, a draw like any other (§2.4, R1838002).
pub fn apply_trade(
    sink: &mut EngineSink<'_>,
    player: PlayerId,
    instance_id: &str,
) -> Result<(), EngineError> {
    why_trade_refused(sink.state, player, instance_id)?;
    let Some(mut card) = sink.state.players[player]
        .hand
        .iter()
        .find(|card| card.id == instance_id)
        .cloned()
    else {
        return Err(EngineError::new(format!("no card {instance_id} in your hand")));
    };
    if TRADE_COST != 0 {
        let side = &mut sink.state.players[player];
        crate::mana::spend_mana(side, TRADE_COST);
        let paid = crate::mana::mana_event(player, side);
        sink.events.push(paid);
    }
    let count = trades_this_turn(sink.state, player) + 1;
    sink.state.players[player].trades = Some(DrawCount {
        turn: sink.state.turn,
        count,
    });
    sink.events.push(GameEvent::Traded { player });
    crate::draw::shuffle_into_library(sink, &mut card, true, None);
    crate::draw::draw(sink, player, 1);
    Ok(())
}
