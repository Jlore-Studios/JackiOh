//! Classic+ Fruit verbs (SPEC §8.7, the cards-plus-d workstream): the Grapes a Grape card rolls (C+ #65
//! Two Grapes, #66 Vine of Grapes; `GRAPE_ODDS`, R382), a draw whose card takes a price (C+ #65.2 Normal
//! Grape, #65.3 Large Grape), a hit on an enemy that is a heal on a friend (the same two Grapes), and a
//! hand replaced card for card by random cards (C+ #65.5 Mythic Grape).
//!
//! Each is card-specific — no Core card and no generic B5 system asks for any of them — so they live
//! here, beside the effects library they are written in, and nowhere else.
//!
//! Port of `packages/engine/src/effects/fruit.ts`.

use serde::{Deserialize, Serialize};

use crate::catalog::{CatalogQueryArgs, excluding_def_id, pick_generated, query};
use crate::draw::{DrawOutcome, draw_one};
use crate::effects::add_to_hand::{AddToHandArgs, add_to_hand};
use crate::effects::cost::{SetCostModArgs, SetCostOverrideArgs, set_cost_mod, set_cost_override};
use crate::effects::damage::{DamageEffectArgs, DamageFlagArgs, damage};
use crate::effects::heal::{HealArgs, heal};
use crate::effects::targets::{PlayerSpec, TargetSpec, player_of};
use crate::numbers::numbered_keywords_on;
use crate::script::{Effect, EffectContext, EngineSink};
use crate::state::{CardInstance, find_instance};
use crate::wire::{GameEvent, PlayerId, Selection, ZoneName};
use crate::zones::{OffFieldZone, move_to_zone, report_graveyard_landing};

// `rollGrape` lives beside `query` in `../catalog` now (R382's generic re-roll reads it there too);
// still exported here, so `@jackioh/engine/effects` keeps one name for it.
pub use crate::catalog::roll_grape;

// ---------------------------------------------------------------------------------------------
// Grapes (C+ #65, #66)
// ---------------------------------------------------------------------------------------------

/// §6.1: the Lucky X the card running the script has now — its running face's printed Lucky, as a
/// Degrade or an Upgrade has moved it (B3.4's X change), 0 without one.
fn lucky_of(ctx: &EffectContext<'_>) -> i32 {
    let Some(own) = ctx.self_.as_ref() else {
        return 0;
    };
    numbered_keywords_on(ctx.sink.state, own)
        .into_iter()
        .find(|keyword| keyword.key.as_str() == "Lucky")
        .map(|keyword| keyword.value)
        .unwrap_or(0)
}

/// `addRolledGrapes`' argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AddRolledGrapesArgs {
    pub count: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lucky: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// C+ #65 Two Grapes, #66 Vine of Grapes: "Add N Grapes to your hand, each rolled" — N independent
/// rolls (R60), each Grape created in the hand through §6.3's Add to hand, so a full hand burns it (§2.4,
/// R4). `radiant` makes every Grape Radiant. The Lucky of the card running the script applies to every
/// roll (the Radiant faces print Lucky 1), unless `lucky` names a number.
///
/// Each Grape is rolled and added before the next is rolled, so a fixed seed gives fixed Grapes in a
/// fixed order (§10.7).
pub fn add_rolled_grapes(args: AddRolledGrapesArgs) -> Effect {
    Effect::new("addRolledGrapes", move |ctx| {
        let count = args.count.max(0);
        let lucky = match args.lucky {
            Some(lucky) => lucky,
            None => lucky_of(ctx),
        };
        for _ in 0..count {
            if ctx.sink.state.result.is_some() {
                return;
            }
            let def_id = roll_grape(&mut *ctx.sink.rng, lucky);
            if def_id.is_empty() {
                return;
            }
            let add = add_to_hand(AddToHandArgs {
                def_id: Some(def_id),
                player: args.player,
                radiant: if args.radiant == Some(true) { Some(true) } else { None },
                ..AddToHandArgs::default()
            });
            (add.apply)(ctx);
        }
    })
}

// ---------------------------------------------------------------------------------------------
// A hit on an enemy, a heal on a friend (C+ #65.2, #65.3)
// ---------------------------------------------------------------------------------------------

/// `damageEnemyOrHealFriend`'s argument.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DamageEnemyOrHealFriendArgs {
    pub amount: i32,
}

/// C+ #65.2 Normal Grape, #65.3 Large Grape: "Choose a Unit or hero. If it's an enemy, deal N damage to
/// it; if it's yours, heal it N." The chosen card or hero (the play's declared target, R81) is judged
/// as the effect resolves: a hero by its seat, a Unit by its controller. An enemy takes one §4.4 hit
/// from the card running the script (a Spell's, so Spell Damage raises it); a friend is healed (§6.3
/// Heal, R19). A target gone by then is nothing.
pub fn damage_enemy_or_heal_friend(args: DamageEnemyOrHealFriendArgs) -> Effect {
    Effect::new("damageEnemyOrHealFriend", move |ctx| {
        let Some(picked) = ctx.targets.first().cloned() else {
            return;
        };
        let side: Option<PlayerId> = match &picked {
            Selection::Hero { player } => Some(*player),
            Selection::Instance { instance_id } => {
                find_instance(ctx.sink.state, instance_id).map(|card| card.controller)
            }
            _ => None,
        };
        let Some(side) = side else {
            return;
        };
        let spec = TargetSpec::Chosen { index: None };
        let effect = if side == ctx.controller {
            heal(HealArgs {
                target: spec,
                amount: Some(args.amount),
                to_full: None,
                up_to: None,
            })
        } else {
            damage(DamageEffectArgs {
                to: spec,
                amount: args.amount,
                flags: DamageFlagArgs::default(),
            })
        };
        (effect.apply)(ctx);
    })
}

