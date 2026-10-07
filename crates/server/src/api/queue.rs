//! Matchmaking (BUILD M7-T3, SPEC §9.5), in three modes (R257). Port of `apps/server/src/api/queue.ts`.
//!
//! §9.5, in full, is what this file implements: "Enqueue asserts the account is active and not in
//! a match, validates the loadout, freezes the chosen deck into the ticket and returns the ticket
//! id. Pairing runs on enqueue plus a sweeper every few seconds; the window widens ±50 rating
//! every 10 s from ±100 and is uncapped after 60 s; both tickets are claimed in one atomic
//! statement. The client shows the queue population instead of an endless spinner."
//!
//! R257 adds the mode: Best of 1 (one saved deck), Conquest (a trio, played as a series, R330) and
//! All Random (R258). A ticket pairs only with a ticket of its own mode; inside a mode the window
//! and R166's order are exactly as before. What a pair becomes depends on the mode: a Best-of-1
//! match on the two frozen decks, an All Random match on two dealt ones, or a Conquest series whose
//! first game waits for both players to pick (`series.rs`).
//!
//! Three pieces, in that order: the three endpoints, one pairing sweep (`try_pair`), and the
//! sweeper that reschedules it (`run_matchmaker`).
//!
//! Two invariants carry the weight:
//!
//!  - **The deck is frozen at enqueue.** §9.4: "Decks are frozen into the queue ticket." The deck
//!    (or the trio) is copied into the ticket by `freeze_choice` and nothing below re-reads a saved
//!    deck after the ticket exists, so a deck edited while queued cannot change the game that
//!    ticket becomes (M7-T3's second acceptance item, §9.8).
//!  - **Both tickets are claimed in one atomic statement**, `tickets_claim_pair`. A match or a series
//!    is created *only* after that statement returns true, so two matchers racing over the same
//!    ticket cannot pair it twice (M7-T3's race test). Losing the race is not an error: it means
//!    someone else already found that player a game.
//!
//! The rating window is `rating_window` from `config.rs`, imported rather than re-derived, so
//! "±100 widening ±50 every 10 s, uncapped after 60 s" is written down exactly once.
//!
//! The route table is `app.rs`'s (SURFACE §11.2), so TS's `createQueueRoutes()` is three handlers
//! here: `enqueue` (`POST /api/queue`, active), `dequeue` (`DELETE /api/queue`, active) and
//! `population` (`GET /api/queue/population`, user).

use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use indexmap::{IndexMap, IndexSet};
use serde_json::{Value, json};

use crate::api::decks::{FrozenChoice, ModeChoiceInput, assert_not_in_series, freeze_choice, read_mode_choice};
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, json};
use crate::api::series::start_series;
use crate::api::series_rules::{NewSeriesInput, NewSeriesSide};
use crate::app::App;
use crate::config::{MATCHMAKER_SWEEP_INTERVAL_SECONDS, rating_window};
use crate::db::store::{FrozenTrio, MatchSeat, Profile, QueueMode, SeriesRow, StartMatchInput, Ticket, TicketStatus};

/// Unit conversion, not configuration: `rating_window` speaks seconds, tickets are stamped in ms.
const MS_PER_SECOND: f64 = 1000.0;

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

/// TS `deps.ids.seed()` (`systemIds.seed`): 16 random bytes as lower-case hex. The engine is seeded
/// only from this (SPEC §9.3).
fn new_seed() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("the system's random source failed");
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn api_error(code: ApiErrorCode, message: impl Into<String>, details: Option<Value>) -> ApiError {
    ApiError { code, message: message.into(), details, retry_after_ms: None }
}

/// TS `badRequest(message)`.
fn bad_request(message: impl Into<String>) -> ApiError {
    api_error(ApiErrorCode::BadRequest, message, None)
}

/// An error that was not an `ApiError` in TS (a store failure, a thrown `Error`), carrying its own
/// sentence for the log line of whoever catches it.
fn internal(error: impl std::fmt::Display) -> ApiError {
    api_error(ApiErrorCode::Internal, error.to_string(), None)
}

