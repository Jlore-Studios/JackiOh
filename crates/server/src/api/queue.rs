//! Matchmaking (BUILD M7-T3, SPEC §9.5) in three modes (R257): Best of 1, Conquest (a trio played
//! as a series, R330, `series.rs`) and All Random (R258); a ticket pairs only within its mode.
//! Pairing runs on enqueue plus a sweeper, the window widening every 10 s up to `MAX_FAIR_GAP`
//! (R640001), in R166's order. R1453: with `LADDER_BOTS=on`, a ticket past its fallback time with no
//! person to pair with, after every pair of people the sweep could make, gets a ladder bot (for
//! Conquest a series, R641000); R1555000: a queue holding no ticket kicks off one match between two
//! grinding bots.
//!
//! Two invariants. The deck is frozen at enqueue (§9.4), so an edit while queued cannot change the
//! game (§9.8). Both tickets are claimed in one atomic statement, `tickets_claim_pair`, before a
//! match or series is made, so racing matchers cannot pair a ticket twice. Routes (SURFACE §11.2):
//! `POST`/`DELETE /api/queue` and `GET /api/queue/population`, the readout (R640002).

use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use indexmap::{IndexMap, IndexSet};
use serde_json::{Value, json};

use jackioh_engine::LeanPick;

use crate::actor::contracts::one_tx;
use crate::api::collection::caller_profile;
use crate::api::decks::{
    FrozenChoice, ModeChoiceInput, already_in_match, assert_free_to_play, freeze_choice, read_mode_choice,
};
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, bad_request, json};
use crate::api::series::start_series;
use crate::api::series_rules::{NewSeriesInput, NewSeriesSide};
use crate::app::{App, now_ms};
use crate::bots::fallback::{
    bot_match_gap_passed, bot_match_gap_since, bot_matches, choose_bot, choose_bot_pair, fallback_due,
    hour_of,
};
use crate::config::{
    BOT_RECENT_OPPONENTS, BOT_VS_BOT_MAX_LIVE, MATCHMAKER_SWEEP_INTERVAL_SECONDS, MS_PER_SECOND,
    QUEUE_READOUT_CACHE_TTL_SECONDS, QUEUE_READOUT_WINDOW_SECONDS, QUEUE_TICKET_STALE_MS,
    QUEUE_WAIT_MIN_SAMPLES, QUEUE_WAIT_QUANTILE, QUEUE_WAIT_STEP_SECONDS, SEAT_COIN_LABEL, rating_window,
};
use crate::db::store::{
    BotRow, FrozenTrio, MatchSeat, PerMode, Profile, QueueMode, SeriesRow, StartMatchInput, Ticket,
    TicketStatus,
};
use crate::seats::{Lease, Seat};

// Small private copies (fullsend rule 5): the clock, the id minters and the error shapes

/// A random v4 id.
pub(crate) fn new_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// 16 random bytes as lower-case hex. The engine is seeded only from this (SPEC §9.3).
pub(crate) fn new_seed() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("the system's random source failed");
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// A list as the JSON array a log line prints.
pub(crate) fn json_list<T: serde::Serialize>(items: &[T]) -> String {
    serde_json::to_string(items).unwrap_or_default()
}

/// SPEC §11 R143: a client never supplies a seed, except in end-to-end mode, where the room and queue
/// endpoints accept one; elsewhere it is rejected, not ignored. Public so `actor/rooms.rs` shares the rule.
pub fn seed_override_of(app: &App, body: &Value) -> Result<Option<String>, ApiError> {
    let Some(value) = body.get("seed") else {
        return Ok(None);
    };
    if !app.env.e2e {
        return Err(bad_request(
            "\"seed\" is only accepted by an end-to-end test server; the server mints it",
        ));
    }
    match value.as_str() {
        Some(seed) if !seed.is_empty() => Ok(Some(seed.to_string())),
        _ => Err(bad_request("\"seed\" must be a non-empty string")),
    }
}

/// Ticket id -> the seed its enqueue asked for (R143). Module state because the sweeper runs `try_pair`
/// too; kept off `Ticket`, whose shape `db/**` persists. `actor/rooms.rs` holds `room code -> seed` alike.
static E2E_SEED_BY_TICKET: LazyLock<Mutex<IndexMap<String, String>>> =
    LazyLock::new(|| Mutex::new(IndexMap::new()));

fn seeds() -> std::sync::MutexGuard<'static, IndexMap<String, String>> {
    E2E_SEED_BY_TICKET
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// For the R143 tests; nothing in `src/` calls it.
pub fn e2e_seed_count() -> usize {
    seeds().len()
}

fn forget_seed(ticket_id: &str) {
    seeds().shift_remove(ticket_id);
}

/// SPEC §11 R1989000: a networked spec pins its seats as it pins the seed (R143), `seat: "p1"` or
/// `"p2"`, refused outside end-to-end mode. A honored pin skips the seed's coin; in a conflict the
/// precedence-first profile (the older ticket, the host) is pinned.
pub fn seat_override_of(app: &App, body: &Value) -> Result<Option<jackioh_engine::PlayerId>, ApiError> {
    use jackioh_engine::PlayerId;
    let Some(value) = body.get("seat") else {
        return Ok(None);
    };
    if !app.env.e2e {
        return Err(bad_request(
            "\"seat\" is only accepted by an end-to-end test server; the seats come from the match seed",
        ));
    }
    match value.as_str() {
        Some("p1") => Ok(Some(PlayerId::P1)),
        Some("p2") => Ok(Some(PlayerId::P2)),
        _ => Err(bad_request("\"seat\" must be \"p1\" or \"p2\"")),
    }
}

/// Ticket id -> the seat its enqueue asked for (R1989000), kept off `Ticket` as the seed is.
static E2E_SEAT_BY_TICKET: LazyLock<Mutex<IndexMap<String, jackioh_engine::PlayerId>>> =
    LazyLock::new(|| Mutex::new(IndexMap::new()));

fn seat_pins() -> std::sync::MutexGuard<'static, IndexMap<String, jackioh_engine::PlayerId>> {
    E2E_SEAT_BY_TICKET
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// For the R1989000 tests; nothing in `src/` calls it.
pub fn e2e_seat_count() -> usize {
    seat_pins().len()
}

fn forget_seat(ticket_id: &str) {
    seat_pins().shift_remove(ticket_id);
}

/// SPEC §11 R1989000: one draw of the seed on `{seed}:seats` at cursor 0; true seats the
/// precedence-first side (the older ticket, the host, the person a bot pairs with, a kickoff's
/// first grinder) as Player 1. Only the seed goes in, so a replay folds unchanged.
pub fn seat_coin_p1_first(seed: &str) -> bool {
    jackioh_engine::Rng::new(&format!("{seed}:{SEAT_COIN_LABEL}"), 0).int(2) == 0
}

/// R1989000: the seat pin for a pair of tickets, consumed. The older ticket's pin wins, mirroring
/// `take_seed_for_pair`; a pin for the younger alone hands the older the free seat. Answers whether the
/// older ticket takes P1, `None` when neither asked, and drops both entries either way.
fn take_seat_pin_for_pair(a: &Ticket, b: &Ticket) -> Option<bool> {
    use jackioh_engine::PlayerId;
    let (older, younger) = older_first(a, b);
    let mut map = seat_pins();
    let older_pin = map.get(&older.id).copied();
    let pin = older_pin.or_else(|| map.get(&younger.id).copied());
    map.shift_remove(&a.id);
    map.shift_remove(&b.id);
    pin.map(|seat| {
        if older_pin.is_some() {
            seat == PlayerId::P1
        } else {
            // The younger asked alone: it takes its pin, so the older takes P1 exactly when the
            // younger takes P2.
            seat == PlayerId::P2
        }
    })
}

