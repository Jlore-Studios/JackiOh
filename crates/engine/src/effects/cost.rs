//! Cost changes on one card instance: the two inputs R65 reads before any player discount. Both
//! persist through a hand, a library and leaving the field (R78), and both go as the card reaches a
//! graveyard or an exile pile (R766), so a card discounted in hand costs its printed cost there.
//! The order the two combine is `effectiveCost`'s (§6.3 Cost, R65); nothing here recomputes it.

use serde::{Deserialize, Serialize};

use crate::damage::DamageTarget;
use crate::effects::targets::{TargetSpec, resolve_target};
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
/// says it was made there (`hiddenFrom`) and a view keeps it unread: a library-wide change (#95) emits
/// one event per card in library order, which would give away a card's place in the batch.
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
