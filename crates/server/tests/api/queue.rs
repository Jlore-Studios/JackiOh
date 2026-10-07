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
//!  - **R257**: a ticket pairs only with a ticket of its own mode; the population is reported per
//!    mode. A Best-of-3 pair becomes a series (R259), not a match.
//!  - **R258**: All Random deals both decks from the match seed and needs no saved deck.
//!  - **R264**: a profile in a series that is not over can neither queue nor be paired.
//!
//! Port of `apps/server/test/api/queue.test.ts`. Everything runs on tokio's paused clock through
//! `test_app()`, so `enqueuedAt` and the widening window are set by this file rather than by the
//! host clock (`app::now_ms()` reads tokio's clock).
//!
//! What moved, and why. `test_app()` is the production wiring on the fake store: the real catalog,
//! the real validator (`jackioh_engine::validator`) and the real registry, which starts a real
//! game on every pairing. So every deck here is real: the catalog's playable ids, sliced into
//! disjoint decks of the engine's `DECK_SIZE`, and every player owns the whole catalog (R111's
//! launch grant). TS's permissive validator, synthetic 24-id catalog and recording match directory
//! have no Rust counterpart: a started match is read off its row (`matches_get`, the fake's
//! `tables.matches`), where TS read `deps.matches.started`. The legacy `{ deckIndex }` body is gone
//! (SURFACE §11.3), so R165's "nothing saved" is a deck id that was never saved and R257's legacy
//! half is its refusal. A failing start is a store fault (`on_call`) instead of a replaced
//! `matches.start`, and R253's expected issues are read for their rule and sentence instead of being
//! recomputed through the adapter TS called.
//!
//! R166's fixture is built by writing tickets straight into the store. That is deliberate: the
//! ruling is about *which* qualifying opponent `try_pair` picks, and the only way to ask that
//! question is to control each ticket's age and rating exactly. The endpoint-driven tests below
//! (R165, R167) go through the router instead, so the route's auth declaration and §9.4's gate
//! take part.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use tokio::sync::MutexGuard;

use jackioh_engine::{DEFAULT_PORTRAIT, pick_portrait_from_seed};
use jackioh_server::actor::engine::deal_random_deck;
use jackioh_server::api::collection::grant_entire_catalog;
use jackioh_server::api::http::AuthLevel;
use jackioh_server::api::queue::{e2e_seed_count, try_pair};
use jackioh_server::app::{App, ROUTES, now_ms};
use jackioh_server::config::{MAX_SAVED_DECKS, MAX_SAVED_TRIOS, rating_window};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{Db, StoreError};

use crate::support::deps::{add_user, call, test_app};

// ---------------------------------------------------------------------------------------------
// Plumbing (private copies: each test file of this binary keeps its own)
// ---------------------------------------------------------------------------------------------

/// A port value built from TS's own object literal, so the test depends on the JSON shape only.
fn from<T: DeserializeOwned>(value: Value) -> T {
    match serde_json::from_value(value.clone()) {
        Ok(parsed) => parsed,
        Err(error) => panic!("{error}: {value}"),
    }
}

fn to_json<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// One store call in its own transaction, as every TS `deps.store.<x>.<y>(…)` call was.
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

/// R143's seeds are held in `queue.rs`'s process-wide map, which every test of this binary shares
/// (TS's module map was one per test file). The tests that send a seed, or count the map, take
/// this, so `e2e_seed_count()` sees only their own.
static SEED_MAP: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// `{ status, body }` of one request (TS's `router(jsonRequest(…))` and `readJson`).
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

// ---------------------------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------------------------

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

/// A field TS wrote `null` that a Rust row may leave absent: either reads as `null`.
fn nullable(row: &Value, key: &str) -> Value {
    row.get(key).cloned().unwrap_or(Value::Null)
}

/// The deck the started match gave this profile's seat (TS read the match directory's `started`).
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

/// The store's call log and a fault to raise in it, as TS's `onCall` gave both.
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

// ---------------------------------------------------------------------------------------------
// R165
// ---------------------------------------------------------------------------------------------

mod r165_queueing_with_no_saved_deck_at_all {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r165_reports_a_loadout_failure_never_a_404_for_a_profile_that_has_never_saved_one() {
        let app = test_app().await;
        let token = active_profile(&app, "never-built", 1000.0).await;

