//! R667: the warning a hand card carries when playing it now would only get it countered (Classic #87
//! Plague Chalice). `view_for` sets `counteredOnPlay` from this on the viewer's own hand cards, and the
//! client draws it and decides nothing (CLAUDE.md rule 7).
//!
//! It asks the cards that would answer the play's announce — the holders `triggers::dispatch_event`
//! offers a `cardAnnounced` to, acting on the field with a trigger registered for it — through their
//! `wouldCounter` hook, the predicate their own counter trigger asks, so the warning and the counter
//! are one rule. Only a counter the viewer may read counts: a face-down card's would say what it is
//! (R33, R97). A card is flagged only when every price it could be played at now (each X and embiggen
//! choice, R65) would be countered, since a price that escapes is a play worth making; whether the
//! mana is there to pay it does not matter, as the glow it rides beside does not wait for mana either.
//!
//! Port of `packages/engine/src/counterWarning.ts` (part 5).

use indexmap::IndexSet;

use crate::play_choices::offered_play_costs;
use crate::preview::backrow_is_public;
use crate::script::WouldCounterArgs;
use crate::state::{CardInstance, GameState};
use crate::triggers::{TriggerHolder, TriggerZone, field_holders_where, triggers_on_event};
use crate::wire::{GameEventType, PlayerId};

/// The cards on the field whose counter trigger would answer a play's announce, as `player` may read them.
///
/// Only a field holder can pass, and only one whose script has `wouldCounter`, so only those are built
/// (`field_holders_where`): a view asks this for every hand card it shows.
fn readable_counters(state: &GameState, player: PlayerId) -> Vec<TriggerHolder> {
    field_holders_where(state, |card| {
        crate::scripts::script_of(state, card).would_counter.is_some()
    })
    .into_iter()
    .filter(|holder| {
        holder.script.would_counter.is_some()
            && !holder.is_trap
            && (holder.zone == TriggerZone::Field
                || (holder.zone == TriggerZone::Backrow && backrow_is_public(state, &holder.card, player)))
            && !triggers_on_event(holder, GameEventType::CardAnnounced).is_empty()
    })
    .collect()
}

/// R667: the ids of `player`'s hand cards that every price they could be played at now would see
/// countered, by a card on the field `player` may read. Empty when no such card is on the field.
pub fn countered_hand_cards(state: &GameState, player: PlayerId) -> IndexSet<String> {
    let counters = readable_counters(state, player);
    if counters.is_empty() {
        return IndexSet::new();
    }
    let countered = |card: &CardInstance| -> bool {
        let costs = offered_play_costs(state, player, card);
        !costs.is_empty()
            && costs.iter().all(|&cost_paid| {
                counters.iter().any(|holder| {
                    holder.script.would_counter.as_ref().is_some_and(|would_counter| {
                        would_counter(WouldCounterArgs {
                            state,
                            self_: &holder.card,
                            controller: holder.controller,
                            player,
                            cost_paid,
                        })
                    })
                })
            })
    };
    state.players[player]
        .hand
        .iter()
        .filter(|card| countered(card))
        .map(|card| card.id.clone())
        .collect()
}
