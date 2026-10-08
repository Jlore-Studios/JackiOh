//! Online replays (SPEC §9.3, R768, issue #510): a finished match, a page of steps at a time, for
//! the two accounts that played it.
//!
//! `GET /api/replays?offset=` lists the caller's finished matches whose action log the retention
//! purge has not taken yet (`MATCH_ACTION_RETENTION_DAYS`), newest first, `REPLAY_LIST_PAGE` to a
//! page: each one's id, mode, end, the seat the caller began in, the result for the caller, its turns
//! and steps, both seats' portraits (R641, R642), the opponent's tag as the match screen showed it
//! (R612), and, when this build cannot show it, part 1's reason (`unavailable`).
//!
//! `GET /api/replays/:matchId?from=&count=` answers steps `[from, from + n)` of the caller's replay,
//! `n` at most `count` and `REPLAY_PAGE_STEPS`, and fewer where a Glitch's swap moves the caller to
//! the other seat or where the engine's page ends early (R768); the next page starts at
//! `from + steps.len()`. A step is `view_for` of the seat the caller played at that step (R677) and
//! its turn, which is exactly what that seat's live view showed (R97, CLAUDE.md rule 7). Nothing else
//! leaves: no seed, no log, no state, no deck. An account that held no seat gets the same 404 as a
//! match that does not exist, so a match id's existence never leaks; a match still open or live is
//! 409; one whose log is purged 410 `gone`; one this build cannot show 422 `replay_unavailable`,
//! with part 1's reason in `details.reason`.
//!
//! The fold is the actor's rebuild's (`registry.rs`): the row's seed, frozen decks, last boards and
//! Glitch boards, its log, and the dealt seats of an All Random match (R433). It is held to the final
//! hash the result recorded (migration 0028). A match that ended before 0028, or that the reaper
//! resolved (R112), has none, so its fold is held to its results row instead: the same winner, reason
//! and turn count, and then to the hash of the state it ends on.
//!
//! Bounded cost: an opened replay (its checkpoints, R768) is kept in memory for at most
//! `REPLAY_CACHE_TTL_SECONDS`, at most `REPLAY_CACHE_MATCHES` of them, the one used longest ago going
//! first; a list opens at most `REPLAY_LIST_PAGE` of them; and both routes together are held to
//! `REPLAY_REQUESTS_PER_MINUTE` an account, a second limiter of the kind R109's is.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex};

use indexmap::IndexMap;
use jackioh_engine::config::{REPLAY_CHECKPOINT_EVERY, REPLAY_PAGE_STEPS};
use jackioh_engine::{
    Action, DEFAULT_PORTRAIT, GameOverReason, PerPlayer, PlayerId, PortraitId, ReplayCheckpoints, ReplayOpen,
    ReplayRecord, ReplayRefusal, ReplayStep, Winner,
};
use serde::Serialize;
use serde_json::json;

use crate::actor::engine::{self, EngineState};
use crate::actor::match_actor::last_boards_of;
use crate::actor::registry::{dealt_for, panic_text};
use crate::api::collection::caller_profile;
use crate::api::crypto::player_tag;
use crate::api::http::{
    ApiError, ApiErrorCode, ApiResult, RateLimiter, Req, account_key, bad_request, create_rate_limiter, lock,
    log_warn, now_ms, ok_of, rate_limited,
};
use crate::app::App;
use crate::config::{
    REPLAY_CACHE_MATCHES, REPLAY_CACHE_TTL_SECONDS, REPLAY_LIST_PAGE, REPLAY_REQUESTS_PER_MINUTE,
};
use crate::db::store::{MatchPhase, QueueMode, ReplayRow, ResultRow};

/// Unit conversions, not configuration.
const MS_PER_SECOND: i64 = 1000;
const MS_PER_MINUTE: i64 = 60 * MS_PER_SECOND;

/// What the replay routes keep between requests, one per `App`: the opened replays and their own
/// allowance.
pub struct Replays {
    pub cache: ReplayCache,
    pub limiter: RateLimiter,
}

impl Default for Replays {
    fn default() -> Replays {
        Replays {
            cache: ReplayCache::default(),
            limiter: create_rate_limiter(REPLAY_REQUESTS_PER_MINUTE, MS_PER_MINUTE),
        }
    }
}

/// The opened replays by match id, the one used longest ago first, each with when it was opened.
/// They hold whole states, so nothing but this module reads them (CLAUDE.md rule 7). The lock is
/// never held across a fold.
#[derive(Default)]
pub struct ReplayCache {
    entries: Mutex<IndexMap<String, (i64, Arc<ReplayOpen>)>>,
}

