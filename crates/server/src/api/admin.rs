//! The dev dashboard's routes (SPEC §14.10, §14.13; issue #3074). The dashboard is `apps/admin`, a
//! separate app behind Cloudflare Access that only sends intent (CLAUDE.md rule 7); every route here
//! checks for itself that the caller is a dev account (`api::dev::dev_caller`, R1300000) and, on a
//! strict server (`DEV_STRICT`, R3074000), that the caller's session proved the two-step code
//! (`aal2`), and answers anyone else 403 `forbidden` before it reads the request's body.
//!
//! - `GET /api/admin/me`: who the caller is, and whether this server is strict.
//! - `GET /api/admin/rating`, `PUT /api/admin/rating`, `POST /api/admin/rating/reset` (R3074001): the
//!   caller's own rating on each ladder, set by hand within `DEV_RATING_MIN`..`DEV_RATING_MAX`, or
//!   reset to the one its games had earned.
//! - `GET /api/admin/leaderboard` (R3074002): every row of a ladder, each marked person or bot.
//! - `GET /api/admin/news`, `POST /api/admin/news`, `DELETE /api/admin/news/:postId` (R3074003):
//!   news posts with no reward, checked as `news.json`'s entries are, and taken down.
//! - `GET /api/admin/audit` (R3074000): the newest `ADMIN_AUDIT_PAGE` audit rows.
//!
//! Every write is one transaction that writes its audit row ([`audit`]) beside what it changes, as
//! every `api::dev` and quest tool does.

use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::{Value, json};

use crate::admin::store::{
    HandSetRating, NEWS_POST_ID_PREFIX, NewAdminAction, NewsPost, RatingChange, RatingGrant,
};
use crate::api::dev::dev_caller;
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, bad_request, caller_of, json, ok_of};
use crate::api::ranked::dev_leaderboard;
use crate::app::{App, now_ms};
use crate::config::{ADMIN_AUDIT_PAGE, DEV_RATING_MAX, DEV_RATING_MIN, NEWS_POSTS_PAGE};
use crate::db::store::{Profile, Tx};
use crate::economy::news::post_problems;
use crate::ranked::glicko2::Glicko;
use crate::ranked::ladder::Ladder;

/// R3074000: the 403 a dev account's session without the two-step code meets on a strict server.
const TWO_STEP_MESSAGE: &str =
    "The dev dashboard needs your two-step code on this server: sign in with your authenticator app.";

/// How many hex digits of a fresh UUID a post's id carries after its prefix: 48 bits, so two posts
/// never meet in practice, and the table's key refuses the one that would.
const POST_ID_HEX_DIGITS: usize = 12;

/// R3074000: the caller, when it is a dev account and, on a strict server, its session proved the
/// two-step code; 403 `forbidden` otherwise. Every route here asks first.
pub(crate) fn admin_caller(app: &App, req: &Req) -> Result<Profile, ApiError> {
    let profile = dev_caller(app, req)?;
    if app.env.dev_strict && !caller_of(req)?.user.second_factor {
        return Err(ApiError::new(ApiErrorCode::Forbidden, TWO_STEP_MESSAGE));
    }
    Ok(profile)
}

/// R3074000: the audit row of one dev or admin action, in the transaction that does it: who, which
/// route (`<METHOD> <path>`, the path as the request named it), the request, the values before and
/// after, and when.
pub(crate) async fn audit(
    tx: &mut Tx<'_>,
    profile_id: &str,
    route: String,
    request: Value,
    before: Value,
    after: Value,
    at: i64,
) -> Result<(), ApiError> {
    tx.admin_actions_append(&NewAdminAction {
        profile_id: profile_id.to_string(),
        route,
        request,
        before,
        after,
        at,
    })
    .await?;
    Ok(())
}

fn log_admin(profile: &Profile, action: &str) {
    tracing::info!(event = "admin.action", "profileId" = %profile.id, action = %action);
}

