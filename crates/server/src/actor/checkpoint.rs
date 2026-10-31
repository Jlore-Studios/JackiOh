//! #1169: a live match's checkpoint (SPEC §9.3, §9.5), so a rebuild folds only the log's tail.
//!
//! When the restart drain stops a live match's actor (`Registry::stop_all`), the actor stores what it
//! holds after the log's newest row (`MatchActor::checkpoint`): the engine state, the play telemetry's
//! recorder (R1442) and the turn each bot last moved in (R639000), as one JSON `body`.
//!
//! The next rebuild (`Registry::rebuild`) resumes all three of its folds from a checkpoint that
//! passes every check in `verify`, and folds only rows after it. On refusal, the whole log is
//! folded; no replay (R768) reads a checkpoint. A refusal logs its reason, never the body.

use serde::{Deserialize, Serialize};

use jackioh_engine::{FoldError, PerPlayer, fnv1a32_utf16};

use crate::actor::engine::{self, EngineState, FoldResult, Pinned};
use crate::actor::telemetry::Recorder;
use crate::config::MATCH_CHECKPOINT_VERSION;
use crate::db::store::{MatchActionRow, MatchCheckpointRow, MatchRow};

/// The rules this build folds a match with (`build.rs`): a checkpoint written under others is refused.
pub const RULES_FINGERPRINT: &str = env!("JACKIOH_RULES_FINGERPRINT");

/// What a checkpoint holds: an actor's state, recorder and bot turns after one log row.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointBody {
    pub state: EngineState,
    pub telemetry: Recorder,
    /// By home seat, as `MatchActorInput::bot_moved_turn`.
    pub bot_moved_turn: PerPlayer<Option<i32>>,
}

/// The row that stores `body` as the state after the log row `seq`, logged under `nonce`.
pub fn checkpoint_row(
    match_row: &MatchRow,
    seq: i64,
    nonce: &str,
    body: &CheckpointBody,
    at: i64,
) -> Result<MatchCheckpointRow, serde_json::Error> {
    let body = serde_json::to_string(body)?;
    Ok(MatchCheckpointRow {
        match_id: match_row.id.clone(),
        seq,
        nonce: nonce.to_string(),
        catalog_version: match_row.catalog_version.clone(),
        rules: RULES_FINGERPRINT.to_string(),
        checkpoint_version: MATCH_CHECKPOINT_VERSION,
        body_hash: fnv1a32_utf16(&body),
        body,
        at,
    })
}

/// Why a checkpoint was refused, in the order `verify` checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    Version,
    Rules,
    Catalog,
    Seq,
    Nonce,
    Hash,
    Body,
    /// Its body reads, but does not write back to the same text: it holds something this build drops.
    Shape,
}

impl Refusal {
    pub fn as_str(self) -> &'static str {
        match self {
            Refusal::Version => "version",
            Refusal::Rules => "rules",
            Refusal::Catalog => "catalog",
            Refusal::Seq => "seq",
            Refusal::Nonce => "nonce",
            Refusal::Hash => "hash",
            Refusal::Body => "body",
            Refusal::Shape => "shape",
        }
    }
}

/// The body of `checkpoint` when it may be resumed for the match `match_row` with the log `log` (seq
/// order, as `matches_actions` reads it), or the first check it fails.
pub fn verify(
    checkpoint: &MatchCheckpointRow,
    match_row: &MatchRow,
    log: &[MatchActionRow],
) -> Result<CheckpointBody, Refusal> {
    if checkpoint.checkpoint_version != MATCH_CHECKPOINT_VERSION {
        return Err(Refusal::Version);
    }
    if checkpoint.rules != RULES_FINGERPRINT {
        return Err(Refusal::Rules);
    }
    if checkpoint.catalog_version != match_row.catalog_version {
        return Err(Refusal::Catalog);
    }
    let row = usize::try_from(checkpoint.seq.saturating_sub(1))
        .ok()
        .and_then(|index| log.get(index))
        .filter(|row| row.seq == checkpoint.seq)
        .ok_or(Refusal::Seq)?;
    if row.action.nonce != checkpoint.nonce {
        return Err(Refusal::Nonce);
    }
    if fnv1a32_utf16(&checkpoint.body) != checkpoint.body_hash {
        return Err(Refusal::Hash);
    }
    let body: CheckpointBody = serde_json::from_str(&checkpoint.body).map_err(|_| Refusal::Body)?;
    if serde_json::to_string(&body).ok().as_deref() != Some(checkpoint.body.as_str()) {
        return Err(Refusal::Shape);
    }
    Ok(body)
}

/// `engine::fold` carried on from `state` over `tail`: each row reduced in turn, a row the engine
/// refuses recorded and skipped, as `fold` records and skips it.
pub fn fold_tail(pinned: Pinned, mut state: EngineState, tail: &[MatchActionRow]) -> FoldResult {
    let mut errors = Vec::new();
    for row in tail {
        let result = engine::reduce(pinned, &state, &row.action);
        if let Some(error) = result.error {
            errors.push(FoldError {
                nonce: row.action.nonce.clone(),
                error,
            });
            continue;
        }
        state = result.state;
    }
    FoldResult { state, errors }
}