/// SPEC §11 R897000: when each profile with an open ticket was last heard from, in epoch ms. Every
/// signed-in request refreshes its caller, so a silent profile's ticket expires before it can seat
/// anyone. One per `App`, not module state: profile ids repeat across parallel tests.
pub struct QueuePresence {
    born: i64,
    heard: Mutex<IndexMap<String, i64>>,
}

impl QueuePresence {
    /// This server's start, on its clock: nothing expires before it has listened one window.
    pub fn new() -> QueuePresence {
        QueuePresence {
            born: now_ms(),
            heard: Mutex::new(IndexMap::new()),
        }
    }

    /// The caller was heard from now: its ticket stays pairable.
    pub fn touch(&self, profile_id: &str, now: i64) {
        self.heard
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(profile_id.to_string(), now);
    }

    /// The ticket is gone (paired, canceled, left or expired): its entry goes with it.
    pub fn forget(&self, profile_id: &str) {
        self.heard
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .shift_remove(profile_id);
    }

    /// The sweep's tidy-up: forgets each profile unheard from for longer than `QUEUE_TICKET_STALE_MS`
    /// that owns no `open` ticket, so the map stays the size of the queue.
    pub fn sweep(&self, now: i64, open: &[Ticket]) {
        let owners: IndexSet<&str> = open.iter().map(|ticket| ticket.profile_id.as_str()).collect();
        self.heard
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retain(|profile_id, heard| {
                now - *heard <= QUEUE_TICKET_STALE_MS || owners.contains(profile_id.as_str())
            });
    }

    /// Whether the server has listened long enough to judge any ticket: one full
    /// `QUEUE_TICKET_STALE_MS` since it started, so a restart cancels nothing on its first sweep.
    pub fn expiry_due(&self, now: i64) -> bool {
        now - self.born >= QUEUE_TICKET_STALE_MS
    }

    pub fn last_heard(&self, profile_id: &str) -> Option<i64> {
        self.heard
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(profile_id)
            .copied()
    }
}

impl Default for QueuePresence {
    fn default() -> QueuePresence {
        QueuePresence::new()
    }
}

/// SPEC §11 R1996000: the readout last counted, kept `QUEUE_READOUT_CACHE_TTL_SECONDS` so every
/// waiting client's polls share one count, one answer for all as it names nobody (R640002). Not shared
/// in flight: readouts that miss together each count, and the last to finish is kept. One per `App`.
#[derive(Default)]
pub struct KeptReadout {
    kept: Mutex<Option<(i64, Value)>>,
}

impl KeptReadout {
    fn fresh(&self, now: i64) -> Option<Value> {
        let ttl_ms = QUEUE_READOUT_CACHE_TTL_SECONDS * MS_PER_SECOND;
        self.kept
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .as_ref()
            .filter(|(counted_at, _)| now - counted_at < ttl_ms)
            .map(|(_, body)| body.clone())
    }

    fn keep(&self, now: i64, body: Value) {
        *self.kept.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some((now, body));
    }
}

/// The older of two tickets first: R166's "oldest first", ties on ticket id.
fn older_first<'a>(a: &'a Ticket, b: &'a Ticket) -> (&'a Ticket, &'a Ticket) {
    let a_first = a.enqueued_at < b.enqueued_at || (a.enqueued_at == b.enqueued_at && a.id <= b.id);
    if a_first { (a, b) } else { (b, a) }
}

/// One pair of a sweep whose claim or start failed after the whole-sweep reads, and the error a caller
/// whose ticket is among them answers with. A lost claim race is no failure: another matcher took it.
pub struct FailedStart {
    pub match_id: String,
    pub ticket_ids: Vec<String>,
    pub error: ApiError,
}

/// What one pairing sweep did: the matches and series it made, and the pairs that failed to start.
pub struct SweepReport {
    pub made: usize,
    pub failed: Vec<FailedStart>,
}

/// The seed for a pair, consumed. Two seeded tickets can disagree (each spec seeds its own enqueue), so
/// the older ticket's seed wins, as in `try_pair`'s "oldest first", and both entries are dropped.
fn take_seed_for_pair(a: &Ticket, b: &Ticket) -> Option<String> {
    let (older, younger) = older_first(a, b);
    let mut map = seeds();
    let seed = map.get(&older.id).or_else(|| map.get(&younger.id)).cloned();
    map.shift_remove(&a.id);
    map.shift_remove(&b.id);
    seed
}

// Enqueue

/// §9.5's enqueue, its checks in §9.5's order: a player already in a match (or a series, R264) or
/// queued is told so before their deck is validated, so a stale client gets the useful error.
/// `seed` (R143) and `seat` (R1989000) are only ever `Some` in end-to-end mode.
async fn enqueue_ticket(
    app: &App,
    profile: &Profile,
    choice: &ModeChoiceInput,
    seed: Option<String>,
    seat: Option<jackioh_engine::PlayerId>,
) -> Result<Ticket, ApiError> {
    // §9.5's "not in a match or a series": a 1v1 match, a series not over (R264) or a live co-op
    // match (R1065000). `AuthLevel::Active` did the account half (§9.4's gate, in the router).
    assert_free_to_play(app, &profile.id).await?;
    let open = one_tx!(app.db, |t| t.tickets_open_for_profile(&profile.id).await?)?;
    if let Some(open) = open {
        return Err(ApiError::with_details(
            ApiErrorCode::AlreadyQueued,
            "you are already in the queue",
            json!({ "ticketId": open.id }),
        ));
    }

    // §9.4: "checked by one validator module shared by client and server, at save and again at queue".
    // R253: a Best-of-1 deck on L2, L3, L5, L6 and a trio on L1–L6, against the current catalog; All
    // Random needs no deck (R258).
    let frozen = freeze_choice(app, &profile.id, choice).await?;

    let (mode, deck, portrait, deck_slots, trio, lean) = match frozen {
        // §9.4, §9.5: frozen. This copy lands in the ticket and later the match or series; the saved
        // deck is never read again. R642: the deck's portrait freezes with it, and R1834002 its slots.
        FrozenChoice::Bo1 { deck } => (
            QueueMode::Bo1,
            deck.cards.clone(),
            deck.portrait.clone(),
            deck.slots.clone(),
            None,
            None,
        ),
        FrozenChoice::Bo3 { trio } => (QueueMode::Bo3, Vec::new(), None, None, Some(*trio), None),
        // R1372, R924000: the player's lean waits in the ticket for the deal, as a deck would.
        FrozenChoice::Random { lean } => (QueueMode::Random, Vec::new(), None, None, None, lean),
    };
    let ticket = Ticket {
        id: new_uuid(),
        profile_id: profile.id.clone(),
        rating: profile.rating,
        mode,
        deck,
        portrait,
        deck_slots,
        trio,
        lean,
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
        let existing = one_tx!(app.db, |t| t.tickets_open_for_profile(&profile.id).await?)?;
        if let Some(existing) = existing {
            return Err(ApiError::with_details(
                ApiErrorCode::AlreadyQueued,
                "you are already in the queue",
                json!({ "ticketId": existing.id }),
            ));
        }
        return Err(ApiError::internal(error.to_string()));
    }

    // R143, after the insert won: a seed remembered for a ticket that does not exist would never be
    // consumed and never dropped.
    if let Some(seed) = seed {
        seeds().insert(ticket.id.clone(), seed);
    }
    // R1989000, the same for a pin.
    if let Some(seat) = seat {
        seat_pins().insert(ticket.id.clone(), seat);
    }

    tracing::info!(
        event = "queue.enqueued",
        profileId = %profile.id,
        ticketId = %ticket.id,
        mode = ticket.mode.as_str(),
    );
    Ok(ticket)
}