/// At a handler's edge, what TS's router did with a thrown non-`ApiError`: log it as
/// `handler.threw` and answer 500 `internal` "something went wrong", never the sentence itself.
fn hide_internal(error: ApiError) -> ApiError {
    if !matches!(error.code, ApiErrorCode::Internal) {
        return error;
    }
    tracing::warn!(event = "handler.threw", message = %error.message);
    api_error(ApiErrorCode::Internal, "something went wrong", None)
}

/// TS `callerProfile(req)` (`collection.ts`): the signed-in profile a non-`none` route carries.
fn caller_profile(req: &Req) -> Result<Profile, ApiError> {
    match &req.caller {
        Some(caller) => Ok(caller.profile.clone()),
        None => Err(bad_request("this endpoint needs a signed-in profile")),
    }
}

/// A list as the JSON array a TS log line printed it as.
fn json_list<T: serde::Serialize>(items: &[T]) -> String {
    serde_json::to_string(items).unwrap_or_default()
}

/// The literal a mode is written as (`queue.enqueued`'s and `queue.paired`'s `mode`).
fn mode_name(mode: QueueMode) -> &'static str {
    match mode {
        QueueMode::Bo1 => "bo1",
        QueueMode::Bo3 => "bo3",
        QueueMode::Random => "random",
    }
}

// ---------------------------------------------------------------------------
// R143 — the end-to-end mode's optional seed
// ---------------------------------------------------------------------------

/// SPEC §11 R143: "Who chooses a match's seed. The server mints it; a client never supplies one. In
/// end-to-end mode the room and queue endpoints accept an optional seed and use it verbatim so a
/// networked spec can be seeded, and outside that mode the field is rejected. §9.3 makes
/// `(seed, log)` the truth without saying who picks the seed, and BUILD requires every spec to set
/// one, so the exception is confined to the test mode."
///
/// Rejected, not ignored: a client that sends a seed against a production server is told its
/// request was not understood, rather than silently getting a match it did not ask for.
///
/// Public because R143 names *two* endpoints, and `actor/rooms.rs` is the other one: the room
/// endpoints read the field through this same function rather than through a second copy of the
/// rule, so "accepted in end-to-end mode, rejected everywhere else" is decided in one place.
pub fn seed_override_of(app: &App, body: &Value) -> Result<Option<String>, ApiError> {
    let Some(value) = body.get("seed") else {
        return Ok(None);
    };
    if !app.env.e2e {
        return Err(bad_request("\"seed\" is only accepted by an end-to-end test server; the server mints it"));
    }
    match value.as_str() {
        Some(seed) if !seed.is_empty() => Ok(Some(seed.to_string())),
        _ => Err(bad_request("\"seed\" must be a non-empty string")),
    }
}

/// ticket id -> the seed its enqueue asked for, until the ticket is paired or cancelled.
///
/// Module state rather than a closure because `try_pair` is also run by the sweeper
/// (`run_matchmaker`), which never sees a handler's locals. Nothing is ever written here outside
/// end-to-end mode — `seed_override_of` has already refused the field by then — and the map
/// empties itself as tickets resolve, so a production process keeps it permanently empty.
///
/// Not in SPEC, and no R-row: where the seed is held is an implementation detail; R143 already
/// rules on the seed itself ("the server mints it; a client never supplies one", with the test-mode
/// exception). `Ticket` carries no seed, and adding one would put a test-mode field into the shape
/// `db/**` persists, so R143's exception is confined to this map the way the row confines it to the
/// mode. `actor/rooms.rs` holds `room code -> seed` for the same reason.
static E2E_SEED_BY_TICKET: LazyLock<Mutex<IndexMap<String, String>>> =
    LazyLock::new(|| Mutex::new(IndexMap::new()));

fn seeds() -> std::sync::MutexGuard<'static, IndexMap<String, String>> {
    E2E_SEED_BY_TICKET.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// For the R143 tests; nothing in `src/` calls it.
pub fn e2e_seed_count() -> usize {
    seeds().len()
}

fn forget_seed(ticket_id: &str) {
    seeds().shift_remove(ticket_id);
}

/// The older of two tickets first: R166's "oldest first", ties on ticket id.
fn older_first<'a>(a: &'a Ticket, b: &'a Ticket) -> (&'a Ticket, &'a Ticket) {
    let a_first = a.enqueued_at < b.enqueued_at || (a.enqueued_at == b.enqueued_at && a.id <= b.id);
    if a_first { (a, b) } else { (b, a) }
}

