//! Practice earning (SPEC §14.8, §9.9, #1303): verified practice games pay half rates.
//!
//! - **Issue** (`POST /api/practice/seeds`): mints a seed with setup and last board (R1303003);
//!   prunes expired seeds (R1303001); enforces `PRACTICE_SEEDS_OPEN_MAX` under economy lock (R1483000).
//! - **Release** (`DELETE /api/practice/seeds/:seed`): deletes unclaimed seed under economy lock (R2379000).
//! - **Claim** (`POST /api/practice/claims`): burns seed first (R1303001), verifies log off-runtime,
//!   and pays rewards capped at day's Jlore (R1483000) and advances quests (R1304001), retrying pay
//!   up to `PRACTICE_PAY_ATTEMPTS` times (R1688000).
//!
//! Issue and claim require live economy (R1302006, R1303000).

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::{Value, json};

use jackioh_engine::{Action, Difficulty, LastBoardEntry, PlayerId};

use crate::actor::engine::{self, Pinned};
use crate::api::collection::caller_profile;
use crate::api::http::{
    ApiError, ApiErrorCode, ApiResult, Req, bad_request, json as json_response, log_info, log_warn,
};
use crate::app::{App, now_ms};
use crate::config::{
    PRACTICE_PAY_ATTEMPTS, PRACTICE_PAY_BACKOFF_MS, PRACTICE_SEED_TTL_SECONDS, PRACTICE_SEEDS_OPEN_MAX,
};
use crate::db::store::{LastBoardKind, StoreResult};
use crate::economy::practice::{ClaimRefusal, ClaimedResult, Verified, verify};
use crate::economy::quests::tally_seat;
use crate::economy::rewards::{GameEconomy, GameMode, GameSummary, pay_game_rewards};
use crate::economy::store::{PracticeDeck, PracticeSeed, PracticeSetup};

/// An issue's status: the seed is a new resource.
const CREATED: u16 = 201;

const MS_PER_SECOND: i64 = 1000;

/// How long a seed waits to be claimed, in milliseconds.
const SEED_TTL_MS: i64 = PRACTICE_SEED_TTL_SECONDS * MS_PER_SECOND;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeedRequest {
    difficulty: Difficulty,
    human_seat: PlayerId,
    deck: PracticeDeck,
    catalog: String,
}

#[derive(Deserialize)]
struct ClaimRequest {
    seed: String,
    result: ClaimedResult,
    log: Vec<Action>,
}

/// A body read as `T`: a field missing or of the wrong shape is a 400 naming it.
fn body_of<T: for<'de> Deserialize<'de>>(body: &Value, what: &str) -> Result<T, ApiError> {
    serde_json::from_value(body.clone()).map_err(|error| bad_request(format!("that is not {what}: {error}")))
}

fn refused(reason: ClaimRefusal) -> ApiResult {
    Ok(json_response(200, json!({ "paid": false, "reason": reason })))
}

/// `POST /api/practice/seeds` with `{ difficulty, humanSeat, deck, catalog }` (`AuthLevel::Active`).
/// 201 `{ seed, lastBoard, expiresAt }`; a 409 while the economy is off, for a catalog version
/// this build cannot fold (R767000), or while the account holds `PRACTICE_SEEDS_OPEN_MAX` unclaimed
/// seeds (R1483000).
pub async fn post_seed(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let request: SeedRequest = body_of(&req.body, "a practice game's setup")?;
    if !app.economy_live {
        return Err(ApiError::new(
            ApiErrorCode::Conflict,
            "Practice games earn nothing yet.",
        ));
    }
    if Pinned::of(&request.catalog).is_err() {
        return Err(ApiError::new(
            ApiErrorCode::Conflict,
            "This game's cards are from a version the server cannot check; reload the page.",
        ));
    }
    let now = now_ms();
    let seed = crate::api::queue::new_seed();
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    // R1483000: the account's lock before the count, so two requests at once see each other's seed.
    tx.economy_lock_account(&profile.id).await?;
    tx.economy_practice_seeds_prune(&profile.id, now - SEED_TTL_MS)
        .await?;
    if tx
        .economy_practice_seeds_open(&profile.id, now - SEED_TTL_MS)
        .await?
        >= PRACTICE_SEEDS_OPEN_MAX
    {
        return Err(ApiError::new(
            ApiErrorCode::Conflict,
            "Claim one of your open practice games before starting another.",
        ));
    }
    let last_board = tx
        .last_boards_get(&profile.id, LastBoardKind::Practice)
        .await?
        .unwrap_or_default();
    tx.economy_practice_seed_insert(&PracticeSeed {
        seed: seed.clone(),
        profile_id: profile.id.clone(),
        setup: PracticeSetup {
            difficulty: request.difficulty,
            human_seat: request.human_seat,
            deck: request.deck,
            last_board: last_board.clone(),
            catalog_version: request.catalog,
        },
        issued_at: now,
        claimed_at: None,
    })
    .await?;
    tx.commit().await?;
    Ok(json_response(
        CREATED,
        json!({ "seed": seed, "lastBoard": last_board, "expiresAt": now + SEED_TTL_MS }),
    ))
}

/// `DELETE /api/practice/seeds/:seed` (`AuthLevel::Active`, [[R2379000]]).
/// Releases caller's unclaimed seed under the account's economy lock (R1483000).
pub async fn delete_seed(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let seed = req.params.get("seed").cloned().unwrap_or_default();
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    tx.economy_lock_account(&profile.id).await?;
    if !tx.economy_practice_seed_release(&profile.id, &seed).await? {
        return Err(ApiError::new(
            ApiErrorCode::NotFound,
            "There is no such open practice game.",
        ));
    }
    tx.commit().await?;
    Ok(json_response(200, json!({ "released": true })))
}

