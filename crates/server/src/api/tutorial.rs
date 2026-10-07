//! Tutorial progress on the account (SPEC §9.10, R320; ← `apps/server/src/api/tutorial.ts`).
//!
//! The device keeps which lessons a player has won, and whether they hid the lesson path, in
//! `localStorage` (R294). An active account keeps the same on the server too, so a player who signs
//! in on another device finds them there, and the client merges the two copies (R321). This file is
//! the account's half: two routes, both `active` (a pending account has the code screen and nothing
//! else, §9.4), both keyed on the profile the verified token names and never on anything in the body.
//!
//! ```text
//!   GET /api/tutorial  -> { progress: { completed, hiddenChoice } }
//!   PUT /api/tutorial  <- { completed: string[], hiddenChoice?: { hidden, at } | null }
//!                      -> { progress } as it stands after the merge
//! ```
//!
//! A PUT merges and never replaces (R320): the account's lessons become the union of the stored and
//! the sent, so a stale device can never remove a completed lesson, and its Hide/Show choice is
//! replaced only by a strictly newer one. The same body sent twice changes nothing, so a client may
//! retry it freely. The merge is the store's (`app.merge_tutorial_progress`, migration 0011), under a
//! lock on the profile, so two devices writing at once cannot lose each other's lesson.
//!
//! The server does not know the lessons: they are the client's (`apps/web/src/tutorial/lessons.ts`),
//! and a lesson added there needs no change here. So an id is checked for its shape only — a
//! lower-case slug of at most `TUTORIAL_LESSON_ID_MAX_LENGTH` characters — and an account holds at
//! most `TUTORIAL_LESSONS_MAX`. None of this is a rule: a lesson is a practice game and records no
//! result (R187), and nothing the server does reads this row.
//!
//! TS's `createTutorialRoutes()` declared the two routes; in Rust the route table is `app.rs`'s
//! `ROUTES` (SURFACE §11.2), which lists them, both `AuthLevel::Active`, as
//! `("GET", "/api/tutorial", get_tutorial)` and `("PUT", "/api/tutorial", put_tutorial)` in TS's
//! order.

use std::sync::Arc;

use indexmap::IndexSet;
use serde::Serialize;
use serde_json::{Value, json};

use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, json};
use crate::app::{App, now_ms};
use crate::config::{TUTORIAL_LESSON_ID_MAX_LENGTH, TUTORIAL_LESSONS_MAX};
use crate::db::store::{Profile, TutorialHiddenChoice, TutorialMergeInput, TutorialMergeOutcome, TutorialProgressRow};

/// R320: a lesson id is a lower-case slug (`basics`, `first-steps`): TS
/// `/^[a-z0-9]+(?:-[a-z0-9]+)*$/u`, checked by hand (the server carries no regex crate).
fn is_lesson_id(entry: &str) -> bool {
    !entry.is_empty()
        && entry
            .split('-')
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()))
}

/// What the client reads: `TutorialAccountProgress` in `apps/web/src/net/api.ts`.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TutorialProgressView {
    pub completed: Vec<String>,
    pub hidden_choice: Option<TutorialHiddenChoice>,
}

fn progress_view(row: Option<&TutorialProgressRow>) -> TutorialProgressView {
    match row {
        None => TutorialProgressView { completed: Vec::new(), hidden_choice: None },
        Some(row) => TutorialProgressView {
            completed: row.completed.clone(),
            hidden_choice: row
                .hidden_choice
                .as_ref()
                .map(|choice| TutorialHiddenChoice { hidden: choice.hidden, at: choice.at }),
        },
    }
}

/// TS `badRequest(message)` (`http.ts`), a private copy (fullsend builder rule 5).
fn bad_request(message: impl Into<String>) -> ApiError {
    ApiError { code: ApiErrorCode::BadRequest, message: message.into(), details: None, retry_after_ms: None }
}

/// TS `callerProfile(req)` (`collection.ts`), a private copy (fullsend builder rule 5): the caller
/// behind a route that declares `active`, which the router has already resolved and gated.
fn caller_profile(req: &Req) -> Result<&Profile, ApiError> {
    req.caller
        .as_ref()
        .map(|caller| &caller.profile)
        .ok_or_else(|| bad_request("this endpoint needs a signed-in profile"))
}

/// The router's catch-all for a failure that is not an `ApiError` (TS `createRouter`'s
/// `handler.threw` line and its 500), for the store's errors, which a TS handler let throw.
fn internal(path: &str, error: impl std::fmt::Display) -> ApiError {
    tracing::warn!(event = "handler.threw", path = path, message = %error);
    ApiError {
        code: ApiErrorCode::Internal,
        message: "something went wrong".to_string(),
        details: None,
        retry_after_ms: None,
    }
}

