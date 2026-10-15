//! B5 E34, the perfect-hand scorer (SPEC §8.7 C+ #27 Zephrys Zealotism, §10.7, R29, R387, R416, R1446).
//!
//! "Replace your hand with the perfect hand." The scoring is the Zephyrs scorer's own (`./scorer`,
//! R29, untouched): `scoreDef` and its dry run on one copy of the state as the card resolves, which
//! draws from a seed of its own, never the match rng, so the same state gives the same hand. Its pool
//! is the scorer's `perfect_pool` (every non-token card of every set that ships but the asking card's
//! own ids, R387, R1446). This adds only the face (Radiant on the Radiant face), the order (score,
//! then card id, since an index repeats across sets) and the hand.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::layers::AuraTable;
use crate::prelude::json_as;
use crate::script::Effect;
use crate::state::{CardInstance, GameState, find_instance};
use crate::subsystems::scorer::{Scored, ScorerOptions, dry_run_base, perfect_pool, score_def_in};
use crate::wire::PlayerId;

/// `rank_perfect_hand`'s options.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct RankPerfectHandOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub self_def_id: Option<String>,
}

/// `replace_hand_with_perfect`'s argument.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ReplaceHandWithPerfectArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
}

/// R416, R1446: the perfect pool but `selfDefId`'s ids (R387), best first, ties by id.
pub fn rank_perfect_hand(
    state: &GameState,
    viewer: PlayerId,
    options: RankPerfectHandOptions,
) -> Vec<Scored> {
    let mut base = dry_run_base(state, viewer);
    // The running card's ids are the pool's exclusions; none when there is no running card.
    let pool = perfect_pool(Some(state), options.self_def_id.as_deref());
    let radiant = options.radiant == Some(true);
    let scorer_options = ScorerOptions {
        radiant: Some(radiant),
    };
    // #892: one aura table for every candidate.
    let auras = AuraTable::new(state);
    let mut ranked: Vec<Scored> = pool
        .into_iter()
        .map(|def| score_def_in(&auras, viewer, def, &scorer_options, base.as_mut()))
        .collect();
    // Best score first; a NaN difference ties, and ties go by card id.
    ranked.sort_by(|a, b| match b.score.partial_cmp(&a.score) {
        Some(Ordering::Equal) | None => a.def.id.cmp(&b.def.id),
        Some(order) => order,
    });
    ranked
}

/// R416: each other card in the controller's hand goes to their graveyard — a replace, not a discard
/// (a unit-token card ceases to exist, R11) — and the top N of the ranking, made on the state before
/// anything moves, arrive in rank order at their printed cost, N being the hand's size.
pub fn replace_hand_with_perfect(args: ReplaceHandWithPerfectArgs) -> Effect {
    Effect::new("replaceHandWithPerfect", move |ctx| {
        let hand: Vec<CardInstance> = ctx.sink.state.players[ctx.controller].hand.clone();
        if hand.is_empty() {
            return;
        }
        let radiant = args.radiant == Some(true);
        let self_def_id = ctx
            .live_self()
            .map(|card| card.def_id.clone())
            .or_else(|| ctx.def_id.clone());
        let picks = rank_perfect_hand(
            ctx.sink.state,
            ctx.controller,
            RankPerfectHandOptions {
                radiant: Some(radiant),
                self_def_id,
            },
        );
        for card in &hand {
            let mut card = card.clone();
            let result = crate::zones::move_to_zone(
                ctx.sink.state,
                &mut card,
                crate::zones::OffFieldZone::Graveyard,
                Default::default(),
            );
            let landed = find_instance(ctx.sink.state, &card.id)
                .cloned()
                .unwrap_or_else(|| card.clone());
            crate::zones::report_graveyard_landing(&mut ctx.sink, &landed, result);
        }
        for scored in picks.iter().take(hand.len()) {
            let effect = crate::effects::add_to_hand::add_to_hand(json_as(
                json!({ "defId": scored.def.id, "radiant": radiant }),
            ));
            (effect.apply)(ctx);
        }
    })
}
