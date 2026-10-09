//! The Conquest series (SPEC §9.5, R330–R338, with R262–R264): the store half of `series_rules.rs`.
//!
//! The rules are pure functions over `SeriesRow`; this file reads a row, applies one and writes it
//! back, and does what a pure function cannot:
//!  - **Compare-and-set** (R263). A series has several writers (both players, the sweeper,
//!    `results.rs`); `series_update` writes only over the version it computed from, and a lost write
//!    re-reads and re-applies the same transition. A pick is persisted before it is acknowledged and
//!    leaves the server only in its owner's projection (R331).
//!  - **The rating move** (R262, R604). A transition that ends a ranked series rates it as one game
//!    (`ranked.rs`, R603), put on the row by the same compare-and-set. A game inside a series is
//!    never rated (`results.rs`); an abandoned series and a room's are unrated.
//!  - **Starting the game** (R331, R263). With both picks in, `next_match_id` starts through
//!    `app.matches` after the commit, since the actor must never run a game the row does not name;
//!    the sweeper starts one a crash left unstarted, after `SERIES_START_GRACE_SECONDS`.
//!
//! Surface contract: docs/v0.3.0/SURFACE.md §11.2. Sweeper: `run_sweeper` (every 5 s).

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use jackioh_engine::Winner;

use crate::actor::contracts::{TerminalOutcome, one_tx};
use crate::api::collection::caller_profile;
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, bad_request, ok};
use crate::api::ranked::{
    RankedGameInput, RankedPlan, RankedSideInput, commit_ranked_game, plan_ranked_game,
};
use crate::api::series_rules::{
    RatingMove, SeriesRefusal, SeriesRefusalReason, abandon_unstarted, already_picked, forfeit_series,
    game_ended, game_seats, new_series, pick_deck, project_series, rate_series, seat_of, series_score,
    timeout_picks,
};
use crate::app::{App, now_ms};
use crate::config::{
    SERIES_START_GIVE_UP_SECONDS, SERIES_START_GRACE_SECONDS, SERIES_SWEEP_INTERVAL_SECONDS,
    SERIES_WRITE_ATTEMPTS,
};
use crate::db::store::{
    MatchSeat, RatedGameKind, RatedReason, SeriesRow, SeriesSeat, SeriesStatus, StartMatchInput, StoreError,
    Tx,
};

pub use crate::api::series_rules::NewSeriesInput;

/// Unit conversion, not configuration: the series constants are stated in seconds.
const MS_PER_SECOND: i64 = 1000;

// Errors

/// What a series write can fail with: a rules refusal, an HTTP answer, a store fault or a plain
/// error.
pub enum SeriesError {
    Refusal(SeriesRefusal),
    Api(ApiError),
    Store(StoreError),
    Other(String),
}

impl std::fmt::Display for SeriesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SeriesError::Refusal(refusal) => f.write_str(&refusal.message),
            SeriesError::Api(error) => f.write_str(&error.message),
            SeriesError::Store(error) => write!(f, "{error}"),
            SeriesError::Other(message) => f.write_str(message),
        }
    }
}

impl std::fmt::Debug for SeriesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl From<SeriesRefusal> for SeriesError {
    fn from(refusal: SeriesRefusal) -> SeriesError {
        SeriesError::Refusal(refusal)
    }
}

impl From<ApiError> for SeriesError {
    fn from(error: ApiError) -> SeriesError {
        SeriesError::Api(error)
    }
}

impl From<StoreError> for SeriesError {
    fn from(error: StoreError) -> SeriesError {
        SeriesError::Store(error)
    }
}

fn other(error: impl std::fmt::Display) -> SeriesError {
    SeriesError::Other(error.to_string())
}

/// What the router made of a handler's error: an `ApiError` as it is, anything else logged and
/// answered 500 "something went wrong".
fn to_api(error: SeriesError) -> ApiError {
    match error {
        SeriesError::Api(error) => error,
        SeriesError::Refusal(refusal) => refusal_to_api(&refusal),
        other => {
            tracing::warn!(event = "handler.threw", message = %other);
            ApiError::new(ApiErrorCode::Internal, "something went wrong")
        }
    }
}

