//! Online replays (SPEC §9.3, R768, #510).
//!
//! `GET /api/replays?offset=` lists caller's finished matches within `MATCH_ACTION_RETENTION_DAYS`,
//! paginated by `REPLAY_LIST_PAGE` with portraits (R641, R642) and opponent usernames (R1436).
//! Flags unplayable replays under unsupported catalog versions (R767000, #1825).
//!
//! `GET /api/replays/:matchId?from=&count=` streams seat-filtered steps (R677, R97, CLAUDE.md rule 7),
//! loadouts (R1834002, R1834003), and support for shared replays (B52).
//! Replays fold under original catalog versions and All Random deals (R433, R112).
//!
//! Cached in memory (up to `REPLAY_CACHE_MATCHES`, `REPLAY_CACHE_TTL_SECONDS`), rate limited
//! to `REPLAY_REQUESTS_PER_MINUTE` per account (R109). An opening not kept in memory folds a whole
//! game, so those run behind one server-wide permit pool, 1v1 and co-op together
//! (`REPLAY_FOLDS_RUNNING_MAX`): concurrent openings of one match share a single fold, and one
//! that waits past `REPLAY_FOLD_WAIT_MS` for it or for its turn is 429 with `retryAfterMs` (R3500000,
//! R192). A cached page waits for nothing.

use std::collections::BTreeSet;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use indexmap::IndexMap;
use jackioh_engine::config::{REPLAY_CHECKPOINT_EVERY, REPLAY_PAGE_STEPS};
use jackioh_engine::{
    Action, DEFAULT_PORTRAIT, GameOverReason, PerPlayer, PlayerId, PortraitId, ReplayCheckpoints, ReplayOpen,
    ReplayRecord, ReplayRefusal, ReplayStep, Winner,
};
use serde::Serialize;
use serde_json::json;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::{Instant, timeout_at};

use crate::actor::engine::{self, EngineState, Pinned};
use crate::actor::match_actor::last_boards_of;
use crate::actor::registry::dealt_for;
use crate::api::collection::caller_profile;
use crate::api::http::{
    ApiError, ApiErrorCode, ApiResult, RateLimiter, Req, account_key, bad_request, create_rate_limiter, lock,
    log_warn, now_ms, ok_of, rate_limited,
};
use crate::app::App;
use crate::config::{
    MS_PER_SECOND, REPLAY_CACHE_MATCHES, REPLAY_CACHE_TTL_SECONDS, REPLAY_FOLD_RETRY_AFTER_MS,
    REPLAY_FOLD_WAIT_MS, REPLAY_FOLDS_RUNNING_MAX, REPLAY_LIST_PAGE, REPLAY_REQUESTS_PER_MINUTE,
};
use crate::db::store::{MatchPhase, Profile, QueueMode, ReplayRow, ResultRow};
use crate::loadout::{Loadout, SeatCards, SeatLoadout, cards_due, shown_defs};

/// Unit conversion, not configuration.
const MS_PER_MINUTE: i64 = 60 * MS_PER_SECOND;

/// R3500000: what an opening refused past `REPLAY_FOLD_WAIT_MS` says.
pub const REPLAY_BUSY_MESSAGE: &str = "other replays are opening right now; try again in a moment";

/// What the replay routes keep between requests, one per `App`: the opened replays, their own
/// allowance and the permits of the folds that run (R3500000), which the co-op replays take too.
pub struct Replays {
    pub cache: ReplayCache,
    pub limiter: RateLimiter,
    pub folds: Arc<Semaphore>,
}

impl Default for Replays {
    fn default() -> Replays {
        Replays {
            cache: ReplayCache::default(),
            limiter: create_rate_limiter(REPLAY_REQUESTS_PER_MINUTE, MS_PER_MINUTE),
            folds: Arc::new(Semaphore::new(REPLAY_FOLDS_RUNNING_MAX)),
        }
    }
}

