//! Rematch offers after a finished non-series match (SPEC §9.5, R672).
//!
//! Either seat may offer a rematch, normal or double-or-nothing; a new match is created only when
//! both seats offer equal stakes. "The opponent is still here" is their match socket being open
//! (`Registry::presence_of`). Games of a Conquest series offer nothing: the series' continue flow
//! owns what comes next. Surface contract: docs/v0.3.0/SURFACE.md §11.2.
//!
//! R1372: an All Random rematch deals fresh decks, each seat's offer carrying its player's "More
//! cards from the newest set" (`leanNewest`, absent = off; the latest offer's stands). A Best-of-1
//! rematch replays its frozen decks and reads no lean.
//!
//! The response carries only offer stakes, presence booleans and ids (CLAUDE.md rule 7).
//!
//! The offers live in a module-level map, like `queue.rs`'s `E2E_SEED_BY_TICKET`: rendezvous state
//! for two sockets, not rows, so a restart drops them and both clients re-offer. Entries without a
//! created game are swept on read past `REMATCH_OFFER_TTL_MS`; one that made a game keeps its id so
//! a late poller still learns where to go.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};

use indexmap::IndexMap;
use jackioh_engine::PlayerId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::collection::caller_profile;
use crate::api::decks::lean_newest_of;
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, json};
use crate::api::queue::{new_seed, new_uuid};
use crate::app::{App, now_ms};
use crate::config::REMATCH_OFFER_TTL_MS;
use crate::db::store::{MatchRow, MatchSeat, MatchStatus, Profile, QueueMode, StartMatchInput};

/// R672: a normal rematch moves the rating once; a double-or-nothing moves it twice.
pub const STAKE_NORMAL: RematchStakes = 1;
/// R672: a double-or-nothing rematch. Ranked matches only (`double_requires_ranked`).
pub const STAKE_DOUBLE: RematchStakes = 2;

/// `STAKE_NORMAL` or `STAKE_DOUBLE`, nothing else.
pub type RematchStakes = i32;

/// `POST /api/matches/:matchId/rematch`: the created game, or null while the seats disagree.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RematchOfferBody {
    pub match_id: Option<String>,
}

/// `GET /api/matches/:matchId/rematch`: both offers, the opponent's presence, the created game, and
/// the mode a rematch of this match plays (R1372: the death screen offers "More cards from the
/// newest set" only beside an All Random one).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RematchStatusBody {
    pub you_offered: Option<RematchStakes>,
    pub opponent_offer: Option<RematchStakes>,
    pub opponent_here: bool,
    pub match_id: Option<String>,
    pub mode: QueueMode,
}

struct OfferEntry {
    /// Seat -> the stakes it offered.
    offers: IndexMap<PlayerId, RematchStakes>,
    /// R1372: seat -> its latest offer's "More cards from the newest set".
    leans: IndexMap<PlayerId, bool>,
    /// `now` of the latest offer; the sweep reads this.
    at: i64,
    /// The game equal stakes made, once made.
    match_id: Option<String>,
    /// Which entry this is: a swept-and-remade entry must not be mistaken for it after a failed create.
    generation: u64,
}

static OFFERS_BY_MATCH: LazyLock<Mutex<IndexMap<String, OfferEntry>>> =
    LazyLock::new(|| Mutex::new(IndexMap::new()));

static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);

fn offers() -> MutexGuard<'static, IndexMap<String, OfferEntry>> {
    OFFERS_BY_MATCH
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// For the tests; nothing in `src/` calls it.
pub fn clear_rematch_offers() {
    offers().clear();
}

/// For the tests; nothing in `src/` calls it.
pub fn rematch_offer_count() -> usize {
    offers().len()
}

// Small private copies (fullsend rule 5): the error shapes

fn no_such_match() -> ApiError {
    ApiError::new(ApiErrorCode::NotFound, "no such match")
}

fn series_game() -> ApiError {
    ApiError::new(
        ApiErrorCode::SeriesGame,
        "series games continue from the series screen",
    )
}

// Offers

