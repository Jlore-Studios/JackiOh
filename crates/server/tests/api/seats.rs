//! R1523000: one seat ledger in the server holds every profile's game, so no two doors seat one
//! profile twice (`src/seats.rs`).
//!
//! #1498's three interleavings, each over every pair of a 1v1 room, a Conquest room and a co-op
//! room: two guests joining one host's two rooms, one person joining two rooms at once, and the
//! queue sweep pairing a ticket while its owner joins a room. The fake store runs one transaction at
//! a time, so the rival door is the ledger call a second door would make, run from the store's race
//! seam (`before_call`) while the first door is in flight: it is refused there, the first door's
//! game stands alone, and the rival's own request is then refused with R1065000's 409 and writes
//! nothing. Then the ledger after a restart, the reaper's reconcile, and the epoch's fence.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};

use jackioh_server::api::queue::try_pair;
use jackioh_server::api::results::run_reaper;
use jackioh_server::app::{App, now_ms};
use jackioh_server::config::{MATCH_REAPER_INTERVAL_SECONDS, MAX_SAVED_DECKS, MAX_SAVED_TRIOS};
use jackioh_server::db::fake::{CallOutcome, FakeTables};
use jackioh_server::db::store::{CollectionEntry, MatchStatus, SeriesStatus, TicketStatus};
use jackioh_server::seats::{Lease, Seat, Seats, Taken};

use crate::support::deps::{TestAppOptions, add_user, call, empty_test_app, record_logs, test_app_with};
use crate::support::store::{fake, store, uuid};

/// The epoch-ms stamp the saved decks carry; nothing reads it back.
const AT: i64 = 1_700_000_000_000;

fn from<T: serde::de::DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value.clone()).unwrap_or_else(|error| panic!("{error}: {value}"))
}

/// The three kinds of game a door starts, and of seat the ledger holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// A 1v1 room's match (All Random).
    Match,
    /// A Conquest room's series.
    Series,
    /// A co-op room's match.
    Duos,
}

const KINDS: [Kind; 3] = [Kind::Match, Kind::Series, Kind::Duos];

fn kind_of(seat: &Seat) -> Kind {
    match seat {
        Seat::Match(_) => Kind::Match,
        Seat::Series { .. } => Kind::Series,
        Seat::Duos(_) => Kind::Duos,
    }
}

/// A seat of `kind` in a game nobody holds yet, as a rival door would reserve it.
fn rival_seat(kind: Kind) -> Seat {
    match kind {
        Kind::Match => Seat::Match("rival-match".to_string()),
        Kind::Series => Seat::Series {
            id: "rival-series".to_string(),
            game: None,
        },
        Kind::Duos => Seat::Duos("rival-duos".to_string()),
    }
}

// ---------------------------------------------------------------------------------------------
// Accounts, decks and trios (private copies: each test file of this binary keeps its own)
// ---------------------------------------------------------------------------------------------

/// Every playable catalog id, in `catalog.json` order.
fn playable() -> Vec<String> {
    jackioh_cards::register_all();
    jackioh_cards::CATALOG
        .iter()
        .filter(|(_, def)| {
            !def.token
                && !serde_json::to_value(&def.tags)
                    .expect("tags")
                    .as_array()
                    .is_some_and(|tags| tags.contains(&json!("Token")))
        })
        .map(|(id, _)| id.clone())
        .collect()
}

/// The `n`th run of `DECK_SIZE` playable ids: legal, and disjoint from every other `n`.
fn deck_at(n: usize) -> Vec<String> {
    let size = usize::try_from(jackioh_engine::config::DECK_SIZE).expect("a deck size");
    playable()[n * size..(n + 1) * size].to_vec()
}

struct Account {
    id: String,
    token: String,
    trio: String,
}

