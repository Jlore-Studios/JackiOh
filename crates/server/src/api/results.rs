//! Results and rating (BUILD M7-T2, SPEC §2.5, §9.5, §9.12, R79, R112, R603, R604).
//! Port of `apps/server/src/api/results.ts`.
//!
//! §9.5: "Every ending records a result and clears both players' in-match state, and a reaper
//! resolves anything past the ceiling." §9.12: a ranked game moves both players' hidden Glicko-2
//! ratings and their ranks on the ladder.
//!
//! One function is that sentence: `record_result` is the `RecordResult` port the actor calls for
//! every one of §2.5's seven endings — `hero-death`, `both-heroes-dead`, `concede`,
//! `draw-accepted`, `turn-cap`, `disconnect` and `match-ceiling` — and it writes exactly one
//! `results` row, rates both players when the match is ranked (R604) and clears both in-match
//! flags in one transaction. (TS built it with the factory `createRecordResult(deps)`; with no
//! ports in the Rust server it is the function itself, and `void_match` is `createVoidMatch`'s.)
//!
//! It is idempotent by design, not by luck: the actor and the reaper can both reach the same
//! terminal match (a crashed actor is exactly the case §9.5's reaper exists for), so the first
//! thing the transaction does is look for the row it is about to write.
//!
//! The rating maths is `ranked/glicko2.rs` and the ladder `ranked/ladder.rs`, reached through
//! `rate_ranked_game` (`api/ranked.rs`), which also writes the record of the rated game (R611). This
//! file only decides which side won: §2.5's draws are a draw (0.5 each), and a concede and a
//! disconnect are a loss like any other.
//!
//! A room challenge is unranked (R604): its row records both ratings unchanged.
//!
//! A game of a Conquest series is written the same way with one difference and one addition
//! (R262, R263): its row leaves both ratings unchanged, because a series moves the rating once, when it
//! ends; and the series' record of the game — and, when the game ends the series, that one rating
//! move — commit in the same transaction as the row (`advance_series_in_tx` in `series.rs`). When the
//! game leaves the series in its next game already (both sides had one deck left, so both picks were
//! made for them, R332), that game is started after the commit.

use std::sync::Arc;
use std::time::Duration;

use indexmap::IndexMap;
use jackioh_engine::{GameOverReason, PlayerId, Winner};

use crate::actor::contracts::{RecordResultInput, TerminalOutcome, VoidMatchInput};
use crate::api::game_records::record_live_game;
use crate::api::http::{ApiError, ApiErrorCode};
use crate::api::ranked::{RankedGameInput, RankedSideInput, rate_ranked_game};
use crate::api::series::{SeriesGameResult, advance_series_in_tx, resume_series};
use crate::app::{App, now_ms};
use crate::config::{MATCH_REAPER_INTERVAL_SECONDS, RATING_START, RESULT_WRITE_ATTEMPTS};
use crate::db::store::{
    LastBoardKind, MatchRow, MatchSeat, Profile, RatedGameKind, RatedReason, ResultRow, SeriesRow, SeriesStatus,
    StoreError, Tx,
};

/// A failure this file reports to its caller (the actor, the reaper): TS rethrew the error itself,
/// so the sentence is the error's own.
fn failure(error: impl std::fmt::Display) -> ApiError {
    ApiError { code: ApiErrorCode::Internal, message: error.to_string(), details: None, retry_after_ms: None }
}

/// A list as the JSON array a TS log line printed it as.
fn json_list(items: &[String]) -> String {
    serde_json::to_string(items).unwrap_or_default()
}

/// The literal a series status is written as.
fn series_status_name(status: SeriesStatus) -> &'static str {
    match status {
        SeriesStatus::Picking => "picking",
        SeriesStatus::Playing => "playing",
        SeriesStatus::Over => "over",
    }
}

/// The side of the match's two seats that won, or null for a draw. §2.5: both heroes dead in the
/// same check, an accepted draw offer, the end of the 30th turn and the hard ceiling are draws;
/// everything else names a winner, a concede and a disconnect included.
fn winner_side_of(outcome: &TerminalOutcome, seats: &(MatchSeat, MatchSeat)) -> Option<usize> {
    let winner = outcome.winner.player()?;
    Some(if winner == seats.0.player { 0 } else { 1 })
}

