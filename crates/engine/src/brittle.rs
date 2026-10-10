//! Brittle X (docs/classic-sets.md B3.3, R385): the count on a card instance, its start-of-turn tick
//! and its crumbling. `turn.rs` runs `brittle_tick` as a stage of the start of a turn, right after the
//! mana refresh (§2.2, R62), and settles after it: the tick only moves counts, marks and cards and
//! opens no prompt, so everything it causes resolves in that settle and parks on `state.work` there.
//!
//! A count ticks on the field only (R638); it starts its turn cycle when the card enters the field
//! (`brittle_count::start_brittle_on_field`).
//!
//! Hidden information (R440): a count that ticks on a card the other player may not read — a face-down
//! trap — ticks silently, since a `counterChanged` there would tell them it is Brittle. Its owner reads
//! the count on the card (`CardView.brittle`). A crumble is never silent: the card goes to a graveyard.

use serde_json::json;

use crate::config::{BACKROW_ZONES, BRITTLE_FIRST_TICK_TURNS, BRITTLE_TICK, UNIT_ZONES};
use crate::prelude::json_as;
use crate::preview::is_face_down;
use crate::script::EngineSink;
use crate::state::{BrittleCounter, CardInstance, GameState, find_instance, find_instance_mut};
use crate::wire::{CounterKind, GameEvent, PlayerId, ZoneName};

pub use crate::brittle_count::{
    active_brittle_count, gain_brittle_count, give_brittle_count, printed_brittle_of, start_brittle_on_field,
};

/// B3.3 rule 2, R638: every card whose count this player's start of turn ticks, in R68's order — the
/// cards they control on the field, units by lane (the top of each pile only, since a card dormant
/// under a Stack is not on the field, R13), then the backrow by lane. Read once, as copies; the tick
/// writes through ids.
fn ticked_cards(state: &GameState, player: PlayerId) -> Vec<CardInstance> {
    let side = &state.players[player];
    let mut cards: Vec<CardInstance> = Vec::new();
    // `zones::active_units_of`: each unit zone's top card in lane order, then the Units its carriers
    // hold, in backrow lane order (R446).
    for lane in 0..UNIT_ZONES.max(0) as usize {
        if let Some(top) = side
            .units
            .get(lane)
            .and_then(|pile| pile.as_ref())
            .and_then(|pile| pile.first())
        {
            cards.push(top.clone());
        }
    }
    for held in side.carried.iter().flatten().flatten() {
        cards.push(held.clone());
    }
    // `zones::slots_of(player, "backrow")` + `zones::card_at`: the acting card of each backrow zone.
    for lane in 0..BACKROW_ZONES.max(0) as usize {
        if let Some(card) = side.backrow.get(lane).and_then(|card| card.as_ref()) {
            cards.push(card.clone());
        }
    }
    cards
}

/// B3.3 rule 2: a count started on player-turn `since` has had its full turn cycle once the turn is
/// `since + BRITTLE_FIRST_TICK_TURNS` or later — the rest of the turn it started on and a whole turn of
/// the other player's (B9 #41's t + 2). After that it ticks at each start of its controller's turn.
pub fn brittle_due(state: &GameState, card: &CardInstance) -> bool {
    match &card.brittle {
        Some(brittle) => state.turn >= brittle.since + BRITTLE_FIRST_TICK_TURNS,
        None => false,
    }
}

/// B3.3 rule 3: a count at 0 crumbles its card. That is an ordinary destroy (§6.3), so the next state
/// check collects it, Indestructible ignores it (R46) and the count stays at 0, checked again at every
/// tick. R215 resets the card in its graveyard, which spends the count (R441).
fn crumble(sink: &mut EngineSink<'_>, card: &CardInstance) {
    sink.events.push(GameEvent::Crumbled {
        instance_id: card.id.clone(),
        def_id: card.def_id.clone(),
        owner: card.owner,
        zone: ZoneName::Field,
    });
    let options = crate::resolve::HookOptions {
        controller: Some(card.controller),
        ..Default::default()
    };
    let mut ctx = crate::resolve::make_context(sink, None, options);
    let effect = crate::effects::destroy::destroy(json_as(json!({
        "target": { "of": "instance", "instanceId": card.id }
    })));
    (effect.apply)(&mut ctx);
}

/// B3.3 rule 2, R638: at the start of `player`'s turn, every Brittle count of theirs on the field that
/// has had a full turn cycle drops by 1, and a count that reaches 0 crumbles its card (rule 3). The
/// cards are read once, before any count moves, so a card the tick moves is not met twice.
pub fn brittle_tick(sink: &mut EngineSink<'_>, player: PlayerId) {
    for listed in ticked_cards(sink.state, player) {
        // The card as it stands now: an earlier card's tick may have moved things since the list was read.
        let card = find_instance(sink.state, &listed.id).cloned().unwrap_or(listed);
        let count = active_brittle_count(&card);
        let (Some(count), Some(brittle)) = (count, card.brittle) else {
            continue;
        };
        if !brittle_due(sink.state, &card) {
            continue;
        }
        if count > 0 {
            let next = (count - BRITTLE_TICK).max(0);
            if let Some(live) = find_instance_mut(sink.state, &card.id) {
                live.brittle = Some(BrittleCounter {
                    count: next,
                    ..brittle
                });
            }
            if !is_face_down(sink.state, &card) {
                sink.events.push(GameEvent::CounterChanged {
                    instance_id: card.id.clone(),
                    counter: CounterKind::Brittle,
                    value: next,
                    placed: None,
                });
            }
            if next > 0 {
                continue;
            }
        }
        crumble(sink, &card);
    }
}