/// `GET /api/admin/me`: `{ profileId, username, strict, secondFactor }`.
pub async fn get_me(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = admin_caller(app, &req)?;
    let caller = caller_of(&req)?;
    Ok(json(
        200,
        json!({
            "profileId": profile.id,
            "username": profile.username(),
            "strict": app.env.dev_strict,
            "secondFactor": caller.user.second_factor,
        }),
    ))
}

// The caller's own rating (R3074001)

fn glicko_view(glicko: &Glicko) -> Value {
    json!({
        "rating": glicko.rating,
        "deviation": glicko.deviation,
        "volatility": glicko.volatility,
    })
}

/// `body.ladder`, which names a ladder exactly.
fn ladder_in(body: &Value) -> Result<Ladder, ApiError> {
    body.get("ladder")
        .and_then(Value::as_str)
        .and_then(Ladder::from_literal)
        .ok_or_else(|| bad_request("\"ladder\" must be \"constructed\" or \"random\""))
}

/// The caller's profile, read in `tx`.
async fn own_profile(tx: &mut Tx<'_>, profile_id: &str) -> Result<Profile, ApiError> {
    tx.profiles_get_by_id(profile_id)
        .await?
        .ok_or_else(|| ApiError::internal("the caller's profile is gone"))
}

/// `{ ladders }`: the caller's rating on each ladder, Constructed first, whether it is hand-set,
/// and the rating its games had earned while it is.
async fn ratings_body(tx: &mut Tx<'_>, profile_id: &str) -> Result<Value, ApiError> {
    let profile = own_profile(tx, profile_id).await?;
    let held = tx.admin_hand_set_get(profile_id).await?;
    let ladders: Vec<Value> = Ladder::ALL
        .iter()
        .map(|&ladder| {
            let glicko = profile.glicko(ladder);
            let kept = held.iter().find(|row| row.ladder == ladder);
            json!({
                "ladder": ladder,
                "rating": glicko.rating,
                "deviation": glicko.deviation,
                "volatility": glicko.volatility,
                "handSet": kept.is_some(),
                "earned": kept.map(|row| glicko_view(&row.earned)),
            })
        })
        .collect();
    Ok(json!({ "ladders": ladders }))
}

/// `GET /api/admin/rating`: the caller's rating on both ladders.
pub async fn get_rating(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = admin_caller(app, &req)?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let body = ratings_body(&mut tx, &profile.id).await?;
    tx.commit().await?;
    Ok(json(200, body))
}

/// `PUT /api/admin/rating` with `{ ladder, rating }` (R3074001): the caller's rating on the ladder,
/// set by hand, its deviation and volatility kept. The first hand-set rating keeps the one its games
/// had earned, which a reset puts back; each change writes a rating ledger row marked `dev` and an
/// audit row. Answers as `GET /api/admin/rating`.
pub async fn put_rating(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = admin_caller(app, &req)?;
    let ladder = ladder_in(&req.body)?;
    let rating = req
        .body
        .get("rating")
        .and_then(Value::as_f64)
        .filter(|rating| rating.is_finite() && (DEV_RATING_MIN..=DEV_RATING_MAX).contains(rating))
        .ok_or_else(|| {
            bad_request(format!(
                "\"rating\" must be a number from {DEV_RATING_MIN} to {DEV_RATING_MAX}"
            ))
        })?;
    let at = now_ms();
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let before = own_profile(&mut tx, &profile.id).await?.glicko(ladder);
    let after = Glicko { rating, ..before };
    let was_hand_set = !tx
        .admin_hand_set_keep(&HandSetRating {
            profile_id: profile.id.clone(),
            ladder,
            earned: before,
            set_at: at,
        })
        .await?;
    tx.profiles_set_glicko(&profile.id, ladder, &after).await?;
    tx.admin_rating_grants_append(&RatingGrant {
        profile_id: profile.id.clone(),
        ladder,
        change: RatingChange::Set,
        before,
        after,
        dev: true,
        at,
    })
    .await?;
    audit(
        &mut tx,
        &profile.id,
        "PUT /api/admin/rating".to_string(),
        json!({ "ladder": ladder, "rating": rating }),
        json!({ "ladder": ladder, "glicko": glicko_view(&before), "handSet": was_hand_set }),
        json!({ "ladder": ladder, "glicko": glicko_view(&after), "handSet": true }),
        at,
    )
    .await?;
    let body = ratings_body(&mut tx, &profile.id).await?;
    tx.commit().await?;
    log_admin(&profile, "rating.set");
    Ok(json(200, body))
}

