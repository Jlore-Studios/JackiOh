//! Rematch offers after a finished non-series match (SPEC §9.5, R672). Surface contract:
//! docs/v0.3.0/SURFACE.md §11.2. Either seat may offer, normal or double-or-nothing; a game is made
//! only when both seats offer equal stakes. A Conquest series game offers none: its continue flow
//! owns what comes next.
//!
//! R1372, R924000: an All Random rematch deals fresh decks on each seat's latest "More cards from"
//! pick (`leanSet`, or R1372's `leanNewest` from an older client); a Best-of-1 one replays its frozen
//! decks. R1638000: a match on another catalog version
//! is `409 update_required` before any offer is recorded (§9.4). R2685000: an offer counts only
//! within `REMATCH_OFFER_TTL_MS` of the match's `finished_at`, as a co-op Play again's (R1398000).
//! Offers live in memory, one book per `App`: a restart drops them and both clients re-offer.
//! Responses carry only stakes, presence booleans and ids (CLAUDE.md rule 7).
//!
//! The rules are `jackioh_shared::host::rematch`'s, pure (#3101): which stakes are asked, which offers
//! are refused before they are recorded, the window an offer is taken in, what two equal offers make and
//! who is answered what. This module is the request's shape, the store reads the facts come from, the
//! start of the game and the telling of a live actor.

use std::sync::{Arc, Mutex, MutexGuard};

use indexmap::IndexSet;
use jackioh_engine::{LeanPick, PerPlayer, PlayerId};
use jackioh_shared::host::rematch::{
    OfferFacts, OfferOutcome, RematchBook, RematchRefusal, RematchSeen, check_offer,
    stakes_of as stakes_requested,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::actor::contracts::one_tx;
use crate::api::collection::caller_profile;
use crate::api::decks::{already_in_match, lean_of};
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, json};
use crate::api::queue::{discard_open, new_seed, new_uuid};
use crate::app::{App, now_ms};
use crate::db::store::{MatchRow, MatchStatus, Profile, ProfileKind, QueueMode, StartMatchInput};
use crate::seats::Seat;

// R672: the stakes of an offer are the rematch host's (`jackioh_shared::host::rematch`, #3101).
pub use jackioh_shared::host::rematch::{RematchStakes, STAKE_DOUBLE, STAKE_NORMAL};

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

/// R672's offers, by the finished match's id: rendezvous state for two sockets, not rows. One per
/// `App` (`App::rematch_offers`), like `QueuePresence`, so parallel tests never share offers and one
/// test's sweep never reaches another's.
#[derive(Default)]
pub struct RematchOffers(Mutex<RematchBook>);

fn offers(app: &App) -> MutexGuard<'_, RematchBook> {
    app.rematch_offers
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// For the tests; nothing in `src/` calls it.
pub fn rematch_offer_count(app: &App) -> usize {
    offers(app).len()
}

/// Sweeps expired rematch offers older than `REMATCH_OFFER_TTL_MS` with no live actor.
pub fn sweep_rematch_offers(app: &App, now: i64) -> usize {
    // R3213000: the book is a leaf lock, taken under nothing but an actor's own (which reads it to tell
    // its seats), so the live matches are read before it, never under it.
    let live: IndexSet<String> = app.matches.live().into_iter().collect();
    offers(app).sweep(now, |match_id| live.contains(match_id))
}

/// R3213000: what each seat of `match_id` is told of its rematch at `now`, in seat order, under one look at
/// the book.
pub(crate) fn rematch_seen(app: &App, match_id: &str, now: i64) -> PerPlayer<RematchSeen> {
    let mut book = offers(app);
    let p1 = book.seen(match_id, PlayerId::P1, now);
    let p2 = book.seen(match_id, PlayerId::P2, now);
    PerPlayer::new(p1, p2)
}

