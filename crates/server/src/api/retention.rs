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
//!  - a chat message, with its reports, `CHAT_RETENTION_DAYS` after it was sent (SPEC S7, B53;
//!    `social::chat::purge`, its own transaction after the others).
//!  - a finished co-op match's action log `MATCH_ACTION_RETENTION_DAYS` after the match ended, as a
//!    1v1 match's (§13.9; `duos::purge_expired_logs`, its own transaction). Its row, the seed, the
//!    decks and the result, is kept, as a 1v1 match's is.
//!  - a game record's detail `GAME_RECORD_RAW_RETENTION_DAYS` after it was written, once it is folded
//!    into the card statistics' totals (R1044001). Its id, source, mode, patch and time are kept. The
//!    loop folds any record written before the totals first (`game_records::fold_pending`).
//!
//! `app.rs` spawns `run_purge`, which runs it at boot and then every
//! `RETENTION_PURGE_INTERVAL_SECONDS`, next to the match reaper. In Postgres it is one call to
//! `app.purge_expired_rows` (migration 0013, with 0029's third cutoff and 0041's fourth).
//!
//! Before it purges, the loop files again any finished match's game record whose write failed (R1596000,
//! `refile_unfiled`), so the purge never takes the log of a game the card statistics lack.

use std::sync::Arc;
use std::time::Duration;

use serde_json::json;

use crate::api::game_records::{fold_pending, refile_live_game};
use crate::api::http::{log_info, log_warn, now_ms};
use crate::app::App;
use crate::config::{
    CODE_ATTEMPT_RETENTION_DAYS, GAME_RECORD_RAW_RETENTION_DAYS, GAME_RECORD_REFILE_GRACE_SECONDS,
    GAME_RECORD_REFILE_WINDOW_SECONDS, MATCH_ACTION_RETENTION_DAYS, PLAY_TELEMETRY_RETENTION_DAYS,
    RETENTION_PURGE_INTERVAL_SECONDS,
};
use crate::db::store::{RetentionPurgeInput, RetentionPurgeResult, StoreError};

/// Unit conversion, not configuration.
const MS_PER_DAY: i64 = 86_400_000;
/// Unit conversion, not configuration.
const MS_PER_SECOND: i64 = 1_000;
/// The epoch: a range's start every match ended after.
const EPOCH_MS: i64 = 0;

pub async fn purge_expired(app: &App) -> Result<RetentionPurgeResult, StoreError> {
    let now = now_ms();
    let mut tx = app.db.begin(None).await?;
    let purged = tx
        .purge_expired(&RetentionPurgeInput {
            code_attempts_before: now - CODE_ATTEMPT_RETENTION_DAYS * MS_PER_DAY,
            match_actions_ended_before: now - MATCH_ACTION_RETENTION_DAYS * MS_PER_DAY,
            play_telemetry_ended_before: now - PLAY_TELEMETRY_RETENTION_DAYS * MS_PER_DAY,
            game_records_recorded_before: now - GAME_RECORD_RAW_RETENTION_DAYS * MS_PER_DAY,
        })
        .await?;
    tx.commit().await?;
    Ok(purged)
}

/// R1596000: refiles every finished match whose record never landed, while its log is held: each
/// pass, those that ended inside the window but past the grace, and, a last time, those whose log
/// the purge takes before its next pass. Answers how many it wrote.
pub async fn refile_unfiled(app: &App) -> Result<usize, StoreError> {
    let now = now_ms();
    let ranges = [
        (
            now - GAME_RECORD_REFILE_WINDOW_SECONDS * MS_PER_SECOND,
            now - GAME_RECORD_REFILE_GRACE_SECONDS * MS_PER_SECOND,
        ),
        (
            EPOCH_MS,
            now - MATCH_ACTION_RETENTION_DAYS * MS_PER_DAY + RETENTION_PURGE_INTERVAL_SECONDS * MS_PER_SECOND,
        ),
    ];
    let mut filed = 0;
    for (ended_after, ended_before) in ranges {
        let mut tx = app.db.begin(None).await?;
        let unfiled = tx.game_records_unfiled(ended_after, ended_before).await?;
        tx.commit().await?;
        for match_id in unfiled {
            if refile_live_game(app, &match_id).await.is_some() {
                filed += 1;
            }
        }
    }
    Ok(filed)
}

/// The retention purge's loop: once now, since a free instance may sleep before an hour is up, and
/// then on its interval. A failure is logged and the next run tries again. Never returns; `app.rs`
/// spawns it.
pub async fn run_purge(app: Arc<App>) {
    loop {
        // R1044000: the records written before the totals are folded first, in transactions of their
        // own, so their detail can go on the purge's cutoff like any other's.
        match fold_pending(&app.db).await {
            Ok(0) => {}
            Ok(folded) => log_info("game_records.folded", json!({ "records": folded })),
            Err(error) => log_warn(
                "game_records.fold_failed",
                json!({ "message": error.to_string() }),
            ),
        }
        // R1596000: a record whose filing failed is filed again before the purge below can take its log.
        match refile_unfiled(&app).await {
            Ok(0) => {}
            Ok(filed) => log_info("game_records.refiled", json!({ "records": filed })),
            Err(error) => log_warn(
                "game_records.refile_failed",
                json!({ "message": error.to_string() }),
            ),
        }
        match purge_expired(&app).await {
            Ok(purged) => {
                if purged.code_attempts + purged.match_actions + purged.play_telemetry + purged.game_records
                    > 0
                {
                    log_info(
                        "retention.purged",
                        json!({
                            "codeAttempts": purged.code_attempts,
                            "matchActions": purged.match_actions,
                            "playTelemetry": purged.play_telemetry,
                            "gameRecords": purged.game_records,
                        }),
                    );
                }
            }
            Err(error) => log_warn("retention.purge_failed", json!({ "message": error.to_string() })),
        }
        // Play together (#941): B53's chat retention, apart, so one failure never holds the other.
        match crate::social::chat::purge(&app).await {
            Ok(0) => {}
            Ok(purged) => log_info("chat.purged", json!({ "messages": purged })),
            Err(error) => log_warn("chat.purge_failed", json!({ "message": error.to_string() })),
        }
        // Duos co-op (#936): the co-op logs, apart too.
        match crate::duos::purge_expired_logs(&app).await {
            Ok(0) => {}
            Ok(purged) => log_info("duos.logs.purged", json!({ "actions": purged })),
            Err(error) => log_warn("duos.logs.purge_failed", json!({ "message": error.to_string() })),
        }
        tokio::time::sleep(Duration::from_secs(RETENTION_PURGE_INTERVAL_SECONDS as u64)).await;
    }
}