/// The opened replays by match id, the one used longest ago first, each with when it was opened: the 1v1
/// replays (`ReplayOpen`) and, as `duos::replay::DuosReplayCache`, the co-op ones. They hold whole states, so
/// nothing but their routes reads them (CLAUDE.md rule 7). The lock is never held across a fold.
pub struct ReplayCache<T = ReplayOpen> {
    entries: Mutex<IndexMap<String, (i64, Arc<T>)>>,
    /// R3500000: a turn for each match being opened, so concurrent openings of it fold it once:
    /// the one that holds the turn folds, the others wait for it and find its replay kept. An entry
    /// lives while a request holds or waits on its turn (`Flight`).
    flights: Mutex<IndexMap<String, Arc<tokio::sync::Mutex<()>>>>,
}

impl<T> Default for ReplayCache<T> {
    fn default() -> Self {
        ReplayCache {
            entries: Mutex::new(IndexMap::new()),
            flights: Mutex::new(IndexMap::new()),
        }
    }
}

/// A request's hold on its match's entry in `ReplayCache::flights`: the last one to let go removes
/// the entry. A handle is taken under the map's lock, so a count of two (the map's and this one's)
/// is the last.
struct Flight<'a, T> {
    cache: &'a ReplayCache<T>,
    key: &'a str,
    turn: Arc<tokio::sync::Mutex<()>>,
}

impl<T> Drop for Flight<'_, T> {
    fn drop(&mut self) {
        let mut flights = lock(&self.cache.flights);
        if Arc::strong_count(&self.turn) <= 2 {
            flights.shift_remove(self.key);
        }
    }
}

impl<T> ReplayCache<T> {
    /// Drops every replay opened `REPLAY_CACHE_TTL_SECONDS` or more before `now`.
    fn expire(entries: &mut IndexMap<String, (i64, Arc<T>)>, now: i64) {
        entries.retain(|_, (opened_at, _)| now - *opened_at < REPLAY_CACHE_TTL_SECONDS * MS_PER_SECOND);
    }

    /// The replay of `match_id` opened less than `REPLAY_CACHE_TTL_SECONDS` ago, now the one used
    /// last.
    pub fn get(&self, match_id: &str, now: i64) -> Option<Arc<T>> {
        let mut entries = lock(&self.entries);
        Self::expire(&mut entries, now);
        let entry = entries.shift_remove(match_id)?;
        let open = Arc::clone(&entry.1);
        entries.insert(match_id.to_string(), entry);
        Some(open)
    }

    /// Keeps the replay of `match_id`, opened at `now`, dropping the ones used longest ago to stay
    /// within `REPLAY_CACHE_MATCHES`.
    pub fn put(&self, match_id: &str, open: Arc<T>, now: i64) {
        let mut entries = lock(&self.entries);
        Self::expire(&mut entries, now);
        entries.shift_remove(match_id);
        while entries.len() >= REPLAY_CACHE_MATCHES {
            entries.shift_remove_index(0);
        }
        entries.insert(match_id.to_string(), (now, open));
    }

    /// How many replays are kept at `now`.
    pub fn held(&self, now: i64) -> usize {
        let mut entries = lock(&self.entries);
        Self::expire(&mut entries, now);
        entries.len()
    }

    /// Forgets every opened replay (the retention purge took logs they were folded from).
    pub fn clear(&self) {
        lock(&self.entries).clear();
    }

    /// The hold on `key`'s turn, the entry made if no request has one. Never held across an await.
    fn flight<'a>(&'a self, key: &'a str) -> Flight<'a, T> {
        let turn = Arc::clone(lock(&self.flights).entry(key.to_string()).or_default());
        Flight {
            cache: self,
            key,
            turn,
        }
    }