// Making a series

/// Makes the series in game 1's pick phase and persists it (R331, R333, R263). Called by pairing
/// (`queue.rs`) and by a Conquest room's join (`actor/rooms.rs`) with the match id they reserved,
/// which becomes game 1's. No match starts and no in-match flag is set: a series in its pick phase
/// is not a match, and `assertNotInSeries` is what keeps its players out of the queue meanwhile.
///
/// It writes inside the caller's transaction `t`, so the claim and the series land together.
pub async fn start_series(
    _app: &App,
    input: NewSeriesInput,
    t: &mut Tx<'_>,
) -> Result<SeriesRow, StoreError> {
    let now = now_ms();
    let series = new_series(&input, now);
    t.series_create(&series).await?;
    // §9.5: a player in a series is in no queue. A series can end with no game played (R333, R334),
    // and then no match result would cancel a ticket they joined (or hosted a room) while one
    // waited, so it would pair them into a match they stopped waiting for. So it goes now.
    for side in [&series.sides.0, &series.sides.1] {
        if let Some(stale) = t.tickets_open_for_profile(&side.profile_id).await? {
            t.tickets_cancel(&stale.id, now).await?;
        }
    }
    tracing::info!(
        event = "series.started",
        seriesId = %series.id,
        players = ?[&series.sides.0.profile_id, &series.sides.1.profile_id],
        matchId = %series.next_match_id,
        pickDeadline = ?series.pick_deadline,
    );
    Ok(series)
}

// Starting a game

/// Starts the game `series.next_match_id` names when the series is `playing` and that match is
/// neither running in this process nor written to the store (R263). Safe to call from several
/// places at once (the pick request, the result whose automatic picks began the game (R332), the
/// sweeper): a start that loses finds the row the winner wrote and stops.
pub async fn ensure_series_game(app: &Arc<App>, series: &SeriesRow) -> Result<(), SeriesError> {
    start_series_game(app, series).await.map(|_| ())
}

/// `ensure_series_game`, answering whether this call is the one that started the match.
async fn start_series_game(app: &Arc<App>, series: &SeriesRow) -> Result<bool, SeriesError> {
    if !matches!(series.status, SeriesStatus::Playing) {
        return Ok(false);
    }
    let match_id = series.next_match_id.clone();
    if app.matches.has(&match_id) {
        return Ok(false);
    }

    let seated = game_seats(series)?;
    let (seats, seed) = (seated.seats, seated.seed);
    let existing = one_tx!(app.db, |t| t.matches_get(&match_id).await?)?;
    if let Some(existing) = existing {
        if matches!(existing.status, crate::db::store::MatchStatus::Live) {
            // Started, and then the process stopped before the in-match flags were written: the match
            // itself rebuilds from its log when a socket arrives, so only the flags need healing.
            mark_in_match(app, &seats, &match_id).await?;
            return Ok(false);
        }
        // The match ended but the series still names it as the game in play. `results.rs` advances
        // the series in the same transaction as the result, so this is a result that was never
        // written. Re-read first: the row we were handed may simply be older than the result.
        let fresh = one_tx!(app.db, |t| t.series_get(&series.id).await?)?;
        if let Some(fresh) = fresh
            && matches!(fresh.status, SeriesStatus::Playing)
            && fresh.next_match_id == match_id
        {
            tracing::error!(event = "series.game_unrecorded", seriesId = %series.id, matchId = %match_id);
        }
        return Ok(false);
    }

    let started = app
        .matches
        .start(
            app,
            StartMatchInput {
                match_id: match_id.clone(),
                seed: seed.clone(),
                catalog_version: series.catalog_version.clone(),
                // A missing flag is unranked (a pre-0022 row, or a room's series): it never rates.
                ranked: series.ranked.unwrap_or(false),
                seats: seats.clone(),
                mode: None,
                stake: None,
            },
        )
        .await;
    if let Err(error) = started {
        // Another start got there first — a request and the sweeper, or a second process — and wrote
        // the row this one was about to write. That start sets the flags.
        if one_tx!(app.db, |t| t.matches_get(&match_id).await?)?.is_some() {
            return Ok(false);
        }
        return Err(SeriesError::Api(error));
    }

    // After the start, not before: for every game after the first the match row is what `profiles`'
    // in-match reference points at, and only the start writes it.
    mark_in_match(app, &seats, &match_id).await?;
    tracing::info!(
        event = "series.game_started",
        seriesId = %series.id,
        matchId = %match_id,
        gameNo = series.games.len(),
        seed = %seed,
    );
    Ok(true)
}

