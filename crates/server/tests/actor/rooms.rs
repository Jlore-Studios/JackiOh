//! Room challenge tests (SPEC §9.5, R79, BUILD M6-T4, M8, SURFACE §11.2, §11.3).
//! Rulings: R143 (seed), R149 (bounded mint), R264 (modes), R1075000 (cancel/list).

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};

use jackioh_engine::newest_shipped_set;
use jackioh_engine::wire::{DEFAULT_PORTRAIT, PortraitId, pick_portrait_from_seed};
use jackioh_server::actor::engine::deal_random_deck;
use jackioh_server::actor::rooms::{e2e_room_seed_count, script_room_codes};
use jackioh_server::api::queue::seat_coin_p1_first;
use jackioh_server::app::{App, now_ms};
use jackioh_server::config::{MAX_SAVED_DECKS, MAX_SAVED_TRIOS, ROOM_CODE_LENGTH, ROOM_CODE_TTL_SECONDS};
use jackioh_server::cosmetics::portrait_cosmetic_id;
use jackioh_server::db::fake::{CallOutcome, FakeTables};
use jackioh_server::db::store::{ChallengeRow, CollectionEntry};
use jackioh_server::economy::store::{CosmeticGrant, EconomyReason};

use crate::support::deps::{TestAppOptions, add_user, call, record_logs, test_app, test_app_with};
use crate::support::engine::playable;
use crate::support::store::{fake, from, store, table, to_json, uuid};

const HOST: &str = "host";
const GUEST: &str = "guest";

/// The epoch-ms stamp the saved decks carry; nothing reads it back.
const AT: i64 = 1_700_000_000_000;

/// R143's seeds sit in one process-wide map (`e2e_room_seed_count`); these tests take turns so one
/// test's seed is never another's count.
static ROOMS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Matches registry started, oldest first; index 0 of `players` and `decks` is p1.
async fn started(app: &App) -> Vec<Value> {
    table(app, |data| json!(data.tables.matches))
        .await
        .into_iter()
        .filter(|row| row["status"] != "open")
        .collect()
}

async fn in_match_of(app: &Arc<App>, profile: &str) -> Value {
    let row = store!(app, t => t.profiles_get_by_id(profile).await.expect("profiles.getById"))
        .expect("the profile");
    to_json(&row)["inMatchId"].clone()
}

fn deck_size() -> usize {
    usize::try_from(jackioh_engine::config::DECK_SIZE).expect("a deck size")
}

/// The `n`th run of `DECK_SIZE` playable ids: legal, and disjoint from every other `n`.
fn deck_at(n: usize) -> Vec<String> {
    let size = deck_size();
    playable()[n * size..(n + 1) * size].to_vec()
}

fn deck() -> Vec<String> {
    deck_at(0)
}

async fn save_deck(app: &Arc<App>, profile_id: &str, id: &str, cards: &[String], name: &str) {
    let outcome = store!(app, t => t
        .decks_upsert(
            &from(json!({
                "id": id,
                "profileId": profile_id,
                "name": name,
                "cards": cards,
                "portrait": null,
                "catalogVersion": jackioh_cards::catalog_version(),
                "createdAt": AT,
                "updatedAt": AT,
            })),
            MAX_SAVED_DECKS as _,
        )
        .await
        .expect("decks.upsert"));
    assert_eq!(to_json(&outcome), "created");
}

/// Every playable card in the profile's collection, so the real validator's L5 passes.
async fn own_everything(app: &Arc<App>, profile_id: &str) {
    let rows: Vec<Value> = playable()
        .iter()
        .map(|id| json!({ "cardId": id, "quantity": 1 }))
        .collect();
    let entries: Vec<CollectionEntry> = from(Value::Array(rows));
    store!(app, t => t.collection_upsert_quantities(profile_id, &entries).await.expect("collection.upsertQuantities"));
}

/// Seeds an active profile for a fresh user and answers its token.
async fn player(app: &Arc<App>, profile_id: &str, user_id: &str, email: &str) -> String {
    let token = add_user(app, user_id, email, true);
    fake(app)
        .lock()
        .await
        .seed_profile(json!({ "id": profile_id, "userId": user_id, "status": "active" }));
    own_everything(app, profile_id).await;
    token
}

/// Host and guest, active, each with one saved deck — `deck()` — so the default choice the helpers
/// below send is Best of 1 on it (R257).
struct Harness {
    app: Arc<App>,
    host: String,
    guest: String,
}

async fn harness(e2e: bool) -> Harness {
    let app = test_app_with(TestAppOptions {
        e2e,
        skip_fixtures: true,
        ..TestAppOptions::default()
    })
    .await;
    let host = player(&app, HOST, "user-host", "host@example.test").await;
    let guest = player(&app, GUEST, "user-guest", "guest@example.test").await;
    save_deck(&app, HOST, &uuid(1), &deck(), "host's deck").await;
    save_deck(&app, GUEST, &uuid(2), &deck(), "guest's deck").await;
    Harness { app, host, guest }
}

/// R1827000: `profile_id` owns `portraits` on its account, as a pack's drop leaves them, so a deck's
/// save may name them.
async fn own_portraits(app: &App, profile_id: &str, portraits: &[PortraitId]) {
    let mut tx = app.db.begin(Some(profile_id)).await.expect("a transaction");
    for portrait in portraits {
        let granted = tx
            .economy_cosmetics_grant(&CosmeticGrant {
                profile_id: profile_id.to_string(),
                card_id: None,
                cosmetic_id: portrait_cosmetic_id(*portrait),
                delta: 1,
                reason: EconomyReason::PackOpen,
                reference: None,
                dev: false,
                at: now_ms(),
            })
            .await
            .expect("the grant");
        assert!(granted, "{portrait} was already owned");
    }
    tx.commit().await.expect("the commit");
}

/// The caller's saved deck, Best of 1, under whatever `body` sets.
fn with_deck(deck_id: &str, body: Value) -> Value {
    let mut merged = json!({ "mode": "bo1", "deckId": deck_id });
    if let Value::Object(fields) = body {
        for (key, value) in fields {
            merged[key.as_str()] = value;
        }
    }
    merged
}

async fn post(app: &Arc<App>, token: &str, path: &str, body: Option<Value>) -> (u16, Value) {
    let (status, _headers, answer) = call(app, "POST", path, Some(token), body.unwrap_or(Value::Null)).await;
    (status, answer)
}

async fn create(app: &Arc<App>, token: &str, body: Value) -> (u16, Value) {
    post(app, token, "/api/rooms", Some(body)).await
}

async fn join(app: &Arc<App>, token: &str, code: &str, body: Value) -> (u16, Value) {
    post(app, token, &format!("/api/rooms/{code}/join"), Some(body)).await
}

async fn cancel(app: &Arc<App>, token: &str, code: &str) -> (u16, Value) {
    let (status, _headers, answer) = call(
        app,
        "DELETE",
        &format!("/api/rooms/{code}"),
        Some(token),
        Value::Null,
    )
    .await;
    (status, answer)
}

async fn open_rooms(app: &Arc<App>, token: &str) -> (u16, Value) {
    let (status, _headers, answer) = call(app, "GET", "/api/rooms", Some(token), Value::Null).await;
    (status, answer)
}

impl Harness {
    async fn create(&self, body: Value) -> (u16, Value) {
        create(&self.app, &self.host, with_deck(&uuid(1), body)).await
    }

    async fn create_as_guest(&self, body: Value) -> (u16, Value) {
        create(&self.app, &self.guest, with_deck(&uuid(2), body)).await
    }

    async fn join(&self, code: &str, body: Value) -> (u16, Value) {
        join(&self.app, &self.guest, code, with_deck(&uuid(2), body)).await
    }

    async fn rooms(&self) -> Vec<Value> {
        table(&self.app, |data| json!(data.tables.rooms)).await
    }
}

async fn play_through(h: &Harness, body: Value, join_body: Value) -> (String, String) {
    let (status, created) = h.create(body).await;
    assert_eq!(status, 200, "{created}");
    let code = created["code"].as_str().expect("a code").to_string();

    let (status, joined) = h.join(&code, join_body).await;
    assert_eq!(status, 200, "{joined}");

    let match_row = started(&h.app).await.pop().expect("the join started a match");
    (code, match_row["seed"].as_str().unwrap_or_default().to_string())
}

mod the_room_code_challenge {
    //! The room-code challenge (§9.5).
    use super::*;

    #[tokio::test]
    async fn creates_a_room_and_starts_the_match_on_the_join_with_the_coin_seating_either_side_first() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let (code, _) = play_through(&h, json!({}), json!({})).await;

