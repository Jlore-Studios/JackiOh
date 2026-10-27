//! Play again after a finished co-op match (SPEC §13.9, R2092000), modelled on the 1v1
//! rematch (`api::rematch`, R672): either human seat (`a1` or `a2`) offers, and once both have the
//! server starts one new co-op match for the same two people against the AI team. Nothing in the
//! client calls it yet: part 2 builds the result panel's button on it, so players see nothing new.
//!
//! `POST /api/duos/matches/:matchId/again` records the caller's offer and answers the new match's
//! id once both seats have offered, else null; `GET` on the same path answers what each seat
//! offered, whether the teammate is still on the finished match and the created game. A co-op game
//! is unranked, so there are no stakes: the new match keeps the same `a1` and `a2` (the host stays
//! a1, §13.9) and the same `difficulty`, and so the same AI team rules and handicaps (§13.7), under
//! a fresh seed and the server's catalog version (R767000). A seat that played a saved deck replays
//! the frozen deck from the finished match's `options`; a seat whose deck was dealt is dealt
//! afresh, leaning on the set its latest offer picked (R924000, R2031000), as an All Random rematch
//! deals. The AI seats are dealt afresh by the AI's deck-builder, as a room's join deals them.
//!
//! Who may offer, and when: only a seat of a finished match. Anyone else, an unknown id and a live
//! match get the replay route's answers (`duos::replay`); a seat emptied by a deleted account
//! (R1497000) is refused; a finished match on another catalog version is `409 update_required`
//! (R1638000), as the co-op room's join refuses one; past `REMATCH_OFFER_TTL_MS` from the match's
//! `ended_at` — the window a finished co-op match's socket reopens in (§13.9, R1398000) — the offer
//! lapses silently, as a 1v1 rematch's does. Making the match refuses before anything is made: a
//! frozen deck with a custom card is `loadout_invalid` (R915005), a seat in another game meets
//! `assert_free_to_play`'s refusal (R1065000, through the seat ledger, R1523000), and a pair with a
//! block either way reads as an unknown code, as a room's does (§13.9).
//!
//! The response carries only booleans and ids (CLAUDE.md rule 7).
//!
//! The offers live in memory, one map per `App` (`DuosAgainOffers`): rendezvous state for two
//! sockets, not rows, so a restart drops them and both people re-offer. An entry without a created
//! game is swept on read past `REMATCH_OFFER_TTL_MS`; one that made a game keeps its id so a late
//! poller still learns where to go. The reaper's tick sweeps every entry past that window whose
//! finished match no actor holds (`sweep_duos_again_offers`).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use indexmap::{IndexMap, IndexSet};
use jackioh_engine::LeanPick;
use jackioh_engine::wire::DuosSeat;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::actor::contracts::one_tx;
use crate::api::collection::caller_profile;
use crate::api::decks::{assert_free_to_play, assert_no_custom_cards, in_game, lean_of};
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, json};
use crate::api::queue::new_uuid;
use crate::app::{App, now_ms};
use crate::config::REMATCH_OFFER_TTL_MS;
use crate::db::store::{FrozenDeck, Profile};
use crate::duos::actor::HUMAN_SEATS;
use crate::duos::registry::registry;
use crate::duos::rooms::{CoopStart, CoopStartOutcome, start_coop_match};
use crate::duos::store::{DuosChoice, DuosMatch};

/// `POST /api/duos/matches/:matchId/again`: the created game, or null while both seats have not
/// offered.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgainOfferBody {
    pub match_id: Option<String>,
}

/// `GET /api/duos/matches/:matchId/again`: what each seat offered, whether the teammate is still
/// on the finished match, and the created game.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgainStatusBody {
    pub you_offered: bool,
    pub teammate_offered: bool,
    pub teammate_here: bool,
    pub match_id: Option<String>,
}

struct OfferEntry {
    /// The seats that offered.
    offers: IndexSet<DuosSeat>,
    /// R924000, R2031000: each seat's latest offer's "More cards from" pick.
    leans: IndexMap<DuosSeat, Option<LeanPick>>,
    /// `now` of the latest offer; the sweep reads this.
    at: i64,
    /// The game both seats' offers made, once made.
    match_id: Option<String>,
    /// Which entry this is: a swept-and-remade entry must not be mistaken for it after a failed create.
    generation: u64,
}