/// R3213000: the book changed for `match_id`: its live actor tells each open seat its status again.
fn tell_seats(app: &App, match_id: &str) {
    if let Some(actor) = app.matches.get(match_id) {
        actor.tell_rematch();
    }
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

/// R1638000: a card patch went live since the match began, for a 1v1 rematch and a co-op Play again.
/// `details` are the co-op join's.
pub(crate) fn cards_changed(app: &App, played_on: &str) -> ApiError {
    ApiError::with_details(
        ApiErrorCode::UpdateRequired,
        "The cards changed since this game. Start a new one.",
        json!({ "expected": app.catalog.version, "received": played_on }),
    )
}

// R1285000, R2685000: whether a finished match's rematch window is open at `now` is the rematch host's
// rule, and so is the co-op Play again's and the reaper's.
pub(crate) use jackioh_shared::host::rematch::in_rematch_window;

// Offers

fn stakes_of(body: &Value) -> Result<RematchStakes, ApiError> {
    stakes_requested(body.get("stakes").and_then(Value::as_f64)).ok_or_else(|| {
        ApiError::new(
            ApiErrorCode::BadRequest,
            "\"stakes\" must be 1 (rematch) or 2 (double-or-nothing)",
        )
    })
}

/// The error a refusal before the offer is recorded answers with.
fn refusal_error(app: &App, refusal: RematchRefusal, played_on: &str) -> ApiError {
    match refusal {
        RematchRefusal::DoubleRequiresRanked => ApiError::new(
            ApiErrorCode::DoubleRequiresRanked,
            "double-or-nothing needs a ranked match",
        ),
        RematchRefusal::MatchNotFinished => ApiError::new(
            ApiErrorCode::MatchNotFinished,
            "offer a rematch once the match is over",
        ),
        RematchRefusal::SeriesGame => series_game(),
        RematchRefusal::UpdateRequired => cards_changed(app, played_on),
    }
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

/// Creates a rematch from equal offers, swapping seats (R1989000, R1372, R924000).
async fn create_rematch(
    app: &Arc<App>,
    finished: &MatchRow,
    mode: QueueMode,
    stakes: RematchStakes,
    leans: (Option<LeanPick>, Option<LeanPick>),
    match_id: &str,
) -> Result<(), ApiError> {
    let random = mode == QueueMode::Random;
    let seed = new_seed();
    // R1989000, R1834002: the seats swap, and the finished match's slots are judged again by the new
    // loadout (`jackioh_shared::host::rematch::next_seats`).
    let seats = crate::actor::engine::run_pinned(Default::default(), || {
        jackioh_shared::host::rematch::next_seats(
            &jackioh_shared::host::rematch::FinishedSeats {
                players: finished.players.clone(),
                decks: finished.decks.clone(),
                portraits: finished
                    .portraits
                    .as_ref()
                    .map(|portraits| (portraits.0.to_string(), portraits.1.to_string())),
                slots: finished
                    .loadouts
                    .as_ref()
                    .map(|loadouts| (loadouts.0.seat.deck_slots(), loadouts.1.seat.deck_slots())),
            },
            random,
            &seed,
            leans,
        )
    });

    // Refuse before minting anything: a seat that found another game meanwhile keeps that game.
    // R1523000: both seats, or neither, reserved in one step; a seat that holds a match, a series
    // not over or a live co-op match refuses with no details, since it may be the other player's
    // (R1065000). Every undo below that runs before the commit gives them back as the lease drops.
    let lease = app
        .seats
        .reserve(
            &[&seats.0.profile_id, &seats.1.profile_id],
            Seat::Match(match_id.to_string()),
        )
        .map_err(|_| already_in_match())?;
    let mut bots = Vec::new();
    {
        let mut tx = app.db.begin(None).await?;
        tx.seats_fence(app.seats.epoch()).await?;
        for seat in [&seats.0, &seats.1] {
            let found = tx
                .profiles_get_many(std::slice::from_ref(&seat.profile_id))
                .await?;
            if found
                .first()
                .is_some_and(|profile| profile.kind == ProfileKind::Bot)
            {
                bots.push(seat.profile_id.clone());
            }
        }
        tx.commit().await?;
    }

    // R639000: a ladder bot is claimed for its rematch in one step, as the fallback claims one for a
    // ticket (R1453): free and not retired, whatever its schedule says. One claimed meanwhile, or any
    // while `LADDER_BOTS` is off, answers as a seat that found another game.
    if !bots.is_empty() {
        if !app.env.ladder_bots {
            return Err(already_in_match());
        }
        let mut tx = app.db.begin(None).await?;
        tx.seats_fence(app.seats.epoch()).await?;
        for bot in &bots {
            let claimed = tx
                .bots_claim_for_rematch(
                    bot,
                    &finished.players,
                    match_id,
                    &finished.catalog_version,
                    now_ms(),
                )
                .await?;
            if !claimed {
                return Err(already_in_match());
            }
        }
        tx.commit().await?;
    }

    // A failed start leaves nothing behind: the caller clears the offer claim and the seats re-offer.
    let started = app
        .matches
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
                win_rates: None,
            },
        )
        .await;
    if let Err(error) = started {
        // R639000: the claimed bot is let go and its skeleton discarded, as `queue.rs` lets go a bot
        // whose fallback start failed.
        if !bots.is_empty() {
            let cleanup = async {
                let mut tx = app.db.begin(None).await?;
                for bot in &bots {
                    tx.profiles_clear_in_match(bot, match_id).await?;
                }
                tx.commit().await?;
                discard_open(app, match_id).await
            }
            .await;
            if let Err(cleanup) = cleanup {
                tracing::error!(event = "rematch.bot_cleanup_failed", matchId = %match_id, message = %cleanup.message);
            }
        }
        return Err(error);
    }

    // After the start, not before: the row is what the in-match reference points at
    // (`profiles_current_match_id_fkey`), so flagging first fails on Postgres. Each seat is
    // claimed compare-and-set (R890000): a seat taken during the start loses here instead of
    // being seated twice, and the taken seat keeps its game and tickets.
    let seated = async {
        let mut tx = app.db.begin(None).await?;
        // R1523000: a newer server holds the seats once the epoch has moved on; nothing is written.
        tx.seats_fence(app.seats.epoch()).await?;
        let mut taken: Option<String> = None;
        for seat in [&seats.0, &seats.1] {
            // A seat the claim already holds (R639000's bots, seated for this match before the
            // start) reads back as seated: only a flag pointing elsewhere loses. The read is
            // only for equality; the compare-and-set stays the guard.
            let held = tx
                .profiles_get_many(std::slice::from_ref(&seat.profile_id))
                .await?;
            let already = held
                .first()
                .is_some_and(|profile| profile.in_match_id.as_deref() == Some(match_id));
            if !already && !tx.profiles_set_in_match(&seat.profile_id, Some(match_id)).await? {
                taken = Some(seat.profile_id.clone());
                break;
            }
            // A stray open ticket would block the re-queue M7-T1 promises after this game ends.
            if let Some(ticket) = tx.tickets_open_for_profile(&seat.profile_id).await? {
                tx.tickets_cancel(&ticket.id, now_ms()).await?;
            }
        }
        tx.commit().await?;
        Ok::<_, ApiError>(taken)
    }
    .await;
    // A seat taken mid-start (the compare-and-set's refusal), a fenced write and a failed write all
    // undo the rematch: kept without its flags it would refuse every socket and end in a draw nobody
    // played. Erasing the live match with no result (R679) lets `on delete set null` release the
    // winner seat's flag and any bot flags `bots_claim_for_rematch` set; a taken seat is untouched.
    let refused = match seated {
        Err(error) => {
            // A fenced write (`unavailable`) is a newer server taking over, not a failure.
            if error.code != ApiErrorCode::Unavailable {
                tracing::error!(
                    event = "rematch.in_match_failed",
                    matchId = %match_id,
                    message = %error.message,
                );
            }
            error
        }
        Ok(Some(taken)) => {
            tracing::error!(event = "seats.drift", source = "compare-and-set", matchId = %match_id);
            tracing::info!(event = "rematch.seat_taken", matchId = %match_id, profileId = %taken);
            already_in_match()
        }
        Ok(None) => {
            lease.commit();
            tracing::info!(event = "rematch.created", from = %finished.id, matchId = %match_id, stakes = stakes);
            return Ok(());
        }
    };
    app.matches.stop(match_id).await;
    let voided = one_tx!(app.db, |t| t.matches_forget_voided(match_id).await?);
    if let Err(error) = voided {
        tracing::error!(event = "rematch.void_failed", matchId = %match_id, message = %error);
    }
    Err(refused)
}

