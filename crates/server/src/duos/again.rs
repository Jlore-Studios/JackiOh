//! Play again after a finished co-op match (SPEC §13.9, R2092000), modeled on 1v1 rematch (R672).
//! `POST /api/duos/matches/:matchId/again` records offers; `GET` checks status (CLAUDE.md rule 7).
//! The new match keeps `a1`/`a2` (§13.9), `difficulty`, AI team rules/handicaps (§13.7) and
//! catalog version (R767000). Saved decks replay; dealt decks re-deal with lean (R924000, R2031000).
//!
//! Only seats of finished matches offer. Deleted accounts (R1497000) and other catalog versions
//! (R1638000) are refused. Offers live in memory and lapse after `REMATCH_OFFER_TTL_MS` (§13.9, R1398000).
//! Custom cards (R915005) and players already in matches (R1065000, R1523000) refuse creation.

use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};

use indexmap::{IndexMap, IndexSet};
use jackioh_engine::LeanPick;
use jackioh_engine::wire::DuosSeat;
use serde::{Deserialize, Serialize};
use tokio::sync::Notify;
use tokio::sync::futures::OwnedNotified;

use crate::actor::contracts::one_tx;
use crate::api::collection::caller_profile;
use crate::api::decks::{already_in_match, assert_free_to_play, assert_no_custom_cards, lean_of};
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, json};
use crate::api::queue::new_uuid;
use crate::api::rematch::cards_changed;
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
    /// The game both seats' offers made, once made — and only once made, so no offer ever reads
    /// an id for a game whose create then fails.
    match_id: Option<String>,
    /// The create in flight, while one offer makes the game: an offer landing meanwhile waits on
    /// it instead of minting a second game. Its `Creating` guard clears it however the create ends.
    creating: Option<Arc<Notify>>,
}

/// R2092000's offers, by the finished match's id: rendezvous state for two sockets, not rows. One
/// per `App` (`App::duos_again_offers`), like `RematchOffers`, so parallel tests never share offers
/// and one test's sweep never reaches another's.
#[derive(Default)]
pub struct DuosAgainOffers(Mutex<IndexMap<String, OfferEntry>>);

/// What an offer does next, decided under the offers' lock (`DuosAgainOffers::claim`).
enum Claim<'a> {
    /// Answer now: the pair is incomplete, or the game is already made.
    Done(Option<String>),
    /// This offer makes the game, holding the claim while it does.
    Make {
        creating: Creating<'a>,
        leans: (Option<LeanPick>, Option<LeanPick>),
    },
    /// Another offer's create is in flight: wait until it lands or is let go, then read again.
    Wait(Pin<Box<OwnedNotified>>),
}

/// Holds the right to make the game both seats offered. Dropping it on publish, refusal, or cancel
/// frees the claim and wakes waiters.
struct Creating<'a> {
    offers: &'a DuosAgainOffers,
    finished: String,
    mine: Arc<Notify>,
}

impl Drop for Creating<'_> {
    fn drop(&mut self) {
        let mut map = self.offers.lock();
        if let Some(entry) = map.get_mut(&self.finished)
            && self.holds(entry)
        {
            entry.creating = None;
        }
        drop(map);
        self.mine.notify_waiters();
    }
}

impl Creating<'_> {
    /// Whether `entry` still waits on this claim's create: a swept and remade entry does not.
    fn holds(&self, entry: &OfferEntry) -> bool {
        entry
            .creating
            .as_ref()
            .is_some_and(|held| Arc::ptr_eq(held, &self.mine))
    }

    /// Publishes the created game id and wakes waiters; remakes entry if swept concurrently.
    fn made(self, id: &str, now: i64) {
        let mut map = self.offers.lock();
        match map.get_mut(&self.finished) {
            Some(entry) if self.holds(entry) => {
                entry.match_id = Some(id.to_string());
            }
            _ => {
                map.insert(
                    self.finished.clone(),
                    OfferEntry {
                        offers: [DuosSeat::A1, DuosSeat::A2].into_iter().collect(),
                        leans: IndexMap::new(),
                        at: now,
                        match_id: Some(id.to_string()),
                        creating: None,
                    },
                );
            }
        }
        drop(map);
    }
}

