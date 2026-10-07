//! The room-code challenge (`actor/rooms.rs`, SPEC §9.5, R79, BUILD M6-T4).
//!
//! Three rulings own this file:
//!
//!  - **R143**, the optional seed. A room's match is not created until someone joins, so the seed
//!    the host posts to `POST /api/rooms` has to survive until `POST /api/rooms/:code/join` — and
//!    outside end-to-end mode the field is refused at both doors. BUILD M8 requires every spec to
//!    set a seed, and `e2e/cypress/e2e/05-reconnect.cy.ts` and `06-room-code.cy.ts` both post one.
//!  - **R149**, the bounded mint: a code is retried a fixed number of times against the codes still
//!    in use, and then the caller is told none is available rather than the server retrying for ever.
//!  - **R264**, the room's mode: a room is made in the host's mode with their deck or trio frozen
//!    into it, a join in another mode is refused with the room's mode named, a Best-of-3 join makes
//!    the series and an All Random join deals both decks.
//!
//! Port of `apps/server/test/match/rooms.test.ts`. The rooms run the real `freeze_choice`
//! (`api/decks.rs`) over decks saved straight into the store. Where TS leaned on test doubles the
//! Rust server does not have (SURFACE §11.3), this file uses the real thing instead: the real
//! validator (so every deck is a legal one of real cards and both players own every card, where TS
//! had a permissive validator and three ids), the real registry (so "the match the join started" is
//! the match row it wrote, read off the store, where TS read its fake directory's `started` list),
//! and a minted seed or code that is the server's own (TS scripted both through `Ids`). The legacy
//! `{ deckIndex }` body is gone (SURFACE §11.3), so a Best-of-1 choice names its deck by id.
//! R149's two tests scripted the code mint to collide, which nothing can do to the server's own
//! random codes; they are listed under GAPS in `.fullsend/notes/part-19-6.md`.

use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};

use jackioh_engine::wire::{pick_portrait_from_seed, DEFAULT_PORTRAIT};
use jackioh_server::actor::engine::deal_random_deck;
use jackioh_server::actor::rooms::e2e_room_seed_count;
use jackioh_server::app::App;
use jackioh_server::config::{MAX_SAVED_DECKS, MAX_SAVED_TRIOS, ROOM_CODE_LENGTH, ROOM_CODE_TTL_SECONDS};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{CollectionEntry, Db};

use crate::support::deps::{add_user, call, test_app, test_app_with, TestAppOptions};

const HOST: &str = "host";
const GUEST: &str = "guest";

/// The epoch-ms stamp the saved decks carry; nothing reads it back.
const AT: i64 = 1_700_000_000_000;

/// R143's seeds sit in one process-wide map (`e2e_room_seed_count`), which TS's suite read one test
/// at a time; these tests take turns so one test's seed is never another's count.
static ROOMS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// One store call in its own transaction, as TS's `deps.store.<sub>.<method>(…)` was.
macro_rules! store {
    ($app:expr, $t:ident => $call:expr) => {{
        let mut $t = $app.db.begin(None).await.expect("the fake store opens a transaction");
        let out = $call;
        $t.commit().await.expect("the fake store commits");
        out
    }};
}

fn from<T: serde::de::DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value.clone()).unwrap_or_else(|error| panic!("{error}: {value}"))
}

fn to_json<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("a serialisable value")
}

/// The fake store behind the test app (SURFACE §11.2's `Db::Fake`).
fn fake(app: &App) -> Arc<tokio::sync::Mutex<FakeData>> {
    match &app.db {
        Db::Fake(data) => Arc::clone(data),
        _ => panic!("the test app runs on the fake store"),
    }
}

/// One table of the fake store, as TS's `deps.store.tables.<name>` read it: rows as JSON.
async fn table(app: &App, pick: impl Fn(&FakeData) -> Value) -> Vec<Value> {
    let data = fake(app);
    let data = data.lock().await;
    match pick(&data) {
        Value::Array(rows) => rows,
        other => panic!("not a table: {other}"),
    }
}