        assert_eq!(code.chars().count(), ROOM_CODE_LENGTH);
        let row = started(&h.app).await.remove(0);
        // The frozen decks, seat-ordered; both decks hold the same cards here.
        assert_eq!(row["decks"], json!([deck(), deck()]));
        // R1989000: the coin on the match seed seats the host and the joiner.
        let seed = row["seed"].as_str().unwrap_or_default().to_string();
        let host_is_p1 = seat_coin_p1_first(&seed);
        let (first, second) = if host_is_p1 { (HOST, GUEST) } else { (GUEST, HOST) };
        assert_eq!(row["players"], json!([first, second]));
        // R604: a room challenge is never ranked.
        assert_eq!(row["ranked"], json!(false));
        // R376: the mode its game record is filed under is read off the room.
        let match_id = row["id"].as_str().expect("a match id").to_string();
        let mode = store!(h.app, t => t.matches_mode_of(&match_id).await.expect("matches.modeOf"));
        assert_eq!(to_json(&mode), "bo1");
        // §9.5: both ends of the lifecycle read the in-match flag.
        assert_eq!(in_match_of(&h.app, HOST).await, json!(match_id));
        assert_eq!(in_match_of(&h.app, GUEST).await, json!(match_id));
    }
}

mod r143_the_optional_seed_on_the_room_endpoints {
    //! R143 — the optional seed on the room endpoints.
    use super::*;

    #[tokio::test]
    async fn r143_rejects_seed_on_post_api_rooms_outside_end_to_end_mode_and_mints_no_room() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;

        let (status, body) = h.create(json!({ "seed": "05-reconnect" })).await;

