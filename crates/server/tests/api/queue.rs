//! SPEC §11 R165, R166 and R167 — the three rulings §9.5's queue makes that §9.5 itself does not
//! state (`src/api/queue.rs`) — and R253, R257, R258 and R264, the queue's modes.
//!
//!  - **R165**: queueing with no saved deck at all is a *loadout* failure, not a missing
//!    resource. A 404 would send a client looking for a route that is working correctly.
//!  - **R166**: a sweep pairs the oldest ticket first, against the oldest opponent its window
//!    admits — not the closest in rating — and ties break on ticket id, so a replay of the same
//!    open set pairs the same way.
//!  - **R167**: a queued player may cancel, and cancelling is idempotent. Cancel never unmakes a
//!    pairing; it only closes a ticket that is still open.
//!  - **R253**: a Best-of-1 deck is checked on L2, L3, L5, L6 and a Best-of-3 trio on L1–L6, by the
//!    real shared validator, with the decks' own names in its sentences.
//!  - **R257**: a ticket pairs only with a ticket of its own mode. A Best-of-3 pair becomes a series
//!    (R259), not a match.
//!  - **R258**: All Random deals both decks from the match seed and needs no saved deck.
//!  - **R264**: a profile in a series that is not over can neither queue nor be paired.
//!  - **R1372**: an All Random ticket's `leanNewest` leans the deck dealt to its own seat on the
//!    newest set, and only that seat's.
//!  - **R924000**: its `leanSet` picks the set: "newest" or any set that ships, and nothing else.
//!  - **R1450**: a start whose cleanup fails too is released by the reaper once the grace has run.
//!  - **R640001**: the window widens as §9.5 says and stops at `MAX_FAIR_GAP`, so two people
//!    further apart than that never pair.
//!  - **R640002**: the queue readout is each mode's expected wait and the last hour's games, every
//!    number counted; no read answers a count of open tickets.
//!
//! Everything runs on tokio's paused clock through `test_app()`, so `enqueuedAt` and the widening
//! window are set by this file rather than by the host clock (`app::now_ms()` reads tokio's clock).
//!
//! `test_app()` is the production wiring on the fake store: the real catalog, validator and registry,
//! which starts a real game on every pairing. So every deck here is real (the catalog's playable ids,
//! sliced into disjoint decks of the engine's `DECK_SIZE`) and every player owns the whole catalog
//! (R111's launch grant). A started match is read off its row (`matches_get`, the fake's
//! `tables.matches`). R165's "nothing saved" is a deck id that was never saved. A failing start is a
//! store fault (`on_call`). Surface contract: docs/v0.3.0/SURFACE.md §11.3.
//!
//! R166's fixture writes tickets straight into the store: the ruling is about *which* qualifying
//! opponent `try_pair` picks, and only exact control of each ticket's age and rating asks that. The
//! endpoint-driven tests (R165, R167) go through the router, so the route's auth declaration and
//! §9.4's gate take part.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use tokio::sync::MutexGuard;

use jackioh_engine::{DEFAULT_PORTRAIT, newest_shipped_set, pick_portrait_from_seed};
use jackioh_server::actor::engine::deal_random_deck;
use jackioh_server::api::collection::grant_entire_catalog;
use jackioh_server::api::http::AuthLevel;
use jackioh_server::api::queue::{e2e_seed_count, expected_wait_seconds, try_pair};
use jackioh_server::api::results::reap_open_matches;
use jackioh_server::app::{App, ROUTES, now_ms};
use jackioh_server::config::{
    BOT_FALLBACK_MAX_SECONDS, MAX_FAIR_GAP, MAX_SAVED_DECKS, MAX_SAVED_TRIOS, OPEN_MATCH_REAP_GRACE_SECONDS,
    QUEUE_READOUT_WINDOW_SECONDS, QUEUE_TICKET_STALE_MS, RATING_WINDOW_FULL_AFTER_SECONDS,
    RATING_WINDOW_START, RATING_WINDOW_WIDEN_BY, RATING_WINDOW_WIDEN_EVERY_SECONDS, SERIES_POLL_SECONDS,
    rating_window,
};
use jackioh_server::db::fake::{CallOutcome, FakeData, FakeTables};
use jackioh_server::db::store::{BotDeck, Db, NewBot, StoreError};

use crate::support::deps::{TestAppOptions, add_user, call, test_app, test_app_with};

// Plumbing (private copies: each test file of this binary keeps its own)

/// A port value built from an object literal, so the test depends on the JSON shape only.
fn from<T: DeserializeOwned>(value: Value) -> T {
    match serde_json::from_value(value.clone()) {
        Ok(parsed) => parsed,
        Err(error) => panic!("{error}: {value}"),
    }
}

fn to_json<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// One store call in its own transaction.
macro_rules! q {
    ($app:expr, $method:ident($($arg:expr),* $(,)?)) => {{
        let mut tx = $app.db.begin(None).await.expect("begin");
        let out = tx.$method($($arg),*).await.expect(stringify!($method));
        tx.commit().await.expect("commit");
        out
    }};
}

async fn fake(app: &App) -> MutexGuard<'_, FakeData> {
    match &app.db {
        Db::Fake(data) => data.lock().await,
        Db::Pg(_) => panic!("the server's unit tests run on the fake store"),
    }
}

/// R143's seeds are held in `queue.rs`'s process-wide map, which every test of this binary shares.
/// The tests that send a seed, or count the map, take this, so `e2e_seed_count()` sees only their own.
static SEED_MAP: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// `{ status, body }` of one request.
async fn request(app: &Arc<App>, method: &str, path: &str, token: &str, body: Option<Value>) -> (u16, Value) {
    let (status, _headers, body) = call(app, method, path, Some(token), body.unwrap_or(Value::Null)).await;
    (status, body)
}

async fn enqueue_with(app: &Arc<App>, token: &str, body: Value) -> (u16, Value) {
    request(app, "POST", "/api/queue", token, Some(body)).await
}

/// Best of 1 on one of the caller's decks.
async fn enqueue(app: &Arc<App>, token: &str, deck_id: &str) -> (u16, Value) {
    enqueue_with(app, token, json!({ "mode": "bo1", "deckId": deck_id })).await
}

async fn cancel(app: &Arc<App>, token: &str) -> (u16, Value) {
    request(app, "DELETE", "/api/queue", token, None).await
}

// Fixtures

/// The engine's deck size (BUILD §2: stated once, in `config.rs`).
fn deck_size() -> usize {
    usize::try_from(jackioh_engine::config::DECK_SIZE).expect("a deck size")
}

/// The catalog's playable ids (no tokens), in catalog order.
fn pool() -> Vec<String> {
    jackioh_cards::register_all();
    jackioh_cards::CATALOG
        .values()
        .filter(|def| !def.token)
        .map(|def| def.id.clone())
        .collect()
}

/// The playable ids, sliced into three disjoint legal decks.
fn decks_from() -> [Vec<String>; 3] {
    let pool = pool();
    let size = deck_size();
    [
        pool[..size].to_vec(),
        pool[size..size * 2].to_vec(),
        pool[size * 2..size * 3].to_vec(),
    ]
}

/// An active profile with a token that verifies as it, owning every card (R111).
async fn active_profile(app: &App, id: &str, rating: f64) -> String {
    let user_id = format!("user-{id}");
    fake(app)
        .await
        .seed_profile(json!({ "id": id, "userId": user_id, "status": "active", "rating": rating }));
    grant_entire_catalog(app, id, None)
        .await
        .expect("the launch grant");
    add_user(app, &user_id, &format!("{id}@example.test"), true)
}

/// A pending profile with a token that verifies as it (§9.4).
async fn pending_profile(app: &App, id: &str) -> String {
    let user_id = format!("user-{id}");
    fake(app)
        .await
        .seed_profile(json!({ "id": id, "userId": user_id, "status": "pending" }));
    grant_entire_catalog(app, id, None)
        .await
        .expect("the launch grant");
    add_user(app, &user_id, &format!("{id}@example.test"), true)
}

static NEXT_ID: AtomicU32 = AtomicU32::new(0);

/// A client-minted deck or trio id (R256): a UUID, unique within the file.
fn uuid() -> String {
    let n = NEXT_ID.fetch_add(1, Ordering::SeqCst) + 1;
    format!("00000000-0000-4000-8000-{n:012}")
}

/// Saves one deck for the profile under `id`, straight into the store.
async fn save_deck_as(app: &App, id: &str, profile_id: &str, cards: &[String], name: &str) {
    let now = now_ms();
    let outcome = q!(
        app,
        decks_upsert(
            &from(json!({
                "id": id,
                "profileId": profile_id,
                "name": name,
                "cards": cards,
                "portrait": null,
                "catalogVersion": app.catalog.version,
                "createdAt": now,
                "updatedAt": now,
            })),
            MAX_SAVED_DECKS as i64,
        )
    );
    assert_eq!(to_json(&outcome), json!("created"));
}

/// Saves one deck for the profile, straight into the store, and returns its id.
async fn save_deck_for(app: &App, profile_id: &str, cards: &[String], name: &str) -> String {
    let id = uuid();
    save_deck_as(app, &id, profile_id, cards, name).await;
    id
}

/// Saves a trio for the profile over the given decks (or three fresh ones) and returns its id.
async fn save_trio_for(
    app: &App,
    profile_id: &str,
    deck_ids: Option<[Option<String>; 3]>,
    name: &str,
) -> String {
    let slots = match deck_ids {
        Some(slots) => slots,
        None => {
            let [one, two, three] = decks_from();
            [
                Some(save_deck_for(app, profile_id, &one, &format!("{profile_id} one")).await),
                Some(save_deck_for(app, profile_id, &two, &format!("{profile_id} two")).await),
                Some(save_deck_for(app, profile_id, &three, &format!("{profile_id} three")).await),
            ]
        }
    };
    let id = uuid();
    let now = now_ms();
    let outcome = q!(
        app,
        trios_upsert(
            &from(json!({
                "id": id,
                "profileId": profile_id,
                "name": name,
                "deckIds": slots,
                "createdAt": now,
                "updatedAt": now,
            })),
            MAX_SAVED_TRIOS as i64,
        )
    );
    assert_eq!(to_json(&outcome), json!("created"));
    id
}

async fn open_count(app: &App) -> i64 {
    let count = q!(app, tickets_count_open());
    to_json(&count).as_i64().expect("a count")
}

async fn ticket(app: &App, id: &str) -> Value {
    q!(app, tickets_get(id))
        .map(|row| to_json(&row))
        .unwrap_or(Value::Null)
}

async fn tickets_table(app: &App) -> Vec<Value> {
    to_json(&fake(app).await.tables.tickets)
        .as_array()
        .cloned()
        .unwrap_or_default()
}

async fn matches_table(app: &App) -> Vec<Value> {
    to_json(&fake(app).await.tables.matches)
        .as_array()
        .cloned()
        .unwrap_or_default()
}

async fn series_table(app: &App) -> Vec<Value> {
    to_json(&fake(app).await.tables.series)
        .as_array()
        .cloned()
        .unwrap_or_default()
}

async fn in_match_of(app: &App, profile_id: &str) -> Value {
    q!(app, profiles_get_by_id(profile_id))
        .map(|profile| to_json(&profile)["inMatchId"].clone())
        .unwrap_or(Value::Null)
}

/// A field a Rust row may leave absent or `null`: either reads as `null`.
fn nullable(row: &Value, key: &str) -> Value {
    row.get(key).cloned().unwrap_or(Value::Null)
}

/// The deck the started match gave this profile's seat.
async fn seat_deck(app: &App, match_id: &str, profile_id: &str) -> Value {
    let row = q!(app, matches_get(match_id))
        .map(|row| to_json(&row))
        .unwrap_or(Value::Null);
    let seat = row["players"]
        .as_array()
        .and_then(|players| players.iter().position(|id| id == profile_id));
    match seat {
        Some(seat) => row["decks"][seat].clone(),
        None => Value::Null,
    }
}

/// Writes a ticket straight into the store, with the age and rating the case needs.
///
/// `enqueuedAt` is an offset from the clock's now, so "older" reads as a smaller number and the
/// widening window (`rating_window`) sees exactly the wait this test intends.
struct SeedTicket<'a> {
    id: &'a str,
    profile_id: &'a str,
    rating: i64,
    waited_ms: i64,
    mode: &'a str,
}