// Pairing

/// §9.5's widening window, read for one ticket at one instant. It stops at `MAX_FAIR_GAP`
/// (R640001), so two people further apart than that never pair, however long they wait.
fn window_for(ticket: &Ticket, now: i64) -> f64 {
    let waited_seconds = (now - ticket.enqueued_at).max(0) as f64 / MS_PER_SECOND as f64;
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
        None => Err(ApiError::internal(format!(
            "Conquest ticket {} holds no trio",
            ticket.id
        ))),
    }
}

/// R890000: undoes a pair's start by letting go only of the seats that point at this match, then
/// forgetting the `open` skeleton; a rival match's flag stays, as do the tickets' claims. R1523000:
/// the ledger lets go of the same seats once the clears have landed; clears that fail leave both
/// held until the reaper frees them (R1450).
async fn cleanup_pair_start(app: &App, a_profile_id: &str, b_profile_id: &str, match_id: &str) {
    let cleanup = async {
        let mut tx = app.db.begin(None).await?;
        tx.profiles_clear_in_match(a_profile_id, match_id).await?;
        tx.profiles_clear_in_match(b_profile_id, match_id).await?;
        tx.commit().await?;
        let seat = Seat::Match(match_id.to_string());
        for profile_id in [a_profile_id, b_profile_id] {
            app.seats.release(profile_id, &seat);
        }
        discard_open(app, match_id).await
    }
    .await;
    if let Err(cleanup) = cleanup {
        tracing::error!(event = "queue.pair_cleanup_failed", matchId = %match_id, message = %cleanup.message);
    }
}

/// R263: forget the `open` skeleton `tickets_claim_pair` wrote, in its own transaction.
pub(crate) async fn discard_open(app: &App, match_id: &str) -> Result<(), ApiError> {
    one_tx!(app.db, |t| t.matches_discard_open(match_id).await?)?;
    Ok(())
}