/// TS `Number.isSafeInteger(at)` on a JSON number, as the integer it is.
fn safe_integer(value: &Value) -> Option<i64> {
    const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;
    let number = if let Some(integer) = value.as_i64() {
        integer
    } else if let Some(float) = value.as_f64() {
        if !float.is_finite() || float.fract() != 0.0 || float.abs() > MAX_SAFE_INTEGER as f64 {
            return None;
        }
        float as i64
    } else {
        return None;
    };
    (number.abs() <= MAX_SAFE_INTEGER).then_some(number)
}

/// R320: `completed` is a list of lesson-id slugs, each at most `TUTORIAL_LESSON_ID_MAX_LENGTH`
/// characters; a repeated id counts once, and at most `TUTORIAL_LESSONS_MAX` distinct ids are taken.
pub fn read_completed(body: &Value) -> Result<Vec<String>, ApiError> {
    let Some(value) = body.get("completed").and_then(Value::as_array) else {
        return Err(bad_request("\"completed\" must be a list of lesson ids"));
    };
    let mut ids: IndexSet<String> = IndexSet::new();
    for entry in value {
        let id = entry.as_str().filter(|id| {
            id.encode_utf16().count() <= TUTORIAL_LESSON_ID_MAX_LENGTH && is_lesson_id(id)
        });
        let Some(id) = id else {
            return Err(bad_request(format!(
                "every lesson id must be a lower-case slug of at most {TUTORIAL_LESSON_ID_MAX_LENGTH} characters"
            )));
        };
        ids.insert(id.to_string());
    }
    if ids.len() > TUTORIAL_LESSONS_MAX {
        return Err(bad_request(format!("at most {TUTORIAL_LESSONS_MAX} lessons can be recorded")));
    }
    Ok(ids.into_iter().collect())
}

/// R320, R321: `hiddenChoice` is absent or null (no choice to propose), or `{ hidden, at }` with `at`
/// a whole number of epoch milliseconds, the choosing device's clock. A time after the server's own
/// is taken as now: a device whose clock runs ahead could otherwise make its choice win over every
/// later one for as long as its clock stays ahead.
pub fn read_hidden_choice(body: &Value, now: i64) -> Result<Option<TutorialHiddenChoice>, ApiError> {
    let value = match body.get("hiddenChoice") {
        None | Some(Value::Null) => return Ok(None),
        Some(value) => value,
    };
    let bad = || bad_request("\"hiddenChoice\" must be null or { hidden: boolean, at: epoch milliseconds }");
    let Some(object) = value.as_object() else {
        return Err(bad());
    };
    let hidden = object.get("hidden").and_then(Value::as_bool);
    let at = object.get("at").and_then(safe_integer);
    match (hidden, at) {
        (Some(hidden), Some(at)) if at >= 0 => Ok(Some(TutorialHiddenChoice { hidden, at: at.min(now) })),
        _ => Err(bad()),
    }
}

/// `GET /api/tutorial` (`active`, so a pending account gets 403, §9.4).
pub async fn get_tutorial(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let mut tx = app.db.begin(Some(profile.id.as_str())).await.map_err(|error| internal("/api/tutorial", error))?;
    let row = tx.tutorial_get(&profile.id).await.map_err(|error| internal("/api/tutorial", error))?;
    tx.commit().await.map_err(|error| internal("/api/tutorial", error))?;
    Ok(json(200, json!({ "progress": progress_view(row.as_ref()) })))
}

/// `PUT /api/tutorial` (`active`, so a pending account gets 403, §9.4).
pub async fn put_tutorial(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    // TS `deps.timers.now()`: the server's one clock (`app::now_ms`, which tokio's paused clock moves).
    let now = now_ms();
    let completed = read_completed(&req.body)?;
    let hidden_choice = read_hidden_choice(&req.body, now)?;
    let has_choice = hidden_choice.is_some();
    let sent = completed.len();

    let input = TutorialMergeInput { profile_id: profile.id.clone(), completed, hidden_choice, at: now };
    let mut tx = app.db.begin(Some(profile.id.as_str())).await.map_err(|error| internal("/api/tutorial", error))?;
    let outcome = tx
        .tutorial_merge(&input, TUTORIAL_LESSONS_MAX as i64)
        .await
        .map_err(|error| internal("/api/tutorial", error))?;
    tx.commit().await.map_err(|error| internal("/api/tutorial", error))?;

    let progress = match outcome {
        TutorialMergeOutcome::Limit => {
            return Err(ApiError {
                code: ApiErrorCode::Conflict,
                message: format!(
                    "This account already records {TUTORIAL_LESSONS_MAX} lessons, the most it can hold."
                ),
                details: Some(json!({ "limit": TUTORIAL_LESSONS_MAX })),
                retry_after_ms: None,
            });
        }
        TutorialMergeOutcome::Merged { progress } => progress,
    };
    tracing::info!(
        event = "tutorial.merged",
        "profileId" = %profile.id,
        sent = sent,
        completed = progress.completed.len(),
        choice = has_choice,
    );
    Ok(json(200, json!({ "progress": progress_view(Some(&progress)) })))
}