/// `POST /api/practice/claims` with `{ seed, result, log }` (`AuthLevel::Active`, #1657).
/// Verifies game log and pays rewards held to daily cap (R1483000). Retries pay on failure (R1688000).
pub async fn post_claim(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let claim: ClaimRequest = body_of(&req.body, "a practice game's claim")?;
    if !app.economy_live {
        return refused(ClaimRefusal::EconomyOff);
    }

    // The seed, burned before the check, so a claim pays once whatever happens after.
    let now = now_ms();
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let Some(held) = tx.economy_practice_seed_get(&profile.id, &claim.seed).await? else {
        return refused(ClaimRefusal::UnknownSeed);
    };
    if held.claimed_at.is_some() {
        return refused(ClaimRefusal::AlreadyClaimed);
    }
    if now - held.issued_at > SEED_TTL_MS {
        return refused(ClaimRefusal::Expired);
    }
    if !tx
        .economy_practice_seed_claim(&profile.id, &claim.seed, now)
        .await?
    {
        return refused(ClaimRefusal::AlreadyClaimed);
    }
    tx.commit().await?;

    // Verification and payout run detached so client disconnects do not drop claims.
    let settled = tokio::spawn(settle(
        app.clone(),
        profile.id.clone(),
        held,
        claim.result,
        claim.log,
        now,
    ));
    settled
        .await
        .map_err(|error| ApiError::internal(format!("the practice claim's check failed: {error}")))?
}

/// A burned seed's claim checked, then, when it passed, paid and its board kept.
async fn settle(
    app: Arc<App>,
    profile_id: String,
    held: PracticeSeed,
    claimed: ClaimedResult,
    log: Vec<Action>,
    now: i64,
) -> ApiResult {
    let match_id = format!("practice:{}", held.seed);
    let verified = match check(&app, held, claimed, log).await? {
        Ok(verified) => verified,
        Err(reason) => return refused(reason),
    };

    let summary = GameSummary {
        mode: GameMode::Practice,
        profile_id,
        player: verified.player,
        won: verified.won,
        actions: verified.actions,
        match_id,
        at: now,
        bot: false,
        tally: tally_seat(verified.seat.as_ref(), verified.attacks, &app.catalog.defs),
    };
    let paid = pay_with_retries(&app, &summary, &verified.last_board).await?;
    Ok(json_response(
        200,
        json!({
            "paid": true,
            "jlore": paid.jlore,
            "xp": paid.xp,
            "jloreCapped": paid.jlore_capped,
            "tracks": paid.tracks,
            "quests": paid.quests,
        }),
    ))
}

/// One try at a passed claim's pay ([[R1688000]]): its rewards and its board in one transaction,
/// which a failure rolls back whole, so the try after it pays once.
async fn pay(
    app: &Arc<App>,
    summary: &GameSummary,
    last_board: &[LastBoardEntry],
) -> StoreResult<GameEconomy> {
    let mut tx = app.db.begin(Some(&summary.profile_id)).await?;
    let paid = pay_game_rewards(&mut tx, summary).await?;
    tx.last_boards_put(
        &summary.profile_id,
        LastBoardKind::Practice,
        last_board,
        summary.at,
    )
    .await?;
    tx.commit().await?;
    Ok(paid)
}

/// Retries [`pay`] up to `PRACTICE_PAY_ATTEMPTS` with exponential backoff ([[R1688000]], [[R1437]]).
async fn pay_with_retries(
    app: &Arc<App>,
    summary: &GameSummary,
    last_board: &[LastBoardEntry],
) -> StoreResult<GameEconomy> {
    let mut wait_ms = PRACTICE_PAY_BACKOFF_MS;
    let mut attempt = 1;
    loop {
        let error = match pay(app, summary, last_board).await {
            Ok(paid) => return Ok(paid),
            Err(error) => error,
        };
        log_warn(
            "practice.claim.payFailed",
            json!({ "matchId": summary.match_id, "attempt": attempt, "message": error.to_string() }),
        );
        if attempt >= PRACTICE_PAY_ATTEMPTS {
            return Err(error);
        }
        tokio::time::sleep(Duration::from_millis(u64::try_from(wait_ms).unwrap_or(0))).await;
        wait_ms *= 2;
        attempt += 1;
    }
}

/// `verify` on the blocking pool under a permit, timed and logged. A panic (a setup the engine
/// cannot create) and a catalog this build no longer folds are `DoesNotFold`.
async fn check(
    app: &Arc<App>,
    held: PracticeSeed,
    claimed: ClaimedResult,
    log: Vec<Action>,
) -> Result<Result<Verified, ClaimRefusal>, ApiError> {
    let _permit = app
        .practice_verifications
        .acquire()
        .await
        .map_err(|_| ApiError::internal("the practice verifications' permits are closed"))?;
    let started = Instant::now();
    let seed = held.seed.clone();
    let ran = engine::off_runtime(app.engine_thread, move || {
        let pinned = Pinned::of(&held.setup.catalog_version).map_err(|_| ClaimRefusal::DoesNotFold)?;
        verify(pinned, &held.seed, &held.setup, claimed, &log)
    })
    .await;
    let ms = started.elapsed().as_millis();
    let checked = ran.unwrap_or_else(|panic| {
        log_warn("practice.claim.panicked", json!({ "seed": seed, "panic": panic }));
        Err(ClaimRefusal::DoesNotFold)
    });
    log_info(
        "practice.claim.verified",
        json!({
            "seed": seed,
            "ms": ms,
            "actions": checked.as_ref().map(|verified| verified.actions).ok(),
            "aiMoves": checked.as_ref().map(|verified| verified.ai_moves).ok(),
            "outcome": checked.as_ref().err(),
        }),
    );
    Ok(checked)
}