fn stakes_of(body: &Value) -> Result<RematchStakes, ApiError> {
    let stakes = body.get("stakes").and_then(Value::as_f64);
    if stakes == Some(STAKE_NORMAL as f64) {
        return Ok(STAKE_NORMAL);
    }
    if stakes == Some(STAKE_DOUBLE as f64) {
        return Ok(STAKE_DOUBLE);
    }
    Err(ApiError::new(
        ApiErrorCode::BadRequest,
        "\"stakes\" must be 1 (rematch) or 2 (double-or-nothing)",
    ))
}

/// The live entry for a match, sweeping one whose offers went stale. An entry that already made a
/// game is never swept: the id is how a late poller finds the game.
fn live_entry<'a>(
    offers: &'a mut IndexMap<String, OfferEntry>,
    match_id: &str,
    now: i64,
) -> Option<&'a mut OfferEntry> {
    let stale = {
        let entry = offers.get(match_id)?;
        entry.match_id.is_none() && now - entry.at > REMATCH_OFFER_TTL_MS
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

/// Creates the rematch both seats offered, as `queue.rs`'s `start_paired_match` makes a paired
/// match: a Best-of-1 replays the frozen decks and portraits under a fresh seed, an All Random deals
/// fresh decks from it, each leaning on the newest set when its seat asked (`leans`, R1372).
async fn create_rematch(
    app: &Arc<App>,
    finished: &MatchRow,
    mode: QueueMode,
    stakes: RematchStakes,
    leans: (bool, bool),
    match_id: &str,
) -> Result<(), ApiError> {
    let seed = new_seed();
    let random = mode == QueueMode::Random;
    let portrait_of = |at: usize| -> Option<String> {
        finished
            .portraits
            .as_ref()
            .map(|portraits| (if at == 0 { &portraits.0 } else { &portraits.1 }).to_string())
    };
    let dealt_portrait = |seat: &str| -> Option<String> {
        Some(jackioh_engine::wire::pick_portrait_from_seed(&format!("{seed}:portrait:{seat}")).to_string())
    };
    let seats = (
        MatchSeat {
            profile_id: finished.players.0.clone(),
            player: PlayerId::P1,
            deck: if random {
                crate::actor::engine::deal_random_deck(
                    &format!("{seed}:p1-deck"),
                    crate::actor::engine::lean_of(leans.0),
                )
            } else {
                finished.decks.0.clone()
            },
            portrait: if random {
                dealt_portrait("p1")
            } else {
                portrait_of(0)
            },
        },
        MatchSeat {
            profile_id: finished.players.1.clone(),
            player: PlayerId::P2,
            deck: if random {
                crate::actor::engine::deal_random_deck(
                    &format!("{seed}:p2-deck"),
                    crate::actor::engine::lean_of(leans.1),
                )
            } else {
                finished.decks.1.clone()
            },
            portrait: if random {
                dealt_portrait("p2")
            } else {
                portrait_of(1)
            },
        },
    );

    // Refuse before minting anything: a seat that found another game meanwhile keeps that game.
    {
        let mut tx = app
            .db
            .begin(None)
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?;
        for seat in [&seats.0, &seats.1] {
            let found = tx
                .profiles_get_many(std::slice::from_ref(&seat.profile_id))
                .await
                .map_err(|error| ApiError::internal(error.to_string()))?;
            if found.first().is_some_and(|profile| profile.in_match_id.is_some()) {
                return Err(ApiError::new(
                    ApiErrorCode::AlreadyInMatch,
                    "finish your current match first",
                ));
            }
        }
        tx.commit()
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?;
    }

    // A failed start leaves nothing behind: the caller clears the offer claim and the seats re-offer.
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
                // Absent reads as 1 downstream; only a double writes its stakes.
                stake: if stakes == STAKE_DOUBLE {
                    Some(i64::from(stakes))
                } else {
                    None
                },
                seats: seats.clone(),
            },
        )
        .await?;

    // After the start, not before: the row is what the in-match reference points at
    // (`profiles_current_match_id_fkey`), so flagging first fails on Postgres. A seat taken during
    // the start keeps its game and tickets; a failure here is logged, not thrown.
    let flagged = async {
        let mut tx = app.db.begin(None).await.map_err(|error| ApiError::internal(error.to_string()))?;
        for seat in [&seats.0, &seats.1] {
            let found = tx.profiles_get_many(std::slice::from_ref(&seat.profile_id)).await.map_err(|error| ApiError::internal(error.to_string()))?;
            let taken = found
                .first()
                .is_some_and(|profile| profile.in_match_id.as_deref().is_some_and(|current| current != match_id));
            if taken {
                tracing::info!(event = "rematch.seat_taken", matchId = %match_id, profileId = %seat.profile_id);
                continue;
            }
            tx.profiles_set_in_match(&seat.profile_id, Some(match_id)).await.map_err(|error| ApiError::internal(error.to_string()))?;
            // A stray open ticket would block the re-queue M7-T1 promises after this game ends.
            if let Some(ticket) = tx.tickets_open_for_profile(&seat.profile_id).await.map_err(|error| ApiError::internal(error.to_string()))? {
                tx.tickets_cancel(&ticket.id, now_ms()).await.map_err(|error| ApiError::internal(error.to_string()))?;
            }
        }
        tx.commit().await.map_err(|error| ApiError::internal(error.to_string()))
    }
    .await;
    if let Err(error) = flagged {
        tracing::error!(event = "rematch.in_match_failed", matchId = %match_id, message = %error.message);
    }
    tracing::info!(event = "rematch.created", from = %finished.id, matchId = %match_id, stakes = stakes);
    Ok(())
}

