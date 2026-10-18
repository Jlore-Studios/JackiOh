//! The policy prior (#1171; `docs/learned-eval.md`, learned move ordering): a table of weights, fitted
//! by `training/ml` on the moves the AI played, that re-sorts the moves the beam expands before it
//! takes its prefix, so a move the heuristic tiers rank past the prefix can still be searched.
//!
//! - **Where.** `AI_MOVE_ORDER` (`config.rs`) chooses. With `MoveOrder::Heuristic`, the default,
//!   nothing changes. With `MoveOrder::Learned`, `search::expansion` sorts each line's moves by the
//!   prior (`prior_order`) before it takes the first `root_branching` or `branching` of them. It
//!   re-sorts and never adds, drops or changes a move, End Turn is always kept, and it spends no node
//!   (R1171001).
//! - **What.** A move is a handful of text keys (`policy_keys`): its kind, its rank in the heuristic
//!   order, the card it plays or acts with, where it lands and what it aims at. Its score is the sum
//!   of its keys' weights; a key the table lacks adds nothing, so a new or custom card is ordered by
//!   its other keys alone.
//! - **What it reads.** Only `redact(state, seat)` (R185): a card is named only where the seat may
//!   read it, by R1006000's rule, and is `hidden` otherwise, and no key holds an instance id, a hero
//!   id or a prompt's id (R1171000). So two states that differ only in what the seat cannot see give
//!   the same keys.
//! - **Labels.** `policy_label` is a decision as `cargo jackioh positions --policy` records it: the
//!   keys of every candidate decide's probe lists there and which one was played (R1171000).
//! - **Which games.** 1v1 only. A Duos duel keeps the heuristic order: the table never saw a team.
//! - **Arithmetic.** `f64` sums in key order and a stable sort, so the order is the same on every
//!   machine (CLAUDE.md rule 4).
//! - **Weights.** `src/policy/weights.json`, compiled in by `build.rs` as `POLICY_PRIOR` (rule 4: data
//!   reaches the crate at compile time). It records `POLICY_KEYS_VERSION` and the catalog it was fitted
//!   on. The checked-in table may be the null one, with no keys, which leaves every order as it is.

use std::cmp::Ordering;

use jackioh_engine::{
    ActionBody, CardInstance, GameState, PLAYER_IDS, PlayerId, Rng, Row, Selection, Zone, backrow_is_public,
    find_def, find_instance,
};
use serde::{Deserialize, Serialize};

use crate::candidates::candidate_actions;
use crate::config::{AI_SEARCH, POLICY_RANK_CAP};
use crate::determinize::{DeterminizeOptions, determinize};
use crate::observe::redact;

/// The layout `policy_keys` writes. Bump it with any change to what a key means or which keys a move
/// gets, and refit: a table records the version it was fitted on (R1171000).
pub const POLICY_KEYS_VERSION: u32 = 1;

/// A fitted policy prior, borrowed: `POLICY_PRIOR` holds the compiled-in one, and a test can lend its own.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PolicyPrior<'a> {
    pub keys_version: u32,
    pub catalog: &'a str,
    /// The keys the table has a weight for, sorted: `weights[i]` is `keys[i]`'s.
    pub keys: &'a [&'a str],
    pub weights: &'a [f64],
}

impl PolicyPrior<'_> {
    /// R1171001: the sum of `keys`' weights, in `keys`' order; a key the table lacks adds nothing.
    pub fn score(&self, keys: &[String]) -> f64 {
        let mut score = 0.0;
        for key in keys {
            if let Ok(at) = self.keys.binary_search(&key.as_str()) {
                score += self.weights.get(at).copied().unwrap_or(0.0);
            }
        }
        score
    }
}

include!(concat!(env!("OUT_DIR"), "/policy_weights.rs"));

/// R1006000's rule: a card's id when the seat may read it (a placeholder has no definition, and a
/// backrow card is read by `backrow_is_public`, §10.8), else nothing.
fn nameable<'a>(view: &'a GameState, seat: PlayerId, card: &'a CardInstance) -> Option<&'a str> {
    find_def(Some(view), &card.def_id)?;
    if matches!(
        card.zone,
        Zone::Field {
            row: Row::Backrow,
            ..
        }
    ) && !backrow_is_public(view, card, seat)
    {
        return None;
    }
    Some(&card.def_id)
}