/// R259: a Conquest pair becomes a series, its first game's match id the one `tickets_claim_pair`
/// reserved (R263); nobody is put in a match until both have picked a deck (`series.rs`). Series
/// seat p1 is R1989000's coin winner, who goes first in odd games.
async fn start_paired_series(app: &App, a: &Ticket, b: &Ticket, match_id: &str) -> Result<(), ApiError> {
    let (older, younger) = older_first(a, b);
    // R1523000: both sides' seats in the series, or neither, before anything is written: a side
    // that took another game since the sweep's read fails the start as a lost seat does.
    let series_id = new_uuid();
    let lease = match app.seats.reserve(
        &[&older.profile_id, &younger.profile_id],
        Seat::Series {
            id: series_id.clone(),
            game: None,
        },
    ) {
        Ok(lease) => lease,
        Err(taken) => {
            tracing::info!(event = "queue.seat_taken", matchId = %match_id, profileId = %taken.profile);
            if let Err(cleanup) = discard_open(app, match_id).await {
                tracing::error!(event = "queue.pair_cleanup_failed", matchId = %match_id, message = %cleanup.message);
            }
            return Err(already_in_match());
        }
    };
    // R143 and R259: the server mints the series seed (each game's is `{seed_base}:{n}`), unless an
    // end-to-end enqueue supplied one. R1989000: sides index 0 is the coin winner on the seed base
    // (a pin honored first), and R335's alternation runs from there.
    let seed_base = take_seed_for_pair(a, b).unwrap_or_else(new_seed);
    let older_is_p1 = take_seat_pin_for_pair(a, b).unwrap_or_else(|| seat_coin_p1_first(&seed_base));
    let (p1, p2) = if older_is_p1 {
        (older, younger)
    } else {
        (younger, older)
    };
    let made = async {
        let input = NewSeriesInput {
            series_id: series_id.clone(),
            first_match_id: match_id.to_string(),
            sides: (
                NewSeriesSide {
                    profile_id: p1.profile_id.clone(),
                    trio: trio_of(p1)?,
                },
                NewSeriesSide {
                    profile_id: p2.profile_id.clone(),
                    trio: trio_of(p2)?,
                },
            ),
            seed_base,
            catalog_version: app.catalog.version.clone(),
            ranked: true,
        };
        // One transaction, so the series row and `start_series`'s stale-ticket cancels land together: a
        // half-landed 'picking' row would hold both players out of the queue (assert_free_to_play)
        // until the pick deadline ran out (R333).
        let mut tx = app.db.begin(None).await?;
        // R1523000: a newer server holds the seats once the epoch has moved on; nothing is written.
        tx.seats_fence(app.seats.epoch()).await?;
        let series = start_series(app, input, &mut tx).await?;
        tx.commit().await?;
        Ok::<SeriesRow, ApiError>(series)
    }
    .await;
    let series = match made {
        Ok(series) => {
            lease.commit();
            series
        }
        Err(error) => {
            // As `start_paired_match`'s cleanup: the `open` skeleton `tickets_claim_pair` wrote points at
            // nothing, so delete it now rather than leave it to the reaper's open-row sweep (R1450).
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

/// The All Random deck R258 deals one seat from the match seed (`{seed}:p1-deck`, `{seed}:p2-deck`),
/// leaning on the set that seat's ticket picked, if it picked one (R1372, R924000).
fn dealt_deck(seed: &str, seat: &str, lean: Option<LeanPick>) -> Vec<String> {
    crate::actor::engine::deal_random_deck(
        &format!("{seed}:{seat}-deck"),
        crate::actor::engine::lean_of(lean),
    )
}

/// All Random's portrait for one seat (R642), dealt from the seed like its deck.
fn dealt_portrait(seed: &str, seat: &str) -> Option<String> {
    Some(jackioh_engine::wire::pick_portrait_from_seed(&format!("{seed}:portrait:{seat}")).to_string())
}

/// Creates the paired match, or for Conquest the series, from two tickets this process has claimed.
/// Best of 1 plays the two frozen decks and portraits (R642); All Random (R258) deals both from the
/// seed, each on its own ticket's lean (R1372, R924000), into the match row so `(seed, decks, log)`
/// replays it.
async fn start_paired_match(app: &Arc<App>, a: &Ticket, b: &Ticket, match_id: &str) -> Result<(), ApiError> {
    if a.mode == QueueMode::Bo3 {
        return start_paired_series(app, a, b, match_id).await;
    }
    // R1523000: both seats, or neither, before anything is written: a player who took another game
    // since the sweep's read fails the start here, as a lost seat does.
    let lease = match app
        .seats
        .reserve(&[&a.profile_id, &b.profile_id], Seat::Match(match_id.to_string()))
    {
        Ok(lease) => lease,
        Err(taken) => {
            tracing::info!(event = "queue.seat_taken", matchId = %match_id, profileId = %taken.profile);
            cleanup_pair_start(app, &a.profile_id, &b.profile_id, match_id).await;
            return Err(already_in_match());
        }
    };
    // R143: the server mints the seed, unless an end-to-end enqueue supplied one.
    let seed = take_seed_for_pair(a, b).unwrap_or_else(new_seed);
    // R1989000: a pin, else the seed's coin, seats the older ticket. Decks, portraits, slots and
    // leans travel with the profile; All Random deals stay seat-keyed.
    let (older, younger) = older_first(a, b);
    let older_is_p1 = take_seat_pin_for_pair(a, b).unwrap_or_else(|| seat_coin_p1_first(&seed));
    let (p1, p2) = if older_is_p1 {
        (older, younger)
    } else {
        (younger, older)
    };
    let random = a.mode == QueueMode::Random;
    let seats = (
        MatchSeat {
            profile_id: p1.profile_id.clone(),
            player: jackioh_engine::PlayerId::P1,
            deck: if random {
                dealt_deck(&seed, "p1", p1.lean)
            } else {
                p1.deck.clone()
            },
            portrait: if random {
                dealt_portrait(&seed, "p1")
            } else {
                p1.portrait.clone().flatten()
            },
            // R1834002: an All Random seat plays no deck of its own, so it wears the account's slots.
            deck_slots: if random { None } else { p1.deck_slots.clone() },
        },
        MatchSeat {
            profile_id: p2.profile_id.clone(),
            player: jackioh_engine::PlayerId::P2,
            deck: if random {
                dealt_deck(&seed, "p2", p2.lean)
            } else {
                p2.deck.clone()
            },
            portrait: if random {
                dealt_portrait(&seed, "p2")
            } else {
                p2.portrait.clone().flatten()
            },
            deck_slots: if random { None } else { p2.deck_slots.clone() },
        },
    );

    // `Registry::start` creates the row (R79's clocks), promoting to `live` the `open` skeleton
    // `tickets_claim_pair` wrote, which satisfies `profiles.current_match_id`'s foreign key for the
    // `set_in_match` writes (§9.5's in-match state) made first.
    {
        let mut tx = app.db.begin(None).await?;
        if let Err(error) = tx.seats_fence(app.seats.epoch()).await {
            drop(tx);
            cleanup_pair_start(app, &a.profile_id, &b.profile_id, match_id).await;
            return Err(ApiError::from(error));
        }
        // §9.5: in-match state is set here and cleared by `results.rs` at every ending. Both seats
        // are claimed compare-and-set (R890000), a guard behind the ledger's: a refusal is drift,
        // and dropping `tx` rolls both back, so the rival flag is never touched.
        let a_seated = tx.profiles_set_in_match(&a.profile_id, Some(match_id)).await?;
        let b_seated = tx.profiles_set_in_match(&b.profile_id, Some(match_id)).await?;
        if !a_seated || !b_seated {
            tracing::error!(event = "seats.drift", source = "compare-and-set", matchId = %match_id);
            // Dropped first: the fake store's lock is held by this transaction, and the cleanup
            // begins its own. The tickets stay claimed.
            drop(tx);
            cleanup_pair_start(app, &a.profile_id, &b.profile_id, match_id).await;
            return Err(already_in_match());
        }
        tx.commit().await?;
    }
    // R1523000: the record holds both seats now, so the ledger does: a start that fails from here
    // lets go of them through its cleanup, as it clears the flags.
    lease.commit();

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
        // The flags have committed, so a failed start would lock both players out until the reaper's
        // open-row sweep (R1450): undo it now (the tickets stay claimed).
        cleanup_pair_start(app, &a.profile_id, &b.profile_id, match_id).await;
        return Err(error);
    }
    tracing::info!(
        event = "queue.paired",
        mode = a.mode.as_str(),
        matchId = %match_id,
        tickets = %json_list(&[a.id.clone(), b.id.clone()]),
        ratings = %json_list(&[a.rating, b.rating]),
    );
    Ok(())
}

/// One pairing sweep. R166: the oldest ticket is paired first, against the oldest opponent its window
/// admits, not the closest in rating; ties break on ticket id. A pair's failed claim or start is
/// logged (`queue.start_failed`) and the sweep moves on; only a failure of its opening reads is `Err`.
pub async fn try_pair(app: &Arc<App>) -> Result<SweepReport, ApiError> {
    let now = now_ms();
    let mut tx = app.db.begin(None).await?;
    let mut open: Vec<Ticket> = tx.tickets_list_open().await?;
    open.sort_by(|x, y| x.enqueued_at.cmp(&y.enqueued_at).then_with(|| x.id.cmp(&y.id)));
    // R897000: a profile with no open ticket is never judged, so one silent for a window is forgotten
    // here, before an empty queue returns; an open ticket's owner keeps its entry for the verdict below.
    app.queue_presence.sweep(now, &open);
    // R1453: one ticket alone can still pair, with a bot.
    if open.is_empty() {
        tx.commit().await?;
        return Ok(SweepReport {
            made: 0,
            failed: Vec::new(),
        });
    }

    // SPEC §11 R897000: a ticket unheard from for longer than `QUEUE_TICKET_STALE_MS` is canceled
    // before pairing, so an absent player is never seated nor takes a phantom ranked loss. A profile
    // never heard from gets `enqueued_at`'s grace, and a restarted server judges nothing until it has
    // listened one window. `tickets_cancel` only closes an open ticket, so a claim landed since stands.
    if app.queue_presence.expiry_due(now) {
        let mut live = Vec::with_capacity(open.len());
        let mut stale = Vec::new();
        for ticket in open {
            let heard = app
                .queue_presence
                .last_heard(&ticket.profile_id)
                .unwrap_or(ticket.enqueued_at);
            if now - heard > QUEUE_TICKET_STALE_MS {
                stale.push(ticket);
            } else {
                live.push(ticket);
            }
        }
        open = live;
        if !stale.is_empty() {
            // The listing read commits before the cancels write: each cancel is its own transaction.
            tx.commit().await?;
            for ticket in &stale {
                one_tx!(app.db, |t| t.tickets_cancel(&ticket.id, now).await?)?;
                forget_seed(&ticket.id);
                forget_seat(&ticket.id);
                app.queue_presence.forget(&ticket.profile_id);
                tracing::info!(
                    event = "queue.expired",
                    profileId = %ticket.profile_id,
                    ticketId = %ticket.id,
                );
            }
            if open.is_empty() {
                return Ok(SweepReport {
                    made: 0,
                    failed: Vec::new(),
                });
            }
            tx = app.db.begin(None).await?;
        }
    }

    // §9.5's "not in a match" holds at pairing too: a ticket whose owner joined a room match, a
    // series (R264) or a live co-op match (R1065000) must not become a second match.
    let owners: Vec<String> = open.iter().map(|ticket| ticket.profile_id.clone()).collect();
    let mut busy: IndexSet<String> = tx
        .profiles_get_many(&owners)
        .await?
        .into_iter()
        .filter(|profile| profile.in_match_id.is_some())
        .map(|profile| profile.id)
        .collect();
    // R641000: the sides of every series not over, a bot's included, which no bot claim may seat.
    let in_series: IndexSet<String> = tx
        .series_active()
        .await?
        .into_iter()
        .flat_map(|series| [series.sides.0.profile_id, series.sides.1.profile_id])
        .collect();
    busy.extend(in_series.iter().cloned());
    // R1065000: nor while a live co-op match holds them, which sets no in-match flag.
    let in_duos = tx.presence_in_duos(&owners).await?;
    busy.extend(in_duos.into_keys());
    tx.commit().await?;
    let mut taken: IndexSet<String> = open
        .iter()
        .filter(|ticket| busy.contains(&ticket.profile_id))
        .map(|ticket| ticket.id.clone())
        .collect();
    let mut report = SweepReport {
        made: 0,
        failed: Vec::new(),
    };

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
            let claimed: Result<bool, ApiError> = async {
                let mut tx = app.db.begin(None).await?;
                // R1523000: a server whose epoch a newer one has taken claims nothing, so both
                // tickets stay open for the newer server's sweep.
                tx.seats_fence(app.seats.epoch()).await?;
                let won = tx.tickets_claim_pair(&a.id, &b.id, &match_id, now).await?;
                tx.commit().await?;
                Ok(won)
            }
            .await;
            let won = match claimed {
                Ok(won) => won,
                // One pair's failed claim ends that pair, not the sweep: both tickets are marked
                // taken so no later iteration retries them, and the sweep moves on.
                Err(error) => {
                    tracing::warn!(event = "queue.start_failed", matchId = %match_id, tickets = %json_list(&[a.id.clone(), b.id.clone()]), message = %error.message);
                    report.failed.push(FailedStart {
                        match_id,
                        ticket_ids: vec![a.id.clone(), b.id.clone()],
                        error,
                    });
                    taken.insert(a.id.clone());
                    taken.insert(b.id.clone());
                    continue;
                }
            };
            if !won {
                // Another matcher got one of them. Never create a match for a ticket we did not claim:
                // find out which one is gone and either try another opponent or drop this ticket.
                let still = one_tx!(app.db, |t| t.tickets_get(&a.id).await?)?;
                if still.is_none_or(|ticket| ticket.status != TicketStatus::Open) {
                    taken.insert(a.id.clone());
                    break;
                }
                taken.insert(b.id.clone());
                continue;
            }

            taken.insert(a.id.clone());
            taken.insert(b.id.clone());
            // SPEC §11 R897000: both tickets left `open`, paired or claimed; their presence goes
            // with them, win or lose of the start that follows.
            app.queue_presence.forget(&a.profile_id);
            app.queue_presence.forget(&b.profile_id);
            match start_paired_match(app, a, b, &match_id).await {
                Ok(()) => {
                    report.made += 1;
                    break;
                }
                // One pair's failed start ends that pair, not the sweep: `start_paired_match`
                // already freed both players, and both tickets are marked taken above.
                Err(error) => {
                    tracing::warn!(event = "queue.start_failed", matchId = %match_id, tickets = %json_list(&[a.id.clone(), b.id.clone()]), message = %error.message);
                    report.failed.push(FailedStart {
                        match_id,
                        ticket_ids: vec![a.id.clone(), b.id.clone()],
                        error,
                    });
                    continue;
                }
            }
        }
    }

    // R1453: only after every pair of people this sweep could make, so a person inside the window
    // always comes before a bot.
    if app.env.ladder_bots {
        let bot_made = pair_with_bots(app, &open, &mut taken, &in_series, now, &mut report.failed).await?;
        report.made += bot_made;
    }

    Ok(report)
}