    /// R3500000: the replay of `match_id`, from memory or else folded by `fold` and kept. A
    /// request waits for the turn on its match (a fold of it already running is then found kept,
    /// and folded once) and for a permit of `folds`, `REPLAY_FOLD_WAIT_MS` in all, and is refused
    /// with 429 past it (R192). `fold` is given the permit and holds it as long as the fold runs,
    /// so one whose request is dropped mid-fold still counts: its replay is lost and the next
    /// request folds the match again.
    pub async fn open<F, Fut>(
        &self,
        folds: &Arc<Semaphore>,
        match_id: &str,
        fold: F,
    ) -> Result<Arc<T>, ApiError>
    where
        F: FnOnce(OwnedSemaphorePermit) -> Fut,
        Fut: Future<Output = Result<T, ApiError>>,
    {
        let busy = || rate_limited(REPLAY_BUSY_MESSAGE, REPLAY_FOLD_RETRY_AFTER_MS);
        let deadline = Instant::now() + Duration::from_millis(REPLAY_FOLD_WAIT_MS);
        // Declared before the guard so it drops after it, and sees the turn let go.
        let flight = self.flight(match_id);
        let _turn = timeout_at(deadline, Arc::clone(&flight.turn).lock_owned())
            .await
            .map_err(|_| busy())?;
        if let Some(open) = self.get(match_id, now_ms()) {
            return Ok(open);
        }
        let permit = timeout_at(deadline, Arc::clone(folds).acquire_owned())
            .await
            .map_err(|_| busy())?
            .map_err(|_| ApiError::internal("the replay folds' permits are closed"))?;
        let open = Arc::new(fold(permit).await?);
        self.put(match_id, Arc::clone(&open), now_ms());
        Ok(open)
    }
}

/// The result of a listed match for the caller, as practice's listing reads it.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReplayResult {
    Win,
    Loss,
    Draw,
}

/// One finished match in `GET /api/replays`.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReplayListing {
    pub match_id: String,
    /// The mode it was made in (R257), or null for a match nothing records the mode of.
    pub mode: Option<QueueMode>,
    pub ended_at: i64,
    /// The seat the caller began in, whose view is step 0.
    pub seat: PlayerId,
    pub result: ReplayResult,
    pub turns: i64,
    /// Step 0 and one per logged action: the log holds only the actions `reduce` accepted (§9.3).
    pub steps: i64,
    /// R641, R642: the portraits the seats were dealt, `vanilla` for a match from before them.
    pub portraits: PerPlayer<PortraitId>,
    pub opponent: ReplayOpponent,
    /// R768: why this build cannot show it, when it cannot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unavailable: Option<ReplayRefusal>,
}

/// The other seat of a listed match, as the match screen shows a seat (R1436: keyed by profile id,
/// shown by username). Both are empty for a seat a deleted account left in Postgres (KNOWN
/// DIVERGENCES in `db/pg.rs`); the memory store keeps the id and finds no username.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReplayOpponent {
    pub profile_id: String,
    pub username: String,
}

/// `GET /api/replays`: one page, and where the next starts (null on the last page).
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReplayList {
    pub replays: Vec<ReplayListing>,
    pub next_offset: Option<i64>,
}

/// `GET /api/replays/:matchId`: steps `[from, from + steps.len())` of the caller's replay, of
/// `total`.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReplaySteps {
    pub match_id: String,
    pub from: usize,
    pub steps: Vec<ReplayStep>,
    pub total: usize,
    /// R1834002: both seats' seat-level loadouts, as the `loadouts` frame carries them.
    pub loadouts: PerPlayer<SeatLoadout>,
    /// R1834003: the card picks of the cards the page's steps show, by seat, as the `cardLoadouts`
    /// frames carry them; a card none of its steps shows is not here.
    pub card_loadouts: Vec<SeatCards>,
}

/// Both routes' own allowance, counted per account like R109's.
fn check_rate(app: &App, profile_id: &str) -> Result<(), ApiError> {
    let key = account_key(profile_id);
    let now = now_ms();
    if app.replays.limiter.allow(&key, now) {
        return Ok(());
    }
    log_warn("replays.rate_limited", json!({ "key": key }));
    Err(rate_limited(
        "too many replay requests; slow down",
        app.replays.limiter.retry_after_ms(&key, now),
    ))
}

/// A whole-number query parameter, `default` when it is absent.
fn query_number(req: &Req, name: &str, default: usize) -> Result<usize, ApiError> {
    match req.query.get(name) {
        None => Ok(default),
        Some(text) => text
            .parse()
            .map_err(|_| bad_request(format!("{name} must be a whole number"))),
    }
}

/// The one answer for a match that does not exist and for one the caller held no seat in.
fn no_such_replay() -> ApiError {
    ApiError::new(ApiErrorCode::NotFound, "no such replay")
}

