//! Results and rating (BUILD M7-T2, SPEC §2.5, §9.5, §9.12, R79, R112, R603).
//!
//! `record_result` processes terminal match outcomes (§2.5), writes the `results` row,
//! updates ratings if ranked (R604), clears in-match flags, and stores final replay hashes (R768).
//! Idempotent to support concurrent actor and reaper (§9.5) execution.
//!
//! Rating updates are delegated to `rate_ranked_game` (R611).
//! Unranked matches record unchanged ratings and unranked pay rates (R604, R1502000).
//! Conquest games commit progress in `advance_series_in_tx`, rating once at series end (R262, R263, R332).

use std::sync::Arc;
use std::time::Duration;

use indexmap::IndexMap;
use jackioh_engine::{ActionBody, GameOverReason, PerPlayer, PlayerId, Winner};

use crate::actor::contracts::{RecordResultInput, TerminalOutcome, VoidMatchInput};
use crate::actor::{engine, match_actor, registry};
use crate::api::game_records::{Finished, Folded, read_finished, record_finished_game};
use crate::api::http::{ApiError, ApiErrorCode};
use crate::api::queue::json_list;
use crate::api::ranked::{RankedGameInput, RankedSideInput, rate_ranked_game};
use crate::api::rematch::sweep_rematch_offers;
use crate::api::series::{SeriesGameResult, advance_series_in_tx, resume_series};
use crate::app::{App, now_ms};
use crate::bug_reports::{Finding, FoundIn, refused_rows};
use crate::config::{
    JLORE_GAMES_DAILY_CAP, MATCH_REAPER_INTERVAL_SECONDS, OPEN_MATCH_REAP_GRACE_MS, RATING_START,
    RESULT_WRITE_ATTEMPTS,
};
use crate::db::store::{
    LastBoardKind, MatchActionRow, MatchRow, MatchSeat, Profile, ProfileKind, RatedGameKind, RatedReason,
    ResultRow, SeriesRow, SeriesStatus, StoreError, Tx,
};
use crate::duos::again::sweep_duos_again_offers;
use crate::economy::quests::tally_seat;
use crate::economy::rewards::{
    GameEconomy, GameMode, GameSummary, game_jlore, is_player_action, pay_game_rewards, utc_day_bounds,
};

/// Wraps an error into an internal ApiError.
fn failure(error: impl std::fmt::Display) -> ApiError {
    ApiError {
        code: ApiErrorCode::Internal,
        message: error.to_string(),
        details: None,
        retry_after_ms: None,
    }
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

/// Whether `player`'s seat won (`Some(true)`), lost (`Some(false)`) or drew (`None`), as the reward
/// hook reads it. R1843004: the other player's `fled` is a draw for this seat, not a loss.
fn won_by(outcome: &TerminalOutcome, player: PlayerId) -> Option<bool> {
    match jackioh_engine::seat_outcome(outcome.winner, outcome.reason, player) {
        jackioh_engine::SeatOutcome::Win => Some(true),
        jackioh_engine::SeatOutcome::Loss => Some(false),
        jackioh_engine::SeatOutcome::Draw => None,
    }
}

/// §14.8, [[R1502000]]: the rates a finished 1v1 match pays at: a game of a Conquest series,
/// ranked or not, as a Conquest game; a ranked match outside one as a ladder game; any other
/// (a room challenge and its rematches, R604) as an unranked game.
fn game_mode_of(in_series: bool, ranked: bool) -> GameMode {
    match (in_series, ranked) {
        (true, _) => GameMode::Conquest,
        (false, true) => GameMode::Ladder,
        (false, false) => GameMode::Unranked,
    }
}

/// R112: a reaper-resolved ceiling draw "records `turns = 0` and leaves both ratings unchanged,
/// where the same draw resolved by a live match actor records the real turn count and applies the
/// ordinary rating move". So the rating move is a parameter of the write, not of the reason.
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
    /// What the reward hook paid each seat, keyed by the engine seat `RecordResultInput.seats`
    /// credits it in: `None` for a seat it did not pay (a bot's side, a match with no row, the
    /// economy off) and for both on a repeated write, which pays nothing (#1657).
    economy: PerPlayer<Option<GameEconomy>>,
}