/// R2092000's offers, by the finished match's id: rendezvous state for two sockets, not rows. One
/// per `App` (`App::duos_again_offers`), like `RematchOffers`, so parallel tests never share offers
/// and one test's sweep never reaches another's.
#[derive(Default)]
pub struct DuosAgainOffers(Mutex<IndexMap<String, OfferEntry>>);

static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);

fn offers(app: &App) -> MutexGuard<'_, IndexMap<String, OfferEntry>> {
    app.duos_again_offers
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// For the tests; nothing in `src/` calls it.
pub fn duos_again_offer_count(app: &App) -> usize {
    offers(app).len()
}

/// The reaper's tick (`api::results::run_reaper`, after `Registry::reap_finished`): drops each
/// entry whose latest offer is older than `REMATCH_OFFER_TTL_MS` and whose finished match no actor
/// holds any more. An unanswered one is what `live_entry` sweeps on a read nobody makes; one that
/// made a game goes too, since the new match's actor holds its own game now. An open socket keeps
/// its actor, and so its offers. Answers how many it dropped.
pub fn sweep_duos_again_offers(app: &Arc<App>, now: i64) -> usize {
    let mut map = offers(app);
    let before = map.len();
    map.retain(|match_id, entry| now - entry.at <= REMATCH_OFFER_TTL_MS || registry(app).has(match_id));
    before - map.len()
}

// Small private copies (fullsend rule 5): the error shapes

fn no_such_match() -> ApiError {
    ApiError::new(ApiErrorCode::NotFound, "no such co-op match")
}

fn not_finished() -> ApiError {
    ApiError::new(ApiErrorCode::Conflict, "this co-op match has not finished")
}

fn not_open() -> ApiError {
    ApiError::new(ApiErrorCode::NotFound, "that co-op room is not open")
}

/// R1638000: a match played on a catalog version other than the server's offers no Play again. An
/// offer on it is `409 update_required` before it is recorded, as the co-op room's join refuses a
/// stale catalog (§13.9): a new game would deal today's cards under the old rules.
fn cards_changed(app: &App, played_on: &str) -> ApiError {
    ApiError::with_details(
        ApiErrorCode::UpdateRequired,
        "The cards changed since this game. Start a new one.",
        json!({ "expected": app.catalog.version, "received": played_on }),
    )
}

// Offers

/// The live entry for a match, sweeping one whose offers went stale. An entry that already made a
/// game is never swept here: the id is how a late poller finds the game, until the reaper's
/// `sweep_duos_again_offers`.
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

/// The match a `:matchId` route names and the caller's seat in it, or the 404 anyone else gets
/// (§13.9): a missing match and another pair's match answer the same, as the replay route's do.
async fn callers_match(app: &App, req: &Req, profile: &Profile) -> Result<(DuosMatch, DuosSeat), ApiError> {
    let match_id = req.params.get("matchId").map(String::as_str).unwrap_or("");
    let found = one_tx!(app.db, |t| t.duos_matches_get(match_id).await?)?;
    let Some(match_row) = found else {
        return Err(no_such_match());
    };
    let seat = if match_row.a1 == profile.id {
        DuosSeat::A1
    } else if match_row.a2 == profile.id {
        DuosSeat::A2
    } else {
        return Err(no_such_match());
    };
    Ok((match_row, seat))
}