/// R112: a reaper-resolved ceiling draw "records `turns = 0` and leaves both ratings unchanged,
/// where the same draw resolved by a live match actor records the real turn count and applies the
/// ordinary rating move". So the rating move is a parameter of the write, not a property of the
/// reason: the same `match-ceiling` draw rates differently depending on who resolved it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RatingPolicy {
    Rated,
    Unchanged,
}

impl RatingPolicy {
    fn as_str(self) -> &'static str {
        match self {
            RatingPolicy::Rated => "rated",
            RatingPolicy::Unchanged => "unchanged",
        }
    }
}

/// What one write did: the row (new, or the one already there), and the series this write advanced
/// — null for a match that is not a series game and for a repeated write, which advances nothing.
struct Written {
    row: ResultRow,
    series: Option<SeriesRow>,
}

/// Why one attempt of the write did not land: the duplicate the retry below exists for, or a real
/// failure.
enum WriteError {
    /// `results_insert` refused a second row for the match (TS `DuplicateResultError`).
    Duplicate(String),
    Failed(String),
}

impl WriteError {
    fn message(self) -> String {
        match self {
            WriteError::Duplicate(message) | WriteError::Failed(message) => message,
        }
    }
}

fn failed(error: impl std::fmt::Display) -> WriteError {
    WriteError::Failed(error.to_string())
}

/// `results_insert`'s error: the duplicate is told apart from every other failure.
fn insert_error(error: StoreError) -> WriteError {
    if matches!(error, StoreError::Duplicate(_)) {
        WriteError::Duplicate(error.to_string())
    } else {
        WriteError::Failed(error.to_string())
    }
}

/// The one write, retried on the collision below. Everything it touches is inside one
/// transaction, so a mid-write failure leaves no half-ended match: either the row, both ratings,
/// both in-match flags and the match's own `finished` state all land, or none of them do.
async fn write_result(
    app: &App,
    input: &RecordResultInput,
    rating_policy: RatingPolicy,
) -> Result<Written, ApiError> {
    let mut attempt: u32 = 1;
    loop {
        let outcome = async {
            let mut tx = app.db.begin(None).await.map_err(failed)?;
            let written = write_once(app, &mut tx, input, rating_policy).await?;
            tx.commit().await.map_err(failed)?;
            Ok::<Written, WriteError>(written)
        }
        .await;
        match outcome {
            Ok(written) => return Ok(written),
            // `results_get_by_match` cannot see a concurrent first writer's uncommitted row, so two
            // writers resolving one match at once both pass the read and one loses on
            // `results_pkey`. That loss is the same repeat, arriving the hard way: the winner's row
            // has committed by the time the loser's next transaction reads for it — and if the
            // winner rolled back instead, the rerun simply lands this write. Anything but the
            // duplicate is a real failure and stands.
            Err(WriteError::Duplicate(_)) if (attempt as u64) < RESULT_WRITE_ATTEMPTS as u64 => {
                attempt += 1;
            }
            Err(error) => return Err(failure(error.message())),
        }
    }
}

