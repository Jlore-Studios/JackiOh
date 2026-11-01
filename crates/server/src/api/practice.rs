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
//! - **A release** (DELETE /api/practice/seeds/:seed) deletes the caller's unclaimed seed under the economy lock, so a game left without saving gives its place back ([[R2379000]]).
//! - **A claim** (`POST /api/practice/claims`) sends the seed, the result and the game's log once
//!   it is over. Its seed is burned first, in a transaction of its own, so it pays once whatever
//!   the check answers ([[R1303001]]); then `economy::practice::verify` folds the log to the
//!   claimed result off the runtime, re-running no AI move ([[R2560000]]), at most
//!   `PRACTICE_VERIFY_CONCURRENCY` claims at once (the rest wait for a permit, and none is
//!   dropped); then a claim that passed pays through the reward hook at practice rates (held to
//!   the day's Jlore cap, [[R1483000]]), moving the account's quests as a
//!   ladder game does ([[R1304001]]), and keeps the board it ended on, in a task of its own that
//!   finishes even when the caller stops waiting.
//!   The pay is one transaction, tried again up to `PRACTICE_PAY_ATTEMPTS` times when the store
//!   fails it ([[R1688000]]); one that never lands answers an error and leaves the seed burned.
//!   No transaction stays open across a check.
//!   Every refusal answers 200 with `paid: false` and why, so the result screen can say it.
//! - **A settle, once** ([[R2422000]]): the burn keeps the claim (its result and log) with the seed,
//!   and the claim settles once. The pay marks the seed `paid` first in its own transaction, only
//!   where it has no mark, so the try that lands pays once whoever runs it; a refusal marks it
//!   `refused`. The server counts the settles running (`App::practice_settles`), and its shutdown
//!   drain waits for them before the store closes (`all_settled`). A claim still unsettled
//!   `PRACTICE_SETTLE_GRACE_SECONDS` after its burn (every try at its pay failed, or the server
//!   stopped or crashed under it) is settled by the retention loop (`sweep_unsettled`): checked
//!   again under the same permits, then paid at the sweep's time, keeping no board, or marked
//!   refused.
//!
//! The issue and the claim answer only while the economy is on ([[R1302006]]): an issue is a 409, a claim
//! `economy_off`. A release answers whatever the switch says: it pays nothing. The page asks for a seed only for a game that can pay ([[R1303000]]).

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::{Value, json};
use tokio::task::JoinHandle;

use jackioh_engine::{Action, Difficulty, LastBoardEntry, PlayerId};

use crate::actor::engine::{self, Pinned};
use crate::api::collection::caller_profile;
use crate::api::http::{
    ApiError, ApiErrorCode, ApiResult, Req, bad_request, json as json_response, log_info, log_warn,
};
use crate::app::{App, now_ms};
use crate::config::{
    PRACTICE_PAY_ATTEMPTS, PRACTICE_PAY_BACKOFF_MS, PRACTICE_SEED_TTL_SECONDS, PRACTICE_SEEDS_OPEN_MAX,
    PRACTICE_SETTLE_GRACE_SECONDS,
};
use crate::db::store::{LastBoardKind, StoreResult};
use crate::economy::practice::{ClaimRefusal, ClaimedResult, Verified, verify};
use crate::economy::quests::tally_seat;
use crate::economy::rewards::{GameEconomy, GameMode, GameSummary, pay_game_rewards};
use crate::economy::store::{PracticeClaim, PracticeDeck, PracticeOutcome, PracticeSeed, PracticeSetup};

/// An issue's status: the seed is a new resource.
const CREATED: u16 = 201;

const MS_PER_SECOND: i64 = 1000;

/// How long a seed waits to be claimed, in milliseconds.
const SEED_TTL_MS: i64 = PRACTICE_SEED_TTL_SECONDS * MS_PER_SECOND;

/// How long a claim waits unsettled before the sweep settles it, in milliseconds (R2422000).
const SETTLE_GRACE_MS: i64 = PRACTICE_SETTLE_GRACE_SECONDS * MS_PER_SECOND;

/// An issue's body.
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
/// seeds (R1483000). It runs in the account's economy turn; past the requests waiting theirs it
/// is 429 `rate_limited` (R2378000).
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
    // R2378000: the account's turn first, so a burst waits here holding no database
    // connection, not on the economy lock below.
    let _turn = app.spends.claim(&profile.id).await?;
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
        claim: None,
        settled: None,
    })
    .await?;
    tx.commit().await?;
    Ok(json_response(
        CREATED,
        json!({ "seed": seed, "lastBoard": last_board, "expiresAt": now + SEED_TTL_MS }),
    ))
}