async fn seed_ticket(app: &App, input: SeedTicket<'_>) -> Value {
    fake(app)
        .await
        .seed_profile(json!({ "id": input.profile_id, "status": "active", "rating": input.rating }));
    let [deck, ..] = decks_from();
    let deck = if input.mode == "bo1" {
        json!(deck)
    } else {
        json!([])
    };
    let trio = if input.mode == "bo3" {
        trio_named(input.profile_id)
    } else {
        Value::Null
    };
    let ticket = json!({
        "id": input.id,
        "profileId": input.profile_id,
        "rating": input.rating,
        "mode": input.mode,
        "deck": deck,
        "trio": trio,
        "catalogVersion": app.catalog.version,
        "enqueuedAt": now_ms() - input.waited_ms,
        "status": "open",
        "matchId": null,
    });
    q!(app, tickets_insert(&from(ticket.clone())));
    ticket
}

/// A frozen trio whose every name says whose it is, over three legal decks.
fn trio_named(owner: &str) -> Value {
    let decks = decks_from();
    let deck = |n: usize| json!({ "name": format!("{owner} {n}"), "cards": decks[n - 1] });
    json!({ "name": format!("{owner}'s trio"), "decks": [deck(1), deck(2), deck(3)] })
}

/// The two profile ids of every match the sweeps created, in seat order.
async fn paired_profiles(app: &App) -> Vec<Value> {
    matches_table(app)
        .await
        .iter()
        .map(|row| row["players"].clone())
        .collect()
}

/// §9.5's mutual-window test, spelled out: the gap has to sit inside *both* windows.
///
/// Every "was left waiting" assertion below needs this first, or it would be satisfied by an
/// opponent the window excluded anyway — and the test would no longer be about R166's *choice*.
fn expect_qualifies(a: (i64, i64), b: (i64, i64)) {
    let gap = (a.0 - b.0).abs() as f64;
    assert!(gap <= rating_window(a.1 as f64 / 1000.0));
    assert!(gap <= rating_window(b.1 as f64 / 1000.0));
}

/// A series row with these two profiles in it, in the given status.
fn series_with(app: &App, p1: &str, p2: &str, status: &str) -> Value {
    json!({
        "id": format!("series-of-{p1}"),
        "sides": [
            { "profileId": p1, "trio": trio_named(p1), "wins": 0, "pick": null },
            { "profileId": p2, "trio": trio_named(p2), "wins": 0, "pick": null },
        ],
        "catalogVersion": app.catalog.version,
        "ranked": true,
        "seedBase": "seed-base",
        "status": status,
        "games": [],
        "nextMatchId": format!("reserved-{p1}"),
        "pickDeadline": null,
        "winner": null,
        "endReason": null,
        "ratingBefore": null,
        "ratingAfter": null,
        "createdAt": 0,
        "updatedAt": 0,
        "endedAt": null,
        "version": 1,
    })
}

/// Saves a deck through `PUT /api/decks/:id`, the endpoint a player would use.
async fn save(app: &Arc<App>, token: &str, deck_id: &str, cards: &[String], portrait: Option<&str>) -> u16 {
    let mut body = json!({ "name": "The deck", "cards": cards, "catalogVersion": app.catalog.version });
    if let Some(portrait) = portrait {
        body["portrait"] = json!(portrait);
    }
    request(app, "PUT", &format!("/api/decks/{deck_id}"), token, Some(body))
        .await
        .0
}

/// The store's call log and a fault to raise in it.
fn record_calls(
    app_data: &mut FakeData,
    fail: impl Fn(&[String], &str) -> bool + Send + Sync + 'static,
) -> Arc<StdMutex<Vec<String>>> {
    let calls: Arc<StdMutex<Vec<String>>> = Arc::default();
    let seen = Arc::clone(&calls);
    app_data.on_call = Some(Arc::new(move |method: &str| {
        let mut seen = seen.lock().expect("calls");
        let refuse = fail(&seen, method);
        seen.push(method.to_string());
        if refuse {
            return Err(StoreError::Other(format!("{method} refused by the test")));
        }
        Ok(())
    }));
    calls
}

// R165

mod r165_queueing_with_no_saved_deck_at_all {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r165_reports_a_loadout_failure_never_a_404_for_a_profile_that_has_never_saved_one() {
        let app = test_app().await;
        let token = active_profile(&app, "never-built", 1000.0).await;

        let (status, body) = enqueue(&app, &token, &uuid()).await;

        // A queue-time refusal the player fixes in the deckbuilder is a loadout failure: 422, like L1–L6.
        assert_eq!(body["error"]["code"], "loadout_invalid");
        assert_eq!(status, 422);
        // A 404 would send a client after a route that works.
        assert_ne!(status, 404);
        assert_ne!(body["error"]["code"], "not_found");
        assert_ne!(body["error"]["code"], "stale_catalog");
        let message = body["error"]["message"].as_str().unwrap_or("").to_lowercase();
        assert!(message.contains("deck"), "message: {message}");

        assert_eq!(open_count(&app).await, 0);
    }

    #[tokio::test(start_paused = true)]
    async fn r165s_control_the_same_profile_and_the_same_route_queue_fine_once_a_deck_is_saved() {
        // Control: only the saved deck changes between the two calls.
        let app = test_app().await;
        let token = active_profile(&app, "builder", 1000.0).await;
        let deck_id = uuid();

        assert_eq!(enqueue(&app, &token, &deck_id).await.0, 422);

        let [deck, ..] = decks_from();
        save_deck_as(&app, &deck_id, "builder", &deck, "builder's deck").await;
        let (status, body) = enqueue(&app, &token, &deck_id).await;

        assert_eq!(status, 200);
        assert_eq!(body["status"], "open");
        assert_eq!(open_count(&app).await, 1);
    }
}

// R166

mod r166_r108_which_qualifying_opponent_a_sweep_pairs {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r166_pairs_the_oldest_ticket_against_the_oldest_opponent_its_window_admits_not_the_closest() {
        // `near` is a far better rating match for `oldest` than `mid` is, and R166 must NOT choose it,
        // because it queued later. `oldest` (1000, waited 30 s) pairs first; `mid` (1080, 20 s) is the oldest
        // that qualifies; `near` (1005, 1 s) is closest in rating. All windows are ±100 at sweep time.
        let app = test_app().await;
        let oldest = (1000, 30_000);
        let mid = (1080, 20_000);
        let near = (1005, 1_000);

        // PREMISE: both candidates are inside both windows, so what follows is R166's *choice* (§9.5).
        expect_qualifies(oldest, mid);
        expect_qualifies(oldest, near);
        assert!((oldest.0 - near.0).abs() < (oldest.0 - mid.0).abs());

        let t = |id, profile_id, (rating, waited_ms): (i64, i64)| SeedTicket {
            id,
            profile_id,
            rating,
            waited_ms,
            mode: "bo1",
        };
        seed_ticket(&app, t("t-oldest", "oldest", oldest)).await;
        seed_ticket(&app, t("t-mid", "mid", mid)).await;
        seed_ticket(&app, t("t-near", "near", near)).await;

        let made = try_pair(&app).await.expect("a sweep").made;

        // PREMISE: exactly one pair was made.
        assert_eq!(made, 1);
        assert_eq!(paired_profiles(&app).await, vec![json!(["oldest", "mid"])]);

        // The closer-rated newcomer still waits: a closest-rating matcher would have left `mid`.
        let still_waiting = ticket(&app, "t-near").await;
        assert_eq!(still_waiting["status"], "open");
        assert_eq!(still_waiting["matchId"], Value::Null);
        assert_eq!(ticket(&app, "t-mid").await["status"], "matched");
    }

    #[tokio::test(start_paused = true)]
    async fn r166_pairs_the_oldest_ticket_first_when_two_pairs_are_available_in_one_sweep() {
        // Four tickets, all mutually in window, so the only question is the order the sweep works in.
        let app = test_app().await;
        for (id, profile_id, rating, waited_ms) in [
            ("t-1", "first", 1000, 40_000),
            ("t-2", "second", 1010, 30_000),
            ("t-3", "third", 1020, 20_000),
            ("t-4", "fourth", 1030, 10_000),
        ] {
            seed_ticket(
                &app,
                SeedTicket {
                    id,
                    profile_id,
                    rating,
                    waited_ms,
                    mode: "bo1",
                },
            )
            .await;
        }

        let made = try_pair(&app).await.expect("a sweep").made;

        assert_eq!(made, 2);
        assert_eq!(
            paired_profiles(&app).await,
            vec![json!(["first", "second"]), json!(["third", "fourth"])]
        );
        assert_eq!(open_count(&app).await, 0);
    }

    #[tokio::test(start_paused = true)]
    async fn r166_breaks_a_tie_on_ticket_id_so_the_same_open_set_always_pairs_the_same_way() {
        // `t-zebra` is inserted before `t-alpha` at the same instant: the id tie-break pairs `oldest` with `t-alpha`.
        let app = test_app().await;
        let oldest = (1000, 30_000);
        let zebra = (1010, 5_000);
        let alpha = (1020, 5_000);

        // PREMISE: both tied candidates qualify, so the loser loses on its id alone.
        expect_qualifies(oldest, zebra);
        expect_qualifies(oldest, alpha);

        let t = |id, profile_id, (rating, waited_ms): (i64, i64)| SeedTicket {
            id,
            profile_id,
            rating,
            waited_ms,
            mode: "bo1",
        };
        seed_ticket(&app, t("t-oldest", "oldest", oldest)).await;
        seed_ticket(&app, t("t-zebra", "zebra", zebra)).await;
        seed_ticket(&app, t("t-alpha", "alpha", alpha)).await;

        // PREMISE: the store hands them over in insertion order, so the assertion is about `try_pair`'s sort.
        let open: Vec<Value> = to_json(&q!(app, tickets_list_open()))
            .as_array()
            .map(|rows| rows.iter().map(|row| row["id"].clone()).collect())
            .unwrap_or_default();
        assert_eq!(open, vec![json!("t-oldest"), json!("t-zebra"), json!("t-alpha")]);

        assert_eq!(try_pair(&app).await.expect("a sweep").made, 1);
        assert_eq!(paired_profiles(&app).await, vec![json!(["oldest", "alpha"])]);
        assert_eq!(ticket(&app, "t-zebra").await["status"], "open");
    }

    #[tokio::test(start_paused = true)]
    async fn r166_stays_inside_95s_window_an_opponent_both_windows_refuse_is_not_paired_at_all() {
        // Control: oldest-first never drags in someone the window excludes (both windows ±100, gap 300).
        let app = test_app().await;
        seed_ticket(
            &app,
            SeedTicket {
                id: "t-low",
                profile_id: "low",
                rating: 1000,
                waited_ms: 1_000,
                mode: "bo1",
            },
        )
        .await;
        seed_ticket(
            &app,
            SeedTicket {
                id: "t-high",
                profile_id: "high",
                rating: 1300,
                waited_ms: 500,
                mode: "bo1",
            },
        )
        .await;

        assert_eq!(try_pair(&app).await.expect("a sweep").made, 0);
        assert!(matches_table(&app).await.is_empty());
        assert_eq!(open_count(&app).await, 2);

        // …and once both windows have widened past the gap, the same two pair (§9.5; R640001 stops them at MAX_FAIR_GAP, still past it).
        tokio::time::advance(Duration::from_millis(120_000)).await;
        // SPEC §11 R897000: the wait outlasts a presence window, and live clients poll through it,
        // so both are heard from before the sweep judges them.
        app.queue_presence.touch("low", now_ms());
        app.queue_presence.touch("high", now_ms());
        assert_eq!(try_pair(&app).await.expect("a sweep").made, 1);
        assert_eq!(paired_profiles(&app).await, vec![json!(["low", "high"])]);
    }
}

// SPEC §11 R897000 — an abandoned ticket expires before it seats anyone

mod r897000_an_abandoned_ticket_expires_before_it_seats_anyone {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r897000_stale_tickets_are_cancelled_by_the_sweep_and_pair_nobody() {
        let app = test_app().await;
        // PREMISE: the two would pair at once — same rating, same instant — so what follows is
        // the expiry, not the window.
        for (id, profile_id) in [("t-gone-1", "gone-1"), ("t-gone-2", "gone-2")] {
            seed_ticket(
                &app,
                SeedTicket {
                    id,
                    profile_id,
                    rating: 1000,
                    waited_ms: 0,
                    mode: "bo1",
                },
            )
            .await;
        }
        tokio::time::advance(Duration::from_millis((QUEUE_TICKET_STALE_MS + 1) as u64)).await;