/// §9.5's in-match state for both players of a series game, in one transaction.
async fn mark_in_match(app: &App, seats: &(MatchSeat, MatchSeat), match_id: &str) -> Result<(), SeriesError> {
    let ids = vec![seats.0.profile_id.clone(), seats.1.profile_id.clone()];
    one_tx!(app.db, |t| {
        let profiles = t.profiles_get_many(&ids).await?;
        for profile in &profiles {
            if profile.in_match_id.as_deref() != Some(match_id) {
                t.profiles_set_in_match(&profile.id, Some(match_id)).await?;
            }
        }
    })?;
    Ok(())
}

/// After a commit that may have left the series `playing`, start its game. A failure is logged and
/// left to the sweeper (R263): the write that led here has committed and must not be reported as
/// failed because the match behind it could not start yet.
pub async fn resume_series(app: &Arc<App>, series: Option<&SeriesRow>) {
    let Some(series) = series else { return };
    if !matches!(series.status, SeriesStatus::Playing) {
        return;
    }
    if let Err(error) = start_series_game(app, series).await {
        tracing::warn!(
            event = "series.start_failed",
            seriesId = %series.id,
            matchId = %series.next_match_id,
            message = %error,
        );
    }
}

// Writing a transition

/// R262, R604: plans a ranked series' one rating move as a game between its two sides, or `None` for
/// a series that does not move the rating: unranked, abandoned, or not over.
async fn plan_series_rating(
    t: &mut Tx<'_>,
    app: &App,
    series: &SeriesRow,
) -> Result<Option<RankedPlan>, SeriesError> {
    let score = series_score(series);
    let (Some(score), Some(end_reason), true) = (score, series.end_reason, series.ranked.unwrap_or(false))
    else {
        return Ok(None);
    };
    let (p1, p2) = &series.sides;
    let input = RankedGameInput {
        id: series.id.clone(),
        kind: RatedGameKind::Series,
        catalog_version: series.catalog_version.clone(),
        sides: (
            RankedSideInput::Player {
                profile_id: p1.profile_id.clone(),
            },
            RankedSideInput::Player {
                profile_id: p2.profile_id.clone(),
            },
        ),
        winner_side: if score == 0.5 {
            None
        } else if score == 1.0 {
            Some(0)
        } else {
            Some(1)
        },
        reason: RatedReason::Series(end_reason),
        at: series.ended_at.unwrap_or_else(now_ms),
        stake: None,
    };
    plan_ranked_game(t, app, &input).await.map(Some).map_err(other)
}