/// The seed for a pair, consumed. Two seeded tickets can disagree — each spec seeds its own
/// enqueue — so the older ticket's seed wins, which is the same "oldest first" tie-break `try_pair`
/// already uses, and both entries are dropped either way.
fn take_seed_for_pair(a: &Ticket, b: &Ticket) -> Option<String> {
    let (older, younger) = older_first(a, b);
    let mut map = seeds();
    let seed = map.get(&older.id).or_else(|| map.get(&younger.id)).cloned();
    map.shift_remove(&a.id);
    map.shift_remove(&b.id);
    seed
}

// ---------------------------------------------------------------------------
// Enqueue
// ---------------------------------------------------------------------------

/// §9.5's enqueue (TS's module-private `enqueue`; `enqueue` here is the route's handler). The order
/// of the checks is the order §9.5 writes them, and it matters: a player already in a match (or a
/// series, R264) or already queued is told so before their deck is validated, so a stale client gets
/// the useful error rather than a deck complaint.
///
/// `seed` is R143's: the seed this enqueue asked for, or `None`. Only ever `Some` in end-to-end mode.
async fn enqueue_ticket(
    app: &App,
    profile: &Profile,
    choice: &ModeChoiceInput,
    seed: Option<String>,
) -> Result<Ticket, ApiError> {
    // §9.5: "asserts the account is active and not in a match". `AuthLevel::Active` did the first
    // half (§9.4's gate, in the router); this is the second.
    if profile.in_match_id.is_some() {
        return Err(api_error(ApiErrorCode::AlreadyInMatch, "finish your current match first", None));
    }
    // R264: between the games of a series a profile is in no match, and still not free to queue.
    assert_not_in_series(app, &profile.id).await?;
    {
        let mut tx = app.db.begin(None).await.map_err(internal)?;
        let open = tx.tickets_open_for_profile(&profile.id).await.map_err(internal)?;
        tx.commit().await.map_err(internal)?;
        if let Some(open) = open {
            return Err(api_error(
                ApiErrorCode::AlreadyQueued,
                "you are already in the queue",
                Some(json!({ "ticketId": open.id })),
            ));
        }
    }

    // §9.4: "checked by one validator module shared by client and server, at save and again at
    // queue". R253: a Best-of-1 deck on L2, L3, L5, L6 and a trio on L1–L6, both against the
    // current catalog; All Random needs no deck at all (R258).
    let frozen = freeze_choice(app, &profile.id, choice).await?;

    let (mode, deck, portrait, trio) = match frozen {
        // §9.4, §9.5: frozen. `freeze_choice` already copied the deck or the trio; this is the copy
        // that lands in the ticket and, later, in the match or the series — the saved deck is never
        // read again. R642: the deck's portrait freezes with it.
        FrozenChoice::Bo1 { deck } => (QueueMode::Bo1, deck.cards.clone(), deck.portrait.clone(), None),
        FrozenChoice::Bo3 { trio } => (QueueMode::Bo3, Vec::new(), None, Some(trio)),
        FrozenChoice::Random => (QueueMode::Random, Vec::new(), None, None),
    };
    let ticket = Ticket {
        id: new_uuid(),
        profile_id: profile.id.clone(),
        rating: profile.rating,
        mode,
        deck,
        portrait,
        trio,
        catalog_version: app.catalog.version.clone(),
        enqueued_at: now_ms(),
        status: TicketStatus::Open,
        match_id: None,
    };

    let inserted = async {
        let mut tx = app.db.begin(None).await?;
        tx.tickets_insert(&ticket).await?;
        tx.commit().await
    }
    .await;
    if let Err(error) = inserted {
        // `tickets_profile_queued_key` (migration 0004) is the race-proof half of "not already
        // queued": two simultaneous enqueues both pass the read above and one loses here.
        let mut tx = app.db.begin(None).await.map_err(internal)?;
        let existing = tx.tickets_open_for_profile(&profile.id).await.map_err(internal)?;
        tx.commit().await.map_err(internal)?;
        if let Some(existing) = existing {
            return Err(api_error(
                ApiErrorCode::AlreadyQueued,
                "you are already in the queue",
                Some(json!({ "ticketId": existing.id })),
            ));
        }
        return Err(internal(error));
    }

    // R143, after the insert won: a seed remembered for a ticket that does not exist would never be
    // consumed and never dropped.
    if let Some(seed) = seed {
        seeds().insert(ticket.id.clone(), seed);
    }

    tracing::info!(
        event = "queue.enqueued",
        profileId = %profile.id,
        ticketId = %ticket.id,
        mode = mode_name(ticket.mode),
    );
    Ok(ticket)
}

