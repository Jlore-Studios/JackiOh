//! The live half of the card statistics (SPEC §9.11, R376): the record a finished match leaves.
//! Port of `apps/server/src/api/game-records.ts`.
//!
//! `record_result` (`results.rs`) calls `record_live_game` once a match's result has committed.
//! The record's game half — each seat's decklist, opening hand, draws and plays, who went first and
//! who won — is read off `(seed, decks, log)` by the engine's `summarize_game`, which the server
//! calls directly (SURFACE §11.3: no engine port). It is filed under:
//!
//!  - the match's mode (R257), read off what made it: its series, its room or its queue tickets
//!    (`matches_mode_of`), so the match row and the path that starts a match are unchanged;
//!  - the patch the build's cards are (R388's newest patch, `jackioh_cards::catalog_version()`);
//!  - two human pilots: a live match is two players, and a prompt the clock answered for one of them
//!    (R79's `timeout`) is still their game;
//!  - `source: "live"`, which is what keeps it apart from the AI's development runs (R378).
//!
//! It is telemetry, so it never costs a result: it runs after the result's transaction, and every
//! failure is logged and swallowed. It is idempotent: the record is keyed by the match id and the
//! store refuses a second one, so a result written twice (an actor healing itself after a restart)
//! files the game once. The reaper's ceiling draws (R112) leave none: the reaper reaches a match
//! whose actor stopped answering, and writes its result without a game to read.

use jackioh_engine::{FoldArgs, GameMode, GameRecord, GameSource, Pilot, PerPlayer};
use serde_json::{Map, Value, json};

use crate::app::App;
use crate::db::store::{MatchRow, QueueMode};

/// R376: both seats of a live match are players.
const LIVE_PILOTS: PerPlayer<Pilot> = PerPlayer { p1: Pilot::Human, p2: Pilot::Human };

/// TS `messageOf(error)`: an error as the sentence a log line carries.
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
/// record exists already, or something failed — each but the third logged. Until migration 0014 is
/// applied the insert fails, which is logged and costs nothing else: no other path reads or writes
/// the table.
///
/// TS returned null at once when no recorder was bound (`deps.games` absent, as in most of its
/// tests); the Rust server always has the engine, so the recorder is always bound.
pub async fn record_live_game(app: &App, match_id: &str) -> Option<GameRecord> {
    match record_live_game_inner(app, match_id).await {
        Ok(record) => record,
        Err(message) => {
            tracing::error!(event = "game.record.failed", matchId = %match_id, message = %message);
            None
        }
    }
}

/// `record_live_game`'s body; an `Err` is the failure TS's `catch` logged.
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
            let reason = if found.is_none() { "no match row" } else { "no series, room or queue ticket made the match" };
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
    // TS's `createGame` threw on decks it refused, inside this function's `try`; the engine panics,
    // and the panic is that error (as `actor::registry` reads it at a start).
    let summarized = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| jackioh_engine::summarize_game(&args)))
        .map_err(crate::actor::registry::panic_text)?;
    let Some(game) = summarized else {
        // The result came from this log's own last action, so a fold that does not reach it is a
        // determinism break (§9.3), and says so as loudly as the registry's fold errors do.
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
