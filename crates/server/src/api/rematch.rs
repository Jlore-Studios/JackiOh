//! Rematch offers after a finished non-series match (SPEC §9.5, R672). Port of
//! `apps/server/src/api/rematch.ts`.
//!
//! After the death screen lands, either seat may offer a rematch — a normal one or a
//! double-or-nothing — and a new match is created only when both seats offer equal stakes.
//! "The opponent is still here" is their match socket being open (`Registry::presence_of`), so the
//! buttons disappear when they leave or log out. Games of a Conquest series offer nothing: the
//! series' continue flow owns what comes next.
//!
//! Two endpoints (TS's `createRematchRoutes()`; the route table is `app.rs`'s, SURFACE §11.2):
//!  - `POST /api/matches/:matchId/rematch { stakes }` (`offer_rematch`, active) upserts the caller's
//!    offer and, when the seats' stakes match, creates the game and answers its id;
//!  - `GET /api/matches/:matchId/rematch` (`rematch_status`, active) answers what each seat offered,
//!    whether the opponent is here, and the created game, if any.
//!
//! The response carries only offer stakes, presence booleans and ids — never decks, hands or
//! ratings (CLAUDE.md rule 7).
//!
//! The offers live in a module-level map, like `queue.rs`'s `E2E_SEED_BY_TICKET`: they are
//! rendezvous state for two sockets, not rows — a restart drops them, and both clients re-offer.
//! Entries without a created game are swept on read past `REMATCH_OFFER_TTL_MS`; an entry that made
//! a game keeps its id so a late poller still learns where to go.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};

use indexmap::IndexMap;
use jackioh_engine::PlayerId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, json};
use crate::app::App;
use crate::config::REMATCH_OFFER_TTL_MS;
use crate::db::store::{MatchRow, MatchSeat, MatchStatus, Profile, QueueMode, StartMatchInput};

/// R672: a normal rematch moves the rating once; a double-or-nothing moves it twice.
pub const STAKE_NORMAL: RematchStakes = 1;
/// R672: a double-or-nothing rematch. Ranked matches only (`double_requires_ranked`).
pub const STAKE_DOUBLE: RematchStakes = 2;

/// TS `1 | 2`: `STAKE_NORMAL` or `STAKE_DOUBLE`, nothing else.
pub type RematchStakes = i32;

/// `POST /api/matches/:matchId/rematch`: the created game, or null while the seats disagree.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RematchOfferBody {
    pub match_id: Option<String>,
}

/// `GET /api/matches/:matchId/rematch`: both offers, the opponent's presence, the created game.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RematchStatusBody {
    pub you_offered: Option<RematchStakes>,
    pub opponent_offer: Option<RematchStakes>,
    pub opponent_here: bool,
    pub match_id: Option<String>,
}

struct OfferEntry {
    /// Seat -> the stakes it offered.
    offers: IndexMap<PlayerId, RematchStakes>,
    /// `now` of the latest offer; the sweep reads this.
    at: i64,
    /// The game equal stakes made, once made.
    match_id: Option<String>,
    /// Which entry this is: TS compared the entry object itself (`offersByMatch.get(id) === entry`)
    /// after a failed create, and a swept-and-remade entry is another object.
    generation: u64,
}

static OFFERS_BY_MATCH: LazyLock<Mutex<IndexMap<String, OfferEntry>>> =
    LazyLock::new(|| Mutex::new(IndexMap::new()));

static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);

fn offers() -> MutexGuard<'static, IndexMap<String, OfferEntry>> {
    OFFERS_BY_MATCH.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// For the tests; nothing in `src/` calls it.
pub fn clear_rematch_offers() {
    offers().clear();
}

/// For the tests; nothing in `src/` calls it.
pub fn rematch_offer_count() -> usize {
    offers().len()
}

// ---------------------------------------------------------------------------
// Small private copies (fullsend rule 5): the clock, the id minters and the error shapes
// ---------------------------------------------------------------------------

/// TS `deps.timers.now()`: epoch milliseconds, on tokio's clock, so a test that pauses and advances
/// time (`tokio::time::pause`, `advance`) moves it as TS's manual timers moved theirs. Unpaused, it
/// is the system clock.
fn now_ms() -> i64 {
    let system = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or(0);
    let tokio_now = tokio::time::Instant::now().into_std();
    let std_now = std::time::Instant::now();
    let ahead = tokio_now.saturating_duration_since(std_now).as_millis() as i64;
    let behind = std_now.saturating_duration_since(tokio_now).as_millis() as i64;
    system + ahead - behind
}

/// TS `deps.ids.uuid()` (`systemIds.uuid`, `crypto.randomUUID()`).
fn new_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// TS `deps.ids.seed()` (`systemIds.seed`): 16 random bytes as lower-case hex.
fn new_seed() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("the system's random source failed");
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn api_error(code: ApiErrorCode, message: impl Into<String>) -> ApiError {
    ApiError { code, message: message.into(), details: None, retry_after_ms: None }
}