// ---------------------------------------------------------------------------
// Pairing
// ---------------------------------------------------------------------------

/// §9.5's widening window, read for one ticket at one instant.
fn window_for(ticket: &Ticket, now: i64) -> f64 {
    let waited_seconds = (now - ticket.enqueued_at).max(0) as f64 / MS_PER_SECOND;
    rating_window(waited_seconds)
}

/// §9.5: the gap has to sit inside *both* windows, so the player who has waited longer cannot drag
/// a freshly queued opponent into a match their own window would refuse. R257: and the two tickets
/// are of one mode — a Best-of-1 player is never handed a series, nor a trio player a single game.
fn qualifies(a: &Ticket, b: &Ticket, now: i64) -> bool {
    if a.profile_id == b.profile_id {
        return false;
    }
    if a.mode != b.mode {
        return false;
    }
    let gap = (a.rating - b.rating).abs();
    gap <= window_for(a, now) && gap <= window_for(b, now)
}

/// A Conquest ticket's frozen trio; one without is a store that lost a column, not a player.
fn trio_of(ticket: &Ticket) -> Result<FrozenTrio, ApiError> {
    match &ticket.trio {
        Some(trio) => Ok(trio.clone()),
        None => Err(internal(format!("Conquest ticket {} holds no trio", ticket.id))),
    }
}

/// R263: forget the `open` skeleton `tickets_claim_pair` wrote, in its own transaction.
async fn discard_open(app: &App, match_id: &str) -> Result<(), ApiError> {
    let mut tx = app.db.begin(None).await.map_err(internal)?;
    tx.matches_discard_open(match_id).await.map_err(internal)?;
    tx.commit().await.map_err(internal)
}

/// R259: a Conquest pair becomes a series, not a match. Its first game's match id is the one
/// `tickets_claim_pair` just reserved (R263), and nobody is put in a match yet: the series opens on
/// a pick phase, and `series.rs` starts game 1 once both players have chosen a deck. Series seat p1
/// is the older ticket, who goes first in odd games.
async fn start_paired_series(app: &App, a: &Ticket, b: &Ticket, match_id: &str) -> Result<(), ApiError> {
    let (older, younger) = older_first(a, b);
    // R143 and R259: the server mints the series seed (each game's is `{seed_base}:{n}`), unless an
    // end-to-end enqueue supplied one.
    let seed_base = take_seed_for_pair(a, b).unwrap_or_else(new_seed);
    let made = async {
        let input = NewSeriesInput {
            series_id: new_uuid(),
            first_match_id: match_id.to_string(),
            sides: (
                NewSeriesSide { profile_id: older.profile_id.clone(), trio: trio_of(older)? },
                NewSeriesSide { profile_id: younger.profile_id.clone(), trio: trio_of(younger)? },
            ),
            seed_base,
            catalog_version: app.catalog.version.clone(),
            // R604: a series the queue pairs is ranked.
            ranked: true,
        };
        // One transaction, so the series row and `start_series`'s stale-ticket cancels land together
        // or not at all: a 'picking' row that half-landed would hold both players out of the queue
        // (assert_not_in_series) until the pick deadline ran it out (R333).
        let mut tx = app.db.begin(None).await.map_err(internal)?;
        let series = start_series(app, &input, &mut tx).await.map_err(internal)?;
        tx.commit().await.map_err(internal)?;
        Ok::<SeriesRow, ApiError>(series)
    }
    .await;
    let series = match made {
        Ok(series) => series,
        Err(error) => {
            // `start_paired_match`'s cleanup, for a pair that becomes a series rather than a match:
            // the claimed tickets stay claimed either way (the pair is lost), but the `open` skeleton
            // `tickets_claim_pair` wrote points at nothing now — a series sets no in-match flag, so
            // nothing else references it — and nothing reaps `open` rows. Delete it or it stays
            // forever.
            if let Err(cleanup) = discard_open(app, match_id).await {
                tracing::error!(event = "queue.pair_cleanup_failed", matchId = %match_id, message = %cleanup.message);
            }
            return Err(error);
        }
    };
    tracing::info!(
        event = "queue.paired",
        mode = "bo3",
        seriesId = %series.id,
        firstMatchId = %match_id,
        tickets = %json_list(&[older.id.clone(), younger.id.clone()]),
        ratings = %json_list(&[older.rating, younger.rating]),
    );
    Ok(())
}