/// §2.5: the endings that always name a winner, as `DECISIVE_REASONS` in `db/pg.rs` lists them: a
/// hero's death, a concede, a disconnect, an effect's win (R1412), a player voted out (R1843000) and
/// a flight that chose a loss for the other player (R1843002). `alt-win` is a draw when both players
/// held a win (R850), and `fled` when both fled (R1843002); every other ending is a draw.
fn names_a_winner(reason: GameOverReason) -> bool {
    matches!(
        reason,
        GameOverReason::HeroDeath
            | GameOverReason::Concede
            | GameOverReason::Disconnect
            | GameOverReason::WonByEffect
            | GameOverReason::VotedOut
            | GameOverReason::FledDecisive
    )
}

/// Whether the results row records an ending `winner` won (R677: its players are the seats as the
/// game ended). A winnerless row whose reason names a winner is one whose winner has deleted their
/// account since, which empties both the winner and that seat (migration 0012), so the emptied
/// seat won; a winnerless `alt-win` or `fled` is a draw or that.
fn records_winner(result: &ResultRow, winner: Winner) -> bool {
    let seat_of = |profile_id: &str| {
        if profile_id == result.players.0 {
            Some(Winner::P1)
        } else if profile_id == result.players.1 {
            Some(Winner::P2)
        } else {
            None
        }
    };
    match result.winner_profile_id.as_deref() {
        Some(won) => seat_of(won) == Some(winner),
        None if names_a_winner(result.reason) => seat_of("") == Some(winner),
        None if matches!(result.reason, GameOverReason::AltWin | GameOverReason::Fled) => {
            winner == Winner::Draw || seat_of("") == Some(winner)
        }
        None => winner == Winner::Draw,
    }
}

/// The caller's result, as their record counts it (`results_record_for`): a winnerless row is a
/// loss when its reason names a winner, and a draw otherwise; R1843004: the other player's `fled` is
/// a draw for the caller.
fn result_for(result: &ResultRow, profile_id: &str) -> ReplayResult {
    match result.winner_profile_id.as_deref() {
        Some(winner) if winner == profile_id => ReplayResult::Win,
        Some(_) if jackioh_engine::config::LOSER_DRAWS_REASONS.contains(&result.reason) => ReplayResult::Draw,
        Some(_) => ReplayResult::Loss,
        None if names_a_winner(result.reason) => ReplayResult::Loss,
        None => ReplayResult::Draw,
    }
}

/// R768: a match with no recorded hash is held to its results row: the fold ends with the same
/// winner, reason and turn count.
fn ends_as_recorded(state: &EngineState, result: &ResultRow) -> bool {
    let snapshot = engine::snapshot(state);
    let Some(ended) = snapshot.result else {
        return false;
    };
    records_winner(result, ended.winner)
        && ended.reason == result.reason
        && i64::from(snapshot.turn) == result.turns
}

fn rules_changed() -> ReplayOpen {
    ReplayOpen::Refused {
        reason: ReplayRefusal::RulesChanged,
    }
}

/// R768: the match's log folded once and checked, as the actor's rebuild folds it, under the catalog
/// version it was played on (R767000). Run off the runtime (`open_uncached`); panics on a setup
/// `create_game` refuses.
fn open_folded(replay: &ReplayRow, log: Vec<Action>, mode: Option<QueueMode>) -> ReplayOpen {
    let row = &replay.row;
    // Refused before anything is folded, the legacy check's fold included.
    let pinned = match Pinned::of(&row.catalog_version) {
        Ok(pinned) => pinned,
        Err(reason) => return ReplayOpen::Refused { reason },
    };
    let (last_boards, glitch_boards) = last_boards_of(row);
    let args = engine::fold_args(
        &row.seed,
        &row.decks,
        log,
        last_boards,
        glitch_boards,
        dealt_for(mode),
        row.win_rates.clone(),
    );
    pinned.run(|| {
        let final_hash = match &row.final_hash {
            Some(final_hash) => final_hash.clone(),
            None => {
                let folded = engine::fold(pinned, &args);
                if !ends_as_recorded(&folded.state, &replay.result) {
                    return rules_changed();
                }
                engine::hash_state(&folded.state)
            }
        };
        engine::replay_open(
            &args,
            &ReplayRecord {
                catalog_version: row.catalog_version.clone(),
                final_hash,
            },
        )
    })
}