/// One attempt of the write, run as the body of `write_result`'s transaction.
async fn write_once(
    app: &App,
    tx: &mut Tx<'_>,
    input: &RecordResultInput,
    input_policy: RatingPolicy,
) -> Result<Written, WriteError> {
    // §9.5 idempotency: one row per match. The fake store and the `results` primary key both
    // refuse a second row, so this read is what turns that refusal into a clean no-op. It only sees
    // committed rows, though — two first writers can both pass it, which is what the retry above
    // is for: the loser's insert meets `results_pkey` instead.
    if let Some(already) = tx.results_get_by_match(&input.match_id).await.map_err(failed)? {
        return Ok(Written { row: already, series: None });
    }

    // R262: a game of a Conquest series is recorded but not rated — the series moves the rating
    // once, when it ends — whoever resolved it. R604: and a match the queue did not pair (a room
    // challenge) is never rated at all. The match row is what says so; a match whose row is gone
    // cannot say it was ranked, so it is not rated.
    let series = tx.series_by_match(&input.match_id).await.map_err(failed)?;
    let match_row: Option<MatchRow> = tx.matches_get(&input.match_id).await.map_err(failed)?;
    let ranked_match = match_row.as_ref().is_some_and(|row| row.ranked == Some(true));
    let rating_policy = if series.is_none() && ranked_match { input_policy } else { RatingPolicy::Unchanged };

    // A ranked series reaches its rating move — and so open_season_in_tx's `lock_seasons` — inside
    // `advance_series_in_tx` below, AFTER `results_insert`, `set_in_match` and the rest have already
    // taken row locks. Take the season lock first: a tx that waits on it while holding profile
    // locks deadlocks with the season opener holding it, whose `reset_ratings` update wants those
    // very rows. Every other path to `lock_seasons` already takes it before writing, and the
    // advisory lock is re-entrant, so `open_season_in_tx`'s own take costs nothing here.
    if series.as_ref().is_some_and(|series| series.ranked == Some(true)) {
        tx.ranked_lock_seasons().await.map_err(failed)?;
    }

    let (seat_a, seat_b) = &input.seats;
    let profiles: Vec<Profile> = tx
        .profiles_get_many(&[seat_a.profile_id.clone(), seat_b.profile_id.clone()])
        .await
        .map_err(failed)?;
    let by_id: IndexMap<String, Profile> =
        profiles.into_iter().map(|profile| (profile.id.clone(), profile)).collect();
    let rating_of = |seat: &MatchSeat| -> f64 {
        if let Some(profile) = by_id.get(&seat.profile_id) {
            return profile.rating;
        }
        // A match cannot outlive its players (the `results` foreign keys say so), so this is a
        // broken match rather than a rule: rate it from the starting rating and shout.
        tracing::error!(
            event = "results.profile_missing",
            matchId = %input.match_id,
            profileId = %seat.profile_id,
        );
        RATING_START as f64
    };

    let mut before: (f64, f64) = (rating_of(seat_a), rating_of(seat_b));
    let mut after: (f64, f64) = (before.0, before.1);
    if rating_policy == RatingPolicy::Rated {
        if let Some(match_row) = &match_row {
            // R603–R611: both hidden ratings, both ranks and the record of the rated game, in this
            // transaction, so the result and its rating move commit together or not at all.
            let rated = rate_ranked_game(
                tx,
                app,
                &RankedGameInput {
                    id: input.match_id.clone(),
                    kind: RatedGameKind::Match,
                    catalog_version: match_row.catalog_version.clone(),
                    sides: (
                        RankedSideInput::Player { profile_id: seat_a.profile_id.clone() },
                        RankedSideInput::Player { profile_id: seat_b.profile_id.clone() },
                    ),
                    winner_side: winner_side_of(&input.outcome, &input.seats),
                    reason: RatedReason::Game(input.outcome.reason),
                    at: input.at,
                    // R672: a double-or-nothing rematch's stakes ride on its row; absent is a normal game.
                    stake: if match_row.stake == Some(2) { Some(2) } else { None },
                },
            )
            .await
            .map_err(failed)?;
            before = (rated.sides.0.before.rating, rated.sides.1.before.rating);
            after = (rated.sides.0.after.rating, rated.sides.1.after.rating);
        }
    }

    let winner_profile_id = match input.outcome.winner.player() {
        None => None,
        Some(winner) => Some((if winner == seat_a.player { seat_a } else { seat_b }).profile_id.clone()),
    };
    let row = ResultRow {
        match_id: input.match_id.clone(),
        players: (seat_a.profile_id.clone(), seat_b.profile_id.clone()),
        winner_profile_id,
        reason: input.outcome.reason,
        turns: i64::from(input.turns),
        ended_at: input.at,
        rating_before: before,
        rating_after: after,
    };
    tx.results_insert(&row).await.map_err(insert_error)?;

    // R417, R565: each seat's board as this game ended, read from its own side, becomes its last
    // server board, with the result or not at all. The reaper reads no state, so it writes none.
    if let Some(last_boards) = &input.last_boards {
        let boards = [&last_boards.0, &last_boards.1];
        for (at, seat) in [seat_a, seat_b].into_iter().enumerate() {
            tx.last_boards_put(&seat.profile_id, LastBoardKind::Server, boards[at], input.at)
                .await
                .map_err(failed)?;
        }
    }

    for seat in [seat_a, seat_b] {
        // R112's "leaves both ratings unchanged" is literal: no rating write happens at all, and the
        // rated path above made the only one there is.
        // §9.5: "Every ending records a result and clears both players' in-match state." Both, for
        // every reason — M7-T1's "past grace they have lost and both can queue again".
        tx.profiles_set_in_match(&seat.profile_id, None).await.map_err(failed)?;
        // A profile can be queued *and* in a match (it accepted a room challenge while waiting), and
        // an open ticket would block the re-queue that M7-T1 promises, so the ending clears that
        // too. This mirrors `app.end_match` in migration 0004, which cancels the same stray ticket.
        if let Some(ticket) = tx.tickets_open_for_profile(&seat.profile_id).await.map_err(failed)? {
            tx.tickets_cancel(&ticket.id, input.at).await.map_err(failed)?;
        }
    }

    // The match row may be gone; a missing match must not lose the result, so it is checked rather
    // than assumed.
    if match_row.is_some() {
        tx.matches_finish(&input.match_id, input.at).await.map_err(failed)?;
    }

    // R263: the series' record of this game, and R262's rating move if it ends the series, in this
    // same transaction — a failure here rolls the result back with it, so the two cannot disagree.
    let advanced = match series {
        None => None,
        Some(series) => Some(
            advance_series_in_tx(
                tx,
                app,
                &series,
                &SeriesGameResult {
                    match_id: input.match_id.clone(),
                    seats: input.seats.clone(),
                    outcome: input.outcome.clone(),
                    at: input.at,
                },
            )
            .await
            .map_err(failed)?,
        ),
    };

    tracing::info!(
        event = "match.ended",
        matchId = %input.match_id,
        reason = %row.reason,
        winner = row.winner_profile_id.as_deref(),
        turns = row.turns,
        ratingPolicy = rating_policy.as_str(),
        seriesId = advanced.as_ref().map(|series| series.id.as_str()),
        seriesStatus = advanced.as_ref().map(|series| series_status_name(series.status)),
    );
    Ok(Written { row, series: advanced })
}