/// TS's `deps.matches.started`: the matches the registry started, oldest first — every match row
/// past its `open` reservation (a room's claim reserves the row; the join's start completes it).
/// Seat order is the row's: index 0 of `players` and `decks` is p1.
async fn started(app: &App) -> Vec<Value> {
    table(app, |data| json!(data.tables.matches)).await.into_iter().filter(|row| row["status"] != "open").collect()
}

async fn in_match_of(app: &Arc<App>, profile: &str) -> Value {
    let row = store!(app, t => t.profiles_get_by_id(profile).await.expect("profiles.getById")).expect("the profile");
    to_json(&row)["inMatchId"].clone()
}

/// A client-minted id (R256).
fn uuid(n: u32) -> String {
    format!("00000000-0000-4000-8000-{n:012}")
}

/// Every playable catalog id, in `catalog.json` order.
fn playable() -> Vec<String> {
    jackioh_cards::register_all();
    jackioh_cards::CATALOG
        .iter()
        .filter(|(_, def)| !def.token && !to_json(&def.tags).as_array().is_some_and(|tags| tags.contains(&json!("Token"))))
        .map(|(id, _)| id.clone())
        .collect()
}

fn deck_size() -> usize {
    usize::try_from(jackioh_engine::config::DECK_SIZE).expect("a deck size")
}

/// The `n`th run of `DECK_SIZE` playable ids: legal, and disjoint from every other `n`.
fn deck_at(n: usize) -> Vec<String> {
    let size = deck_size();
    playable()[n * size..(n + 1) * size].to_vec()
}

/// TS's `DECK` (three ids the permissive validator let through): one legal deck.
fn deck() -> Vec<String> {
    deck_at(0)
}

/// Saves a deck for a profile straight into the store.
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

/// Every playable card in the profile's collection, so the real validator's L5 passes (TS's
/// permissive validator checked no ownership).
async fn own_everything(app: &Arc<App>, profile_id: &str) {
    let rows: Vec<Value> = playable().iter().map(|id| json!({ "cardId": id, "quantity": 1 })).collect();
    let entries: Vec<CollectionEntry> = from(Value::Array(rows));
    store!(app, t => t.collection_upsert_quantities(profile_id, &entries).await.expect("collection.upsertQuantities"));
}

/// Seeds an active profile for a fresh user and answers its token.
async fn player(app: &Arc<App>, profile_id: &str, user_id: &str, email: &str) -> String {
    let token = add_user(app, user_id, email, true);
    fake(app).lock().await.seed_profile(json!({ "id": profile_id, "userId": user_id, "status": "active" }));
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
    // TS `createTestDeps({ e2e })`: the empty store either way, end-to-end mode only when asked.
    let app = test_app_with(TestAppOptions { e2e, skip_fixtures: true, ..TestAppOptions::default() }).await;
    let host = player(&app, HOST, "user-host", "host@example.test").await;
    let guest = player(&app, GUEST, "user-guest", "guest@example.test").await;
    save_deck(&app, HOST, &uuid(1), &deck(), "host's deck").await;
    save_deck(&app, GUEST, &uuid(2), &deck(), "guest's deck").await;
    Harness { app, host, guest }
}

/// TS's `{ deckIndex: 0, ...body }`: the caller's saved deck, Best of 1, under whatever `body` sets.
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