async fn rematch_mode(app: &App, finished: &MatchRow) -> Result<QueueMode, ApiError> {
    // A series game never reaches here, so `bo3` is a store that lost a row: refuse it like one.
    let mut tx = app
        .db
        .begin(None)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let mode = tx
        .matches_mode_of(&finished.id)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    tx.commit()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    if mode == Some(QueueMode::Bo3) {
        return Err(series_game());
    }
    // A match nothing made: replay its frozen decks as a Best of 1 rather than refuse it.
    Ok(mode.unwrap_or(QueueMode::Bo1))
}

/// The match a `:matchId` route names and the caller's seat in it, or the 404 anyone else gets
/// (§9.1): a missing match and another profile's match answer the same.
async fn callers_match(app: &App, req: &Req, profile: &Profile) -> Result<(MatchRow, PlayerId), ApiError> {
    let match_id = req.params.get("matchId").map(String::as_str).unwrap_or("");
    let mut tx = app
        .db
        .begin(None)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let found = tx
        .matches_get(match_id)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    tx.commit()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
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
    let mut tx = app
        .db
        .begin(None)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let series = tx
        .series_with_game(match_id)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    tx.commit()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    Ok(series.is_some())
}

// Routes

/// `POST /api/matches/:matchId/rematch` (active): offer a rematch (or meet one). `AuthLevel::Active`
/// is §9.4's gate, as on the queue and the rooms; a seat of another match — or of none — learns
/// nothing beyond "no such match" (§9.1).
///
/// It takes `&Arc<App>` because equal offers start the match and `Registry::start` needs one.
pub async fn offer_rematch(app: &Arc<App>, req: Req) -> ApiResult {
    offer_rematch_route(app, req).await
}