/// The `RecordResult` port the actor holds (TS `ActorDeps.recordResult`, made by
/// `createRecordResult(deps)`). Every terminal reason comes through here, and a ranked match gets
/// the ordinary Glicko-2 rating move (R603, R112's "live match actor" half). Once the result is in,
/// the game is filed for the card statistics (R376, `game_records.rs`).
pub async fn record_result(app: &Arc<App>, input: RecordResultInput) -> Result<ResultRow, ApiError> {
    let written = write_result(app, &input, RatingPolicy::Rated).await?;
    // After the commit: a series whose next game began already (R332) gets its match. A
    // failure to start it is the sweeper's to heal (R263), never this result's.
    resume_series(app, written.series.as_ref()).await;
    // R376: after the commit too, and never at the result's expense — it logs and swallows its
    // own failures, and a second write of the same match files nothing.
    record_live_game(app, &input.match_id).await;
    Ok(written.row)
}

/// R679: the `VoidMatch` port the actor holds (TS `ActorDeps.voidMatch`, made by
/// `createVoidMatch(deps)`) — what a Glitch's void outcome does instead of `record_result`. The match
/// never happened: no `results` row, no rating move, no last board, no game record. The store
/// forgets the row and its log and lets both players go (`matches_forget_voided`), and one log line
/// names the match and both profiles, for abuse checks.
///
/// A voided game of a Conquest series is treated as a game that never started: the series stays
/// `playing` on the same game, which is started again at once, with the same seats, decks and seed
/// (`resume_series`) — the one thing `series.rs` does for a game a restart interrupted. Nothing is
/// recorded in the series, so the score and both players' locked decks are what they were.
pub async fn void_match(app: &Arc<App>, input: VoidMatchInput) -> Result<(), ApiError> {
    let mut tx = app.db.begin(None).await.map_err(failure)?;
    let series = tx.series_by_match(&input.match_id).await.map_err(failure)?;
    tx.matches_forget_voided(&input.match_id).await.map_err(failure)?;
    tx.commit().await.map_err(failure)?;
    let players = [input.players.0.clone(), input.players.1.clone()];
    tracing::warn!(
        event = "match.voided",
        matchId = %input.match_id,
        players = %json_list(&players),
        at = input.at,
        seriesId = series.as_ref().map(|series| series.id.as_str()),
    );
    resume_series(app, series.as_ref()).await;
    Ok(())
}

