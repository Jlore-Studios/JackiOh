//! Hidden Nerfs (issue #1845, Stinkyscape #123 Gokugains; R1845000, R1845001): Nerfs put on a card in
//! a hand or a deck that no seat reads, the card's own owner included, until the card is played.
//!
//! `degrade_secretly` adds to a card's `hiddenNerfs` count and reports nothing: no event, no view
//! field, and the AI's `redact` and the dry run's `concealed_copy` drop it (`state::forget_hidden_nerfs`),
//! so no seat's decision hangs on it (R1845000).
//!
//! `land_hidden_nerfs` is §10.5 step 3's: as the card is played or cast, after a replacement, Feng
//! Shui, Gifted Program and the Garden, its count is spent as that many §6.3 Nerfs from every row but
//! the cost's (`tune::degrade_sparing_cost`), reported as every Nerf is (R177, R440), so the card's
//! price is never touched and its Nerfs are seen as it is played (R1845001).

use indexmap::IndexSet;

use crate::layers::card_keywords;
use crate::resolve::{HookOptions, make_context};
use crate::script::{Effect, EffectContext, EngineSink};
use crate::state::{CardInstance, GameState, find_instance_mut};
use crate::wire::{KeywordKind, TargetAim, ZoneName, has_keyword};

use super::card_scope::{CardScopeOptions, cards_in_card_scope};
use super::tune::{TuneArgs, degrade_sparing_cost, named_card};

/// R1845000: a card a hidden Nerf can wait on — one in a hand or a deck, which no Immutable card is
/// (R23, R386).
fn can_hide(state: &GameState, card: &CardInstance) -> bool {
    matches!(card.zone.z(), ZoneName::Hand | ZoneName::Library)
        && !has_keyword(&card_keywords(state, card), KeywordKind::Immutable)
}

/// R60, R129, R242: `random` different ids of the pool, kept in the pool's order — all of them, with
/// nothing drawn, when the pool has no more.
fn picked(ctx: &mut EffectContext<'_>, pool: Vec<String>, random: Option<i32>) -> Vec<String> {
    let Some(random) = random else {
        return pool;
    };
    let count = random.max(0) as usize;
    if count == 0 {
        return Vec::new();
    }
    if pool.len() <= count {
        return pool;
    }
    let chosen: IndexSet<String> = ctx.rng.shuffle(&pool).into_iter().take(count).collect();
    pool.into_iter().filter(|id| chosen.contains(id)).collect()
}

/// R1845000: `times` hidden Nerfs (default 1) on each card `args` reaches that can take them. A random
/// pick draws among the cards that can take them, so it never lands on nothing while another could.
pub fn degrade_secretly(args: TuneArgs) -> Effect {
    Effect::new("degradeSecretly", move |ctx| {
        let times = args.times.unwrap_or(1).max(0);
        if times == 0 {
            return;
        }
        let pool: Vec<String> = match &args.scope {
            None => named_card(ctx, args.target.as_ref(), args.instance_id.as_deref())
                .filter(|card| can_hide(ctx.state, card))
                .map(|card| vec![card.id])
                .unwrap_or_default(),
            // MD-B1, R940: a Nerf is harmful, so an immune card in a tribal scope is passed by.
            Some(scope) => cards_in_card_scope(
                ctx,
                scope,
                Some(&CardScopeOptions {
                    whole_hidden_piles: None,
                    aim: Some(TargetAim::Harm),
                }),
            )
            .into_iter()
            .filter(|entry| entry.matches && can_hide(ctx.state, &entry.card))
            .map(|entry| entry.card.id)
            .collect(),
        };
        for id in picked(ctx, pool, args.random) {
            if let Some(card) = find_instance_mut(ctx.state, &id) {
                card.hidden_nerfs = Some(card.hidden_nerfs.unwrap_or(0).saturating_add(times));
            }
        }
    })
}

/// R1845001: the played card's hidden Nerfs landed, run as its controller's; none draws nothing.
pub fn land_hidden_nerfs(sink: &mut EngineSink<'_>, instance_id: &str) {
    let Some(live) = find_instance_mut(sink.state, instance_id) else {
        return;
    };
    let Some(count) = live.hidden_nerfs.take() else {
        return;
    };
    let card = live.clone();
    let mut ctx = make_context(
        sink,
        Some(&card),
        HookOptions {
            controller: Some(card.controller),
            ..HookOptions::default()
        },
    );
    for _ in 0..count.max(0) {
        degrade_sparing_cost(&mut ctx, &card);
    }
}