impl DuosAgainOffers {
    fn lock(&self) -> MutexGuard<'_, IndexMap<String, OfferEntry>> {
        self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Records `seat`'s offer and decides next action; check and claim run under one lock.
    fn claim(&self, finished: &str, seat: DuosSeat, lean: Option<LeanPick>, now: i64) -> Claim<'_> {
        let mut map = self.lock();
        // Sweeps a stale entry, so the offer starts a fresh one.
        live_entry(&mut map, finished, now);
        let entry = map.entry(finished.to_string()).or_insert_with(|| OfferEntry {
            offers: IndexSet::new(),
            leans: IndexMap::new(),
            at: now,
            match_id: None,
            creating: None,
        });
        entry.at = now;
        entry.offers.insert(seat);
        entry.leans.insert(seat, lean);
        if let Some(created) = &entry.match_id {
            return Claim::Done(Some(created.clone()));
        }
        if !entry.offers.contains(&seat.teammate()) {
            return Claim::Done(None);
        }
        if let Some(inflight) = &entry.creating {
            // Registered under the lock, which `Creating::drop` takes before it wakes anyone, so a
            // create that ends in between cannot be missed.
            let mut wait = Box::pin(Arc::clone(inflight).notified_owned());
            wait.as_mut().enable();
            return Claim::Wait(wait);
        }
        let mine = Arc::new(Notify::new());
        entry.creating = Some(Arc::clone(&mine));
        let lean_at = |at: DuosSeat| entry.leans.get(&at).copied().flatten();
        let leans = (lean_at(DuosSeat::A1), lean_at(DuosSeat::A2));
        Claim::Make {
            creating: Creating {
                offers: self,
                finished: finished.to_string(),
                mine,
            },
            leans,
        }
    }
}

fn offers(app: &App) -> MutexGuard<'_, IndexMap<String, OfferEntry>> {
    app.duos_again_offers.lock()
}

/// Drops offer entries older than `REMATCH_OFFER_TTL_MS` with no active actor in registry.
/// Returns count of dropped entries.
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

// Offers

/// Returns the live entry for a match, sweeping unanswered offers past `REMATCH_OFFER_TTL_MS`.
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

/// Records the offer and makes the new match once both offer. In-flight creates cause waiters to
/// wait on notification; failed creates release the claim for another attempt.
async fn offer_as(
    app: &Arc<App>,
    match_row: &DuosMatch,
    seat: DuosSeat,
    lean: Option<LeanPick>,
) -> Result<Option<String>, ApiError> {
    loop {
        match app.duos_again_offers.claim(&match_row.id, seat, lean, now_ms()) {
            // The pair is incomplete, or another offer already made the game: answer what the
            // entry holds. A re-offer after the game was made learns its id here.
            Claim::Done(created) => return Ok(created),
            // The create in flight was published, refused or let go: read again.
            Claim::Wait(wait) => wait.await,
            Claim::Make { creating, leans } => {
                let id = new_uuid();
                // A refusal returns here, and the dropped claim lets the seats re-offer.
                create_again(app, match_row, leans, &id).await?;
                creating.made(&id, now_ms());
                return Ok(Some(id));
            }
        }
    }
}