/// The All Random deck R258 deals one seat from the match seed (`{seed}:p1-deck`, `{seed}:p2-deck`).
fn dealt_deck(seed: &str, seat: &str) -> Vec<String> {
    crate::actor::engine::deal_random_deck(&format!("{seed}:{seat}-deck"))
}

/// All Random's portrait for one seat (R642), dealt from the seed like its deck.
fn dealt_portrait(seed: &str, seat: &str) -> Option<String> {
    Some(jackioh_engine::wire::pick_portrait_from_seed(&format!("{seed}:portrait:{seat}")).to_string())
}

/// Creates the paired match — or, for Conquest, the series. Called only with two tickets this
/// process has already claimed, which is what makes it safe to write: the claim is the mutual
/// exclusion.
///
/// Best of 1 plays the two decks the tickets froze. All Random (R258) deals both from the match
/// seed and the seat, `{seed}:p1-deck` and `{seed}:p2-deck`, and the dealt decks go into the match
/// row like any frozen deck, so `(seed, decks, log)` replays it as ever. Portraits ride the same
/// way (R642): the ticket's own for Best of 1, a uniform pick dealt from the seed for All Random.
async fn start_paired_match(app: &Arc<App>, a: &Ticket, b: &Ticket, match_id: &str) -> Result<(), ApiError> {
    if a.mode == QueueMode::Bo3 {
        return start_paired_series(app, a, b, match_id).await;
    }
    // R143: the server mints the seed, unless an end-to-end enqueue supplied one.
    let seed = take_seed_for_pair(a, b).unwrap_or_else(new_seed);
    let random = a.mode == QueueMode::Random;
    let seats = (
        MatchSeat {
            profile_id: a.profile_id.clone(),
            player: jackioh_engine::PlayerId::P1,
            deck: if random { dealt_deck(&seed, "p1") } else { a.deck.clone() },
            portrait: if random { dealt_portrait(&seed, "p1") } else { a.portrait.clone() },
        },
        MatchSeat {
            profile_id: b.profile_id.clone(),
            player: jackioh_engine::PlayerId::P2,
            deck: if random { dealt_deck(&seed, "p2") } else { b.deck.clone() },
            portrait: if random { dealt_portrait(&seed, "p2") } else { b.portrait.clone() },
        },
    );

    // NO `matches_create` HERE. `Registry::start` builds the row -- seed, both frozen decks,
    // R79's clocks, status live -- and calls `matches_create` itself (registry.rs), so a create here
    // made it TWICE for one id. The second call found the row already `live` and `matches_create`
    // refuses that by contract ("anything else -> the id is taken"), so the SECOND player's enqueue
    // returned 500 after the match had already been made, leaving both players in a match no client
    // had been given the id of.
    //
    // The room path never had this bug and shows the shape that works: `rooms_claim` writes the
    // `open` row and the registry's create promotes it to `live` -- one create, one promotion.
    // `tickets_claim_pair` writes the same `open` skeleton for a pair, so the queue now behaves
    // identically.
    //
    // What stays in a transaction is the pair of `set_in_match` writes, which is what §9.5 means by
    // in-match state; `profiles.current_match_id` is a foreign key into `matches` and the `open`
    // row `tickets_claim_pair` wrote is what satisfies it.
    {
        let mut tx = app.db.begin(None).await.map_err(internal)?;
        // §9.5: in-match state is set here and cleared by `results.rs` at every ending.
        tx.profiles_set_in_match(&a.profile_id, Some(match_id)).await.map_err(internal)?;
        tx.profiles_set_in_match(&b.profile_id, Some(match_id)).await.map_err(internal)?;
        tx.commit().await.map_err(internal)?;
    }

    // R604: a match the queue pairs is ranked.
    let started = app
        .matches
        .start(
            app,
            StartMatchInput {
                match_id: match_id.to_string(),
                seed,
                catalog_version: app.catalog.version.clone(),
                ranked: true,
                seats,
                mode: None,
                stake: None,
            },
        )
        .await;
    if let Err(error) = started {
        // The `set_in_match` transaction above has already committed, so a failed start leaves the
        // `open` skeleton `tickets_claim_pair` wrote with both profiles pointing at it — and nothing
        // reaps `open` rows, so the two would stay locked out of the queue, rooms and account
        // deletion forever. Undo it before the error stands: the tickets are claimed either way (the
        // pair is lost), but the players must come free.
        let cleanup = async {
            let mut tx = app.db.begin(None).await.map_err(internal)?;
            tx.profiles_set_in_match(&a.profile_id, None).await.map_err(internal)?;
            tx.profiles_set_in_match(&b.profile_id, None).await.map_err(internal)?;
            tx.commit().await.map_err(internal)?;
            discard_open(app, match_id).await
        }
        .await;
        if let Err(cleanup) = cleanup {
            tracing::error!(event = "queue.pair_cleanup_failed", matchId = %match_id, message = %cleanup.message);
        }
        return Err(error);
    }
    tracing::info!(
        event = "queue.paired",
        mode = mode_name(a.mode),
        matchId = %match_id,
        tickets = %json_list(&[a.id.clone(), b.id.clone()]),
        ratings = %json_list(&[a.rating, b.rating]),
    );
    Ok(())
}

