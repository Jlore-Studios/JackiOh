//! B5 E34, the perfect-hand scorer (SPEC §8.7 C+ #27 Zephrys Zealotism, §10.7, R29, R387, R416).
//!
//! "Replace your hand with the perfect hand of Classic and Classic+ cards." The scoring is the Zephyrs
//! scorer's own (`./scorer`, R29, untouched): `scoreDef` and its dry run on one copy of the state as
//! the card resolves, which draws from a seed of its own, never the match rng, so the same state gives
//! the same hand. This adds only the pool (every non-token Classic and Classic+ card but the asking
//! card, R387), the face (Radiant on the Radiant face), the order (score, then card id, since an index
//! repeats across sets) and the hand.
//!
//! Port of `packages/engine/src/subsystems/perfectHand.ts`.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::prelude::json_as;
use crate::script::Effect;
use crate::state::{CardInstance, GameState, find_instance};
use crate::subsystems::scorer::{Scored, ScorerOptions, dry_run_base, score_def};
use crate::wire::PlayerId;

/// `rankPerfectHand`'s options (TS `{ radiant?: boolean; selfDefId?: string }`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RankPerfectHandOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub self_def_id: Option<String>,
}

/// `replaceHandWithPerfect`'s argument (TS `{ radiant?: boolean }`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceHandWithPerfectArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
}

/// R416: every non-token Classic and Classic+ card but `selfDefId`'s (R387), best first, ties by id.
pub fn rank_perfect_hand(state: &GameState, viewer: PlayerId, options: RankPerfectHandOptions) -> Vec<Scored> {
    let mut base = dry_run_base(state, viewer);
    // TS `excludingDefId({ set: ["Classic", "Classic+"] }, selfDefId)`: the running card's ids added to
    // the pool's exclusions, none when there is no running card.
    let excluded: Vec<String> = match options.self_def_id.as_deref() {
        None => Vec::new(),
        Some(self_def_id) => {
            let mut ids: Vec<String> = Vec::new();
            for id in crate::catalog::self_def_ids(Some(state), self_def_id) {
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
            ids
        }
    };
    let pool = if excluded.is_empty() {
        json!({ "set": ["Classic", "Classic+"] })
    } else {
        json!({ "set": ["Classic", "Classic+"], "excludeDefId": excluded })
    };
    let pool: crate::catalog::CatalogQueryArgs = json_as(pool);
    let radiant = options.radiant == Some(true);
    let scorer_options = ScorerOptions {
        radiant: Some(radiant),
    };
    let mut ranked: Vec<Scored> = crate::catalog::query(&pool)
        .into_iter()
        .map(|def| score_def(state, viewer, def, &scorer_options, base.as_mut()))
        .collect();
    // TS `b.score - a.score || (a.def.id < b.def.id ? -1 : 1)`: a NaN difference ties, as JS reads it.
    ranked.sort_by(|a, b| match b.score.partial_cmp(&a.score) {
        Some(Ordering::Equal) | None => a.def.id.cmp(&b.def.id),
        Some(order) => order,
    });
    ranked
}

/// R416: each other card in the controller's hand goes to their graveyard — a replace, not a discard
/// (#76's reading; a unit-token card ceases to exist, R11) — and the top N of the ranking, made on the
/// state before anything moves, arrive in rank order at their printed cost. N is the hand's size, so
/// an empty hand ranks nothing and gets nothing.
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
            // TS read the live object's zone after the move; the card as it now stands.
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
