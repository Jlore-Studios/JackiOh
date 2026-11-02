//! Tutorial progress on the account (SPEC §9.10, R320, R294, R321; §9.4, SURFACE §11.2).
//!
//! ```text
//!   GET /api/tutorial  -> { progress: { completed, hiddenChoice } }
//!   PUT /api/tutorial  <- { completed: string[], hiddenChoice?: { hidden, at } | null }
//!                      -> { progress } as it stands after the merge
//! ```
//!
//! PUT merges completed lessons and hidden choices without removing progress (R320, R187).
//! Enforces length limits (`TUTORIAL_LESSON_ID_MAX_LENGTH`, `TUTORIAL_LESSONS_MAX`).

use std::sync::Arc;

use indexmap::IndexSet;
use serde::Serialize;
use serde_json::{Value, json};

use crate::api::collection::caller_profile;
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, bad_request, json};
use crate::app::{App, now_ms};
use crate::config::{TUTORIAL_LESSON_ID_MAX_LENGTH, TUTORIAL_LESSONS_MAX};
use crate::db::store::{TutorialHiddenChoice, TutorialMergeInput, TutorialMergeOutcome, TutorialProgressRow};

/// R320: checks whether lesson id is a lower-case slug.
fn is_lesson_id(entry: &str) -> bool {
    !entry.is_empty()
        && entry.split('-').all(|part| {
            !part.is_empty() && part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

/// Client tutorial progress representation.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TutorialProgressView {
    pub completed: Vec<String>,
    pub hidden_choice: Option<TutorialHiddenChoice>,
}

fn progress_view(row: Option<&TutorialProgressRow>) -> TutorialProgressView {
    match row {
        None => TutorialProgressView {
            completed: Vec::new(),
            hidden_choice: None,
        },
        Some(row) => TutorialProgressView {
            completed: row.completed.clone(),
            hidden_choice: row.hidden_choice.as_ref().map(|choice| TutorialHiddenChoice {
                hidden: choice.hidden,
                at: choice.at,
            }),
        },
    }
}

/// Checks whether a JSON number is a safe integer.
fn safe_integer(value: &Value) -> Option<i64> {
    const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;
    let number = if let Some(integer) = value.as_i64() {
        integer
    } else {
        let float = value.as_f64()?;
        if !float.is_finite() || float.fract() != 0.0 || float.abs() > MAX_SAFE_INTEGER as f64 {
            return None;
        }
        float as i64
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
        let id = entry
            .as_str()
            .filter(|id| id.encode_utf16().count() <= TUTORIAL_LESSON_ID_MAX_LENGTH && is_lesson_id(id));
        let Some(id) = id else {
            return Err(bad_request(format!(
                "every lesson id must be a lower-case slug of at most {TUTORIAL_LESSON_ID_MAX_LENGTH} characters"
            )));
        };
        ids.insert(id.to_string());
    }
    if ids.len() > TUTORIAL_LESSONS_MAX {
        return Err(bad_request(format!(
            "at most {TUTORIAL_LESSONS_MAX} lessons can be recorded"
        )));
    }
    Ok(ids.into_iter().collect())
}

/// R320, R321: parses optional `{ hidden, at }` choice, clamping future timestamps to now.
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
        (Some(hidden), Some(at)) if at >= 0 => Ok(Some(TutorialHiddenChoice {
            hidden,
            at: at.min(now),
        })),
        _ => Err(bad()),
    }
}

/// `GET /api/tutorial` (`active`, so a pending account gets 403, §9.4).
pub async fn get_tutorial(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let mut tx = app.db.begin(Some(profile.id.as_str())).await?;
    let row = tx.tutorial_get(&profile.id).await?;
    tx.commit().await?;
    Ok(json(200, json!({ "progress": progress_view(row.as_ref()) })))
}

/// `PUT /api/tutorial` (`active`, so a pending account gets 403, §9.4).
pub async fn put_tutorial(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let now = now_ms();
    let completed = read_completed(&req.body)?;
    let hidden_choice = read_hidden_choice(&req.body, now)?;
    let has_choice = hidden_choice.is_some();
    let sent = completed.len();

    let input = TutorialMergeInput {
        profile_id: profile.id.clone(),
        completed,
        hidden_choice,
        at: now,
    };
    let mut tx = app.db.begin(Some(profile.id.as_str())).await?;
    let outcome = tx.tutorial_merge(&input, TUTORIAL_LESSONS_MAX as i64).await?;
    tx.commit().await?;

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