/// Records the offer and, once both seats have offered, makes the new co-op match outside the lock.
/// One new game per pair of offers: the check and the claim run under one lock, so two offers
/// landing together cannot mint two games; a failed create clears the claim, so the seats re-offer.
async fn offer_as(
    app: &Arc<App>,
    match_row: &DuosMatch,
    seat: DuosSeat,
    lean: Option<LeanPick>,
) -> Result<Option<String>, ApiError> {
    // The claim: the minted game, or none while the pair is incomplete.
    let claimed = {
        let mut map = offers(app);
        let now = now_ms();
        if live_entry(&mut map, &match_row.id, now).is_none() {
            let generation = NEXT_GENERATION.fetch_add(1, Ordering::Relaxed);
            map.insert(
                match_row.id.clone(),
                OfferEntry {
                    offers: IndexSet::new(),
                    leans: IndexMap::new(),
                    at: now,
                    match_id: None,
                    generation,
                },
            );
        }
        let Some(entry) = map.get_mut(&match_row.id) else {
            return Err(ApiError::internal(
                "the play again offer vanished while it was being written",
            ));
        };
        entry.at = now;
        entry.offers.insert(seat);
        entry.leans.insert(seat, lean);
        if entry.match_id.is_some() {
            None
        } else if entry.offers.contains(&seat.teammate()) {
            let id = new_uuid();
            entry.match_id = Some(id.clone());
            let lean_of = |at: DuosSeat| entry.leans.get(&at).copied().flatten();
            Some((
                id,
                entry.generation,
                (lean_of(DuosSeat::A1), lean_of(DuosSeat::A2)),
            ))
        } else {
            None
        }
    };

    let Some((new_id, generation, leans)) = claimed else {
        // The pair is incomplete, or another offer already made the game: answer what the entry
        // holds. A re-offer after the game was made learns its id here.
        return Ok(offers(app)
            .get(&match_row.id)
            .and_then(|entry| entry.match_id.clone()));
    };
    if let Err(error) = create_again(app, match_row, leans, &new_id).await {
        let mut map = offers(app);
        if let Some(entry) = map.get_mut(&match_row.id)
            && entry.generation == generation
        {
            entry.match_id = None;
        }
        return Err(error);
    }
    Ok(Some(new_id))
}

/// Makes the new co-op match both seats offered: the same `a1` and `a2` (the host stays a1) and the
/// same `difficulty`, a fresh seed, frozen decks replayed and dealt seats dealt afresh. Refuses
/// before anything is made: a seat that found another game meanwhile keeps that game.
async fn create_again(
    app: &Arc<App>,
    finished: &DuosMatch,
    leans: (Option<LeanPick>, Option<LeanPick>),
    match_id: &str,
) -> Result<(), ApiError> {
    // R915005: a deck with a custom card plays only in practice, so the frozen decks a new game
    // would replay are asked the validator's L7, as the queue asks it, before anything is claimed.
    // A dealt seat is dealt afresh from the catalog, which holds no custom card.
    let mut frozen: Vec<&[String]> = Vec::new();
    if !finished.options.dealt.contains(&DuosSeat::A1) {
        frozen.push(&finished.options.decks.a1);
    }
    if !finished.options.dealt.contains(&DuosSeat::A2) {
        frozen.push(&finished.options.decks.a2);
    }
    assert_no_custom_cards(&frozen)?;

    // Refuse before making anything: a seat that found another game meanwhile keeps that game.
    // R1523000: either person in a 1v1 match, a Conquest series that is not over or a live co-op
    // match is refused with no details, since it may be the other person's (R1065000).
    assert_free_to_play(app, &finished.a1).await?;
    assert_free_to_play(app, &finished.a2).await?;

    // §13.9: to a pair with a block either way the game is over, as a room answers an unknown code.
    let blocked = one_tx!(app.db, |t| t.blocks_between(&finished.a1, &finished.a2).await?)?;
    if blocked {
        return Err(not_open());
    }

    // A seat whose deck was dealt is dealt afresh, leaning on the set its latest offer picked
    // (R924000, R2031000); a saved-deck seat replays the finished match's frozen deck. The rebuilt
    // choice carries no name, portrait or slots: `person_deck` reads only the cards.
    let choice_of = |seat: DuosSeat, lean: Option<LeanPick>, cards: &[String]| {
        if finished.options.dealt.contains(&seat) {
            DuosChoice::Random { lean }
        } else {
            DuosChoice::Deck {
                deck: FrozenDeck {
                    name: String::new(),
                    cards: cards.to_vec(),
                    portrait: None,
                    slots: None,
                },
                catalog_version: finished.options.catalog_version.clone(),
            }
        }
    };
    let host_choice = choice_of(DuosSeat::A1, leans.0, &finished.options.decks.a1);
    let guest_choice = choice_of(DuosSeat::A2, leans.1, &finished.options.decks.a2);

    // The start goes through the seat ledger, as every door into a game does (R1523000). It writes
    // the match row and starts the actor, as a room's join does.
    let outcome = start_coop_match(
        app,
        CoopStart {
            match_id: match_id.to_string(),
            a1: &finished.a1,
            a2: &finished.a2,
            host_choice: &host_choice,
            guest_choice: &guest_choice,
            difficulty: finished.difficulty,
            seed: None,
            claim_room: None,
        },
        |taken| in_game(&taken.seat),
    )
    .await?;
    match outcome {
        CoopStartOutcome::Started => Ok(()),
        // No room is ever claimed, so neither answer can come back; an internal error says so loudly.
        CoopStartOutcome::Blocked | CoopStartOutcome::Unavailable => {
            Err(ApiError::internal("a play again start claimed a room"))
        }
    }
}