/// R1453: pairs each ticket still open and past its fallback time, oldest first, with the bot
/// `choose_bot` names, claimed in one statement (`tickets_claim_for_bot`; a Conquest ticket's in
/// `start_bot_series`, R641000). A lost claim waits for the next sweep; a failed claim or start is
/// logged and collected into `failed`; only a failure of the opening `bots_list` is `Err`.
async fn pair_with_bots(
    app: &Arc<App>,
    open: &[Ticket],
    taken: &mut IndexSet<String>,
    in_series: &IndexSet<String>,
    now: i64,
    failed: &mut Vec<FailedStart>,
) -> Result<usize, ApiError> {
    let due: Vec<&Ticket> = open
        .iter()
        .filter(|ticket| !taken.contains(&ticket.id) && fallback_due(ticket, now))
        .collect();
    if due.is_empty() {
        return Ok(0);
    }
    let bots = one_tx!(app.db, |t| t.bots_list().await?)?;
    let hour = hour_of(now);
    let mut busy: IndexSet<String> = in_series.clone();
    let mut made = 0;
    for ticket in due {
        let recent = {
            let mut tx = app.db.begin(Some(&ticket.profile_id)).await?;
            let recent = tx
                .results_recent_opponents(&ticket.profile_id, BOT_RECENT_OPPONENTS)
                .await?;
            tx.commit().await?;
            recent
        };
        let Some(bot) = choose_bot(ticket, &bots, &recent, &busy, hour, &app.catalog.version) else {
            continue;
        };
        let match_id = new_uuid();
        if ticket.mode == QueueMode::Bo3 {
            match start_bot_series(app, ticket, bot, &match_id, now).await {
                // SPEC §11 R897000: the ticket left `open`; its presence goes with it.
                Ok(true) => {
                    app.queue_presence.forget(&ticket.profile_id);
                }
                Ok(false) => continue,
                // One bot pairing's failed start ends that pairing, not the fallback.
                Err(error) => {
                    tracing::warn!(event = "queue.start_failed", matchId = %match_id, tickets = %json_list(std::slice::from_ref(&ticket.id)), message = %error.message);
                    failed.push(FailedStart {
                        match_id,
                        ticket_ids: vec![ticket.id.clone()],
                        error,
                    });
                    taken.insert(ticket.id.clone());
                    continue;
                }
            }
            taken.insert(ticket.id.clone());
            busy.insert(bot.profile_id.clone());
            made += 1;
            continue;
        }
        // R1523000: the person's seat and the bot's, or neither, before the claim; one taken since
        // the sweep's read leaves the ticket for the next sweep, as a lost claim does.
        let Ok(lease) = app.seats.reserve(
            &[&ticket.profile_id, &bot.profile_id],
            Seat::Match(match_id.clone()),
        ) else {
            continue;
        };
        let claimed: Result<bool, ApiError> = async {
            let mut tx = app.db.begin(None).await?;
            tx.seats_fence(app.seats.epoch()).await?;
            let won = tx
                .tickets_claim_for_bot(&ticket.id, &bot.profile_id, &match_id, now)
                .await?;
            tx.commit().await?;
            Ok(won)
        }
        .await;
        let won = match claimed {
            Ok(won) => won,
            // One bot pairing's failed claim ends that pairing, not the fallback.
            Err(error) => {
                tracing::warn!(event = "queue.start_failed", matchId = %match_id, tickets = %json_list(std::slice::from_ref(&ticket.id)), message = %error.message);
                failed.push(FailedStart {
                    match_id,
                    ticket_ids: vec![ticket.id.clone()],
                    error,
                });
                taken.insert(ticket.id.clone());
                continue;
            }
        };
        if !won {
            continue;
        }
        taken.insert(ticket.id.clone());
        busy.insert(bot.profile_id.clone());
        app.queue_presence.forget(&ticket.profile_id);
        // A failed start ends this pairing only: `start_bot_match` already freed both sides.
        if let Err(error) = start_bot_match(app, ticket, bot, &match_id, lease).await {
            tracing::warn!(event = "queue.start_failed", matchId = %match_id, tickets = %json_list(std::slice::from_ref(&ticket.id)), message = %error.message);
            failed.push(FailedStart {
                match_id,
                ticket_ids: vec![ticket.id.clone()],
                error,
            });
            continue;
        }
        made += 1;
    }
    Ok(made)
}

