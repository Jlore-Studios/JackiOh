//! Trade offers (issue #1845, Stinkyscape #77 Yuma; R1845002): "Trade offer your opponent at the start
//! of their turn. You give them any card in your hand and they have to Discover a card in their own to
//! give to you."
//!
//! A delayed effect (`delay`, R62) "in 1 turn" of the opponent's own (`TRADE_OFFER_TURNS`, R1181000),
//! badged on the offerer's side (R169), that comes whatever becomes of its card (R76), its steps under
//! `TRADE_OFFER_HOOK` (answered as `CHAOS_ETERNAL_HOOK` is, R1241). The offerer's `hand` prompt opens on
//! the other player's turn (R1279000), a card changes hands as C #9's does, a full hand burning it (R12,
//! R317), and the opponent's Discover answer continues the offerer's sequence (B5 E18).
//!
//! R1877001: the offer carries its card's Lucky as made (`lucky_on`, R1438); with Lucky X the Discover
//! draws 1 + X times (R65's cost picks the set), once when the hand holds no more than the options.

use indexmap::IndexMap;

use crate::catalog::def_of;
use crate::config::{DISCOVER_OPTIONS, TRADE_OFFER_TURNS};
use crate::mana::effective_cost;
use crate::ownership::take_into_hand;
use crate::prompts::{OpenPromptArgs, open_prompt, resume_self};
use crate::query::lucky_on;
use crate::script::{Effect, EffectContext, Hook, hook};
use crate::state::{CardInstance, GameState, PromptOption, Resume, find_instance};
use crate::wire::{Phase, PlayerId, PromptKind, Selection, Zone, opponent_of};

use super::delay::{DelayArgs, DelayAt, DelayPlayer, delay};

/// R1845002: the hook a trade offer's steps are stored under, which the engine resolves itself.
pub const TRADE_OFFER_HOOK: &str = "@tradeOffer";
/// The delayed step: the offerer's pick opens.
const OFFER_STEP: &str = "offer";
/// The offerer's answer: the card goes over, and the opponent's Discover opens.
const GIVE_STEP: &str = "give";
/// The opponent's answer: their card comes back.
const TAKE_STEP: &str = "take";
/// R1877001: the data key the offer's Lucky rides under, from the card that made it to the Discover.
pub const TRADE_OFFER_LUCKY_KEY: &str = "tradeLucky";
/// R169: the countdown badge's caption on the offerer's side while the offer waits.
pub const TRADE_OFFER_LABEL: &str = "Trade offer";
/// The offerer's prompt.
pub const TRADE_OFFER_GIVE_PROMPT: &str = "Trade offer: give your opponent a card from your hand";
/// The opponent's prompt.
pub const TRADE_OFFER_TAKE_PROMPT: &str = "Trade offer: Discover a card in your hand to give your opponent";

/// R1845002: "trade offer your opponent at the start of their turn", made for this card's controller,
/// carrying the Lucky the card has now (R1877001).
pub fn trade_offer() -> Effect {
    Effect::new("tradeOffer", |ctx| {
        let lucky = ctx.self_.as_ref().map_or(0, |card| lucky_on(ctx.state, card));
        let data = (lucky > 0).then(|| IndexMap::from([(TRADE_OFFER_LUCKY_KEY.to_string(), lucky.into())]));
        let offer = delay(DelayArgs {
            at: DelayAt {
                phase: Phase::Start,
                player: DelayPlayer::Enemy,
            },
            step: OFFER_STEP.into(),
            hook: Some(TRADE_OFFER_HOOK.into()),
            data,
            watch: None,
            next: None,
            mark: None,
            hand_watch: None,
            turns: Some(TRADE_OFFER_TURNS),
            label: Some(TRADE_OFFER_LABEL.into()),
        });
        (offer.apply)(ctx);
    })
}

/// R1845002: the step a trade offer's continuation names, or `None` for a step it never stores.
pub fn trade_offer_step(step: &str) -> Option<Hook> {
    match step {
        OFFER_STEP => Some(hook(|_ctx| vec![Effect::new("tradeOffer:offer", offer)])),
        GIVE_STEP => Some(hook(|_ctx| vec![Effect::new("tradeOffer:give", give)])),
        TAKE_STEP => Some(hook(|_ctx| vec![Effect::new("tradeOffer:take", take)])),
        _ => None,
    }
}

/// The continuation of this run at `step`, under the offer's own hook.
fn step_resume(ctx: &EffectContext<'_>, step: &str) -> Resume {
    Resume {
        hook: TRADE_OFFER_HOOK.to_string(),
        ..resume_self(ctx, step, IndexMap::new())
    }
}

