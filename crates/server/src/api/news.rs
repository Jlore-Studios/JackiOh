//! The news an account may claim a reward from (SPEC §14.13, #2875). `GET /api/news` answers the live
//! entries, newest first, each marked claimed or not for the caller, and nobody else's claims (§14);
//! `POST /api/news/:newsId/claim` claims one entry's reward, once, while the economy is live
//! (R2875000). The entries, their window and the claim are `economy::news`'s.

use std::sync::Arc;

use crate::api::collection::caller_profile;
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, json};
use crate::app::{App, now_ms};
use crate::economy::idempotency::RequestKey;
use crate::economy::news::{board, claim_news_once, posts_live_from};

/// The claim's route, as its `Idempotency-Key` is kept under (R2124000).
const CLAIM_ROUTE: &str = "/api/news/:newsId/claim";

/// `GET /api/news` (`AuthLevel::Active`): `{ news }`, the entries live now ([[R2875001]]) and the
/// dev dashboard's posts standing now ([[R3074003]]), each marked `claimed` ([[R2875002]]). A read:
/// no turn, no lock.
pub async fn get_news(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let now = now_ms();
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let claimed = tx.economy_news_claims_get(&profile.id).await?;
    let posts = tx.admin_posts_standing(posts_live_from(now), now).await?;
    tx.commit().await?;
    Ok(json(200, board(&app.news, &posts, &claimed, now)))
}

/// `POST /api/news/:newsId/claim` (`AuthLevel::Active`): [`claim_news_once`] ([[R2875000]]), with an
/// optional `Idempotency-Key` whose hash covers the entry (R2124000), in the account's spend turn
/// (R2378000); answers as `GET /api/news`. 409 while the economy is off, writing nothing.
pub async fn post_claim_news(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    if !app.economy_live {
        return Err(ApiError::new(
            ApiErrorCode::Conflict,
            "News rewards can't be claimed yet.",
        ));
    }
    let news_id = req
        .params
        .get("newsId")
        .cloned()
        .ok_or_else(|| ApiError::new(ApiErrorCode::NotFound, "That news isn't posted."))?;
    let request = RequestKey::of_target(app, &req, CLAIM_ROUTE, &news_id)?;
    let _turn = app.spends.claim(&profile.id).await?;
    claim_news_once(app, &profile.id, &news_id, request.as_ref())
        .await?
        .answer()
}
