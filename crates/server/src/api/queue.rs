//! Matchmaking (BUILD M7-T3, SPEC §9.5), in three modes (R257).
//!
//! §9.5: enqueue asserts the account is active and not in a match, validates the loadout, freezes the
//! chosen deck into the ticket and returns its id. Pairing runs on enqueue plus a sweeper every few
//! seconds; the window widens ±50 rating every 10 s from ±100 and is uncapped after 60 s; both tickets
//! are claimed in one atomic statement.
//!
//! R257 adds the mode: Best of 1 (one saved deck), Conquest (a trio, played as a series, R330) and
//! All Random (R258). A ticket pairs only with a ticket of its own mode; inside a mode the window
//! and R166's order are exactly as before. What a pair becomes depends on the mode: a Best-of-1
//! match on the two frozen decks, an All Random match on two dealt ones, or a Conquest series whose
//! first game waits for both players to pick (`series.rs`).
//!
//! Three pieces: the endpoints, one pairing sweep (`try_pair`) and the sweeper (`run_matchmaker`).
//! Two invariants:
//!
//!  - **The deck is frozen at enqueue** (§9.4). `freeze_choice` copies the deck or trio into the
//!    ticket and nothing below re-reads a saved deck, so an edit while queued cannot change the game
//!    (M7-T3's second acceptance item, §9.8).
//!  - **Both tickets are claimed in one atomic statement**, `tickets_claim_pair`. A match or series is
//!    created only after it returns true, so racing matchers cannot pair a ticket twice (M7-T3's race
//!    test). Losing the race is not an error.
//!
//! The window is `rating_window` from `config.rs`, so its numbers are written down once. The route
//! table is `app.rs`'s (SURFACE §11.2): `enqueue` (`POST /api/queue`, active), `dequeue` (`DELETE`,
//! active) and `population` (`GET /api/queue/population`, user).

use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use indexmap::{IndexMap, IndexSet};
use serde_json::{Value, json};

use crate::api::collection::caller_profile;
use crate::api::decks::{
    FrozenChoice, ModeChoiceInput, assert_not_in_series, freeze_choice, read_mode_choice,
};
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, bad_request, json};
use crate::api::series::start_series;
use crate::api::series_rules::{NewSeriesInput, NewSeriesSide};
use crate::app::{App, now_ms};
use crate::config::{MATCHMAKER_SWEEP_INTERVAL_SECONDS, rating_window};
use crate::db::store::{
    FrozenTrio, MatchSeat, Profile, QueueMode, SeriesRow, StartMatchInput, Ticket, TicketStatus,
};

/// Unit conversion, not configuration: `rating_window` speaks seconds, tickets are stamped in ms.
const MS_PER_SECOND: f64 = 1000.0;

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

// R143 — the end-to-end mode's optional seed

/// SPEC §11 R143: the server mints a match's seed and a client never supplies one, except that in
/// end-to-end mode the room and queue endpoints accept an optional seed and use it verbatim so a
/// networked spec can be seeded; outside that mode the field is rejected.
///
/// Rejected, not ignored: a client that sends a seed to a production server is told its request was
/// not understood.
///
/// Public because R143 names two endpoints: `actor/rooms.rs` reads the field through this same
/// function, so the rule is decided in one place.
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

/// ticket id -> the seed its enqueue asked for, until the ticket is paired or cancelled.
///
/// Module state because `try_pair` is also run by the sweeper, which never sees a handler's locals.
/// Nothing is written here outside end-to-end mode (`seed_override_of` refuses the field), and the
/// map empties as tickets resolve.
///
/// Not in SPEC, no R-row: `Ticket` carries no seed, since adding one would put a test-mode field into
/// the shape `db/**` persists, so R143's exception is confined to this map. `actor/rooms.rs` holds
/// `room code -> seed` for the same reason.
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

// Enqueue