/// An active account owning every card, with three saved decks and a trio of them.
async fn account(app: &Arc<App>, n: u32) -> Account {
    let id = uuid(n);
    let user_id = format!("user-{n}");
    fake(app)
        .lock()
        .await
        .seed_profile(json!({ "id": id, "userId": user_id, "status": "active" }));
    let token = add_user(app, &user_id, &format!("{user_id}@example.test"), true);
    let rows: Vec<Value> = playable()
        .iter()
        .map(|card| json!({ "cardId": card, "quantity": 1 }))
        .collect();
    let entries: Vec<CollectionEntry> = from(Value::Array(rows));
    store!(app, t => t.collection_upsert_quantities(&id, &entries).await.expect("collection.upsertQuantities"));
    let base = n * 10;
    let deck_ids = [uuid(base + 1), uuid(base + 2), uuid(base + 3)];
    for (at, deck_id) in deck_ids.iter().enumerate() {
        store!(app, t => t
            .decks_upsert(
                &from(json!({
                    "id": deck_id,
                    "profileId": id,
                    "name": format!("deck {at}"),
                    "cards": deck_at(at),
                    "portrait": null,
                    "catalogVersion": jackioh_cards::catalog_version(),
                    "createdAt": AT,
                    "updatedAt": AT,
                })),
                MAX_SAVED_DECKS as _,
            )
            .await
            .expect("decks.upsert"));
    }
    let trio = uuid(base + 4);
    store!(app, t => t
        .trios_upsert(
            &from(json!({
                "id": trio,
                "profileId": id,
                "name": "trio",
                "deckIds": deck_ids,
                "createdAt": 0,
                "updatedAt": 0,
            })),
            MAX_SAVED_TRIOS as _,
        )
        .await
        .expect("trios.upsert"));
    Account { id, token, trio }
}

/// `count` accounts on a fresh app with no fixtures.
async fn accounts(count: u32) -> (Arc<App>, Vec<Account>) {
    jackioh_cards::register_all();
    let app = empty_test_app().await;
    let mut made = Vec::new();
    for n in 1..=count {
        made.push(account(&app, n).await);
    }
    (app, made)
}

async fn post(app: &Arc<App>, who: &Account, path: &str, body: Value) -> (u16, Value) {
    let (status, _headers, body) = call(app, "POST", path, Some(&who.token), body).await;
    (status, body)
}

/// A room of `kind` the host opens; answers its code.
async fn open(app: &Arc<App>, host: &Account, kind: Kind) -> String {
    let (status, body) = match kind {
        Kind::Match => post(app, host, "/api/rooms", json!({ "mode": "random" })).await,
        Kind::Series => {
            post(
                app,
                host,
                "/api/rooms",
                json!({ "mode": "bo3", "trioId": host.trio }),
            )
            .await
        }
        Kind::Duos => {
            post(
                app,
                host,
                "/api/duos/rooms",
                json!({ "difficulty": "easy", "random": true }),
            )
            .await
        }
    };
    assert!(status == 200 || status == 201, "{kind:?}: {status} {body}");
    body["code"].as_str().expect("a room code").to_string()
}

/// `who` joins the room of `kind` under `code`.
async fn join(app: &Arc<App>, who: &Account, kind: Kind, code: &str) -> (u16, Value) {
    match kind {
        Kind::Match => {
            post(
                app,
                who,
                &format!("/api/rooms/{code}/join"),
                json!({ "mode": "random" }),
            )
            .await
        }
        Kind::Series => {
            post(
                app,
                who,
                &format!("/api/rooms/{code}/join"),
                json!({ "mode": "bo3", "trioId": who.trio }),
            )
            .await
        }
        Kind::Duos => {
            post(
                app,
                who,
                &format!("/api/duos/rooms/{code}/join"),
                json!({ "random": true }),
            )
            .await
        }
    }
}

/// Whether nobody has claimed the room: the join that lost wrote nothing.
async fn unclaimed(app: &Arc<App>, kind: Kind, code: &str) -> bool {
    match kind {
        Kind::Match | Kind::Series => store!(app, t => t
            .rooms_get(code)
            .await
            .expect("rooms.get")
            .expect("the room")
            .guest_profile_id
            .is_none()),
        Kind::Duos => store!(app, t => t
            .duos_rooms_get(code)
            .await
            .expect("duos.rooms.get")
            .expect("the room")
            .guest
            .is_none()),
    }
}

/// Every game the store holds live: 1v1 matches, series not over, co-op matches not ended.
async fn games(app: &App) -> usize {
    let data = fake(app);
    let data = data.lock().await;
    let tables = &data.tables;
    tables
        .matches
        .iter()
        .filter(|row| row.status == MatchStatus::Live)
        .count()
        + tables
            .series
            .iter()
            .filter(|row| row.status != SeriesStatus::Over)
            .count()
        + tables
            .duos
            .matches
            .iter()
            .filter(|row| row.ended_at.is_none())
            .count()
}

async fn in_match(app: &Arc<App>, profile_id: &str) -> Option<String> {
    store!(app, t => t
        .profiles_get_by_id(profile_id)
        .await
        .expect("profiles.getById")
        .expect("the profile")
        .in_match_id)
}