/// Why one attempt of the write did not land: the duplicate the retry below exists for, or a real
/// failure.
enum WriteError {
    /// Refused a second row for the match.
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
/// both in-match flags and the match's own `finished` state all land, or none of them do. #2213: the
/// match and its log are read and folded once before the transaction, so no fold runs under the
/// season or economy locks, and a retry on a duplicate reuses that fold. It answers the fold with the
/// write, for the game record to read.
async fn write_result(
    app: &App,
    input: &RecordResultInput,
    rating_policy: RatingPolicy,
) -> Result<(Written, Option<Finished>), ApiError> {
    let finished = read_finished(app, &input.match_id).await.map_err(failure)?;
    let mut attempt: u32 = 1;
    loop {
        let outcome = async {
            let mut tx = app.db.begin(None).await.map_err(failed)?;
            let written = write_once(app, &mut tx, input, rating_policy, finished.as_ref()).await?;
            tx.commit().await.map_err(failed)?;
            Ok::<Written, WriteError>(written)
        }
        .await;
        match outcome {
            Ok(written) => {
                // R1523000: the ending's release, once the flags' clear has landed (a repeat of the
                // write releases again, which changes nothing): both seats leave the match, a series
                // game's back to between games, so a next game the write began is admitted, and a
                // series the game ended lets both sides go.
                for seat in [&input.seats.0, &input.seats.1] {
                    app.seats.end_game(&seat.profile_id, &input.match_id);
                }
                if let Some(series) = &written.series {
                    crate::seats::free_series(app, series);
                }
                return Ok((written, finished));
            }
            // `results_get_by_match` cannot see a concurrent first writer's uncommitted row, so two writers
            // resolving one match at once both pass the read and one loses on `results_pkey`: the same
            // repeat, arriving the hard way. The winner's row has committed by the loser's next read, or
            // if it rolled back the rerun lands this write. Anything but the duplicate stands as a failure.
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
    finished: Option<&Finished>,
) -> Result<Written, WriteError> {
    // §9.5 idempotency: one row per match. The fake store and the `results` primary key both refuse a
    // second row, so this read turns that refusal into a clean no-op. It only sees committed rows, so two
    // first writers can both pass it, which the retry above is for: the loser's insert meets `results_pkey`.
    if let Some(already) = tx.results_get_by_match(&input.match_id).await.map_err(failed)? {
        return Ok(Written {
            row: already,
            series: None,
            economy: PerPlayer::new(None, None),
        });
    }

    // R262: a game of a Conquest series is recorded but not rated — the series moves the rating
    // once, when it ends — whoever resolved it. R604: and a match the queue did not pair (a room
    // challenge) is never rated at all. The match row is what says so; a match whose row is gone
    // cannot say it was ranked, so it is not rated.
    let series = tx.series_by_match(&input.match_id).await.map_err(failed)?;
    let match_row: Option<MatchRow> = tx.matches_get(&input.match_id).await.map_err(failed)?;
    let ranked_match = match_row.as_ref().is_some_and(|row| row.ranked == Some(true));
    let rating_policy = if series.is_none() && ranked_match {
        input_policy
    } else {
        RatingPolicy::Unchanged
    };

    // A ranked series reaches its rating move (and so `open_season_in_tx`'s `lock_seasons`) inside
    // `advance_series_in_tx` below, AFTER `results_insert`, `clear_in_match` and the rest have taken row
    // locks. Take the season lock first: a tx that waits on it while holding profile locks deadlocks with
    // the season opener holding it, whose `reset_ratings` update wants those rows. The advisory lock is
    // re-entrant, so `open_season_in_tx`'s own take costs nothing here.
    if series.as_ref().is_some_and(|series| series.ranked == Some(true)) {
        tx.ranked_lock_seasons().await.map_err(failed)?;
    }

    let (seat_a, seat_b) = &input.seats;
    let profiles: Vec<Profile> = tx
        .profiles_get_many(&[seat_a.profile_id.clone(), seat_b.profile_id.clone()])
        .await
        .map_err(failed)?;
    let by_id: IndexMap<String, Profile> = profiles
        .into_iter()
        .map(|profile| (profile.id.clone(), profile))
        .collect();
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
        RATING_START
    };

    // R610, R1452: a ladder bot's side is rated as the bot it is, on its own `bot_ratings` row.
    let side_of = |seat: &MatchSeat| -> RankedSideInput {
        let profile_id = seat.profile_id.clone();
        match by_id.get(&seat.profile_id).map(|profile| profile.kind) {
            Some(ProfileKind::Bot) => RankedSideInput::Bot { bot_id: profile_id },
            _ => RankedSideInput::Player { profile_id },
        }
    };

    let mut before: (f64, f64) = (rating_of(seat_a), rating_of(seat_b));
    let mut after: (f64, f64) = (before.0, before.1);
    if rating_policy == RatingPolicy::Rated
        && let Some(match_row) = &match_row
    {
        // R603–R611: both hidden ratings, both ranks and the record of the rated game, in this
        // transaction, so the result and its rating move commit together or not at all.
        let rated = rate_ranked_game(
            tx,
            app,
            &RankedGameInput {
                id: input.match_id.clone(),
                kind: RatedGameKind::Match,
                catalog_version: match_row.catalog_version.clone(),
                sides: (side_of(seat_a), side_of(seat_b)),
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

    let winner_profile_id = input.outcome.winner.player().map(|winner| {
        (if winner == seat_a.player { seat_a } else { seat_b })
            .profile_id
            .clone()
    });
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

    // §14.8: every finished match pays Jlore and XP through the one reward hook at its mode's rates
    // ([[R1502000]]: `game_mode_of`), in this same transaction, so a payout failure rolls the row
    // back with it. A game of a Conquest series pays as its own result, though the series moves the
    // rating only once (R262). Deliberately not gated on `rating_policy`: a reaper ceiling draw is
    // unrated yet still pays ([[R1302002]]). A missing match row, a voided match (that path never
    // reaches here) and a bot's side ([[R1454]]) pay nothing. The server ran the match, so it
    // counts each seat's actions from its own log, the moves that seat took and not the server's
    // own rows ([[R1302004]]); the client reports nothing. Nothing is paid while `ECONOMY_LIVE` is
    // off (§14.12, [[R1302006]]): the economy is built behind it, and a ledger row is never
    // deleted. The same hook moves each seat's quests (§14.9, [[R1304001]]) by what the game did:
    // its plays and fired Traps, read off the log by the engine's fold (`quests_game`), and its
    // attacks, counted from the log. What it paid each person is answered to the actor (#1657).
    let mut economy = PerPlayer::new(None, None);
    if app.economy_live
        && let Some(mode) = match_row
            .as_ref()
            .map(|row| game_mode_of(series.is_some(), row.ranked == Some(true)))
    {
        // [[R1301003]]: both seats' economy locks, in profile-id order, so two payouts of one
        // account run one after the other and two ladder results sharing a player never wait on
        // each other in opposite orders. Taking one account's lock twice is safe: the advisory
        // lock is re-entrant, as is each seat's payout below retaking its own.
        let mut ordered = [seat_a.profile_id.clone(), seat_b.profile_id.clone()];
        ordered.sort();
        for profile_id in &ordered {
            tx.economy_lock_account(profile_id).await.map_err(failed)?;
        }
        // #2213: `read_finished` read the log, and folded it, before this transaction. The log is
        // append-only and a finished game takes no action, so this transaction could read no row that
        // read did not, and no fold runs under these locks.
        let log: &[MatchActionRow] = match finished {
            Some(finished) => &finished.log,
            None => &[],
        };
        let game = finished.and_then(|finished| quests_game(&input.match_id, &finished.folded));
        for seat in [seat_a, seat_b] {
            let taken = |keep: fn(&ActionBody) -> bool| {
                let count = log
                    .iter()
                    .filter(|logged| logged.action.player_id == seat.player && keep(&logged.action.body))
                    .count();
                i64::try_from(count).unwrap_or(i64::MAX)
            };
            let actions = taken(is_player_action);
            let attacks = taken(|body| matches!(body, ActionBody::Attack { .. }));
            let bot = matches!(
                by_id.get(&seat.profile_id).map(|profile| profile.kind),
                Some(ProfileKind::Bot)
            );
            let paid = pay_game_rewards(
                tx,
                &GameSummary {
                    mode,
                    profile_id: seat.profile_id.clone(),
                    player: seat.player,
                    won: won_by(&input.outcome, seat.player),
                    actions,
                    match_id: input.match_id.clone(),
                    at: input.at,
                    bot,
                    tally: tally_seat(
                        game.map(|game| &game.seats[seat.player]),
                        attacks,
                        &app.catalog.defs,
                    ),
                },
            )
            .await
            .map_err(failed)?;
            economy[seat.player] = (!bot).then_some(paid);
        }
    }

    // R417, R565: each seat's board as this game ended, read from its own side, becomes its last server
    // board, with the result or not at all. The reaper's ceiling draw reads no state, so it writes none.
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
        // R1896000: only a flag that still names this match is this match's state, so a late result
        // leaves a newer match's flag alone, and a seat whose profile row is gone has none to clear.
        tx.profiles_clear_in_match(&seat.profile_id, &input.match_id)
            .await
            .map_err(failed)?;
        // A profile can be queued *and* in a match (it accepted a room challenge while waiting), and
        // an open ticket would block the re-queue that M7-T1 promises, so the ending clears that
        // too. This mirrors `app.end_match` in migration 0004, which cancels the same stray ticket.
        if let Some(ticket) = tx
            .tickets_open_for_profile(&seat.profile_id)
            .await
            .map_err(failed)?
        {
            tx.tickets_cancel(&ticket.id, input.at).await.map_err(failed)?;
        }
    }

    // The match row may be gone; a missing match must not lose the result, so it is checked rather
    // than assumed.
    if match_row.is_some() {
        tx.matches_finish(&input.match_id, input.at)
            .await
            .map_err(failed)?;
        // R768: the final state's hash, with the result or not at all; the reaper's ceiling draw has none
        // to write.
        if let Some(final_hash) = &input.final_hash {
            tx.matches_record_final_hash(&input.match_id, final_hash)
                .await
                .map_err(failed)?;
        }
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
                    outcome: input.outcome,
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
    Ok(Written {
        row,
        series: advanced,
        economy,
    })
}

/// §14.9: the finished game a ladder result's quests read (`economy::quests::tally_seat`): the
/// match's `(seed, decks, log)` folded by the engine's `summarize_game`, off the runtime under the
/// catalog the match started on (#1035, R767000). It is the fold `read_finished` made before the
/// transaction (#2213), the one the game record reads (`game_records.rs`). `None`, logged, when the
/// log cannot be read or reaches no result: the quests then count the log's attacks alone, and the
/// result is written all the same.
fn quests_game<'a>(
    match_id: &str,
    folded: &'a Result<Folded, String>,
) -> Option<&'a jackioh_engine::GameSummary> {
    match folded {
        Ok(Folded { game: Some(game), .. }) => Some(game),
        Ok(Folded { game: None, .. }) => {
            tracing::warn!(event = "quests.game.unread", matchId = %match_id, message = "the log reaches no result");
            None
        }
        Err(message) => {
            tracing::warn!(event = "quests.game.unread", matchId = %match_id, message = %message);
            None
        }
    }
}

/// Every terminal reason comes through here, and a ranked match gets the ordinary Glicko-2 rating move
/// (R603, R112's "live match actor" half). Once the result is in, the game is filed for the card
/// statistics (R376, `game_records.rs`).
pub async fn record_result(app: &Arc<App>, input: RecordResultInput) -> Result<ResultRow, ApiError> {
    Ok(record_result_with_economy(app, input).await?.row)
}

/// What [`record_result_with_economy`] answers: the result's row, and what its write paid each
/// seat, keyed by the engine seat `RecordResultInput.seats` credits it in (#1657). A seat the
/// write paid nothing (a bot's side, the economy off) has `None`, and so has every seat when the
/// match's result was already written: a repeat pays nothing, so it has nothing to tell.
pub struct Recorded {
    pub row: ResultRow,
    pub economy: PerPlayer<Option<GameEconomy>>,
}

/// [`record_result`], answering what the write paid each seat as well, for the actor to push in
/// that seat's views (#1657).
pub async fn record_result_with_economy(
    app: &Arc<App>,
    input: RecordResultInput,
) -> Result<Recorded, ApiError> {
    let (written, finished) = write_result(app, &input, RatingPolicy::Rated).await?;
    // After the commit: a series whose next game began already (R332) gets its match. A
    // failure to start it is the sweeper's to heal (R263), never this result's.
    resume_series(app, written.series.as_ref()).await;
    // R376: after the commit too, and never at the result's expense: it logs and swallows its own
    // failures, a second write of the same match files nothing, and one that failed is filed again by
    // the retention loop while the match's log is held (R1596000). #2213: from the write's one fold.
    record_finished_game(app, &input.match_id, finished, &input.seats).await;
    Ok(Recorded {
        row: written.row,
        economy: written.economy,
    })
}

/// R1253000: `ceiling_draw_input`'s draw, written as the reaper writes it (both ratings unchanged,
/// no game record), then the series' next game (R332, R334).
pub async fn record_ceiling_draw(app: &Arc<App>, input: RecordResultInput) -> Result<ResultRow, ApiError> {
    let (written, _) = write_result(app, &input, RatingPolicy::Unchanged).await?;
    resume_series(app, written.series.as_ref()).await;
    Ok(written.row)
}

/// R1483000: whether the daily Jlore cap held back each account's pay for the finished match
/// `match_id`, keyed by the seat each account began in (`MatchRow.players`). Read off the ledger
/// after the write, so a fresh result, a repeat of it and an actor rebuilt on the finished match
/// all answer alike: an account's pay was held back when its `game` row for the match is below its
/// mode's rate for its result and its UTC day has reached `JLORE_GAMES_DAILY_CAP`. A game the
/// reward hook did not pay (a bot's side) or any game while the economy is off holds nothing back.
pub async fn game_jlore_caps(app: &App, match_id: &str) -> Result<PerPlayer<bool>, ApiError> {
    let none = PerPlayer::new(false, false);
    if !app.economy_live {
        return Ok(none);
    }
    let mut tx = app.db.begin(None).await.map_err(failure)?;
    let result = tx.results_get_by_match(match_id).await.map_err(failure)?;
    let match_row = tx.matches_get(match_id).await.map_err(failure)?;
    // The write recorded the game in its series, so the series is found by the games it holds, not
    // by the game in play (`series_by_match`), which has moved on.
    let series = tx.series_with_game(match_id).await.map_err(failure)?;
    let (Some(result), Some(match_row)) = (result, match_row) else {
        return Ok(none);
    };
    let mode = game_mode_of(series.is_some(), match_row.ranked == Some(true));
    let (first, second) = match_row.players;
    let profiles = tx
        .profiles_get_many(&[first.clone(), second.clone()])
        .await
        .map_err(failure)?;
    let (from, until) = utc_day_bounds(result.ended_at);
    let mut capped = [false, false];
    for (held_back, profile_id) in capped.iter_mut().zip([&first, &second]) {
        if profiles
            .iter()
            .any(|profile| profile.id == *profile_id && profile.kind == ProfileKind::Bot)
        {
            continue;
        }
        let won = result
            .winner_profile_id
            .as_ref()
            .map(|winner| winner == profile_id);
        let totals = tx
            .economy_game_jlore_totals(profile_id, from, until, match_id)
            .await
            .map_err(failure)?;
        *held_back =
            totals.reference < i64::from(game_jlore(mode, won)) && totals.day >= JLORE_GAMES_DAILY_CAP;
    }
    tx.commit().await.map_err(failure)?;
    Ok(PerPlayer::new(capped[0], capped[1]))
}

/// R679: the `VoidMatch` port the actor holds: what a Glitch's void outcome does instead of
/// `record_result`. The match never happened: no `results` row, no rating move, no last board, no game
/// record. The store forgets the row and its log and lets both players go (`matches_forget_voided`), and
/// one log line names the match and both profiles, for abuse checks.
///
/// A voided game of a Conquest series is treated as a game that never started: the series stays
/// `playing` on the same game, which is started again at once, with the same seats, decks and seed
/// (`resume_series`), as `series.rs` does for a game a restart interrupted. Nothing is recorded in the
/// series, so the score and both players' locked decks are what they were.
pub async fn void_match(app: &Arc<App>, input: VoidMatchInput) -> Result<(), ApiError> {
    let mut tx = app.db.begin(None).await.map_err(failure)?;
    let series = tx.series_by_match(&input.match_id).await.map_err(failure)?;
    tx.matches_forget_voided(&input.match_id).await.map_err(failure)?;
    tx.commit().await.map_err(failure)?;
    // R1523000: both players leave the match the store forgot; a series game's back to between
    // games, which admits the same game started again below.
    for profile_id in [&input.players.0, &input.players.1] {
        app.seats.end_game(profile_id, &input.match_id);
    }
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

/// R112: the reaper's ceiling draw records no turn count (R833000: a log that ends records its own).
const REAPER_TURNS: i32 = 0;

/// R112: the ceiling's draw the reaper writes; R1253000: and a live actor whose clock's action
/// panicked.
pub fn ceiling_draw_input(match_row: &MatchRow, at: i64) -> RecordResultInput {
    RecordResultInput {
        match_id: match_row.id.clone(),
        seats: seats_of(match_row),
        outcome: TerminalOutcome {
            winner: Winner::Draw,
            reason: GameOverReason::MatchCeiling,
        },
        turns: REAPER_TURNS,
        at,
        last_boards: None,
        final_hash: None,
    }
}

/// Reaps matches past ceiling (§9.5, §2.5, R79): each `live` match whose `clocks.ceiling_at` has passed
/// is ended with both players' in-match state cleared and its actor, if any, dropped. R833000: a log
/// that already ends (R1437) records its real result; any other ends as a `match-ceiling` draw with
/// `turns = 0` and both ratings unchanged (R112). Safe beside a live actor: both go through the same
/// idempotent write, so the first counts and the other returns that row untouched. Answers the ids it
/// resolved.
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
        // R833000: a log that already ends is recorded as it ended; a recovery that fails falls through
        // to the ceiling's draw below.
        if let Some(recovered) = fold_terminal_input(app, &match_row, now).await {
            match record_result(app, recovered).await {
                Ok(_) => {
                    resolved.push(match_row.id.clone());
                    if app.matches.has(&match_row.id) {
                        app.matches.stop(&match_row.id).await;
                    }
                    tracing::info!(event = "reaper.recovered", matchId = %match_row.id);
                    continue;
                }
                Err(error) => {
                    tracing::error!(event = "reaper.recovery.failed", matchId = %match_row.id, message = %error.message);
                }
            }
        }
        let input = ceiling_draw_input(&match_row, now);
        let advanced = match write_result(app, &input, RatingPolicy::Unchanged).await {
            Ok((written, _)) => written.series,
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

/// R833000: the result a reap-bound match's log already holds, as the actor would have recorded it
/// (`MatchActor::on_terminal`): its `(seed, log)` folded once, as a rebuild folds it (§9.5). `None`
/// leaves the match to R112's ceiling draw: a log that does not end, a fold that panics or refuses
/// an action the log holds (§9.3's determinism break: no permanent result is written from a game that
/// does not reconstruct), one that ends in a void (R679: the void path forgets the match, a result
/// write never does), or a store that fails to read the log.
async fn fold_terminal_input(app: &Arc<App>, match_row: &MatchRow, now: i64) -> Option<RecordResultInput> {
    let read = async {
        let mut tx = app.db.begin(None).await?;
        let log = tx.matches_actions(&match_row.id).await?;
        let mode = tx.matches_mode_of(&match_row.id).await?;
        tx.commit().await?;
        Ok::<_, StoreError>((log, mode))
    };
    let (log, mode) = match read.await {
        Ok(read) => read,
        Err(error) => {
            tracing::error!(event = "reaper.fold.unread", matchId = %match_row.id, message = %error);
            return None;
        }
    };

    let (last_boards, glitch_boards) = match_actor::last_boards_of(match_row);
    let args = engine::fold_args(
        &match_row.seed,
        &match_row.decks,
        log.iter().map(|row| row.action.clone()).collect(),
        last_boards,
        glitch_boards,
        registry::dealt_for(mode),
    );
    // #1035: off the runtime, as a rebuild folds, under the catalog the match started on
    // (R388, R767000).
    let pinned = registry::pinned_for(match_row);
    let folded = match engine::off_runtime(app.engine_thread, move || engine::fold(pinned, &args)).await {
        Ok(folded) => folded,
        Err(message) => {
            tracing::error!(event = "reaper.fold.panicked", matchId = %match_row.id, message = %message);
            return None;
        }
    };
    if !folded.errors.is_empty() {
        tracing::error!(event = "reaper.fold.errors", matchId = %match_row.id, errors = ?folded.errors);
        app.bug_reports.file(
            app.env.deployed_commit.as_deref(),
            Finding {
                match_id: match_row.id.clone(),
                found_in: FoundIn::Reaper,
                catalog_version: match_row.catalog_version.clone(),
                actions: log.len(),
                refused: refused_rows(&log, &folded.errors),
            },
        );
        return None;
    }
    let snapshot = engine::snapshot(&folded.state);
    let result = snapshot.result?;
    if result.reason == GameOverReason::Voided {
        return None;
    }

    // R677: credited by who plays each seat now, as the actor's `credited_seats` does.
    let mut seats = seats_of(match_row);
    if snapshot.seats_swapped {
        std::mem::swap(&mut seats.0.profile_id, &mut seats.1.profile_id);
    }
    Some(RecordResultInput {
        match_id: match_row.id.clone(),
        seats,
        outcome: result,
        turns: snapshot.turn,
        at: now,
        last_boards: Some(engine::last_boards(&folded.state)),
        final_hash: Some(engine::hash_state(&folded.state)),
    })
}

/// R1450: the reaper's second sweep: deletes every `open` row more than `OPEN_MATCH_REAP_GRACE_SECONDS`
/// past its `ceiling_at` (a room nobody joined, a claimed room whose match never started, a queue
/// pairing whose start and cleanup both failed), freeing its room code and both players. The grace
/// outlasts any start in flight. A sweep that deleted rows holds the seat ledger to the record at once
/// (R1523000). Answers how many rows it deleted.
pub async fn reap_open_matches(app: &App) -> Result<u64, ApiError> {
    let before = now_ms() - OPEN_MATCH_REAP_GRACE_MS;
    let mut tx = app.db.begin(None).await.map_err(failure)?;
    let reaped = tx.matches_reap_open(before).await.map_err(failure)?;
    tx.commit().await.map_err(failure)?;
    if reaped > 0 {
        tracing::warn!(event = "reaper.open_released", rows = reaped);
        // R1523000: the players these rows held go from the seat ledger too, which a start whose
        // cleanup failed left holding them as the record did.
        crate::seats::reconcile(app).await;
    }
    Ok(reaped)
}

/// Seat order is the match row's player order: index 0 is p1 (`MatchRow.players`).
fn seats_of(match_row: &MatchRow) -> (MatchSeat, MatchSeat) {
    (
        MatchSeat {
            profile_id: match_row.players.0.clone(),
            player: PlayerId::P1,
            deck: match_row.decks.0.clone(),
            portrait: None,
            deck_slots: None,
        },
        MatchSeat {
            profile_id: match_row.players.1.clone(),
            player: PlayerId::P2,
            deck: match_row.decks.1.clone(),
            portrait: None,
            deck_slots: None,
        },
    )
}

/// Background loop running reaper sweeps every `MATCH_REAPER_INTERVAL_SECONDS`
/// (SURFACE §11.2, R1450, R1523000).
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
        if let Err(error) = reap_open_matches(&app).await {
            tracing::warn!(event = "matches.open_reaper_failed", message = %error.message);
        }
        crate::seats::reconcile(&app).await;
        app.matches.reap_finished(now_ms()).await;
        sweep_rematch_offers(&app, now_ms());
        sweep_duos_again_offers(&app, now_ms());
    }
}