/// §9.5's enqueue (the route's handler is `enqueue` below). The order of the checks is the order
/// §9.5 writes them, and it matters: a player already in a match (or a series, R264) or already
/// queued is told so before their deck is validated, so a stale client gets the useful error.
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
        return Err(ApiError::new(
            ApiErrorCode::AlreadyInMatch,
            "finish your current match first",
        ));
    }
    // R264: between the games of a series a profile is in no match, and still not free to queue.
    assert_not_in_series(app, &profile.id).await?;
    {
        let mut tx = app
            .db
            .begin(None)
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?;
        let open = tx
            .tickets_open_for_profile(&profile.id)
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?;
        tx.commit()
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?;
        if let Some(open) = open {
            return Err(ApiError::with_details(
                ApiErrorCode::AlreadyQueued,
                "you are already in the queue",
                json!({ "ticketId": open.id }),
            ));
        }
    }

    // §9.4: "checked by one validator module shared by client and server, at save and again at
    // queue". R253: a Best-of-1 deck on L2, L3, L5, L6 and a trio on L1–L6, both against the
    // current catalog; All Random needs no deck at all (R258).
    let frozen = freeze_choice(app, &profile.id, choice).await?;

    let (mode, deck, portrait, trio, lean_newest) = match frozen {
        // §9.4, §9.5: frozen. This copy lands in the ticket and later the match or series; the saved
        // deck is never read again. R642: the deck's portrait freezes with it.
        FrozenChoice::Bo1 { deck } => (
            QueueMode::Bo1,
            deck.cards.clone(),
            deck.portrait.clone(),
            None,
            false,
        ),
        FrozenChoice::Bo3 { trio } => (QueueMode::Bo3, Vec::new(), None, Some(trio), false),
        // R1372: the player's lean waits in the ticket for the deal, as a deck would.
        FrozenChoice::Random { lean_newest } => (QueueMode::Random, Vec::new(), None, None, lean_newest),
    };
    let ticket = Ticket {
        id: new_uuid(),
        profile_id: profile.id.clone(),
        rating: profile.rating,
        mode,
        deck,
        portrait,
        trio,
        lean_newest,
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
        let mut tx = app
            .db
            .begin(None)
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?;
        let existing = tx
            .tickets_open_for_profile(&profile.id)
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?;
        tx.commit()
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?;
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

    tracing::info!(
        event = "queue.enqueued",
        profileId = %profile.id,
        ticketId = %ticket.id,
        mode = ticket.mode.as_str(),
    );
    Ok(ticket)
}

// Pairing

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
        None => Err(ApiError::internal(format!(
            "Conquest ticket {} holds no trio",
            ticket.id
        ))),
    }
}

/// R263: forget the `open` skeleton `tickets_claim_pair` wrote, in its own transaction.
async fn discard_open(app: &App, match_id: &str) -> Result<(), ApiError> {
    let mut tx = app
        .db
        .begin(None)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    tx.matches_discard_open(match_id)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    tx.commit()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))
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
                NewSeriesSide {
                    profile_id: older.profile_id.clone(),
                    trio: trio_of(older)?,
                },
                NewSeriesSide {
                    profile_id: younger.profile_id.clone(),
                    trio: trio_of(younger)?,
                },
            ),
            seed_base,
            catalog_version: app.catalog.version.clone(),
            // R604: a series the queue pairs is ranked.
            ranked: true,
        };
        // One transaction, so the series row and `start_series`'s stale-ticket cancels land together: a
        // half-landed 'picking' row would hold both players out of the queue (assert_not_in_series)
        // until the pick deadline ran out (R333).
        let mut tx = app
            .db
            .begin(None)
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?;
        let series = start_series(app, input, &mut tx)
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?;
        tx.commit()
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?;
        Ok::<SeriesRow, ApiError>(series)
    }
    .await;
    let series = match made {
        Ok(series) => series,
        Err(error) => {
            // `start_paired_match`'s cleanup for a pair that becomes a series: the `open` skeleton
            // `tickets_claim_pair` wrote points at nothing and nothing reaps `open` rows, so delete it.
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
/// leaning on the newest set when that seat's ticket asked (R1372).
fn dealt_deck(seed: &str, seat: &str, lean_newest: bool) -> Vec<String> {
    crate::actor::engine::deal_random_deck(
        &format!("{seed}:{seat}-deck"),
        crate::actor::engine::lean_of(lean_newest),
    )
}

/// All Random's portrait for one seat (R642), dealt from the seed like its deck.
fn dealt_portrait(seed: &str, seat: &str) -> Option<String> {
    Some(jackioh_engine::wire::pick_portrait_from_seed(&format!("{seed}:portrait:{seat}")).to_string())
}

/// Creates the paired match — or, for Conquest, the series. Called only with two tickets this
/// process has already claimed, which is what makes it safe to write.
///
/// Best of 1 plays the two frozen decks. All Random (R258) deals both from the match seed and seat,
/// each leaning on the newest set exactly when its own ticket asked (R1372: a leaning ticket pairs
/// with one that does not), and the dealt decks go into the match row so `(seed, decks, log)` replays
/// it. Portraits ride the same way (R642): the ticket's own for Best of 1, a uniform pick dealt from
/// the seed for All Random.
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
            deck: if random {
                dealt_deck(&seed, "p1", a.lean_newest)
            } else {
                a.deck.clone()
            },
            portrait: if random {
                dealt_portrait(&seed, "p1")
            } else {
                a.portrait.clone().flatten()
            },
        },
        MatchSeat {
            profile_id: b.profile_id.clone(),
            player: jackioh_engine::PlayerId::P2,
            deck: if random {
                dealt_deck(&seed, "p2", b.lean_newest)
            } else {
                b.deck.clone()
            },
            portrait: if random {
                dealt_portrait(&seed, "p2")
            } else {
                b.portrait.clone().flatten()
            },
        },
    );

    // No `matches_create` here: `Registry::start` builds the row (seed, frozen decks, R79's clocks,
    // status live) and calls it itself, so a create here made it twice and the second player's enqueue
    // returned 500 after the match existed. The room path has the working shape: `rooms_claim` writes
    // the `open` row and the registry's create promotes it to `live`; `tickets_claim_pair` writes the
    // same skeleton.
    //
    // What stays in a transaction is the pair of `set_in_match` writes (§9.5's in-match state);
    // `profiles.current_match_id` is a foreign key into `matches`, satisfied by the `open` row.
    {
        let mut tx = app
            .db
            .begin(None)
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?;
        // §9.5: in-match state is set here and cleared by `results.rs` at every ending.
        tx.profiles_set_in_match(&a.profile_id, Some(match_id))
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?;
        tx.profiles_set_in_match(&b.profile_id, Some(match_id))
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?;
        tx.commit()
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?;
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
        // The `set_in_match` transaction has committed, so a failed start leaves the `open` skeleton
        // `tickets_claim_pair` wrote with both profiles pointing at it, and nothing reaps `open` rows:
        // the players would stay locked out. Undo it before the error stands (the tickets stay claimed).
        let cleanup = async {
            let mut tx = app
                .db
                .begin(None)
                .await
                .map_err(|error| ApiError::internal(error.to_string()))?;
            tx.profiles_set_in_match(&a.profile_id, None)
                .await
                .map_err(|error| ApiError::internal(error.to_string()))?;
            tx.profiles_set_in_match(&b.profile_id, None)
                .await
                .map_err(|error| ApiError::internal(error.to_string()))?;
            tx.commit()
                .await
                .map_err(|error| ApiError::internal(error.to_string()))?;
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
        mode = a.mode.as_str(),
        matchId = %match_id,
        tickets = %json_list(&[a.id.clone(), b.id.clone()]),
        ratings = %json_list(&[a.rating, b.rating]),
    );
    Ok(())
}

