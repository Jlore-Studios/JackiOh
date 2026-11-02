//! Tradable and the Trade (§6.1, R1838000–R1838003), Hearthstone's Tradeable as the designer wrote it:
//! in their own main phase a player may put a Tradable card from their hand into their deck at a
//! random position and draw a card, at `TRADE_COST` (no mana), up to their Trades a turn
//! (`TRADES_PER_TURN`, or the figure an effect set for them), counted over all their Tradable cards.
//!
//! One function says whether a Trade is legal, `why_trade_refused`, which `legal_actions`, the reducer
//! and the view's `tradeReason` all ask, so a greyed-out control and a refusal cannot disagree (§9.3).
//! Stinkyscape's trade cards (#1864) add a hand aura, more Trades a turn and the traded card's id.
//!
//! The Trade is hidden information like a draw (R1838001): `traded` names only the player, the card
//! goes in through `draw::shuffle_into_library` (its slot hidden from both and the card from the other
//! player, R97, shown to its owner, R311) and comes out through §2.4's draw. It is no play, no cast and
//! no discard (R1838002).

use serde_json::Value;

use crate::config::{LIBRARY_CAP, TRADE_COST, TRADES_PER_TURN};
use crate::script::{AuraEntry, AuraHook, EffectContext, EngineSink, HookArgs, StatMod};
use crate::state::{CardInstance, DrawCount, EngineError, GameState};
use crate::wire::{GameEvent, Keyword, KeywordKind, Phase, PlayerId, Zone};

/// R1838000, R1864002: whether this card may be Traded from a hand, read through the layers (§10.4) as
/// Cloaked is (`alt_play::is_cloaked`): printed, granted, or lent by an aura reaching its hand.
pub fn is_tradable(state: &GameState, card: &CardInstance) -> bool {
    crate::layers::unit_has(state, card, KeywordKind::Tradable)
}

/// R1838000, R1864004: the Trades `player` may make each turn: their own figure or the most a card on
/// their field gives, the larger, never a sum. Every rule that reads the limit reads it here.
pub fn trades_per_turn_of(state: &GameState, player: PlayerId) -> i32 {
    let own = state.players[player].trades_per_turn.unwrap_or(TRADES_PER_TURN);
    trades_given(state, player).map_or(own, |given| own.max(given))
}

/// R1864004: the most Trades a turn any card acting on `player`'s field gives (`Script::trades_per_turn`);
/// a Unit a carrier holds is one (R446), and a face-down card's text sleeps (R1487000).
fn trades_given(state: &GameState, player: PlayerId) -> Option<i32> {
    let mut most: Option<i32> = None;
    for source in crate::zones::acting_on(state, player, crate::zones::StandingAt::ItsLane) {
        if source.controller != player
            || source.vanilla
            || crate::alt_play::is_dormant(source)
            || crate::preview::lasting_text_off(state, source)
        {
            continue;
        }
        let script = crate::scripts::script_of(state, source);
        let Some(hook) = script.trades_per_turn.as_ref() else {
            continue;
        };
        let given = hook(HookArgs {
            state,
            self_: source,
            radiant: source.radiant,
        });
        most = Some(most.map_or(given, |most| most.max(given)));
    }
    most
}

/// R1864002: "Aura: Cards in your hand have Tradable." (The Syndicate, The Trade Union).
pub fn tradable_hand_aura() -> AuraHook {
    crate::script::aura_hook(|args| {
        let controller = args.self_.controller;
        vec![AuraEntry {
            applies: Box::new(
                move |card: &CardInstance| matches!(card.zone, Zone::Hand { player } if player == controller),
            ),
            mod_: StatMod {
                keywords: Some(vec![Keyword::Tradable]),
                ..StatMod::default()
            },
        }]
    })
}

/// R1864003: the key a trigger on `traded` finds the traded card's id under in its data.
pub const TRADED_ID_KEY: &str = "tradedId";

/// R1864003: the card the Trade being answered put into a deck, read as a trigger on its `traded` is
/// queued. `traded` names no card (R1838001), but the loop offers an action's events in order from
/// `state.dispatch` (`triggers::dispatch_new_events`) and `apply_trade` reports the `shuffledIn` right
/// after `traded`, so the next event owed (passing a cast-on-draw `drawn` held back, R58) is that
/// shuffle-in. `None` when it is not one: a deck at R80's cap turned the card away.
pub fn traded_card_after(state: &GameState) -> Option<String> {
    let next = state
        .dispatch
        .iter()
        .find(|item| !crate::draw_complete::held_back(state, &item.event))?;
    match &next.event {
        GameEvent::ShuffledIn { instance_id, .. } => Some(instance_id.clone()),
        _ => None,
    }
}

/// R1864003: in a trigger on `traded`, the card that Trade put into a deck; `None` in any other hook.
pub fn traded_card_of(ctx: &EffectContext<'_>) -> Option<String> {
    ctx.data
        .get(TRADED_ID_KEY)
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// R1838000: the Trades `player` has made this turn (a count kept for an earlier turn reads as 0).
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

/// R1838000, R1853003: why `player` may not take a main-phase deal now (a Trade, or a buy or a sale at
/// the Marketplace), or `Ok`.
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

/// R1838000: why `player` may not Trade the card `instance_id` names now, or `Ok`. Its last check is
/// room in their deck for the card (R80's cap, which would otherwise turn it away).
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
    if crate::query::room_in(state, player, crate::zones::OffFieldZone::Library) == 0 {
        return Err(EngineError::new(format!(
            "your deck is full ({LIBRARY_CAP} cards)"
        )));
    }
    Ok(())
}

/// R1838000, R1838001: the Trade. Refused as `why_trade_refused` refuses it; otherwise it pays
/// `TRADE_COST`, counts the Trade, reports `traded`, shuffles the card into its owner's deck at a
/// random position under the top card (R2657003) and draws one card for `player`, a draw like any other
/// (§2.4, R1838002).
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
    // R2657003: never back on top, so the draw brings another card while the deck holds one.
    crate::draw::shuffle_into_library_below_top(sink, &mut card);
    crate::draw::draw(sink, player, 1);
    Ok(())
}
