//! Play together's social layer (SPEC S7, #936; issues #937, #938, #941): friends by `name#tag`,
//! blocks, presence, challenges and chat between friends.
//!
//! - `friends`: the friend list, requests (B43–B45) and blocks (B46).
//! - `presence`: the heartbeat and each friend's status, `offline`, `online` or `inMatch` (B47).
//! - `challenges`: a challenge to a friend (B48, B49).
//! - `chat`: messages between friends and retention purge (B53).
//!
//! Every route is `AuthLevel::Active` and acts for its caller, named by the token. A player is
//! named by profile id everywhere and shown by username (`PlayerRef`, R1436); no log line carries a
//! username or a message's body. Times leave as ISO-8601 strings (`iso`), as S7's types write them.

pub mod challenges;
pub mod chat;
pub mod friends;
pub mod presence;
pub mod shares;
pub mod watch;

use axum::body::Body;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;
use indexmap::IndexMap;
use serde::Serialize;
use serde_json::{Value, json};
use time::OffsetDateTime;

use crate::api::http::{
    ApiError, ApiErrorCode, ApiResult, RateLimiter, Req, create_rate_limiter, json as json_response,
};
use crate::config::{CHALLENGES_PER_HOUR, CHAT_MESSAGES_PER_MINUTE, FRIEND_REQUESTS_PER_HOUR, NANOS_PER_MS};
use crate::db::store::{Profile, StoreError, Tx};

/// Unit conversions, not configuration (a day's and a millisecond's are `crate::config`'s).
pub(crate) const MS_PER_MINUTE: i64 = 60_000;
pub(crate) const MS_PER_HOUR: i64 = 60 * MS_PER_MINUTE;

/// `Date.prototype.toISOString`'s form, which S7's time strings take: `2026-10-14T12:34:56.789Z`.
const ISO_FORMAT: &str = "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z";

/// What the social routes keep between requests, one per `App`: their own allowances (R109).
pub struct Social {
    /// B43: `FRIEND_REQUESTS_PER_HOUR` friend requests an hour per account.
    pub friend_requests: RateLimiter,
    /// B53: `CHAT_MESSAGES_PER_MINUTE` chat messages a minute per sender.
    pub chat: RateLimiter,
    /// B48: `CHALLENGES_PER_HOUR` challenges an hour per challenger.
    pub challenges: RateLimiter,
}

impl Default for Social {
    fn default() -> Social {
        Social {
            friend_requests: create_rate_limiter(FRIEND_REQUESTS_PER_HOUR, MS_PER_HOUR),
            chat: create_rate_limiter(CHAT_MESSAGES_PER_MINUTE, MS_PER_MINUTE),
            challenges: create_rate_limiter(CHALLENGES_PER_HOUR, MS_PER_HOUR),
        }
    }
}

/// S7's `PlayerRef`: a player as every social answer names one, by profile id, with the username
/// as shown (`Max`, or `Max#3` with a tag, R1432).
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerRef {
    pub profile_id: String,
    pub username: String,
}

impl PlayerRef {
    pub fn of(profile: &Profile) -> PlayerRef {
        PlayerRef {
            profile_id: profile.id.clone(),
            username: profile.username(),
        }
    }
}

/// The named profiles by id, one store read, for answers that name players. An id whose profile
/// is gone is left out (its rows cascaded with it).
pub(crate) async fn profiles_by_id(
    tx: &mut Tx<'_>,
    ids: &[String],
) -> Result<IndexMap<String, Profile>, StoreError> {
    let mut wanted: Vec<String> = ids.to_vec();
    wanted.sort();
    wanted.dedup();
    Ok(tx
        .profiles_get_many(&wanted)
        .await?
        .into_iter()
        .map(|profile| (profile.id.clone(), profile))
        .collect())
}

/// Epoch ms as an ISO-8601 instant in UTC, as `Date.prototype.toISOString` writes it, or the
/// number itself for an instant `OffsetDateTime` cannot hold.
pub fn iso(ms: i64) -> String {
    try_iso(ms).unwrap_or_else(|_| ms.to_string())
}

/// `iso`, or why the instant cannot be written: `mint-code` fails on that rather than print a
/// number for an invite's expiry (`cli::mint_code`).
pub(crate) fn try_iso(ms: i64) -> Result<String, time::Error> {
    let at = OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * NANOS_PER_MS)?;
    let format = time::format_description::parse_borrowed::<1>(ISO_FORMAT)?;
    Ok(at.format(&format)?)
}

/// A refusal with a code of S7's own: not logged, answering `{ "error": { code, message } }`.
pub(crate) fn refused(status: u16, code: &str, message: &str) -> ApiResult {
    Ok(json_response(
        status,
        json!({ "error": { "code": code, "message": message } }),
    ))
}

/// 404 `not_found`: what every social route answers for a player, request, challenge or message the
/// caller may not reach, a block included, so an answer never tells "blocked" from "unknown" (B43).
pub(crate) fn not_found(message: &str) -> ApiError {
    ApiError::new(ApiErrorCode::NotFound, message)
}

pub(crate) fn created(body: Value) -> Response {
    json_response(201, body)
}

pub(crate) fn no_content() -> Response {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NO_CONTENT;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

pub(crate) fn param(req: &Req, name: &str) -> String {
    req.params.get(name).cloned().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_writes_what_to_iso_string_writes() {
        assert_eq!(iso(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso(1_760_445_296_789), "2025-10-14T12:34:56.789Z");
        assert_eq!(
            try_iso(1_760_445_296_789).ok().as_deref(),
            Some("2025-10-14T12:34:56.789Z")
        );
    }

    /// An instant past what `OffsetDateTime` holds: the social routes write the number, and
    /// `mint-code` fails with the range error's own words.
    #[test]
    fn an_instant_out_of_range_is_its_number_or_the_range_error() {
        assert_eq!(iso(i64::MAX), i64::MAX.to_string());
        let range = OffsetDateTime::from_unix_timestamp_nanos(i128::from(i64::MAX) * NANOS_PER_MS)
            .err()
            .map(|error| error.to_string());
        assert!(range.is_some());
        assert_eq!(try_iso(i64::MAX).err().map(|error| error.to_string()), range);
    }

    /// The social routes' and the co-op rooms' refusals (`duos::rooms`): the API's one error shape,
    /// under a code of their own, answered rather than failed.
    #[tokio::test]
    async fn a_refusal_is_the_api_error_shape_under_its_own_code() {
        let Ok(response) = refused(409, "full", "someone already joined that room") else {
            panic!("a refusal is an answer, not a failure");
        };
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL),
            Some(&HeaderValue::from_static("no-store"))
        );
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("the body");
        assert_eq!(
            serde_json::from_slice::<Value>(&bytes).ok(),
            Some(json!({ "error": { "code": "full", "message": "someone already joined that room" } }))
        );
    }
}