        assert_eq!(status, 400);
        assert_eq!(body["error"]["code"], "bad_request");
        assert!(
            body["error"]["message"]
                .as_str()
                .unwrap_or_default()
                .contains("seed"),
            "{body}"
        );
        // Refused, never ignored: the request bought nothing.
        assert!(h.rooms().await.is_empty());
    }

    #[tokio::test]
    async fn r143_rejects_seed_on_the_join_endpoint_outside_end_to_end_mode() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let (_, created) = h.create(json!({})).await;
        let code = created["code"].as_str().expect("a code").to_string();

        let (status, body) = h.join(&code, json!({ "seed": "05-reconnect" })).await;

        assert_eq!(status, 400);
        assert!(
            body["error"]["message"]
                .as_str()
                .unwrap_or_default()
                .contains("seed"),
            "{body}"
        );
        assert!(started(&h.app).await.is_empty());
    }

    #[tokio::test]
    async fn r143_uses_the_hosts_seed_verbatim_for_the_match_the_join_creates_in_end_to_end_mode() {
        let _turn = ROOMS.lock().await;
        let h = harness(true).await;

        let (_, seed) = play_through(&h, json!({ "seed": "06-room-code" }), json!({})).await;

        assert_eq!(seed, "06-room-code");
        // Consumed with the room, so nothing accumulates across a long-running server.
        assert_eq!(e2e_room_seed_count() as i64, 0);
    }

    #[tokio::test]
    async fn r143_takes_the_joiners_seed_when_the_host_supplied_none_and_the_hosts_when_both_did() {
        let _turn = ROOMS.lock().await;
        let joiner_only = harness(true).await;
        assert_eq!(
            play_through(&joiner_only, json!({}), json!({ "seed": "from-the-joiner" }))
                .await
                .1,
            "from-the-joiner"
        );

        // Both: the room was created first, so its seed is the one the match runs on.
        let both = harness(true).await;
        assert_eq!(
            play_through(
                &both,
                json!({ "seed": "from-the-host" }),
                json!({ "seed": "from-the-joiner" })
            )
            .await
            .1,
            "from-the-host"
        );
        assert_eq!(e2e_room_seed_count() as i64, 0);
    }

    #[tokio::test]
    async fn r143_still_mints_a_seed_when_none_is_supplied_s9_3_the_server_owns_it() {
        let _turn = ROOMS.lock().await;
        let h = harness(true).await;
        // The server's own seed is opaque, and present.
        let (_, seed) = play_through(&h, json!({}), json!({})).await;
        assert!(!seed.is_empty());
    }

    #[tokio::test]
    async fn r143_refuses_a_seed_that_is_not_a_non_empty_string_even_in_end_to_end_mode() {
        let _turn = ROOMS.lock().await;
        let h = harness(true).await;

        assert_eq!(h.create(json!({ "seed": 7 })).await.0, 400);
        assert_eq!(h.create(json!({ "seed": "" })).await.0, 400);
        assert!(h.rooms().await.is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn r143_drops_a_seed_whose_room_expired_so_an_unjoined_room_cannot_leak_one() {
        let _turn = ROOMS.lock().await;
        let h = harness(true).await;
        let (status, _) = h.create(json!({ "seed": "never-joined" })).await;
        assert_eq!(status, 200);
        assert_eq!(e2e_room_seed_count() as i64, 1);

        // Past the room's TTL, a second room is created: the stale entry goes with it.
        let ttl_ms = ROOM_CODE_TTL_SECONDS as u64 * 1000;
        tokio::time::advance(Duration::from_millis(ttl_ms + 1)).await;
        let (status, live) = h.create(json!({ "seed": "the-live-one" })).await;
        assert_eq!(status, 200, "{live}");

        assert_eq!(e2e_room_seed_count() as i64, 1);
        let code = live["code"].as_str().expect("a code").to_string();
        let (status, joined) = h.join(&code, json!({})).await;
        assert_eq!(status, 200, "{joined}");
        assert_eq!(
            started(&h.app).await.pop().expect("a started match")["seed"],
            "the-live-one"
        );
    }
}

mod r149_the_bounded_room_code_mint {
    //! R149 — the bounded room-code mint (§9.5, R110).
    use super::*;

    #[tokio::test]
    async fn r149_retries_past_a_code_that_is_already_in_use_and_mints_the_next_one() {
        let _turn = ROOMS.lock().await;
        let _codes = script_room_codes(&["AAA234", "AAA234", "AAA234", "BBB234"]);
        let h = harness(false).await;

        let (status, first) = h.create(json!({})).await;
        assert_eq!(status, 200, "{first}");
        assert_eq!(first["code"], "AAA234");

        let (status, second) = h.create_as_guest(json!({})).await;
        assert_eq!(status, 200, "{second}");
        assert_eq!(second["code"], "BBB234");
    }

    #[tokio::test]
    async fn r149_gives_up_after_a_bounded_number_of_collisions_and_says_no_code_is_available() {
        let _turn = ROOMS.lock().await;
        let log = record_logs();
        let _codes = script_room_codes(&["AAA234"]);
        let h = harness(false).await;
        let (status, first) = h.create(json!({})).await;
        assert_eq!(status, 200, "{first}");

        let (status, body) = h.create_as_guest(json!({})).await;

        assert_eq!(status, 503);
        assert_eq!(body["error"]["code"], "unavailable");
        assert!(
            body["error"]["message"]
                .as_str()
                .unwrap_or_default()
                .contains("could not allocate a room code"),
            "{body}"
        );
        // Bounded, and loud: an exhausted code space is an operator's problem, not a silent retry loop.
        let alerts = log.named("room.code.exhausted");
        assert_eq!(alerts.len(), 1, "{alerts:?}");
        assert_eq!(alerts[0].level, "alert");
        assert_eq!(h.rooms().await.len(), 1);
    }
}

// §9.4 / §9.5 / §9.8 — the host's deck is frozen into the room

/// Host deck freezing into room at creation (§9.4, §9.5, §9.8).
mod the_hosts_deck_is_frozen_into_the_room {
    use super::*;

    /// Three disjoint decks out of the catalog: A, B and C.
    fn decks_of() -> (Vec<String>, Vec<String>, Vec<String>) {
        (deck_at(0), deck_at(1), deck_at(2))
    }

    fn host_deck() -> String {
        uuid(11)
    }

    fn guest_deck() -> String {
        uuid(12)
    }

    struct Freeze {
        app: Arc<App>,
        host: String,
        guest: String,
        decks: (Vec<String>, Vec<String>, Vec<String>),
    }

    async fn freeze_harness() -> Freeze {
        let app = test_app().await;
        let host = player(&app, HOST, "user-host", "host@example.test").await;
        let guest = player(&app, GUEST, "user-guest", "guest@example.test").await;
        Freeze {
            app,
            host,
            guest,
            decks: decks_of(),
        }
    }

    async fn save(h: &Freeze, token: &str, deck_id: &str, cards: &[String]) -> u16 {
        call(
            &h.app,
            "PUT",
            &format!("/api/decks/{deck_id}"),
            Some(token),
            json!({ "name": "Deck", "cards": cards, "catalogVersion": jackioh_cards::catalog_version() }),
        )
        .await
        .0
    }

    /// The deck the started match gave this profile's seat.
    async fn deck_in_match_for(h: &Freeze, profile_id: &str) -> Option<Value> {
        let row = started(&h.app).await.pop()?;
        let seat = row["players"]
            .as_array()?
            .iter()
            .position(|player| player == profile_id)?;
        Some(row["decks"][seat].clone())
    }

    #[tokio::test]
    async fn a_deck_saved_between_the_create_and_the_join_does_not_change_the_hosts_deck_s9_8() {
        let _turn = ROOMS.lock().await;
        let h = freeze_harness().await;
        let (a, b, c) = h.decks.clone();

        // Host frozen and saved decks share no cards.
        assert!(!a.is_empty());
        assert!(a.iter().all(|card| !c.contains(card)));

        assert_eq!(save(&h, &h.host, &host_deck(), &a).await, 200);
        assert_eq!(save(&h, &h.guest, &guest_deck(), &b).await, 200);

        let (status, created) =
            create(&h.app, &h.host, json!({ "mode": "bo1", "deckId": host_deck() })).await;
        assert_eq!(status, 200, "{created}");
        let code = created["code"].as_str().expect("a code").to_string();
        let rooms = table(&h.app, |data| json!(data.tables.rooms)).await;
        assert_eq!(rooms.last().expect("the room")["hostDeck"], json!(a));

        assert_eq!(save(&h, &h.host, &host_deck(), &c).await, 200);
        // PREMISE: the save landed — otherwise there is nothing that could leak into the match.
        let saved =
            store!(h.app, t => t.decks_get(&host_deck()).await.expect("decks.get")).expect("the saved deck");
        assert_eq!(to_json(&saved)["cards"], json!(c));
        // …and the room is untouched by it.
        let rooms = table(&h.app, |data| json!(data.tables.rooms)).await;
        assert_eq!(rooms.last().expect("the room")["hostDeck"], json!(a));

        let (status, joined) = join(
            &h.app,
            &h.guest,
            &code,
            json!({ "mode": "bo1", "deckId": guest_deck() }),
        )
        .await;
        assert_eq!(status, 200, "{joined}");

        assert_eq!(deck_in_match_for(&h, HOST).await, Some(json!(a)));
        assert_eq!(deck_in_match_for(&h, GUEST).await, Some(json!(b)));
        let all_started = Value::Array(started(&h.app).await);
        assert!(!all_started.to_string().contains(c[0].as_str()));
    }

    #[tokio::test]
    async fn the_control_the_same_swap_made_before_the_create_is_the_deck_the_room_freezes() {
        // Verifies room does not ignore saved decks entirely.
        let _turn = ROOMS.lock().await;
        let h = freeze_harness().await;
        let (a, b, c) = h.decks.clone();

        assert_eq!(save(&h, &h.host, &host_deck(), &a).await, 200);
        assert_eq!(save(&h, &h.guest, &guest_deck(), &b).await, 200);
        assert_eq!(save(&h, &h.host, &host_deck(), &c).await, 200);

        let (_, created) = create(&h.app, &h.host, json!({ "mode": "bo1", "deckId": host_deck() })).await;
        let code = created["code"].as_str().expect("a code").to_string();
        let rooms = table(&h.app, |data| json!(data.tables.rooms)).await;
        assert_eq!(rooms.last().expect("the room")["hostDeck"], json!(c));

        let (status, _) = join(
            &h.app,
            &h.guest,
            &code,
            json!({ "mode": "bo1", "deckId": guest_deck() }),
        )
        .await;
        assert_eq!(status, 200);
        assert_eq!(deck_in_match_for(&h, HOST).await, Some(json!(c)));
    }
}

// R264 — rooms carry a mode

mod r264_rooms_carry_a_mode {
    //! R264 — rooms carry a mode (§9.5, R257).
    use super::*;

    /// Saves three decks and a trio for a profile, returning the trio's id. The three decks share no
    /// card (R253), as the real validator holds a trio to.
    async fn save_trio(app: &Arc<App>, profile_id: &str, base: u32) -> String {
        let ids = [uuid(base), uuid(base + 1), uuid(base + 2)];
        for (index, id) in ids.iter().enumerate() {
            save_deck(
                app,
                profile_id,
                id,
                &deck_at(index),
                &format!("{profile_id} {}", index + 1),
            )
            .await;
        }
        let trio_id = uuid(base + 3);
        let outcome = store!(app, t => t
            .trios_upsert(
                &from(json!({
                    "id": trio_id,
                    "profileId": profile_id,
                    "name": format!("{profile_id}'s trio"),
                    "deckIds": ids,
                    "createdAt": 0,
                    "updatedAt": 0,
                })),
                MAX_SAVED_TRIOS as _,
            )
            .await
            .expect("trios.upsert"));
        assert_eq!(to_json(&outcome), "created");
        trio_id
    }

    async fn create_in(h: &Harness, body: Value) -> String {
        let (status, answer) = create(&h.app, &h.host, body).await;
        assert_eq!(status, 200, "{answer}");
        answer["code"].as_str().expect("a code").to_string()
    }

    async fn join_with(h: &Harness, code: &str, body: Value) -> (u16, Value) {
        join(&h.app, &h.guest, code, body).await
    }

    #[tokio::test]
    async fn r264_answers_a_create_with_the_rooms_code_expiry_and_mode_and_freezes_the_choice() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let (_, body) = create(&h.app, &h.host, json!({ "mode": "bo1", "deckId": uuid(1) })).await;
        let room = h.rooms().await.remove(0);
        // `CreateRoomResponse` in apps/web's api.ts, exactly.
        assert_eq!(
            body,
            json!({ "code": room["code"], "expiresAt": room["expiresAt"], "mode": "bo1" })
        );
        assert_eq!(room["mode"], "bo1");
        assert_eq!(room["hostDeck"], json!(deck()));
        assert_eq!(room["hostTrio"], Value::Null);
    }

    #[tokio::test]
    async fn r264_refuses_a_join_in_another_mode_than_the_rooms_naming_the_rooms_mode() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let host_trio = save_trio(&h.app, HOST, 20).await;
        let code = create_in(&h, json!({ "mode": "bo3", "trioId": host_trio })).await;

        // A Best-of-1 joiner and an All Random one.
        for body in [
            json!({ "mode": "bo1", "deckId": uuid(2) }),
            json!({ "mode": "random" }),
        ] {
            let (status, refused) = join_with(&h, &code, body).await;
            assert_eq!(status, 409);
            assert_eq!(refused["error"]["code"], "conflict");
            assert_eq!(
                refused["error"]["message"],
                "This room plays Conquest: pick one of your trios."
            );
            assert_eq!(refused["error"]["details"], json!({ "mode": "bo3" }));
        }
        // Refused before anything was claimed: the room is still open to the right choice.
        assert_eq!(h.rooms().await[0]["guestProfileId"], Value::Null);

        let random = create_in(&h, json!({ "mode": "random" })).await;
        let (status, wrong) = join_with(&h, &random, json!({ "mode": "bo1", "deckId": uuid(2) })).await;
        assert_eq!(status, 409);
        assert_eq!(wrong["error"]["details"], json!({ "mode": "random" }));
    }

    #[tokio::test]
    async fn r264_makes_the_series_when_a_best_of_3_room_is_joined_with_the_coin_as_series_p1_and_no_match_yet()
     {
        let _turn = ROOMS.lock().await;
        let h = harness(true).await;
        let host_trio = save_trio(&h.app, HOST, 20).await;
        let guest_trio = save_trio(&h.app, GUEST, 30).await;
        let code = create_in(
            &h,
            json!({ "mode": "bo3", "trioId": host_trio, "seed": "room-series" }),
        )
        .await;
        let room = h.rooms().await.remove(0);
        assert_eq!(room["mode"], "bo3");
        assert_eq!(room["hostDeck"], json!([]));
        let names: Vec<Value> = room["hostTrio"]["decks"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|deck| deck["name"].clone())
            .collect();
        assert_eq!(names, vec![json!("host 1"), json!("host 2"), json!("host 3")]);

        let (status, answer) = join_with(&h, &code, json!({ "mode": "bo3", "trioId": guest_trio })).await;
        assert_eq!(status, 200, "{answer}");

        let series = table(&h.app, |data| json!(data.tables.series)).await.remove(0);
        // R1989000: the coin on the seed base seats the host and the joiner; the joiner learns
        // their actual seat.
        let host_is_p1 = seat_coin_p1_first("room-series");
        let joiner_seat = if host_is_p1 { "p2" } else { "p1" };
        assert_eq!(
            answer,
            json!({ "matchId": null, "seriesId": series["id"], "code": code, "seat": joiner_seat, "mode": "bo3" })
        );
        let sides = series["sides"].as_array().cloned().unwrap_or_default();
        let (first, second) = if host_is_p1 { (HOST, GUEST) } else { (GUEST, HOST) };
        assert_eq!(
            sides
                .iter()
                .map(|side| side["profileId"].clone())
                .collect::<Vec<_>>(),
            vec![json!(first), json!(second)]
        );
        if host_is_p1 {
            assert_eq!(sides[0]["trio"], room["hostTrio"]);
            assert_eq!(sides[1]["trio"]["name"], "guest's trio");
        } else {
            assert_eq!(sides[0]["trio"]["name"], "guest's trio");
            assert_eq!(sides[1]["trio"], room["hostTrio"]);
        }
        // R604: a room's series is never ranked either.
        assert_eq!(series["ranked"], json!(false));
        // R263: game 1's match id is the one the claim reserved; R143: the host's seed is the base.
        assert_eq!(series["nextMatchId"], h.rooms().await[0]["matchId"]);
        assert_eq!(series["seedBase"], "room-series");
        assert_eq!(e2e_room_seed_count() as i64, 0);
        // The series opens on its pick phase: no match, nobody in one.
        assert!(started(&h.app).await.is_empty());
        let profiles = table(&h.app, |data| json!(data.tables.profiles)).await;
        assert!(profiles.iter().all(|row| row["inMatchId"].is_null()));
    }

    #[tokio::test]
    async fn r264_deals_both_decks_when_an_all_random_room_is_joined_and_needs_no_saved_deck() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        // Neither player's saved deck is used: All Random asks for none (R258).
        fake(&h.app).lock().await.tables.decks.clear();
        let code = create_in(&h, json!({ "mode": "random" })).await;
        let room = h.rooms().await.remove(0);
        assert_eq!(room["mode"], "random");
        assert_eq!(room["hostDeck"], json!([]));
        assert_eq!(room["hostTrio"], Value::Null);

        let (status, answer) = join_with(&h, &code, json!({ "mode": "random" })).await;
        assert_eq!(status, 200, "{answer}");

        let row = started(&h.app).await.remove(0);
        let seed = row["seed"].as_str().unwrap_or_default().to_string();
        let match_id = row["id"].as_str().expect("a match id").to_string();
        // R1989000: the coin on the match seed seats the host and the joiner; the joiner learns
        // their actual seat. The deals stay seat-keyed below.
        let host_is_p1 = seat_coin_p1_first(&seed);
        let joiner_seat = if host_is_p1 { "p2" } else { "p1" };
        assert_eq!(
            answer,
            json!({ "matchId": match_id, "seriesId": null, "code": code, "seat": joiner_seat, "mode": "random" })
        );
        let (first, second) = if host_is_p1 { (HOST, GUEST) } else { (GUEST, HOST) };
        assert_eq!(row["players"], json!([first, second]));
        let mode = store!(h.app, t => t.matches_mode_of(&match_id).await.expect("matches.modeOf"));
        assert_eq!(to_json(&mode), "random");
        assert_eq!(
            row["decks"],
            json!([
                deal_random_deck(&format!("{seed}:p1-deck"), None),
                deal_random_deck(&format!("{seed}:p2-deck"), None)
            ])
        );
        assert_eq!(in_match_of(&h.app, HOST).await, json!(match_id));
        assert_eq!(in_match_of(&h.app, GUEST).await, json!(match_id));
    }

    /// R1372: the share of a dealt deck from the newest set, which a deck that leans on it holds at
    /// least `ceil(DECK_SIZE × leanMinShare)` of (R1370).
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

    fn lean_floor() -> usize {
        (f64::from(jackioh_engine::config::DECK_SIZE) * jackioh_ai::AI_DECK.lean_min_share).ceil() as usize
    }

    #[tokio::test]
    async fn r1372_an_all_random_room_leans_the_host_s_deck_alone_when_only_the_host_asked() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let code = create_in(&h, json!({ "mode": "random", "leanNewest": true })).await;
        // R924000: an older client's `leanNewest: true` is the pick "newest".
        assert_eq!(h.rooms().await[0]["hostLeanSet"], "newest");

        let (status, answer) = join_with(&h, &code, json!({ "mode": "random" })).await;
        assert_eq!(status, 200, "{answer}");
        let row = started(&h.app).await.remove(0);
        let seed = row["seed"].as_str().unwrap_or_default().to_string();
        let newest = Some(newest_shipped_set());
        // R1989000: the host's lean rides whichever seat the coin gave the host; the row stays
        // seat-ordered.
        let host_is_p1 = seat_coin_p1_first(&seed);
        let (host_stream, guest_stream) = if host_is_p1 {
            ("p1-deck", "p2-deck")
        } else {
            ("p2-deck", "p1-deck")
        };
        let host_deck = json!(deal_random_deck(&format!("{seed}:{host_stream}"), newest));
        let guest_deck = json!(deal_random_deck(&format!("{seed}:{guest_stream}"), None));
        let (p1_deck, p2_deck) = if host_is_p1 {
            (host_deck.clone(), guest_deck.clone())
        } else {
            (guest_deck.clone(), host_deck.clone())
        };
        assert_eq!(row["decks"], json!([p1_deck, p2_deck]));
        let host_seat = usize::from(!host_is_p1);
        assert!(
            newest_in(&row["decks"][host_seat]) >= lean_floor(),
            "{}",
            row["decks"][host_seat]
        );
    }

    #[tokio::test]
    async fn r1372_an_all_random_room_leans_the_joiner_s_deck_alone_when_only_the_joiner_asked() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let code = create_in(&h, json!({ "mode": "random" })).await;
        assert_eq!(h.rooms().await[0]["hostLeanSet"], Value::Null);

        let (status, answer) = join_with(&h, &code, json!({ "mode": "random", "leanNewest": true })).await;
        assert_eq!(status, 200, "{answer}");
        let row = started(&h.app).await.remove(0);
        let seed = row["seed"].as_str().unwrap_or_default().to_string();
        let newest = Some(newest_shipped_set());
        // R1989000: the joiner's lean rides whichever seat the coin gave the joiner; the row stays
        // seat-ordered.
        let guest_is_p1 = !seat_coin_p1_first(&seed);
        let (guest_stream, host_stream) = if guest_is_p1 {
            ("p1-deck", "p2-deck")
        } else {
            ("p2-deck", "p1-deck")
        };
        let guest_deck = json!(deal_random_deck(&format!("{seed}:{guest_stream}"), newest));
        let host_deck = json!(deal_random_deck(&format!("{seed}:{host_stream}"), None));
        let (p1_deck, p2_deck) = if guest_is_p1 {
            (guest_deck.clone(), host_deck.clone())
        } else {
            (host_deck.clone(), guest_deck.clone())
        };
        assert_eq!(row["decks"], json!([p1_deck, p2_deck]));
        let guest_seat = usize::from(!guest_is_p1);
        assert!(
            newest_in(&row["decks"][guest_seat]) >= lean_floor(),
            "{}",
            row["decks"][guest_seat]
        );
        // The lean is real: the same seed deals the joiner another deck without it.
        assert_ne!(
            row["decks"][guest_seat],
            json!(deal_random_deck(&format!("{seed}:{guest_stream}"), None))
        );
    }

    #[tokio::test]
    async fn r1372_a_lean_that_is_not_a_boolean_is_refused_and_one_that_is_absent_is_off() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let (status, refused) =
            create(&h.app, &h.host, json!({ "mode": "random", "leanNewest": "yes" })).await;
        assert_eq!(status, 400, "{refused}");
        assert!(h.rooms().await.is_empty());
        let (status, made) = create(&h.app, &h.host, json!({ "mode": "random", "leanNewest": null })).await;
        assert_eq!(status, 200, "{made}");
        assert_eq!(h.rooms().await[0]["hostLeanSet"], Value::Null);
    }

    #[tokio::test]
    async fn r924000_each_seat_of_a_room_leans_on_the_set_its_own_player_picked() {
        use jackioh_engine::SetName;
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let code = create_in(&h, json!({ "mode": "random", "leanSet": "Core" })).await;
        assert_eq!(h.rooms().await[0]["hostLeanSet"], "Core");

        let (status, answer) = join_with(&h, &code, json!({ "mode": "random", "leanSet": "Classic+" })).await;
        assert_eq!(status, 200, "{answer}");
        let row = started(&h.app).await.remove(0);
        let seed = row["seed"].as_str().unwrap_or_default().to_string();
        // R1989000: each seat's deal leans on its own player's set; the host's rides whichever seat
        // the coin gave the host, the row staying seat-ordered.
        let host_is_p1 = seat_coin_p1_first(&seed);
        let (host_stream, guest_stream) = if host_is_p1 {
            ("p1-deck", "p2-deck")
        } else {
            ("p2-deck", "p1-deck")
        };
        let host_deck = json!(deal_random_deck(
            &format!("{seed}:{host_stream}"),
            Some(SetName::Core)
        ));
        let guest_deck = json!(deal_random_deck(
            &format!("{seed}:{guest_stream}"),
            Some(SetName::ClassicPlus)
        ));
        let (p1_deck, p2_deck) = if host_is_p1 {
            (host_deck, guest_deck)
        } else {
            (guest_deck, host_deck)
        };
        assert_eq!(row["decks"], json!([p1_deck, p2_deck]));
    }

    #[tokio::test]
    async fn r924000_a_room_refuses_a_lean_on_a_set_that_does_not_ship() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let (status, refused) =
            create(&h.app, &h.host, json!({ "mode": "random", "leanSet": "Boss-X" })).await;
        assert_eq!(status, 400, "{refused}");
        assert!(h.rooms().await.is_empty());
    }

    #[tokio::test]
    async fn r264_keeps_a_profile_in_a_series_from_creating_or_joining_a_room() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let code = create_in(&h, json!({ "mode": "bo1", "deckId": uuid(1) })).await;
        let trio = json!({ "name": "t", "decks": [{ "name": "a", "cards": [] }, { "name": "b", "cards": [] }, { "name": "c", "cards": [] }] });
        let series = from(json!({
            "id": "series-1",
            "sides": [
                { "profileId": GUEST, "trio": trio, "wins": 0, "pick": null },
                { "profileId": "someone", "trio": trio, "wins": 0, "pick": null },
            ],
            "catalogVersion": jackioh_cards::catalog_version(),
            "ranked": false,
            "seedBase": "s",
            "status": "picking",
            "games": [],
            "nextMatchId": "reserved",
            "pickDeadline": null,
            "winner": null,
            "endReason": null,
            "ratingBefore": null,
            "ratingAfter": null,
            "createdAt": 0,
            "updatedAt": 0,
            "endedAt": null,
            "version": 1,
        }));
        store!(h.app, t => t.series_create(&series).await.expect("series.create"));
        // R1523000: the seat written straight into the store, read as a restart would read it.
        jackioh_server::seats::hydrate(&h.app)
            .await
            .expect("the ledger reads the store");

        let (status, body) = h.join(&code, json!({})).await;
        assert_eq!(status, 409);
        assert_eq!(body["error"]["code"], "already_in_match");
        assert_eq!(body["error"]["details"], json!({ "seriesId": "series-1" }));
        assert_eq!(h.rooms().await[0]["guestProfileId"], Value::Null);

        let (status, created) = h.create_as_guest(json!({})).await;
        assert_eq!(status, 409);
        assert_eq!(created["error"]["code"], "already_in_match");
        assert_eq!(h.rooms().await.len(), 1);
    }

    #[tokio::test]
    async fn r264_a_player_queued_before_joining_a_best_of_3_room_leaves_the_queue_so_a_series_that_ends_before_game_1_pairs_no_one_later()
     {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let third = player(&h.app, "third", "user-third", "third@example.test").await;
        save_deck(&h.app, "third", &uuid(90), &deck(), "third's deck").await;

        // The guest waits in the Best-of-1 queue, alone, and meanwhile takes a Best-of-3 challenge.
        let (status, queued) = post(
            &h.app,
            &h.guest,
            "/api/queue",
            Some(json!({ "mode": "bo1", "deckId": uuid(2) })),
        )
        .await;
        assert_eq!(status, 200, "{queued}");
        let host_trio = save_trio(&h.app, HOST, 20).await;
        let guest_trio = save_trio(&h.app, GUEST, 30).await;
        let code = create_in(&h, json!({ "mode": "bo3", "trioId": host_trio })).await;
        let (status, joined) = join_with(&h, &code, json!({ "mode": "bo3", "trioId": guest_trio })).await;
        assert_eq!(status, 200, "{joined}");
        let series_id = joined["seriesId"].as_str().expect("a series id").to_string();

        // The series ends before its first game: the guest forfeits at the pick.
        let (status, forfeited) = post(
            &h.app,
            &h.guest,
            &format!("/api/series/{series_id}/forfeit"),
            None,
        )
        .await;
        assert_eq!(status, 200, "{forfeited}");

        // Someone queues for Best of 1. The ticket the guest left behind must not become a match.
        let (status, other) = post(
            &h.app,
            &third,
            "/api/queue",
            Some(json!({ "mode": "bo1", "deckId": uuid(90) })),
        )
        .await;
        assert_eq!(status, 200, "{other}");
        assert_eq!(other["status"], "open");
        assert_eq!(other["matchId"], Value::Null);
        assert!(started(&h.app).await.is_empty());
        assert_eq!(in_match_of(&h.app, GUEST).await, Value::Null);
        assert!(
            store!(h.app, t => t.tickets_open_for_profile(GUEST).await.expect("tickets.openForProfile"))
                .is_none()
        );
    }

    #[tokio::test]
    async fn a_join_whose_host_has_gone_into_another_game_is_refused_and_the_room_stays_open() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let code = create_in(&h, json!({ "mode": "bo1", "deckId": uuid(1) })).await;
        store!(h.app, t => t.profiles_set_in_match(HOST, Some("elsewhere")).await.expect("profiles.setInMatch"));
        // R1523000: the seat written straight into the store, read as a restart would read it.
        jackioh_server::seats::hydrate(&h.app)
            .await
            .expect("the ledger reads the store");

        let (status, refused) = h.join(&code, json!({})).await;
        assert_eq!(status, 409);
        assert_eq!(refused["error"]["code"], "conflict");
        assert_eq!(h.rooms().await[0]["guestProfileId"], Value::Null);
        assert!(started(&h.app).await.is_empty());

        // The control: once the host is free the same join goes through.
        store!(h.app, t => t.profiles_set_in_match(HOST, None).await.expect("profiles.setInMatch"));
        jackioh_server::seats::hydrate(&h.app)
            .await
            .expect("the ledger reads the store");
        assert_eq!(h.join(&code, json!({})).await.0, 200);
    }
}