/// `POST /api/admin/rating/reset` with `{ ladder }` (R3074001): the ladder's rating put back to the
/// one its games had earned when it was first hand-set, which takes it back onto the public ladder;
/// a ledger row and an audit row. 409 when the ladder is not hand-set, writing nothing.
pub async fn post_reset_rating(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = admin_caller(app, &req)?;
    let ladder = ladder_in(&req.body)?;
    let at = now_ms();
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let Some(held) = tx.admin_hand_set_clear(&profile.id, ladder).await? else {
        return Err(ApiError::new(
            ApiErrorCode::Conflict,
            "That ladder's rating is not hand-set.",
        ));
    };
    let before = own_profile(&mut tx, &profile.id).await?.glicko(ladder);
    let after = held.earned;
    tx.profiles_set_glicko(&profile.id, ladder, &after).await?;
    tx.admin_rating_grants_append(&RatingGrant {
        profile_id: profile.id.clone(),
        ladder,
        change: RatingChange::Reset,
        before,
        after,
        dev: true,
        at,
    })
    .await?;
    audit(
        &mut tx,
        &profile.id,
        "POST /api/admin/rating/reset".to_string(),
        json!({ "ladder": ladder }),
        json!({ "ladder": ladder, "glicko": glicko_view(&before), "handSet": true }),
        json!({ "ladder": ladder, "glicko": glicko_view(&after), "handSet": false }),
        at,
    )
    .await?;
    let body = ratings_body(&mut tx, &profile.id).await?;
    tx.commit().await?;
    log_admin(&profile, "rating.reset");
    Ok(json(200, body))
}

// The whole ladder (R3074002)

/// `GET /api/admin/leaderboard?ladder=` (`constructed`, the default, or `random`): every row of the
/// ladder this season, each marked person or bot.
pub async fn get_leaderboard(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = admin_caller(app, &req)?;
    let ladder = Ladder::parse(req.query.get("ladder").map(String::as_str));
    ok_of(&dev_leaderboard(app, &profile.id, ladder).await?)
}

// News posts (R3074003)

/// Each listed profile's username as shown, read in `tx`.
async fn usernames(tx: &mut Tx<'_>, profile_ids: Vec<String>) -> Result<IndexMap<String, String>, ApiError> {
    Ok(tx
        .profiles_get_many(&profile_ids)
        .await?
        .iter()
        .map(|profile| (profile.id.clone(), profile.username()))
        .collect())
}

/// A post as the dashboard reads it: its author by username, or `""` once the account is gone.
fn post_view(post: &NewsPost, usernames: &IndexMap<String, String>) -> Value {
    json!({
        "id": post.id,
        "title": post.title,
        "summary": post.summary,
        "postedAt": post.posted_at,
        "postedBy": post
            .posted_by
            .as_ref()
            .and_then(|id| usernames.get(id))
            .cloned()
            .unwrap_or_default(),
        "takenDownAt": post.taken_down_at,
    })
}

/// `GET /api/admin/news`: `{ posts }`, the newest `NEWS_POSTS_PAGE` posts, taken down or not, newest
/// first.
pub async fn get_posts(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = admin_caller(app, &req)?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let posts = tx.admin_posts_recent(NEWS_POSTS_PAGE).await?;
    let authors = usernames(
        &mut tx,
        posts.iter().filter_map(|post| post.posted_by.clone()).collect(),
    )
    .await?;
    tx.commit().await?;
    let posts: Vec<Value> = posts.iter().map(|post| post_view(post, &authors)).collect();
    Ok(json(200, json!({ "posts": posts })))
}