        let report = try_pair(&app).await.expect("a sweep");

        assert_eq!(report.made, 0);
        assert!(matches_table(&app).await.is_empty());
        for id in ["t-gone-1", "t-gone-2"] {
            assert_eq!(ticket(&app, id).await["status"], "cancelled");
        }
        assert_eq!(open_count(&app).await, 0);
    }

    #[tokio::test(start_paused = true)]
    async fn r897000_a_ticket_that_keeps_being_touched_never_expires() {
        let app = test_app().await;
        for (id, profile_id) in [("t-kept-1", "kept-1"), ("t-kept-2", "kept-2")] {
            seed_ticket(
                &app,
                SeedTicket {
                    id,
                    profile_id,
                    rating: 1000,
                    waited_ms: 0,
                    mode: "bo1",
                },
            )
            .await;
        }
        // Three waits of half a window, heard from after each: never silent a full window.
        for _ in 0..3 {
            tokio::time::advance(Duration::from_millis(6_000)).await;
            app.queue_presence.touch("kept-1", now_ms());
            app.queue_presence.touch("kept-2", now_ms());
        }

        let report = try_pair(&app).await.expect("a sweep");

        assert_eq!(report.made, 1);
        assert_eq!(paired_profiles(&app).await, vec![json!(["kept-1", "kept-2"])]);
    }

    #[tokio::test(start_paused = true)]
    async fn r897000_any_signed_in_request_refreshes_the_ticket() {
        let app = test_app().await;
        let token = active_profile(&app, "polling", 1000.0).await;
        seed_ticket(
            &app,
            SeedTicket {
                id: "t-polling",
                profile_id: "polling",
                rating: 1000,
                waited_ms: 0,
                mode: "bo1",
            },
        )
        .await;
        tokio::time::advance(Duration::from_millis((QUEUE_TICKET_STALE_MS - 1_000) as u64)).await;
        // The queued client's own poll: a signed-in read with no queue body at all.
        let (status, _) = request(&app, "GET", "/api/auth/me", &token, None).await;
        assert_eq!(status, 200);
        // Silent ever since, but heard from a window ago: still open, and still alone.
        tokio::time::advance(Duration::from_millis(6_000)).await;

        let report = try_pair(&app).await.expect("a sweep");

        assert_eq!(report.made, 0);
        assert_eq!(ticket(&app, "t-polling").await["status"], "open");
    }

    /// Two tickets further apart than `RATING_WINDOW_START`, enqueued together: the pair is made
    /// only once the window has widened to the gap, and `sweep_every_poll` runs a sweep after
    /// each `SERIES_POLL_SECONDS`, touching those of `heard` as a polling client would. Yields the
    /// seconds waited when the sweep paired them, if it did within `RATING_WINDOW_FULL_AFTER_SECONDS`.
    async fn sweep_every_poll(app: &Arc<App>, heard: &[&str]) -> Option<i64> {
        let mut waited = 0;
        while waited < RATING_WINDOW_FULL_AFTER_SECONDS {
            tokio::time::advance(Duration::from_secs(SERIES_POLL_SECONDS as u64)).await;
            waited += SERIES_POLL_SECONDS;
            for profile_id in heard {
                app.queue_presence.touch(profile_id, now_ms());
            }
            if try_pair(app).await.expect("a sweep").made == 1 {
                return Some(waited);
            }
        }
        None
    }

    /// The gap, and the seconds the widening window takes to reach it (R640001).
    const WINDOW_GAP: i64 = RATING_WINDOW_START + 3 * RATING_WINDOW_WIDEN_BY;
    const WINDOW_REACHES_GAP_AFTER_SECONDS: i64 =
        (WINDOW_GAP - RATING_WINDOW_START) / RATING_WINDOW_WIDEN_BY * RATING_WINDOW_WIDEN_EVERY_SECONDS;

    /// PREMISE of the test that waits it out: the wait is longer than the TTL.
    const _: () = assert!(WINDOW_REACHES_GAP_AFTER_SECONDS * 1_000 > QUEUE_TICKET_STALE_MS);