// R642 — the portraits on a room's match (§9.5)

/// R642: portrait assignment and freezing for rooms (SPEC §9.5, R258).
mod r642_the_portraits_on_a_rooms_match {
    use super::*;

    #[tokio::test]
    async fn r642_deals_an_all_random_rooms_portraits_from_the_seed_seat_by_seat_like_the_decks() {
        let _turn = ROOMS.lock().await;
        let h = harness(true).await;
        let (status, created) = h
            .create(json!({ "mode": "random", "seed": "room-portraits" }))
            .await;
        assert_eq!(status, 200, "{created}");
        let code = created["code"].as_str().expect("a code").to_string();

        let (status, joined) = h.join(&code, json!({ "mode": "random" })).await;
        assert_eq!(status, 200, "{joined}");

        let row = started(&h.app).await.remove(0);
        // PREMISE: the host's seed really is the match's (R143); R1989000's coin on it seats the
        // host and the joiner.
        assert_eq!(row["seed"], "room-portraits");
        let host_is_p1 = seat_coin_p1_first("room-portraits");
        let (first, second) = if host_is_p1 { (HOST, GUEST) } else { (GUEST, HOST) };
        assert_eq!(row["players"], json!([first, second]));
        // Each seat's pick uses seat-keyed suffix in seat order.
        let expected = json!([
            pick_portrait_from_seed("room-portraits:portrait:p1"),
            pick_portrait_from_seed("room-portraits:portrait:p2"),
        ]);
        assert_eq!(row["portraits"], expected);
    }

