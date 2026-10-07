//! Cards between the players' piles (docs/classic-sets.md B5 E16, with E2's hand and deck half):
//! cards handed from one hand to the other (Classic #9 Income Tax), a card taken out of the other
//! player's deck (Classic+ #12.3 Fluffy Grip) and a draw from the other player's deck (Classic #58
//! Common Resources). Every one of them is `ownership.ts`'s change of owner: the card becomes the
//! taker's for good, its later piles are the taker's, and the taker's hand cap burns what does not fit
//! into the taker's graveyard (§2.4). Swapping whole decks is R73's library swap (`swap.ts`), not this.
//!
//! Port of `packages/engine/src/effects/give.ts`. TS wrote the riders through the live instance; Rust
//! writes them through `state::find_instance_mut` by the card's id.

use serde::{Deserialize, Serialize};

use crate::effects::choose::{LibraryFilter, matches_library_filter};
use crate::effects::targets::{PlayerSpec, player_of};
use crate::ownership::{LibraryEnd, TakenTo, draw_from_library_of, take_into_hand};
use crate::script::{Effect, EffectContext};
use crate::state::{CardInstance, find_instance_mut};
use crate::wire::{PlayerId, Selection, opponent_of};

/// What a taken card is given as it lands in the taker's hand: a cost (`costOverride`, Classic+ #12.3's
/// "it costs (0)"), a discount (`costMod`, Classic #9 Radiant's "cost (1) less") and the Radiant face
/// (#12.3 Radiant). As `addToHand`'s riders are: the face goes on first, since the card is Radiant
/// wherever it ends up (R74), and a price goes on only once the card is in the hand — one the hand cap
/// burned is an ordinary graveyard card (R4).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TakenRiders {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_override: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_mod: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
}

fn take_with(ctx: &mut EffectContext<'_>, card: &CardInstance, taker: PlayerId, riders: &TakenRiders) {
    let mut card = card.clone();
    if riders.radiant == Some(true) {
        if let Some(live) = find_instance_mut(ctx.sink.state, &card.id) {
            live.radiant = true;
        }
        card.radiant = true;
    }
    // `ownership.takeIntoHand` answers where the card landed: in the hand, burned, or nothing at all.
    if !matches!(take_into_hand(ctx, &card, taker), Some(TakenTo::Hand)) {
        return;
    }
    if let Some(live) = find_instance_mut(ctx.sink.state, &card.id) {
        if let Some(cost) = riders.cost_override {
            live.cost_override = Some(cost.max(0));
        }
        if let Some(amount) = riders.cost_mod {
            live.cost_mod += amount;
        }
    }
}

/// The instance ids a step's answered prompt picked (`ctx.targets`), in offered order.
fn chosen_ids(ctx: &EffectContext<'_>) -> Vec<String> {
    ctx.targets
        .iter()
        .filter_map(|selection| match selection {
            Selection::Instance { instance_id } => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

/// `giveFromHand`'s `cards`: which cards of the hand go.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum GiveFromHandCards {
    Chosen,
    Unchosen,
    All,
    Random,
}

/// `giveFromHand`'s argument: `{ from?; cards?; count? } & TakenRiders`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct GiveFromHandArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cards: Option<GiveFromHandCards>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
    #[serde(flatten)]
    pub riders: TakenRiders,
}

/// B5 E16: cards out of `from`'s hand into the other player's hand, as theirs — the default `from` is
/// the enemy, so the taker is this card's controller; `from: "self"` gives your own cards away. `cards` names
/// which: `"chosen"` the ones the step's prompt picked, `"unchosen"` every other card of that hand
/// (Classic #9: "they keep one card of their choice and give you the rest" — their own hand pick, then
/// this), `"all"` the whole hand, `"random"` `count` different cards at random (R60). Hand order,
/// snapshotted first, so the cap burns the last of them.
pub fn give_from_hand(args: GiveFromHandArgs) -> Effect {
    Effect::new("giveFromHand", move |ctx| {
        let from = player_of(ctx, args.from.unwrap_or(PlayerSpec::Enemy));
        let hand: Vec<CardInstance> = ctx.sink.state.players[from].hand.clone();
        let chosen = chosen_ids(ctx);
        let which = args.cards.unwrap_or(GiveFromHandCards::Chosen);
        let taken: Vec<CardInstance> = match which {
            GiveFromHandCards::All => hand,
            GiveFromHandCards::Chosen => hand
                .into_iter()
                .filter(|card| chosen.contains(&card.id))
                .collect(),
            GiveFromHandCards::Unchosen => hand
                .into_iter()
                .filter(|card| !chosen.contains(&card.id))
                .collect(),
            GiveFromHandCards::Random => {
                let count = args.count.unwrap_or(1).max(0) as usize;
                ctx.sink.rng.shuffle(&hand).into_iter().take(count).collect()
            }
        };
        for card in &taken {
            take_with(ctx, card, opponent_of(from), &args.riders);
        }
    })
}

/// `takeFromLibrary`'s `pick`: which matching cards of the library go.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum TakeFromLibraryPick {
    Random,
    Top,
    Bottom,
    Chosen,
}