/// Creates a room and joins it, returning the code and the seed of the match the join started.
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
    async fn creates_a_room_and_starts_the_match_on_the_join_with_the_host_as_p1() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let (code, _) = play_through(&h, json!({}), json!({})).await;

        assert_eq!(code.chars().count(), ROOM_CODE_LENGTH);
        let row = started(&h.app).await.remove(0);
        // The frozen decks, host first: the row's seat order is p1, p2.
        assert_eq!(row["decks"], json!([deck(), deck()]));
        assert_eq!(row["players"], json!([HOST, GUEST]));
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
        assert!(body["error"]["message"].as_str().unwrap_or_default().contains("seed"), "{body}");
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
        assert!(body["error"]["message"].as_str().unwrap_or_default().contains("seed"), "{body}");
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
        assert_eq!(play_through(&joiner_only, json!({}), json!({ "seed": "from-the-joiner" })).await.1, "from-the-joiner");

        // Both: the room was created first, so its seed is the one the match runs on.
        let both = harness(true).await;
        assert_eq!(
            play_through(&both, json!({ "seed": "from-the-host" }), json!({ "seed": "from-the-joiner" })).await.1,
            "from-the-host"
        );
        assert_eq!(e2e_room_seed_count() as i64, 0);
    }

    #[tokio::test]
    async fn r143_still_mints_a_seed_when_none_is_supplied_s9_3_the_server_owns_it() {
        let _turn = ROOMS.lock().await;
        let h = harness(true).await;
        // TS's fake `Ids` minted `seed-<n>`; the server's own seed is opaque, and present.
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
        assert_eq!(started(&h.app).await.pop().expect("a started match")["seed"], "the-live-one");
    }
}

// ---------------------------------------------------------------------------
// §9.4 / §9.5 / §9.8 — the host's deck is frozen into the room
// ---------------------------------------------------------------------------

/// The room half of §9.8's "Deck swapped after matchmaking → decks are frozen into the ticket". A
/// room has the same exposure as a queue ticket and a wider window for it: `actor/rooms.rs`
/// freezes the host's deck at `POST /api/rooms` and the match is not created until somebody joins —
/// which may be up to `ROOM_CODE_TTL_SECONDS` later, with the deck builder open the whole time.
///
/// These put the real `PUT /api/decks/:id` on the same router, so "the host edits the deck" is the
/// endpoint a player would use and the freeze under test is the production one.
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
        Freeze { app, host, guest, decks: decks_of() }
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
        let seat = row["players"].as_array()?.iter().position(|player| player == profile_id)?;
        Some(row["decks"][seat].clone())
    }

    #[tokio::test]
    async fn a_deck_saved_between_the_create_and_the_join_does_not_change_the_hosts_deck_s9_8() {
        let _turn = ROOMS.lock().await;
        let h = freeze_harness().await;
        let (a, b, c) = h.decks.clone();

        // PREMISE: the deck the host freezes and the one they swap to share no card, so "the match used
        // the frozen deck" and "the match used the saved deck" cannot both be true.
        assert!(!a.is_empty());
        assert!(a.iter().all(|card| !c.contains(card)));

        assert_eq!(save(&h, &h.host, &host_deck(), &a).await, 200);
        assert_eq!(save(&h, &h.guest, &guest_deck(), &b).await, 200);

        // 1. The host opens a room on the deck. The freeze happens here.
        let (status, created) = create(&h.app, &h.host, json!({ "mode": "bo1", "deckId": host_deck() })).await;
        assert_eq!(status, 200, "{created}");
        let code = created["code"].as_str().expect("a code").to_string();
        let rooms = table(&h.app, |data| json!(data.tables.rooms)).await;
        assert_eq!(rooms.last().expect("the room")["hostDeck"], json!(a));

        // 2. The swap, while the room sits open waiting for somebody to type the code.
        assert_eq!(save(&h, &h.host, &host_deck(), &c).await, 200);
        // PREMISE: the save landed — otherwise there is nothing that could leak into the match.
        let saved = store!(h.app, t => t.decks_get(&host_deck()).await.expect("decks.get")).expect("the saved deck");
        assert_eq!(to_json(&saved)["cards"], json!(c));
        // …and the room is untouched by it.
        let rooms = table(&h.app, |data| json!(data.tables.rooms)).await;
        assert_eq!(rooms.last().expect("the room")["hostDeck"], json!(a));

        // 3. The guest joins on their own deck, so each seat is identifiable.
        let (status, joined) = join(&h.app, &h.guest, &code, json!({ "mode": "bo1", "deckId": guest_deck() })).await;
        assert_eq!(status, 200, "{joined}");

        assert_eq!(deck_in_match_for(&h, HOST).await, Some(json!(a)));
        assert_eq!(deck_in_match_for(&h, GUEST).await, Some(json!(b)));
        // The substitute deck reached no seat at all.
        let all_started = Value::Array(started(&h.app).await);
        assert!(!all_started.to_string().contains(c[0].as_str()));
    }

    #[tokio::test]
    async fn the_control_the_same_swap_made_before_the_create_is_the_deck_the_room_freezes() {
        // Without this, the test above would pass against a room that ignored saved decks entirely.
        // Exactly one thing moves between the two: whether the save happens before or after the create.
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

        let (status, _) = join(&h.app, &h.guest, &code, json!({ "mode": "bo1", "deckId": guest_deck() })).await;
        assert_eq!(status, 200);
        assert_eq!(deck_in_match_for(&h, HOST).await, Some(json!(c)));
    }
}

