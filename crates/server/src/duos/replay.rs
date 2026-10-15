//! `GET /api/duos/matches/:matchId/replay` (fullsend SPEC S6, B41): a finished co-op match, step by
//! step, as the caller's own seat saw it: `200 { seat, steps }`, `steps` being
//! `duos_view_for(state, seat)` as the game began and after each logged action, which is exactly what
//! that seat's live view showed (B23, CLAUDE.md rule 7). Nothing else leaves: no seed, no log, no
//! state, no deck.
//!
//! Only the two people who played it may read it; anyone else, and an id no match has, get the same
//! 404, so a match id's existence never leaks (R768's rule). As the 1v1 route answers: a match still
//! live is 409 (§12.9: a finished co-op game); one whose log the retention purge has taken
//! (`MATCH_ACTION_RETENTION_DAYS` after it ended) is 410 `gone`; one this build cannot show is 422
//! `replay_unavailable` with the reason in `details.reason`: `earlier_patch` for a catalog version
//! this build neither is nor carries, `rules_changed` for a log that no longer folds to its end or
//! to the result the match recorded. The fold runs under the catalog the match was played on
//! (R767000) and is never cut short: an action it refuses refuses the whole replay.
//!
//! Bounded cost: the fold runs on the blocking pool, once per match: both people's steps are kept
//! (`DuosReplayCache`, in the co-op registry) for `REPLAY_CACHE_TTL_SECONDS`, at most
//! `REPLAY_CACHE_MATCHES` of them, the one used longest ago going first, as the 1v1 replays are; and
//! the route shares the 1v1 replays' allowance (`REPLAY_REQUESTS_PER_MINUTE` an account, R768).

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex};

use indexmap::IndexMap;
use serde::Serialize;
use serde_json::json;

use jackioh_engine::ReplayRefusal;
use jackioh_engine::wire::{DuosAction, DuosSeat, DuosView};

use crate::actor::contracts::one_tx;
use crate::actor::registry::panic_text;
use crate::api::collection::caller_profile;
use crate::api::http::{
    ApiError, ApiErrorCode, ApiResult, Req, account_key, lock, log_warn, now_ms, ok_of, rate_limited,
};
use crate::app::App;
use crate::config::{REPLAY_CACHE_MATCHES, REPLAY_CACHE_TTL_SECONDS};
use crate::duos::engine::{self, DuosSteps, Pinned};
use crate::duos::registry::registry;
use crate::duos::store::{DuosMatch, action_of};

/// Unit conversion, not configuration.
const MS_PER_SECOND: i64 = 1000;

/// B41's answer.
#[derive(Serialize)]
struct DuosReplay<'a> {
    seat: DuosSeat,
    steps: &'a [DuosView],
}

/// A finished match folded once: both people's steps, or why this build cannot show it.
pub enum DuosOpened {
    Ready(DuosSteps),
    Refused(ReplayRefusal),
}

type Entries = IndexMap<String, (i64, Arc<DuosOpened>)>;

/// The opened co-op replays by match id, the one used longest ago first, each with when it was
/// opened. They hold every step of both people's views, so nothing but this route reads them. The
/// lock is never held across a fold.
#[derive(Default)]
pub struct DuosReplayCache {
    entries: Mutex<Entries>,
}

impl DuosReplayCache {
    fn expire(entries: &mut Entries, now: i64) {
        entries.retain(|_, (opened_at, _)| now - *opened_at < REPLAY_CACHE_TTL_SECONDS * MS_PER_SECOND);
    }

    /// The replay of `match_id` opened less than `REPLAY_CACHE_TTL_SECONDS` ago, now the one used
    /// last.
    pub fn get(&self, match_id: &str, now: i64) -> Option<Arc<DuosOpened>> {
        let mut entries = lock(&self.entries);
        Self::expire(&mut entries, now);
        let entry = entries.shift_remove(match_id)?;
        let open = Arc::clone(&entry.1);
        entries.insert(match_id.to_string(), entry);
        Some(open)
    }

    /// Keeps the replay of `match_id`, opened at `now`, within `REPLAY_CACHE_MATCHES`.
    pub fn put(&self, match_id: &str, open: Arc<DuosOpened>, now: i64) {
        let mut entries = lock(&self.entries);
        Self::expire(&mut entries, now);
        entries.shift_remove(match_id);
        while entries.len() >= REPLAY_CACHE_MATCHES {
            entries.shift_remove_index(0);
        }
        entries.insert(match_id.to_string(), (now, open));
    }