/// `takeFromLibrary`'s argument: `{ from?; pick?; count?; filter? } & TakenRiders`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TakeFromLibraryArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pick: Option<TakeFromLibraryPick>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<LibraryFilter>,
    #[serde(flatten)]
    pub riders: TakenRiders,
}

/// B5 E2, E16: cards out of `from`'s library into the other player's hand, as theirs (Classic+
/// #12.3 Fluffy Grip: "steal a random Unit from your opponent's deck"). `pick` is `"random"` — `count`
/// different matching cards (R60) — `"top"` or `"bottom"`, or `"chosen"`, the cards the step's
/// prompt picked out of it. `filter` narrows the pool as a library reveal does (a unit-token card
/// never leaves a library for a hand this way, R218). No match takes nothing. The library's owner
/// learns only that cards left it (R466).
pub fn take_from_library(args: TakeFromLibraryArgs) -> Effect {
    Effect::new("takeFromLibrary", move |ctx| {
        let from = player_of(ctx, args.from.unwrap_or(PlayerSpec::Enemy));
        let filter = args.filter.clone().unwrap_or_default();
        let pool: Vec<CardInstance> = {
            let state = &*ctx.sink.state;
            state.players[from]
                .library
                .iter()
                .filter(|&card| matches_library_filter(state, card, &filter))
                .cloned()
                .collect()
        };
        let count = args.count.unwrap_or(1).max(0) as usize;
        let pick = args.pick.unwrap_or(TakeFromLibraryPick::Random);
        let chosen = chosen_ids(ctx);
        let taken: Vec<CardInstance> = match pick {
            TakeFromLibraryPick::Chosen => pool
                .into_iter()
                .filter(|card| chosen.contains(&card.id))
                .collect(),
            TakeFromLibraryPick::Top => pool.into_iter().take(count).collect(),
            TakeFromLibraryPick::Bottom => {
                let start = pool.len().saturating_sub(count);
                pool[start..].iter().rev().cloned().collect()
            }
            TakeFromLibraryPick::Random => ctx.sink.rng.shuffle(&pool).into_iter().take(count).collect(),
        };
        for card in &taken {
            take_with(ctx, card, opponent_of(from), &args.riders);
        }
    })
}

/// `drawFromOpponent`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DrawFromOpponentArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<LibraryEnd>,
}

/// B5 E16: one draw of this card's controller's, taken from the other player's library — its bottom
/// card by default (Classic #58 Common Resources). It is a draw of the controller's
/// (`ownership.drawFromLibraryOf`): their hand cap, a cast on draw for them, the draw counters. An
/// empty library gives nothing and deals no fatigue. One draw per effect, so a card that draws two
/// returns two, and a cast on draw that asks something pauses the list between them (R113).
pub fn draw_from_opponent(args: DrawFromOpponentArgs) -> Effect {
    Effect::new("drawFromOpponent", move |ctx| {
        let drawer = ctx.controller;
        let from = player_of(ctx, PlayerSpec::Enemy);
        let end = args.end.unwrap_or(LibraryEnd::Bottom);
        draw_from_library_of(ctx, drawer, from, end);
    })
}