    #[tokio::test]
    async fn r642_freezes_both_seats_portraits_the_hosts_with_the_room_the_joiners_with_the_join() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let put_portrait = |token: String, deck_id: String, portrait: &'static str| {
            let app = Arc::clone(&h.app);
            async move {
                call(
                    &app,
                    "PUT",
                    &format!("/api/decks/{deck_id}"),
                    Some(token.as_str()),
                    json!({ "name": "Deck", "cards": deck(), "catalogVersion": jackioh_cards::catalog_version(), "portrait": portrait }),
                )
                .await
                .0
            }
        };

        // Host and guest save their decks under portraits they own (R1827000), through the endpoint
        // a player would use.
        own_portraits(&h.app, HOST, &[PortraitId::Gary, PortraitId::Timmy]).await;
        own_portraits(&h.app, GUEST, &[PortraitId::Shredder]).await;
        assert_eq!(put_portrait(h.host.clone(), uuid(1), "gary").await, 200);
        assert_eq!(put_portrait(h.guest.clone(), uuid(2), "shredder").await, 200);

        // The host's freezes into the room here — the same moment its deck does (§9.4).
        let (status, created) = h.create(json!({ "mode": "bo1", "deckId": uuid(1) })).await;
        assert_eq!(status, 200, "{created}");
        let code = created["code"].as_str().expect("a code").to_string();
        assert_eq!(h.rooms().await[0]["hostPortrait"], "gary");

        // The host re-saves under another portrait while the room waits for its guest: the room's copy
        // does not move (the §9.8 freeze, one field wider).
        assert_eq!(put_portrait(h.host.clone(), uuid(1), "timmy").await, 200);
        let saved =
            store!(h.app, t => t.decks_get(&uuid(1)).await.expect("decks.get")).expect("the saved deck");
        assert_eq!(to_json(&saved)["portrait"], "timmy");
        assert_eq!(h.rooms().await[0]["hostPortrait"], "gary");

        let (status, joined) = h.join(&code, json!({ "mode": "bo1", "deckId": uuid(2) })).await;
        assert_eq!(status, 200, "{joined}");

        // The host's seat carries the frozen one, not "timmy"; the guest's carries what the join
        // froze. Both land on the match row in seat order, R1989000's coin deciding whose seat is
        // whose.
        let row = started(&h.app).await.remove(0);
        let seed = row["seed"].as_str().unwrap_or_default().to_string();
        let (first_portrait, second_portrait) = if seat_coin_p1_first(&seed) {
            ("gary", "shredder")
        } else {
            ("shredder", "gary")
        };
        assert_eq!(row["portraits"], json!([first_portrait, second_portrait]));
    }

    #[tokio::test]
    async fn r642_reads_a_room_made_before_portraits_host_portrait_null_as_vanilla_on_the_row() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        // The harness's saved decks carry `portrait: null`: R641's default, and what every deck saved
        // before portraits existed holds.
        play_through(&h, json!({}), json!({})).await;

        assert_eq!(h.rooms().await[0]["hostPortrait"], Value::Null);
        assert_eq!(
            started(&h.app).await.remove(0)["portraits"],
            json!([DEFAULT_PORTRAIT, DEFAULT_PORTRAIT])
        );
    }
}