/// Writes `next` over `before` inside the caller's transaction, and with it everything an ending
/// owes: R262's rating move, recorded on the row, and — for a series that ends before game 1 was
/// played — the reserved match id released (R263). `None` when the compare-and-set lost; nothing has
/// been written then, so the caller may re-read and try again in the same transaction.
async fn commit_series(
    t: &mut Tx<'_>,
    app: &App,
    before: &SeriesRow,
    next: SeriesRow,
) -> Result<Option<SeriesRow>, SeriesError> {
    let ends = matches!(next.status, SeriesStatus::Over) && !matches!(before.status, SeriesStatus::Over);
    // Planned before the compare-and-set and written only after it wins: a lost write must leave no
    // rating behind, since the caller retries inside this same transaction.
    let plan = if ends {
        plan_series_rating(t, app, &next).await?
    } else {
        None
    };
    let row = if ends {
        rate_series(
            &next,
            plan.as_ref().map(|plan| RatingMove {
                before: (plan.row.sides.0.before.rating, plan.row.sides.1.before.rating),
                after: (plan.row.sides.0.after.rating, plan.row.sides.1.after.rating),
            }),
        )
    } else {
        next
    };

    if !t.series_update(&row).await? {
        return Ok(None);
    }
    if !ends {
        return Ok(Some(row));
    }

    if let Some(plan) = &plan {
        commit_ranked_game(t, plan, row.ended_at.unwrap_or_else(now_ms))
            .await
            .map_err(other)?;
    }
    if row.games.is_empty() {
        t.matches_discard_open(&row.next_match_id).await?;
    }

    tracing::info!(
        event = "series.ended",
        seriesId = %row.id,
        winner = ?row.winner,
        endReason = ?row.end_reason,
        games = row.games.len(),
        ratingBefore = ?row.rating_before,
        ratingAfter = ?row.rating_after,
    );
    Ok(Some(row))
}

/// The row a transition was computed from, and the row it wrote.
#[derive(Clone, Debug)]
pub struct Written {
    pub before: SeriesRow,
    pub after: SeriesRow,
}

/// One transition, written by compare-and-set (R263). Each attempt reads the row, applies
/// `transition` and writes it in one transaction; a lost write is retried on a fresh read. A
/// `SeriesRefusal` from the rules is answered as it is. After the commit, a series that has just
/// begun a game gets that game started.
async fn write_transition(
    app: &Arc<App>,
    series_id: &str,
    transition: impl Fn(&SeriesRow) -> Result<SeriesRow, SeriesRefusal>,
) -> Result<Written, SeriesError> {
    let attempts = SERIES_WRITE_ATTEMPTS as i64;
    for _attempt in 1..=attempts {
        let mut t = app.db.begin(None).await?;
        let Some(before) = t.series_get(series_id).await? else {
            return Err(SeriesError::Api(series_not_found()));
        };
        // A refusal returns here, and dropping `t` rolls the attempt back.
        let next = transition(&before)?;
        let after = commit_series(&mut t, app, &before, next).await?;
        t.commit().await?;
        let Some(after) = after else { continue };
        let written = Written { before, after };
        if !matches!(written.before.status, SeriesStatus::Playing) {
            resume_series(app, Some(&written.after)).await;
        }
        return Ok(written);
    }
    tracing::warn!(event = "series.write_contended", seriesId = %series_id, attempts = attempts);
    Err(SeriesError::Api(ApiError::new(
        ApiErrorCode::Conflict,
        "This series changed while your request was on its way. Try again.",
    )))
}

// A game's result (called by `results.rs` inside its transaction)

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SeriesGameResult {
    pub match_id: String,
    /// The match's seats, `.0` being the match's p1 — not necessarily series p1 (R335).
    pub seats: (MatchSeat, MatchSeat),
    pub outcome: TerminalOutcome,
    pub at: i64,
}

/// The series seat that won the match, found by profile id, or `Winner::Draw`.
fn series_winner_of(series: &SeriesRow, result: &SeriesGameResult) -> Result<Winner, SeriesError> {
    let Some(player) = result.outcome.winner.player() else {
        return Ok(Winner::Draw);
    };
    let seat = [&result.seats.0, &result.seats.1]
        .into_iter()
        .find(|candidate| candidate.player == player);
    let winner: Option<SeriesSeat> = seat.and_then(|seat| seat_of(series, &seat.profile_id));
    match winner {
        Some(winner) => Ok(Winner::from(winner)),
        None => Err(SeriesError::Other(format!(
            "series {}: the winner of {} is not one of its players",
            series.id, result.match_id
        ))),
    }
}

