//! Cost changes on one card instance: the two inputs R65 reads before any player discount, combined
//! by `effectiveCost` alone (§6.3 Cost). Both persist through a hand, a library and leaving the field
//! (R78) and go as the card reaches a graveyard or an exile pile (R766). R1848006 adds Inflation
//! (Stinkyscape #153 Stimmycheck), a chance roll per card that raises its `costMod` (`inflate`).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::{INFLATION_CHANCE, INFLATION_CHANCE_STEP, INFLATION_COST, INFLATION_ROLL_SIDES};
use crate::damage::DamageTarget;
use crate::effects::targets::{PlayerSpec, TargetSpec, player_of, resolve_target};
use crate::mana::effective_cost;
use crate::script::{Effect, EffectContext};
use crate::state::{CardInstance, find_instance, find_instance_mut};
use crate::wire::{GameEvent, PLAYER_IDS, ZoneName};

/// A cost change names a card instance in any zone, never a hero.
fn card_of(ctx: &EffectContext<'_>, spec: &TargetSpec) -> Option<CardInstance> {
    match resolve_target(ctx, spec) {
        Some(DamageTarget::Unit { instance }) => Some(instance),
        _ => None,
    }
}

/// R4: a price a card is given as it returns to a hand (#31, #37r, #72, #72r) is its price in that
/// hand. A full hand burns the card (§2.4) and R78 would carry the change on, so `inHandOnly` prices
/// only a card in a hand when it applies.
fn priced(card: &CardInstance, in_hand_only: Option<bool>) -> bool {
    in_hand_only != Some(true) || card.zone.z() == ZoneName::Hand
}

/// R65: the event reports `effectiveCost` now. R177: a change to a library card is one nobody could
/// read (§3), so the event is `hiddenFrom` both for good, or a library-wide change's (#95) events, in
/// library order, would give away a card's place once it reads openly.
fn emit_cost(ctx: &mut EffectContext<'_>, card: &CardInstance) {
    let cost = effective_cost(ctx.sink.state, card, Default::default());
    let hidden_from = if card.zone.z() == ZoneName::Library {
        Some(PLAYER_IDS.to_vec())
    } else {
        None
    };
    ctx.sink.events.push(GameEvent::CostChanged {
        instance_id: card.id.clone(),
        cost,
        hidden_from,
    });
}

/// The card as it stands in the state after a write, or the copy when it is in no pile.
fn as_it_stands(ctx: &EffectContext<'_>, card: CardInstance) -> CardInstance {
    find_instance(ctx.sink.state, &card.id).cloned().unwrap_or(card)
}

/// `setCostMod`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SetCostModArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    pub amount: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_hand_only: Option<bool>,
    /// R429, R766: this is the price the card's own end-of-turn return gives it (#31 KY's Math
    /// Equation's climb), noted on it beside its `costMod` (`resolve::RETURN_PRICE_KEY`), so its next
    /// return gives back this much and no other change to its price.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_price: Option<bool>,
}

/// Add to this instance's `costMod` (#7 Jewelosco Scarab, #31 KY's Math Equation, #37r Gravedigger),
/// for good and between zones (R78).
pub fn set_cost_mod(args: SetCostModArgs) -> Effect {
    Effect::new("setCostMod", move |ctx| {
        let spec = args.target.clone().unwrap_or(TargetSpec::SelfCard);
        let Some(mut card) = card_of(ctx, &spec) else {
            return;
        };
        if !priced(&card, args.in_hand_only) {
            return;
        }
        let amount = args.amount;
        if amount == 0 {
            return;
        }
        match find_instance_mut(ctx.sink.state, &card.id) {
            Some(live) => {
                live.cost_mod += amount;
                if args.return_price == Some(true) {
                    crate::resolve::add_return_price(live, amount);
                }
            }
            None => card.cost_mod += amount,
        }
        let card = as_it_stands(ctx, card);
        emit_cost(ctx, &card);
    })
}