    /// Forgets every opened replay (the retention purge took logs they were folded from).
    pub fn clear(&self) {
        lock(&self.entries).clear();
    }

    /// How many replays are kept at `now`.
    pub fn held(&self, now: i64) -> usize {
        let mut entries = lock(&self.entries);
        Self::expire(&mut entries, now);
        entries.len()
    }
}

fn no_such_match() -> ApiError {
    ApiError::new(ApiErrorCode::NotFound, "no such co-op match")
}

/// R768's per-account allowance, shared with the 1v1 replay routes.
fn check_rate(app: &App, profile_id: &str) -> Result<(), ApiError> {
    let key = account_key(profile_id);
    let now = now_ms();
    if app.replays.limiter.allow(&key, now) {
        return Ok(());
    }
    log_warn("duos.replays.rate_limited", json!({ "key": key }));
    Err(rate_limited(
        "too many replay requests; slow down",
        app.replays.limiter.retry_after_ms(&key, now),
    ))
}

/// The match folded under the catalog it was played on (R767000) and held to the result it
/// recorded. A setup the catalog refuses (a panic in `duos_create`) is `rules_changed`, as the 1v1
/// replay's is.
fn open(row: &DuosMatch, log: Vec<DuosAction>) -> DuosOpened {
    let pinned = match Pinned::of(&row.options.catalog_version) {
        Ok(pinned) => pinned,
        Err(reason) => return DuosOpened::Refused(reason),
    };
    let options = engine::create_options(row);
    let folded = catch_unwind(AssertUnwindSafe(|| engine::steps_for(pinned, &options, &log)));
    let steps = match folded {
        Ok(Ok(steps)) => steps,
        Ok(Err(reason)) => return DuosOpened::Refused(reason),
        Err(payload) => {
            tracing::error!(event = "duos.replay.panicked", matchId = %row.id, message = %panic_text(payload));
            return DuosOpened::Refused(ReplayRefusal::RulesChanged);
        }
    };
    let ended_as_recorded = match (&steps.result, &row.result) {
        (Some(result), Some(recorded)) => serde_json::to_value(result).ok().as_ref() == Some(recorded),
        _ => false,
    };
    if !ended_as_recorded {
        return DuosOpened::Refused(ReplayRefusal::RulesChanged);
    }
    DuosOpened::Ready(steps)
}

/// `GET /api/duos/matches/:matchId/replay` (B41).
pub async fn get_replay(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    check_rate(app, &profile.id)?;
    let match_id = req.params.get("matchId").cloned().unwrap_or_default();
    let Some(row) = one_tx!(app.db, |t| t.duos_matches_get(&match_id).await?)? else {
        return Err(no_such_match());
    };
    // The seat check comes first and answers what an unknown id answers.
    let seat = if row.a1 == profile.id {
        DuosSeat::A1
    } else if row.a2 == profile.id {
        DuosSeat::A2
    } else {
        return Err(no_such_match());
    };
    if row.ended_at.is_none() {
        return Err(ApiError::new(
            ApiErrorCode::Conflict,
            "this co-op match has not finished",
        ));
    }

    let replays = &registry(app).replays;
    let kept = replays.get(&row.id, now_ms());
    let opened = match kept {
        Some(opened) => opened,
        None => {
            let log = one_tx!(app.db, |t| t.duos_actions(&row.id).await?)?;
            // A game ends on an action, so a finished match with no log is one the purge has taken.
            if log.is_empty() {
                return Err(ApiError::new(ApiErrorCode::Gone, "expired"));
            }
            let actions: Vec<DuosAction> = log
                .iter()
                .map(|entry| action_of(&entry.action))
                .collect::<Result<_, _>>()?;
            let folding = row.clone();
            let opened = tokio::task::spawn_blocking(move || open(&folding, actions))
                .await
                .map_err(|error| ApiError::internal(error.to_string()))?;
            let opened = Arc::new(opened);
            replays.put(&row.id, Arc::clone(&opened), now_ms());
            opened
        }
    };
    match opened.as_ref() {
        DuosOpened::Ready(steps) => ok_of(&DuosReplay {
            seat,
            steps: steps.of(seat),
        }),
        DuosOpened::Refused(reason) => Err(ApiError::with_details(
            ApiErrorCode::ReplayUnavailable,
            format!("this replay cannot be shown: {reason}"),
            json!({ "reason": reason }),
        )),
    }
}