/// R334, R263: records the game `result.match_id` in `series` and, when that ends the series, R262's
/// rating move, all inside `t`, the transaction `results.rs` writes the result in, so they commit
/// together or not at all. The next game's match id is minted here, when its pick phase opens.
pub async fn advance_series_in_tx(
    t: &mut Tx<'_>,
    app: &App,
    series: &SeriesRow,
    result: &SeriesGameResult,
) -> Result<SeriesRow, SeriesError> {
    let mut current = series.clone();
    let mut attempt: i64 = 1;
    loop {
        let winner = series_winner_of(&current, result)?;
        let new_match_id = uuid::Uuid::new_v4().to_string();
        let next = game_ended(&current, winner, result.outcome.reason, result.at, &new_match_id)?;
        if let Some(written) = commit_series(t, app, &current, next).await? {
            return Ok(written);
        }

        let reread = t.series_get(&current.id).await?;
        match reread {
            Some(reread)
                if attempt < SERIES_WRITE_ATTEMPTS as i64
                    && matches!(reread.status, SeriesStatus::Playing)
                    && reread.next_match_id == result.match_id =>
            {
                current = reread;
            }
            _ => {
                return Err(SeriesError::Other(format!(
                    "series {}: the result of {} could not be recorded",
                    current.id, result.match_id
                )));
            }
        }
        attempt += 1;
    }
}

// The sweeper (R333, R263)

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SeriesSweep {
    pub timed_out: Vec<String>,
    pub started: Vec<String>,
    pub abandoned: Vec<String>,
}

/// One series of the sweep. `Ok(())` covers "nothing to do".
async fn sweep_one(
    app: &Arc<App>,
    series: &SeriesRow,
    now: i64,
    swept: &mut SeriesSweep,
) -> Result<(), SeriesError> {
    let grace_ms = SERIES_START_GRACE_SECONDS * MS_PER_SECOND;
    let give_up_ms = SERIES_START_GIVE_UP_SECONDS * MS_PER_SECOND;

    if matches!(series.status, SeriesStatus::Picking) {
        match series.pick_deadline {
            Some(deadline) if now >= deadline => {}
            _ => return Ok(()),
        }
        let Written { after, .. } = write_transition(app, &series.id, |row| timeout_picks(row, now)).await?;
        swept.timed_out.push(series.id.clone());
        tracing::info!(
            event = "series.pick_timeout",
            seriesId = %series.id,
            outcome = if matches!(after.status, SeriesStatus::Over) { "abandoned" } else { "auto-picked" },
        );
        return Ok(());
    }
    if !matches!(series.status, SeriesStatus::Playing) {
        return Ok(());
    }
    if now - series.updated_at < grace_ms {
        return Ok(());
    }
    if app.matches.has(&series.next_match_id) {
        return Ok(());
    }
    // R263: a game that has had every chance to start and still has no match row cannot be
    // started, so the series is given up rather than holding both players in it for ever. A
    // live or finished match row means the game did start, and is never given up here.
    if now - series.updated_at >= give_up_ms
        && one_tx!(app.db, |t| t.matches_get(&series.next_match_id).await?)?.is_none()
    {
        write_transition(app, &series.id, |row| abandon_unstarted(row, now)).await?;
        swept.abandoned.push(series.id.clone());
        tracing::error!(event = "series.game_unstartable", seriesId = %series.id, matchId = %series.next_match_id);
        return Ok(());
    }
    if start_series_game(app, series).await? {
        swept.started.push(series.id.clone());
        tracing::warn!(event = "series.game_recovered", seriesId = %series.id, matchId = %series.next_match_id);
    }
    Ok(())
}

/// One sweep over every series that is not over: a pick phase past its deadline is settled (R333);
/// a `playing` series whose match is not running and whose row has sat unchanged for
/// `SERIES_START_GRACE_SECONDS` gets it started (R263), the grace keeping the sweeper from racing
/// the starting request; one still without a match row `SERIES_START_GIVE_UP_SECONDS` after its
/// picks is abandoned, unrated. One series that fails does not stop the sweep.
pub async fn sweep_series(app: &Arc<App>) -> Result<SeriesSweep, StoreError> {
    let now = now_ms();
    let mut swept = SeriesSweep::default();

    let active = one_tx!(app.db, |t| t.series_active().await?)?;
    for series in &active {
        match sweep_one(app, series, now, &mut swept).await {
            Ok(()) => {}
            // Another writer moved the series first: the rules refused a transition that no longer
            // applies, which is the sweep having nothing to do.
            Err(SeriesError::Refusal(_)) => {}
            Err(error) => {
                tracing::warn!(event = "series.sweep_failed", seriesId = %series.id, message = %error);
            }
        }
    }
    Ok(swept)
}