// R1834002 — the loadouts on a room's match

/// R1834002: Best-of-1 room freezes loadout slots on host and join (R642); match row stores both.
mod r1834002_the_loadouts_on_a_rooms_match {
    use super::*;

    fn defaults() -> Value {
        json!({
            "table": "table-tavern",
            "field": "field-none",
            "cardBack": "card-back-standard",
            "foil": "foil-none",
            "finish": "finish-plain",
            "border": "border-standard",
        })
    }

    #[tokio::test]
    async fn r1834002_a_rooms_match_freezes_both() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let put_deck = |token: String, deck_id: String, slots: Value| {
            let app = Arc::clone(&h.app);
            async move {
                let mut body = json!({ "name": "Deck", "cards": deck(), "catalogVersion": jackioh_cards::catalog_version() });
                if let (Some(body), Some(slots)) = (body.as_object_mut(), slots.as_object()) {
                    body.extend(slots.clone());
                }
                call(
                    &app,
                    "PUT",
                    &format!("/api/decks/{deck_id}"),
                    Some(token.as_str()),
                    body,
                )
                .await
                .0
            }
        };
        assert_eq!(
            put_deck(h.host.clone(), uuid(1), json!({ "tableCosmetic": "table-felt" })).await,
            200
        );
        assert_eq!(
            put_deck(
                h.guest.clone(),
                uuid(2),
                json!({ "fieldCosmetic": "field-embers" })
            )
            .await,
            200
        );
        // The guest's account picks a card back its deck leaves to it.
        let (status, _, body) = call(
            &h.app,
            "PUT",
            "/api/settings",
            Some(h.guest.as_str()),
            json!({ "groups": { "cosmetics": { "at": 1, "values": { "cardBack": "card-back-starfield", "field": "field-rain" } } } }),
        )
        .await;
        assert_eq!(status, 200, "{body}");

        let (status, created) = h.create(json!({ "mode": "bo1", "deckId": uuid(1) })).await;
        assert_eq!(status, 200, "{created}");
        let code = created["code"].as_str().expect("a code").to_string();
        assert_eq!(
            h.rooms().await[0]["hostDeckSlots"],
            json!({ "table": "table-felt" })
        );
        // A re-save while the room waits does not reach it.
        assert_eq!(
            put_deck(h.host.clone(), uuid(1), json!({ "tableCosmetic": "table-wood" })).await,
            200
        );
        assert_eq!(
            h.rooms().await[0]["hostDeckSlots"],
            json!({ "table": "table-felt" })
        );

        let (status, joined) = h.join(&code, json!({ "mode": "bo1", "deckId": uuid(2) })).await;
        assert_eq!(status, 200, "{joined}");
        let row = started(&h.app).await.remove(0);
        // R1989000: each seat's loadout travels with its profile; the coin decides whose seat is
        // whose.
        let seed = row["seed"].as_str().unwrap_or_default().to_string();
        let host_seat = usize::from(!seat_coin_p1_first(&seed));
        let mut host = defaults();
        host["table"] = json!("table-felt");
        let mut guest = defaults();
        guest["field"] = json!("field-embers");
        guest["cardBack"] = json!("card-back-starfield");
        assert_eq!(row["loadouts"][host_seat]["seat"], host);
        assert_eq!(row["loadouts"][1 - host_seat]["seat"], guest);
    }

    #[tokio::test]
    async fn r1834002_an_all_random_room_wears_the_accounts_picks() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let (status, _, body) = call(
            &h.app,
            "PUT",
            "/api/settings",
            Some(h.host.as_str()),
            json!({ "groups": { "cosmetics": { "at": 1, "values": { "table": "table-neon-grid" } } } }),
        )
        .await;
        assert_eq!(status, 200, "{body}");
        let (status, created) = h.create(json!({ "mode": "random" })).await;
        assert_eq!(status, 200, "{created}");
        let code = created["code"].as_str().expect("a code").to_string();
        let (status, joined) = h.join(&code, json!({ "mode": "random" })).await;
        assert_eq!(status, 200, "{joined}");
        let row = started(&h.app).await.remove(0);
        // R1989000: the host's loadout rides whichever seat the coin gave the host.
        let seed = row["seed"].as_str().unwrap_or_default().to_string();
        let host_seat = usize::from(!seat_coin_p1_first(&seed));
        let mut host = defaults();
        host["table"] = json!("table-neon-grid");
        assert_eq!(row["loadouts"][host_seat]["seat"], host);
        assert_eq!(row["loadouts"][1 - host_seat]["seat"], defaults());
    }
}

// R890000 — a match seat is claimed atomically (§9.5)

/// R890000, the room half: the join seats both seats compare-and-set before the start, and a
/// host holds at most three open rooms.
mod r890000_a_match_seat_is_claimed_atomically {
    use super::*;

    #[tokio::test]
    async fn r890000_join_loses_when_queue_seats_the_host_first() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let (status, created) = h.create(json!({})).await;
        assert_eq!(status, 200, "{created}");
        let code = created["code"].as_str().expect("a code").to_string();

        // Queue seats host before room claim; rival write committed.
        fake(&h.app).lock().await.before_call = Some(Arc::new(|method: &str, tables: &mut FakeTables| {
            if method == "rooms.claim"
                && let Some(row) = tables.profiles.iter_mut().find(|row| row.id == HOST)
                && row.in_match_id.is_none()
            {
                row.in_match_id = Some("r89-elsewhere".to_string());
            }
            CallOutcome::Proceed
        }));

        let (status, refused) = h.join(&code, json!({})).await;
        fake(&h.app).lock().await.before_call = None;
        assert_eq!(status, 409, "{refused}");
        assert_eq!(refused["error"]["code"], "conflict");
        // No match started, nobody was seated by the join, and the host keeps the game it found.
        assert!(started(&h.app).await.is_empty());
        assert_eq!(in_match_of(&h.app, HOST).await, json!("r89-elsewhere"));
        assert_eq!(in_match_of(&h.app, GUEST).await, Value::Null);