/// R768: `open_folded` in one job off the runtime (#1035), as Duos's replay opens: a long log is a
/// long fold. R3500000: the job holds `permit` until it ends, even if its request is dropped.
async fn open_uncached(
    app: &App,
    replay: &ReplayRow,
    log: Vec<Action>,
    mode: Option<QueueMode>,
    permit: OwnedSemaphorePermit,
) -> ReplayOpen {
    let folding = replay.clone();
    let opened = engine::off_runtime(app.engine_thread, move || {
        let _permit = permit;
        open_folded(&folding, log, mode)
    })
    .await;
    opened.unwrap_or_else(|message| {
        // A setup `create_game` refuses: the frozen decks no longer pass this build's checks.
        tracing::error!(event = "replay.open.panicked", matchId = %replay.row.id, message = %message);
        rules_changed()
    })
}

/// The replay of a match the caller may read: the one kept in memory, else `log` folded and kept,
/// behind R3500000's turn and permit (429 past `REPLAY_FOLD_WAIT_MS`).
async fn open_replay(
    app: &App,
    replay: &ReplayRow,
    kept: Option<Arc<ReplayOpen>>,
    log: Vec<Action>,
    mode: Option<QueueMode>,
) -> Result<Arc<ReplayOpen>, ApiError> {
    if let Some(open) = kept {
        return Ok(open);
    }
    app.replays
        .cache
        .open(&app.replays.folds, &replay.row.id, |permit| async move {
            Ok(open_uncached(app, replay, log, mode, permit).await)
        })
        .await
}

/// R768000: why a listed match cannot be shown, as far as is known without folding it (#1825): a
/// catalog version this build neither is nor carries (R767000), else the refusal of an opening
/// still kept. Any other match lists as one to open, and opening it says why if it cannot be
/// shown, as practice's list does.
fn known_refusal(app: &App, replay: &ReplayRow, now: i64) -> Option<ReplayRefusal> {
    if let Err(reason) = Pinned::of(&replay.row.catalog_version) {
        return Some(reason);
    }
    match app.replays.cache.get(&replay.row.id, now)?.as_ref() {
        ReplayOpen::Refused { reason } => Some(*reason),
        ReplayOpen::Ready { .. } => None,
    }
}

/// R677: the seat the account that began in `home` plays at step `from`, and for how many steps
/// from there, up to `count`, it goes on playing it, so a page never spans a Glitch's swap and every
/// step of it is one seat's view. Read off the states as `replay_page` reads them, from the kept state
/// at or before `from`. `from` is a step of the replay and `count` at least 1.
fn seat_run(
    pinned: Pinned,
    checkpoints: &ReplayCheckpoints,
    home: PlayerId,
    from: usize,
    count: usize,
) -> (PlayerId, usize) {
    let end = (from + count).min(checkpoints.accepted.len() + 1);
    let mut at = from - from % REPLAY_CHECKPOINT_EVERY;
    let Some(mut state) = checkpoints.states.get(at / REPLAY_CHECKPOINT_EVERY).cloned() else {
        return (home, 1);
    };
    while at < from {
        state = engine::reduce(pinned, &state, &checkpoints.accepted[at]).state;
        at += 1;
    }
    let seat = engine::seat_played_by(&state, home);
    let mut run = 1;
    while at + 1 < end {
        state = engine::reduce(pinned, &state, &checkpoints.accepted[at]).state;
        at += 1;
        if engine::seat_played_by(&state, home) != seat {
            break;
        }
        run += 1;
    }
    (seat, run)
}

/// The seat `profile_id` began the match in (the row's players are the seats as it began).
fn home_seat(replay: &ReplayRow, profile_id: &str) -> PlayerId {
    if replay.row.players.0 == profile_id {
        PlayerId::P1
    } else {
        PlayerId::P2
    }
}

