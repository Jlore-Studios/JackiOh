//! Cost changes on one card instance: the two inputs R65 reads before any player discount. Both
//! persist through a hand, a library and leaving the field (R78), and both go as the card reaches a
//! graveyard or an exile pile (R766), so a card discounted in hand costs its printed cost there.
//! The order the two combine is `effectiveCost`'s (§6.3 Cost, R65); nothing here recomputes it.
//!
//! R1848006 adds Inflation (Stinkyscape #153 Stimmycheck): a chance roll per card in a hand and a deck
//! that raises a card's `costMod` (`inflate`), the chance carried on the copy a Stimmycheck makes.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::{INFLATION_CHANCE, INFLATION_CHANCE_STEP, INFLATION_COST, INFLATION_ROLL_SIDES};
use crate::damage::DamageTarget;
use crate::effects::targets::{PlayerSpec, TargetSpec, player_of, resolve_target};
use crate::mana::effective_cost;
use crate::script::{Effect, EffectContext};
use crate::state::{CardInstance, find_instance, find_instance_mut};
use crate::wire::{GameEvent, PLAYER_IDS, ZoneName};

/// A cost change names a card instance in any zone, so a hero selection is not one of them.
fn card_of(ctx: &EffectContext<'_>, spec: &TargetSpec) -> Option<CardInstance> {
    match resolve_target(ctx, spec) {
        Some(DamageTarget::Unit { instance }) => Some(instance),
        _ => None,
    }
}

/// R4: a price a card is given as it returns to a hand (#31's +1, #37r's and #72's "1 less", #72r's
/// "0") is its price in that hand. A full hand burns the card (§2.4) and R78 would carry the change
/// into every later zone, so with `inHandOnly` the change lands only on a card that is in a hand when
/// it applies; written after the move, it is skipped for a burned card.
fn priced(card: &CardInstance, in_hand_only: Option<bool>) -> bool {
    in_hand_only != Some(true) || card.zone.z() == ZoneName::Hand
}

/// R65: the event reports what the card costs now, which is `effectiveCost` and nothing else. R177: a
/// change made to a card in a library is one nobody could read where it happened (§3), so the event
/// says it was made there (`hiddenFrom`) and a view keeps it unread for good: a library-wide change (#95)
/// emits one event per card in library order, so once a card reads openly its event would give away its
/// place in the batch.
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

/// Add to this instance's `costMod` (#7 Jewelosco Scarab's −1, #31 KY's Math Equation's +1 per
/// return, #37r Gravedigger). The change is permanent and travels with the card between zones (R78).
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

/// Set this instance's `costOverride`: #54 Straaza's "they cost 1", #41r Sheepish's 0-cost Lava
/// Golem, Craft a Card's 0 (R77). R65 starts the calculation from the override in place of the
/// printed cost, then still adds `costMod` and the player's discounts.
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

/// R1848006: where a card carries the Inflation chance it rolls at (its `memory`), in percent — the
/// copy a Stimmycheck adds to its hand carries its grown chance there (`add_to_hand`'s `memory`).
pub const INFLATION_CHANCE_KEY: &str = "inflationChance";

/// R1848006: the chance, in percent, a card's Inflation rolls at — the one it carries
/// (`INFLATION_CHANCE_KEY`), else `printed` — held to 0..=`INFLATION_ROLL_SIDES`.
pub fn inflation_chance_of(card: Option<&CardInstance>, printed: i32) -> i32 {
    card.and_then(|card| card.memory.get(INFLATION_CHANCE_KEY))
        .and_then(Value::as_i64)
        .map_or(printed, |carried| carried as i32)
        .clamp(0, INFLATION_ROLL_SIDES)
}

/// R1848006: the chance the copy a Stimmycheck makes carries ("+10% chance at Inflation"), never past
/// a certain hit.
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

/// R1848006 (Stinkyscape #153 Stimmycheck): "cards in your hand and deck have a 15% chance for
/// Inflation to hit, which causes them to cost (1) more". One roll per card, from the match rng, in
/// the hand's order and then the deck's from the top; a card it hits gains `INFLATION_COST` on its
/// `costMod`, which rides it through the hand and the deck as any price change does and R766 takes
/// off in a graveyard or an exile pile. An X-cost card, whose price no change reaches (R65), is passed
/// by without a roll. The roll is luck-based (§6.1): Lucky X on the running card and its controller's
/// Luck roll it again and keep the better, which is a miss on the controller's own cards and a hit on
/// the opponent's. A chance of 0 rolls nothing and one of 100 hits every card with no roll. The cards
/// are ones no two players both read, so, as R440 has a change over a hidden scope, nothing announces
/// them: their owner reads the new prices off the view.
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
