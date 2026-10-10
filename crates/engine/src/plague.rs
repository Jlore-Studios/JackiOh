//! Plague Counters (SPEC §6.3 Plague Counter; docs/classic-sets.md B5 E19; R471): the counter on a
//! permanent, how many a placement puts there, and how they come off again.
//!
//! A Plague Counter is `instance.counters.plague` (§10.1), cleared by R78 when the card leaves the
//! field. This module owns all three things that touch it, so no caller can disagree about them:
//!   - a placement (`place_plague_on`): N tokens on one card, multiplied by what that card says
//!     (Classic #27 Pestilent Slime's "doubled", `Script.plague_multiplier`), reported once as
//!     `counterChanged` with `placed`, however many tokens (R471). Every gain is a placement, Core #91
//!     Fed Fauci's "+1 Plague Counter" included;
//!   - a removal (`remove_plague`): Classic #78 Mutate Spell's "remove a Plague Counter" and Classic
//!     #74 Corpse Plantation's tokens spent as mana, paid through this. It carries no `placed`;
//!   - the reads (`plague_on`, `plague_on_field`, `permanents_on_field`), re-exported by `query.rs`.

use crate::config::PLAGUE_MULTIPLIER_NONE;
use crate::script::{EngineSink, HookArgs};
use crate::scripts::script_of;
use crate::state::{CardInstance, GameState, find_instance_mut};
use crate::wire::{CounterKind, GameEvent, PlayerId, Row, ZoneName, opponent_of};
use crate::zones::{card_at, is_buried, slots_of};

/// The tokens a card carries now; an untouched card carries none (§6.3 Plague Counter).
pub fn plague_on(card: &CardInstance) -> i32 {
    card.counters.plague.unwrap_or(0).max(0)
}

/// Every permanent on the field in R68's order — the given side first (the active player's when
/// `first` is `None`), units lane 1 upward then the backrow lane 1 upward, then the other side: the
/// top of each unit pile only (R13), and every backrow card, face-down or not.
pub fn permanents_on_field(state: &GameState, first: impl Into<Option<PlayerId>>) -> Vec<&CardInstance> {
    let first = first.into().unwrap_or(state.active);
    let sides = [first, opponent_of(first)];
    let mut out: Vec<&CardInstance> = Vec::new();
    for player in sides {
        for row in [Row::Units, Row::Backrow] {
            for slot in slots_of(player, row) {
                if let Some(card) = card_at(state, slot) {
                    out.push(card);
                }
            }
        }
    }
    out
}

/// Classic #59 Plague Doctor: "the number of Plague Counters on the field" — every token on every
/// permanent, both sides, face-down cards included; or one side's only, with `player`.
pub fn plague_on_field(state: &GameState, player: impl Into<Option<PlayerId>>) -> i32 {
    let player: Option<PlayerId> = player.into();
    permanents_on_field(state, None)
        .into_iter()
        .filter(|card| player.is_none_or(|player| card.controller == player))
        .map(plague_on)
        .sum()
}

/// Whether a card can carry Plague Counters now: a permanent on the field, not dormant (R13).
pub fn carries_plague(state: &GameState, card: &CardInstance) -> bool {
    card.zone.z() == ZoneName::Field && !is_buried(state, card)
}

/// R471: what one placement onto this card is multiplied by — its text's `plague_multiplier` (Classic
/// #27), read on the face it wears; a card with no such text, or a Vanilla one (§6.3), multiplies by
/// 1. Never below 1, since a multiplier shrinks nothing.
pub fn plague_multiplier_of(state: &GameState, card: &CardInstance) -> i32 {
    let script = script_of(state, card);
    let Some(hook) = script.plague_multiplier.as_ref() else {
        return PLAGUE_MULTIPLIER_NONE;
    };
    let value = hook(HookArgs {
        state,
        self_: card,
        radiant: card.radiant,
    });
    PLAGUE_MULTIPLIER_NONE.max(value)
}

/// R471: one placement of `amount` Plague Counters on `card`, multiplied by the card's multiplier.
/// Returns how many went on — 0 when the card is not a permanent on the field or the amount is not
/// positive, in which case nothing changes and nothing is reported.
pub fn place_plague_on(sink: &mut EngineSink<'_>, card: &CardInstance, amount: i32) -> i32 {
    let base = amount;
    if base <= 0 || !carries_plague(sink.state, card) {
        return 0;
    }
    let placed = base * plague_multiplier_of(sink.state, card);
    let live_plague =
        crate::state::find_instance(sink.state, &card.id).map_or_else(|| plague_on(card), plague_on);
    let value = live_plague + placed;
    if let Some(live) = find_instance_mut(sink.state, &card.id) {
        live.counters.plague = Some(value);
    }
    sink.events.push(GameEvent::CounterChanged {
        instance_id: card.id.clone(),
        counter: CounterKind::Plague,
        value,
        placed: Some(placed),
    });
    placed
}

/// Take up to `amount` Plague Counters off a card (Classic #78's "remove a Plague Counter", Classic #74's
/// tokens spent as mana). Returns how many came off; the count floors at 0, and a removal that takes
/// nothing reports nothing.
pub fn remove_plague(sink: &mut EngineSink<'_>, card: &CardInstance, amount: i32) -> i32 {
    let had = crate::state::find_instance(sink.state, &card.id).map_or_else(|| plague_on(card), plague_on);
    let removed = had.min(amount.max(0));
    if removed == 0 {
        return 0;
    }
    let value = had - removed;
    if let Some(live) = find_instance_mut(sink.state, &card.id) {
        live.counters.plague = if value == 0 { None } else { Some(value) };
    }
    sink.events.push(GameEvent::CounterChanged {
        instance_id: card.id.clone(),
        counter: CounterKind::Plague,
        value,
        placed: None,
    });
    removed
}
