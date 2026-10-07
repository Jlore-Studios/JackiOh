//! Two small gifts a Classic+ card gives its own side (SPEC §8.7): Armor its hero keeps for the rest of
//! the game (C+ #46 Felinor Flagbearer) and a discount on a random card of a hand (C+ #49 Jay Fungus).
//!
//! Port of `packages/engine/src/effects/perks.ts`.

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::effects::cost::set_cost_mod;
use crate::effects::targets::{PlayerSpec, player_of};
use crate::mana::{effective_cost, is_x_cost};
use crate::prelude::json_as;
use crate::script::Effect;
use crate::state::CardInstance;

/// `gainHeroArmor`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GainHeroArmorArgs {
    pub amount: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// C+ #46: "Your hero gains +N Armor for the rest of the game" — §4.4 step 2's per-hit reduction, kept on
/// the hero itself (`hero.armor`), so it outlasts the card that gave it and stacks with every other
/// source (`damage::hero_armor_of`, R124). The view carries the hero's Armor; no event of its own.
pub fn gain_hero_armor(args: GainHeroArmorArgs) -> Effect {
    Effect::new("gainHeroArmor", move |ctx| {
        let amount = args.amount.max(0);
        if amount == 0 {
            return;
        }
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        ctx.state.players[player].hero.armor += amount;
    })
}

/// `discountRandomInHand`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiscountRandomInHandArgs {
    pub amount: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// C+ #49: "A random card in your hand costs (N) less" — `costMod` −N on one card drawn uniformly from
/// the hand cards it can make cheaper: cost above 0 now (R65's `effective_cost`) and not an X-cost card
/// (R65: modifiers never reach X). None such is nothing to do, so nothing is drawn (R129). The cost
/// floors at 0 when read (§2.3); `set_cost_mod` reports it under R177's sentinel to the other seat.
pub fn discount_random_in_hand(args: DiscountRandomInHandArgs) -> Effect {
    Effect::new("discountRandomInHand", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let hand = ctx.state.players[player].hand.clone();
        let mut cheaper: Vec<CardInstance> = Vec::new();
        for card in hand {
            if !is_x_cost(ctx.state, &card) && effective_cost(ctx.state, &card, Default::default()) > 0 {
                cheaper.push(card);
            }
        }
        if cheaper.is_empty() {
            return;
        }
        let Some(card) = ctx.sink.rng.pick(&cheaper).cloned() else {
            return;
        };
        let effect = set_cost_mod(json_as(json!({
            "target": { "of": "instance", "instanceId": card.id },
            "amount": -args.amount,
        })));
        (effect.apply)(ctx);
    })
}