/// `own` for the seat's side, `enemy` for the other.
fn side(seat: PlayerId, player: PlayerId) -> &'static str {
    if player == seat { "own" } else { "enemy" }
}

/// What an instance is to the seat, by side and place: `own.units`, `enemy.backrow`, `own.hand`, …;
/// `other` when the view does not hold it.
fn instance_class(view: &GameState, seat: PlayerId, id: &str) -> String {
    let Some(card) = find_instance(view, id) else {
        return "other".to_string();
    };
    let place = match card.zone {
        Zone::Field { row, .. } => row.as_str(),
        _ => card.zone.z().as_str(),
    };
    format!("{}.{place}", side(seat, card.controller))
}

/// One target's keys: `aim:<kind>:<class>`, and `aimcard:<kind>:<card>` when it is a card the seat may name.
fn aim_keys(view: &GameState, seat: PlayerId, kind: &str, target: &Selection, keys: &mut Vec<String>) {
    let class = match target {
        Selection::Instance { instance_id } => {
            if let Some(name) = find_instance(view, instance_id).and_then(|card| nameable(view, seat, card)) {
                keys.push(format!("aimcard:{kind}:{name}"));
            }
            instance_class(view, seat, instance_id)
        }
        Selection::Hero { player } => format!("hero.{}", side(seat, *player)),
        Selection::Zone { player, row, .. } => format!("zone.{}.{}", side(seat, *player), row.as_str()),
        Selection::Mode { option } => {
            if find_instance(view, option).is_some() {
                "mode".to_string()
            } else {
                format!("mode.{option}")
            }
        }
        Selection::Craft { .. } => "craft".to_string(),
        Selection::None => "none".to_string(),
    };
    keys.push(format!("aim:{kind}:{class}"));
}

/// R1171000: `action`'s keys for `seat`, sorted and each once. `view` must already be what the seat
/// may know (`redact`). `rank` is the move's place among the non-End-Turn moves in the heuristic
/// order, capped at `POLICY_RANK_CAP`. No key holds an instance id, a hero id or a prompt's id.
pub fn policy_keys(
    view: &GameState,
    seat: PlayerId,
    action: &ActionBody,
    rank: Option<usize>,
) -> Vec<String> {
    let kind = action.action_type().as_str();
    let mut keys = vec![format!("kind:{kind}")];
    if let Some(rank) = rank {
        keys.push(format!("rank:{}", rank.min(POLICY_RANK_CAP)));
    }

    let source = match action {
        ActionBody::Play { instance_id, .. }
        | ActionBody::Activate { instance_id, .. }
        | ActionBody::ActivatePower { instance_id, .. }
        | ActionBody::SwitchPosition { instance_id } => Some(instance_id),
        ActionBody::Attack { attacker_id, .. } => Some(attacker_id),
        _ => None,
    };
    let source_name = source
        .and_then(|id| find_instance(view, id))
        .and_then(|card| nameable(view, seat, card));
    if let Some(id) = source {
        let name = match (find_instance(view, id), source_name) {
            (None, _) => "unknown",
            (Some(_), Some(name)) => name,
            (Some(_), None) => "hidden",
        };
        keys.push(format!("card:{kind}:{name}"));
    }

    let mut targets: &[Selection] = &[];
    let mut modes: &[String] = &[];
    match action {
        ActionBody::Play {
            zone,
            x,
            embiggen,
            equip,
            tributes,
            targets: play_targets,
            modes: play_modes,
            plague,
            face_down,
            ..
        } => {
            if let Some(zone) = zone {
                keys.push(format!("zone:{}", zone.row.as_str()));
            }
            if *equip == Some(true) {
                keys.push("equip".to_string());
            }
            if *embiggen == Some(true) {
                keys.push("embiggen".to_string());
            }
            if face_down.is_some() {
                keys.push("faceDown".to_string());
            }
            if plague.is_some() {
                keys.push("plague".to_string());
            }
            if let Some(x) = x {
                keys.push(format!("x:{x}"));
            }
            if let Some(tributes) = tributes {
                keys.push(format!("tributes:{}", tributes.len()));
            }
            targets = play_targets.as_deref().unwrap_or_default();
            modes = play_modes.as_deref().unwrap_or_default();
        }
        ActionBody::Activate {
            ability,
            targets: activate_targets,
            modes: activate_modes,
            ..
        } => {
            if let (Some(ability), Some(name)) = (ability, source_name) {
                keys.push(format!("ability:{name}:{ability}"));
            }
            targets = activate_targets.as_deref().unwrap_or_default();
            modes = activate_modes.as_deref().unwrap_or_default();
        }
        ActionBody::ActivatePower {
            targets: power_targets,
            ..
        } => targets = power_targets.as_deref().unwrap_or_default(),
        ActionBody::Answer { selection, .. } => targets = selection,
        ActionBody::Attack { target_id, .. } => {
            match PLAYER_IDS
                .into_iter()
                .find(|player| *target_id == format!("hero-{}", player.as_str()))
            {
                Some(player) => keys.push(format!("aim:{kind}:hero.{}", side(seat, player))),
                None => aim_keys(
                    view,
                    seat,
                    kind,
                    &Selection::Instance {
                        instance_id: target_id.clone(),
                    },
                    &mut keys,
                ),
            }
        }
        _ => {}
    }
    for mode in modes {
        if find_instance(view, mode).is_some() {
            keys.push(format!("mode:{kind}:instance"));
        } else {
            keys.push(format!("mode:{kind}:{mode}"));
        }
    }
    for target in targets {
        aim_keys(view, seat, kind, target, &mut keys);
    }

    keys.sort();
    keys.dedup();
    keys
}