async fn rematch_mode(app: &App, finished: &MatchRow) -> Result<QueueMode, ApiError> {
    // A series game never reaches here, so `bo3` is a store that lost a row: refuse it like one.
    let mode = one_tx!(app.db, |t| t.matches_mode_of(&finished.id).await?)?;
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
    let found = one_tx!(app.db, |t| t.matches_get(match_id).await?)?;
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
    let series = one_tx!(app.db, |t| t.series_with_game(match_id).await?)?;
    Ok(series.is_some())
}

// Routes

/// `POST /api/matches/:matchId/rematch` (active): offers or meets a rematch (§9.4, §9.1).
pub async fn offer_rematch(app: &Arc<App>, req: Req) -> ApiResult {
    offer_rematch_route(app, req).await
}

async fn offer_rematch_route(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let stakes = stakes_of(&req.body)?;
    // R1372, R924000: absent is no lean, so an offer from a client before it is a plain rematch.
    let lean = lean_of(&req.body)?;
    let (match_row, seat) = callers_match(app, &req, &profile).await?;
    let match_id = offer_as(app, &match_row, seat, stakes, lean).await?;
    let created = RematchOfferBody { match_id };
    Ok(json(
        200,
        serde_json::to_value(created).map_err(|error| ApiError::internal(error.to_string()))?,
    ))
}