/// The profile id in the seat `profile_id` did not begin in.
fn opponent_of<'a>(replay: &'a ReplayRow, profile_id: &str) -> &'a str {
    match home_seat(replay, profile_id) {
        PlayerId::P1 => &replay.row.players.1,
        PlayerId::P2 => &replay.row.players.0,
    }
}

/// `GET /api/replays?offset=`.
pub async fn list_replays(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    check_rate(app, &profile.id)?;
    let offset = query_number(&req, "offset", 0)?;
    let offset = i64::try_from(offset).map_err(|_| bad_request("offset is too large"))?;
    let page = REPLAY_LIST_PAGE as i64;

    // One past the page, to know whether another follows, each row already paired with its mode
    // in that one batched read. No row's log is read and no replay opened: `known_refusal` folds
    // nothing (#1825).
    let now = now_ms();
    let mut tx = app.db.begin(None).await?;
    let mut entries = tx.replays_list_with_modes(&profile.id, page + 1, offset).await?;
    let more = entries.len() > REPLAY_LIST_PAGE;
    entries.truncate(REPLAY_LIST_PAGE);
    // R1436: every read joins the username in, so a rename shows on past games too.
    let opponents: Vec<String> = entries
        .iter()
        .map(|entry| opponent_of(&entry.replay, &profile.id))
        .filter(|opponent| !opponent.is_empty())
        .map(str::to_string)
        .collect();
    let profiles: Vec<Profile> = tx.profiles_get_many(&opponents).await?;
    tx.commit().await?;

    let mut replays = Vec::with_capacity(entries.len());
    for entry in &entries {
        let (replay, mode) = (&entry.replay, entry.mode);
        let opponent = opponent_of(replay, &profile.id);
        let portraits = replay
            .row
            .portraits
            .unwrap_or((DEFAULT_PORTRAIT, DEFAULT_PORTRAIT));
        replays.push(ReplayListing {
            match_id: replay.row.id.clone(),
            mode,
            ended_at: replay.result.ended_at,
            seat: home_seat(replay, &profile.id),
            result: result_for(&replay.result, &profile.id),
            turns: replay.result.turns,
            steps: replay.actions + 1,
            portraits: PerPlayer::new(portraits.0, portraits.1),
            opponent: ReplayOpponent {
                profile_id: opponent.to_string(),
                username: profiles
                    .iter()
                    .find(|found| found.id == opponent)
                    .map(Profile::username)
                    .unwrap_or_default(),
            },
            unavailable: known_refusal(app, replay, now),
        });
    }
    ok_of(&ReplayList {
        replays,
        // Saturating: an offset past every row lists nothing and must not overflow on the way.
        next_offset: more.then_some(offset.saturating_add(page)),
    })
}