    async fn seed_far_apart(app: &App, ids: [(&str, &str, i64); 2]) {
        for (id, profile_id, rating) in ids {
            seed_ticket(
                app,
                SeedTicket {
                    id,
                    profile_id,
                    rating,
                    waited_ms: 0,
                    mode: "bo1",
                },
            )
            .await;
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r897000_two_heard_tickets_wait_out_the_rating_window_past_the_ttl() {
        let app = test_app().await;
        seed_far_apart(
            &app,
            [("t-near", "near", 1000), ("t-far", "far", 1000 + WINDOW_GAP)],
        )
        .await;

        let paired_after = sweep_every_poll(&app, &["near", "far"]).await;

        // The wait runs past `QUEUE_TICKET_STALE_MS` (below), and both are still there at its end.
        assert_eq!(paired_after, Some(WINDOW_REACHES_GAP_AFTER_SECONDS));
        assert_eq!(paired_profiles(&app).await.len(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn r897000_a_silent_ticket_does_not_outwait_the_rating_window() {
        let app = test_app().await;
        seed_far_apart(
            &app,
            [("t-near", "near", 1000), ("t-far", "far", 1000 + WINDOW_GAP)],
        )
        .await;

        // `far` is an API-only client that enqueued and never asked for anything again.
        let paired_after = sweep_every_poll(&app, &["near"]).await;

        assert_eq!(paired_after, None);
        assert_eq!(ticket(&app, "t-far").await["status"], "cancelled");
        assert_eq!(ticket(&app, "t-near").await["status"], "open");
        assert!(matches_table(&app).await.is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn r897000_get_me_answers_queued_while_a_ticket_is_open() {
        let app = test_app().await;
        let token = active_profile(&app, "queuer", 1000.0).await;
        let [deck, ..] = decks_from();
        let deck_id = save_deck_for(&app, "queuer", &deck, "queuer's deck").await;

        let (_, before) = request(&app, "GET", "/api/auth/me", &token, None).await;
        assert_eq!(before["queued"], false);

        let (status, queued) = enqueue(&app, &token, &deck_id).await;
        assert_eq!(status, 200);
        assert_eq!(queued["status"], "open");
        let (_, during) = request(&app, "GET", "/api/auth/me", &token, None).await;
        assert_eq!(during["queued"], true);

        let (status, _) = cancel(&app, &token).await;
        assert_eq!(status, 200);
        let (_, after) = request(&app, "GET", "/api/auth/me", &token, None).await;
        assert_eq!(after["queued"], false);
    }
}

// R167

mod r167_r108_r143_how_a_player_leaves_the_queue {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r167_cancels_an_open_ticket_and_says_so() {
        let app = test_app().await;
        let token = active_profile(&app, "leaver", 1000.0).await;
        let [deck, ..] = decks_from();
        let deck_id = save_deck_for(&app, "leaver", &deck, "leaver's deck").await;

        // PREMISE: there is something to cancel.
        let (status, queued) = enqueue(&app, &token, &deck_id).await;
        let ticket_id = queued["ticketId"].as_str().unwrap_or("").to_string();
        assert_eq!(status, 200);
        assert!(!ticket_id.is_empty());
        assert_eq!(open_count(&app).await, 1);

        let (status, body) = cancel(&app, &token).await;

        assert_eq!(status, 200);
        assert_eq!(body, json!({ "cancelled": true, "ticketId": ticket_id }));
        assert_eq!(ticket(&app, &ticket_id).await["status"], "cancelled");
        assert_eq!(open_count(&app).await, 0);

        // §9.5's enqueue condition (active, not in a match) holds again, which is why R167 exists.
        let (status, again) = enqueue(&app, &token, &deck_id).await;
        assert_eq!(status, 200);
        assert_ne!(again["ticketId"], json!(ticket_id));
    }

    #[tokio::test(start_paused = true)]
    async fn r167_is_idempotent_a_second_cancel_is_told_nothing_was_cancelled_not_given_an_error() {
        let app = test_app().await;
        let token = active_profile(&app, "twice", 1000.0).await;
        let [deck, ..] = decks_from();
        let deck_id = save_deck_for(&app, "twice", &deck, "twice's deck").await;

        // PREMISE: cancel the real thing first, so "cancelled: false" means "already gone".
        let ticket_id = enqueue(&app, &token, &deck_id).await.1["ticketId"]
            .as_str()
            .unwrap_or("")
            .to_string();
        assert_eq!(cancel(&app, &token).await.1["cancelled"], true);

        let (status, second) = cancel(&app, &token).await;

        assert_eq!(status, 200);
        assert_eq!(second, json!({ "cancelled": false }));
        assert_eq!(ticket(&app, &ticket_id).await["status"], "cancelled");
        assert_eq!(tickets_table(&app).await.len(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn r167_never_unmakes_a_pairing_a_ticket_paired_a_moment_earlier_reports_nothing_cancelled() {
        let app = test_app().await;
        let one = active_profile(&app, "p-one", 1000.0).await;
        let two = active_profile(&app, "p-two", 1000.0).await;
        let [first_deck, second_deck, _] = decks_from();
        let deck_one = save_deck_for(&app, "p-one", &first_deck, "p-one's deck").await;
        let deck_two = save_deck_for(&app, "p-two", &second_deck, "p-two's deck").await;

        let first_ticket = enqueue(&app, &one, &deck_one).await.1["ticketId"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let (_, paired) = enqueue(&app, &two, &deck_two).await;

        // PREMISE: the race R167 is about happened: the sweeper on enqueue paired them.
        assert_eq!(paired["status"], "matched");
        assert_ne!(paired["matchId"], Value::Null);
        assert_eq!(ticket(&app, &first_ticket).await["status"], "matched");

        let (status, late) = cancel(&app, &one).await;

        assert_eq!(status, 200);
        assert_eq!(late, json!({ "cancelled": false }));
        let stands = ticket(&app, &first_ticket).await;
        assert_eq!(stands["status"], "matched");
        assert_eq!(stands["matchId"], paired["matchId"]);
        assert_eq!(matches_table(&app).await.len(), 1);
        assert_eq!(in_match_of(&app, "p-one").await, paired["matchId"]);
    }

    #[tokio::test(start_paused = true)]
    async fn r167_answers_a_player_who_was_never_queued_the_same_way_with_no_error_to_act_on() {
        let app = test_app().await;
        let token = active_profile(&app, "never-queued", 1000.0).await;

        let (status, body) = cancel(&app, &token).await;

        assert_eq!(status, 200);
        assert_eq!(body, json!({ "cancelled": false }));
        assert!(tickets_table(&app).await.is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn r167s_cancel_is_94_gated_like_the_rest_of_the_queue_a_pending_account_gets_403() {
        // Control: `DELETE /api/queue` answers `cancelled: false` only for a ticket that is not there.
        let app = test_app().await;
        let token = pending_profile(&app, "pending").await;

        let (status, body) = cancel(&app, &token).await;

        assert_eq!(status, 403);
        assert_eq!(body["error"]["code"], "account_pending");
    }
}

// §9.4's gate on the queue (BUILD M6-T1)

/// BUILD M6-T1: "a pending account cannot call collection, loadout or queue endpoints (403)". The
/// other routes are checked in `collection.rs` and `decks.rs`; this is enqueueing, which §9.4 names
/// ("no collection, loadout, queue or match"). R167's block is about *leaving* the queue.
mod section_94s_gate_on_the_queue {
    use super::*;

    #[test]
    fn declares_both_queue_mutations_active_so_the_gate_is_on_the_route_and_not_in_a_handler() {
        let declared: Vec<(String, bool)> = ROUTES
            .iter()
            .filter(|route| route.1 == "/api/queue")
            .map(|route| (route.0.to_string(), matches!(route.2, AuthLevel::Active)))
            .collect();
        // `src/api/queue.rs` leaves the 403 to §9.4's gate, which holds only while these say `Active`.
        assert_eq!(
            declared,
            vec![("POST".to_string(), true), ("DELETE".to_string(), true)]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_pending_account_gets_403_from_post_api_queue_and_nothing_is_queued_on_the_way_out() {
        let app = test_app().await;
        let token = pending_profile(&app, "pending").await;
        let [deck, ..] = decks_from();
        let deck_id = save_deck_for(&app, "pending", &deck, "pending's deck").await;

        let (status, body) = enqueue(&app, &token, &deck_id).await;

        assert_eq!(status, 403);
        assert_eq!(body["error"]["code"], "account_pending");
        assert!(tickets_table(&app).await.is_empty());
        assert_eq!(open_count(&app).await, 0);
    }

    #[tokio::test(start_paused = true)]
    async fn the_control_the_same_profile_and_the_same_request_queue_fine_once_the_account_is_active() {
        // Control: the deck is saved first in both calls, so only `status` differs.
        let app = test_app().await;
        let token = pending_profile(&app, "activating").await;
        let [deck, ..] = decks_from();
        let deck_id = save_deck_for(&app, "activating", &deck, "activating's deck").await;

        assert_eq!(enqueue(&app, &token, &deck_id).await.0, 403);

        q!(app, profiles_set_status("activating", from(json!("active"))));
        let (status, body) = enqueue(&app, &token, &deck_id).await;

        assert_eq!(status, 200);
        assert_eq!(body["status"], "open");
    }
}

// §9.4 / §9.5 / §9.8 — the deck is frozen into the ticket

/// §9.8's abuse vector: "Deck swapped after matchmaking → decks are frozen into the ticket" (§9.4).
/// Enqueue, then save a different deck under the same id through `PUT /api/decks/:id`, then pair.
/// Every case carries its control: a match using deck A proves nothing unless the same fixture, with
/// the save moved earlier, uses deck B.
mod section_98_decks_are_frozen_into_the_queue_ticket {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn a_deck_saved_after_enqueue_does_not_change_the_match_that_ticket_becomes() {
        let app = test_app().await;
        let swapper = active_profile(&app, "swapper", 1000.0).await;
        let rival = active_profile(&app, "rival", 1000.0).await;
        let [frozen, rival_cards, substitute] = decks_from();
        let deck_id = uuid();
        assert_eq!(save(&app, &swapper, &deck_id, &frozen, None).await, 200);
        let rival_deck = save_deck_for(&app, "rival", &rival_cards, "rival's deck").await;

        // PREMISE: the two decks share no card, so a match on the new deck could not look the same.
        assert!(!frozen.is_empty());
        assert!(frozen.iter().all(|card_id| !substitute.contains(card_id)));

        let (_, queued) = enqueue_with(&app, &swapper, json!({ "mode": "bo1", "deckId": deck_id })).await;
        let ticket_id = queued["ticketId"].as_str().unwrap_or("").to_string();
        assert_eq!(queued["status"], "open");
        assert_eq!(ticket(&app, &ticket_id).await["deck"], json!(frozen));

        // The swap, while the ticket is still open.
        assert_eq!(save(&app, &swapper, &deck_id, &substitute, None).await, 200);
        // PREMISE: the save landed; a rejected save would make every assertion below pass vacuously.
        let saved = q!(app, decks_get(&deck_id))
            .map(|deck| to_json(&deck))
            .unwrap_or(Value::Null);
        assert_eq!(saved["cards"], json!(substitute));
        assert_eq!(ticket(&app, &ticket_id).await["deck"], json!(frozen));

        let (_, paired) = enqueue_with(&app, &rival, json!({ "mode": "bo1", "deckId": rival_deck })).await;
        assert_eq!(paired["status"], "matched");
        let match_id = paired["matchId"].as_str().expect("a match id").to_string();

        // §9.4, §9.5: the match runs the deck the ticket froze, not the one the player is holding now.
        assert_eq!(seat_deck(&app, &match_id, "swapper").await, json!(frozen));
        let row = q!(app, matches_get(&match_id))
            .map(|row| to_json(&row))
            .unwrap_or(Value::Null);
        assert!(
            !row["decks"]
                .to_string()
                .contains(&format!("\"{}\"", substitute[0]))
        );
    }

    #[tokio::test(start_paused = true)]
    async fn the_control_the_same_swap_made_before_the_enqueue_is_the_deck_the_match_uses() {
        // Control: the save moves before enqueue, so the match uses deck B; else the test above could pass on a queue that ignored saved decks.
        let app = test_app().await;
        let swapper = active_profile(&app, "early", 1000.0).await;
        let rival = active_profile(&app, "late", 1000.0).await;
        let [first, rival_cards, substitute] = decks_from();
        let deck_id = uuid();
        assert_eq!(save(&app, &swapper, &deck_id, &first, None).await, 200);
        let rival_deck = save_deck_for(&app, "late", &rival_cards, "late's deck").await;

        assert_eq!(save(&app, &swapper, &deck_id, &substitute, None).await, 200);

        let (_, queued) = enqueue_with(&app, &swapper, json!({ "mode": "bo1", "deckId": deck_id })).await;
        assert_eq!(
            ticket(&app, queued["ticketId"].as_str().unwrap_or("")).await["deck"],
            json!(substitute)
        );

        let (_, paired) = enqueue_with(&app, &rival, json!({ "mode": "bo1", "deckId": rival_deck })).await;
        assert_eq!(paired["status"], "matched");
        let match_id = paired["matchId"].as_str().expect("a match id").to_string();
        assert_eq!(seat_deck(&app, &match_id, "early").await, json!(substitute));
    }
}

// R257 — queue modes

mod r257_queue_modes {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r257_queues_best_of_1_by_deck_id_and_refuses_the_legacy_body_with_no_mode() {
        let app = test_app().await;
        let modern = active_profile(&app, "modern", 1000.0).await;
        let legacy = active_profile(&app, "legacy", 1000.0).await;
        let [one, two, _] = decks_from();
        let deck_id = save_deck_for(&app, "modern", &one, "modern's deck").await;
        // Two saved decks for the second player: it queues the younger one by its id.
        save_deck_for(&app, "legacy", &one, "older").await;
        tokio::time::advance(Duration::from_millis(1_000)).await;
        let younger = save_deck_for(&app, "legacy", &two, "younger").await;

        // The legacy `{ deckIndex }` body, with no mode, is gone; it queues nothing (SURFACE §11.3).
        let (status, _) = enqueue_with(&app, &legacy, json!({ "deckIndex": 1 })).await;
        assert_eq!(status, 400);
        assert!(tickets_table(&app).await.is_empty());

        let (_, first) = enqueue_with(&app, &modern, json!({ "mode": "bo1", "deckId": deck_id })).await;
        assert_eq!(first["status"], "open");
        assert_eq!(first["matchId"], Value::Null);
        assert_eq!(first["seriesId"], Value::Null);
        assert_eq!(first["mode"], "bo1");

        let (_, second) = enqueue(&app, &legacy, &younger).await;
        assert_eq!(second["status"], "matched");
        assert_eq!(second["mode"], "bo1");
        assert_eq!(second["seriesId"], Value::Null);
        let match_id = second["matchId"].as_str().expect("a match id").to_string();
        assert_eq!(
            matches_table(&app).await.first().map(|row| row["id"].clone()),
            Some(json!(match_id))
        );
        // R604: a match the queue paired is ranked.
        let row = q!(app, matches_get(&match_id))
            .map(|row| to_json(&row))
            .unwrap_or(Value::Null);
        assert_eq!(row["ranked"], true);
        // R376: the mode its game record is filed under is read off the tickets it was paired from.
        assert_eq!(to_json(&q!(app, matches_mode_of(&match_id))), json!("bo1"));

        assert_eq!(seat_deck(&app, &match_id, "modern").await, json!(one));
        assert_eq!(seat_deck(&app, &match_id, "legacy").await, json!(two));
        // §9.5: a Best-of-1 pairing puts both players in the match.
        for id in ["modern", "legacy"] {
            assert_eq!(in_match_of(&app, id).await, json!(match_id));
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r257_never_pairs_tickets_of_different_modes_however_long_they_have_waited() {
        // Same rating, long waits: every window admits every ticket, so only the mode can stop a pairing.
        let app = test_app().await;
        seed_ticket(
            &app,
            SeedTicket {
                id: "t-bo1",
                profile_id: "one",
                rating: 1000,
                waited_ms: 90_000,
                mode: "bo1",
            },
        )
        .await;
        seed_ticket(
            &app,
            SeedTicket {
                id: "t-bo3",
                profile_id: "three",
                rating: 1000,
                waited_ms: 80_000,
                mode: "bo3",
            },
        )
        .await;
        seed_ticket(
            &app,
            SeedTicket {
                id: "t-rnd",
                profile_id: "random",
                rating: 1000,
                waited_ms: 70_000,
                mode: "random",
            },
        )
        .await;

        assert_eq!(try_pair(&app).await.expect("a sweep").made, 0);
        assert!(matches_table(&app).await.is_empty());
        assert!(series_table(&app).await.is_empty());
        assert_eq!(open_count(&app).await, 3);

        // Control: a second All Random ticket pairs with the first, though the Best-of-1 and Best-of-3 tickets are older.
        seed_ticket(
            &app,
            SeedTicket {
                id: "t-rnd-2",
                profile_id: "random-2",
                rating: 1000,
                waited_ms: 10_000,
                mode: "random",
            },
        )
        .await;
        assert_eq!(try_pair(&app).await.expect("a sweep").made, 1);
        assert_eq!(paired_profiles(&app).await, vec![json!(["random", "random-2"])]);
        assert_eq!(ticket(&app, "t-bo1").await["status"], "open");
        assert_eq!(ticket(&app, "t-bo3").await["status"], "open");
    }

    #[tokio::test(start_paused = true)]
    async fn r257_makes_a_best_of_3_pair_a_series_r259_the_older_ticket_is_series_p1_and_no_match_starts() {
        let _seeds = SEED_MAP.lock().await;
        let app = test_app().await;
        let older = active_profile(&app, "older", 1000.0).await;
        let younger = active_profile(&app, "younger", 1000.0).await;
        let older_trio = save_trio_for(&app, "older", None, "older's trio").await;
        let younger_trio = save_trio_for(&app, "younger", None, "younger's trio").await;

        let (_, first) = enqueue_with(
            &app,
            &older,
            json!({ "mode": "bo3", "trioId": older_trio, "seed": "series-seed" }),
        )
        .await;
        assert_eq!(first["status"], "open");
        assert_eq!(first["mode"], "bo3");
        assert_eq!(first["seriesId"], Value::Null);
        // A Best-of-3 ticket freezes the trio, with the decks' names, and no single deck.
        let first_ticket = first["ticketId"].as_str().unwrap_or("").to_string();
        let frozen = ticket(&app, &first_ticket).await;
        assert_eq!(frozen["deck"], json!([]));
        let names: Vec<Value> = frozen["trio"]["decks"]
            .as_array()
            .map(|decks| decks.iter().map(|deck| deck["name"].clone()).collect())
            .unwrap_or_default();
        assert_eq!(
            names,
            vec![json!("older one"), json!("older two"), json!("older three")]
        );

        tokio::time::advance(Duration::from_millis(1_000)).await;
        let (_, second) =
            enqueue_with(&app, &younger, json!({ "mode": "bo3", "trioId": younger_trio })).await;

        let series = series_table(&app).await;
        let series = series.first().cloned().expect("a series");
        assert_eq!(second["status"], "matched");
        assert_eq!(second["mode"], "bo3");
        assert_eq!(second["matchId"], Value::Null);
        assert_eq!(second["seriesId"], series["id"]);
        // R604: a series the queue paired is ranked.
        assert_eq!(series["ranked"], true);
        let sides: Vec<Value> = series["sides"]
            .as_array()
            .map(|sides| sides.iter().map(|side| side["profileId"].clone()).collect())
            .unwrap_or_default();
        assert_eq!(sides, vec![json!("older"), json!("younger")]);
        assert_eq!(series["sides"][0]["trio"], frozen["trio"]);
        assert_eq!(series["sides"][1]["trio"]["name"], "younger's trio");
        // R263: game 1's match id is the one the claim reserved on both tickets.
        assert_eq!(
            series["nextMatchId"],
            ticket(&app, &first_ticket).await["matchId"]
        );
        // R143: the seed the older ticket brought is the series' seed base.
        assert_eq!(series["seedBase"], "series-seed");
        assert_eq!(e2e_seed_count(), 0);
        // Nobody is in a match yet: the series opens on its pick phase.
        assert!(
            to_json(&q!(app, matches_live()))
                .as_array()
                .is_some_and(Vec::is_empty)
        );
        let profiles = to_json(&fake(&app).await.tables.profiles);
        assert!(
            profiles
                .as_array()
                .is_some_and(|rows| rows.iter().all(|row| nullable(row, "inMatchId").is_null()))
        );
    }
}

// R640001 — the window stops at MAX_FAIR_GAP

mod r640001_the_window_stops_at_max_fair_gap {
    use super::*;

    #[test]
    fn r640001_the_window_widens_as_before_and_stops_at_max_fair_gap() {
        let step = RATING_WINDOW_WIDEN_EVERY_SECONDS as f64;
        let full = RATING_WINDOW_FULL_AFTER_SECONDS as f64;
        assert_eq!(rating_window(0.0), RATING_WINDOW_START as f64);
        assert_eq!(
            rating_window(step),
            (RATING_WINDOW_START + RATING_WINDOW_WIDEN_BY) as f64
        );
        assert!(rating_window(full - step) < MAX_FAIR_GAP as f64);
        assert_eq!(rating_window(full), MAX_FAIR_GAP as f64);
        assert_eq!(rating_window(86_400.0), MAX_FAIR_GAP as f64);
    }

    #[tokio::test(start_paused = true)]
    async fn r640001_never_pairs_two_people_further_apart_than_max_fair_gap_however_long_they_wait() {
        const HOUR_MS: i64 = 3_600_000;
        let app = test_app().await;
        let t = |id, profile_id, rating, waited_ms| SeedTicket {
            id,
            profile_id,
            rating,
            waited_ms,
            mode: "bo1",
        };
        seed_ticket(&app, t("t-low", "low", 1000, HOUR_MS + 1_000)).await;
        seed_ticket(&app, t("t-far", "far", 1000 + MAX_FAIR_GAP + 1, HOUR_MS)).await;
        assert_eq!(try_pair(&app).await.expect("a sweep").made, 0);
        assert!(matches_table(&app).await.is_empty());

        // Control: one exactly MAX_FAIR_GAP away whose window has reached it pairs with the oldest.
        seed_ticket(
            &app,
            t(
                "t-edge",
                "edge",
                1000 + MAX_FAIR_GAP,
                RATING_WINDOW_FULL_AFTER_SECONDS * 1000,
            ),
        )
        .await;
        expect_qualifies(
            (1000, HOUR_MS + 1_000),
            (1000 + MAX_FAIR_GAP, RATING_WINDOW_FULL_AFTER_SECONDS * 1000),
        );
        assert_eq!(try_pair(&app).await.expect("a sweep").made, 1);
        assert_eq!(paired_profiles(&app).await, vec![json!(["low", "edge"])]);
        assert_eq!(ticket(&app, "t-far").await["status"], "open");
    }
}

// R640002 — the queue readout

mod r640002_the_queue_readout {
    use super::*;

    /// Two tickets of `mode` that waited `waits_ms`, claimed together now, as a sweep claims a pair.
    async fn claimed_pair(app: &App, ids: [&str; 2], mode: &str, waits_ms: [i64; 2]) {
        for (id, waited_ms) in ids.into_iter().zip(waits_ms) {
            seed_ticket(
                app,
                SeedTicket {
                    id,
                    profile_id: id,
                    rating: 1000,
                    waited_ms,
                    mode,
                },
            )
            .await;
        }
        let match_id = format!("match-{}", ids[0]);
        assert!(q!(app, tickets_claim_pair(ids[0], ids[1], &match_id, now_ms())));
    }

    /// A game between `players` that ended now.
    async fn ended(app: &App, match_id: &str, players: [&str; 2]) {
        q!(
            app,
            results_insert(&from(json!({
                "matchId": match_id,
                "players": players,
                "winnerProfileId": players[0],
                "reason": "concede",
                "turns": 3,
                "endedAt": now_ms(),
                "ratingBefore": [1000, 1000],
                "ratingAfter": [1000, 1000],
            })))
        );
    }

    #[test]
    fn r640002_the_expected_wait_is_the_upper_quartile_rounded_up_to_the_next_step() {
        assert_eq!(expected_wait_seconds(&[]), None);
        assert_eq!(expected_wait_seconds(&[1_000; 4]), None, "too few to say");
        assert_eq!(expected_wait_seconds(&[0; 5]), Some(10));
        assert_eq!(expected_wait_seconds(&[29_999; 5]), Some(30));
        // A wait on a step goes past it, so "under" holds.
        assert_eq!(expected_wait_seconds(&[30_000; 5]), Some(40));
        // The upper quartile, not the longest: three waits in four are under it.
        assert_eq!(
            expected_wait_seconds(&[1_000, 2_000, 3_000, 4_000, 5_000, 6_000, 7_000, 300_000]),
            Some(10)
        );
    }

    #[tokio::test(start_paused = true)]
    async fn r640002_reports_each_mode_s_expected_wait_and_the_last_hour_s_games_and_nothing_else() {
        let app = test_app().await;
        let token = active_profile(&app, "watcher", 1000.0).await;
        // Just over the window ago: an All Random pair claimed and a game ended. Counted, the pair
        // would give All Random six waits, enough for an expected wait, and the games would be four.
        claimed_pair(&app, ["t-old-1", "t-old-2"], "random", [1_000, 2_000]).await;
        ended(&app, "m-old", ["t-old-1", "t-old-2"]).await;
        tokio::time::advance(Duration::from_secs(QUEUE_READOUT_WINDOW_SECONDS as u64 + 1)).await;

        // In the window: six Best-of-1 waits claimed in pairs, and two All Random pairs.
        claimed_pair(&app, ["t-1", "t-2"], "bo1", [4_000, 8_000]).await;
        claimed_pair(&app, ["t-3", "t-4"], "bo1", [12_000, 16_000]).await;
        claimed_pair(&app, ["t-5", "t-6"], "bo1", [21_000, 26_000]).await;
        claimed_pair(&app, ["t-r1", "t-r2"], "random", [5_000, 5_000]).await;
        claimed_pair(&app, ["t-r3", "t-r4"], "random", [5_000, 5_000]).await;
        // A ticket still open and one cancelled have waited for nothing.
        for id in ["t-open", "t-gone"] {
            seed_ticket(
                &app,
                SeedTicket {
                    id,
                    profile_id: id,
                    rating: 1000,
                    waited_ms: 5_000,
                    mode: "random",
                },
            )
            .await;
        }
        q!(app, tickets_cancel("t-gone", now_ms()));
        // Three games, one of them against a ladder bot.
        fake(&app)
            .await
            .seed_profile(json!({ "id": "a-bot", "kind": "bot" }));
        ended(&app, "m-1", ["t-1", "t-2"]).await;
        ended(&app, "m-2", ["t-3", "t-4"]).await;
        ended(&app, "m-3", ["t-5", "a-bot"]).await;

        let (status, body) = request(&app, "GET", "/api/queue/population", &token, None).await;

        assert_eq!(status, 200);
        // Best of 1's upper quartile is the 21 s wait, so under 30 s; All Random's four waits are too
        // few; Conquest has none.
        assert_eq!(
            body,
            json!({
                "gamesLastHour": 3,
                "expectedWaitSeconds": { "bo1": 30, "bo3": null, "random": null },
            })
        );
    }

    #[tokio::test(start_paused = true)]
    async fn r640002_enqueue_answers_its_ticket_and_no_queue_count() {
        let app = test_app().await;
        let token = active_profile(&app, "lone", 1000.0).await;
        let [one, ..] = decks_from();
        let deck_id = save_deck_for(&app, "lone", &one, "lone's deck").await;
        let (status, body) = enqueue(&app, &token, &deck_id).await;
        assert_eq!(status, 200, "{body}");
        let mut keys: Vec<&str> = body
            .as_object()
            .expect("an object")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, vec!["matchId", "mode", "seriesId", "status", "ticketId"]);
    }
}

// R258 — All Random

mod r258_all_random {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r258_deals_both_players_a_deck_from_the_match_seed_and_the_seat_with_no_saved_deck_needed() {
        let app = test_app().await;
        let one = active_profile(&app, "rng-one", 1000.0).await;
        let two = active_profile(&app, "rng-two", 1000.0).await;
        // PREMISE: neither player has saved anything; a Best-of-1 enqueue would be refused (R165).
        for id in ["rng-one", "rng-two"] {
            assert!(
                to_json(&q!(app, decks_list(id)))
                    .as_array()
                    .is_some_and(Vec::is_empty)
            );
        }

        let (_, first) = enqueue_with(&app, &one, json!({ "mode": "random" })).await;
        assert_eq!(first["status"], "open");
        assert_eq!(first["mode"], "random");
        assert_eq!(
            ticket(&app, first["ticketId"].as_str().unwrap_or("")).await["deck"],
            json!([])
        );
        // The older ticket is p1 (R166: oldest first, ties on the ticket id). The server's ids are random,
        // so the first enqueue is made older by a millisecond of the paused clock.
        tokio::time::advance(Duration::from_millis(1)).await;

        let (_, second) = enqueue_with(&app, &two, json!({ "mode": "random" })).await;
        assert_eq!(second["status"], "matched");
        assert_eq!(second["mode"], "random");
        assert_eq!(second["seriesId"], Value::Null);

        let match_id = second["matchId"].as_str().expect("a match id").to_string();
        let row = q!(app, matches_get(&match_id))
            .map(|row| to_json(&row))
            .expect("the match row");
        // R604: a match the queue paired is ranked — All Random included.
        assert_eq!(row["ranked"], true);
        assert_eq!(to_json(&q!(app, matches_mode_of(&match_id))), json!("random"));
        let seed = row["seed"].as_str().expect("a seed").to_string();
        let p1_deck = json!(deal_random_deck(&format!("{seed}:p1-deck"), None));
        let p2_deck = json!(deal_random_deck(&format!("{seed}:p2-deck"), None));
        assert_eq!(seat_deck(&app, &match_id, "rng-one").await, p1_deck);
        assert_eq!(seat_deck(&app, &match_id, "rng-two").await, p2_deck);
        // Two seats, two different deals.
        assert_ne!(p1_deck, p2_deck);
        // Frozen into the match row, so `(seed, decks, log)` replays it.
        assert_eq!(row["decks"], json!([p1_deck, p2_deck]));
    }

    #[tokio::test(start_paused = true)]
    async fn r258_deals_from_the_seed_an_end_to_end_enqueue_supplied_r143_so_a_spec_can_pin_the_decks() {
        let _seeds = SEED_MAP.lock().await;
        let app = test_app().await;
        let one = active_profile(&app, "pin-one", 1000.0).await;
        let two = active_profile(&app, "pin-two", 1000.0).await;

        enqueue_with(&app, &one, json!({ "mode": "random", "seed": "spec-seed" })).await;
        // The older ticket is p1 (R166: oldest first, ties on the ticket id). The server's ids are random,
        // so the first enqueue is made older by a millisecond of the paused clock.
        tokio::time::advance(Duration::from_millis(1)).await;
        let (_, paired) = enqueue_with(&app, &two, json!({ "mode": "random" })).await;
        let match_id = paired["matchId"].as_str().expect("a match id").to_string();

        let row = q!(app, matches_get(&match_id))
            .map(|row| to_json(&row))
            .expect("the match row");
        assert_eq!(row["seed"], "spec-seed");
        assert_eq!(
            seat_deck(&app, &match_id, "pin-one").await,
            json!(deal_random_deck("spec-seed:p1-deck", None))
        );
        assert_eq!(
            seat_deck(&app, &match_id, "pin-two").await,
            json!(deal_random_deck("spec-seed:p2-deck", None))
        );
    }
}

// R1372 — All Random's "More cards from the newest set", per seat

mod r1372_more_cards_from_the_newest_set {
    use super::*;

    /// How many of a dealt deck's cards are of the newest set that ships (R1371).
    fn newest_in(deck: &Value) -> usize {
        let newest = newest_shipped_set();
        deck.as_array()
            .map(|ids| {
                ids.iter()
                    .filter(|id| {
                        id.as_str()
                            .and_then(|id| jackioh_cards::CATALOG.get(id))
                            .is_some_and(|def| def.set == newest)
                    })
                    .count()
            })
            .unwrap_or(0)
    }

    #[tokio::test(start_paused = true)]
    async fn r1372_leans_the_deck_of_exactly_the_seat_whose_ticket_asked_and_pairs_it_with_one_that_did_not()
    {
        let app = test_app().await;
        let one = active_profile(&app, "lean-one", 1000.0).await;
        let two = active_profile(&app, "lean-two", 1000.0).await;

        let (status, first) = enqueue_with(&app, &one, json!({ "mode": "random", "leanNewest": true })).await;
        assert_eq!(status, 200, "{first}");
        // The ask waits in the ticket, as a Best-of-1 deck would (migration 0036's column, R924000:
        // an older client's `leanNewest: true` is the pick "newest").
        assert_eq!(
            ticket(&app, first["ticketId"].as_str().unwrap_or("")).await["leanSet"],
            "newest"
        );
        // The older ticket is p1 (R166), as in R258's test above.
        tokio::time::advance(Duration::from_millis(1)).await;
        let (_, second) = enqueue_with(&app, &two, json!({ "mode": "random" })).await;
        assert_eq!(
            second["status"], "matched",
            "a ticket that leans pairs with one that does not"
        );

        let match_id = second["matchId"].as_str().expect("a match id").to_string();
        let row = q!(app, matches_get(&match_id))
            .map(|row| to_json(&row))
            .expect("the match row");
        let seed = row["seed"].as_str().expect("a seed").to_string();
        let leaned = json!(deal_random_deck(
            &format!("{seed}:p1-deck"),
            Some(newest_shipped_set())
        ));
        let plain = json!(deal_random_deck(&format!("{seed}:p2-deck"), None));
        assert_eq!(seat_deck(&app, &match_id, "lean-one").await, leaned);
        assert_eq!(seat_deck(&app, &match_id, "lean-two").await, plain);
        // Frozen into the match row like any dealt deck, so `(seed, decks, log)` replays it (R258).
        assert_eq!(row["decks"], json!([leaned, plain]));
        // R1370: at least half of the asking seat's deck is of the newest set.
        let floor =
            (f64::from(jackioh_engine::config::DECK_SIZE) * jackioh_ai::AI_DECK.lean_min_share).ceil();
        assert!(newest_in(&leaned) as f64 >= floor, "{leaned}");
        assert_ne!(leaned, json!(deal_random_deck(&format!("{seed}:p1-deck"), None)));
    }

    #[tokio::test(start_paused = true)]
    async fn r1372_an_enqueue_without_the_field_does_not_lean_and_one_that_is_not_a_boolean_is_refused() {
        let app = test_app().await;
        let one = active_profile(&app, "lean-absent", 1000.0).await;
        let (status, refused) = enqueue_with(&app, &one, json!({ "mode": "random", "leanNewest": 1 })).await;
        assert_eq!(status, 400, "{refused}");
        assert!(tickets_table(&app).await.is_empty());

        let (status, queued) = enqueue_with(&app, &one, json!({ "mode": "random" })).await;
        assert_eq!(status, 200, "{queued}");
        assert_eq!(
            ticket(&app, queued["ticketId"].as_str().unwrap_or("")).await["leanSet"],
            Value::Null
        );
    }

    /// How many of a dealt deck's cards are of `set`.
    fn of_set(deck: &Value, set: jackioh_engine::SetName) -> usize {
        deck.as_array()
            .map(|ids| {
                ids.iter()
                    .filter(|id| {
                        id.as_str()
                            .and_then(|id| jackioh_cards::CATALOG.get(id))
                            .is_some_and(|def| def.set == set)
                    })
                    .count()
            })
            .unwrap_or(0)
    }

    #[tokio::test(start_paused = true)]
    async fn r924000_a_ticket_leans_on_the_set_it_picked_and_its_partner_on_its_own() {
        use jackioh_engine::SetName;
        let app = test_app().await;
        let one = active_profile(&app, "pick-one", 1000.0).await;
        let two = active_profile(&app, "pick-two", 1000.0).await;

        let (status, first) =
            enqueue_with(&app, &one, json!({ "mode": "random", "leanSet": "Classic" })).await;
        assert_eq!(status, 200, "{first}");
        assert_eq!(
            ticket(&app, first["ticketId"].as_str().unwrap_or("")).await["leanSet"],
            "Classic"
        );
        tokio::time::advance(Duration::from_millis(1)).await;
        // A ticket that leans on another set pairs with it all the same.
        let (_, second) = enqueue_with(&app, &two, json!({ "mode": "random", "leanSet": "newest" })).await;
        assert_eq!(second["status"], "matched", "{second}");

        let match_id = second["matchId"].as_str().expect("a match id").to_string();
        let row = q!(app, matches_get(&match_id))
            .map(|row| to_json(&row))
            .expect("the match row");
        let seed = row["seed"].as_str().expect("a seed").to_string();
        let classic = json!(deal_random_deck(
            &format!("{seed}:p1-deck"),
            Some(SetName::Classic)
        ));
        let newest = json!(deal_random_deck(
            &format!("{seed}:p2-deck"),
            Some(newest_shipped_set())
        ));
        assert_eq!(seat_deck(&app, &match_id, "pick-one").await, classic);
        assert_eq!(seat_deck(&app, &match_id, "pick-two").await, newest);
        // R1370's floor holds for any set the player picks, not only the newest.
        let floor =
            (f64::from(jackioh_engine::config::DECK_SIZE) * jackioh_ai::AI_DECK.lean_min_share).ceil();
        assert!(of_set(&classic, SetName::Classic) as f64 >= floor, "{classic}");
        assert!(of_set(&newest, newest_shipped_set()) as f64 >= floor, "{newest}");
    }

    #[tokio::test(start_paused = true)]
    async fn r924000_a_lean_on_a_set_that_does_not_ship_or_on_nothing_known_is_refused() {
        let app = test_app().await;
        let one = active_profile(&app, "pick-bad", 1000.0).await;
        for lean in [
            json!("Boss"),
            json!("Classic+ "),
            json!("classic"),
            json!(true),
            json!(3),
        ] {
            let (status, refused) =
                enqueue_with(&app, &one, json!({ "mode": "random", "leanSet": lean })).await;
            assert_eq!(status, 400, "{lean}: {refused}");
        }
        assert!(tickets_table(&app).await.is_empty());
        // `leanSet` is read before an older client's `leanNewest`, so a body with both leans on the pick.
        let (status, queued) = enqueue_with(
            &app,
            &one,
            json!({ "mode": "random", "leanSet": "Core", "leanNewest": true }),
        )
        .await;
        assert_eq!(status, 200, "{queued}");
        assert_eq!(
            ticket(&app, queued["ticketId"].as_str().unwrap_or("")).await["leanSet"],
            "Core"
        );
    }
}

// A start that fails

mod a_paired_match_whose_start_fails {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn frees_both_players_instead_of_locking_them_on_the_claims_skeleton() {
        let app = test_app().await;
        let one = active_profile(&app, "starter-a", 1000.0).await;
        let two = active_profile(&app, "starter-b", 1000.0).await;
        let [first, second, _] = decks_from();
        let deck_a = save_deck_for(&app, "starter-a", &first, "starter-a's deck").await;
        let deck_b = save_deck_for(&app, "starter-b", &second, "starter-b's deck").await;

        enqueue_with(&app, &one, json!({ "mode": "bo1", "deckId": deck_a })).await;
        // The registry writes the match row only once the game has begun; refusing that write is a
        // start that fails after the claim.
        let calls = record_calls(&mut *fake(&app).await, |_, method| method == "matches.create");

        // The claim won, the in-match transaction committed, and then the start failed — the pair
        // is lost either way; what must NOT be lost is the players.
        assert_eq!(
            enqueue_with(&app, &two, json!({ "mode": "bo1", "deckId": deck_b }))
                .await
                .0,
            500
        );
        assert!(
            calls
                .lock()
                .expect("calls")
                .iter()
                .any(|method| method == "matches.discardOpen")
        );
        for id in ["starter-a", "starter-b"] {
            assert_eq!(in_match_of(&app, id).await, Value::Null);
        }

        // Free means free: the claimed ticket is never open, so a fresh enqueue is not blocked.
        fake(&app).await.on_call = None;
        let (_, again) = enqueue_with(&app, &one, json!({ "mode": "bo1", "deckId": deck_a })).await;
        assert_eq!(again["status"], "open");
    }

    #[tokio::test(start_paused = true)]
    async fn r1450_a_start_whose_cleanup_fails_too_is_released_by_the_reaper_once_the_grace_has_run() {
        let app = test_app().await;
        let one = active_profile(&app, "stranded-a", 1000.0).await;
        let two = active_profile(&app, "stranded-b", 1000.0).await;
        let [first, second, _] = decks_from();
        let deck_a = save_deck_for(&app, "stranded-a", &first, "stranded-a's deck").await;
        let deck_b = save_deck_for(&app, "stranded-b", &second, "stranded-b's deck").await;

        enqueue_with(&app, &one, json!({ "mode": "bo1", "deckId": deck_a })).await;
        // The start fails at `matches.create`, and the start-failure cleanup's conditional
        // clears fail too (R890000 renamed them from `profiles.setInMatch` to
        // `profiles.clearInMatch`; both names are refused so the cleanup fails either way).
        record_calls(&mut *fake(&app).await, |seen, method| {
            method == "matches.create"
                || ((method == "profiles.setInMatch" || method == "profiles.clearInMatch")
                    && seen.iter().any(|previous| previous == "matches.create"))
        });

        assert_eq!(
            enqueue_with(&app, &two, json!({ "mode": "bo1", "deckId": deck_b }))
                .await
                .0,
            500
        );
        fake(&app).await.on_call = None;

        let stranded_a = in_match_of(&app, "stranded-a").await;
        let stranded_b = in_match_of(&app, "stranded-b").await;
        assert!(stranded_a.is_string());
        assert_eq!(stranded_a, stranded_b);

        assert_eq!(
            enqueue_with(&app, &one, json!({ "mode": "bo1", "deckId": deck_a }))
                .await
                .0,
            409
        );

        tokio::time::advance(Duration::from_secs((OPEN_MATCH_REAP_GRACE_SECONDS - 10) as u64)).await;
        assert_eq!(reap_open_matches(&app).await.unwrap(), 0);
        assert_eq!(in_match_of(&app, "stranded-a").await, stranded_a);
        assert_eq!(in_match_of(&app, "stranded-b").await, stranded_b);

        tokio::time::advance(Duration::from_secs(20)).await;
        assert_eq!(reap_open_matches(&app).await.unwrap(), 1);
        assert_eq!(in_match_of(&app, "stranded-a").await, Value::Null);
        assert_eq!(in_match_of(&app, "stranded-b").await, Value::Null);

        let (_, again) = enqueue_with(&app, &one, json!({ "mode": "bo1", "deckId": deck_a })).await;
        assert_eq!(again["status"], "open");
    }

    #[tokio::test(start_paused = true)]
    async fn frees_a_conquest_pair_the_same_way_rolling_the_half_made_series_row_back_with_it() {
        let app = test_app().await;
        let one = active_profile(&app, "series-a", 1000.0).await;
        let two = active_profile(&app, "series-b", 1000.0).await;
        let trio_a = save_trio_for(&app, "series-a", None, "series-a's trio").await;
        let trio_b = save_trio_for(&app, "series-b", None, "series-b's trio").await;

        enqueue_with(&app, &one, json!({ "mode": "bo3", "trioId": trio_a })).await;
        // The claim won, `series.create` landed inside the transaction, then the start failed. Without the
        // rollback the 'picking' row would hold both players out of the queue (R264) until the pick deadline
        // ran the series out (R333). The fault is the one store call right after the series row, and only it.
        let calls = record_calls(&mut *fake(&app).await, |seen, _method| {
            seen.last().is_some_and(|previous| previous == "series.create")
        });

        assert_eq!(
            enqueue_with(&app, &two, json!({ "mode": "bo3", "trioId": trio_b }))
                .await
                .0,
            500
        );
        let seen = calls.lock().expect("calls").clone();
        assert!(
            seen.iter().any(|method| method == "series.create"),
            "calls: {seen:?}"
        );
        assert!(
            seen.iter().any(|method| method == "matches.discardOpen"),
            "calls: {seen:?}"
        );
        // The transaction's half: nothing of the series remains, and no in-match flag was ever set.
        fake(&app).await.on_call = None;
        assert!(series_table(&app).await.is_empty());
        for id in ["series-a", "series-b"] {
            assert_eq!(in_match_of(&app, id).await, Value::Null);
        }

        // Free means free: the same player can queue straight back up.
        let (_, again) = enqueue_with(&app, &one, json!({ "mode": "bo3", "trioId": trio_a })).await;
        assert_eq!(again["status"], "open");
    }
}

// One pair's failed start no longer ends the sweep or fails another player's enqueue

mod a_failed_start_does_not_end_the_sweep {
    use super::*;

    /// Unit conversion, not configuration: the fallback time speaks seconds.
    const MS_PER_SECOND: i64 = 1000;

    /// Past the longest fallback time any ticket draws, so a lone ticket is always due its bot.
    fn past_any_fallback() -> i64 {
        (BOT_FALLBACK_MAX_SECONDS + 1) * MS_PER_SECOND
    }

    #[tokio::test(start_paused = true)]
    async fn older_bo1_pair_fails_younger_random_pair_still_starts() {
        let app = test_app().await;
        let t = |id, profile_id, rating, waited_ms, mode| SeedTicket {
            id,
            profile_id,
            rating,
            waited_ms,
            mode,
        };
        seed_ticket(&app, t("t-bo1-a", "bo1-a", 1000, 3_000, "bo1")).await;
        seed_ticket(&app, t("t-bo1-b", "bo1-b", 1000, 2_000, "bo1")).await;
        seed_ticket(&app, t("t-rnd-c", "rnd-c", 1000, 1_000, "random")).await;
        seed_ticket(&app, t("t-rnd-d", "rnd-d", 1000, 0, "random")).await;

        // The registry writes the match row only once the game has begun; refusing that write
        // once is a start that fails for the first pair only.
        let _calls = record_calls(&mut *fake(&app).await, |seen, method| {
            method == "matches.create" && !seen.iter().any(|previous| previous == "matches.create")
        });

        let report = try_pair(&app).await.expect("a sweep carries on");
        assert_eq!(report.made, 1);
        assert_eq!(report.failed.len(), 1);
        let mut failed = report.failed[0].ticket_ids.clone();
        failed.sort();
        assert_eq!(failed, vec!["t-bo1-a".to_string(), "t-bo1-b".to_string()]);

        // The younger All Random pair started in the same sweep.
        assert_eq!(paired_profiles(&app).await, vec![json!(["rnd-c", "rnd-d"])]);
        for id in ["t-rnd-c", "t-rnd-d"] {
            assert_eq!(ticket(&app, id).await["status"], "matched");
        }
        // The failed pair's players were freed as they are today.
        for id in ["bo1-a", "bo1-b"] {
            assert_eq!(in_match_of(&app, id).await, Value::Null);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn third_player_enqueues_unrelated_to_a_failed_pair_gets_200() {
        let app = test_app().await;
        seed_ticket(
            &app,
            SeedTicket {
                id: "t-a",
                profile_id: "pair-a",
                rating: 1000,
                waited_ms: 1_000,
                mode: "bo1",
            },
        )
        .await;
        seed_ticket(
            &app,
            SeedTicket {
                id: "t-b",
                profile_id: "pair-b",
                rating: 1000,
                waited_ms: 900,
                mode: "bo1",
            },
        )
        .await;
        // Every `matches.create` fails: the seeded pair's start fails during the third player's
        // own enqueue sweep.
        let _calls = record_calls(&mut *fake(&app).await, |_, method| method == "matches.create");

        // Rated far from the failing pair, so no window of this sweep admits it as an opponent.
        let third = active_profile(&app, "third", 1600.0).await;
        let [_, _, lone] = decks_from();
        let deck_id = save_deck_for(&app, "third", &lone, "third's deck").await;
        let (status, body) = enqueue_with(&app, &third, json!({ "mode": "bo1", "deckId": deck_id })).await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["status"], "open");
        assert!(body["ticketId"].is_string());

        // The unrelated pair still failed as before, and both its players were freed.
        for id in ["pair-a", "pair-b"] {
            assert_eq!(in_match_of(&app, id).await, Value::Null);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn failed_person_pair_does_not_skip_the_bot_fallback() {
        let app = test_app_with(TestAppOptions {
            env: vec![("LADDER_BOTS".to_string(), "on".to_string())],
            skip_fixtures: true,
            ..TestAppOptions::default()
        })
        .await;
        // One ladder bot, rated with the people, on all day, dealt for this catalog.
        let bot: NewBot = from(json!({
            "profileId": "bot-fallback",
            "strength": 2,
            "speed": 1.0,
            "persona": "balanced",
            "activeFromHour": 0,
            "activeHours": 24,
            "decks": [],
            "decksCatalogVersion": null,
            "createdAt": now_ms() - 30 * 86_400_000_i64,
            "seedRating": 1000.0,
        }));
        q!(app, bots_create(&bot));
        let [lone, ..] = decks_from();
        q!(
            app,
            bots_set_decks(
                "bot-fallback",
                &[BotDeck {
                    cards: lone,
                    portrait: pick_portrait_from_seed("bot-deck").to_string(),
                }],
                &app.catalog.version
            )
        );

        // Two people who pair (and fail) first, plus one lone ticket past its fallback time that
        // no person can pair with once the pair is taken.
        let past = past_any_fallback();
        let t = |id, profile_id, waited_ms| SeedTicket {
            id,
            profile_id,
            rating: 1000,
            waited_ms,
            mode: "bo1",
        };
        seed_ticket(&app, t("t-p1", "pair-one", past + 60_000)).await;
        seed_ticket(&app, t("t-p2", "pair-two", past + 50_000)).await;
        seed_ticket(&app, t("t-lone", "lone", past)).await;

        let _calls = record_calls(&mut *fake(&app).await, |seen, method| {
            method == "matches.create" && !seen.iter().any(|previous| previous == "matches.create")
        });

        let report = try_pair(&app).await.expect("a sweep carries on");
        assert_eq!(report.made, 1);
        assert_eq!(report.failed.len(), 1);

        // The bot game started despite the person pair's failure.
        let rows = paired_profiles(&app).await;
        assert_eq!(rows.len(), 1);
        let players = rows[0].as_array().cloned().unwrap_or_default();
        assert!(players.contains(&json!("lone")), "{players:?}");
        assert!(players.contains(&json!("bot-fallback")), "{players:?}");
        for id in ["pair-one", "pair-two"] {
            assert_eq!(in_match_of(&app, id).await, Value::Null);
        }
    }
}

// R642 — the portrait on the ticket and the seat

/// R642, the queue half: a Best-of-1 ticket freezes its deck's portrait with the deck (§9.4, §9.8), an
/// All Random match deals each seat's portrait from the match seed like the deck (R258), and the pair
/// lands on the match row seat-ordered, which the actor's `portraits` frame reads back.
mod r642_the_portrait_the_ticket_freezes_and_the_seed_deals {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r642_freezes_the_decks_portrait_into_the_ticket_and_the_edit_after_cannot_reach_it() {
        let app = test_app().await;
        let swapper = active_profile(&app, "swapper", 1000.0).await;
        let rival = active_profile(&app, "rival", 1000.0).await;
        let deck_id = uuid();
        let rival_deck_id = uuid();
        let [mine, theirs, _] = decks_from();
        assert_eq!(save(&app, &swapper, &deck_id, &mine, Some("gary")).await, 200);
        assert_eq!(
            save(&app, &rival, &rival_deck_id, &theirs, Some("shredder")).await,
            200
        );

        // Queue with the deck: its portrait freezes into the ticket here.
        let (_, queued) = enqueue_with(&app, &swapper, json!({ "mode": "bo1", "deckId": deck_id })).await;
        let ticket_id = queued["ticketId"].as_str().unwrap_or("").to_string();
        assert_eq!(queued["status"], "open");
        assert_eq!(ticket(&app, &ticket_id).await["portrait"], "gary");

        // The re-save, while the ticket is still open, lands; the ticket does not move.
        assert_eq!(save(&app, &swapper, &deck_id, &mine, Some("timmy")).await, 200);
        // PREMISE: the new portrait is what the deck holds now, so there is something to leak.
        let deck = q!(app, decks_get(&deck_id))
            .map(|deck| to_json(&deck))
            .unwrap_or(Value::Null);
        assert_eq!(deck["portrait"], "timmy");
        assert_eq!(ticket(&app, &ticket_id).await["portrait"], "gary");
        // The older ticket is p1 (R166: oldest first, ties on the ticket id). The server's ids are random,
        // so the first enqueue is made older by a millisecond of the paused clock.
        tokio::time::advance(Duration::from_millis(1)).await;

        // The pairing: each seat carries what its ticket froze.
        let (_, paired) = enqueue_with(&app, &rival, json!({ "mode": "bo1", "deckId": rival_deck_id })).await;
        assert_eq!(paired["status"], "matched");

        let row = matches_table(&app).await.last().cloned().expect("the match row");
        assert_eq!(row["players"], json!(["swapper", "rival"]));
        assert_eq!(row["portraits"], json!(["gary", "shredder"]));
    }

    #[tokio::test(start_paused = true)]
    async fn r642_reads_a_best_of_1_ticket_with_no_portrait_as_vanilla_on_the_seat_and_the_row() {
        let app = test_app().await;
        let one = active_profile(&app, "plain-one", 1000.0).await;
        let two = active_profile(&app, "plain-two", 1000.0).await;
        // `save_deck_for` writes `portrait: null`: R641's default.
        let [first, second, _] = decks_from();
        let a = save_deck_for(&app, "plain-one", &first, "plain-one's deck").await;
        let b = save_deck_for(&app, "plain-two", &second, "plain-two's deck").await;

        enqueue_with(&app, &one, json!({ "mode": "bo1", "deckId": a })).await;
        let (_, paired) = enqueue_with(&app, &two, json!({ "mode": "bo1", "deckId": b })).await;
        assert_eq!(paired["status"], "matched");

        let portraits: Vec<Value> = tickets_table(&app)
            .await
            .iter()
            .map(|row| nullable(row, "portrait"))
            .collect();
        assert_eq!(portraits, vec![Value::Null, Value::Null]);
        let row = matches_table(&app).await.last().cloned().expect("the match row");
        assert_eq!(row["portraits"], json!([DEFAULT_PORTRAIT, DEFAULT_PORTRAIT]));
    }

    #[tokio::test(start_paused = true)]
    async fn r642_deals_an_all_random_matchs_portraits_from_the_seed_seat_by_seat_like_the_decks() {
        let _seeds = SEED_MAP.lock().await;
        let app = test_app().await;
        let one = active_profile(&app, "rng-p1", 1000.0).await;
        let two = active_profile(&app, "rng-p2", 1000.0).await;

        enqueue_with(&app, &one, json!({ "mode": "random", "seed": "portrait-seed" })).await;
        // The older ticket is p1 (R166: oldest first, ties on the ticket id). The server's ids are random,
        // so the first enqueue is made older by a millisecond of the paused clock.
        tokio::time::advance(Duration::from_millis(1)).await;
        let (_, paired) = enqueue_with(&app, &two, json!({ "mode": "random" })).await;
        assert_eq!(paired["status"], "matched");

        // An All Random ticket freezes no deck and no portrait: both are the match's to deal.
        let portraits: Vec<Value> = tickets_table(&app)
            .await
            .iter()
            .map(|row| nullable(row, "portrait"))
            .collect();
        assert_eq!(portraits, vec![Value::Null, Value::Null]);

        let row = matches_table(&app).await.last().cloned().expect("the match row");
        // PREMISE: the seed really is the one the enqueue supplied (R143), seat order as paired.
        assert_eq!(row["seed"], "portrait-seed");
        assert_eq!(row["players"], json!(["rng-p1", "rng-p2"]));
        // Each seat's pick is `pick_portrait_from_seed` on its seat-keyed suffix, as the decks are dealt.
        let expected = json!([
            pick_portrait_from_seed("portrait-seed:portrait:p1"),
            pick_portrait_from_seed("portrait-seed:portrait:p2"),
        ]);
        assert_eq!(row["portraits"], expected);
    }
}

// R264 — a profile in a series

mod r264_a_series_that_is_not_over_holds_its_players_out_of_the_queue {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r264_refuses_to_queue_a_profile_whose_series_is_not_over_naming_the_series() {
        let app = test_app().await;
        let token = active_profile(&app, "mid-series", 1000.0).await;
        active_profile(&app, "opponent", 1000.0).await;
        let [deck, ..] = decks_from();
        let deck_id = save_deck_for(&app, "mid-series", &deck, "mid-series's deck").await;
        q!(
            app,
            series_create(&from(series_with(&app, "mid-series", "opponent", "picking")))
        );

        let (status, body) = enqueue(&app, &token, &deck_id).await;

        assert_eq!(status, 409);
        assert_eq!(body["error"]["code"], "already_in_match");
        assert_eq!(body["error"]["message"], "Finish your Conquest series first.");
        assert_eq!(
            body["error"]["details"],
            json!({ "seriesId": "series-of-mid-series" })
        );
        assert!(tickets_table(&app).await.is_empty());

        // The control: once the series is over the same request queues.
        let mut over = series_with(&app, "mid-series", "opponent", "over");
        over["version"] = json!(2);
        assert!(q!(app, series_update(&from(over))));
        assert_eq!(enqueue(&app, &token, &deck_id).await.0, 200);
    }

    #[tokio::test(start_paused = true)]
    async fn r264_never_pairs_a_ticket_whose_owner_entered_a_series_after_queueing() {
        let app = test_app().await;
        seed_ticket(
            &app,
            SeedTicket {
                id: "t-left",
                profile_id: "left",
                rating: 1000,
                waited_ms: 5_000,
                mode: "bo1",
            },
        )
        .await;
        seed_ticket(
            &app,
            SeedTicket {
                id: "t-right",
                profile_id: "right",
                rating: 1000,
                waited_ms: 4_000,
                mode: "bo1",
            },
        )
        .await;
        active_profile(&app, "room-rival", 1000.0).await;
        // `left` joined a Best-of-3 room while the ticket sat open.
        q!(
            app,
            series_create(&from(series_with(&app, "left", "room-rival", "picking")))
        );

        assert_eq!(try_pair(&app).await.expect("a sweep").made, 0);
        assert_eq!(ticket(&app, "t-right").await["status"], "open");
    }
}

// R253 — the real validator at enqueue

/// The production wiring: the real §8 catalog, validator, R111's launch grant and `POST /api/queue`.
/// It does not re-test L1–L6 (`crates/engine/src/validator.rs` does), only that a deck or trio the shared
/// module refuses is refused at enqueue with its own issues and no ticket. Expectations are read off the
/// refusal itself, not typed out.
mod r253_what_may_be_queued_through_the_real_validator {
    use super::*;

    struct Real {
        app: Arc<App>,
        token: String,
        pool: Vec<String>,
        size: usize,
    }

    /// The production wiring for one active profile that owns every card; the deck size is the engine config's (BUILD §2).
    async fn real_wiring() -> Real {
        let app = test_app().await;
        let token = active_profile(&app, "real", 1000.0).await;
        Real {
            app,
            token,
            pool: pool(),
            size: deck_size(),
        }
    }

    /// PREMISE: the shared module really refuses this, and first for the rule named.
    async fn expect_refused(w: &Real, body: Value, rule: &str) -> Value {
        let (status, refused) = enqueue_with(&w.app, &w.token, body).await;
        assert_eq!(status, 422, "{refused}");
        assert_eq!(refused["error"]["code"], "loadout_invalid");
        let details = refused["error"]["details"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        assert_eq!(
            details.first().map(|issue| issue["rule"].clone()),
            Some(json!(rule)),
            "{refused}"
        );
        assert_eq!(
            Some(&refused["error"]["message"]),
            details.first().map(|issue| &issue["message"])
        );
        assert!(tickets_table(&w.app).await.is_empty());
        refused
    }

    #[tokio::test(start_paused = true)]
    async fn r253_queues_a_legal_best_of_1_deck_of_real_cards_the_premise() {
        let w = real_wiring().await;
        let deck = w.pool[..w.size].to_vec();
        let deck_id = save_deck_for(&w.app, "real", &deck, "Midrange").await;
        let (status, _) = enqueue_with(&w.app, &w.token, json!({ "mode": "bo1", "deckId": deck_id })).await;
        assert_eq!(status, 200);
        assert_eq!(
            tickets_table(&w.app).await.first().map(|row| row["deck"].clone()),
            Some(json!(deck))
        );
    }

    #[tokio::test(start_paused = true)]
    async fn r253_refuses_a_best_of_1_deck_one_card_short_l2_naming_the_deck_as_the_player_named_it() {
        let w = real_wiring().await;
        let short = w.pool[..w.size - 1].to_vec();
        // A draft this short is saved without complaint (R250); the queue is where it is judged.
        let deck_id = save_deck_for(&w.app, "real", &short, "Midrange").await;

        let refused = expect_refused(&w, json!({ "mode": "bo1", "deckId": deck_id }), "L2").await;
        assert!(
            refused["error"]["message"]
                .as_str()
                .unwrap_or("")
                .contains("Midrange")
        );
    }

    #[tokio::test(start_paused = true)]
    async fn r253_refuses_a_best_of_1_deck_holding_a_token_l3_and_one_the_collection_no_longer_covers_l5() {
        let w = real_wiring().await;
        let token = jackioh_cards::CATALOG
            .values()
            .find(|def| def.token)
            .map(|def| def.id.clone())
            .unwrap_or_default();
        assert!(!token.is_empty());
        let mut with_token = vec![token];
        with_token.extend(w.pool[1..w.size].iter().cloned());
        let token_deck = save_deck_for(&w.app, "real", &with_token, "Sheepish").await;
        expect_refused(&w, json!({ "mode": "bo1", "deckId": token_deck }), "L3").await;

        // L5: the collection is server-owned and can move after a save (§9.8).
        let legal = save_deck_for(&w.app, "real", &w.pool[..w.size], "Legal").await;
        fake(&w.app).await.tables.collection.clear();
        let refused = expect_refused(&w, json!({ "mode": "bo1", "deckId": legal }), "L5").await;
        assert!(
            refused["error"]["message"]
                .as_str()
                .unwrap_or("")
                .contains("Legal")
        );
    }

    #[tokio::test(start_paused = true)]
    async fn r253_refuses_a_best_of_3_trio_whose_decks_share_a_card_l4_naming_both_decks() {
        let w = real_wiring().await;
        let size = w.size;
        let one = w.pool[..size].to_vec();
        let shared = one[0].clone();
        let mut two = vec![shared.clone()];
        two.extend(w.pool[size + 1..size * 2].iter().cloned());
        let three = w.pool[size * 2..size * 3].to_vec();
        let ids = [
            Some(save_deck_for(&w.app, "real", &one, "Aggro").await),
            Some(save_deck_for(&w.app, "real", &two, "Control").await),
            Some(save_deck_for(&w.app, "real", &three, "Tempo").await),
        ];
        // A loose trio is saved (R252) and judged here.
        let trio_id = save_trio_for(&w.app, "real", Some(ids), "Main").await;

        let refused = expect_refused(&w, json!({ "mode": "bo3", "trioId": trio_id }), "L4").await;
        let message = refused["error"]["message"].as_str().unwrap_or("").to_string();
        assert!(message.contains("Aggro"), "{message}");
        assert!(message.contains("Control"), "{message}");
        assert_eq!(refused["error"]["details"][0]["cardId"], json!(shared));
    }

    #[tokio::test(start_paused = true)]
    async fn r915005_refuses_a_best_of_1_deck_and_a_trio_holding_a_custom_card_l7_before_any_other_rule() {
        let w = real_wiring().await;
        let size = w.size;
        let custom = "custom-0123456789abcdef0123456789abcdef".to_string();
        // Legal by every other rule: the custom card is one card of a full deck of owned cards.
        let mut homebrew = vec![custom.clone()];
        homebrew.extend(w.pool[1..size].iter().cloned());
        let deck_id = save_deck_for(&w.app, "real", &homebrew, "Homebrew").await;
        let refused = expect_refused(&w, json!({ "mode": "bo1", "deckId": deck_id }), "L7").await;
        assert_eq!(refused["error"]["details"].as_array().map(Vec::len), Some(1));
        assert_eq!(refused["error"]["details"][0]["cardId"], json!(custom));
        let message = refused["error"]["message"].as_str().unwrap_or("").to_string();
        assert!(
            message.contains("Homebrew") && message.contains(&custom),
            "{message}"
        );

        // A deck that breaks L2 too still reads L7 first.
        let short = save_deck_for(&w.app, "real", std::slice::from_ref(&custom), "Short").await;
        let refused = expect_refused(&w, json!({ "mode": "bo1", "deckId": short }), "L7").await;
        let rules: Vec<Value> = refused["error"]["details"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|issue| issue["rule"].clone())
            .collect();
        assert_eq!(rules, vec![json!("L7"), json!("L2")]);

        // A trio with the custom card in its second deck.
        let ids = [
            Some(save_deck_for(&w.app, "real", &w.pool[size..size * 2], "Aggro").await),
            Some(deck_id.clone()),
            Some(save_deck_for(&w.app, "real", &w.pool[size * 2..size * 3], "Tempo").await),
        ];
        let trio_id = save_trio_for(&w.app, "real", Some(ids), "Main").await;
        let refused = expect_refused(&w, json!({ "mode": "bo3", "trioId": trio_id }), "L7").await;
        assert_eq!(refused["error"]["details"][0]["deck"], json!(2));
    }

    #[tokio::test(start_paused = true)]
    async fn r253_refuses_a_best_of_3_trio_with_an_empty_slot_as_l1() {
        let w = real_wiring().await;
        let size = w.size;
        let one = w.pool[..size].to_vec();
        let two = w.pool[size..size * 2].to_vec();
        let ids = [
            Some(save_deck_for(&w.app, "real", &one, "Aggro").await),
            None,
            Some(save_deck_for(&w.app, "real", &two, "Control").await),
        ];
        let trio_id = save_trio_for(&w.app, "real", Some(ids), "Gappy").await;

        expect_refused(&w, json!({ "mode": "bo3", "trioId": trio_id }), "L1").await;
    }

    #[tokio::test(start_paused = true)]
    async fn r253_queues_a_legal_trio_of_real_cards_freezing_its_three_decks_with_their_names() {
        let w = real_wiring().await;
        let size = w.size;
        let decks: Vec<Vec<String>> = (0..3)
            .map(|n| w.pool[size * n..size * (n + 1)].to_vec())
            .collect();
        let ids = [
            Some(save_deck_for(&w.app, "real", &decks[0], "Aggro").await),
            Some(save_deck_for(&w.app, "real", &decks[1], "Control").await),
            Some(save_deck_for(&w.app, "real", &decks[2], "Tempo").await),
        ];
        let trio_id = save_trio_for(&w.app, "real", Some(ids), "Main").await;

        let (status, _) = enqueue_with(&w.app, &w.token, json!({ "mode": "bo3", "trioId": trio_id })).await;

        assert_eq!(status, 200);
        let trio = tickets_table(&w.app)
            .await
            .first()
            .map(|row| row["trio"].clone())
            .unwrap_or(Value::Null);
        assert_eq!(trio["name"], "Main");
        for (n, name) in ["Aggro", "Control", "Tempo"].iter().enumerate() {
            let deck = &trio["decks"][n];
            assert_eq!(deck["name"], *name);
            assert_eq!(deck["cards"], json!(decks[n]));
            assert_eq!(nullable(deck, "portrait"), Value::Null);
        }
    }
}

// R890000: the seat compare-and-set between the sweep's read and the claim

mod r890000_a_room_join_between_the_sweep_read_and_the_claim {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r890000_room_join_between_sweep_read_and_claim_leaves_one_match() {
        let app = test_app().await;
        let t = |id, profile_id| SeedTicket {
            id,
            profile_id,
            rating: 1000,
            waited_ms: 1_000,
            mode: "bo1",
        };
        seed_ticket(&app, t("t-r89-a", "r89-a")).await;
        seed_ticket(&app, t("t-r89-b", "r89-b")).await;

        // A room join lands between the sweep's busy-set read and the claim: it seats `r89-a`
        // in its own match first, as `series_recovery.rs`'s `race_first_update` writes its rival
        // row straight into the tables.
        fake(&app).await.before_call = Some(Arc::new(|method: &str, tables: &mut FakeTables| {
            if method == "tickets.claimPair"
                && let Some(row) = tables.profiles.iter_mut().find(|row| row.id == "r89-a")
                && row.in_match_id.is_none()
            {
                row.in_match_id = Some("r89-room-match".to_string());
            }
            CallOutcome::Proceed
        }));

        let report = try_pair(&app).await.expect("a sweep");
        fake(&app).await.before_call = None;

        // The pair never started: no match row, one failed start, and `r89-a` still points at
        // the room match the rival won while `r89-b` was never seated. Both tickets stay
        // claimed, as a failed start leaves them.
        assert_eq!(report.made, 0);
        assert_eq!(report.failed.len(), 1);
        assert!(matches_table(&app).await.is_empty());
        assert_eq!(in_match_of(&app, "r89-a").await, json!("r89-room-match"));
        assert_eq!(in_match_of(&app, "r89-b").await, Value::Null);
        assert_eq!(ticket(&app, "t-r89-a").await["status"], "matched");
        assert_eq!(ticket(&app, "t-r89-b").await["status"], "matched");
    }
}