        let (status, body) = enqueue(&app, &token, &uuid()).await;

        // The ruling, exactly: "a queue-time refusal the player fixes in the deckbuilder reports as a
        // loadout failure". 422, alongside the L1–L6 failures.
        assert_eq!(body["error"]["code"], "loadout_invalid");
        assert_eq!(status, 422);
        // A 404 would say the *endpoint* found nothing, sending a client after a route that works.
        assert_ne!(status, 404);
        assert_ne!(body["error"]["code"], "not_found");
        // Nor is it a staleness problem: the remedy is the deckbuilder, not a client update.
        assert_ne!(body["error"]["code"], "stale_catalog");
        let message = body["error"]["message"].as_str().unwrap_or("").to_lowercase();
        assert!(message.contains("deck"), "message: {message}");

        // Nothing was queued on the way to the refusal.
        assert_eq!(open_count(&app).await, 0);
    }

    #[tokio::test(start_paused = true)]
    async fn r165s_control_the_same_profile_and_the_same_route_queue_fine_once_a_deck_is_saved() {
        // Without this the 422 above could be any of a dozen things the route refuses — a bad token, a
        // pending account, a broken catalog. Only one thing changes between the two calls.
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

// ---------------------------------------------------------------------------------------------
// R166
// ---------------------------------------------------------------------------------------------

mod r166_r108_which_qualifying_opponent_a_sweep_pairs {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r166_pairs_the_oldest_ticket_against_the_oldest_opponent_its_window_admits_not_the_closest() {
        // The whole point of the fixture: `near` is a far better rating match for `oldest` than `mid`
        // is, and it is the one R166 must NOT choose, because it queued later.
        //
        //   oldest  rating 1000, waited 30 s   <- paired first
        //   mid     rating 1080, waited 20 s   <- qualifies (gap 80), and is the oldest that does
        //   near    rating 1005, waited  1 s   <- qualifies (gap 5) and is far closer in rating
        //
        // Every wait is under 10 s of widening apart, so all three windows are ±100 at sweep time and
        // the choice is genuinely between "oldest" and "closest" rather than between "qualifies" and
        // "does not".
        let app = test_app().await;
        let oldest = (1000, 30_000);
        let mid = (1080, 20_000);
        let near = (1005, 1_000);

        // PREMISE: both candidates really are inside both windows, so what follows is R166's *choice*
        // and not §9.5's window quietly excluding one of them.
        expect_qualifies(oldest, mid);
        expect_qualifies(oldest, near);
        // …and `near` is the better rating match by a wide margin, which is what it must not win on.
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

        let made = try_pair(&app).await.expect("a sweep");

        // PREMISE: a pair really was made, and exactly one — three tickets cannot make two.
        assert_eq!(made, 1);
        assert_eq!(paired_profiles(&app).await, vec![json!(["oldest", "mid"])]);

        // The closer-rated newcomer is still waiting, which is the half that discriminates: a
        // closest-rating matcher would have paired `oldest` with `near` and left `mid`.
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

        let made = try_pair(&app).await.expect("a sweep");

        assert_eq!(made, 2);
        // The oldest pair is made first, and each ticket takes the oldest opponent left to it.
        assert_eq!(
            paired_profiles(&app).await,
            vec![json!(["first", "second"]), json!(["third", "fourth"])]
        );
        assert_eq!(open_count(&app).await, 0);
    }

    #[tokio::test(start_paused = true)]
    async fn r166_breaks_a_tie_on_ticket_id_so_the_same_open_set_always_pairs_the_same_way() {
        // `t-zebra` is inserted before `t-alpha` and they queued at the very same instant. Insertion
        // order would pair `oldest` with `t-zebra`; the id tie-break pairs it with `t-alpha`.
        let app = test_app().await;
        let oldest = (1000, 30_000);
        let zebra = (1010, 5_000);
        let alpha = (1020, 5_000);

        // PREMISE: both tie-ed candidates qualify, so the loser below loses on its id and nothing else.
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

        // PREMISE: the store really does hand them over in insertion order for an equal `enqueuedAt`,
        // so the assertion below is about `try_pair`'s sort and not about the store's.
        let open: Vec<Value> = to_json(&q!(app, tickets_list_open()))
            .as_array()
            .map(|rows| rows.iter().map(|row| row["id"].clone()).collect())
            .unwrap_or_default();
        assert_eq!(open, vec![json!("t-oldest"), json!("t-zebra"), json!("t-alpha")]);

        assert_eq!(try_pair(&app).await.expect("a sweep"), 1);
        assert_eq!(paired_profiles(&app).await, vec![json!(["oldest", "alpha"])]);
        assert_eq!(ticket(&app, "t-zebra").await["status"], "open");
    }

    #[tokio::test(start_paused = true)]
    async fn r166_stays_inside_95s_window_an_opponent_both_windows_refuse_is_not_paired_at_all() {
        // The control on "oldest first": it never drags in someone the window excludes. Both have
        // waited under 10 s, so both windows are ±100 and a gap of 300 is outside them.
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

        assert_eq!(try_pair(&app).await.expect("a sweep"), 0);
        assert!(matches_table(&app).await.is_empty());
        assert_eq!(open_count(&app).await, 2);

        // …and once the older one has waited long enough for its window to widen past the gap, the
        // same two pair. (§9.5: uncapped after 60 s, so both windows admit the gap by then.)
        tokio::time::advance(Duration::from_millis(120_000)).await;
        assert_eq!(try_pair(&app).await.expect("a sweep"), 1);
        assert_eq!(paired_profiles(&app).await, vec![json!(["low", "high"])]);
    }
}

// ---------------------------------------------------------------------------------------------
// R167
// ---------------------------------------------------------------------------------------------

mod r167_r108_r143_how_a_player_leaves_the_queue {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r167_cancels_an_open_ticket_and_says_so() {
        let app = test_app().await;
        let token = active_profile(&app, "leaver", 1000.0).await;
        let [deck, ..] = decks_from();
        let deck_id = save_deck_for(&app, "leaver", &deck, "leaver's deck").await;

        // PREMISE: there is something to cancel. Every negative below is vacuous without it.
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

        // §9.5's enqueue condition — active and not in a match — is satisfiable again, which is the
        // reason R167 exists: without cancel, a player who queued by mistake waits to be paired.
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

        // PREMISE, again: cancel the real thing first, so "cancelled: false" below means "already
        // gone" rather than "nothing was ever queued".
        let ticket_id = enqueue(&app, &token, &deck_id).await.1["ticketId"]
            .as_str()
            .unwrap_or("")
            .to_string();
        assert_eq!(cancel(&app, &token).await.1["cancelled"], true);

        let (status, second) = cancel(&app, &token).await;

        assert_eq!(status, 200);
        assert_eq!(second, json!({ "cancelled": false }));
        // The first cancel is not undone or re-applied by the second.
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

        // PREMISE: the race R167 is about really happened — the sweeper on enqueue paired them.
        assert_eq!(paired["status"], "matched");
        assert_ne!(paired["matchId"], Value::Null);
        assert_eq!(ticket(&app, &first_ticket).await["status"], "matched");

        // The client's cancel arrives after the pairing. It closes nothing.
        let (status, late) = cancel(&app, &one).await;

        assert_eq!(status, 200);
        assert_eq!(late, json!({ "cancelled": false }));
        // The match stands, and the player still belongs in it.
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
        // The control on "always 200": `DELETE /api/queue` is not an open door that answers
        // `cancelled: false` to anybody who asks.
        let app = test_app().await;
        let token = pending_profile(&app, "pending").await;

        let (status, body) = cancel(&app, &token).await;

        assert_eq!(status, 403);
        assert_eq!(body["error"]["code"], "account_pending");
    }
}

// ---------------------------------------------------------------------------------------------
// §9.4's gate on the queue (BUILD M6-T1)
// ---------------------------------------------------------------------------------------------

/// BUILD M6-T1's last acceptance item: "a pending account cannot call collection, loadout or queue
/// endpoints (403)". Collection and decks are checked against their own routes in `collection.rs`
/// and `decks.rs`; the queue half was only ever checked for `DELETE` (R167's test above), which left
/// enqueueing — the endpoint §9.4 actually names when it says a pending account gets "no
/// collection, loadout, queue or match" — asserted by nobody.
///
/// Its own block rather than a sixth test inside R167's: R167 is a ruling about *leaving* the
/// queue, and this is M6-T1's gate on entering it.
mod section_94s_gate_on_the_queue {
    use super::*;

    #[test]
    fn declares_both_queue_mutations_active_so_the_gate_is_on_the_route_and_not_in_a_handler() {
        let declared: Vec<(String, bool)> = ROUTES
            .iter()
            .filter(|route| route.1 == "/api/queue")
            .map(|route| (route.0.to_string(), matches!(route.2, AuthLevel::Active)))
            .collect();
        // `src/api/queue.rs` says the gate "is §9.4's gate ... so a pending account gets 403 here
        // without this handler saying anything about it". That is only true while these say `Active`.
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
        // The refusal happens before the handler, so no ticket exists and nothing was logged as queued.
        assert!(tickets_table(&app).await.is_empty());
        assert_eq!(open_count(&app).await, 0);
    }

    #[tokio::test(start_paused = true)]
    async fn the_control_the_same_profile_and_the_same_request_queue_fine_once_the_account_is_active() {
        // Without this, the 403 above could be any of the other things the route refuses. The deck is
        // saved first in both cases, so the only difference between the two calls is `status`.
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

// ---------------------------------------------------------------------------------------------
// §9.4 / §9.5 / §9.8 — the deck is frozen into the ticket
// ---------------------------------------------------------------------------------------------

/// §9.8's abuse vector, by its own name: "Deck swapped after matchmaking → decks are frozen into the
/// ticket". §9.4 states the rule ("Decks are frozen into the queue ticket") and `src/api/queue.rs`
/// claims it in its header: nothing re-reads a saved deck after the ticket exists.
///
/// So this block runs the vector end to end and through HTTP — enqueue, then **save a different
/// deck under the same id**, then pair — through the deck routes of the same router, because "edits
/// the deck" is something the player does with `PUT /api/decks/:id` and not something a test does
/// to the store.
///
/// Every case carries its control, and the controls are the point: an assertion that a match used
/// deck A is worthless unless the same fixture, with the save moved earlier, uses deck B.
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

        // PREMISE: the two decks share no card at all. Without this the assertions below could be
        // satisfied by a match that used the *new* deck and simply looked the same.
        assert!(!frozen.is_empty());
        assert!(frozen.iter().all(|card_id| !substitute.contains(card_id)));

        // 1. Queue with the deck. The ticket freezes it here and nowhere else.
        let (_, queued) = enqueue_with(&app, &swapper, json!({ "mode": "bo1", "deckId": deck_id })).await;
        let ticket_id = queued["ticketId"].as_str().unwrap_or("").to_string();
        assert_eq!(queued["status"], "open");
        assert_eq!(ticket(&app, &ticket_id).await["deck"], json!(frozen));

        // 2. The swap, through the endpoint a player would use, while the ticket is still open.
        assert_eq!(save(&app, &swapper, &deck_id, &substitute, None).await, 200);
        // PREMISE: the save really landed. A rejected save would make every assertion below pass for
        // the wrong reason — there would be nothing to leak into the match.
        let saved = q!(app, decks_get(&deck_id))
            .map(|deck| to_json(&deck))
            .unwrap_or(Value::Null);
        assert_eq!(saved["cards"], json!(substitute));
        // …and the ticket is untouched by it.
        assert_eq!(ticket(&app, &ticket_id).await["deck"], json!(frozen));

        // 3. Someone pairs with the queued player, and the match is created.
        let (_, paired) = enqueue_with(&app, &rival, json!({ "mode": "bo1", "deckId": rival_deck })).await;
        assert_eq!(paired["status"], "matched");
        let match_id = paired["matchId"].as_str().expect("a match id").to_string();

        // §9.4, §9.5: the match runs the deck the ticket froze, not the one the player is holding now.
        assert_eq!(seat_deck(&app, &match_id, "swapper").await, json!(frozen));
        // The substitute deck reached neither the match row nor anything stored with it.
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
        // Without this, the test above would pass against a queue that ignored saved decks entirely.
        // Exactly one thing moves between the two: whether the save happens before or after enqueue.
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

// ---------------------------------------------------------------------------------------------
// R257 — queue modes
// ---------------------------------------------------------------------------------------------

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

        // SURFACE §11.3: the legacy `{ deckIndex }` body, with no mode, is gone; it queues nothing.
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
        // Same rating, long waits: every window admits every other ticket, so only the mode can stop a
        // pairing.
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

        assert_eq!(try_pair(&app).await.expect("a sweep"), 0);
        assert!(matches_table(&app).await.is_empty());
        assert!(series_table(&app).await.is_empty());
        assert_eq!(open_count(&app).await, 3);

        // The control: a second All Random ticket pairs with the first one, and only with it — even
        // though the Best-of-1 and Best-of-3 tickets are older.
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
        assert_eq!(try_pair(&app).await.expect("a sweep"), 1);
        assert_eq!(paired_profiles(&app).await, vec![json!(["random", "random-2"])]);
        assert_eq!(ticket(&app, "t-bo1").await["status"], "open");
        assert_eq!(ticket(&app, "t-bo3").await["status"], "open");
    }

    #[tokio::test(start_paused = true)]
    async fn r257_reports_the_queue_population_in_total_and_per_mode() {
        let app = test_app().await;
        seed_ticket(
            &app,
            SeedTicket {
                id: "t-a",
                profile_id: "a",
                rating: 1000,
                waited_ms: 0,
                mode: "bo1",
            },
        )
        .await;
        seed_ticket(
            &app,
            SeedTicket {
                id: "t-b",
                profile_id: "b",
                rating: 1500,
                waited_ms: 0,
                mode: "bo3",
            },
        )
        .await;
        seed_ticket(
            &app,
            SeedTicket {
                id: "t-c",
                profile_id: "c",
                rating: 2000,
                waited_ms: 0,
                mode: "bo3",
            },
        )
        .await;
        let token = active_profile(&app, "watcher", 1000.0).await;

        let (status, body) = request(&app, "GET", "/api/queue/population", &token, None).await;

        assert_eq!(status, 200);
        assert_eq!(
            body,
            json!({ "population": 3, "byMode": { "bo1": 1, "bo3": 2, "random": 0 } })
        );
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

// ---------------------------------------------------------------------------------------------
// R258 — All Random
// ---------------------------------------------------------------------------------------------

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
        // The older ticket is p1 (R166: oldest first, ties on the ticket id). TS's fake ids rose with
        // each enqueue, so its tie went to the first; the server's ids are random, so the first
        // enqueue is made older by a millisecond of the paused clock instead.
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
        let p1_deck = json!(deal_random_deck(&format!("{seed}:p1-deck")));
        let p2_deck = json!(deal_random_deck(&format!("{seed}:p2-deck")));
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
        // The older ticket is p1 (R166: oldest first, ties on the ticket id). TS's fake ids rose with
        // each enqueue, so its tie went to the first; the server's ids are random, so the first
        // enqueue is made older by a millisecond of the paused clock instead.
        tokio::time::advance(Duration::from_millis(1)).await;
        let (_, paired) = enqueue_with(&app, &two, json!({ "mode": "random" })).await;
        let match_id = paired["matchId"].as_str().expect("a match id").to_string();

        let row = q!(app, matches_get(&match_id))
            .map(|row| to_json(&row))
            .expect("the match row");
        assert_eq!(row["seed"], "spec-seed");
        assert_eq!(
            seat_deck(&app, &match_id, "pin-one").await,
            json!(deal_random_deck("spec-seed:p1-deck"))
        );
        assert_eq!(
            seat_deck(&app, &match_id, "pin-two").await,
            json!(deal_random_deck("spec-seed:p2-deck"))
        );
    }
}

// ---------------------------------------------------------------------------------------------
// A start that fails
// ---------------------------------------------------------------------------------------------

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
        // start that fails after the claim (TS replaced `matches.start` with one that threw).
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

        // Free means free: the same player can queue straight back up (their ticket is claimed,
        // never open, so nothing blocks a fresh enqueue).
        fake(&app).await.on_call = None;
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
        // The claim won, `series.create` had already landed inside the transaction, and then the
        // start failed — what a transaction exists to take back. Without it the 'picking' row would
        // hold both players out of the queue (R264) until the pick deadline ran the series out
        // (R333), and the `open` skeleton forever. The fault is the one store call right after the
        // series row is written (TS made the log line after it throw), and only that one: the
        // cleanup after it must go through.
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

// ---------------------------------------------------------------------------------------------
// R642 — the portrait on the ticket and the seat
// ---------------------------------------------------------------------------------------------

/// R642, the queue half: a Best-of-1 ticket freezes its deck's portrait with the deck (§9.4, §9.8),
/// an All Random match deals each seat's portrait from the match seed the way it deals the deck
/// (R258), and either way the pair lands on the match row seat-ordered — which is what the actor's
/// `portraits` frame reads back.
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

        // 1. Queue with the deck: its portrait freezes into the ticket here and nowhere else.
        let (_, queued) = enqueue_with(&app, &swapper, json!({ "mode": "bo1", "deckId": deck_id })).await;
        let ticket_id = queued["ticketId"].as_str().unwrap_or("").to_string();
        assert_eq!(queued["status"], "open");
        assert_eq!(ticket(&app, &ticket_id).await["portrait"], "gary");

        // 2. The re-save, while the ticket is still open. It lands — and the ticket does not move.
        assert_eq!(save(&app, &swapper, &deck_id, &mine, Some("timmy")).await, 200);
        // PREMISE: the new portrait really is what the deck holds now, so there is something to leak.
        let deck = q!(app, decks_get(&deck_id))
            .map(|deck| to_json(&deck))
            .unwrap_or(Value::Null);
        assert_eq!(deck["portrait"], "timmy");
        assert_eq!(ticket(&app, &ticket_id).await["portrait"], "gary");
        // The older ticket is p1 (R166: oldest first, ties on the ticket id). TS's fake ids rose with
        // each enqueue, so its tie went to the first; the server's ids are random, so the first
        // enqueue is made older by a millisecond of the paused clock instead.
        tokio::time::advance(Duration::from_millis(1)).await;

        // 3. The pairing: the older ticket is p1, and each seat carries what its ticket froze.
        let (_, paired) = enqueue_with(&app, &rival, json!({ "mode": "bo1", "deckId": rival_deck_id })).await;
        assert_eq!(paired["status"], "matched");

        // …and the match row keeps them in seat order, which is what `portraits` frames read back.
        let row = matches_table(&app).await.last().cloned().expect("the match row");
        assert_eq!(row["players"], json!(["swapper", "rival"]));
        assert_eq!(row["portraits"], json!(["gary", "shredder"]));
    }