/// R641000: the ranked (R604) series a Conquest ticket and its bot play, the claim and the series row
/// in one transaction; between games the series alone holds the bot. Series p1 is R1989000's coin
/// winner (the person's pin first), first in odd games (R335). False when the bot has no trio or the
/// claim lost.
async fn start_bot_series(
    app: &Arc<App>,
    ticket: &Ticket,
    bot: &BotRow,
    match_id: &str,
    now: i64,
) -> Result<bool, ApiError> {
    let person_trio = trio_of(ticket)?;
    let Some(bot_trio) = bot.trio.clone() else {
        return Ok(false);
    };
    // R1523000: both sides' seats in the series, or neither, before the claim.
    let series_id = new_uuid();
    let Ok(lease) = app.seats.reserve(
        &[&ticket.profile_id, &bot.profile_id],
        Seat::Series {
            id: series_id.clone(),
            game: None,
        },
    ) else {
        return Ok(false);
    };
    let mut tx = app.db.begin(None).await?;
    tx.seats_fence(app.seats.epoch()).await?;
    let won = tx
        .tickets_claim_for_bot_series(&ticket.id, &bot.profile_id, match_id, now)
        .await?;
    if !won {
        return Ok(false);
    }
    // R143 and R259: the server mints the series seed, unless an end-to-end enqueue supplied one.
    // R1989000: sides index 0 is the coin winner on the seed base, the person's pin honored first.
    let seed_base = seeds().shift_remove(&ticket.id).unwrap_or_else(new_seed);
    let pin = seat_pins().shift_remove(&ticket.id);
    let person_is_p1 = pin
        .map(|seat| seat == jackioh_engine::PlayerId::P1)
        .unwrap_or_else(|| seat_coin_p1_first(&seed_base));
    let (first, second) = if person_is_p1 {
        (
            NewSeriesSide {
                profile_id: ticket.profile_id.clone(),
                trio: person_trio,
            },
            NewSeriesSide {
                profile_id: bot.profile_id.clone(),
                trio: bot_trio,
            },
        )
    } else {
        (
            NewSeriesSide {
                profile_id: bot.profile_id.clone(),
                trio: bot_trio,
            },
            NewSeriesSide {
                profile_id: ticket.profile_id.clone(),
                trio: person_trio,
            },
        )
    };
    let input = NewSeriesInput {
        series_id,
        first_match_id: match_id.to_string(),
        sides: (first, second),
        seed_base,
        catalog_version: app.catalog.version.clone(),
        ranked: true,
    };
    let series = start_series(app, input, &mut tx).await?;
    tx.commit().await?;
    lease.commit();
    tracing::info!(
        event = "queue.paired_bot",
        mode = "bo3",
        seriesId = %series.id,
        firstMatchId = %match_id,
        ticket = %ticket.id,
        botId = %bot.profile_id,
        ratings = %json_list(&[ticket.rating, bot.rating]),
    );
    Ok(true)
}

/// R1453: the ranked (R604) match a ticket and its claimed bot play, seated by the seed's coin
/// (R1989000, the person's pin first). Best of 1 plays the frozen deck against a bot deck picked from
/// the seed (`{seed}:bot-deck`); All Random deals both seats (R258, R642), the bot's with no lean.
/// `lease` holds both seats in the ledger (R1523000) until the record holds them too.
async fn start_bot_match(
    app: &Arc<App>,
    ticket: &Ticket,
    bot: &BotRow,
    match_id: &str,
    lease: Lease<'_>,
) -> Result<(), ApiError> {
    let seed = seeds().shift_remove(&ticket.id).unwrap_or_else(new_seed);
    // R1989000: the seed's coin seats the person and the bot, the person's pin honored first.
    let pin = seat_pins().shift_remove(&ticket.id);
    let person_is_p1 = pin
        .map(|seat| seat == jackioh_engine::PlayerId::P1)
        .unwrap_or_else(|| seat_coin_p1_first(&seed));
    let (human_player, bot_player) = if person_is_p1 {
        (jackioh_engine::PlayerId::P1, jackioh_engine::PlayerId::P2)
    } else {
        (jackioh_engine::PlayerId::P2, jackioh_engine::PlayerId::P1)
    };
    let random = ticket.mode == QueueMode::Random;
    let human = MatchSeat {
        profile_id: ticket.profile_id.clone(),
        player: human_player,
        deck: if random {
            dealt_deck(&seed, human_player.as_str(), ticket.lean)
        } else {
            ticket.deck.clone()
        },
        portrait: if random {
            dealt_portrait(&seed, human_player.as_str())
        } else {
            ticket.portrait.clone().flatten()
        },
        deck_slots: if random { None } else { ticket.deck_slots.clone() },
    };
    // Everything after the claim lets both go if it fails, as the claim already seated the bot;
    // else only the stale-start release (R1454) or the open-row sweep (R1450) would. §9.5's in-match
    // state for the player, compare-and-set (R890000): a room join that seated them since loses the
    // seat here; the transaction is over before the cleanup begins its own (the fake store's lock).
    let seated: Result<bool, ApiError> = async {
        let mut tx = app.db.begin(None).await?;
        tx.seats_fence(app.seats.epoch()).await?;
        let won = tx
            .profiles_set_in_match(&ticket.profile_id, Some(match_id))
            .await?;
        if won {
            tx.commit().await?;
        }
        Ok(won)
    }
    .await;
    match seated {
        // R1523000: the record holds both seats now (the claim seated the bot), so the ledger does.
        Ok(true) => lease.commit(),
        Ok(false) => {
            tracing::error!(event = "seats.drift", source = "compare-and-set", matchId = %match_id);
            // Nothing was committed: undo the claim (the ticket stays claimed).
            cleanup_pair_start(app, &ticket.profile_id, &bot.profile_id, match_id).await;
            return Err(already_in_match());
        }
        Err(error) => {
            cleanup_pair_start(app, &ticket.profile_id, &bot.profile_id, match_id).await;
            return Err(error);
        }
    }

    let started = async {
        let bot_seat = if random {
            MatchSeat {
                profile_id: bot.profile_id.clone(),
                player: bot_player,
                deck: dealt_deck(&seed, bot_player.as_str(), None),
                portrait: dealt_portrait(&seed, bot_player.as_str()),
                deck_slots: None,
            }
        } else {
            let picked = jackioh_engine::Rng::new(&format!("{seed}:bot-deck"), 0)
                .pick(&bot.decks)
                .cloned()
                .ok_or_else(|| ApiError::internal(format!("bot {} holds no deck", bot.profile_id)))?;
            // R1834005: the bot's deck carries the slots it drew for it, as a person's does.
            MatchSeat {
                profile_id: bot.profile_id.clone(),
                player: bot_player,
                deck: picked.cards,
                portrait: Some(picked.portrait),
                deck_slots: picked.slots,
            }
        };
        // The row is seat-ordered — index 0 is p1 — whoever the coin gave it to.
        let seats = if person_is_p1 {
            (human, bot_seat)
        } else {
            (bot_seat, human)
        };

        // The mode is read off the claimed ticket.
        app.matches
            .start(
                app,
                StartMatchInput {
                    match_id: match_id.to_string(),
                    seed: seed.clone(),
                    catalog_version: app.catalog.version.clone(),
                    ranked: true,
                    seats,
                    mode: None,
                    stake: None,
                },
            )
            .await
    }
    .await;
    if let Err(error) = started {
        // As `start_paired_match`'s: both profiles are let go and the skeleton discarded before the
        // error stands (the ticket stays claimed). The clears are conditional (R890000), which
        // also releases the bot `tickets_claim_for_bot` seated.
        cleanup_pair_start(app, &ticket.profile_id, &bot.profile_id, match_id).await;
        return Err(error);
    }
    tracing::info!(
        event = "queue.paired_bot",
        mode = ticket.mode.as_str(),
        matchId = %match_id,
        ticket = %ticket.id,
        botId = %bot.profile_id,
        ratings = %json_list(&[ticket.rating, bot.rating]),
    );
    Ok(())
}