/// An error that was not an `ApiError` in TS (a store failure, a thrown `Error`).
fn internal(error: impl std::fmt::Display) -> ApiError {
    api_error(ApiErrorCode::Internal, error.to_string())
}

/// At a handler's edge, what TS's router did with a thrown non-`ApiError`: log it as
/// `handler.threw` and answer 500 `internal` "something went wrong", never the sentence itself.
fn hide_internal(error: ApiError) -> ApiError {
    if !matches!(error.code, ApiErrorCode::Internal) {
        return error;
    }
    tracing::warn!(event = "handler.threw", message = %error.message);
    api_error(ApiErrorCode::Internal, "something went wrong")
}

/// TS `callerProfile(req)` (`collection.ts`): the signed-in profile a non-`none` route carries.
fn caller_profile(req: &Req) -> Result<Profile, ApiError> {
    match &req.caller {
        Some(caller) => Ok(caller.profile.clone()),
        None => Err(api_error(ApiErrorCode::BadRequest, "this endpoint needs a signed-in profile")),
    }
}

fn no_such_match() -> ApiError {
    api_error(ApiErrorCode::NotFound, "no such match")
}

fn series_game() -> ApiError {
    api_error(ApiErrorCode::SeriesGame, "series games continue from the series screen")
}

// ---------------------------------------------------------------------------
// Offers
// ---------------------------------------------------------------------------

fn stakes_of(body: &Value) -> Result<RematchStakes, ApiError> {
    let stakes = body.get("stakes").and_then(Value::as_f64);
    if stakes == Some(STAKE_NORMAL as f64) {
        return Ok(STAKE_NORMAL);
    }
    if stakes == Some(STAKE_DOUBLE as f64) {
        return Ok(STAKE_DOUBLE);
    }
    Err(api_error(ApiErrorCode::BadRequest, "\"stakes\" must be 1 (rematch) or 2 (double-or-nothing)"))
}

/// The live entry for a match, sweeping one whose offers went stale. An entry that already made a
/// game is never swept: the id is how a late poller finds the game.
fn live_entry<'a>(
    offers: &'a mut IndexMap<String, OfferEntry>,
    match_id: &str,
    now: i64,
) -> Option<&'a mut OfferEntry> {
    let stale = match offers.get(match_id) {
        None => return None,
        Some(entry) => entry.match_id.is_none() && now - entry.at > REMATCH_OFFER_TTL_MS as i64,
    };
    if stale {
        offers.shift_remove(match_id);
        return None;
    }
    offers.get_mut(match_id)
}

fn seat_of(match_row: &MatchRow, profile_id: &str) -> Option<PlayerId> {
    if match_row.players.0 == profile_id {
        return Some(PlayerId::P1);
    }
    if match_row.players.1 == profile_id {
        return Some(PlayerId::P2);
    }
    None
}

fn other(seat: PlayerId) -> PlayerId {
    if seat == PlayerId::P1 { PlayerId::P2 } else { PlayerId::P1 }
}