/// Creates the new co-op match: same seats and difficulty, fresh seed, replaying or re-dealing decks.
async fn create_again(
    app: &Arc<App>,
    finished: &DuosMatch,
    leans: (Option<LeanPick>, Option<LeanPick>),
    match_id: &str,
) -> Result<(), ApiError> {
    // R915005: validates that replayed frozen decks contain no custom cards.
    let mut frozen: Vec<&[String]> = Vec::new();
    if !finished.options.dealt.contains(&DuosSeat::A1) {
        frozen.push(&finished.options.decks.a1);
    }
    if !finished.options.dealt.contains(&DuosSeat::A2) {
        frozen.push(&finished.options.decks.a2);
    }
    assert_no_custom_cards(&frozen)?;

    // R1523000, R1065000: refuse without details if either player is already in a match.
    assert_free_to_play(app, &finished.a1)
        .await
        .map_err(|_| already_in_match())?;
    assert_free_to_play(app, &finished.a2)
        .await
        .map_err(|_| already_in_match())?;

    // §13.9: to a pair with a block either way the game is over, as a room answers an unknown code.
    let blocked = one_tx!(app.db, |t| t.blocks_between(&finished.a1, &finished.a2).await?)?;
    if blocked {
        return Err(not_open());
    }

    // Dealt seats re-deal with latest lean (R924000, R2031000); saved decks replay frozen deck.
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

    // Starts through seat ledger (R1523000), writing match row and starting actor.
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
        |_| already_in_match(),
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

/// `POST /api/duos/matches/:matchId/again`: offer to play again (§9.4, §13.9).
/// Starts match when both seats offer.
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
    // R1638000: a match played on a catalog version other than the server's offers no Play again, as
    // the co-op room's join refuses a stale catalog (§13.9): a new game would deal today's cards under
    // the old rules. It is `409 update_required` before the offer is recorded, so nothing is kept and
    // no game is made.
    if match_row.options.catalog_version != app.catalog.version {
        return Err(cards_changed(app, &match_row.options.catalog_version));
    }
    let Some(ended_at) = match_row.ended_at else {
        return Err(not_finished());
    };
    let now = now_ms();
    if now - ended_at > REMATCH_OFFER_TTL_MS {
        // R1398000: offer lapses silently past reopen window; keep match id if already created.
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

/// `GET /api/duos/matches/:matchId/again`: offer and teammate presence status (§13.9).
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    const FINISHED: &str = "finished-match";

    /// `a2`'s offer completes the pair and claims the create; `a1`'s re-offer meanwhile waits on it.
    fn claimed(offers: &DuosAgainOffers) -> (Creating<'_>, Pin<Box<OwnedNotified>>) {
        assert!(matches!(
            offers.claim(FINISHED, DuosSeat::A1, None, 0),
            Claim::Done(None)
        ));
        let Claim::Make { creating, .. } = offers.claim(FINISHED, DuosSeat::A2, None, 0) else {
            panic!("the offer that completes the pair makes the game");
        };
        let Claim::Wait(wait) = offers.claim(FINISHED, DuosSeat::A1, None, 0) else {
            panic!("an offer landing mid-create waits on it");
        };
        (creating, wait)
    }

    #[tokio::test(start_paused = true)]
    async fn a_create_whose_request_is_dropped_lets_its_claim_go_and_wakes_its_waiters() {
        let offers = DuosAgainOffers::default();
        let (creating, wait) = claimed(&offers);
        // The request making the game is dropped mid-create (its client hung up): so is its claim.
        drop(creating);
        tokio::time::timeout(Duration::from_secs(1), wait)
            .await
            .expect("the waiting offer wakes");
        // The woken offer reads again and makes the game itself, instead of waiting for ever.
        assert!(matches!(
            offers.claim(FINISHED, DuosSeat::A1, None, 0),
            Claim::Make { .. }
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn a_waiting_offer_reads_the_game_the_create_made() {
        let offers = DuosAgainOffers::default();
        let (creating, wait) = claimed(&offers);
        creating.made("new-match", 0);
        tokio::time::timeout(Duration::from_secs(1), wait)
            .await
            .expect("the waiting offer wakes");
        assert!(matches!(
            offers.claim(FINISHED, DuosSeat::A1, None, 0),
            Claim::Done(Some(id)) if id == "new-match"
        ));
    }
}
