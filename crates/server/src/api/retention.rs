//! The retention purge: rows nothing reads any more are deleted once they are old, so the privacy
//! policy can state how long each is kept. Not in SPEC, and no R-row. Port of
//! `apps/server/src/api/retention.ts`, plus the purge loop `src/index.ts` ran (SURFACE §11.2:
//! `api::retention::run_purge(app)`, boot + hourly).
//!
//!  - `code_attempts` (profile id, peppered IP hash, time) after `CODE_ATTEMPT_RETENTION_DAYS`. The
//!    limits that read them look back `CODE_ATTEMPT_WINDOW_SECONDS` at most.
//!  - a finished match's action log `MATCH_ACTION_RETENTION_DAYS` after the match ended. Only a live
//!    match is replayed from its log; the result row, and so the rating history, is kept.
//!  - a finished match's play telemetry `PLAY_TELEMETRY_RETENTION_DAYS` after the match ended (R1442).
//!
//! `app.rs` spawns `run_purge`, which runs it at boot and then every
//! `RETENTION_PURGE_INTERVAL_SECONDS`, next to the match reaper. In Postgres it is one call to
//! `app.purge_expired_rows` (migration 0013, with 0029's third cutoff).

use std::sync::Arc;
use std::time::Duration;

use serde_json::json;

use crate::api::http::{log_info, log_warn, now_ms};
use crate::app::App;
use crate::config::{
    CODE_ATTEMPT_RETENTION_DAYS, MATCH_ACTION_RETENTION_DAYS, PLAY_TELEMETRY_RETENTION_DAYS,
    RETENTION_PURGE_INTERVAL_SECONDS,
};
use crate::db::store::{RetentionPurgeInput, RetentionPurgeResult, StoreError};

/// Unit conversion, not configuration.
const MS_PER_DAY: i64 = 86_400_000;

pub async fn purge_expired(app: &App) -> Result<RetentionPurgeResult, StoreError> {
    let now = now_ms();
    let mut tx = app.db.begin(None).await?;
    let purged = tx
        .purge_expired(&RetentionPurgeInput {
            code_attempts_before: now - CODE_ATTEMPT_RETENTION_DAYS * MS_PER_DAY,
            match_actions_ended_before: now - MATCH_ACTION_RETENTION_DAYS * MS_PER_DAY,
            play_telemetry_ended_before: now - PLAY_TELEMETRY_RETENTION_DAYS * MS_PER_DAY,
        })
        .await?;
    tx.commit().await?;
    Ok(purged)
}

/// The retention purge's loop: once now, since a free instance may sleep before an hour is up, and
/// then on its interval. A failure is logged and the next run tries again. Never returns; `app.rs`
/// spawns it.
pub async fn run_purge(app: Arc<App>) {
    loop {
        match purge_expired(&app).await {
            Ok(purged) => {
                if purged.code_attempts + purged.match_actions + purged.play_telemetry > 0 {
                    log_info(
                        "retention.purged",
                        json!({
                            "codeAttempts": purged.code_attempts,
                            "matchActions": purged.match_actions,
                            "playTelemetry": purged.play_telemetry,
                        }),
                    );
                }
            }
            Err(error) => log_warn("retention.purge_failed", json!({ "message": error.to_string() })),
        }
        tokio::time::sleep(Duration::from_secs(RETENTION_PURGE_INTERVAL_SECONDS as u64)).await;
    }
}