/// `GET /api/replays/:matchId?from=&count=`.
pub async fn get_replay(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    check_rate(app, &profile.id)?;
    let match_id = req.params.get("matchId").map(String::as_str).unwrap_or("");
    let from = query_number(&req, "from", 0)?;
    let count = query_number(&req, "count", REPLAY_PAGE_STEPS)?;
    if count == 0 {
        return Err(bad_request("count must be at least 1"));
    }

    let now = now_ms();
    let mut tx = app.db.begin(None).await?;
    // The seat check comes first and answers what an unknown id answers, so nothing past it tells
    // an account that held no seat anything about the match. Social (#940): B52's friend holding a
    // share reads it from the sharer's seat (`replay_reader`).
    let Some((seats, reader)) = replay_reader(&mut tx, match_id, &profile.id).await? else {
        return Err(no_such_replay());
    };
    if seats.phase != MatchPhase::Over {
        return Err(ApiError::new(
            ApiErrorCode::Conflict,
            "this match has not finished",
        ));
    }
    let Some(replay) = tx.replays_get(match_id).await? else {
        return Err(no_such_replay());
    };
    // The retention purge takes a finished match's whole log at once.
    if replay.actions == 0 {
        return Err(ApiError::new(ApiErrorCode::Gone, "expired"));
    }
    let kept = app.replays.cache.get(match_id, now);
    let (log, mode) = match kept {
        Some(_) => (Vec::new(), None),
        None => (
            tx.matches_actions(match_id)
                .await?
                .into_iter()
                .map(|row| row.action)
                .collect(),
            tx.matches_mode_of(match_id).await?,
        ),
    };
    tx.commit().await?;

    let open = open_replay(app, &replay, kept, log, mode).await?;
    let (total, checkpoints) = match open.as_ref() {
        ReplayOpen::Refused { reason } => {
            return Err(ApiError::with_details(
                ApiErrorCode::ReplayUnavailable,
                format!("this replay cannot be shown: {reason}"),
                json!({ "reason": reason }),
            ));
        }
        ReplayOpen::Ready { steps, checkpoints } => (*steps, checkpoints),
    };
    if from >= total {
        return Err(bad_request(format!("no step {from}: this replay has {total}")));
    }
    // R767000: the walk to `from` reduces past a kept state, so under the catalog the fold read.
    let (seat, run) = seat_run(
        Pinned::of_checkpoints(checkpoints),
        checkpoints,
        home_seat(&replay, &reader),
        from,
        count.min(REPLAY_PAGE_STEPS),
    );
    let page = engine::replay_page(checkpoints, seat, from, run);
    // R1834003: what the page's views show, read against the cards the opening dealt (step 0's state).
    let loadouts = replay
        .row
        .loadouts
        .clone()
        .unwrap_or_else(|| (Loadout::defaults(), Loadout::defaults()));
    let dealt = checkpoints
        .states
        .first()
        .map(engine::dealt_cards)
        .unwrap_or_default();
    let shown: BTreeSet<(PlayerId, String)> = page
        .steps
        .iter()
        .flat_map(|step| shown_defs(&step.view, &dealt))
        .collect();
    let card_loadouts = cards_due(&shown, &loadouts, &mut BTreeSet::new());
    ok_of(&ReplaySteps {
        match_id: replay.row.id.clone(),
        from: page.from,
        steps: page.steps,
        total,
        loadouts: PerPlayer::new(loadouts.0.seat, loadouts.1.seat),
        card_loadouts,
    })
}

// Social: watching and shared replays (#939, #940)

use crate::db::store::{MatchSeats, StoreError, Tx};

