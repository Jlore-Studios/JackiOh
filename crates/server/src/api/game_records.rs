//! The live half of the card statistics (SPEC §9.11, R376): the record a finished match leaves.
//! `record_result` (`results.rs`) calls `record_finished_game` once a result has committed, with the
//! fold `read_finished` made before the result's transaction, which its quests read too (#2213). The
//! game half is read off `(seed, decks, log)` by the engine's `summarize_game` (SURFACE §11.3) and
//! filed under the match's mode (R257), the patch it started on (R388, R767000), each seat's pilot
//! (`human` even when the clock answered for one, R79; `ai` for a ladder bot, R1451, R1452; on the
//! seat its account plays as the game ends, R677) and `source: "live"`, apart from the AI's (R378).
//!
//! Telemetry never costs a result: it runs after the result's transaction, and every failure is
//! logged and swallowed. It is keyed by match id, so a result written twice files the game once; the
//! reaper's ceiling draws (R112) leave none. R1596000: a failed filing is made again by the retention
//! loop while the log is held. R1044000: the store folds each record into the totals in the same
//! transaction.

use jackioh_engine::{
    FoldArgs, FoldError, GameMode, GameRecord, GameSource, GameSummary, PerPlayer, Pilot, PlayerId,
    RecordBuild,
};
use serde_json::{Map, Value, json};

use crate::actor::engine;
use crate::actor::registry::pinned_for;
use crate::api::http::log_alert;
use crate::app::App;
use crate::bug_reports::{Finding, FoundIn, refused_rows};
use crate::config::STATS_FOLD_BATCH_RECORDS;
use crate::db::store::{
    Db, FoldedRecords, MatchActionRow, MatchRow, MatchSeat, Profile, ProfileKind, QueueMode, StoreError,
};