/// One pairing sweep. Oldest ticket first, each against the oldest qualifying opponent.
///
/// NOT IN SPEC, and no R-row yet — PROPOSED RULING for §11: the oldest ticket is paired first, against
/// the oldest opponent its window admits, not the closest in rating. §9.5 fixes the window and is
/// silent on the choice inside it; the window already is the rating rule, so picking the closest
/// re-applies it and leaves the longest waiter waiting again. Ties break on ticket id, so a sweep is
/// deterministic. Affects: §9.5, R108; `api/queue.rs`.
///
/// Answers how many matches this sweep made.
pub async fn try_pair(app: &Arc<App>) -> Result<usize, ApiError> {
    let now = now_ms();
    let mut tx = app
        .db
        .begin(None)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let mut open: Vec<Ticket> = tx
        .tickets_list_open()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    open.sort_by(|x, y| x.enqueued_at.cmp(&y.enqueued_at).then_with(|| x.id.cmp(&y.id)));
    if open.len() < 2 {
        tx.commit()
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?;
        return Ok(0);
    }

    // §9.5's "not in a match" holds at pairing too: a ticket left open while its owner joined a room
    // match must not become a second match, nor (R264) while they are in a series. One read each for
    // the sweep; `taken` covers the pairs this sweep makes.
    let owners: Vec<String> = open.iter().map(|ticket| ticket.profile_id.clone()).collect();
    let mut busy: IndexSet<String> = tx
        .profiles_get_many(&owners)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .into_iter()
        .filter(|profile| profile.in_match_id.is_some())
        .map(|profile| profile.id)
        .collect();
    for series in tx
        .series_active()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
    {
        busy.insert(series.sides.0.profile_id.clone());
        busy.insert(series.sides.1.profile_id.clone());
    }
    tx.commit()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let mut taken: IndexSet<String> = open
        .iter()
        .filter(|ticket| busy.contains(&ticket.profile_id))
        .map(|ticket| ticket.id.clone())
        .collect();
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
                let mut tx = app
                    .db
                    .begin(None)
                    .await
                    .map_err(|error| ApiError::internal(error.to_string()))?;
                let won = tx
                    .tickets_claim_pair(&a.id, &b.id, &match_id, now)
                    .await
                    .map_err(|error| ApiError::internal(error.to_string()))?;
                tx.commit()
                    .await
                    .map_err(|error| ApiError::internal(error.to_string()))?;
                won
            };
            if !won {
                // Another matcher got one of them. Never create a match for a ticket we did not claim:
                // find out which one is gone and either try another opponent or drop this ticket.
                let still = {
                    let mut tx = app
                        .db
                        .begin(None)
                        .await
                        .map_err(|error| ApiError::internal(error.to_string()))?;
                    let still = tx
                        .tickets_get(&a.id)
                        .await
                        .map_err(|error| ApiError::internal(error.to_string()))?;
                    tx.commit()
                        .await
                        .map_err(|error| ApiError::internal(error.to_string()))?;
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
/// `MATCHMAKER_SWEEP_INTERVAL_SECONDS`). `app.rs` spawns this loop (SURFACE §11.2) and aborting the
/// task stops it. Each sweep starts one interval after the last finished, on tokio's clock, so a test
/// drives it with `tokio::time::advance`.
pub async fn run_matchmaker(app: Arc<App>) {
    loop {
        tokio::time::sleep(Duration::from_secs(MATCHMAKER_SWEEP_INTERVAL_SECONDS as u64)).await;
        if let Err(error) = try_pair(&app).await {
            // A failed sweep must not kill the sweeper: the next one may well succeed.
            tracing::warn!(event = "matchmaker.sweep_failed", message = %error.message);
        }
    }
}

// Routes

/// `POST /api/queue` (active): §9.5's enqueue. `AuthLevel::Active` is §9.4's gate, so a pending
/// account gets 403 before this handler runs.
///
/// Pairing is attempted inline, so a player who pairs at once learns it from the response's status.
///
/// It takes `&Arc<App>` where SURFACE §11.2 writes `&App`: `Registry::start` takes `&Arc<App>`.
pub async fn enqueue(app: &Arc<App>, req: Req) -> ApiResult {
    enqueue_route(app, req).await
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

    let mut tx = app
        .db
        .begin(None)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let current = tx
        .tickets_get(&ticket.id)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let status = current
        .as_ref()
        .map(|current| current.status)
        .unwrap_or(ticket.status);
    // A paired Conquest ticket's `match_id` is game 1's reserved id, which is not a match anyone
    // can open yet: the player goes to the series to pick a deck, so it answers with that.
    let paired = status == TicketStatus::Matched;
    let series_id = if paired && ticket.mode == QueueMode::Bo3 {
        tx.series_active_for(&profile.id)
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?
            .map(|series| series.id)
    } else {
        None
    };
    let match_id = if paired && ticket.mode != QueueMode::Bo3 {
        current.as_ref().and_then(|current| current.match_id.clone())
    } else {
        None
    };
    let population = tx
        .tickets_count_open()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    tx.commit()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
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
/// NOT IN SPEC, and no R-row yet — PROPOSED RULING for §11: a queued player may cancel, and
/// cancelling is idempotent. §9.5 never says how a player leaves, yet its "not in a match" assertion
/// has no other way to become satisfiable again. Idempotent because the sweeper (R108) can pair a
/// ticket between the client deciding to cancel and the request arriving: cancel never unmakes a
/// pairing, it only closes a ticket still open.
/// Affects: §9.5, R108, R143; `api/queue.rs`, migration `0004_matches.sql`.
pub async fn dequeue(app: &Arc<App>, req: Req) -> ApiResult {
    dequeue_route(app, req).await
}

async fn dequeue_route(app: &App, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let mut tx = app
        .db
        .begin(None)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let Some(ticket) = tx
        .tickets_open_for_profile(&profile.id)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
    else {
        tx.commit()
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?;
        return Ok(json(200, json!({ "cancelled": false })));
    };
    tx.tickets_cancel(&ticket.id, now_ms())
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    tx.commit()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    // R143: a cancelled ticket will never be paired, so its seed is dropped with it.
    forget_seed(&ticket.id);
    tracing::info!(event = "queue.cancelled", profileId = %profile.id, ticketId = %ticket.id);
    Ok(json(200, json!({ "cancelled": true, "ticketId": ticket.id })))
}

/// `GET /api/queue/population` (user): §9.5's queue population instead of an endless spinner.
///
/// `AuthLevel::User`, not `Active` or `None`: the count is one aggregate that names nobody, and §9.4's
/// "no queue" is about joining, which `POST /api/queue` still refuses. `None` would give an
/// anonymous caller a live-traffic feed.
///
/// R257: the total, and per mode, so the lobby can say how many are waiting for each.
pub async fn population(app: &Arc<App>, _req: Req) -> ApiResult {
    population_route(app).await
}

async fn population_route(app: &App) -> ApiResult {
    let mut tx = app
        .db
        .begin(None)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let population = tx
        .tickets_count_open()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let by_mode = tx
        .tickets_count_open_by_mode()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    tx.commit()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    Ok(json(200, json!({ "population": population, "byMode": by_mode })))
}