        // The claim was undone and the code freed, so nobody is locked: once the host is free
        // a new room plays through.
        assert!(h.rooms().await.is_empty());
        store!(h.app, t => t.profiles_set_in_match(HOST, None).await.expect("profiles.setInMatch"));
        let (status, created) = h.create(json!({})).await;
        assert_eq!(status, 200, "{created}");
        let code = created["code"].as_str().expect("a code").to_string();
        assert_eq!(h.join(&code, json!({})).await.0, 200);
    }

    #[tokio::test]
    async fn r890000_join_loses_when_queue_seats_the_guest_first() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let (status, created) = h.create(json!({})).await;
        assert_eq!(status, 200, "{created}");
        let code = created["code"].as_str().expect("a code").to_string();

        // The queue seats the joiner after the host's seat won: only the joiner loses. Hooked
        // on the claim, which commits, for the same reason as the host race above.
        fake(&h.app).lock().await.before_call = Some(Arc::new(|method: &str, tables: &mut FakeTables| {
            if method == "rooms.claim"
                && let Some(row) = tables.profiles.iter_mut().find(|row| row.id == GUEST)
                && row.in_match_id.is_none()
            {
                row.in_match_id = Some("r89-elsewhere".to_string());
            }
            CallOutcome::Proceed
        }));

        let (status, refused) = h.join(&code, json!({})).await;
        fake(&h.app).lock().await.before_call = None;
        assert_eq!(status, 409, "{refused}");
        assert_eq!(refused["error"]["code"], "already_in_match");
        assert!(started(&h.app).await.is_empty());
        // The host's won seat rolled back with the transaction; the guest keeps its game.
        assert_eq!(in_match_of(&h.app, HOST).await, Value::Null);
        assert_eq!(in_match_of(&h.app, GUEST).await, json!("r89-elsewhere"));
    }

    #[tokio::test]
    async fn r890000_create_refuses_a_fourth_open_room() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        for _ in 0..3 {
            let (status, created) = h.create(json!({})).await;
            assert_eq!(status, 200, "{created}");
        }
        let (status, refused) = h.create(json!({})).await;
        assert_eq!(status, 409, "{refused}");
        assert_eq!(refused["error"]["code"], "conflict");
        assert_eq!(h.rooms().await.len(), 3);
    }
}

mod r1075000_a_host_cancels_and_finds_their_room {
    //! R1075000 — `DELETE /api/rooms/:code` and `GET /api/rooms` (§9.5).
    use super::*;

    /// A room the host made, as its code.
    async fn host_room(h: &Harness) -> String {
        let (status, created) = h.create(json!({})).await;
        assert_eq!(status, 200, "{created}");
        created["code"].as_str().expect("a code").to_string()
    }

    fn codes(answer: &Value) -> Vec<String> {
        answer["rooms"]
            .as_array()
            .expect("a rooms list")
            .iter()
            .map(|room| room["code"].as_str().expect("a code").to_string())
            .collect()
    }

    #[tokio::test]
    async fn r1075000_the_host_cancels_an_unjoined_room_and_its_code_joins_as_unknown() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let code = host_room(&h).await;

        let (status, answer) = cancel(&h.app, &h.host, &code).await;

        assert_eq!(status, 200, "{answer}");
        assert_eq!(answer, json!({ "cancelled": true }));
        assert!(h.rooms().await.is_empty());
        let (status, refused) = h.join(&code, json!({})).await;
        assert_eq!(status, 404, "{refused}");
        assert_eq!(refused["error"]["code"], "not_found");
    }

    #[tokio::test]
    async fn r1075000_a_second_cancel_and_an_unknown_code_report_canceled_false() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let code = host_room(&h).await;
        assert_eq!(
            cancel(&h.app, &h.host, &code).await.1,
            json!({ "cancelled": true })
        );

        let (status, again) = cancel(&h.app, &h.host, &code).await;
        assert_eq!((status, again), (200, json!({ "cancelled": false })));
        let (status, unknown) = cancel(&h.app, &h.host, "ZZZ999").await;
        assert_eq!((status, unknown), (200, json!({ "cancelled": false })));
    }

    #[tokio::test]
    async fn r1075000_a_non_host_cancel_is_refused_and_the_room_stays() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let code = host_room(&h).await;

        let (status, refused) = cancel(&h.app, &h.guest, &code).await;

        assert_eq!(status, 404, "{refused}");
        assert_eq!(refused["error"]["code"], "not_found");
        assert_eq!(h.rooms().await.len(), 1);
        assert_eq!(h.join(&code, json!({})).await.0, 200);
    }

    #[tokio::test]
    async fn r1075000_a_joined_room_is_never_touched() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let (code, _) = play_through(&h, json!({}), json!({})).await;
        let match_id = started(&h.app).await.pop().expect("the join started a match")["id"].clone();

        let (status, answer) = cancel(&h.app, &h.host, &code).await;

        assert_eq!((status, answer), (200, json!({ "cancelled": false })));
        assert_eq!(started(&h.app).await.len(), 1);
        assert_eq!(in_match_of(&h.app, HOST).await, match_id);
        let rooms = h.rooms().await;
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0]["guestProfileId"], json!(GUEST));
    }

    #[tokio::test]
    async fn r1075000_a_cancel_frees_a_slot_of_the_open_room_cap() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let first = host_room(&h).await;
        host_room(&h).await;
        host_room(&h).await;
        assert_eq!(h.create(json!({})).await.0, 409);

        assert_eq!(
            cancel(&h.app, &h.host, &first).await.1,
            json!({ "cancelled": true })
        );

        let (status, created) = h.create(json!({})).await;
        assert_eq!(status, 200, "{created}");
    }

    #[tokio::test]
    async fn r1075000_drops_the_end_to_end_seed_with_the_room() {
        let _turn = ROOMS.lock().await;
        let h = harness(true).await;
        let (status, created) = h.create(json!({ "seed": "cancelled-room" })).await;
        assert_eq!(status, 200, "{created}");
        let code = created["code"].as_str().expect("a code").to_string();
        assert_eq!(e2e_room_seed_count() as i64, 1);

        assert_eq!(
            cancel(&h.app, &h.host, &code).await.1,
            json!({ "cancelled": true })
        );

        assert_eq!(e2e_room_seed_count() as i64, 0);
    }

    #[tokio::test(start_paused = true)]
    async fn r1075000_lists_the_callers_open_rooms_newest_first_and_nobody_elses() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let (_, first) = h.create(json!({})).await;
        let older = first["code"].as_str().expect("a code").to_string();
        tokio::time::advance(Duration::from_secs(1)).await;
        let (_, second) = h.create(json!({})).await;
        let newer = second["code"].as_str().expect("a code").to_string();
        let (_, third) = h.create_as_guest(json!({})).await;
        let guests = third["code"].as_str().expect("a code").to_string();

        let (status, listed) = open_rooms(&h.app, &h.host).await;

        assert_eq!(status, 200, "{listed}");
        assert_eq!(
            listed["rooms"][0],
            json!({ "code": newer, "expiresAt": second["expiresAt"], "mode": "bo1" })
        );
        assert_eq!(codes(&listed), vec![newer.clone(), older.clone()]);
        assert_eq!(codes(&open_rooms(&h.app, &h.guest).await.1), vec![guests]);

        // A joined room is the match's, not a lobby's.
        assert_eq!(h.join(&older, json!({})).await.0, 200);
        assert_eq!(codes(&open_rooms(&h.app, &h.host).await.1), vec![newer]);

        tokio::time::advance(Duration::from_secs(ROOM_CODE_TTL_SECONDS as u64 + 1)).await;
        assert!(codes(&open_rooms(&h.app, &h.host).await.1).is_empty());
        assert!(codes(&open_rooms(&h.app, &h.guest).await.1).is_empty());
    }

    #[tokio::test]
    async fn r1075000_a_challenges_room_is_neither_listed_nor_canceled_here() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let code = host_room(&h).await;
        let challenge: ChallengeRow = from(json!({
            "id": uuid(9),
            "fromProfile": HOST,
            "toProfile": GUEST,
            "roomCode": code,
            "mode": "bo1",
            "createdAt": now_ms(),
            "expiresAt": now_ms() + 60_000,
            "state": "open",
        }));
        store!(h.app, t => t.challenges_create(&challenge, now_ms()).await.expect("challenges.create"));

        assert!(codes(&open_rooms(&h.app, &h.host).await.1).is_empty());
        let (status, refused) = cancel(&h.app, &h.host, &code).await;
        assert_eq!(status, 409, "{refused}");
        assert_eq!(refused["error"]["code"], "conflict");
        let (status, refused) = cancel(&h.app, &h.guest, &code).await;
        assert_eq!(status, 404, "{refused}");
        assert_eq!(h.rooms().await.len(), 1);
    }
}