/// R1044000: folds every record not yet in the totals, `STATS_FOLD_BATCH_RECORDS` to a transaction.
/// The retention loop runs it before each purge, so a purge never meets a record it may not empty
/// (and `jackioh-server stats-fold` once after migration 0041). R1715000: a record whose detail does
/// not read is set aside, alerted (`game_records.fold_poison`) once its batch has committed.
pub async fn fold_pending(db: &Db) -> Result<FoldedRecords, StoreError> {
    let mut done = FoldedRecords::default();
    loop {
        let mut tx = db.begin(None).await?;
        let batch = tx.game_records_fold_pending(STATS_FOLD_BATCH_RECORDS).await?;
        tx.commit().await?;
        for record in &batch.unreadable {
            log_alert(
                "game_records.fold_poison",
                json!({ "recordId": record.id, "message": record.message }),
            );
        }
        let taken = batch.folded + batch.unreadable.len();
        done.folded += batch.folded;
        done.unreadable.extend(batch.unreadable);
        if taken < STATS_FOLD_BATCH_RECORDS {
            return Ok(done);
        }
    }
}

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
pub(crate) fn fold_args_of(row: &MatchRow, log: Vec<Value>) -> Result<FoldArgs, serde_json::Error> {
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

/// #2213: what one fold of a finished match's log answers: its summary, or `None` when the log
/// reaches no result, and then the actions the fold refused (empty when it did reach one).
pub(crate) struct Folded {
    pub game: Option<GameSummary>,
    pub refused: Vec<FoldError>,
}

/// #2213: a finished match as `record_result` reads it before its transaction: the row, the log and
/// the log's one fold, which its quests and its game record both read. A finished match's log is
/// final: it is append-only, and `MatchActor::apply_action` refuses any action once the game has a
/// result. An `Err` is the fold's failure (an unreadable log, a panic), as each reader logs it.
pub(crate) struct Finished {
    pub row: MatchRow,
    pub log: Vec<MatchActionRow>,
    pub folded: Result<Folded, String>,
}

/// #2213: reads match `match_id` and its log in a transaction of its own, and folds the log once off
/// the runtime, with no transaction open: so a fold, however long, holds no lock and no connection.
/// `None` when the match has no row.
pub(crate) async fn read_finished(app: &App, match_id: &str) -> Result<Option<Finished>, StoreError> {
    let mut tx = app.db.begin(None).await?;
    let Some(row) = tx.matches_get(match_id).await? else {
        tx.commit().await?;
        return Ok(None);
    };
    let log = tx.matches_actions(match_id).await?;
    tx.commit().await?;
    let folded = fold_log(app, &row, &log, false).await.map(|(folded, _)| folded);
    Ok(Some(Finished { row, log, folded }))
}

/// #2213: one fold of a match's log, off the runtime (#1035): the summary and, for a log that reaches
/// no result, the actions it refuses. `read_swap` is asked only by the refile path, which has no
/// credited seats to read a Glitch's seat swap from (R677). The engine panics on decks it refuses,
/// and the panic is the `Err`.
async fn fold_log(
    app: &App,
    row: &MatchRow,
    log: &[MatchActionRow],
    read_swap: bool,
) -> Result<(Folded, bool), String> {
    let mut entries = Vec::with_capacity(log.len());
    for entry in log {
        entries.push(serde_json::to_value(&entry.action).map_err(message_of)?);
    }
    let args = fold_args_of(row, entries).map_err(message_of)?;
    let pinned = pinned_for(row);
    engine::off_runtime(app.engine_thread, move || {
        let game = engine::summarize_game(pinned, &args);
        let swapped =
            game.is_some() && read_swap && engine::snapshot(&engine::fold(pinned, &args).state).seats_swapped;
        let refused = if game.is_none() {
            engine::fold(pinned, &args).errors
        } else {
            Vec::new()
        };
        (Folded { game, refused }, swapped)
    })
    .await
}

/// R376, R1451: a person's seat is `human`; a ladder bot's is `ai`, so no human figure reads it
/// unasked.
fn pilot_of(profiles: &[Profile], profile_id: &str) -> Pilot {
    let bot = profiles
        .iter()
        .any(|profile| profile.id == profile_id && profile.kind == ProfileKind::Bot);
    if bot { Pilot::Ai } else { Pilot::Human }
}

/// Logs why a match is not filed: it has no row, or no series, room or queue ticket made it.
fn skipped(match_id: &str, found: bool) {
    let reason = if found {
        "no series, room or queue ticket made the match"
    } else {
        "no match row"
    };
    tracing::warn!(event = "game.record.skipped", matchId = %match_id, reason = %reason);
}

/// Files one finished match for the card statistics: the record it wrote, or `None` when no series,
/// room or ticket made the match, the log reaches no result, the record exists already, or
/// something failed (logged, except the existing record).
pub async fn record_live_game(app: &App, match_id: &str) -> Option<GameRecord> {
    file_live_game(app, match_id, app.env.deployed_commit.as_deref()).await
}

/// R1596000: `record_live_game` for the retention loop's refile (`retention::refile_unfiled`). The
/// filing server may be a later deploy than the one that played, so no build (R1367000).
pub async fn refile_live_game(app: &App, match_id: &str) -> Option<GameRecord> {
    file_live_game(app, match_id, None).await
}

/// The filing's body; an `Err` is the failure to log.
async fn file_live_game(app: &App, match_id: &str, played_on: Option<&str>) -> Option<GameRecord> {
    match record_live_game_inner(app, match_id, played_on).await {
        Ok(record) => record,
        Err(message) => {
            tracing::error!(event = "game.record.failed", matchId = %match_id, message = %message);
            None
        }
    }
}

/// `file_live_game`'s body, the refile path (R1596000) and `record_live_game`'s: it reads the match
/// and folds its log for itself; an `Err` is the failure to log.
async fn record_live_game_inner(
    app: &App,
    match_id: &str,
    played_on: Option<&str>,
) -> Result<Option<GameRecord>, String> {
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
            skipped(match_id, found.is_some());
            return Ok(None);
        }
    };

    let log = tx.matches_actions(match_id).await.map_err(message_of)?;
    // R376, R1451: a person's seat is `human`; a ladder bot's is `ai`, so no human figure reads it
    // unasked.
    let profiles = tx
        .profiles_get_many(&[row.players.0.clone(), row.players.1.clone()])
        .await
        .map_err(message_of)?;
    tx.commit().await.map_err(message_of)?;
    let began = (
        pilot_of(&profiles, &row.players.0),
        pilot_of(&profiles, &row.players.1),
    );
    // R677, R1451: each seat is filed under the account that plays it as the game ends, as the
    // result credits it, so after a Glitch's seat swap a bot's `ai` goes on the seat it ended in. Only a
    // game with a bot in it can tell, so only it folds the log again to read the swap (#1035).
    let (folded, swapped) = fold_log(app, &row, &log, began.0 != began.1).await?;
    let pilots = if swapped {
        PerPlayer::new(began.1, began.0)
    } else {
        PerPlayer::new(began.0, began.1)
    };
    file_game(app, &row, &log, folded, mode, pilots, played_on).await
}

