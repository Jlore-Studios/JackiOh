//! Casts from the deck (Stinkyscape+ E8, R3558007): a card whose `castFromDeckWhen` holds is cast
//! from its owner's library the moment its owner's condition holds at a settled state check —
//! "when your exile has 5 cards, cast this (wherever in your deck this is)". The cast is the
//! owner's own (R453): they make its choices. Like every standing rule it draws no rng, and the
//! per-action cap keeps a card that returns itself to its deck from casting for ever.

use crate::config::DECK_CAST_ACTION_CAP;
use crate::script::{EngineSink, HookArgs};
use crate::state::QueuedTrigger;
use crate::state::{CardInstance, active_first, find_instance, find_instance_mut};
use crate::triggers::{DECK_CAST, queue_deck_cast};
use crate::wire::ZoneName;

/// R3558007: whether `card` may be deck-cast now: its standing condition holds for its owner, and
/// the owner may cast at all (R452's random-cast chain).
fn deck_cast_ready(state: &crate::state::GameState, card: &CardInstance) -> bool {
    let script = crate::scripts::script_of(state, card);
    let Some(when) = script.cast_from_deck_when.as_ref() else {
        return false;
    };
    when(HookArgs {
        state,
        self_: card,
        radiant: card.radiant,
    }) && crate::random_cast::may_cast_now(state, card.owner)
}

/// R3558007: queue every card whose deck-cast condition holds, the active player's side first, each
/// library oldest first — at most `DECK_CAST_ACTION_CAP` an action. A card already queued waits its
/// turn; a card cast since queueing fizzles at its run (`run_deck_cast`).
pub fn queue_deck_casts(sink: &mut EngineSink<'_>) {
    let mut ready: Vec<CardInstance> = Vec::new();
    for player in active_first(sink.state) {
        for card in crate::triggers::library_in_creation_order(sink.state, player) {
            if !deck_cast_ready(sink.state, card) {
                continue;
            }
            if sink
                .state
                .trigger_queue
                .iter()
                .any(|entry| entry.hook == DECK_CAST && entry.instance_id == card.id)
            {
                continue;
            }
            if !ready.iter().any(|queued| queued.id == card.id) {
                ready.push(card.clone());
            }
        }
    }
    for card in &ready {
        if sink.frontier.get().deck_casts >= DECK_CAST_ACTION_CAP {
            return;
        }
        queue_deck_cast(sink, card);
        sink.frontier.get_mut().deck_casts += 1;
    }
}

/// R3558007: run one queued deck cast: the card is found again now, and cast only while it still
/// lies in its owner's library and may be cast. The owner answers for it (R453), as a cast from
/// the hand is theirs.
pub fn run_deck_cast(sink: &mut EngineSink<'_>, entry: &QueuedTrigger) {
    let Some(card) = find_instance(sink.state, &entry.instance_id).cloned() else {
        return;
    };
    if card.zone.z() != ZoneName::Library {
        return;
    }
    if !crate::random_cast::may_cast_now(sink.state, card.owner) {
        return;
    }
    if !deck_cast_ready(sink.state, &card) {
        return;
    }
    if let Some(live) = find_instance_mut(sink.state, &card.id) {
        live.controller = live.owner;
    }
    let Some(card) = find_instance(sink.state, &entry.instance_id).cloned() else {
        return;
    };
    crate::resolve::cast_card(sink, &card, crate::resolve::CastOptions::default());
}
