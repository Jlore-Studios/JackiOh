//! Practice earning (SPEC §14.8, §9.9, #1303): a practice game pays half rates once the server has
//! checked it, on a seed the server issued for it.
//!
//! - **An issue** (`POST /api/practice/seeds`) mints a seed and keeps it with the profile and the
//!   game's setup: the AI tier, the human's seat, the deck and the catalog version, which the page
//!   sends, and the human's last board, which the server keeps ([[R1303003]]) and answers with the
//!   seed for the game to begin with. The same transaction prunes the profile's seeds issued more
//!   than `PRACTICE_SEED_TTL_SECONDS` ago, so an abandoned game's seed goes ([[R1303001]]), and
//!   refuses the request, issuing nothing, while the account already holds
//!   `PRACTICE_SEEDS_OPEN_MAX` unclaimed seeds ([[R1483000]]), counted under the account's economy
//!   lock so requests made at once cannot pass it together.
//! - **A claim** (`POST /api/practice/claims`) sends the seed, the result and the game's log once
//!   it is over. Its seed is burned first, in a transaction of its own, so it pays once whatever
//!   the check answers ([[R1303001]]); then `economy::practice::verify` folds the log and re-runs
//!   the AI off the runtime, at most `PRACTICE_VERIFY_CONCURRENCY` claims at once (the rest wait
//!   for a permit, and none is dropped); then a claim that passed pays through the reward hook at
//!   practice rates (held to the day's Jlore cap, [[R1483000]]), moving the account's quests as a
//!   ladder game does ([[R1304001]]), and keeps the board it ended on, in a task of its own that
//!   finishes even when the caller stops waiting.
//!   No transaction stays open across a check.
//!   Every refusal answers 200 with `paid: false` and why, so the result screen can say it.
//!
//! Both answer only while the economy is on ([[R1302006]]): an issue is a 409, a claim
//! `economy_off`. The page asks for a seed only for a game that can pay ([[R1303000]]).

use std::sync::Arc;
use std::time::Instant;

use serde::Deserialize;
use serde_json::{Value, json};

use jackioh_engine::{Action, Difficulty, PlayerId};

use crate::actor::engine::{self, Pinned};
use crate::api::collection::caller_profile;
use crate::api::http::{
    ApiError, ApiErrorCode, ApiResult, Req, bad_request, json as json_response, log_info, log_warn,
};
use crate::app::{App, now_ms};
use crate::config::{PRACTICE_SEED_TTL_SECONDS, PRACTICE_SEEDS_OPEN_MAX};
use crate::db::store::LastBoardKind;
use crate::economy::practice::{ClaimRefusal, ClaimedResult, Verified, verify};
use crate::economy::quests::tally_seat;
use crate::economy::rewards::{GameMode, GameSummary, pay_game_rewards};
use crate::economy::store::{PracticeDeck, PracticeSeed, PracticeSetup};

/// An issue's status: the seed is a new resource.
const CREATED: u16 = 201;

/// Milliseconds in a second.
const MS_PER_SECOND: i64 = 1000;

/// How long a seed waits to be claimed, in milliseconds.
const SEED_TTL_MS: i64 = PRACTICE_SEED_TTL_SECONDS * MS_PER_SECOND;

/// An issue's body.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeedRequest {
    difficulty: Difficulty,
    human_seat: PlayerId,
    deck: PracticeDeck,
    catalog: String,
}

/// A claim's body.
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

/// `POST /api/practice/claims` with `{ seed, result, log }` (`AuthLevel::Active`). 200
/// `{ paid: true, jlore, xp, jloreCapped, tracks, quests }` for a game that passed, the game's own
/// pay, its Jlore short of the game's rate when the day's cap held some back (R1483000), each track
/// it moved and each quest it completed (`economy::rewards::GameEconomy`, #1657); 200
/// `{ paid: false, reason }` for every refusal.
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

    // The check and the pay run as a task of their own, so a caller that stops waiting (the page's
    // request timeout behind a long queue, a closed tab) leaves its burned seed to be checked and
    // paid all the same: nothing past the burn is dropped.
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

    let won = verified.won;
    let mut tx = app.db.begin(Some(&profile_id)).await?;
    let paid = pay_game_rewards(
        &mut tx,
        &GameSummary {
            mode: GameMode::Practice,
            profile_id: profile_id.clone(),
            player: verified.player,
            won,
            actions: verified.actions,
            match_id,
            at: now,
            bot: false,
            tally: tally_seat(verified.seat.as_ref(), verified.attacks, &app.catalog.defs),
        },
    )
    .await?;
    tx.last_boards_put(&profile_id, LastBoardKind::Practice, &verified.last_board, now)
        .await?;
    tx.commit().await?;
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