/// R1171000: one decision as `cargo jackioh positions --policy` records it: the keys of each move the
/// AI considers there, in its order, and which of them was played (`None` when it was none of them).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PolicyLabel {
    pub candidates: Vec<Vec<String>>,
    pub chosen: Option<usize>,
}

/// R1171000: the label of `played` at `state` for `seat`. The candidates are decide's own, listed on
/// its probe world (`AI_SEARCH.probe_seed`), so a replay of the log gives the same label; their keys
/// read `redact(state, seat)` alone. End Turn has no rank; every other move is ranked in that order.
pub fn policy_label(state: &GameState, seat: PlayerId, played: &ActionBody) -> PolicyLabel {
    let view = redact(state, seat);
    let probe = determinize(
        &view,
        seat,
        &mut Rng::new(AI_SEARCH.probe_seed, 0),
        DeterminizeOptions::default(),
    );
    let moves = candidate_actions(&probe, seat);
    let mut rank = 0;
    let candidates = moves
        .iter()
        .map(|action| {
            if matches!(action, ActionBody::EndTurn) {
                return policy_keys(&view, seat, action, None);
            }
            rank += 1;
            policy_keys(&view, seat, action, Some(rank - 1))
        })
        .collect();
    PolicyLabel {
        candidates,
        chosen: moves.iter().position(|action| action == played),
    }
}

/// R1171001: `moves` (non-End-Turn, in the heuristic order) sorted by `prior`, best first, equal
/// scores in their own order. Untouched with a table of no keys, or in a Duos duel.
pub fn prior_order(
    state: &GameState,
    seat: PlayerId,
    moves: Vec<ActionBody>,
    prior: &PolicyPrior<'_>,
) -> Vec<ActionBody> {
    if prior.keys.is_empty() || state.players[seat].duos.is_some() {
        return moves;
    }
    let view = redact(state, seat);
    let mut scored: Vec<(f64, ActionBody)> = moves
        .into_iter()
        .enumerate()
        .map(|(rank, action)| {
            (
                prior.score(&policy_keys(&view, seat, &action, Some(rank))),
                action,
            )
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));
    scored.into_iter().map(|(_, action)| action).collect()
}
