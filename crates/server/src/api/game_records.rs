//! The live half of the card statistics (SPEC §9.11, R376): the record a finished match leaves.
//!
//! `record_result` (`results.rs`) calls `record_live_game` once a result has committed. The game half
//! (decklists, opening hands, draws, plays, first player, winner) is read off `(seed, decks, log)` by
//! the engine's `summarize_game`, called directly (SURFACE §11.3). It is filed under:
//!  - the match's mode (R257), read off its series, room or queue tickets (`matches_mode_of`);
//!  - the patch the build's cards are (R388, `jackioh_cards::catalog_version()`);
//!  - two human pilots, even when the clock answered a prompt for one (R79's `timeout`);
//!  - `source: "live"`, apart from the AI's development runs (R378).
//!
//! Telemetry never costs a result: it runs after the result's transaction and every failure is logged
//! and swallowed. It is idempotent (keyed by match id; the store refuses a second), so a result
//! written twice files the game once. The reaper's ceiling draws (R112) leave none: no game to read.

use jackioh_engine::{FoldArgs, GameMode, GameRecord, GameSource, PerPlayer, Pilot};
use serde_json::{Map, Value, json};

use crate::app::App;
use crate::db::store::{MatchRow, QueueMode};

/// R376: both seats of a live match are players.
const LIVE_PILOTS: PerPlayer<Pilot> = PerPlayer {
    p1: Pilot::Human,
    p2: Pilot::Human,
};

/// An error as the sentence a log line carries.
fn message_of(error: impl std::fmt::Display) -> String {
    error.to_string()
}

/// R257's mode as the record files it: `QueueMode` and the wire's `GameMode` are the same three
/// literals (`bo1`, `bo3`, `random`).
fn game_mode_of(mode: QueueMode) -> GameMode {
    match mode {
        QueueMode::Bo1 => GameMode::Bo1,
        QueueMode::Bo3 => GameMode::Bo3,
        QueueMode::Random => GameMode::Random,
    }
}

/// The fold's input for a finished match: its seed, decks and log, and (R417, R678) the frozen
/// boards it was created with, so the fold reads them as the live game did.
fn fold_args_of(row: &MatchRow, log: Vec<Value>) -> Result<FoldArgs, serde_json::Error> {
    let mut args = Map::new();
    args.insert("seed".into(), json!(row.seed));
    args.insert("decks".into(), serde_json::to_value(&row.decks)?);
    args.insert("log".into(), Value::Array(log));
    // R417, R678: a match's frozen boards are setup, so the fold reads them as the live game did.
    if let Some(last_boards) = &row.last_boards {
        args.insert("lastBoards".into(), serde_json::to_value(last_boards)?);
    }
    if let Some(glitch_boards) = &row.glitch_boards {
        args.insert("glitchBoards".into(), serde_json::to_value(glitch_boards)?);
    }
    serde_json::from_value(Value::Object(args))
}

/// Files one finished match for the card statistics. Answers the record it wrote, or `None` when
/// it wrote none: no series, room or ticket made the match, the log does not reach a result, the
/// record exists already, or something failed (logged, except the existing record). Until migration
/// 0014 is applied the insert fails, which is logged and costs nothing else.
pub async fn record_live_game(app: &App, match_id: &str) -> Option<GameRecord> {
    match record_live_game_inner(app, match_id).await {
        Ok(record) => record,
        Err(message) => {
            tracing::error!(event = "game.record.failed", matchId = %match_id, message = %message);
            None
        }
    }
}

/// `record_live_game`'s body; an `Err` is the failure to log.
async fn record_live_game_inner(app: &App, match_id: &str) -> Result<Option<GameRecord>, String> {
    let mut tx = app.db.begin(None).await.map_err(message_of)?;
    let found = tx.matches_get(match_id).await.map_err(message_of)?;
    let mode = match &found {
        None => None,
        Some(_) => tx.matches_mode_of(match_id).await.map_err(message_of)?,
    };
    let (row, mode) = match (found, mode) {
        (Some(row), Some(mode)) => (row, mode),
        (found, _) => {
            tx.commit().await.map_err(message_of)?;
            let reason = if found.is_none() {
                "no match row"
            } else {
                "no series, room or queue ticket made the match"
            };
            tracing::warn!(event = "game.record.skipped", matchId = %match_id, reason = %reason);
            return Ok(None);
        }
    };

    let log = tx.matches_actions(match_id).await.map_err(message_of)?;
    tx.commit().await.map_err(message_of)?;
    let actions = log.len();
    let mut entries = Vec::with_capacity(actions);
    for entry in &log {
        entries.push(serde_json::to_value(&entry.action).map_err(message_of)?);
    }
    let args = fold_args_of(&row, entries).map_err(message_of)?;
    // The engine panics on decks it refuses; the panic is that error (as `actor::registry` reads it).
    let summarized = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        jackioh_engine::summarize_game(&args)
    }))
    .map_err(crate::actor::registry::panic_text)?;
    let Some(game) = summarized else {
        // A fold that does not reach the result is a determinism break (§9.3); say so loudly.
        tracing::error!(event = "game.record.unfinished", matchId = %match_id, actions = actions);
        return Ok(None);
    };

    let record = GameRecord {
        id: match_id.to_string(),
        source: GameSource::Live,
        mode: game_mode_of(mode),
        patch: jackioh_cards::catalog_version().to_string(),
        pilots: LIVE_PILOTS,
        game,
    };
    let mut tx = app.db.begin(None).await.map_err(message_of)?;
    let inserted = tx.game_records_insert(&record).await.map_err(message_of)?;
    tx.commit().await.map_err(message_of)?;
    if !inserted {
        return Ok(None);
    }
    tracing::info!(
        event = "game.recorded",
        matchId = %match_id,
        mode = %mode.as_str(),
        patch = %record.patch,
    );
    Ok(Some(record))
}