/// `POST /api/admin/news` with `{ title, summary }` (R3074003): a post with no reward, its title and
/// summary trimmed and checked as `news.json`'s entries are (`economy::news::post_problems`), live
/// for `NEWS_TTL_DAYS` from now. 201 `{ post }`; 400 naming every problem, writing nothing.
pub async fn post_post(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = admin_caller(app, &req)?;
    let text = |key: &str| {
        req.body
            .get(key)
            .and_then(Value::as_str)
            .map(|value| value.trim().to_string())
            .ok_or_else(|| bad_request(format!("\"{key}\" must be a string")))
    };
    let title = text("title")?;
    let summary = text("summary")?;
    let problems = post_problems(&title, &summary);
    if !problems.is_empty() {
        return Err(bad_request(problems.join(" ")));
    }
    let at = now_ms();
    let hex = uuid::Uuid::new_v4().simple().to_string();
    let post = NewsPost {
        id: format!("{NEWS_POST_ID_PREFIX}{}", &hex[..POST_ID_HEX_DIGITS]),
        title,
        summary,
        posted_at: at,
        posted_by: Some(profile.id.clone()),
        taken_down_at: None,
    };
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    tx.admin_posts_create(&post).await?;
    let authors = usernames(&mut tx, vec![profile.id.clone()]).await?;
    let view = post_view(&post, &authors);
    audit(
        &mut tx,
        &profile.id,
        "POST /api/admin/news".to_string(),
        json!({ "title": post.title, "summary": post.summary }),
        Value::Null,
        view.clone(),
        at,
    )
    .await?;
    tx.commit().await?;
    log_admin(&profile, "news.post");
    Ok(json(201, json!({ "post": view })))
}

/// `DELETE /api/admin/news/:postId` (R3074003): takes the post down, so `GET /api/news` no longer
/// lists it and its claim is refused; the row stays. 404 for no such post, 409 for one down already.
pub async fn delete_post(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = admin_caller(app, &req)?;
    let post_id = req.params.get("postId").cloned().unwrap_or_default();
    let at = now_ms();
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let Some(before) = tx.admin_posts_get(&post_id).await? else {
        return Err(ApiError::new(ApiErrorCode::NotFound, "No post has that id."));
    };
    let Some(after) = tx.admin_posts_take_down(&post_id, at).await? else {
        return Err(ApiError::new(
            ApiErrorCode::Conflict,
            "That post is taken down already.",
        ));
    };
    let authors = usernames(&mut tx, before.posted_by.iter().cloned().collect()).await?;
    let view = post_view(&after, &authors);
    audit(
        &mut tx,
        &profile.id,
        format!("DELETE /api/admin/news/{post_id}"),
        json!({ "postId": post_id }),
        post_view(&before, &authors),
        view.clone(),
        at,
    )
    .await?;
    tx.commit().await?;
    log_admin(&profile, "news.take_down");
    Ok(json(200, json!({ "post": view })))
}

// The audit (R3074000)

/// `GET /api/admin/audit`: `{ actions }`, the newest `ADMIN_AUDIT_PAGE` audit rows, newest first,
/// each with its author's username (`""` once the account is gone).
pub async fn get_audit(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = admin_caller(app, &req)?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let actions = tx.admin_actions_recent(ADMIN_AUDIT_PAGE).await?;
    let mut authors: Vec<String> = actions
        .iter()
        .filter_map(|action| action.profile_id.clone())
        .collect();
    authors.sort();
    authors.dedup();
    let names = usernames(&mut tx, authors).await?;
    tx.commit().await?;
    let actions: Vec<Value> = actions
        .iter()
        .map(|action| {
            json!({
                "id": action.id,
                "profileId": action.profile_id.clone().unwrap_or_default(),
                "username": action
                    .profile_id
                    .as_ref()
                    .and_then(|id| names.get(id))
                    .cloned()
                    .unwrap_or_default(),
                "route": action.route,
                "request": action.request,
                "before": action.before,
                "after": action.after,
                "at": action.at,
            })
        })
        .collect();
    Ok(json(200, json!({ "actions": actions })))
}