/// R263: the sweeper, every `SERIES_SWEEP_INTERVAL_SECONDS`, spawned at boot by `app.rs`. The wait
/// is `tokio::time`'s, so a test drives it with a paused clock; a failed sweep never stops the next.
pub async fn run_sweeper(app: Arc<App>) {
    let interval =
        Duration::from_millis(u64::try_from(SERIES_SWEEP_INTERVAL_SECONDS * MS_PER_SECOND).unwrap_or(0));
    loop {
        tokio::time::sleep(interval).await;
        if let Err(error) = sweep_series(&app).await {
            tracing::warn!(event = "series.sweeper_failed", message = %error);
        }
    }
}

/// The sweeper task's stop handle.
pub struct SeriesSweeper {
    handle: tokio::task::JoinHandle<()>,
}

impl SeriesSweeper {
    pub fn stop(&self) {
        self.handle.abort();
    }
}

/// `run_sweeper` on its own task, with a handle to stop it.
pub fn start_series_sweeper(app: Arc<App>) -> SeriesSweeper {
    SeriesSweeper {
        handle: tokio::spawn(run_sweeper(app)),
    }
}

// Routes

/// One answer for a missing series and one the caller is not in, so an id reveals nothing.
fn series_not_found() -> ApiError {
    ApiError::new(ApiErrorCode::NotFound, "This series could not be found.")
}

/// A rules refusal as the player hears it: a bad slot is their request, the rest is timing.
fn refusal_to_api(refusal: &SeriesRefusal) -> ApiError {
    if matches!(
        refusal.reason,
        SeriesRefusalReason::SlotOutOfRange | SeriesRefusalReason::SlotWon
    ) {
        return bad_request(&refusal.message);
    }
    ApiError::new(ApiErrorCode::Conflict, &refusal.message)
}

fn param<'a>(req: &'a Req, name: &str) -> &'a str {
    req.params.get(name).map(String::as_str).unwrap_or("")
}

/// The caller's series and seat, or the 404 a stranger gets.
async fn callers_series(req: &Req, app: &App) -> Result<(SeriesRow, SeriesSeat, String), ApiError> {
    let profile = caller_profile(req)?;
    let id = param(req, "id").to_string();
    let series = one_tx!(app.db, |t| t.series_get(&id).await?).map_err(|error| to_api(error.into()))?;
    let seated = series.and_then(|series| seat_of(&series, &profile.id).map(|seat| (series, seat)));
    match seated {
        Some((series, seat)) => Ok((series, seat, profile.id.clone())),
        None => Err(series_not_found()),
    }
}

fn body_field<'a>(body: &'a Value, key: &str) -> Option<&'a Value> {
    body.as_object()
        .and_then(|object: &Map<String, Value>| object.get(key))
}

/// A JSON number with no fraction, as `Number.isInteger` reads it.
fn whole_number(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .filter(|number| number.is_finite() && number.fract() == 0.0)
}

fn slot_of(body: &Value) -> Result<i32, ApiError> {
    match body_field(body, "slot").and_then(whole_number) {
        Some(slot) => Ok(slot as i32),
        None => Err(bad_request("Pick one of the three decks in your trio.")),
    }
}

/// R331: the game a pick is for, when the request names it (the client always does).
fn game_no_of(body: &Value) -> Result<Option<i32>, ApiError> {
    let value = match body_field(body, "gameNo") {
        None | Some(Value::Null) => return Ok(None),
        Some(value) => value,
    };
    match whole_number(value) {
        Some(game_no) if game_no >= 1.0 => Ok(Some(game_no as i32)),
        _ => Err(bad_request(
            r#""gameNo" must be the number of the game the pick is for"#,
        )),
    }
}

