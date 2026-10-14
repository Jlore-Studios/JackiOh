//! Sharing a finished match's replay with a friend (SPEC S7, B52; #940).
//!
//! `POST /api/replays/:matchId/shares` `{ friendId }`: a seat holder of the finished match shares its
//! replay with a friend, 204. A caller who held no seat in a finished match of that id, and a
//! `friendId` that is not the caller's friend or that either side has blocked, get the one 404, so
//! neither a match's existence nor a friendship leaks. `DELETE /api/replays/:matchId/shares/:friendId`
//! takes the caller's own share back, 204 whether or not there was one. `GET /api/replays/shared`
//! lists what friends have shared with the caller, newest first: `{ shared: SharedReplay[] }`.
//!
//! The friend reads the replay through `GET /api/replays/:matchId`, from the sharer's seat
//! (`api/replays.rs`'s `replay_reader`). Every read, there and here, judges the friendship anew
//! (`shares_sharers`, `shares_for`), so an unshare, an unfriend or a block ends it at once; anyone
//! else still gets the unknown-id answer (R768).

use std::sync::Arc;

use axum::body::Body;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;
use indexmap::IndexSet;
use serde::Serialize;
use serde_json::json;

use crate::api::collection::caller_profile;
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, log_info, now_ms, ok_of, str};
use crate::app::App;
use crate::db::store::{MatchPhase, ReplayShare};

/// Unit conversion, not configuration: epoch milliseconds to the nanoseconds `time` counts.
const NANOS_PER_MS: i128 = 1_000_000;

/// S7's `PlayerRef`: a profile as the client shows it, its username rendered as the match screen
/// renders one (`name`, or `name#tag` with a tag; R1436).
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerRef {
    pub profile_id: String,
    pub username: String,
}

/// S7's `SharedReplay`: one replay a friend shared with the caller. The times are ISO 8601 strings,
/// as S7 types them.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SharedReplay {
    pub match_id: String,
    pub sharer: PlayerRef,
    pub created_at: String,
    pub ended_at: Option<String>,
}

/// `GET /api/replays/shared`'s answer.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct SharedReplays {
    pub shared: Vec<SharedReplay>,
}

/// JS `new Date(ms).toISOString()`: `2026-10-14T12:34:56.789Z`, the form S7's time strings take.
pub fn iso_string(ms: i64) -> String {
    let format = time::format_description::parse_borrowed::<1>(
        "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z",
    );
    time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * NANOS_PER_MS)
        .ok()
        .zip(format.ok())
        .and_then(|(at, format)| at.format(&format).ok())
        .unwrap_or_else(|| ms.to_string())
}

/// The one answer for a match the caller may not share and a friend it may not share with.
fn no_such_share() -> ApiError {
    ApiError::new(ApiErrorCode::NotFound, "no such replay or friend")
}

/// 204, no body.
fn no_content() -> Response {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NO_CONTENT;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn param<'a>(req: &'a Req, name: &str) -> &'a str {
    req.params.get(name).map(String::as_str).unwrap_or("")
}

/// `POST /api/replays/:matchId/shares` `{ friendId }`.
pub async fn share_replay(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let match_id = param(&req, "matchId");
    let friend_id = str(&req.body, "friendId")?;

    let mut tx = app.db.begin(Some(&profile.id)).await?;
    // B52: only a seat holder shares, and only a finished match.
    let seated = tx
        .matches_seats(match_id)
        .await?
        .is_some_and(|seats| seats.phase == MatchPhase::Over && seats.players.contains(&profile.id));
    if !seated || !tx.watch_friends_unblocked(&profile.id, &friend_id).await? {
        return Err(no_such_share());
    }
    tx.shares_put(&ReplayShare {
        match_id: match_id.to_string(),
        sharer: profile.id.clone(),
        friend: friend_id,
        created_at: now_ms(),
    })
    .await?;
    tx.commit().await?;
    log_info("replay.shared", json!({ "matchId": match_id }));
    Ok(no_content())
}

/// `DELETE /api/replays/:matchId/shares/:friendId`: the caller's own share, taken back.
pub async fn unshare_replay(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    tx.shares_remove(param(&req, "matchId"), &profile.id, param(&req, "friendId"))
        .await?;
    tx.commit().await?;
    Ok(no_content())
}

/// `GET /api/replays/shared`.
pub async fn list_shared(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let rows = tx.shares_for(&profile.id).await?;
    let sharers: Vec<String> = rows
        .iter()
        .map(|row| row.sharer.clone())
        .collect::<IndexSet<String>>()
        .into_iter()
        .collect();
    // R1436: each username read now, so a rename shows here too.
    let profiles = tx.profiles_get_many(&sharers).await?;
    tx.commit().await?;

    let shared = rows
        .into_iter()
        .filter_map(|row| {
            let sharer = profiles.iter().find(|found| found.id == row.sharer)?;
            Some(SharedReplay {
                match_id: row.match_id,
                sharer: PlayerRef {
                    profile_id: sharer.id.clone(),
                    username: sharer.username(),
                },
                created_at: iso_string(row.created_at),
                ended_at: row.ended_at.map(iso_string),
            })
        })
        .collect();
    ok_of(&SharedReplays { shared })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_read_as_javascript_writes_them() {
        assert_eq!(iso_string(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso_string(1_500), "1970-01-01T00:00:01.500Z");
    }
}
