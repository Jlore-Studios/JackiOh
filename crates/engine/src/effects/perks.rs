//! Two small gifts a Classic+ card gives its own side (SPEC §8.7): Armor its hero keeps for the rest of
//! the game (C+ #46 Felinor Flagbearer) and a discount on a random card of a hand (C+ #49 Jay Fungus).

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::damage::hero_armor_of;
use crate::effects::cost::set_cost_mod;
use crate::effects::targets::{PlayerSpec, player_of};
use crate::mana::{effective_cost, is_x_cost};
use crate::prelude::json_as;
use crate::script::Effect;
use crate::state::CardInstance;
use crate::wire::GameEvent;

/// `gainHeroArmor`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct GainHeroArmorArgs {
    pub amount: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// C+ #46: §4.4 step 2's per-hit reduction kept on the hero itself, so it outlasts the card that gave
/// it and stacks with every other source (`damage::hero_armor_of`, R124). R3857000: a gain above 0
/// reports `armorGained`, its amount and the hero's Armor after, withheld from the trigger loop.
pub fn gain_hero_armor(args: GainHeroArmorArgs) -> Effect {
    Effect::new("gainHeroArmor", move |ctx| {
        let amount = args.amount.max(0);
        if amount == 0 {
            return;
        }
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        ctx.state.players[player].hero.armor += amount;
        let armor = hero_armor_of(&*ctx.sink.state, player);
        ctx.sink.events.push(GameEvent::ArmorGained {
            player,
            amount,
            armor,
        });
        // R3857000: a report nothing answers, so the frontier never takes it (R1361's way, as `rolled`'s).
        let at = ctx.sink.events.len() - 1;
        crate::triggers::withhold_report(&mut ctx.sink, at);
    })
}

/// `discountRandomInHand`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DiscountRandomInHandArgs {
    pub amount: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// C+ #49: −N on one hand card drawn uniformly from those it can make cheaper (cost above 0 now, not
/// X-cost: R65); with none, nothing is drawn (R129). The cost floors at 0 when read (§2.3), and
/// `set_cost_mod` reports it under R177's sentinel to the other seat.
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