// Routes

/// `POST /api/duos/matches/:matchId/again` (active): offer to play again (or meet the teammate's
/// offer). `AuthLevel::Active` is §9.4's gate, as on the rooms; a seat of another pair's match — or
/// of none — learns nothing beyond "no such co-op match" (§13.9).
///
/// It takes `&Arc<App>` because equal offers start the match and `registry::start` needs one.
pub async fn offer_again(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    // R924000: absent is no lean, so an offer from a client before the picker is a plain offer.
    let lean = lean_of(&req.body)?;
    let (match_row, seat) = callers_match(app, &req, &profile).await?;
    if match_row.ended_at.is_none() {
        return Err(not_finished());
    }
    // R1497000: a seat emptied by a deleted account cannot be played again; the remaining seat's
    // offer is refused and nothing is made.
    if match_row.a1.is_empty() || match_row.a2.is_empty() {
        return Err(ApiError::new(
            ApiErrorCode::Conflict,
            "that co-op game cannot be played again",
        ));
    }
    // R1638000: refused before the offer is recorded, so nothing is kept and no game is made.
    if match_row.options.catalog_version != app.catalog.version {
        return Err(cards_changed(app, &match_row.options.catalog_version));
    }
    let Some(ended_at) = match_row.ended_at else {
        return Err(not_finished());
    };
    let now = now_ms();
    if now - ended_at > REMATCH_OFFER_TTL_MS {
        // R1398000: the window a finished co-op match's socket reopens in has closed, so the offer
        // is refused the way the 1v1 rematch route refuses an expired offer: it lapses silently,
        // recording nothing. An entry that made a game keeps its id, so the seat that polls late
        // still learns where to go.
        let mut map = offers(app);
        let answered = match map.get(&match_row.id) {
            Some(entry) if entry.match_id.is_some() => entry.match_id.clone(),
            _ => {
                map.shift_remove(&match_row.id);
                None
            }
        };
        let created = AgainOfferBody { match_id: answered };
        return Ok(json(
            200,
            serde_json::to_value(created).map_err(|error| ApiError::internal(error.to_string()))?,
        ));
    }
    let match_id = offer_as(app, &match_row, seat, lean).await?;
    let created = AgainOfferBody { match_id };
    Ok(json(
        200,
        serde_json::to_value(created).map_err(|error| ApiError::internal(error.to_string()))?,
    ))
}

/// `GET /api/duos/matches/:matchId/again` (active): what each seat offered, whether the teammate
/// is still on the finished match, and the created game. A seat's own pair only: anyone else gets
/// the same "no such co-op match" as for a missing id (§13.9).
pub async fn again_status(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let (match_row, seat) = callers_match(app, &req, &profile).await?;
    if match_row.ended_at.is_none() {
        return Err(not_finished());
    }

    let (you_offered, teammate_offered, created) = {
        let mut map = offers(app);
        match live_entry(&mut map, &match_row.id, now_ms()) {
            None => (false, false, None),
            Some(entry) => (
                entry.offers.contains(&seat),
                entry.offers.contains(&seat.teammate()),
                entry.match_id.clone(),
            ),
        }
    };
    // Gone whenever no live actor holds the match or the teammate's socket closed, and never
    // rebuilt: presence is about who is here now.
    let teammate_here = registry(app)
        .get(&match_row.id)
        .map(|actor| {
            let presence = actor.presence();
            HUMAN_SEATS
                .iter()
                .position(|&held| held == seat.teammate())
                .is_some_and(|index| presence[index])
        })
        .unwrap_or(false);
    let status = AgainStatusBody {
        you_offered,
        teammate_offered,
        teammate_here,
        match_id: created,
    };
    Ok(json(
        200,
        serde_json::to_value(status).map_err(|error| ApiError::internal(error.to_string()))?,
    ))
}