impl ReplayCache {
    /// Drops every replay opened `REPLAY_CACHE_TTL_SECONDS` or more before `now`.
    fn expire(entries: &mut IndexMap<String, (i64, Arc<ReplayOpen>)>, now: i64) {
        entries.retain(|_, (opened_at, _)| now - *opened_at < REPLAY_CACHE_TTL_SECONDS * MS_PER_SECOND);
    }

    /// The replay of `match_id` opened less than `REPLAY_CACHE_TTL_SECONDS` ago, now the one used
    /// last.
    pub fn get(&self, match_id: &str, now: i64) -> Option<Arc<ReplayOpen>> {
        let mut entries = lock(&self.entries);
        Self::expire(&mut entries, now);
        let entry = entries.shift_remove(match_id)?;
        let open = Arc::clone(&entry.1);
        entries.insert(match_id.to_string(), entry);
        Some(open)
    }

    /// Keeps the replay of `match_id`, opened at `now`, dropping the ones used longest ago to stay
    /// within `REPLAY_CACHE_MATCHES`.
    pub fn put(&self, match_id: &str, open: Arc<ReplayOpen>, now: i64) {
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
    /// R612: the opponent's tag, the only name the match screen showed; null once the opponent's
    /// account is deleted.
    pub opponent_tag: Option<String>,
    /// R768: why this build cannot show it, when it cannot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unavailable: Option<ReplayRefusal>,
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

/// §2.5: a hero's death, a concede and a disconnect always name a winner; every other ending is a
/// draw.
fn names_a_winner(reason: GameOverReason) -> bool {
    matches!(
        reason,
        GameOverReason::HeroDeath | GameOverReason::Concede | GameOverReason::Disconnect
    )
}

/// The seat the results row credits with the win (R677: its players are the seats as the game
/// ended), `Draw` for a draw, `None` when it names a winner no seat holds. A winnerless row whose
/// reason names a winner is one whose winner has deleted their account since, which empties both
/// the winner and that seat (migration 0012).
fn recorded_winner(result: &ResultRow) -> Option<Winner> {
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
        Some(winner) => seat_of(winner),
        None if names_a_winner(result.reason) => seat_of(""),
        None => Some(Winner::Draw),
    }
}

fn result_for(result: &ResultRow, profile_id: &str) -> ReplayResult {
    if result.winner_profile_id.as_deref() == Some(profile_id) {
        ReplayResult::Win
    } else if recorded_winner(result) == Some(Winner::Draw) {
        ReplayResult::Draw
    } else {
        ReplayResult::Loss
    }
}

/// R768: a match with no recorded hash is held to its results row: the fold ends with the same
/// winner, reason and turn count.
fn ends_as_recorded(state: &EngineState, result: &ResultRow) -> bool {
    let snapshot = engine::snapshot(state);
    let Some(ended) = snapshot.result else {
        return false;
    };
    recorded_winner(result) == Some(ended.winner)
        && ended.reason == result.reason
        && i64::from(snapshot.turn) == result.turns
}

fn rules_changed() -> ReplayOpen {
    ReplayOpen::Refused {
        reason: ReplayRefusal::RulesChanged,
    }
}

/// R768: the match's log folded once and checked, as the actor's rebuild folds it.
fn open_uncached(replay: &ReplayRow, log: Vec<Action>, mode: Option<QueueMode>) -> ReplayOpen {
    let row = &replay.row;
    // Refused before anything is folded, the legacy check's fold included.
    if row.catalog_version != jackioh_cards::catalog_version() {
        return ReplayOpen::Refused {
            reason: ReplayRefusal::EarlierPatch,
        };
    }
    let (last_boards, glitch_boards) = last_boards_of(row);
    let args = engine::fold_args(
        &row.seed,
        &row.decks,
        log,
        last_boards,
        glitch_boards,
        dealt_for(mode),
    );
    let opened = catch_unwind(AssertUnwindSafe(|| {
        let final_hash = match &row.final_hash {
            Some(final_hash) => final_hash.clone(),
            None => {
                let folded = engine::fold(&args);
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
    }));
    opened.unwrap_or_else(|payload| {
        // A setup `create_game` refuses: the frozen decks no longer pass this build's checks.
        tracing::error!(event = "replay.open.panicked", matchId = %row.id, message = %panic_text(payload));
        rules_changed()
    })
}

/// The replay of a match the caller may read: the one kept in memory, else `log` folded and kept.
fn open_replay(
    app: &App,
    replay: &ReplayRow,
    kept: Option<Arc<ReplayOpen>>,
    log: Vec<Action>,
    mode: Option<QueueMode>,
) -> Arc<ReplayOpen> {
    if let Some(open) = kept {
        return open;
    }
    let open = Arc::new(open_uncached(replay, log, mode));
    app.replays.cache.put(&replay.row.id, Arc::clone(&open), now_ms());
    open
}

/// R677: the seat the account that began in `home` plays at step `from`, and for how many steps
/// from there, up to `count`, it goes on playing it, so a page never spans a Glitch's swap and every
/// step of it is one seat's view. Read off the states as `replay_page` reads them, from the kept state
/// at or before `from`. `from` is a step of the replay and `count` at least 1.
fn seat_run(checkpoints: &ReplayCheckpoints, home: PlayerId, from: usize, count: usize) -> (PlayerId, usize) {
    let end = (from + count).min(checkpoints.accepted.len() + 1);
    let mut at = from - from % REPLAY_CHECKPOINT_EVERY;
    let Some(mut state) = checkpoints.states.get(at / REPLAY_CHECKPOINT_EVERY).cloned() else {
        return (home, 1);
    };
    while at < from {
        state = engine::reduce(&state, &checkpoints.accepted[at]).state;
        at += 1;
    }
    let seat = engine::seat_played_by(&state, home);
    let mut run = 1;
    while at + 1 < end {
        state = engine::reduce(&state, &checkpoints.accepted[at]).state;
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

/// `GET /api/replays?offset=`.
pub async fn list_replays(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    check_rate(app, &profile.id)?;
    let offset = query_number(&req, "offset", 0)?;
    let offset = i64::try_from(offset).map_err(|_| bad_request("offset is too large"))?;
    let page = REPLAY_LIST_PAGE as i64;

    // One past the page, to know whether another follows. A replay already open needs no log.
    let now = now_ms();
    let mut tx = app.db.begin(None).await?;
    let mut rows = tx.replays_list(&profile.id, page + 1, offset).await?;
    let more = rows.len() > REPLAY_LIST_PAGE;
    rows.truncate(REPLAY_LIST_PAGE);
    let mut reads = Vec::with_capacity(rows.len());
    for replay in &rows {
        let mode = tx.matches_mode_of(&replay.row.id).await?;
        let kept = app.replays.cache.get(&replay.row.id, now);
        let log = match kept {
            Some(_) => Vec::new(),
            None => tx
                .matches_actions(&replay.row.id)
                .await?
                .into_iter()
                .map(|row| row.action)
                .collect(),
        };
        reads.push((mode, kept, log));
    }
    tx.commit().await?;

    let mut replays = Vec::with_capacity(rows.len());
    for (replay, (mode, kept, log)) in rows.iter().zip(reads) {
        let open = open_replay(app, replay, kept, log, mode);
        let seat = home_seat(replay, &profile.id);
        let opponent = match seat {
            PlayerId::P1 => &replay.row.players.1,
            PlayerId::P2 => &replay.row.players.0,
        };
        let portraits = replay
            .row
            .portraits
            .unwrap_or((DEFAULT_PORTRAIT, DEFAULT_PORTRAIT));
        replays.push(ReplayListing {
            match_id: replay.row.id.clone(),
            mode,
            ended_at: replay.result.ended_at,
            seat,
            result: result_for(&replay.result, &profile.id),
            turns: replay.result.turns,
            steps: replay.actions + 1,
            portraits: PerPlayer::new(portraits.0, portraits.1),
            // A deleted account's seat reads as the empty id (KNOWN DIVERGENCES in `db/pg.rs`).
            opponent_tag: (!opponent.is_empty()).then(|| player_tag(opponent)),
            unavailable: match open.as_ref() {
                ReplayOpen::Refused { reason } => Some(*reason),
                ReplayOpen::Ready { .. } => None,
            },
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
    // an account that held no seat anything about the match.
    let Some(seats) = tx
        .matches_seats(match_id)
        .await?
        .filter(|seats| seats.players.contains(&profile.id))
    else {
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

    let open = open_replay(app, &replay, kept, log, mode);
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
    let (seat, run) = seat_run(
        checkpoints,
        home_seat(&replay, &profile.id),
        from,
        count.min(REPLAY_PAGE_STEPS),
    );
    let page = engine::replay_page(checkpoints, seat, from, run);
    ok_of(&ReplaySteps {
        match_id: replay.row.id.clone(),
        from: page.from,
        steps: page.steps,
        total,
    })
}