/// B52: whose seat `caller` reads the replay of `match_id` from, with the match's seats: their own
/// when they held one; else, for a finished match, the seat of the first seat holder who shared it
/// with them and is still their friend, neither blocking the other (`shares_sharers`). `None` for
/// anyone else, who gets what an unknown id gets, so an unshare, an unfriend or a block ends the
/// read at once.
async fn replay_reader(
    tx: &mut Tx<'_>,
    match_id: &str,
    caller: &str,
) -> Result<Option<(MatchSeats, String)>, StoreError> {
    let Some(seats) = tx.matches_seats(match_id).await? else {
        return Ok(None);
    };
    if seats.players.iter().any(|player| player == caller) {
        return Ok(Some((seats, caller.to_string())));
    }
    if seats.phase != MatchPhase::Over {
        return Ok(None);
    }
    let sharer = tx
        .shares_sharers(match_id, caller)
        .await?
        .into_iter()
        .find(|sharer| seats.players.contains(sharer));
    Ok(sharer.map(|sharer| (seats, sharer)))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    /// Sleeps on the paused clock, as a fold takes time without taking a thread.
    const FOLD_MS: u64 = 10;

    fn folds() -> Arc<Semaphore> {
        Arc::new(Semaphore::new(REPLAY_FOLDS_RUNNING_MAX))
    }

    fn flights_held(cache: &ReplayCache<usize>) -> usize {
        lock(&cache.flights).len()
    }

    #[tokio::test(start_paused = true)]
    async fn r3500000_concurrent_opens_of_one_match_fold_it_once() {
        let (cache, folds) = (ReplayCache::<usize>::default(), folds());
        let folded = AtomicUsize::new(0);
        let open = || {
            cache.open(&folds, "m-1", |_permit| async {
                folded.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(FOLD_MS)).await;
                Ok(7)
            })
        };
        let (a, b, c) = tokio::join!(open(), open(), open());
        assert_eq!(folded.load(Ordering::SeqCst), 1);
        for answer in [a, b, c] {
            assert_eq!(*answer.expect("every open answers"), 7);
        }
        assert_eq!(cache.held(now_ms()), 1);
        assert_eq!(flights_held(&cache), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn r3500000_opens_of_different_matches_never_exceed_the_fold_cap() {
        let (cache, folds) = (Arc::new(ReplayCache::<usize>::default()), folds());
        let (running, peak) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let opens: Vec<_> = (0..6)
            .map(|n| {
                let (cache, folds) = (Arc::clone(&cache), Arc::clone(&folds));
                let (running, peak) = (Arc::clone(&running), Arc::clone(&peak));
                tokio::spawn(async move {
                    cache
                        .open(&folds, &format!("m-{n}"), |permit| async move {
                            let _permit = permit;
                            let now = running.fetch_add(1, Ordering::SeqCst) + 1;
                            peak.fetch_max(now, Ordering::SeqCst);
                            tokio::time::sleep(Duration::from_millis(FOLD_MS)).await;
                            running.fetch_sub(1, Ordering::SeqCst);
                            Ok(n)
                        })
                        .await
                })
            })
            .collect();
        for open in opens {
            open.await.expect("the task ran").expect("every open answers");
        }
        let peak = peak.load(Ordering::SeqCst);
        assert!((1..=REPLAY_FOLDS_RUNNING_MAX).contains(&peak), "peak {peak}");
        assert_eq!(cache.held(now_ms()), 6);
        assert_eq!(flights_held(&cache), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn r3500000_an_open_waiting_for_a_long_fold_of_its_match_is_a_429_and_leaves_the_fold_alone() {
        let (cache, folds) = (ReplayCache::<usize>::default(), folds());
        let folded = AtomicUsize::new(0);
        let long_fold = Duration::from_millis(REPLAY_FOLD_WAIT_MS * 2);
        let first = cache.open(&folds, "m-1", |_permit| async {
            folded.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(long_fold).await;
            Ok(7)
        });
        let second = async {
            // The first has taken the turn and the permit by the time this one asks.
            tokio::time::sleep(Duration::from_millis(1)).await;
            cache
                .open(&folds, "m-1", |_permit| async {
                    folded.fetch_add(1, Ordering::SeqCst);
                    Ok(8)
                })
                .await
        };
        let (first, second) = tokio::join!(first, second);
        assert_eq!(*first.expect("the fold that holds the turn answers"), 7);
        let error = second.expect_err("the turn is not free for the whole wait");
        assert_eq!(error.code, ApiErrorCode::RateLimited);
        assert_eq!(error.retry_after_ms, Some(REPLAY_FOLD_RETRY_AFTER_MS));
        assert_eq!(folded.load(Ordering::SeqCst), 1);
        assert_eq!(cache.held(now_ms()), 1);
        assert_eq!(flights_held(&cache), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn r3500000_an_open_past_the_wait_is_a_429_with_retry_after_ms() {
        let (cache, folds) = (ReplayCache::<usize>::default(), folds());
        let taken = Arc::clone(&folds)
            .acquire_many_owned(u32::try_from(REPLAY_FOLDS_RUNNING_MAX).expect("a small cap"))
            .await
            .expect("the permits are free");
        let folded = AtomicUsize::new(0);
        let error = cache
            .open(&folds, "m-1", |_permit| async {
                folded.fetch_add(1, Ordering::SeqCst);
                Ok(1)
            })
            .await
            .expect_err("no permit is free for the whole wait");
        assert_eq!(error.code, ApiErrorCode::RateLimited);
        assert_eq!(error.message, REPLAY_BUSY_MESSAGE);
        assert_eq!(error.retry_after_ms, Some(REPLAY_FOLD_RETRY_AFTER_MS));
        assert_eq!(folded.load(Ordering::SeqCst), 0);
        assert_eq!(cache.held(now_ms()), 0);
        assert_eq!(flights_held(&cache), 0);

        drop(taken);
        let opened = cache
            .open(&folds, "m-1", |_permit| async { Ok(2) })
            .await
            .expect("a permit is free again");
        assert_eq!(*opened, 2);
    }
}
