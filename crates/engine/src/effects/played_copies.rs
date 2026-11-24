//! Copies of Units the opponent played (R4594009): `summonPlayedCopies`, for Infinite Assault
//! #229 The Harvester and #230 Everything You've Ever Played.
//!
//! The pool is the opponent's Units played, read off the recursion log (`recursion_log::units_played_by`,
//! R4589000), each definition looked up through `find_def` — a missing definition is skipped — kept
//! to Units. With `fill`, one
//! `pick_generated_unique` per `fill_board_zones` slot, so the copies are distinct, uniform and
//! balanced across the board (R4594009); otherwise one pick, summoned `copies` times (default 1),
//! so the Harvester's Radiant face is one pick twice. `radiant` makes the copies Radiant. With
//! nothing to draw from, or nowhere to go, it draws nothing (R129).

use serde::{Deserialize, Serialize};

use crate::catalog::{find_def, pick_generated_unique};
use crate::effects::summon::{SummonPlacement, summon_fresh};
use crate::script::Effect;
use crate::wire::{CardDef, CardType, PlayerId, opponent_of};
use crate::zones::fill_board_zones;

/// `summonPlayedCopies`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SummonPlayedCopiesArgs {
    /// How many copies of the one pick, without `fill`. Default 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copies: Option<i32>,
    /// Summon one copy per open unit zone instead. Default false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<bool>,
    /// The copies are Radiant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
}

fn played_unit_pool(state: &crate::state::GameState, opponent: PlayerId) -> Vec<CardDef> {
    crate::recursion_log::units_played_by(state, opponent)
        .iter()
        .filter_map(|def_id| find_def(Some(state), def_id))
        .filter(|def| def.type_ == CardType::Unit)
        .cloned()
        .collect()
}

/// R4594009: summon fresh copies of Units the opponent played this game onto the controller's
/// board, on their printed face — Radiant ones when `radiant`.
pub fn summon_played_copies(args: SummonPlayedCopiesArgs) -> Effect {
    Effect::new("summonPlayedCopies", move |ctx| {
        let player = ctx.controller;
        let pool: Vec<CardDef> = played_unit_pool(ctx.state, opponent_of(player));
        if pool.is_empty() {
            return;
        }
        let pool_refs: Vec<&CardDef> = pool.iter().collect();
        let slots = fill_board_zones(ctx.state, player);
        if slots.is_empty() {
            return;
        }
        if args.fill == Some(true) {
            for slot in slots {
                let Some(def_id) = pick_generated_unique(ctx.sink.rng, &pool_refs, None, &mut ctx.draw_tally)
                    .map(|def| def.id.clone())
                else {
                    return;
                };
                let at = SummonPlacement {
                    lane: Some(slot.lane),
                    radiant: args.radiant,
                    ..Default::default()
                };
                summon_fresh(ctx, &def_id, player, &at);
            }
            return;
        }
        let copies = args.copies.unwrap_or(1).max(0);
        if copies <= 0 {
            return;
        }
        let Some(def_id) = pick_generated_unique(ctx.sink.rng, &pool_refs, None, &mut ctx.draw_tally)
            .map(|def| def.id.clone())
        else {
            return;
        };
        for _ in 0..copies {
            let at = SummonPlacement {
                radiant: args.radiant,
                ..Default::default()
            };
            summon_fresh(ctx, &def_id, player, &at);
        }
    })
}