/// A transition requested by a player: refusals become their HTTP answers.
async fn player_transition(
    app: &Arc<App>,
    series_id: &str,
    transition: impl Fn(&SeriesRow) -> Result<SeriesRow, SeriesRefusal>,
) -> Result<SeriesRow, ApiError> {
    match write_transition(app, series_id, transition).await {
        Ok(written) => Ok(written.after),
        Err(SeriesError::Refusal(refusal)) => Err(refusal_to_api(&refusal)),
        Err(error) => Err(to_api(error)),
    }
}

fn view(series: &SeriesRow, profile_id: &str) -> Value {
    serde_json::to_value(project_series(series, profile_id, now_ms())).unwrap_or(Value::Null)
}

/// `GET /api/series/:id` (active): the caller's view of the series (R336). 404 when it does not
/// exist or they are not in it.
pub async fn get_series(app: &Arc<App>, req: Req) -> ApiResult {
    let (series, _seat, profile_id) = callers_series(&req, app).await?;
    Ok(ok(view(&series, &profile_id)))
}

/// `POST /api/series/:id/pick` (active), R331: `{ slot, gameNo? }`. The pick is sealed and shown to
/// the other side only as "picked"; the one that completes both starts the game and the answer
/// names it in `currentMatchId`. The same slot sent again is answered with the current view as the
/// success it was; a pick naming another game is never applied. 409 for a different slot once a
/// pick is in, another game's pick, a game in play, or after the pick clock ran out (R333); 400 for
/// a slot out of range or whose deck has won (R330).
pub async fn pick(app: &Arc<App>, req: Req) -> ApiResult {
    let slot = slot_of(&req.body)?;
    let game_no = game_no_of(&req.body)?;
    let (series, seat, profile_id) = callers_series(&req, app).await?;
    let now = now_ms();
    match write_transition(app, &series.id, |row| pick_deck(row, seat, slot, now, game_no)).await {
        Ok(Written { after, .. }) => {
            tracing::info!(event = "series.picked", seriesId = %series.id, seat = %seat, status = ?after.status);
            Ok(ok(view(&after, &profile_id)))
        }
        Err(SeriesError::Refusal(refusal)) => {
            if matches!(
                refusal.reason,
                SeriesRefusalReason::PickSealed
                    | SeriesRefusalReason::NotPicking
                    | SeriesRefusalReason::StalePick
            ) {
                let current = one_tx!(app.db, |t| t.series_get(&series.id).await?)
                    .map_err(|error| to_api(error.into()))?;
                if let Some(current) = current
                    && already_picked(&current, seat, slot, game_no)
                {
                    return Ok(ok(view(&current, &profile_id)));
                }
            }
            Err(refusal_to_api(&refusal))
        }
        Err(error) => Err(to_api(error)),
    }
}

/// `POST /api/series/:id/forfeit` (active): R334: leave the series between games; the other side
/// wins it. 409 during a game.
pub async fn forfeit(app: &Arc<App>, req: Req) -> ApiResult {
    let (series, seat, profile_id) = callers_series(&req, app).await?;
    let now = now_ms();
    let after = player_transition(app, &series.id, |row| forfeit_series(row, seat, now)).await?;
    Ok(ok(view(&after, &profile_id)))
}

/// `GET /api/matches/:matchId/series` (active): the series a match was a game of, for the board's
/// series banner once the game ends: `null` when the match is not a series game or the caller is not
/// one of its players.
pub async fn match_series(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let match_id = param(&req, "matchId").to_string();
    let series =
        one_tx!(app.db, |t| t.series_with_game(&match_id).await?).map_err(|error| to_api(error.into()))?;
    let projected = series.and_then(|series| project_series(&series, &profile.id, now_ms()));
    Ok(ok(json!({ "series": projected })))
}