mod a_deck_with_a_custom_card_r915005 {
    //! R915005 — a deck with a custom card plays only in practice: a room refuses it at both doors
    //! with the validator's L7, before anything is claimed.
    use super::*;

    /// The harness's deck with its first card swapped for a custom card: legal by every other rule.
    fn custom_deck() -> Vec<String> {
        let mut cards = deck();
        cards[0] = "custom-0123456789abcdef0123456789abcdef".to_string();
        cards
    }

    fn assert_l7(status: u16, refused: &Value) {
        assert_eq!(status, 422, "{refused}");
        assert_eq!(refused["error"]["code"], "loadout_invalid");
        let details = refused["error"]["details"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        assert_eq!(details.len(), 1, "{refused}");
        assert_eq!(details[0]["rule"], "L7");
        assert_eq!(details[0]["cardId"], json!(custom_deck()[0]));
        assert_eq!(refused["error"]["message"], details[0]["message"]);
    }

    #[tokio::test]
    async fn r915005_refuses_a_room_create_and_a_join_with_a_custom_card_deck_l7_and_claims_nothing() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        save_deck(&h.app, HOST, &uuid(9), &custom_deck(), "host's homebrew").await;
        save_deck(&h.app, GUEST, &uuid(10), &custom_deck(), "guest's homebrew").await;

        // The host's door: no room is made.
        let (status, refused) = create(&h.app, &h.host, json!({ "mode": "bo1", "deckId": uuid(9) })).await;
        assert_l7(status, &refused);
        assert!(h.rooms().await.is_empty());

        // The guest's door: the room stays open to a deck that may play, and no match starts.
        let (status, made) = h.create(json!({})).await;
        assert_eq!(status, 200, "{made}");
        let code = made["code"].as_str().expect("a code").to_string();
        let (status, refused) = join(
            &h.app,
            &h.guest,
            &code,
            json!({ "mode": "bo1", "deckId": uuid(10) }),
        )
        .await;
        assert_l7(status, &refused);
        assert_eq!(h.rooms().await[0]["guestProfileId"], Value::Null);
        assert!(started(&h.app).await.is_empty());

        // Control: the guest's own legal deck joins the same room.
        let (status, joined) = h.join(&code, json!({})).await;
        assert_eq!(status, 200, "{joined}");
    }
}

// R1989000 — the match seed's coin seats the host and the joiner

/// R1989000: room seed coin seats both orders deterministically; seat pin skips coin.
mod r1989000_the_seed_coin_seats_the_room {
    use super::*;

    /// A fixed set of seeds. The coin test below asserts it deals both seat orders, so the set is
    /// pinned without pinning the answer: it fails if the stream ever stops varying.
    const SEEDS: [&str; 8] = [
        "room-coin-alpha",
        "room-coin-bravo",
        "room-coin-charlie",
        "room-coin-delta",
        "room-coin-echo",
        "room-coin-foxtrot",
        "room-coin-golf",
        "room-coin-hotel",
    ];

    /// A room the host makes with `create_body`, joined by the guest with `join_body`: the join's
    /// answer with the match row it started.
    async fn played(h: &Harness, create_body: Value, join_body: Value) -> (Value, Value) {
        let (status, created) = h.create(create_body).await;
        assert_eq!(status, 200, "{created}");
        let code = created["code"].as_str().expect("a code").to_string();
        let (status, joined) = h.join(&code, join_body).await;
        assert_eq!(status, 200, "{joined}");
        let row = started(&h.app).await.pop().expect("the join started a match");
        (joined, row)
    }

    #[tokio::test]
    async fn r1989000_a_fixed_seed_set_seats_both_orders_and_the_same_seed_seats_the_same_way() {
        let _turn = ROOMS.lock().await;
        let mut seen_host_first = false;
        let mut seen_guest_first = false;
        for seed in SEEDS {
            // A fresh harness per seed: a match seats its profiles, and they stay seated.
            let h = harness(true).await;
            let (joined, row) = played(&h, json!({ "seed": seed }), json!({})).await;
            assert_eq!(row["seed"], seed);
            let host_is_p1 = seat_coin_p1_first(seed);
            let joiner_seat = if host_is_p1 { "p2" } else { "p1" };
            assert_eq!(joined["seat"], joiner_seat, "seed {seed}");
            let (first, second) = if host_is_p1 { (HOST, GUEST) } else { (GUEST, HOST) };
            assert_eq!(row["players"], json!([first, second]), "seed {seed}");
            seen_host_first |= host_is_p1;
            seen_guest_first |= !host_is_p1;
            // The same seed run again seats the same way.
            let h = harness(true).await;
            let (again_joined, again) = played(&h, json!({ "seed": seed }), json!({})).await;
            assert_eq!(again["players"], json!([first, second]), "seed {seed} again");
            assert_eq!(again_joined["seat"], joiner_seat, "seed {seed} again");
        }
        assert!(
            seen_host_first && seen_guest_first,
            "the fixed set must deal both seat orders"
        );
        assert_eq!(e2e_room_seed_count() as i64, 0);
    }

    #[tokio::test]
    async fn r1989000_a_host_pin_skips_the_coin_with_the_host_winning_a_conflict() {
        let _turn = ROOMS.lock().await;
        // Pin against coin proves pin skipped it; host takes pinned seat.
        for seed in SEEDS {
            let h = harness(true).await;
            let host_pin = if seat_coin_p1_first(seed) { "p2" } else { "p1" };
            let joiner_pin = if host_pin == "p1" { "p2" } else { "p1" };
            let (joined, row) = played(
                &h,
                json!({ "seed": seed, "seat": host_pin }),
                json!({ "seat": joiner_pin }),
            )
            .await;
            let (first, second, joiner_seat) = if host_pin == "p1" {
                (HOST, GUEST, "p2")
            } else {
                (GUEST, HOST, "p1")
            };
            assert_eq!(row["players"], json!([first, second]), "seed {seed}");
            assert_eq!(joined["seat"], joiner_seat, "seed {seed}");
        }
        assert_eq!(e2e_room_seed_count() as i64, 0);
    }

    #[tokio::test]
    async fn r1989000_a_joiner_only_pin_seats_the_joiner_and_the_host_takes_the_free_seat() {
        let _turn = ROOMS.lock().await;
        let h = harness(true).await;
        let (joined, row) = played(&h, json!({}), json!({ "seat": "p1" })).await;
        assert_eq!(row["players"], json!([GUEST, HOST]));
        assert_eq!(joined["seat"], "p1");
        assert_eq!(e2e_room_seed_count() as i64, 0);
    }

    #[tokio::test]
    async fn r1989000_a_host_seat_only_pin_keeps_the_joiners_seed() {
        let _turn = ROOMS.lock().await;
        let h = harness(true).await;
        let (joined, row) = played(&h, json!({ "seat": "p1" }), json!({ "seed": "from-the-joiner" })).await;
        assert_eq!(row["seed"], "from-the-joiner");
        assert_eq!(row["players"], json!([HOST, GUEST]));
        assert_eq!(joined["seat"], "p2");
        assert_eq!(e2e_room_seed_count() as i64, 0);
    }

    #[tokio::test]
    async fn r1989000_seat_is_refused_outside_end_to_end_mode_on_both_doors() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let (status, refused) = h.create(json!({ "seat": "p1" })).await;
        assert_eq!(status, 400, "{refused}");
        assert!(
            refused["error"]["message"]
                .as_str()
                .unwrap_or_default()
                .contains("seat"),
            "{refused}"
        );
        assert!(h.rooms().await.is_empty());

        let code = {
            let (status, created) = h.create(json!({})).await;
            assert_eq!(status, 200, "{created}");
            created["code"].as_str().expect("a code").to_string()
        };
        let (status, refused) = h.join(&code, json!({ "seat": "p1" })).await;
        assert_eq!(status, 400, "{refused}");
        assert!(
            refused["error"]["message"]
                .as_str()
                .unwrap_or_default()
                .contains("seat"),
            "{refused}"
        );
        assert!(started(&h.app).await.is_empty());
        assert_eq!(e2e_room_seed_count() as i64, 0);
    }
}