/// Creates the rematch both seats offered, exactly like `queue.rs`'s `start_paired_match` makes a
/// paired match: a Best-of-1 replays the finished row's frozen decks and portraits under a fresh
/// seed, an All Random deals fresh decks from that seed (`{seed}:p1-deck`, `{seed}:p2-deck`, the
/// same suffix scheme), and both profiles go in-match in one transaction first.
async fn create_rematch(
    app: &Arc<App>,
    finished: &MatchRow,
    mode: QueueMode,
    stakes: RematchStakes,
    match_id: &str,
) -> Result<(), ApiError> {
    let seed = new_seed();
    let random = mode == QueueMode::Random;
    let portrait_of = |at: usize| -> Option<String> {
        finished.portraits.as_ref().map(|portraits| (if at == 0 { &portraits.0 } else { &portraits.1 }).to_string())
    };
    let dealt_portrait = |seat: &str| -> Option<String> {
        Some(jackioh_engine::wire::pick_portrait_from_seed(&format!("{seed}:portrait:{seat}")).to_string())
    };
    let seats = (
        MatchSeat {
            profile_id: finished.players.0.clone(),
            player: PlayerId::P1,
            deck: if random {
                crate::actor::engine::deal_random_deck(&format!("{seed}:p1-deck"))
            } else {
                finished.decks.0.clone()
            },
            portrait: if random { dealt_portrait("p1") } else { portrait_of(0) },
        },
        MatchSeat {
            profile_id: finished.players.1.clone(),
            player: PlayerId::P2,
            deck: if random {
                crate::actor::engine::deal_random_deck(&format!("{seed}:p2-deck"))
            } else {
                finished.decks.1.clone()
            },
            portrait: if random { dealt_portrait("p2") } else { portrait_of(1) },
        },
    );

    // Refuse before minting anything: one of them found another game while the offers were coming
    // in, so the rematch loses and neither seat is stolen out of the game it is actually in.
    {
        let mut tx = app.db.begin(None).await.map_err(internal)?;
        for seat in [&seats.0, &seats.1] {
            let found = tx.profiles_get_many(&[seat.profile_id.clone()]).await.map_err(internal)?;
            if found.first().is_some_and(|profile| profile.in_match_id.is_some()) {
                return Err(api_error(ApiErrorCode::AlreadyInMatch, "finish your current match first"));
            }
        }
        tx.commit().await.map_err(internal)?;
    }

    // The start writes the row (`registry.rs`); a failed start leaves nothing behind, so there is
    // nothing to undo — the caller clears the offer claim and the seats simply offer again.
    app.matches
        .start(
            app,
            StartMatchInput {
                match_id: match_id.to_string(),
                seed: seed.clone(),
                catalog_version: finished.catalog_version.clone(),
                // The rematch is ranked exactly when the finished match was (§9.5).
                ranked: finished.ranked.unwrap_or(false),
                mode: Some(mode),
                // Absent reads as 1 downstream; only a double writes its stakes, and only ranked games
                // reach here with 2 (`double_requires_ranked` above).
                stake: if stakes == STAKE_DOUBLE { Some(stakes) } else { None },
                seats: seats.clone(),
            },
        )
        .await?;

    // After the start, not before (`series.rs`'s `mark_in_match`): the row is what the in-match
    // reference points at (`profiles_current_match_id_fkey`), so flagging first is a foreign-key
    // failure on Postgres. A seat taken during the start keeps its game and its tickets; like the
    // series' post-start flags, a failure here is logged, not thrown — the game exists either way.
    let flagged = async {
        let mut tx = app.db.begin(None).await.map_err(internal)?;
        for seat in [&seats.0, &seats.1] {
            let found = tx.profiles_get_many(&[seat.profile_id.clone()]).await.map_err(internal)?;
            let taken = found
                .first()
                .is_some_and(|profile| profile.in_match_id.as_deref().is_some_and(|current| current != match_id));
            if taken {
                tracing::info!(event = "rematch.seat_taken", matchId = %match_id, profileId = %seat.profile_id);
                continue;
            }
            tx.profiles_set_in_match(&seat.profile_id, Some(match_id)).await.map_err(internal)?;
            // A stray open ticket would block the re-queue M7-T1 promises after this game ends, so
            // the ending is not the only place that clears one (`results.rs`).
            if let Some(ticket) = tx.tickets_open_for_profile(&seat.profile_id).await.map_err(internal)? {
                tx.tickets_cancel(&ticket.id, now_ms()).await.map_err(internal)?;
            }
        }
        tx.commit().await.map_err(internal)
    }
    .await;
    if let Err(error) = flagged {
        tracing::error!(event = "rematch.in_match_failed", matchId = %match_id, message = %error.message);
    }
    tracing::info!(event = "rematch.created", from = %finished.id, matchId = %match_id, stakes = stakes);
    Ok(())
}

async fn rematch_mode(app: &App, finished: &MatchRow) -> Result<QueueMode, ApiError> {
    // A series game never reaches here (refused below), so `bo3` below is a store that lost a row,
    // not a player: refuse it like one.
    let mut tx = app.db.begin(None).await.map_err(internal)?;
    let mode = tx.matches_mode_of(&finished.id).await.map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    if mode == Some(QueueMode::Bo3) {
        return Err(series_game());
    }
    // A match nothing made (no tickets, room, series or rematch row): replay its frozen decks as a
    // Best of 1 rather than refusing a game both seats want.
    Ok(mode.unwrap_or(QueueMode::Bo1))
}

/// The match a `:matchId` route names and the caller's seat in it, or the 404 anyone else gets
/// (§9.1): a missing match and another profile's match answer the same.
async fn callers_match(app: &App, req: &Req, profile: &Profile) -> Result<(MatchRow, PlayerId), ApiError> {
    let match_id = req.params.get("matchId").map(String::as_str).unwrap_or("");
    let mut tx = app.db.begin(None).await.map_err(internal)?;
    let found = tx.matches_get(match_id).await.map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    let Some(match_row) = found else {
        return Err(no_such_match());
    };
    let Some(seat) = seat_of(&match_row, &profile.id) else {
        return Err(no_such_match());
    };
    Ok((match_row, seat))
}

