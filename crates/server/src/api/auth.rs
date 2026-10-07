//! The endpoints the invite gate needs: the route half of `apps/server/src/api/auth.ts`. The
//! provider half (Supabase Auth, the JWT tiers, R194's liveness cache, R665's second factor) is
//! `crate::auth`.
//!
//! SPEC §9.4 lets a pending account "log in, verify its email and see the code screen, and nothing
//! else", so `/api/auth/me` is declared `AuthLevel::User` rather than `Active`: it is the code
//! screen's only read. The gate itself lives in `http.rs` (`assert_active`), never here.
//!
//! Routes (SURFACE §11.3): `POST /api/auth/signin` (it answers only under `E2E=1`, where the
//! fixture provider knows passwords; Supabase answers 503 as TS did), `GET /api/profile`,
//! `DELETE /api/account`, `GET /api/auth/me`. `POST /api/auth/signup` is not ported.

use std::future::Future;
use std::sync::Arc;

use axum::body::Body;
use axum::response::Response;
use serde_json::{Value, json};

use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Caller, Req, json};
use crate::app::App;
use crate::auth::{ACCOUNT_DELETION_UNAVAILABLE_MESSAGE, AuthError, Session};

// NOT IN SPEC, and no R-row yet — PROPOSED RULING for §11:
//   Topic: The scope of the identical sign-up and sign-in error (extends R145)
//   Ruling: Sign-up and sign-in answer identically for every outcome that depends on **whether an
//     account exists** — "no such account", "wrong password" and "already registered" — so neither
//     endpoint becomes an account-enumeration oracle. This is R145's principle one door earlier:
//     R145 makes the redemption error identical for everything that depends on the code and
//     distinct for everything that depends only on the caller's own account, and an email address
//     is exactly the fact an unauthenticated caller must not be able to probe. §9.8's brute-force
//     row asks for the invite gate to resist enumeration, which a sign-up endpoint that says "that
//     address is taken" undoes. One error per endpoint rather than one for both, because the two
//     endpoints are already distinguishable by the route.
//   Affects: §9.2, §9.4, §9.8, R145; `api/auth.ts`.
//
// §9.4 does not write these strings; the ruling above is about their being one string, not about
// their wording. (Sign-up's own string went with the route, SURFACE §11.3.)
const SIGN_IN_FAILED_MESSAGE: &str = "That email and password do not match an account.";

// Not in SPEC, and no R-row: wording only, for `DELETE /api/account`.
const ACCOUNT_DELETION_IN_MATCH_MESSAGE: &str = "Finish or concede your match before you delete your account.";
const ACCOUNT_DELETION_IN_SERIES_MESSAGE: &str = "Finish your Conquest series before you delete your account.";

fn unauthorized() -> ApiError {
    ApiError::new(ApiErrorCode::Unauthorized, "sign in first")
}

/// TS `str(body, key)` (http.ts): a non-empty string field, or a 400 naming it.
fn required_str(body: &Value, key: &str) -> Result<String, ApiError> {
    match body.get(key) {
        Some(Value::String(value)) if !value.is_empty() => Ok(value.clone()),
        _ => Err(ApiError::new(ApiErrorCode::BadRequest, format!("\"{key}\" must be a string"))),
    }
}

/// The caller of a route that declared `User` or `Active`; the guard is here because `Req` types it
/// optional.
fn caller_of(req: &Req) -> Result<&Caller, ApiError> {
    req.caller.as_ref().ok_or_else(unauthorized)
}

/// Anything the provider rejects becomes a 401 rather than a 500: a rejected sign-in is an expected
/// outcome, not a server fault, and the provider's own wording never reaches the client (it would
/// distinguish "no such account" from "wrong password"). An unavailable answer the provider raised
/// itself (the 503 for a server that does not broker passwords) is passed through unchanged.
async fn call_provider<T>(
    event: &'static str,
    message: &str,
    call: impl Future<Output = Result<T, AuthError>>,
) -> Result<T, ApiError> {
    match call.await {
        Ok(value) => Ok(value),
        Err(AuthError::Unavailable(text)) => Err(ApiError::new(ApiErrorCode::Unavailable, text)),
        Err(error) => {
            // Tokens and keys are never part of these arguments, so nothing secret is logged.
            tracing::warn!(event = event, reason = %error);
            Err(ApiError::new(ApiErrorCode::Unauthorized, message))
        }
    }
}

fn session_body(session: &Session) -> Value {
    json!({
        "accessToken": session.access_token,
        "refreshToken": session.refresh_token,
        "expiresAt": session.expires_at,
    })
}

/// `POST /api/auth/signin` (`AuthLevel::None`). §9.2 puts sign-in in the browser, against Supabase
/// Auth; this is the server-side equivalent BUILD M8's fixture accounts use under `E2E=1`. On a
/// Supabase deployment the provider answers 503 with where sign-in actually happens.
pub async fn sign_in(app: &Arc<App>, req: Req) -> ApiResult {
    let email = required_str(&req.body, "email")?;
    let password = required_str(&req.body, "password")?;
    let session = call_provider("auth.signin_rejected", SIGN_IN_FAILED_MESSAGE, app.auth.sign_in(&email, &password)).await?;
    Ok(json(
        200,
        json!({
            "userId": session.user.user_id,
            "emailVerified": session.user.email_verified,
            "session": session_body(&session),
        }),
    ))
}