/// #2213: `record_result`'s filing of a finished match, once its result has committed: the fold is
/// `read_finished`'s, made before the result's transaction, so the filing folds nothing. `finished`
/// is `None` when that read found no match row. Each seat is filed under the account the result
/// credits it to (`seats`, R677, R1451). Answers like `record_live_game`.
pub(crate) async fn record_finished_game(
    app: &App,
    match_id: &str,
    finished: Option<Finished>,
    seats: &(MatchSeat, MatchSeat),
) -> Option<GameRecord> {
    match record_finished_game_inner(app, match_id, finished, seats).await {
        Ok(record) => record,
        Err(message) => {
            tracing::error!(event = "game.record.failed", matchId = %match_id, message = %message);
            None
        }
    }
}

/// `record_finished_game`'s body; an `Err` is the failure to log.
async fn record_finished_game_inner(
    app: &App,
    match_id: &str,
    finished: Option<Finished>,
    seats: &(MatchSeat, MatchSeat),
) -> Result<Option<GameRecord>, String> {
    let mut tx = app.db.begin(None).await.map_err(message_of)?;
    let mode = match &finished {
        None => None,
        Some(_) => tx.matches_mode_of(match_id).await.map_err(message_of)?,
    };
    let (finished, mode) = match (finished, mode) {
        (Some(finished), Some(mode)) => (finished, mode),
        (finished, _) => {
            tx.commit().await.map_err(message_of)?;
            skipped(match_id, finished.is_some());
            return Ok(None);
        }
    };
    let profiles = tx
        .profiles_get_many(&[seats.0.profile_id.clone(), seats.1.profile_id.clone()])
        .await
        .map_err(message_of)?;
    tx.commit().await.map_err(message_of)?;
    let Finished { row, log, folded } = finished;
    let folded = folded?;
    // R677, R1451: each seat under the account the result credits it to, so after a Glitch's seat swap a
    // bot's `ai` goes on the seat it ended in, with no fold to read the swap.
    let pilot_on = |player: PlayerId| {
        let seat = if seats.0.player == player {
            &seats.0
        } else {
            &seats.1
        };
        pilot_of(&profiles, &seat.profile_id)
    };
    let pilots = PerPlayer::new(pilot_on(PlayerId::P1), pilot_on(PlayerId::P2));
    file_game(
        app,
        &row,
        &log,
        folded,
        mode,
        pilots,
        app.env.deployed_commit.as_deref(),
    )
    .await
}

/// Files the record of a folded match: the shared end of both paths. A fold that reaches no result is
/// a determinism break and files a bug report instead; the record exists already when the store
/// refuses it (`None`).
async fn file_game(
    app: &App,
    row: &MatchRow,
    log: &[MatchActionRow],
    folded: Folded,
    mode: QueueMode,
    pilots: PerPlayer<Pilot>,
    played_on: Option<&str>,
) -> Result<Option<GameRecord>, String> {
    let Some(game) = folded.game else {
        let actions = log.len();
        // A fold that does not reach the result is a determinism break (§9.3); say so loudly, and file
        // it for the night bot (`bug_reports.rs`).
        tracing::error!(event = "game.record.unfinished", matchId = %row.id, actions = actions);
        app.bug_reports.file(
            app.env.deployed_commit.as_deref(),
            Finding {
                match_id: row.id.clone(),
                found_in: FoundIn::FinishedLog,
                catalog_version: row.catalog_version.clone(),
                actions,
                refused: refused_rows(log, &folded.refused),
            },
        );
        return Ok(None);
    };

    let record = GameRecord {
        id: row.id.clone(),
        source: GameSource::Live,
        mode: game_mode_of(mode),
        patch: row.catalog_version.clone(),
        // R1367000: the commit of the deploy that played it (none when filed again, R1596000), and its AI generation when a ladder bot played a seat.
        build: RecordBuild::of(played_on, jackioh_ai::ai_generation(), &pilots),
        pilots,
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
        matchId = %row.id,
        mode = %mode.as_str(),
        patch = %record.patch,
    );
    Ok(Some(record))
}