async fn offer_rematch_route(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let stakes = stakes_of(&req.body)?;
    // R1372: absent is off, so an offer from a client before it is a plain rematch.
    let lean_newest = lean_newest_of(&req.body)?;
    let (match_row, seat) = callers_match(app, &req, &profile).await?;
    // R672: only a ranked match can spawn a double-or-nothing.
    if stakes == STAKE_DOUBLE && match_row.ranked != Some(true) {
        return Err(ApiError::new(
            ApiErrorCode::DoubleRequiresRanked,
            "double-or-nothing needs a ranked match",
        ));
    }
    if match_row.status != MatchStatus::Finished {
        return Err(ApiError::new(
            ApiErrorCode::MatchNotFinished,
            "offer a rematch once the match is over",
        ));
    }
    if is_series_game(app, &match_row.id).await? {
        return Err(series_game());
    }

    let now = now_ms();
    // One new game per pair of equal offers: check and claim run under one lock, so two offers
    // landing together cannot mint two games; a failed create clears the claim.
    let (claimed, answered) = {
        let mut map = offers();
        if live_entry(&mut map, &match_row.id, now).is_none() {
            let generation = NEXT_GENERATION.fetch_add(1, Ordering::Relaxed);
            map.insert(
                match_row.id.clone(),
                OfferEntry {
                    offers: IndexMap::new(),
                    leans: IndexMap::new(),
                    at: now,
                    match_id: None,
                    generation,
                },
            );
        }
        let Some(entry) = map.get_mut(&match_row.id) else {
            return Err(ApiError::internal(
                "the rematch offer vanished while it was being written",
            ));
        };
        entry.at = now;
        entry.offers.insert(seat, stakes);
        entry.leans.insert(seat, lean_newest);
        if entry.offers.get(&seat.opponent()) == Some(&stakes) && entry.match_id.is_none() {
            let new_id = new_uuid();
            entry.match_id = Some(new_id.clone());
            let lean_of = |at: PlayerId| entry.leans.get(&at).copied().unwrap_or(false);
            let leans = (lean_of(PlayerId::P1), lean_of(PlayerId::P2));
            (Some((new_id, entry.generation, leans)), None)
        } else {
            (None, entry.match_id.clone())
        }
    };

    let made = claimed.is_some();
    let match_id = match claimed {
        None => answered,
        Some((new_id, generation, leans)) => {
            let created = async {
                let mode = rematch_mode(app, &match_row).await?;
                create_rematch(app, &match_row, mode, stakes, leans, &new_id).await
            }
            .await;
            if let Err(error) = created {
                let mut map = offers();
                if let Some(entry) = map.get_mut(&match_row.id)
                    && entry.generation == generation
                {
                    entry.match_id = None;
                }
                return Err(error);
            }
            Some(new_id)
        }
    };
    // R1442: the offer, and whether it made the rematch, among the finished match's signals.
    crate::actor::telemetry::note_rematch(app, &match_row.id, seat, made).await;
    let created = RematchOfferBody { match_id };
    Ok(json(
        200,
        serde_json::to_value(created).map_err(|error| ApiError::internal(error.to_string()))?,
    ))
}

/// `GET /api/matches/:matchId/rematch` (active): what each seat offered, whether the opponent is
/// still on the match, and the created game. A seat's own row only: anyone else gets the same "no
/// such match" as for a missing id (§9.1).
pub async fn rematch_status(app: &Arc<App>, req: Req) -> ApiResult {
    rematch_status_route(app, req).await
}

async fn rematch_status_route(app: &App, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let (match_row, seat) = callers_match(app, &req, &profile).await?;
    // A finished series game offers no rematch: the Conquest continue flow owns what comes next.
    if is_series_game(app, &match_row.id).await? {
        return Err(series_game());
    }

    let (you_offered, opponent_offer, created) = {
        let mut map = offers();
        match live_entry(&mut map, &match_row.id, now_ms()) {
            None => (None, None, None),
            Some(entry) => (
                entry.offers.get(&seat).copied(),
                entry.offers.get(&seat.opponent()).copied(),
                entry.match_id.clone(),
            ),
        }
    };
    let presence = app.matches.presence_of(&match_row.id);
    // R1372: the mode a rematch would play, read as the rematch itself reads it.
    let mode = rematch_mode(app, &match_row).await?;
    let status = RematchStatusBody {
        you_offered,
        opponent_offer,
        // Gone whenever no live actor holds the match or the opponent's socket closed.
        opponent_here: presence.is_some_and(|presence| presence[seat.opponent()]),
        match_id: created,
        mode,
    };
    Ok(json(
        200,
        serde_json::to_value(status).map_err(|error| ApiError::internal(error.to_string()))?,
    ))
}
