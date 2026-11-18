//! #4169: a live co-op or 2v2 match's checkpoint (SPEC §9.3, §13.9), so a rebuild folds only the log's
//! tail, as a 1v1 match's does (#1169, `actor::checkpoint`).
//!
//! When the restart drain stops a live match's actor (`DuosRegistry::stop_all`), and after every
//! `MATCH_CHECKPOINT_MIN_ACTIONS`-th row it logs while the match is played, the actor stores the
//! `DuosState` it holds after the log's newest row (`DuosActor::checkpoint`) as one JSON `body`, in
//! `public.duos_match_checkpoints` (migration 0096). The seq counter and the nonce map are the log's,
//! and the AI seats' cursors start afresh, as a full fold's do, so the state is all it holds.
//!
//! The next rebuild (`DuosRegistry::rebuild`) resumes from a checkpoint that passes every check a 1v1
//! checkpoint passes (`actor::checkpoint::verify_body`, the one function both call), and folds only
//! the rows after it (`engine::fold_tail`). On refusal, the whole log is folded; no replay (B41) reads a
//! checkpoint. A refusal logs its reason, never the body, which holds all four hands.

use serde::{Deserialize, Serialize};

use jackioh_engine::duos::DuosState;

use crate::actor::checkpoint::{Refusal, row_at, row_for, verify_body};
use crate::db::store::MatchCheckpointRow;
use crate::duos::store::{DuosActionRow, DuosMatch};

/// What a co-op checkpoint holds: the actor's game after one log row.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DuosCheckpointBody {
    pub state: DuosState,
}

/// The row that stores `body` as the state after the log row `seq`, logged under `nonce`.
pub fn checkpoint_row(
    row: &DuosMatch,
    seq: i64,
    nonce: &str,
    body: &DuosCheckpointBody,
    at: i64,
) -> Result<MatchCheckpointRow, serde_json::Error> {
    row_for(&row.id, &row.options.catalog_version, seq, nonce, body, at)
}

/// The body of `checkpoint` when it may be resumed for the match `row` with the log `log` (seq order,
/// as `duos_actions` reads it), or the first check it fails.
pub fn verify(
    checkpoint: &MatchCheckpointRow,
    row: &DuosMatch,
    log: &[DuosActionRow],
) -> Result<DuosCheckpointBody, Refusal> {
    verify_body(
        checkpoint,
        &row.options.catalog_version,
        row_at(log, checkpoint.seq).map(|logged| (logged.seq, logged.nonce.as_str())),
    )
}