/// The rival door: on the first `method` the store is asked, inside the first door's transaction,
/// `rival` runs against the app's ledger, and its answer is kept for the test to read.
async fn rival_at<T: Send + 'static>(
    app: &Arc<App>,
    method: &'static str,
    rival: impl Fn(&Seats) -> T + Send + Sync + 'static,
) -> Arc<Mutex<Option<T>>> {
    let answer: Arc<Mutex<Option<T>>> = Arc::new(Mutex::new(None));
    let fired = AtomicBool::new(false);
    let kept = Arc::clone(&answer);
    let weak = Arc::downgrade(app);
    fake(app).lock().await.before_call = Some(Arc::new(move |called: &str, _tables: &mut FakeTables| {
        if called == method
            && !fired.swap(true, Ordering::SeqCst)
            && let Some(app) = weak.upgrade()
        {
            *kept.lock().expect("the rival's answer") = Some(rival(&app.seats));
        }
        CallOutcome::Proceed
    }));
    answer
}

async fn clear_rival(app: &Arc<App>) {
    fake(app).lock().await.before_call = None;
}

/// The rival's reservation, refused on `profile`.
fn refused_on<T>(answer: &Arc<Mutex<Option<Result<T, Taken>>>>, profile: &str) -> Seat {
    match answer.lock().expect("the rival's answer").take() {
        Some(Err(taken)) => {
            assert_eq!(
                taken.profile, profile,
                "the rival is refused on the profile both doors want"
            );
            taken.seat
        }
        Some(Ok(_)) => panic!("the rival reserved a seat the first door holds"),
        None => panic!("the rival never ran"),
    }
}

// ---------------------------------------------------------------------------------------------
// #1498's interleavings
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn r1523000_two_guests_join_one_hosts_two_rooms() {
    for first in KINDS {
        for second in KINDS {
            let case = format!("{first:?} then {second:?}");
            let (app, people) = accounts(3).await;
            let [host, guest, rival] = &people[..] else {
                unreachable!()
            };
            let a = open(&app, host, first).await;
            let b = open(&app, host, second).await;

            let (rival_id, host_id) = (rival.id.clone(), host.id.clone());
            let answer = rival_at(&app, "seats.fence", move |seats| {
                seats
                    .reserve(&[&rival_id, &host_id], rival_seat(second))
                    .map(drop)
            })
            .await;
            let (status, body) = join(&app, guest, first, &a).await;
            assert_eq!(status, 200, "{case}: {body}");
            clear_rival(&app).await;
            let pending = refused_on(&answer, &host.id);
            assert_eq!(
                kind_of(&pending),
                first,
                "{case}: the host's seat was the first join's"
            );
            assert_eq!(games(&app).await, 1, "{case}: one live game");

            let (status, body) = join(&app, rival, second, &b).await;
            assert_eq!(status, 409, "{case}: {body}");
            assert_eq!(body["error"]["code"], "conflict", "{case}: {body}");
            assert!(
                unclaimed(&app, second, &b).await,
                "{case}: the losing join claimed nothing"
            );
            assert_eq!(games(&app).await, 1, "{case}: still one live game");
            assert_eq!(app.seats.of(&rival.id), None, "{case}");
            assert_eq!(in_match(&app, &rival.id).await, None, "{case}");
        }
    }
}

#[tokio::test]
async fn r1523000_one_person_joins_two_rooms_at_once() {
    for first in KINDS {
        for second in KINDS {
            let case = format!("{first:?} then {second:?}");
            let (app, people) = accounts(3).await;
            let [person, one, other] = &people[..] else {
                unreachable!()
            };
            let a = open(&app, one, first).await;
            let b = open(&app, other, second).await;

            let (person_id, other_id) = (person.id.clone(), other.id.clone());
            let answer = rival_at(&app, "seats.fence", move |seats| {
                seats
                    .reserve(&[&person_id, &other_id], rival_seat(second))
                    .map(drop)
            })
            .await;
            let (status, joined) = join(&app, person, first, &a).await;
            assert_eq!(status, 200, "{case}: {joined}");
            clear_rival(&app).await;
            refused_on(&answer, &person.id);
            assert_eq!(games(&app).await, 1, "{case}: one live game");

            let (status, body) = join(&app, person, second, &b).await;
            assert_eq!(status, 409, "{case}: {body}");
            assert_eq!(body["error"]["code"], "already_in_match", "{case}: {body}");
            let details = &body["error"]["details"];
            match first {
                Kind::Match => assert_eq!(*details, Value::Null, "{case}: {body}"),
                Kind::Series => assert_eq!(
                    *details,
                    json!({ "seriesId": joined["seriesId"] }),
                    "{case}: {body}"
                ),
                Kind::Duos => assert_eq!(
                    *details,
                    json!({ "duosMatchId": joined["matchId"] }),
                    "{case}: {body}"
                ),
            }
            assert!(
                unclaimed(&app, second, &b).await,
                "{case}: the losing join claimed nothing"
            );
            assert_eq!(games(&app).await, 1, "{case}: still one live game");
        }
    }
}

