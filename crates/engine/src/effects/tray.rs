//! Stinkyscape's Resources (#1840, R1840000–R1840006): the verbs a card composes over the Resource
//! tray and Credits, each a front for `crate::resources`' rule.
//!
//! - `gain_resources` — "Gain N random Resources", from the catalog's Resources or a pool the card
//!   names, Radiant on a Radiant gain (R1840002); "Fill your hand with Resources" fills the tray up to
//!   `RESOURCE_TRAY_CAP` (`fill`, R1840004). A gain into a full tray is a sale (R1840001).
//! - `discard_resource` — "Discard a Narcor": the oldest tray Resource of those definitions ceases to
//!   exist (R1840004). No hand rule reaches the tray, so a hand discard never takes one.
//! - `gain_market_credits` — "Gain N Credits" (R1840005).

use serde::{Deserialize, Serialize};

use crate::config::RESOURCE_TRAY_CAP;
use crate::effects::targets::{PlayerSpec, player_of};
use crate::resources::{add_market_credits, gain_resource, roll_resource};
use crate::script::Effect;
use crate::wire::GameEvent;
use crate::zones::cease_to_exist;

/// `gainResources`' argument: how many Resources, whose tray (default the controller's own), whether
/// they are Radiant, the pool they come from (default every Resource), and `fill` for "fill your hand
/// with Resources", which gains as many as the tray has room for and reads no `count`.
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

/// R1840000–R1840002, R1840004: gain random Resources one at a time, each rolled as it is gained, so a
/// gain past the cap is sold (R1840001) and the next is rolled all the same. A pool that holds no
/// Resource gains nothing and draws nothing.
pub fn gain_resources(args: GainResourcesArgs) -> Effect {
    Effect::new("gainResources", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let count = if args.fill == Some(true) {
            let held = ctx.sink.state.players[player]
                .resource_tray
                .as_ref()
                .map_or(0, Vec::len) as i32;
            RESOURCE_TRAY_CAP - held
        } else {
            args.count
        };
        let radiant = args.radiant == Some(true);
        for _ in 0..count.max(0) {
            let sink = &mut ctx.sink;
            let Some(def_id) = roll_resource(sink.state, sink.rng, args.pool.as_deref()) else {
                return;
            };
            gain_resource(sink, player, &def_id, radiant);
        }
    })
}

/// `discardResource`'s argument: the definitions it may take (any Resource when empty) and whose tray
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

/// R1840004: "Discard a Narcor" — the oldest Resource in the tray of one of `def_ids` (any, with none
/// named) ceases to exist, with the public `resourceDiscarded`. No prompt: an empty tray, or one with
/// no match, discards nothing.
pub fn discard_resource(args: DiscardResourceArgs) -> Effect {
    Effect::new("discardResource", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let Some(mut card) = ctx.sink.state.players[player]
            .resource_tray
            .iter()
            .flatten()
            .find(|card| args.def_ids.is_empty() || args.def_ids.contains(&card.def_id))
            .cloned()
        else {
            return;
        };
        cease_to_exist(ctx.sink.state, &mut card);
        ctx.sink.events.push(GameEvent::ResourceDiscarded {
            player,
            instance_id: card.id.clone(),
            def_id: card.def_id.clone(),
        });
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
