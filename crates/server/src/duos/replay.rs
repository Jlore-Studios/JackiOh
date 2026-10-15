//! `GET /api/duos/matches/:matchId/replay` (fullsend SPEC S6, B41): a co-op match, step by step, as
//! the caller's own seat saw it: `200 { seat, steps }`, `steps` being `duos_view_for(state, seat)` as
//! the game began and after each logged action, which is exactly what that seat's live view showed
//! (B23, CLAUDE.md rule 7). Nothing else leaves: no seed, no log, no state, no deck.
//!
//! Only the two people who played it may read it; anyone else, and an id no match has, get the same
//! 404, so a match id's existence never leaks (R768's rule). The fold runs on the blocking pool, and
//! the route shares the 1v1 replays' allowance (`REPLAY_REQUESTS_PER_MINUTE` an account, R768).

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use serde::Serialize;
use serde_json::json;

use jackioh_engine::wire::{DuosAction, DuosSeat, DuosView};

use crate::actor::contracts::one_tx;
use crate::actor::registry::panic_text;
use crate::api::collection::caller_profile;
use crate::api::http::{
    ApiError, ApiErrorCode, ApiResult, Req, account_key, log_warn, now_ms, ok_of, rate_limited,
};
use crate::app::App;
use crate::duos::engine;
use crate::duos::store::action_of;

/// B41's answer.
#[derive(Serialize)]
struct DuosReplay {
    seat: DuosSeat,
    steps: Vec<DuosView>,
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

/// `GET /api/duos/matches/:matchId/replay` (B41).
pub async fn get_replay(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    check_rate(app, &profile.id)?;
    let match_id = req.params.get("matchId").cloned().unwrap_or_default();
    let (row, log) = one_tx!(app.db, |t| {
        let row = t.duos_matches_get(&match_id).await?;
        let log = match &row {
            Some(_) => t.duos_actions(&match_id).await?,
            None => Vec::new(),
        };
        (row, log)
    })?;
    let Some(row) = row else {
        return Err(no_such_match());
    };
    let seat = if row.a1 == profile.id {
        DuosSeat::A1
    } else if row.a2 == profile.id {
        DuosSeat::A2
    } else {
        return Err(no_such_match());
    };
    let actions: Vec<DuosAction> = log
        .iter()
        .map(|entry| action_of(&entry.action))
        .collect::<Result<_, _>>()?;
    let options = engine::create_options(&row);
    let folded = tokio::task::spawn_blocking(move || {
        catch_unwind(AssertUnwindSafe(|| engine::steps_for(&options, &actions, seat)))
    })
    .await
    .map_err(|error| ApiError::internal(error.to_string()))?;
    let steps = folded
        .map_err(|payload| ApiError::internal(panic_text(payload)))?
        .map_err(ApiError::internal)?;
    ok_of(&DuosReplay { seat, steps })
}
