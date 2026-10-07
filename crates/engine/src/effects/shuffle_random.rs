//! Shuffle random catalog cards into a library (§6.3 Shuffle into, §5.1; Classic+ #40 Appropriations'
//! Education, E39): `count` independent picks from the pool (repeats allowed, R60; never the running
//! card, R387), each made Radiant and enchanted as asked before it goes in at a uniformly random
//! position, so the enchantment rides it from the start (`enchantments.rs`). R80's cap turns the rest
//! away, as `shuffle_into` does; an empty pool does nothing and draws nothing (R129).
//!
//! Port of `packages/engine/src/effects/shuffleRandom.ts`.

use serde::{Deserialize, Serialize};

use crate::catalog::{CatalogQueryArgs, excluding_def_id, pick_generated, query};
use crate::draw::shuffle_into_library;
use crate::effects::targets::{PlayerSpec, player_of};
use crate::script::{Effect, EffectContext};
use crate::state::{CardInstance, new_instance};
use crate::wire::{Enchantment, PlayerId, Zone};

/// `shuffleRandomFromCatalog`'s arguments.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShuffleRandomFromCatalogArgs {
    pub query: CatalogQueryArgs,
    pub count: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enchantments: Option<Vec<Enchantment>>,
}

pub fn shuffle_random_from_catalog(args: ShuffleRandomFromCatalogArgs) -> Effect {
    Effect::new("shuffleRandomFromCatalog", move |ctx| {
        let own = ctx
            .self_
            .as_ref()
            .map(|me| me.def_id.clone())
            .or_else(|| ctx.def_id.clone());
        let pool = query(&excluding_def_id(Some(&*ctx.state), &args.query, own.as_deref()));
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let mut at = 0;
        while at < args.count && !pool.is_empty() {
            // R673: a card generated into a deck may be Glitch (`catalog::GlitchOdds` is the state).
            let Some(def_id) = pick_generated(ctx.sink.rng, &pool, Some(&*ctx.sink.state)).map(|def| def.id.clone()) else {
                return;
            };
            let mut card = new_instance(&mut *ctx.state, &def_id, player, Zone::Library { player });
            if args.radiant == Some(true) {
                card.radiant = true;
            }
            for enchantment in args.enchantments.iter().flatten() {
                crate::enchantments::add_enchantment(&mut card, enchantment);
            }
            shuffle_into_library(ctx, &mut card, false, None);
            at += 1;
        }
    })
}