/// An open ticket written through the store, as a race leaves it: rating 1000, enqueued now, its
/// deck or trio frozen.
async fn write_ticket(app: &Arc<App>, id: &str, profile_id: &str, mode: &str) {
    let frozen = |n: usize| json!({ "name": format!("deck {n}"), "cards": deck_at(n) });
    let (deck, trio) = match mode {
        "bo3" => (
            json!([]),
            json!({ "name": "trio", "decks": [frozen(0), frozen(1), frozen(2)] }),
        ),
        _ => (json!(deck_at(0)), Value::Null),
    };
    let ticket = json!({
        "id": id,
        "profileId": profile_id,
        "rating": 1000,
        "mode": mode,
        "deck": deck,
        "trio": trio,
        "catalogVersion": app.catalog.version,
        "enqueuedAt": now_ms(),
        "status": "open",
        "matchId": null,
    });
    store!(app, t => t.tickets_insert(&from(ticket)).await.expect("tickets.insert"));
}

/// The seat the sweep's pair start reserves for a ticket of `mode`.
fn sweep_seat(mode: &str) -> Seat {
    match mode {
        "bo3" => rival_seat(Kind::Series),
        _ => rival_seat(Kind::Match),
    }
}

#[tokio::test]
async fn r1523000_a_join_beats_the_sweep_pairing_its_owners_ticket() {
    for mode in ["bo1", "bo3"] {
        for kind in KINDS {
            let case = format!("a {mode} ticket, a {kind:?} room");
            let (app, people) = accounts(3).await;
            let [owner, opponent, host] = &people[..] else {
                unreachable!()
            };
            write_ticket(&app, "t-owner", &owner.id, mode).await;
            write_ticket(&app, "t-opponent", &opponent.id, mode).await;
            let code = open(&app, host, kind).await;

            let (owner_id, opponent_id) = (owner.id.clone(), opponent.id.clone());
            let answer = rival_at(&app, "seats.fence", move |seats| {
                seats
                    .reserve(&[&owner_id, &opponent_id], sweep_seat(mode))
                    .map(drop)
            })
            .await;
            let (status, body) = join(&app, owner, kind, &code).await;
            assert_eq!(status, 200, "{case}: {body}");
            clear_rival(&app).await;
            refused_on(&answer, &owner.id);

            let report = try_pair(&app).await.expect("a sweep");
            assert_eq!(report.made, 0, "{case}");
            assert_eq!(games(&app).await, 1, "{case}: the join's game alone");
            assert_eq!(app.seats.of(&opponent.id), None, "{case}");
        }
    }
}