    #[tokio::test(start_paused = true)]
    async fn r642_reads_a_best_of_1_ticket_with_no_portrait_as_vanilla_on_the_seat_and_the_row() {
        let app = test_app().await;
        let one = active_profile(&app, "plain-one", 1000.0).await;
        let two = active_profile(&app, "plain-two", 1000.0).await;
        // `save_deck_for` writes `portrait: null` — R641's default, and what a deck saved before
        // portraits existed holds.
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
        // The older ticket is p1 (R166: oldest first, ties on the ticket id). TS's fake ids rose with
        // each enqueue, so its tie went to the first; the server's ids are random, so the first
        // enqueue is made older by a millisecond of the paused clock instead.
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
        // Each seat's pick is `pick_portrait_from_seed` on its own seat-keyed suffix — the same scheme
        // the decks are dealt on — so both `:portrait:p1` and `:portrait:p2` are covered here.
        let expected = json!([
            pick_portrait_from_seed("portrait-seed:portrait:p1"),
            pick_portrait_from_seed("portrait-seed:portrait:p2"),
        ]);
        assert_eq!(row["portraits"], expected);
    }
}

// ---------------------------------------------------------------------------------------------
// R264 — a profile in a series
// ---------------------------------------------------------------------------------------------

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

        assert_eq!(try_pair(&app).await.expect("a sweep"), 0);
        assert_eq!(ticket(&app, "t-right").await["status"], "open");
    }
}

// ---------------------------------------------------------------------------------------------
// R253 — the real validator at enqueue
// ---------------------------------------------------------------------------------------------

/// The production wiring: the real §8 catalog, the real validator, R111's launch grant, and
/// `POST /api/queue`. It does not re-test L1–L6 — `crates/engine/src/validator.rs` does — only that
/// a deck or trio the shared module refuses is refused at enqueue, with the module's own issues, the
/// decks' own names in them, and no ticket written. No sentence is typed out: every expectation is
/// read off the refusal itself (its first issue's rule, and that the error's message is that issue's).
mod r253_what_may_be_queued_through_the_real_validator {
    use super::*;

    struct Real {
        app: Arc<App>,
        token: String,
        pool: Vec<String>,
        size: usize,
    }

    /// The production wiring for one active profile that owns every card. The deck size is the
    /// engine config's (BUILD §2), never spelled here.
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