/// One pairing sweep. Oldest ticket first, and for each of them the oldest qualifying opponent, so
/// the pair that has waited longest is made first.
///
/// NOT IN SPEC, and no R-row yet — PROPOSED RULING for §11:
///   Topic: Which qualifying opponent a sweep pairs
///   Ruling: The oldest ticket is paired first, and against the oldest opponent its window admits —
///     not the closest in rating. §9.5 fixes the window (±100 rating, widening ±50 every 10 s,
///     uncapped after 60 s) and says nothing about the choice inside it, and the window is already
///     the rating rule: picking the closest rating inside a window that was widened precisely
///     because nobody closer was there re-applies the same criterion twice and leaves the player
///     who has waited longest waiting again. Wait time is also the one thing a queued player can
///     watch going up, which §9.5's "queue population instead of an endless spinner" is about. Ties
///     break on ticket id, so a sweep is deterministic and a replay of the same open set pairs the
///     same way.
///   Affects: §9.5, R108; `api/queue.rs`.
///
/// Answers how many matches this sweep made.
pub async fn try_pair(app: &Arc<App>) -> Result<usize, ApiError> {
    let now = now_ms();
    let mut tx = app.db.begin(None).await.map_err(internal)?;
    let mut open: Vec<Ticket> = tx.tickets_list_open().await.map_err(internal)?;
    open.sort_by(|x, y| x.enqueued_at.cmp(&y.enqueued_at).then_with(|| x.id.cmp(&y.id)));
    if open.len() < 2 {
        tx.commit().await.map_err(internal)?;
        return Ok(0);
    }

    // §9.5's "not in a match" holds at pairing too, not only at enqueue: a ticket left open while
    // its owner joined a room match must not become a second match for them — nor, R264, while its
    // owner is in a series that a room join made. One read of each for the whole sweep, and `taken`
    // covers the pairs this sweep makes as it goes.
    let owners: Vec<String> = open.iter().map(|ticket| ticket.profile_id.clone()).collect();
    let mut busy: IndexSet<String> = tx
        .profiles_get_many(&owners)
        .await
        .map_err(internal)?
        .into_iter()
        .filter(|profile| profile.in_match_id.is_some())
        .map(|profile| profile.id)
        .collect();
    for series in tx.series_active().await.map_err(internal)? {
        busy.insert(series.sides.0.profile_id.clone());
        busy.insert(series.sides.1.profile_id.clone());
    }
    tx.commit().await.map_err(internal)?;
    let mut taken: IndexSet<String> =
        open.iter().filter(|ticket| busy.contains(&ticket.profile_id)).map(|ticket| ticket.id.clone()).collect();
    let mut made = 0;

    for a in &open {
        if taken.contains(&a.id) {
            continue;
        }
        for b in &open {
            if b.id == a.id || taken.contains(&b.id) {
                continue;
            }
            if !qualifies(a, b, now) {
                continue;
            }

            let match_id = new_uuid();
            // §9.5: "both tickets are claimed in one atomic statement". Everything before this line is
            // a guess; only a `true` here gives this process the right to create a match.
            let won = {
                let mut tx = app.db.begin(None).await.map_err(internal)?;
                let won = tx.tickets_claim_pair(&a.id, &b.id, &match_id, now).await.map_err(internal)?;
                tx.commit().await.map_err(internal)?;
                won
            };
            if !won {
                // Another matcher got one of them. Never create a match for a ticket we did not claim:
                // find out which one is gone and either try another opponent or drop this ticket.
                let still = {
                    let mut tx = app.db.begin(None).await.map_err(internal)?;
                    let still = tx.tickets_get(&a.id).await.map_err(internal)?;
                    tx.commit().await.map_err(internal)?;
                    still
                };
                if still.is_none_or(|ticket| ticket.status != TicketStatus::Open) {
                    taken.insert(a.id.clone());
                    break;
                }
                taken.insert(b.id.clone());
                continue;
            }

            taken.insert(a.id.clone());
            taken.insert(b.id.clone());
            start_paired_match(app, a, b, &match_id).await?;
            made += 1;
            break;
        }
    }

    Ok(made)
}