/// Whether the match was a game of a Conquest series (`series_with_game`).
async fn is_series_game(app: &App, match_id: &str) -> Result<bool, ApiError> {
    let mut tx = app.db.begin(None).await.map_err(internal)?;
    let series = tx.series_with_game(match_id).await.map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(series.is_some())
}

// ---------------------------------------------------------------------------
// Routes
// ---------------------------------------------------------------------------

/// `POST /api/matches/:matchId/rematch` (active): offer a rematch (or meet one). `AuthLevel::Active`
/// is §9.4's gate, as on the queue and the rooms; a seat of another match — or of none — learns
/// nothing beyond "no such match" (§9.1).
///
/// It takes `&Arc<App>` where SURFACE §11.2 writes `&App`: equal offers start the match, and
/// `Registry::start` takes `&Arc<App>`.
pub async fn offer_rematch(app: &Arc<App>, req: Req) -> ApiResult {
    offer_rematch_route(app, req).await.map_err(hide_internal)
}

async fn offer_rematch_route(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let stakes = stakes_of(&req.body)?;
    let (match_row, seat) = callers_match(app, &req, &profile).await?;
    // R672: only a ranked match can spawn a double-or-nothing.
    if stakes == STAKE_DOUBLE && match_row.ranked != Some(true) {
        return Err(api_error(ApiErrorCode::DoubleRequiresRanked, "double-or-nothing needs a ranked match"));
    }
    if match_row.status != MatchStatus::Finished {
        return Err(api_error(ApiErrorCode::MatchNotFinished, "offer a rematch once the match is over"));
    }
    if is_series_game(app, &match_row.id).await? {
        return Err(series_game());
    }

    let now = now_ms();
    // One new game for one pair of equal offers. The check and the claim run under one lock, so
    // two offers landing together cannot mint two games; a failed create clears the claim and
    // returns the error, so a retry mints the next id.
    let (claimed, answered) = {
        let mut map = offers();
        if live_entry(&mut map, &match_row.id, now).is_none() {
            let generation = NEXT_GENERATION.fetch_add(1, Ordering::Relaxed);
            map.insert(
                match_row.id.clone(),
                OfferEntry { offers: IndexMap::new(), at: now, match_id: None, generation },
            );
        }
        let Some(entry) = map.get_mut(&match_row.id) else {
            return Err(internal("the rematch offer vanished while it was being written"));
        };
        entry.at = now;
        entry.offers.insert(seat, stakes);
        if entry.offers.get(&other(seat)) == Some(&stakes) && entry.match_id.is_none() {
            let new_id = new_uuid();
            entry.match_id = Some(new_id.clone());
            (Some((new_id, entry.generation)), None)
        } else {
            (None, entry.match_id.clone())
        }
    };

    let match_id = match claimed {
        None => answered,
        Some((new_id, generation)) => {
            let created = async {
                let mode = rematch_mode(app, &match_row).await?;
                create_rematch(app, &match_row, mode, stakes, &new_id).await
            }
            .await;
            if let Err(error) = created {
                let mut map = offers();
                if let Some(entry) = map.get_mut(&match_row.id) {
                    if entry.generation == generation {
                        entry.match_id = None;
                    }
                }
                return Err(error);
            }
            Some(new_id)
        }
    };
    let created = RematchOfferBody { match_id };
    Ok(json(200, serde_json::to_value(created).map_err(internal)?))
}

/// `GET /api/matches/:matchId/rematch` (active): what each seat offered, whether the opponent is
/// still on the match, and the created game. A seat's own row only: anyone else gets the same "no
/// such match" as for a missing id (§9.1).
pub async fn rematch_status(app: &App, req: Req) -> ApiResult {
    rematch_status_route(app, req).await.map_err(hide_internal)
}

async fn rematch_status_route(app: &App, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let (match_row, seat) = callers_match(app, &req, &profile).await?;
    // A finished series game offers no rematch either: the Conquest continue flow owns what
    // comes next, so the death screen never polls one into view (`match.tsx`).
    if is_series_game(app, &match_row.id).await? {
        return Err(series_game());
    }

    let (you_offered, opponent_offer, created) = {
        let mut map = offers();
        match live_entry(&mut map, &match_row.id, now_ms()) {
            None => (None, None, None),
            Some(entry) => (
                entry.offers.get(&seat).copied(),
                entry.offers.get(&other(seat)).copied(),
                entry.match_id.clone(),
            ),
        }
    };
    let presence = app.matches.presence_of(&match_row.id);
    let status = RematchStatusBody {
        you_offered,
        opponent_offer,
        // Gone whenever no live actor holds the match (a restart, the reaper) or the opponent's
        // socket closed: leaving, logging out and closing the tab all read as gone.
        opponent_here: presence.is_some_and(|presence| presence[other(seat)]),
        match_id: created,
    };
    Ok(json(200, serde_json::to_value(status).map_err(internal)?))
}