// ---------------------------------------------------------------------------
// R264 — rooms carry a mode
// ---------------------------------------------------------------------------

mod r264_rooms_carry_a_mode {
    //! R264 — rooms carry a mode (§9.5, R257).
    use super::*;

    /// Saves three decks and a trio for a profile, returning the trio's id. The three decks share no
    /// card (R253), as the real validator holds a trio to.
    async fn save_trio(app: &Arc<App>, profile_id: &str, base: u32) -> String {
        let ids = [uuid(base), uuid(base + 1), uuid(base + 2)];
        for (index, id) in ids.iter().enumerate() {
            save_deck(app, profile_id, id, &deck_at(index), &format!("{profile_id} {}", index + 1)).await;
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
        assert_eq!(body, json!({ "code": room["code"], "expiresAt": room["expiresAt"], "mode": "bo1" }));
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

        // A Best-of-1 joiner and an All Random one (TS also sent a legacy body with no mode, which
        // SURFACE §11.3 retires).
        for body in [json!({ "mode": "bo1", "deckId": uuid(2) }), json!({ "mode": "random" })] {
            let (status, refused) = join_with(&h, &code, body).await;
            assert_eq!(status, 409);
            assert_eq!(refused["error"]["code"], "conflict");
            assert_eq!(refused["error"]["message"], "This room plays Conquest: pick one of your trios.");
            assert_eq!(refused["error"]["details"], json!({ "mode": "bo3" }));
        }
        // Refused before anything was claimed: the room is still open to the right choice.
        assert_eq!(h.rooms().await[0]["guestProfileId"], Value::Null);

        // An All Random room names its own mode the same way.
        let random = create_in(&h, json!({ "mode": "random" })).await;
        let (status, wrong) = join_with(&h, &random, json!({ "mode": "bo1", "deckId": uuid(2) })).await;
        assert_eq!(status, 409);
        assert_eq!(wrong["error"]["details"], json!({ "mode": "random" }));
    }

    #[tokio::test]
    async fn r264_makes_the_series_when_a_best_of_3_room_is_joined_the_host_as_series_p1_and_no_match_yet() {
        let _turn = ROOMS.lock().await;
        let h = harness(true).await;
        let host_trio = save_trio(&h.app, HOST, 20).await;
        let guest_trio = save_trio(&h.app, GUEST, 30).await;
        let code = create_in(&h, json!({ "mode": "bo3", "trioId": host_trio, "seed": "room-series" })).await;
        let room = h.rooms().await.remove(0);
        assert_eq!(room["mode"], "bo3");
        assert_eq!(room["hostDeck"], json!([]));
        let names: Vec<Value> =
            room["hostTrio"]["decks"].as_array().into_iter().flatten().map(|deck| deck["name"].clone()).collect();
        assert_eq!(names, vec![json!("host 1"), json!("host 2"), json!("host 3")]);

        let (status, answer) = join_with(&h, &code, json!({ "mode": "bo3", "trioId": guest_trio })).await;
        assert_eq!(status, 200, "{answer}");

        let series = table(&h.app, |data| json!(data.tables.series)).await.remove(0);
        assert_eq!(answer, json!({ "matchId": null, "seriesId": series["id"], "code": code, "seat": "p2", "mode": "bo3" }));
        let sides = series["sides"].as_array().cloned().unwrap_or_default();
        assert_eq!(sides.iter().map(|side| side["profileId"].clone()).collect::<Vec<_>>(), vec![json!(HOST), json!(GUEST)]);
        assert_eq!(sides[0]["trio"], room["hostTrio"]);
        assert_eq!(sides[1]["trio"]["name"], "guest's trio");
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
        assert_eq!(answer, json!({ "matchId": match_id, "seriesId": null, "code": code, "seat": "p2", "mode": "random" }));
        assert_eq!(row["players"], json!([HOST, GUEST]));
        let mode = store!(h.app, t => t.matches_mode_of(&match_id).await.expect("matches.modeOf"));
        assert_eq!(to_json(&mode), "random");
        assert_eq!(
            row["decks"],
            json!([deal_random_deck(&format!("{seed}:p1-deck")), deal_random_deck(&format!("{seed}:p2-deck"))])
        );
        assert_eq!(in_match_of(&h.app, HOST).await, json!(match_id));
        assert_eq!(in_match_of(&h.app, GUEST).await, json!(match_id));
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
    async fn r264_a_player_queued_before_joining_a_best_of_3_room_leaves_the_queue_so_a_series_that_ends_before_game_1_pairs_no_one_later() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        // TS put the room, queue and series routes on one router; the app's router serves them all.
        let third = player(&h.app, "third", "user-third", "third@example.test").await;
        save_deck(&h.app, "third", &uuid(90), &deck(), "third's deck").await;

        // The guest waits in the Best-of-1 queue, alone, and meanwhile takes a Best-of-3 challenge.
        let (status, queued) = post(&h.app, &h.guest, "/api/queue", Some(json!({ "mode": "bo1", "deckId": uuid(2) }))).await;
        assert_eq!(status, 200, "{queued}");
        let host_trio = save_trio(&h.app, HOST, 20).await;
        let guest_trio = save_trio(&h.app, GUEST, 30).await;
        let code = create_in(&h, json!({ "mode": "bo3", "trioId": host_trio })).await;
        let (status, joined) = join_with(&h, &code, json!({ "mode": "bo3", "trioId": guest_trio })).await;
        assert_eq!(status, 200, "{joined}");
        let series_id = joined["seriesId"].as_str().expect("a series id").to_string();

        // The series ends before its first game: the guest forfeits at the pick.
        let (status, forfeited) = post(&h.app, &h.guest, &format!("/api/series/{series_id}/forfeit"), None).await;
        assert_eq!(status, 200, "{forfeited}");

        // Someone queues for Best of 1. The ticket the guest left behind must not become a match.
        let (status, other) = post(&h.app, &third, "/api/queue", Some(json!({ "mode": "bo1", "deckId": uuid(90) }))).await;
        assert_eq!(status, 200, "{other}");
        assert_eq!(other["status"], "open");
        assert_eq!(other["matchId"], Value::Null);
        assert!(started(&h.app).await.is_empty());
        assert_eq!(in_match_of(&h.app, GUEST).await, Value::Null);
        assert!(store!(h.app, t => t.tickets_open_for_profile(GUEST).await.expect("tickets.openForProfile")).is_none());
    }

    #[tokio::test]
    async fn a_join_whose_host_has_gone_into_another_game_is_refused_and_the_room_stays_open() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        let code = create_in(&h, json!({ "mode": "bo1", "deckId": uuid(1) })).await;
        store!(h.app, t => t.profiles_set_in_match(HOST, Some("elsewhere")).await.expect("profiles.setInMatch"));

        let (status, refused) = h.join(&code, json!({})).await;
        assert_eq!(status, 409);
        assert_eq!(refused["error"]["code"], "conflict");
        assert_eq!(h.rooms().await[0]["guestProfileId"], Value::Null);
        assert!(started(&h.app).await.is_empty());

        // The control: once the host is free the same join goes through.
        store!(h.app, t => t.profiles_set_in_match(HOST, None).await.expect("profiles.setInMatch"));
        assert_eq!(h.join(&code, json!({})).await.0, 200);
    }
}