/// R112: the reaper's ceiling draw records no turn count.
const REAPER_TURNS: i32 = 0;

/// §9.5's reaper: "a reaper resolves anything past the ceiling". Every `live` match whose
/// `clocks.ceiling_at` has passed ends as a draw by `match-ceiling` (§2.5, R79) with both players'
/// in-match state cleared, and its in-memory actor — if the process still has one — is dropped.
///
/// R112 fixes what a reaper-resolved draw records: `turns = 0` and both ratings unchanged. A match
/// the reaper reaches is one whose actor is not answering, so the turn count it would need is
/// exactly the thing it cannot read, and it does not guess.
///
/// Safe to run beside a live actor: both paths go through the same idempotent write, so whichever
/// gets there first is the one that counts and the other returns that row untouched.
///
/// Answers the ids it resolved.
pub async fn reap_stuck_matches(app: &Arc<App>) -> Result<Vec<String>, ApiError> {
    let now = now_ms();
    let live = {
        let mut tx = app.db.begin(None).await.map_err(failure)?;
        let live = tx.matches_live().await.map_err(failure)?;
        tx.commit().await.map_err(failure)?;
        live
    };
    let mut resolved: Vec<String> = Vec::new();

    for match_row in live {
        if match_row.clocks.ceiling_at > now {
            continue;
        }
        let input = RecordResultInput {
            match_id: match_row.id.clone(),
            seats: seats_of(&match_row),
            outcome: TerminalOutcome { winner: Winner::Draw, reason: GameOverReason::MatchCeiling },
            turns: REAPER_TURNS,
            at: now,
            last_boards: None,
        };
        let advanced = match write_result(app, &input, RatingPolicy::Unchanged).await {
            Ok(written) => written.series,
            Err(error) => {
                // One wedged match must not stop the sweep.
                tracing::error!(event = "reaper.failed", matchId = %match_row.id, message = %error.message);
                continue;
            }
        };
        resolved.push(match_row.id.clone());
        if app.matches.has(&match_row.id) {
            app.matches.stop(&match_row.id).await;
        }
        // A reaped series game is a drawn game like any other (R334), and may leave the next game to
        // start (R332).
        resume_series(app, advanced.as_ref()).await;
    }

    if !resolved.is_empty() {
        tracing::warn!(event = "reaper.resolved", matches = %json_list(&resolved));
    }
    Ok(resolved)
}

/// Seat order is the match row's player order: index 0 is p1 (`MatchRow.players`).
fn seats_of(match_row: &MatchRow) -> (MatchSeat, MatchSeat) {
    (
        MatchSeat {
            profile_id: match_row.players.0.clone(),
            player: PlayerId::P1,
            deck: match_row.decks.0.clone(),
            portrait: None,
        },
        MatchSeat {
            profile_id: match_row.players.1.clone(),
            player: PlayerId::P2,
            deck: match_row.decks.1.clone(),
            portrait: None,
        },
    )
}

/// The reaper's loop (SURFACE §11.2: `api::results::run_reaper(app)`, every
/// `MATCH_REAPER_INTERVAL_SECONDS`), as TS's `start()` in `src/index.ts` ran it: the first sweep one
/// interval after boot, each next one an interval after the last finished, a failure logged and the
/// next sweep tried all the same. Runs until its task is aborted.
pub async fn run_reaper(app: Arc<App>) {
    loop {
        tokio::time::sleep(Duration::from_secs(MATCH_REAPER_INTERVAL_SECONDS as u64)).await;
        match reap_stuck_matches(&app).await {
            Ok(ids) => {
                if !ids.is_empty() {
                    tracing::warn!(event = "matches.reaped", ids = %json_list(&ids));
                }
            }
            Err(error) => {
                tracing::warn!(event = "matches.reaper_failed", message = %error.message);
            }
        }
    }
}