/// `setCostOverride`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SetCostOverrideArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    pub cost: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_hand_only: Option<bool>,
}

/// Set this instance's `costOverride` (#54 Straaza, #41r Sheepish, Craft a Card, R77): R65 starts
/// from it in place of the printed cost, then still adds `costMod` and the player's discounts.
pub fn set_cost_override(args: SetCostOverrideArgs) -> Effect {
    Effect::new("setCostOverride", move |ctx| {
        let spec = args.target.clone().unwrap_or(TargetSpec::SelfCard);
        let Some(mut card) = card_of(ctx, &spec) else {
            return;
        };
        if !priced(&card, args.in_hand_only) {
            return;
        }
        let cost = args.cost.max(0);
        match find_instance_mut(ctx.sink.state, &card.id) {
            Some(live) => live.cost_override = Some(cost),
            None => card.cost_override = Some(cost),
        }
        let card = as_it_stands(ctx, card);
        emit_cost(ctx, &card);
    })
}

/// R1848006: the `memory` key of a card's own Inflation chance, in percent (a Stimmycheck's copy).
pub const INFLATION_CHANCE_KEY: &str = "inflationChance";

/// R1848006: a card's Inflation chance: the one it carries, else `printed`.
pub fn inflation_chance_of(card: Option<&CardInstance>, printed: i32) -> i32 {
    card.and_then(|card| card.memory.get(INFLATION_CHANCE_KEY))
        .and_then(Value::as_i64)
        .map_or(printed, |carried| carried as i32)
        .clamp(0, INFLATION_ROLL_SIDES)
}

/// R1848006: the chance a Stimmycheck's copy carries, never past a certain hit.
pub fn next_inflation_chance(chance: i32) -> i32 {
    chance
        .saturating_add(INFLATION_CHANCE_STEP)
        .clamp(0, INFLATION_ROLL_SIDES)
}

/// `inflate`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct InflateArgs {
    /// The printed chance, in percent. Absent: `INFLATION_CHANCE` (Stimmycheck's 15). A card that
    /// carries its own chance (`INFLATION_CHANCE_KEY`) rolls at that instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chance: Option<i32>,
    /// Whose hand and deck, relative to the controller. Default "self".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// R1848006 (Stinkyscape #153 Stimmycheck): one roll per card, hand then deck, from the match rng; a
/// hit adds `INFLATION_COST` to its `costMod` (R766). An X-cost card, which no price change reaches
/// (R65), takes no roll. Luck-based (§6.1): Lucky X and Luck reroll and keep a miss on the
/// controller's own cards, a hit on the opponent's. A hidden scope, so nothing is announced (R440).
pub fn inflate(args: InflateArgs) -> Effect {
    Effect::new("inflate", move |ctx| {
        let chance = inflation_chance_of(ctx.self_.as_ref(), args.chance.unwrap_or(INFLATION_CHANCE));
        if chance <= 0 {
            return;
        }
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let own = player == ctx.controller;
        let lucky = ctx
            .self_
            .as_ref()
            .map_or(0, |card| crate::query::lucky_on(ctx.sink.state, card))
            + crate::query::luck_of(ctx.sink.state, ctx.controller);
        let side = &ctx.sink.state.players[player];
        let rolled: Vec<String> = side
            .hand
            .iter()
            .chain(side.library.iter())
            .filter(|card| !crate::mana::is_x_cost(ctx.sink.state, card))
            .map(|card| card.id.clone())
            .collect();
        for id in rolled {
            let hit = chance >= INFLATION_ROLL_SIDES
                || ctx.sink.rng.lucky(
                    lucky,
                    |rng| rng.int(INFLATION_ROLL_SIDES) < chance,
                    |kept, next| if own { kept && next } else { kept || next },
                );
            if hit && let Some(card) = find_instance_mut(ctx.sink.state, &id) {
                card.cost_mod += INFLATION_COST;
            }
        }
    })
}
