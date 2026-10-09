//! The username routes (SPEC §9.4, R1432–R1435): the preview a player sees while typing, the save,
//! and the skip that answers the prompt after activation. Every route acts on the caller's own
//! account, named by the token; none takes a profile id or another player's name.
//!
//! The server is the only judge of a name (§9.1, CLAUDE.md rule 7). The client sends what was
//! typed, shows the preview's verdict as it stands, and saves exactly the username that verdict
//! named; a save whose outcome has changed since (someone took `Max` first, so it would now be
//! `Max#1`) is refused with 409 and a fresh preview, so a player never ends up with a tag they were
//! not shown. The checks are `crate::username`; the taken-name lookup and the claim are the store's
//! (`profiles_username_tag_for`, `profiles_claim_username`). All three routes are
//! `AuthLevel::Active`, so R109's per-account limit counts them like every other request.
//!
//! A name is never logged (R1436): it is a label the player chose, and nothing here needs it in a
//! log line to be diagnosed.

use std::sync::Arc;

use serde_json::{Value, json};

use crate::api::collection::caller_profile;
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, bad_request, json, now_ms, str};
use crate::app::App;
use crate::config::{USERNAME_CHANGE_COOLDOWN_MS, USERNAME_CHANGE_COOLDOWN_SECONDS};
use crate::db::store::{Profile, ProfileStatus, UsernameClaim, UsernameClaimOutcome};
use crate::username::{normalize_username, parse_username, render_username, username_key};

/// Unit conversion, not configuration: the cooldown is stated in seconds and said in hours.
const SECONDS_PER_HOUR: i64 = 3600;

/// R1435: a preview's `reason` while the cooldown runs. Its sentence is [`cooldown_message`].
const COOLDOWN_REASON: &str = "cooldown";

/// R1435: a preview's `reason` for the name the player already has.
const UNCHANGED_REASON: &str = "unchanged";

/// R1435: what a preview says of the name the player already has.
const UNCHANGED_MESSAGE: &str = "That's already your username.";

/// R1435: a refused save's message, beside the fresh preview in its `details`.
const CHANGED_MESSAGE: &str = "the username a save would give has changed since the preview";

/// R1435: a refused save's message for a body that names no username.
const NOT_A_USERNAME_MESSAGE: &str = "\"username\" must be a username as a preview gave it";

/// R1435: the sentence a preview shows while the cooldown runs; `nextChangeAt` says when it ends.
fn cooldown_message() -> String {
    format!(
        "You can change your username once every {} hours.",
        USERNAME_CHANGE_COOLDOWN_SECONDS / SECONDS_PER_HOUR
    )
}

/// R1435: when the caller may next change their username, or none when they may now. The first
/// change away from the default is never blocked: a profile that never changed has no clock.
fn next_change_at(profile: &Profile, now: i64) -> Option<i64> {
    profile
        .username_changed_at
        .map(|changed_at| changed_at + USERNAME_CHANGE_COOLDOWN_MS)
        .filter(|next| *next > now)
}

/// R1435: the caller's own username as `GET /api/auth/me` and every username route answer with it:
/// the name as shown, when the next change is allowed (null while one is), and whether the prompt
/// after activation is still owed. Only ever the caller's own: no read about another player carries
/// the last two (R1436).
pub fn own_username(profile: &Profile, now: i64) -> Value {
    json!({
        "name": profile.username(),
        "nextChangeAt": next_change_at(profile, now),
        "promptOwed": profile.status == ProfileStatus::Active && !profile.username_prompted,
    })
}

/// A preview while the cooldown runs.
fn cooldown_preview(next_change_at: i64) -> Value {
    json!({
        "ok": false,
        "reason": COOLDOWN_REASON,
        "message": cooldown_message(),
        "nextChangeAt": next_change_at,
    })
}

/// R1435: what a save of `raw` would give the caller now, checked in this order: the cooldown, the
/// name's form and the filter (R1432, R1433), then the tag it would carry (R1434) and whether that
/// is the name they already have. `{ ok: true, base, username, tagged }` names the exact username
/// a save would give, `base` in its stored NFKC form and `tagged` true when the bare base is taken.
async fn preview(app: &App, profile: &Profile, raw: &str, now: i64) -> Result<Value, ApiError> {
    if let Some(next_change_at) = next_change_at(profile, now) {
        return Ok(cooldown_preview(next_change_at));
    }
    let base = match normalize_username(raw) {
        Ok(base) => base,
        Err(refusal) => {
            return Ok(json!({ "ok": false, "reason": refusal.code(), "message": refusal.message() }));
        }
    };
    let key = username_key(&base);
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let tag = tx.profiles_username_tag_for(&profile.id, &key).await?;
    tx.commit().await?;
    let username = render_username(&base, tag);
    if username == profile.username() {
        return Ok(json!({ "ok": false, "reason": UNCHANGED_REASON, "message": UNCHANGED_MESSAGE }));
    }
    Ok(json!({ "ok": true, "base": base, "username": username, "tagged": tag.is_some() }))
}