/// Submits a rematch offer for a ladder bot (R639000, R1372).
pub async fn offer_for_bot(
    app: &Arc<App>,
    match_id: &str,
    seat: PlayerId,
    stakes: RematchStakes,
) -> Result<Option<String>, ApiError> {
    if !app.env.ladder_bots {
        return Ok(None);
    }
    let found = one_tx!(app.db, |t| t.matches_get(match_id).await?)?;
    let Some(match_row) = found else {
        return Ok(None);
    };
    offer_as(app, &match_row, seat, stakes, None).await
}

/// Handles a rematch offer for `seat`, creating a game if matched (R639000).
async fn offer_as(
    app: &Arc<App>,
    match_row: &MatchRow,
    seat: PlayerId,
    stakes: RematchStakes,
    lean: Option<LeanPick>,
) -> Result<Option<String>, ApiError> {
    // R672, R1638000: refused before the offer is recorded, so nothing is kept and no rematch is made.
    // Whether the match was a series game is read only for a match that is over, as it always was.
    let finished = match_row.status == MatchStatus::Finished;
    let series_game = finished && is_series_game(app, &match_row.id).await?;
    check_offer(&OfferFacts {
        stakes,
        ranked: match_row.ranked,
        finished,
        series_game,
        catalog_current: match_row.catalog_version == app.catalog.version,
    })
    .map_err(|refusal| refusal_error(app, refusal, &match_row.catalog_version))?;

    let now = now_ms();
    // One new game per pair of equal offers: the check and the claim are one step under one lock, so two
    // offers landing together cannot mint two games; a failed create clears the claim. R2685000: an
    // offer past the finished match's window lapses silently.
    let outcome = offers(app).offer(
        &match_row.id,
        match_row.finished_at,
        seat,
        stakes,
        lean,
        now,
        &new_uuid(),
    );

    let made = matches!(outcome, OfferOutcome::Claimed { .. });
    let match_id = match outcome {
        // R2685000: a lapsed offer records nothing, tells no actor and makes no game: the entry that
        // already made one is how a seat that offers or polls late still learns where to go.
        OfferOutcome::Lapsed { answered } => return Ok(answered),
        OfferOutcome::Waiting { answered } => answered,
        OfferOutcome::Claimed {
            match_id: new_id,
            generation,
            leans,
        } => {
            let created = async {
                let mode = rematch_mode(app, match_row).await?;
                create_rematch(app, match_row, mode, stakes, leans, &new_id).await
            }
            .await;
            if let Err(error) = created {
                offers(app).release(&match_row.id, generation);
                // R3213000: the offer stands while the claim is given back, and the seats are told so.
                tell_seats(app, &match_row.id);
                return Err(error);
            }
            Some(new_id)
        }
    };
    // R1442: the offer, and whether it made the rematch, among the finished match's signals.
    crate::actor::telemetry::note_rematch(app, &match_row.id, seat, made).await;
    if match_id.is_none()
        && let Some(actor) = app.matches.get(&match_row.id)
    {
        actor.on_rematch_offer(seat, stakes);
    }
    // R3213000: each open seat of the finished match is told the book as it is now.
    tell_seats(app, &match_row.id);
    Ok(match_id)
}

/// `GET /api/matches/:matchId/rematch` (active): rematch status for caller's seat (§9.1).
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

    let seen = offers(app).seen(&match_row.id, seat, now_ms());
    let presence = app.matches.presence_of(&match_row.id);
    // R1372: the mode a rematch would play, read as the rematch itself reads it.
    let mode = rematch_mode(app, &match_row).await?;
    let status = RematchStatusBody {
        you_offered: seen.you_offered,
        opponent_offer: seen.opponent_offer,
        // Gone whenever no live actor holds the match or the opponent's socket closed.
        opponent_here: presence.is_some_and(|presence| presence[seat.opponent()]),
        match_id: seen.match_id,
        mode,
    };
    Ok(json(
        200,
        serde_json::to_value(status).map_err(|error| ApiError::internal(error.to_string()))?,
    ))
}