#[tokio::test]
async fn r1523000_the_sweep_pair_start_refuses_when_a_seat_was_taken_mid_flight() {
    for mode in ["bo1", "bo3"] {
        for kind in KINDS {
            let case = format!("a {mode} pair, a {kind:?} seat");
            let (app, people) = accounts(2).await;
            let [owner, opponent] = &people[..] else {
                unreachable!()
            };
            write_ticket(&app, "t-owner", &owner.id, mode).await;
            write_ticket(&app, "t-opponent", &opponent.id, mode).await;

            // A door that reserved and committed the owner's seat between the sweep's read and its
            // pair start, as a co-op join landing then does.
            let owner_id = owner.id.clone();
            let answer = rival_at(&app, "tickets.claimPair", move |seats| {
                seats.reserve(&[&owner_id], rival_seat(kind)).map(Lease::commit)
            })
            .await;
            let report = try_pair(&app).await.expect("a sweep");
            clear_rival(&app).await;
            assert_eq!(
                answer.lock().expect("the rival's answer").take(),
                Some(Ok(())),
                "{case}: the rival's door took the seat"
            );

            assert_eq!(report.made, 0, "{case}");
            assert_eq!(report.failed.len(), 1, "{case}: the pair's start failed");
            assert_eq!(report.failed[0].error.code.as_str(), "already_in_match", "{case}");
            let data = fake(&app);
            let data = data.lock().await;
            assert!(data.tables.matches.is_empty(), "{case}: no match");
            assert!(
                data.tables.reserved.is_empty(),
                "{case}: the claim's skeleton is gone"
            );
            assert!(data.tables.series.is_empty(), "{case}: no series");
            drop(data);
            assert_eq!(in_match(&app, &owner.id).await, None, "{case}");
            assert_eq!(in_match(&app, &opponent.id).await, None, "{case}");
            assert_eq!(app.seats.of(&owner.id), Some(rival_seat(kind)), "{case}");
            assert_eq!(app.seats.of(&opponent.id), None, "{case}: the lease gave it back");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Boot, reconcile and the fence
// ---------------------------------------------------------------------------------------------

/// A new process over `app`'s store, its ledger booted as `serve` boots it.
async fn restart(app: &Arc<App>) -> Arc<App> {
    let restarted = test_app_with(TestAppOptions {
        db: Some(app.db.clone()),
        ..TestAppOptions::default()
    })
    .await;
    jackioh_server::seats::boot(&restarted)
        .await
        .expect("the seat ledger boots");
    restarted
}

#[tokio::test]
async fn r1523000_a_restart_rebuilds_the_seats_of_a_live_co_op_match_and_a_series() {
    let (app, people) = accounts(4).await;
    let [host, guest, conquest_host, conquest_guest] = &people[..] else {
        unreachable!()
    };
    let code = open(&app, host, Kind::Duos).await;
    assert_eq!(join(&app, guest, Kind::Duos, &code).await.0, 200);
    let code = open(&app, conquest_host, Kind::Series).await;
    let (status, joined) = join(&app, conquest_guest, Kind::Series, &code).await;
    assert_eq!(status, 200, "{joined}");
    let series_id = joined["seriesId"].as_str().expect("a series id").to_string();
    let ids: Vec<String> = people.iter().map(|person| person.id.clone()).collect();
    let seats_on = |app: &Arc<App>| -> Vec<Option<Seat>> { ids.iter().map(|id| app.seats.of(id)).collect() };

    // During the series' pick phase.
    let before = seats_on(&app);
    assert!(matches!(before[0], Some(Seat::Duos(_))), "{before:?}");
    assert_eq!(
        before[2],
        Some(Seat::Series {
            id: series_id.clone(),
            game: None
        })
    );
    let picking = restart(&app).await;
    assert_eq!(seats_on(&picking), before);

    // After both picks the restarted process starts game 1.
    for (person, n) in [(conquest_host, 3), (conquest_guest, 4)] {
        let token = add_user(
            &picking,
            &format!("user-{n}"),
            &format!("user-{n}@example.test"),
            true,
        );
        let (status, _headers, body) = call(
            &picking,
            "POST",
            &format!("/api/series/{series_id}/pick"),
            Some(&token),
            json!({ "slot": 0, "gameNo": 1 }),
        )
        .await;
        assert_eq!(status, 200, "{}: {body}", person.id);
    }
    let playing = seats_on(&picking);
    assert!(
        matches!(&playing[2], Some(Seat::Series { id, game: Some(_) }) if *id == series_id),
        "{playing:?}"
    );
    assert_eq!(playing[2], playing[3]);
    let again = restart(&picking).await;
    assert_eq!(seats_on(&again), playing);
}

#[tokio::test(start_paused = true)]
async fn r1523000_a_seat_whose_ending_skipped_its_release_is_freed_within_one_reaper_tick() {
    let logs = record_logs();
    let (app, people) = accounts(2).await;
    let [host, guest] = &people[..] else {
        unreachable!()
    };
    let code = open(&app, host, Kind::Match).await;
    let (status, joined) = join(&app, guest, Kind::Match, &code).await;
    assert_eq!(status, 200, "{joined}");
    let match_id = joined["matchId"].as_str().expect("a match id").to_string();
    assert_eq!(app.seats.of(&host.id), Some(Seat::Match(match_id.clone())));

    // The match ends in the store with no release: no actor wrote it.
    app.matches.stop(&match_id).await;
    store!(app, t => t.matches_finish(&match_id, now_ms()).await.expect("matches.finish"));
    for person in [host, guest] {
        store!(app, t => t.profiles_set_in_match(&person.id, None).await.expect("profiles.setInMatch"));
    }
    assert_eq!(app.seats.of(&host.id), Some(Seat::Match(match_id.clone())));

    let reaper = tokio::spawn(run_reaper(app.clone()));
    tokio::time::sleep(Duration::from_secs(MATCH_REAPER_INTERVAL_SECONDS as u64 + 1)).await;
    reaper.abort();

    assert_eq!(app.seats.of(&host.id), None);
    assert_eq!(app.seats.of(&guest.id), None);
    let drift = logs.named("seats.drift");
    assert_eq!(drift.len(), 2, "{drift:?}");
}

#[tokio::test]
async fn r1523000_reconcile_adopts_a_seat_the_ledger_lacks() {
    let logs = record_logs();
    let (app, people) = accounts(1).await;
    let person = &people[0];
    store!(app, t => t.profiles_set_in_match(&person.id, Some("elsewhere")).await.expect("profiles.setInMatch"));
    assert_eq!(app.seats.of(&person.id), None);

    jackioh_server::seats::reconcile(&app).await;

    assert_eq!(
        app.seats.of(&person.id),
        Some(Seat::Match("elsewhere".to_string()))
    );
    assert_eq!(logs.named("seats.drift").len(), 1);
}

#[tokio::test]
async fn r1523000_a_claim_on_the_old_epoch_writes_nothing() {
    for kind in KINDS {
        let (app, people) = accounts(2).await;
        let [host, guest] = &people[..] else {
            unreachable!()
        };
        let code = open(&app, host, kind).await;

        // A newer server boots while the join is in flight: its epoch lands between the join's read
        // of the room and its claim.
        let read = if kind == Kind::Duos {
            "duos.rooms.get"
        } else {
            "rooms.get"
        };
        let fired = AtomicBool::new(false);
        fake(&app).lock().await.before_call = Some(Arc::new(move |called: &str, tables: &mut FakeTables| {
            if called == read && !fired.swap(true, Ordering::SeqCst) {
                tables.seat_epoch += 1;
            }
            CallOutcome::Proceed
        }));
        let (status, body) = join(&app, guest, kind, &code).await;
        clear_rival(&app).await;

        assert_eq!(status, 503, "{kind:?}: {body}");
        assert_eq!(body["error"]["code"], "unavailable", "{kind:?}: {body}");
        assert!(unclaimed(&app, kind, &code).await, "{kind:?}: no claim");
        assert_eq!(games(&app).await, 0, "{kind:?}: no game");
        for person in [host, guest] {
            assert_eq!(in_match(&app, &person.id).await, None, "{kind:?}: no flag");
            assert_eq!(app.seats.of(&person.id), None, "{kind:?}: no seat");
        }
    }
}

#[tokio::test]
async fn r1523000_a_sweep_on_the_old_epoch_claims_no_pair() {
    for mode in ["bo1", "bo3"] {
        let (app, people) = accounts(2).await;
        let [owner, opponent] = &people[..] else {
            unreachable!()
        };
        write_ticket(&app, "t-owner", &owner.id, mode).await;
        write_ticket(&app, "t-opponent", &opponent.id, mode).await;
        // A newer server has booted: the old one's sweep still runs until it is stopped.
        fake(&app).lock().await.tables.seat_epoch += 1;

        let report = try_pair(&app).await.expect("a sweep");
        assert_eq!(report.made, 0, "{mode}");
        assert_eq!(report.failed.len(), 1, "{mode}: the pair's claim was fenced");
        assert_eq!(report.failed[0].error.code.as_str(), "unavailable", "{mode}");

        let data = fake(&app);
        let data = data.lock().await;
        for ticket in &data.tables.tickets {
            assert_eq!(
                ticket.status,
                TicketStatus::Open,
                "{mode}: {} stays open",
                ticket.id
            );
            assert_eq!(ticket.match_id, None, "{mode}: {} names no match", ticket.id);
        }
        assert!(data.tables.ticket_claims.is_empty(), "{mode}: no claim");
        assert!(data.tables.reserved.is_empty(), "{mode}: no skeleton");
        assert!(data.tables.matches.is_empty(), "{mode}: no match");
        assert!(data.tables.series.is_empty(), "{mode}: no series");
        drop(data);
        for person in [owner, opponent] {
            assert_eq!(in_match(&app, &person.id).await, None, "{mode}: no flag");
            assert_eq!(app.seats.of(&person.id), None, "{mode}: no seat");
        }
    }
}