/// §9.5: "Pairing runs on enqueue plus a sweeper every few seconds" (R108: every 3 s,
/// `MATCHMAKER_SWEEP_INTERVAL_SECONDS`). TS's `startMatchmaker(deps)` rescheduled it through the
/// timers port and answered a `stop`; here `app.rs` spawns this loop (SURFACE §11.2) and aborting the
/// task is the stop. Each sweep starts one interval after the last one finished, on tokio's clock, so
/// a test drives it with `tokio::time::advance`.
pub async fn run_matchmaker(app: Arc<App>) {
    loop {
        tokio::time::sleep(Duration::from_secs(MATCHMAKER_SWEEP_INTERVAL_SECONDS as u64)).await;
        if let Err(error) = try_pair(&app).await {
            // A failed sweep must not kill the sweeper: the next one may well succeed.
            tracing::warn!(event = "matchmaker.sweep_failed", message = %error.message);
        }
    }
}

// ---------------------------------------------------------------------------
// Routes
// ---------------------------------------------------------------------------

/// `POST /api/queue` (active): §9.5's enqueue. `AuthLevel::Active` is §9.4's gate ("A pending
/// account ... nothing else: no collection, loadout, queue or match"), so a pending account gets 403
/// here without this handler saying anything about it.
///
/// Pairing is attempted inline ("Pairing runs on enqueue plus a sweeper"), which is why the
/// response reports the ticket's status: a player who paired immediately learns it from the
/// same round trip instead of waiting for a sweep.
///
/// It takes `&Arc<App>` where SURFACE §11.2 writes `&App`: pairing inline starts the match, and
/// `Registry::start` takes `&Arc<App>` (the actor task owns a handle on the app).
pub async fn enqueue(app: &Arc<App>, req: Req) -> ApiResult {
    enqueue_route(app, req).await.map_err(hide_internal)
}