/// A hand card as an option, as `choose_from_hand` offers one.
fn hand_option(state: &GameState, card: &CardInstance) -> PromptOption {
    PromptOption {
        key: format!("instance:{}", card.id),
        label: def_of(Some(state), &card.def_id).name.clone(),
        selection: Selection::Instance {
            instance_id: card.id.clone(),
        },
        cost: None,
        radiant: if card.radiant { Some(true) } else { None },
    }
}

/// The card the answer picked.
fn chosen_id(ctx: &EffectContext<'_>) -> Option<String> {
    ctx.targets.iter().find_map(|pick| match pick {
        Selection::Instance { instance_id } => Some(instance_id.clone()),
        _ => None,
    })
}

/// The card `id` names while it is in `player`'s hand.
fn in_hand_of(state: &GameState, id: &str, player: PlayerId) -> Option<CardInstance> {
    find_instance(state, id)
        .filter(|card| card.zone == Zone::Hand { player })
        .cloned()
}

/// Step 1: the offerer picks the card to give, while each hand holds one; otherwise nothing happens.
fn offer(ctx: &mut EffectContext<'_>) {
    let offerer = ctx.controller;
    let taker = opponent_of(offerer);
    if ctx.state.players[offerer].hand.is_empty() || ctx.state.players[taker].hand.is_empty() {
        return;
    }
    let options: Vec<PromptOption> = ctx.state.players[offerer]
        .hand
        .iter()
        .map(|card| hand_option(ctx.state, card))
        .collect();
    let resume = step_resume(ctx, GIVE_STEP);
    open_prompt(
        ctx,
        OpenPromptArgs {
            player: offerer,
            kind: PromptKind::Hand,
            aim: None,
            prompt: TRADE_OFFER_GIVE_PROMPT.to_string(),
            options,
            min: Some(1),
            max: Some(1),
            budget: None,
            owner: None,
            resume,
        },
    );
}

/// Step 2: the picked card goes over, then the opponent Discovers one of their other hand cards.
fn give(ctx: &mut EffectContext<'_>) {
    let offerer = ctx.controller;
    let taker = opponent_of(offerer);
    let Some(given) = chosen_id(ctx).and_then(|id| in_hand_of(ctx.state, &id, offerer)) else {
        return;
    };
    take_into_hand(ctx, &given, taker);
    let pool: Vec<CardInstance> = ctx.state.players[taker]
        .hand
        .iter()
        .filter(|card| card.id != given.id)
        .cloned()
        .collect();
    if pool.is_empty() {
        return;
    }
    let shown = DISCOVER_OPTIONS.max(0) as usize;
    let lucky = ctx
        .data
        .get(TRADE_OFFER_LUCKY_KEY)
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0) as i32;
    let lucky = if pool.len() > shown { lucky } else { 0 };
    let costs: Vec<i32> = pool
        .iter()
        .map(|card| effective_cost(ctx.state, card, Default::default()))
        .collect();
    let indices: Vec<usize> = (0..pool.len()).collect();
    // R50, R60: as `offer_instances` draws a Discover's options out of real instances; R1877001: Lucky
    // draws them again and keeps the set whose cheapest card costs most, the earlier on a tie.
    let cheapest = |set: &[usize]| set.iter().map(|&i| costs[i]).min().unwrap_or(0);
    let offered = ctx.rng.lucky(
        lucky,
        |rng| {
            rng.shuffle(&indices)
                .into_iter()
                .take(shown)
                .collect::<Vec<usize>>()
        },
        |best, next| {
            if cheapest(&next) > cheapest(&best) {
                next
            } else {
                best
            }
        },
    );
    let options: Vec<PromptOption> = offered
        .iter()
        .map(|&i| hand_option(ctx.state, &pool[i]))
        .collect();
    let resume = step_resume(ctx, TAKE_STEP);
    open_prompt(
        ctx,
        OpenPromptArgs {
            player: taker,
            kind: PromptKind::Discover,
            aim: None,
            prompt: TRADE_OFFER_TAKE_PROMPT.to_string(),
            options,
            min: Some(1),
            max: Some(1),
            budget: None,
            // B5 E18: the answer continues the offerer's sequence, so the card comes to their hand.
            owner: Some(offerer),
            resume,
        },
    );
}

/// Step 3: the Discovered card goes to the offerer's hand.
fn take(ctx: &mut EffectContext<'_>) {
    let offerer = ctx.controller;
    let taker = opponent_of(offerer);
    let Some(taken) = chosen_id(ctx).and_then(|id| in_hand_of(ctx.state, &id, taker)) else {
        return;
    };
    take_into_hand(ctx, &taken, offerer);
}