/// R1555000: kicks off a ranked (R604) Best-of-1 match between two bots when fewer than
/// `BOT_VS_BOT_MAX_LIVE` are live, no ticket is open, a bot is grinding, none ended in the last
/// `BOT_VS_BOT_MIN_GAP_SECONDS`, and `choose_bot_pair` names a pair, claimed in one statement
/// (`bots_claim_pair`). R1989000: the seed's coin seats them; rated as any bot game (R610, R611,
/// R640000). A bot match a restart cut off is folded back first (§9.5).
pub async fn kick_off_bot_match(app: &Arc<App>, now: i64) -> Result<Option<String>, ApiError> {
    let (bots, queued) = {
        let mut tx = app.db.begin(None).await?;
        let bots = tx.bots_list().await?;
        let queued = tx.tickets_count_open().await?;
        tx.commit().await?;
        (bots, queued)
    };
    let live = bot_matches(&bots);
    for match_id in live.iter().filter(|match_id| !app.matches.has(match_id)) {
        if let Err(error) = app.matches.actor_for(app, match_id).await {
            tracing::warn!(event = "bots.match.resume_failed", matchId = %match_id, message = %error);
        }
    }
    if queued > 0
        || live.len() >= BOT_VS_BOT_MAX_LIVE
        || !bots.iter().any(|bot| bot.grinding && bot.retired_at.is_none())
    {
        return Ok(None);
    }
    let (busy, games) = {
        let mut tx = app.db.begin(None).await?;
        let busy: IndexSet<String> = tx
            .series_active()
            .await?
            .into_iter()
            .flat_map(|series| [series.sides.0.profile_id, series.sides.1.profile_id])
            .collect();
        let games = tx.ranked_bot_games_since(bot_match_gap_since(now)).await?;
        tx.commit().await?;
        (busy, games)
    };
    if !bot_match_gap_passed(&games, now) {
        return Ok(None);
    }
    let mut recent: IndexMap<String, Vec<String>> = IndexMap::new();
    for grinder in bots.iter().filter(|bot| bot.grinding && bot.retired_at.is_none()) {
        let faced = one_tx!(app.db, |t| {
            t.results_recent_opponents(&grinder.profile_id, BOT_RECENT_OPPONENTS)
                .await?
        })?;
        recent.insert(grinder.profile_id.clone(), faced);
    }
    let Some((grinder, opponent)) =
        choose_bot_pair(&bots, &recent, &busy, hour_of(now), &app.catalog.version)
    else {
        return Ok(None);
    };
    let match_id = new_uuid();
    let players = (grinder.profile_id.clone(), opponent.profile_id.clone());
    // R1523000: both bots' seats, or neither, before the claim; a bot another door took since the
    // read (a rematch, a person's fallback) is a claim that loses.
    let Ok(lease) = app
        .seats
        .reserve(&[&players.0, &players.1], Seat::Match(match_id.clone()))
    else {
        return Ok(None);
    };
    let mut tx = app.db.begin(None).await?;
    tx.seats_fence(app.seats.epoch()).await?;
    if !tx
        .bots_claim_pair(&players, &match_id, &app.catalog.version, now)
        .await?
    {
        // Dropping `tx` rolls it back, and dropping `lease` gives both seats back.
        return Ok(None);
    }
    tx.commit().await?;
    lease.commit();

    let seed = new_seed();
    // R1989000: the seed's coin seats the grinder and its opponent; no pin exists here, since no
    // ticket asked for one.
    let grinder_is_p1 = seat_coin_p1_first(&seed);
    let (grinder_player, opponent_player) = if grinder_is_p1 {
        (jackioh_engine::PlayerId::P1, jackioh_engine::PlayerId::P2)
    } else {
        (jackioh_engine::PlayerId::P2, jackioh_engine::PlayerId::P1)
    };
    let seat = |bot: &BotRow, player: jackioh_engine::PlayerId| -> Result<MatchSeat, ApiError> {
        let picked = jackioh_engine::Rng::new(&format!("{seed}:bot-deck:{}", player.as_str()), 0)
            .pick(&bot.decks)
            .cloned()
            .ok_or_else(|| ApiError::internal(format!("bot {} holds no deck", bot.profile_id)))?;
        Ok(MatchSeat {
            profile_id: bot.profile_id.clone(),
            player,
            deck: picked.cards,
            portrait: Some(picked.portrait),
            deck_slots: picked.slots,
        })
    };
    let started = async {
        // The row is seat-ordered — index 0 is p1 — whatever the coin gave the grinder.
        let grinder_seat = seat(grinder, grinder_player)?;
        let opponent_seat = seat(opponent, opponent_player)?;
        let seats = if grinder_is_p1 {
            (grinder_seat, opponent_seat)
        } else {
            (opponent_seat, grinder_seat)
        };
        app.matches
            .start(
                app,
                StartMatchInput {
                    match_id: match_id.clone(),
                    seed: seed.clone(),
                    catalog_version: app.catalog.version.clone(),
                    ranked: true,
                    seats,
                    mode: Some(QueueMode::Bo1),
                    stake: None,
                },
            )
            .await
    }
    .await;
    if let Err(error) = started {
        // As `start_bot_match`'s: both bots are let go and the skeleton discarded.
        cleanup_pair_start(app, &players.0, &players.1, &match_id).await;
        return Err(error);
    }
    tracing::info!(
        event = "queue.paired_bots",
        matchId = %match_id,
        botIds = %json_list(&[players.0.clone(), players.1.clone()]),
        ratings = %json_list(&[grinder.rating, opponent.rating]),
    );
    Ok(Some(match_id))
}