async fn enqueue_route(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    // R257: the mode and the deck or trio.
    let choice = read_mode_choice(&req.body)?;
    // R143: an optional `seed`, accepted only by an end-to-end test server and rejected — never
    // ignored — anywhere else.
    let seed = seed_override_of(app, &req.body)?;
    let ticket = enqueue_ticket(app, &profile, &choice, seed).await?;
    try_pair(app).await?;

    let mut tx = app.db.begin(None).await.map_err(internal)?;
    let current = tx.tickets_get(&ticket.id).await.map_err(internal)?;
    let status = current.as_ref().map(|current| current.status).unwrap_or(ticket.status);
    // A paired Conquest ticket's `match_id` is game 1's reserved id, which is not a match anyone
    // can open yet: the player goes to the series to pick a deck, so it answers with that.
    let paired = status == TicketStatus::Matched;
    let series_id = if paired && ticket.mode == QueueMode::Bo3 {
        tx.series_active_for(&profile.id).await.map_err(internal)?.map(|series| series.id)
    } else {
        None
    };
    let match_id = if paired && ticket.mode != QueueMode::Bo3 {
        current.as_ref().and_then(|current| current.match_id.clone())
    } else {
        None
    };
    let population = tx.tickets_count_open().await.map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(json(
        200,
        json!({
            "ticketId": ticket.id,
            "status": status,
            "matchId": match_id,
            "seriesId": series_id,
            "population": population,
            "mode": ticket.mode,
        }),
    ))
}

/// `DELETE /api/queue` (active): leaving the queue. Idempotent; `cancelled: false` when there was
/// nothing open to cancel.
///
/// NOT IN SPEC, and no R-row yet — PROPOSED RULING for §11:
///   Topic: How a player leaves the queue
///   Ruling: A queued player may cancel, and cancelling is idempotent: a client that cancels
///     twice, or whose ticket was paired a moment earlier, is told nothing was cancelled rather
///     than given an error it cannot act on. §9.5 describes enqueue and pairing and never says
///     how a player leaves, but its own enqueue assertion — the account is active "and not in a
///     match" — has no other way to become satisfiable again: without cancel a player who
///     queued by mistake is held until somebody pairs with them. It must be idempotent because
///     the race is unavoidable and one-sided: the sweeper (R108) can pair a ticket between the
///     client deciding to cancel and the request arriving, and at that point the match exists
///     and the player belongs in it. So cancel never unmakes a pairing; it only closes a ticket
///     that is still open.
///   Affects: §9.5, R108, R143; `api/queue.rs`, migration `0004_matches.sql`.
///
/// `tickets.status = 'cancelled'` in migration 0004 is what this writes.
pub async fn dequeue(app: &App, req: Req) -> ApiResult {
    dequeue_route(app, req).await.map_err(hide_internal)
}

async fn dequeue_route(app: &App, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let mut tx = app.db.begin(None).await.map_err(internal)?;
    let Some(ticket) = tx.tickets_open_for_profile(&profile.id).await.map_err(internal)? else {
        tx.commit().await.map_err(internal)?;
        return Ok(json(200, json!({ "cancelled": false })));
    };
    tx.tickets_cancel(&ticket.id, now_ms()).await.map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    // R143: a cancelled ticket will never be paired, so its seed is dropped with it.
    forget_seed(&ticket.id);
    tracing::info!(event = "queue.cancelled", profileId = %profile.id, ticketId = %ticket.id);
    Ok(json(200, json!({ "cancelled": true, "ticketId": ticket.id })))
}

/// `GET /api/queue/population` (user): §9.5: "The client shows the queue population instead of an
/// endless spinner."
///
/// `AuthLevel::User` rather than `Active` or `None`. The count is one aggregate about the server,
/// not about any profile: it names nobody, and §9.4's "no queue" for a pending account is about
/// joining the queue and being matched, which `POST /api/queue` still refuses. Making it `None`
/// would hand an anonymous caller a free live-traffic feed, and requiring a token costs the lobby
/// nothing — it has one either way, since a player reaches this screen signed in. If a later review
/// reads §9.4's gate as covering even the count, this becomes `Active` and nothing else changes.
///
/// R257: the total, and per mode, so the lobby can say how many are waiting for each.
pub async fn population(app: &App, _req: Req) -> ApiResult {
    population_route(app).await.map_err(hide_internal)
}

async fn population_route(app: &App) -> ApiResult {
    let mut tx = app.db.begin(None).await.map_err(internal)?;
    let population = tx.tickets_count_open().await.map_err(internal)?;
    let by_mode = tx.tickets_count_open_by_mode().await.map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(json(200, json!({ "population": population, "byMode": by_mode })))
}
