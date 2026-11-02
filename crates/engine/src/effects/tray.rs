//! Stinkyscape's Resources (#1840, R1840000–R1840006; #2250, R2250000, R2250001): the verbs a card
//! composes over Resources (hand cards) and Credits, each a front for `crate::resources`' rule, and
//! `add_market_lot`, Call to Chaos (Scape Edition)'s "a Call to Chaos at 20" (`crate::marketplace`).

use serde::{Deserialize, Serialize};

use crate::effects::targets::{PlayerSpec, player_of};
use crate::marketplace::add_fixed_market_lot;
use crate::resources::{add_market_credits, gain_resource, is_resource_card, roll_resource};
use crate::script::Effect;

/// R1855002: the memory key of a board replacement's rider naming the card's own Resource pool (a list
/// of definition ids), which its gains come from in place of the pool its text names.
pub const RESOURCE_POOL_KEY: &str = "resourcePool";

/// `gainResources`' argument: how many Resources, whose hand (default the controller's own), whether
/// they are Radiant, the pool they come from (default every Resource), and `fill` for "fill your hand
/// with Resources", which gains as many as the hand has room for and reads no `count`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct GainResourcesArgs {
    #[serde(default)]
    pub count: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pool: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<bool>,
}

/// R1840002, R2250000: gain random Resources one at a time, each rolled as it is gained and added to the
/// hand, so a gain into a full hand burns (R4) and the next is rolled all the same. A pool that holds no
/// Resource gains nothing and draws nothing.
pub fn gain_resources(args: GainResourcesArgs) -> Effect {
    Effect::new("gainResources", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let count = if args.fill == Some(true) {
            crate::query::room_in(ctx.sink.state, player, crate::zones::OffFieldZone::Hand)
        } else {
            args.count
        };
        let radiant = args.radiant == Some(true);
        // R1855002: a preset card's rider names the pool its own gains come from.
        let pool: Option<Vec<String>> = ctx
            .self_
            .as_ref()
            .and_then(|card| card.memory.get(RESOURCE_POOL_KEY))
            .and_then(|value| serde_json::from_value(value.clone()).ok())
            .or_else(|| args.pool.clone());
        for _ in 0..count.max(0) {
            let sink = &mut ctx.sink;
            let Some(def_id) = roll_resource(sink.state, sink.rng, pool.as_deref()) else {
                return;
            };
            gain_resource(sink, player, &def_id, radiant);
        }
    })
}

/// `discardResource`'s argument: the definitions it may take (any Resource when empty) and whose hand
/// (default the controller's own).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DiscardResourceArgs {
    #[serde(default)]
    pub def_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// R1840004, R2250000: "Discard a Narcor" — the Resource longest in the hand of one of `def_ids` (any,
/// with none named) is discarded, a hand discard like any other (`discarded`), which pays its Credits
/// (R2250001). No prompt: a hand with no match discards nothing.
pub fn discard_resource(args: DiscardResourceArgs) -> Effect {
    Effect::new("discardResource", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let state = &*ctx.sink.state;
        let Some(card) = state.players[player]
            .hand
            .iter()
            .find(|card| {
                is_resource_card(state, card)
                    && (args.def_ids.is_empty() || args.def_ids.contains(&card.def_id))
            })
            .cloned()
        else {
            return;
        };
        crate::effects::move_::discard_from_hand(&mut ctx.sink, &card);
    })
}

/// `gainMarketCredits`' argument: how many Credits, and whose (default the controller's own).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct GainMarketCreditsArgs {
    pub amount: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// R1840005: "Gain N Credits" — Paycheck's and Megabase's shape. A gain of 0 or less does nothing.
pub fn gain_market_credits(args: GainMarketCreditsArgs) -> Effect {
    Effect::new("gainMarketCredits", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        add_market_credits(&mut ctx.sink, player, args.amount);
    })
}

/// `addMarketLot`'s argument: the ids the lot is drawn from, its price in Credits, and whose lot it is
/// (default the controller's own).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct AddMarketLotArgs {
    pub pool: Vec<String>,
    pub price: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// R1853005: a lot for the rest of the game — one card of `pool`, drawn as it is added, offered to its
/// player alone at `price` Credits that never move, a copy a cycle. Opens the Marketplace.
pub fn add_market_lot(args: AddMarketLotArgs) -> Effect {
    Effect::new("addMarketLot", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        add_fixed_market_lot(&mut ctx.sink, player, &args.pool, args.price);
    })
}
