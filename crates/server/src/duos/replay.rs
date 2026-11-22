//! `GET /api/duos/matches/:matchId/replay` (fullsend SPEC S6, B41): a finished co-op match, step by
//! step, as the caller's seat saw it (B23, CLAUDE.md rule 7): `200 { seat, steps }`.
//!
//! Only the people who played it (two, or four in a 2v2, R3731003) may read it, each their own seat's;
//! anyone else gets 404 (R768). A live match is 409 (§13.9); a purged log is 410 `gone`; an
//! unsupported match is 422 `replay_unavailable`. The fold
//! runs under the catalog the match was played on (R767000) on the blocking pool, cached in
//! `DuosReplayCache` (`REPLAY_CACHE_TTL_SECONDS`, `REPLAY_CACHE_MATCHES`). R3500000: an opening not
//! kept waits for the permit every replay fold takes (`REPLAY_FOLDS_RUNNING_MAX`, the 1v1 replays'
//! too), shares one fold with a concurrent opening of its match, and is 429 with `retryAfterMs`
//! past `REPLAY_FOLD_WAIT_MS`.

use std::sync::Arc;

use serde::Serialize;
use serde_json::json;

use jackioh_engine::ReplayRefusal;
use jackioh_engine::wire::{DuosAction, DuosSeat, DuosView};

use crate::actor::contracts::one_tx;
use crate::actor::engine as actor_engine;
use crate::api::collection::caller_profile;
use crate::api::http::{
    ApiError, ApiErrorCode, ApiResult, Req, account_key, log_warn, now_ms, ok_of, rate_limited,
};
use crate::api::replays::ReplayCache;
use crate::app::App;
use crate::duos::engine::{self, DuosSteps, Pinned};
use crate::duos::registry::registry;
use crate::duos::store::{DuosMatch, action_of};

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

/// The opened co-op replays: `ReplayCache` over every step of both people's views.
pub type DuosReplayCache = ReplayCache<DuosOpened>;

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
/// recorded. A setup or log the engine refuses is `rules_changed`, as the 1v1 replay's is; a
/// panicking setup or fold propagates to the caller, which answers `internal` (`get_replay`).
fn open(row: &DuosMatch, log: Vec<DuosAction>) -> DuosOpened {
    let pinned = match Pinned::of(&row.options.catalog_version) {
        Ok(pinned) => pinned,
        Err(reason) => return DuosOpened::Refused(reason),
    };
    let options = engine::create_options(row);
    let steps = match engine::steps_for(pinned, &options, &log, row.team_b()) {
        Ok(steps) => steps,
        Err(reason) => return DuosOpened::Refused(reason),
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
    // R3731003: each of a 2v2's four people opens their own seat's.
    let Some(seat) = row.seat_of(&profile.id) else {
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
            let engine_thread = app.engine_thread;
            let match_id = row.id.clone();
            let panicked_id = match_id.clone();
            // #1035: the open in one job off the runtime, as a 1v1 replay opens: a long log is a
            // long fold. R3500000: the job holds `permit` until it ends.
            replays
                .open(&app.replays.folds, &match_id, |permit| async move {
                    actor_engine::off_runtime(engine_thread, move || {
                        let _permit = permit;
                        open(&folding, actions)
                    })
                    .await
                    .map_err(|message| {
                        tracing::error!(event = "duos.replay.panicked", matchId = %panicked_id, message = %message);
                        ApiError::internal(message)
                    })
                })
                .await?
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