// ---------------------------------------------------------------------------------------------
// A draw whose card takes a price (C+ #65.2, #65.3)
// ---------------------------------------------------------------------------------------------

/// R596: the card one draw put in a hand, by the `drawn` event that draw made: the first `drawn` of
/// that player after `from` — a draw's own event comes before anything its card's cast draws (R58) —
/// and only when the draw ended with the card in a hand (`drawn`, or Infinite Reserves' Rush Token). A
/// card cast on draw never reaches the hand, and the card its chain's repeat then brings is that repeat's
/// card, not this draw's; a burned one is in the graveyard, a fatigue draw brings none and a limited
/// draw none either (§2.4, R4, R58, R457), so each of those has no card.
///
/// (TS `ctx: Pick<EffectContext, "state" | "events">`: a sink, which an `EffectContext` derefs to.)
pub fn card_this_draw_put_in_hand(
    sink: &EngineSink<'_>,
    player: PlayerId,
    from: usize,
    outcome: DrawOutcome,
) -> Option<CardInstance> {
    if !matches!(outcome, DrawOutcome::Drawn | DrawOutcome::Token) {
        return None;
    }
    for event in sink.events.iter().skip(from) {
        let GameEvent::Drawn {
            player: drawer,
            instance_id,
            ..
        } = event
        else {
            continue;
        };
        if *drawer != player {
            continue;
        }
        return find_instance(sink.state, instance_id)
            .filter(|card| card.zone.z() == ZoneName::Hand)
            .cloned();
    }
    None
}

/// `drawPriced`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DrawPricedArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_mod: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_override: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// C+ #65.2 "Draw N. Each costs (1) less." / #65.3 "Draw N. Each costs (0).": ONE draw (§2.4 — "draw N"
/// is N of these, one effect each, so a draw whose cast-on-draw card asks pauses the rest of the list
/// and the answer makes the rest, R113), and the card that draw put in the hand takes the price: a
/// `costMod` that stacks with every other modifier, or a `costOverride` (R65). A card cast on draw, a
/// burned card, a fatigue draw and a limited draw take nothing (`cardThisDrawPutInHand`).
pub fn draw_priced(args: DrawPricedArgs) -> Effect {
    Effect::new("drawPriced", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let from = ctx.sink.events.len();
        let outcome = draw_one(ctx, player, None);
        let Some(card) = card_this_draw_put_in_hand(ctx, player, from, outcome) else {
            return;
        };
        let target = TargetSpec::Instance {
            instance_id: card.id.clone(),
        };
        if let Some(cost) = args.cost_override {
            let price = set_cost_override(SetCostOverrideArgs {
                target: Some(target.clone()),
                cost,
                in_hand_only: Some(true),
            });
            (price.apply)(ctx);
        }
        if let Some(amount) = args.cost_mod {
            let price = set_cost_mod(SetCostModArgs {
                target: Some(target),
                amount,
                in_hand_only: Some(true),
            });
            (price.apply)(ctx);
        }
    })
}

// ---------------------------------------------------------------------------------------------
// A hand replaced card for card (C+ #65.5)
// ---------------------------------------------------------------------------------------------

/// `replaceHandWithRandom`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceHandWithRandomArgs {
    pub query: CatalogQueryArgs,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_override: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// C+ #65.5 Mythic Grape: "Replace your hand with random Mythic cards. They cost (0)." Every card in the
/// player's hand as this resolves goes to its owner's graveyard — moved, NOT discarded: no `discarded`
/// event, so nothing that answers a discard sees it; `enteredGraveyard` reports it, and a unit-token
/// card ceases to exist instead (R11) — and as many random cards of `query` arrive, repeats allowed
/// (R60), each created through §6.3's Add to hand with the riders given (Core #76 Field of Dreams'
/// reading of "replace your hand"). The card running the script is never one of them (R387). An empty
/// hand moves nothing and draws nothing from the rng (R129).
pub fn replace_hand_with_random(args: ReplaceHandWithRandomArgs) -> Effect {
    Effect::new("replaceHandWithRandom", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        // A snapshot: each move splices the hand.
        let replaced: Vec<CardInstance> = ctx.sink.state.players[player].hand.clone();
        if replaced.is_empty() {
            return;
        }
        for card in &replaced {
            // The card as it stands now (TS read the live object's zone).
            let Some(card) = find_instance(ctx.sink.state, &card.id).cloned() else {
                continue;
            };
            if card.zone.z() != ZoneName::Hand {
                continue;
            }
            let moved = move_to_zone(ctx.sink.state, &card, OffFieldZone::Graveyard, Default::default());
            let landed = find_instance(ctx.sink.state, &card.id).cloned().unwrap_or(card);
            report_graveyard_landing(ctx, &landed, moved);
        }
        let own = ctx
            .self_
            .as_ref()
            .map(|card| card.def_id.clone())
            .or_else(|| ctx.def_id.clone());
        let pool = query(&excluding_def_id(&args.query, own.as_deref()));
        if pool.is_empty() {
            return;
        }
        for _ in 0..replaced.len() {
            if ctx.sink.state.result.is_some() {
                return;
            }
            // R673: a card generated into a hand may be Glitch.
            let Some(def) = pick_generated(&mut *ctx.sink.rng, &pool, Some(&*ctx.sink.state)) else {
                return;
            };
            let def_id = def.id.clone();
            let add = add_to_hand(AddToHandArgs {
                def_id: Some(def_id),
                player: args.player,
                radiant: if args.radiant == Some(true) { Some(true) } else { None },
                cost_override: args.cost_override,
                ..AddToHandArgs::default()
            });
            (add.apply)(ctx);
        }
    })
}