/// §9.5: "Pairing runs on enqueue plus a sweeper every few seconds" (R108,
/// `MATCHMAKER_SWEEP_INTERVAL_SECONDS`); `app.rs` spawns it (SURFACE §11.2). Each sweep starts one
/// interval after the last, on tokio's clock, so a test drives it with `tokio::time::advance`.
pub async fn run_matchmaker(app: Arc<App>) {
    loop {
        tokio::time::sleep(Duration::from_secs(MATCHMAKER_SWEEP_INTERVAL_SECONDS as u64)).await;
        // R1454: a bot (and its player) a cut-off start still holds is let go before the sweep,
        // whatever `LADDER_BOTS` says now, since a restart may have turned it off.
        if let Err(error) = crate::bots::pool::release_stale_starts(&app, now_ms()).await {
            tracing::warn!(event = "bots.start.release_failed", message = %error);
        }
        if let Err(error) = try_pair(&app).await {
            // A failed sweep must not kill the sweeper: the next one may well succeed.
            tracing::warn!(event = "matchmaker.sweep_failed", message = %error.message);
        }
        // R1555000: after the sweep, so a ticket it could pair is never left for a match between
        // two bots, and only with the queue empty.
        if app.env.ladder_bots
            && let Err(error) = kick_off_bot_match(&app, now_ms()).await
        {
            tracing::warn!(event = "bots.match.failed", message = %error.message);
        }
    }
}

// Routes

/// `POST /api/queue` (active, §9.4's gate: a pending account gets 403 before this runs): §9.5's
/// enqueue, pairing inline so a player who pairs at once learns it from the response's status. Takes
/// `&Arc<App>` where SURFACE §11.2 writes `&App`, as `Registry::start` does.
pub async fn enqueue(app: &Arc<App>, req: Req) -> ApiResult {
    enqueue_route(app, req).await
}

async fn enqueue_route(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let choice = read_mode_choice(&req.body)?;
    let seed = seed_override_of(app, &req.body)?;
    let seat = seat_override_of(app, &req.body)?;
    let ticket = enqueue_ticket(app, &profile, &choice, seed, seat).await?;
    let report = try_pair(app).await?;
    // The sweep carries on past one pair's failed start, so only a caller whose own ticket is
    // among the failures answers with its error; anyone else reads their ticket's status below.
    if let Some(own) = report
        .failed
        .into_iter()
        .find(|failure| failure.ticket_ids.iter().any(|id| id == &ticket.id))
    {
        return Err(own.error);
    }

    let mut tx = app.db.begin(None).await?;
    let current = tx.tickets_get(&ticket.id).await?;
    let status = current
        .as_ref()
        .map(|current| current.status)
        .unwrap_or(ticket.status);
    // A paired Conquest ticket's `match_id` is game 1's reserved id, which is not a match anyone
    // can open yet: the player goes to the series to pick a deck, so it answers with that.
    let paired = status == TicketStatus::Matched;
    let series_id = if paired && ticket.mode == QueueMode::Bo3 {
        tx.series_active_for(&profile.id).await?.map(|series| series.id)
    } else {
        None
    };
    let match_id = if paired && ticket.mode != QueueMode::Bo3 {
        current.as_ref().and_then(|current| current.match_id.clone())
    } else {
        None
    };
    tx.commit().await?;
    // R640002: no count of open tickets, which would tell a lone queuer that a bot came.
    Ok(json(
        200,
        json!({
            "ticketId": ticket.id,
            "status": status,
            "matchId": match_id,
            "seriesId": series_id,
            "mode": ticket.mode,
        }),
    ))
}

/// `DELETE /api/queue` (active): leaving the queue, idempotent (`cancelled: false` when nothing was
/// open). R167: the sweeper (R108) can pair a ticket before the request arrives; cancel never unmakes
/// a pairing, it only closes a ticket still open.
pub async fn dequeue(app: &Arc<App>, req: Req) -> ApiResult {
    dequeue_route(app, req).await
}

async fn dequeue_route(app: &App, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let mut tx = app.db.begin(None).await?;
    let Some(ticket) = tx.tickets_open_for_profile(&profile.id).await? else {
        tx.commit().await?;
        return Ok(json(200, json!({ "cancelled": false })));
    };
    tx.tickets_cancel(&ticket.id, now_ms()).await?;
    tx.commit().await?;
    // R143: a canceled ticket will never be paired, so its seed is dropped with it. R1989000:
    // its seat pin goes with it.
    forget_seed(&ticket.id);
    forget_seat(&ticket.id);
    app.queue_presence.forget(&profile.id);
    tracing::info!(event = "queue.cancelled", profileId = %profile.id, ticketId = %ticket.id);
    Ok(json(200, json!({ "cancelled": true, "ticketId": ticket.id })))
}

/// R640002: the expected wait, in seconds, that `waits_ms` (each a claimed ticket's wait, in ms)
/// calls for: the `QUEUE_WAIT_QUANTILE` of them, rounded up to the next whole `QUEUE_WAIT_STEP_SECONDS`
/// (a wait on a step goes to the one past it, so "under" holds). `None` below
/// `QUEUE_WAIT_MIN_SAMPLES` waits, which is too few to say anything.
pub fn expected_wait_seconds(waits_ms: &[i64]) -> Option<i64> {
    if waits_ms.len() < QUEUE_WAIT_MIN_SAMPLES {
        return None;
    }
    let mut sorted = waits_ms.to_vec();
    sorted.sort_unstable();
    let rank = ((QUEUE_WAIT_QUANTILE * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len());
    let step_ms = QUEUE_WAIT_STEP_SECONDS * MS_PER_SECOND;
    Some((sorted[rank - 1].max(0) / step_ms + 1) * QUEUE_WAIT_STEP_SECONDS)
}

/// `GET /api/queue/population` (user, §9.5): the readout instead of an endless spinner. `AuthLevel::User`
/// because it holds aggregates that name nobody (§9.4's "no queue" is about joining); `None` would give an
/// anonymous caller a live-traffic feed. R640002: per mode (R257), the expected wait the tickets claimed
/// in the last `QUEUE_READOUT_WINDOW_SECONDS` call for (null when too few) and the games that ended in
/// it, bot ones included; no count of open tickets, which would tell a lone queuer that a bot came.
/// R1996000: counted at most once per `QUEUE_READOUT_CACHE_TTL_SECONDS` and kept (`KeptReadout`).
pub async fn population(app: &Arc<App>, _req: Req) -> ApiResult {
    population_route(app).await
}

async fn population_route(app: &App) -> ApiResult {
    let now = now_ms();
    if let Some(body) = app.queue_readout.fresh(now) {
        return Ok(json(200, body));
    }
    let since = now - QUEUE_READOUT_WINDOW_SECONDS * MS_PER_SECOND;
    let mut tx = app.db.begin(None).await?;
    let waits = tx.tickets_recent_waits(since).await?;
    let games = tx.results_count_since(since).await?;
    tx.commit().await?;
    let mut by_mode: PerMode<Vec<i64>> = PerMode::default();
    for wait in &waits {
        by_mode[wait.mode].push(wait.waited_ms);
    }
    let expected = PerMode {
        bo1: expected_wait_seconds(&by_mode.bo1),
        bo3: expected_wait_seconds(&by_mode.bo3),
        random: expected_wait_seconds(&by_mode.random),
    };
    let body = json!({ "gamesLastHour": games, "expectedWaitSeconds": expected });
    app.queue_readout.keep(now, body.clone());
    Ok(json(200, body))
}