/// A refused save: `code`'s status, with the preview that says why in `details`.
fn refused(code: ApiErrorCode, message: impl Into<String>, preview: Value) -> ApiError {
    ApiError::with_details(code, message, preview)
}

// The three `AuthLevel::Active` rows of `app.rs`'s `ROUTES`: GET /api/username/preview →
// get_preview; PUT /api/username → put_username; POST /api/username/skip → post_skip.

/// `GET /api/username/preview?name=…`: the preview of `name` (R1435). Always 200: a refused name
/// is a verdict, not an error. An empty or missing `name` is previewed as typed, too short.
pub async fn get_preview(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let raw = req.query.get("name").map(String::as_str).unwrap_or("");
    Ok(json(200, preview(app, &profile, raw, now_ms()).await?))
}

/// `PUT /api/username` with `{ "username": "Max#3" }`, the username a preview gave, exactly (R1435).
///
/// - 200 `{ username }`, the caller's own username as `/me` carries it, once the claim lands: the
///   cooldown starts and the prompt is answered.
/// - 400 `bad_request` for a body that is not a username, or one whose base the checks refuse
///   (`details` is that refusal, as a preview gives it).
/// - 409 `conflict` when the save no longer gives what was sent: the base would carry another tag
///   now, the name is the caller's already, the cooldown runs, or the base was not sent in its
///   stored form. `details` is a fresh preview of the sent base, which the client shows instead.
pub async fn put_username(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let sent = str(&req.body, "username")?;
    let Some((base, expected_tag)) = parse_username(&sent) else {
        return Err(bad_request(NOT_A_USERNAME_MESSAGE));
    };
    let now = now_ms();
    let stored = match normalize_username(base) {
        Ok(stored) => stored,
        Err(refusal) => {
            return Err(refused(
                ApiErrorCode::BadRequest,
                refusal.message(),
                json!({ "ok": false, "reason": refusal.code(), "message": refusal.message() }),
            ));
        }
    };
    if stored != base || render_username(&stored, expected_tag) == profile.username() {
        return Err(refused(
            ApiErrorCode::Conflict,
            CHANGED_MESSAGE,
            preview(app, &profile, base, now).await?,
        ));
    }
    let claim = UsernameClaim {
        profile_id: profile.id.clone(),
        key: username_key(&stored),
        base: stored,
        expected_tag,
        at: now,
        cooldown_ms: USERNAME_CHANGE_COOLDOWN_MS,
    };
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    match tx.profiles_claim_username(&claim).await? {
        UsernameClaimOutcome::Claimed { .. } => {
            let saved = tx.profiles_get_by_id(&profile.id).await?;
            tx.commit().await?;
            let saved = saved.ok_or_else(|| ApiError::internal("the profile that claimed a username is gone"))?;
            Ok(json(200, json!({ "username": own_username(&saved, now) })))
        }
        UsernameClaimOutcome::Cooldown { next_change_at } => {
            drop(tx);
            Err(refused(ApiErrorCode::Conflict, cooldown_message(), cooldown_preview(next_change_at)))
        }
        UsernameClaimOutcome::Changed { .. } => {
            drop(tx);
            Err(refused(
                ApiErrorCode::Conflict,
                CHANGED_MESSAGE,
                preview(app, &profile, base, now).await?,
            ))
        }
    }
}

/// `POST /api/username/skip`: "Skip for now" (R1435). Answers the prompt after activation and keeps
/// the name as it is; 200 `{ username }`. Answering it again changes nothing.
pub async fn post_skip(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    tx.profiles_answer_username_prompt(&profile.id).await?;
    let answered = tx.profiles_get_by_id(&profile.id).await?;
    tx.commit().await?;
    let answered = answered.ok_or_else(|| ApiError::internal("the profile that skipped the prompt is gone"))?;
    Ok(json(200, json!({ "username": own_username(&answered, now_ms()) })))
}