/// `DELETE /api/practice/seeds/:seed` (`AuthLevel::Active`, [[R2379000]]): the caller's unclaimed seed
/// given back, deleted under the account's economy lock (R1483000's count takes it too), so its
/// place is free for the next issue. 200 `{ released: true }`; a 404, changing nothing, for a seed
/// that is not the caller's, was claimed, or was never issued. It runs in the account's economy
/// turn; past the requests waiting theirs it is 429 `rate_limited` (R2378000).
pub async fn delete_seed(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let seed = req.params.get("seed").cloned().unwrap_or_default();
    // R2378000: the account's turn first, so a burst waits here holding no database
    // connection, not on the economy lock below.
    let _turn = app.spends.claim(&profile.id).await?;
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

/// `POST /api/practice/claims` with `{ seed, result, log }` (`AuthLevel::Active`). 200
/// `{ paid: true, jlore, xp, jloreCapped, tracks, quests }` for a game that passed, the game's own
/// pay, its Jlore short of the game's rate when the day's cap held some back (R1483000), each track
/// it moved and each quest it completed (`economy::rewards::GameEconomy`, #1657); 200
/// `{ paid: false, reason }` for every refusal, `already_claimed` too when another settle of the
/// claim, the retention loop's, settled it first (R2422000); a 500 when every try at the pay fails,
/// the seed burned all the same (R1688000) and its claim kept for the sweep to pay (R2422000).
pub async fn post_claim(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let claim: ClaimRequest = body_of(&req.body, "a practice game's claim")?;
    if !app.economy_live {
        return refused(ClaimRefusal::EconomyOff);
    }

    // The seed, burned before the check, so a claim pays once whatever happens after; the claim
    // kept with it until it settles, so one whose server goes away is settled later (R2422000).
    let now = now_ms();
    let claimed = PracticeClaim {
        result: claim.result,
        log: claim.log,
    };
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
        .economy_practice_seed_claim(&profile.id, &claim.seed, now, &claimed)
        .await?
    {
        return refused(ClaimRefusal::AlreadyClaimed);
    }
    tx.commit().await?;

    // The check and the pay run as a task of their own, so a caller that stops waiting (the page's
    // request timeout behind a long queue, a closed tab) leaves its burned seed to be checked and
    // paid all the same: nothing past the burn is dropped.
    let settled = spawn_settle(app, held, claimed, now, true)
        .await
        .map_err(|error| ApiError::internal(format!("the practice claim's check failed: {error}")))??;
    match settled {
        Settlement::Paid(paid) => Ok(json_response(
            200,
            json!({
                "paid": true,
                "jlore": paid.jlore,
                "xp": paid.xp,
                "jloreCapped": paid.jlore_capped,
                "tracks": paid.tracks,
                "quests": paid.quests,
            }),
        )),
        Settlement::Refused(reason) => refused(reason),
        Settlement::Already => refused(ClaimRefusal::AlreadyClaimed),
    }
}

/// [[R2422000]]: one settle running, counted on `App::practice_settles` from before its task is
/// spawned until the task ends, so the shutdown drain waits for it ([`all_settled`]).
struct Settling(Arc<App>);

impl Settling {
    fn enter(app: &Arc<App>) -> Settling {
        app.practice_settles.send_modify(|running| *running += 1);
        Settling(Arc::clone(app))
    }
}

impl Drop for Settling {
    fn drop(&mut self) {
        self.0
            .practice_settles
            .send_modify(|running| *running = running.saturating_sub(1));
    }
}

/// [[R2422000]]: resolves once no settle is running, the live claims' whose callers have gone
/// included. The shutdown drain awaits it before it closes the store (`app::serve_until`).
pub async fn all_settled(app: &App) {
    let mut running = app.practice_settles.subscribe();
    let _ = running.wait_for(|running| *running == 0).await;
}

/// How a settle of a claim ended.
enum Settlement {
    /// It passed and was paid: the game's own pay.
    Paid(GameEconomy),
    /// Its check refused it, and its seed is marked so.
    Refused(ClaimRefusal),
    /// It passed, but another settle of the claim marked the seed first, so it paid nothing.
    Already,
}

/// [`settle`] as a task of its own, counted ([`Settling`]) from before it is spawned: a caller that
/// stops waiting leaves it to finish, and a shutdown waits for it.
fn spawn_settle(
    app: &Arc<App>,
    held: PracticeSeed,
    claim: PracticeClaim,
    at: i64,
    live: bool,
) -> JoinHandle<Result<Settlement, ApiError>> {
    let settling = Settling::enter(app);
    let app = Arc::clone(app);
    tokio::spawn(async move {
        let _settling = settling;
        settle(&app, held, claim, at, live).await
    })
}

/// A burned seed's claim checked, then marked refused, or paid at `at` and, for a `live` claim,
/// its board kept ([[R1303003]]). A swept claim keeps no board: the account may have played a
/// newer game since, whose board it would replace ([[R2422000]]).
async fn settle(
    app: &Arc<App>,
    held: PracticeSeed,
    claim: PracticeClaim,
    at: i64,
    live: bool,
) -> Result<Settlement, ApiError> {
    let (seed, profile_id) = (held.seed.clone(), held.profile_id.clone());
    let verified = match check(app, held, claim.result, claim.log).await? {
        Ok(verified) => verified,
        Err(reason) => {
            mark_refused(app, &profile_id, &seed).await;
            return Ok(Settlement::Refused(reason));
        }
    };

    let summary = GameSummary {
        mode: GameMode::Practice,
        profile_id,
        player: verified.player,
        won: verified.won,
        actions: verified.actions,
        match_id: format!("practice:{seed}"),
        at,
        bot: false,
        tally: tally_seat(verified.seat.as_ref(), verified.attacks, &app.catalog.defs),
    };
    let board = live.then_some(verified.last_board.as_slice());
    Ok(match pay_with_retries(app, &seed, &summary, board).await? {
        Some(paid) => Settlement::Paid(paid),
        None => Settlement::Already,
    })
}

/// [[R2422000]]: a refused claim's seed marked `refused`, in a transaction of its own. A store that
/// fails it is logged and nothing more: the claim stays unsettled, and the sweep checks it again and
/// refuses it then.
async fn mark_refused(app: &App, profile_id: &str, seed: &str) {
    let marked: StoreResult<()> = async {
        let mut tx = app.db.begin(Some(profile_id)).await?;
        tx.economy_practice_seed_settle(profile_id, seed, PracticeOutcome::Refused, now_ms())
            .await?;
        tx.commit().await
    }
    .await;
    if let Err(error) = marked {
        log_warn(
            "practice.claim.markFailed",
            json!({ "seed": seed, "message": error.to_string() }),
        );
    }
}

/// One try at a passed claim's pay ([[R1688000]]): the seed marked `paid` ([[R2422000]]), its
/// rewards and, when `board` is given, its board, in one transaction, which a failure rolls back
/// whole, the mark with it, so the try after it pays once. `None`, paying nothing, when the seed
/// was settled already: another settle of the claim paid or refused it first.
async fn pay(
    app: &Arc<App>,
    seed: &str,
    summary: &GameSummary,
    board: Option<&[LastBoardEntry]>,
) -> StoreResult<Option<GameEconomy>> {
    let mut tx = app.db.begin(Some(&summary.profile_id)).await?;
    // R2422000: the mark first, and only where there is none, so a pay lands once whoever runs it.
    if !tx
        .economy_practice_seed_settle(&summary.profile_id, seed, PracticeOutcome::Paid, now_ms())
        .await?
    {
        return Ok(None);
    }
    let paid = pay_game_rewards(&mut tx, summary).await?;
    if let Some(board) = board {
        tx.last_boards_put(&summary.profile_id, LastBoardKind::Practice, board, summary.at)
            .await?;
    }
    tx.commit().await?;
    Ok(Some(paid))
}

/// [[R1688000]]: [`pay`], tried up to `PRACTICE_PAY_ATTEMPTS` times, the first wait
/// `PRACTICE_PAY_BACKOFF_MS` and each next one twice the last, as the match actor tries a result
/// ([[R1437]]). Answers the last try's error when none lands; the seed stays burned, its claim kept
/// for the sweep ([[R2422000]]).
async fn pay_with_retries(
    app: &Arc<App>,
    seed: &str,
    summary: &GameSummary,
    board: Option<&[LastBoardEntry]>,
) -> StoreResult<Option<GameEconomy>> {
    let mut wait_ms = PRACTICE_PAY_BACKOFF_MS;
    let mut attempt = 1;
    loop {
        let error = match pay(app, seed, summary, board).await {
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

/// What one sweep of the unsettled claims settled ([[R2422000]]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Swept {
    pub paid: usize,
    pub refused: usize,
}

/// [[R2422000]]: settles every claim still unsettled `PRACTICE_SETTLE_GRACE_SECONDS` after it
/// burned its seed, the oldest first and one at a time, each as a counted task of its own: checked
/// again under the verify permits, then paid at the sweep's time with no board, or marked refused.
/// A claim another settle marks first pays nothing here. Nothing while the economy is off
/// ([[R1302006]]): the claims wait for it. A settle that fails is logged and its claim left for
/// the next sweep. The retention loop runs it (`api::retention::run_purge`).
pub async fn sweep_unsettled(app: &Arc<App>) -> StoreResult<Swept> {
    let mut swept = Swept::default();
    if !app.economy_live {
        return Ok(swept);
    }
    let mut tx = app.db.begin(None).await?;
    let unsettled = tx
        .economy_practice_seeds_unsettled(now_ms() - SETTLE_GRACE_MS)
        .await?;
    tx.commit().await?;
    for mut held in unsettled {
        let Some(claim) = held.claim.take() else {
            continue;
        };
        let seed = held.seed.clone();
        let message = match spawn_settle(app, held, claim, now_ms(), false).await {
            Ok(Ok(Settlement::Paid(_))) => {
                swept.paid += 1;
                continue;
            }
            Ok(Ok(Settlement::Refused(_))) => {
                swept.refused += 1;
                continue;
            }
            Ok(Ok(Settlement::Already)) => continue,
            Ok(Err(error)) => error.to_string(),
            Err(error) => error.to_string(),
        };
        log_warn(
            "practice.sweep.settleFailed",
            json!({ "seed": seed, "message": message }),
        );
    }
    Ok(swept)
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