// ---------------------------------------------------------------------------
// R642 — the portraits on a room's match (§9.5)
// ---------------------------------------------------------------------------

/// R642, the room half: a Best-of-1 room freezes the host deck's portrait at `POST /api/rooms`,
/// with the deck, and an All Random room deals each seat's portrait off the match seed — the same
/// `pick_portrait_from_seed` the queue uses (R258's dealing, one layer down).
///
/// The match row these tests read is the real write: the registry's `start` writes it, seat order
/// and `portrait_or_default` included (TS wrote it through its fake directory for these tests).
mod r642_the_portraits_on_a_rooms_match {
    use super::*;

    #[tokio::test]
    async fn r642_deals_an_all_random_rooms_portraits_from_the_seed_seat_by_seat_like_the_decks() {
        let _turn = ROOMS.lock().await;
        let h = harness(true).await;
        let (status, created) = h.create(json!({ "mode": "random", "seed": "room-portraits" })).await;
        assert_eq!(status, 200, "{created}");
        let code = created["code"].as_str().expect("a code").to_string();

        let (status, joined) = h.join(&code, json!({ "mode": "random" })).await;
        assert_eq!(status, 200, "{joined}");

        let row = started(&h.app).await.remove(0);
        // PREMISE: the host's seed really is the match's (R143), the host p1 as ever (§9.5).
        assert_eq!(row["seed"], "room-portraits");
        assert_eq!(row["players"], json!([HOST, GUEST]));
        // Each seat's pick is `pick_portrait_from_seed` on its own seat-keyed suffix — both
        // `:portrait:p1` and `:portrait:p2` — and the row the actor will read keeps them in that
        // seat order.
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

        // Host and guest save their decks under portraits, through the endpoint a player would use.
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
        let saved = store!(h.app, t => t.decks_get(&uuid(1)).await.expect("decks.get")).expect("the saved deck");
        assert_eq!(to_json(&saved)["portrait"], "timmy");
        assert_eq!(h.rooms().await[0]["hostPortrait"], "gary");

        let (status, joined) = h.join(&code, json!({ "mode": "bo1", "deckId": uuid(2) })).await;
        assert_eq!(status, 200, "{joined}");

        // The host's seat carries the frozen one, not "timmy"; the guest's carries what the join
        // froze. Both land on the match row in seat order.
        assert_eq!(started(&h.app).await.remove(0)["portraits"], json!(["gary", "shredder"]));
    }

    #[tokio::test]
    async fn r642_reads_a_room_made_before_portraits_host_portrait_null_as_vanilla_on_the_row() {
        let _turn = ROOMS.lock().await;
        let h = harness(false).await;
        // The harness's saved decks carry `portrait: null`: R641's default, and what every deck saved
        // before portraits existed holds.
        play_through(&h, json!({}), json!({})).await;

        assert_eq!(h.rooms().await[0]["hostPortrait"], Value::Null);
        assert_eq!(started(&h.app).await.remove(0)["portraits"], json!([DEFAULT_PORTRAIT, DEFAULT_PORTRAIT]));
    }
}
