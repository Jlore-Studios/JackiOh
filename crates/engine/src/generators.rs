//! R1956001: which card generated a Created card (R943), so the client can dress the Created card as
//! its owner's copy of the generator is dressed (R1655000's foil, finish and border).
//! `GameState.generators` maps an instance id to its root generator's catalog id, so a token's token
//! carries the first card. Presentation only (CLAUDE.md rule 7): no rule reads it, the hash leaves it
//! out (`replay::hash_state`) and only the Created card's owner is told it (`CardView.createdBy`).
//!
//! The effect verbs that mint a card for a card record it as they mint. A card a rule mints (The Coin,
//! setup, a Resource, a Starship), a fused card, a card given to the other player and a card whose
//! generator is no catalog card (it wears no per-card pick) record nothing.

use crate::script::EffectContext;
use crate::state::{CardInstance, GameState, find_instance};
use crate::wire::{GameEvent, PlayerId};

/// Record the card running `ctx`'s script as `card`'s generator (`record_generator_of`). A script
/// whose card has ceased to exist still names its definition (R127), which stands in for the card.
pub fn record_generator(ctx: &mut EffectContext<'_>, card: &CardInstance) {
    let (owner, root) = match (&ctx.self_, &ctx.def_id) {
        (Some(maker), _) => (maker.owner, root_of(ctx.state, maker)),
        (None, Some(def_id)) => (ctx.controller, def_id.clone()),
        (None, None) => return,
    };
    record(ctx.state, owner, root, card);
}

/// Record `maker` as `card`'s generator, or `maker`'s own generator when a card Created it too.
/// Nothing is recorded when `card` is not `maker`'s owner's (a card given to the other player wears
/// none of the generator's dress), or when the root is no catalog card.
pub fn record_generator_of(state: &mut GameState, maker: &CardInstance, card: &CardInstance) {
    let root = root_of(state, maker);
    record(state, maker.owner, root, card);
}

fn record(state: &mut GameState, owner: PlayerId, root: String, card: &CardInstance) {
    if owner != card.owner || crate::catalog::find_def(None, &root).is_none() {
        return;
    }
    state.generators.insert(card.id.clone(), root);
}

/// R2089000: the seat running `ctx` made `card_id`, which it has just put in a hand, a library or
/// another card's place; when that is another seat's, the last event naming the card (nothing has moved
/// it since) names the maker as its `creator`, so the game record credits the seat that made it (§9.11).
/// A card burned or dropped by a full zone is credited to the seat it went to. Engine bookkeeping: no
/// rule reads it and no view forwards it.
pub fn credit_creator(ctx: &mut EffectContext<'_>, card_id: &str) {
    let creator = ctx.controller;
    let controller = find_instance(ctx.sink.state, card_id).map(|card| card.controller);
    for event in ctx.sink.events.iter_mut().rev() {
        let (receiver, slot) = match event {
            GameEvent::AddedToHand {
                player,
                instance_id,
                creator: slot,
                ..
            }
            | GameEvent::ShuffledIn {
                player,
                instance_id,
                creator: slot,
                ..
            } if instance_id == card_id => (Some(*player), slot),
            GameEvent::Transformed {
                new_instance_id,
                creator: slot,
                ..
            } if new_instance_id == card_id => (controller, slot),
            _ => continue,
        };
        if receiver.is_some_and(|receiver| receiver != creator) {
            *slot = Some(creator);
        }
        return;
    }
}

/// R227: a Created card that takes a fresh id going face-down (set from hand, summoned or flickered
/// into a backrow) keeps its generator, as C+ #35's history keeps the card (R419).
pub fn rename_generator(state: &mut GameState, from: &str, to: &str) {
    if let Some(root) = state.generators.shift_remove(from) {
        state.generators.insert(to.to_string(), root);
    }
}

/// The generator a card's own generator recorded, or the card's own definition.
fn root_of(state: &GameState, maker: &CardInstance) -> String {
    state
        .generators
        .get(&maker.id)
        .cloned()
        .unwrap_or_else(|| maker.def_id.clone())
}

/// `card`'s generator as `viewer` is told it: only their own card's (§14.2: the client never infers
/// another's cosmetics), on a view `view_for` builds only for a card the viewer may read.
pub fn generator_shown_to(state: &GameState, card: &CardInstance, viewer: PlayerId) -> Option<String> {
    if card.owner != viewer {
        return None;
    }
    state.generators.get(&card.id).cloned()
}