/// `GET /api/profile` (`AuthLevel::Active`): the account screen: who you are signed in as, and how
/// you have done.
///
/// Separate from `/api/auth/me` rather than folded into it because `me` is the GATE's read —
/// every guarded route resolves through it — and counting a profile's whole results history on
/// each of those would put a scan behind every page load. This one is `active`, so it is only
/// reachable by an account that can actually have a record.
pub async fn get_profile(app: &Arc<App>, req: Req) -> ApiResult {
    let caller = caller_of(&req)?;
    let profile = &caller.profile;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let record = tx.results_record_for(&profile.id).await?;
    tx.commit().await?;
    let played = record.wins as i64 + record.losses as i64 + record.draws as i64;
    Ok(json(
        200,
        json!({
            "id": profile.id,
            "email": caller.user.email,
            "status": profile.status,
            // R612: never the hidden rating. The rank it moves is `GET /api/ranked`'s.
            "record": { "wins": record.wins, "losses": record.losses, "draws": record.draws },
            // Computed here so the client cannot disagree with itself about what counts as a played
            // match. Draws count as played and as neither win nor loss, which is the convention every
            // ladder uses; null rather than 0 when nothing has been played, so the screen can say
            // "no matches yet" instead of "0%".
            "winRate": if played == 0 { Value::Null } else { json!(record.wins as f64 / played as f64) },
        }),
    ))
}

/// `DELETE /api/account` (`AuthLevel::User`): deletes the caller's own account: the profile and
/// everything only it owns (the store, with migration 0012's foreign keys), then the sign-in itself
/// (the auth provider). `User`, not `Active`, so a pending or banned account can leave too. 204 with
/// no body on success.
///
/// A player in a live match or a Conquest series is refused with 409 rather than having the
/// game ended for them: the actor and the series rules own how a game ends, and conceding
/// first is one click away. A queue ticket or an unjoined room is simply deleted with the
/// account. The profile goes first, so a provider that fails afterwards leaves nothing but a
/// sign-in; the same request, retried, removes the fresh pending profile that sign-in gets and
/// tries the provider again.
pub async fn delete_account(app: &Arc<App>, req: Req) -> ApiResult {
    let caller = caller_of(&req)?;
    let profile = &caller.profile;
    if !app.auth.can_delete_users() {
        return Err(ApiError::new(ApiErrorCode::Unavailable, ACCOUNT_DELETION_UNAVAILABLE_MESSAGE));
    }
    if profile.in_match_id.is_some() {
        return Err(ApiError::new(ApiErrorCode::AlreadyInMatch, ACCOUNT_DELETION_IN_MATCH_MESSAGE));
    }
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let series = tx.series_active_for(&profile.id).await?;
    tx.commit().await?;
    if series.is_some() {
        return Err(ApiError::new(ApiErrorCode::Conflict, ACCOUNT_DELETION_IN_SERIES_MESSAGE));
    }

    let mut tx = app.db.begin(Some(&profile.id)).await?;
    tx.profiles_remove(&profile.id).await?;
    tx.commit().await?;
    match app.auth.delete_user(&caller.user.user_id).await {
        Ok(()) => {}
        Err(AuthError::Unavailable(message)) => return Err(ApiError::new(ApiErrorCode::Unavailable, message)),
        Err(error) => return Err(ApiError::new(ApiErrorCode::Unavailable, error.to_string())),
    }
    // No id in the line: the account is gone, and so is the reason to name it.
    tracing::info!(event = "account.deleted");
    let mut response = Response::new(Body::empty());
    *response.status_mut() = axum::http::StatusCode::NO_CONTENT;
    response.headers_mut().insert("cache-control", axum::http::HeaderValue::from_static("no-store"));
    Ok(response)
}

/// `GET /api/auth/me` (`AuthLevel::User`). §9.4: declared `user`, not `active`, because this *is* the
/// code screen's read — a pending account must be able to see that it needs a code. Every other
/// authenticated endpoint is `active` and 403s for the same caller (BUILD M6-T1).
pub async fn get_me(app: &Arc<App>, req: Req) -> ApiResult {
    let caller = caller_of(&req)?;
    let profile = &caller.profile;
    // R259, R264: the Conquest series this profile is in, while it is not over. Between games
    // `currentMatchId` is null and this is the only way a player who waited — the older ticket,
    // or the room's host — learns there is a deck to pick. Its own series only, like the match.
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let series = tx.series_active_for(&profile.id).await?;
    tx.commit().await?;
    let needs_invite_code = matches!(profile.status, crate::db::store::ProfileStatus::Pending);
    Ok(json(
        200,
        json!({
            // R612: the hidden rating is never sent, not even to its owner.
            "profile": { "id": profile.id, "status": profile.status },
            // §9.4: "Redeeming an invite code flips pending to active", so only a pending account is
            // shown the code screen. A banned account is not offered a way out of it.
            "needsInviteCode": needs_invite_code,
            "emailVerified": caller.user.email_verified,
            // §9.5: non-null while this profile is in a match, cleared by every ending. Without it a
            // player who was WAITING — the host of a room, or the first ticket in the queue — is never
            // told the match they are already in: the other player's HTTP response carried the id and
            // theirs did not, so they sat on /play while their opponent sat on the board. It is also
            // the only way back into a match after a reload that lost the URL.
            //
            // Safe to return to its owner: it is this caller's own profile row, the same row whose
            // status is already here, and a match id is not a capability — the socket
            // still authenticates and the actor still stamps the seat from the token (§9.3).
            "currentMatchId": profile.in_match_id,
            "currentSeriesId": series.map(|series| series.id),
            // The address this account is tied to, so a player can see WHICH account they are signed
            // in as. Read from the auth provider's user (the same place §9.4 step 1 reads
            // `emailVerified` from), never from the token's user_metadata, which is user-editable.
            "email": caller.user.email,
        }),
    ))
}
