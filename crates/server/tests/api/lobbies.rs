//! Private lobbies over HTTP (SPEC §9.5, §13.9; issue #3730; R3730000 to R3730006): a lobby's seats,
//! picks, Ready, settings, invites and the host's Start, 1v1 and co-op, and the classic rooms an old
//! link still opens; and a 2v2 between people (issue #3731; R3731000, R3731001, R3731003).

use std::sync::Arc;

use serde_json::{Value, json};

use jackioh_server::app::{App, now_ms};
use jackioh_server::config::{MAX_OPEN_ROOMS_PER_HOST, MAX_SAVED_DECKS, ROOM_CODE_TTL_SECONDS};
use jackioh_server::db::store::CollectionEntry;
use jackioh_server::seats::Seat;

use crate::support::deps::{call, empty_test_app};
use crate::support::engine::playable;
use crate::support::store::{Account, account, advance, befriend, from, store, uuid};

/// The epoch-ms stamp a saved deck carries; nothing reads it back.
const AT: i64 = 1_700_000_000_000;

async fn api(app: &Arc<App>, method: &str, path: &str, token: &str, body: Value) -> (u16, Value) {
    let (status, _headers, body) = call(app, method, path, Some(token), body).await;
    (status, body)
}

/// Whether the refusal names `reason` (`details.reason`) or is the error code `reason`.
fn refused_for(body: &Value, reason: &str) -> bool {
    body["error"]["details"]["reason"] == reason || body["error"]["code"] == reason
}

/// Host, guest and a third account, the catalog registered.
struct Three {
    app: Arc<App>,
    host: Account,
    guest: Account,
    third: Account,
}

async fn three() -> Three {
    jackioh_cards::register_all();
    let app = empty_test_app().await;
    let host = account(&app, 1, "Ana").await;
    let guest = account(&app, 2, "Bo").await;
    let third = account(&app, 3, "Cy").await;
    Three {
        app,
        host,
        guest,
        third,
    }
}

/// Every playable card in the profile's collection, and one saved deck `deck_id` named `name`.
async fn with_saved_deck(app: &Arc<App>, who: &Account, deck_id: &str, name: &str) {
    let rows: Vec<Value> = playable()
        .iter()
        .map(|id| json!({ "cardId": id, "quantity": 1 }))
        .collect();
    let entries: Vec<CollectionEntry> = from(Value::Array(rows));
    store!(app, t => t.collection_upsert_quantities(&who.id, &entries).await.expect("collection.upsertQuantities"));
    let size = usize::try_from(jackioh_engine::config::DECK_SIZE).expect("a deck size");
    let cards = playable()[..size].to_vec();
    let outcome = store!(app, t => t
        .decks_upsert(
            &from(json!({
                "id": deck_id,
                "profileId": who.id,
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
    assert_eq!(serde_json::to_value(outcome).expect("an outcome"), "created");
}

async fn create(app: &Arc<App>, host: &Account, body: Value) -> String {
    let (status, body) = api(app, "POST", "/api/lobbies", &host.token, body).await;
    assert_eq!(status, 201, "{body}");
    body["code"].as_str().expect("a lobby code").to_string()
}

async fn duel(app: &Arc<App>, host: &Account, mode: &str) -> String {
    create(app, host, json!({ "format": "duel", "mode": mode })).await
}

async fn coop(app: &Arc<App>, host: &Account) -> String {
    create(app, host, json!({ "format": "coop", "difficulty": "easy" })).await
}

/// `POST /api/lobbies/:code/<verb>` as `who`.
async fn act(app: &Arc<App>, who: &Account, code: &str, verb: &str, body: Value) -> (u16, Value) {
    api(
        app,
        "POST",
        &format!("/api/lobbies/{code}/{verb}"),
        &who.token,
        body,
    )
    .await
}

async fn ok_act(app: &Arc<App>, who: &Account, code: &str, verb: &str, body: Value) {
    let (status, answer) = act(app, who, code, verb, body).await;
    assert!(status == 200 || status == 204, "{verb}: {status} {answer}");
}

async fn read(app: &Arc<App>, who: &Account, code: &str) -> (u16, Value) {
    api(
        app,
        "GET",
        &format!("/api/lobbies/{code}"),
        &who.token,
        Value::Null,
    )
    .await
}

async fn view(app: &Arc<App>, who: &Account, code: &str) -> Value {
    let (status, body) = read(app, who, code).await;
    assert_eq!(status, 200, "{body}");
    body
}

/// Each seat's deck state, host first.
fn states(view: &Value) -> Vec<String> {
    view["seats"]
        .as_array()
        .expect("the seats")
        .iter()
        .map(|seat| seat["state"].as_str().expect("a state").to_string())
        .collect()
}

/// An All Random 1v1 lobby the guest is in, both dealt at random and ready.
async fn ready_random_duel(t: &Three) -> String {
    let code = duel(&t.app, &t.host, "random").await;
    ok_act(&t.app, &t.guest, &code, "enter", Value::Null).await;
    for who in [&t.host, &t.guest] {
        ok_act(&t.app, who, &code, "pick", json!({ "mode": "random" })).await;
        ok_act(&t.app, who, &code, "ready", json!({ "ready": true })).await;
    }
    code
}

#[tokio::test]
async fn r3730002_two_members_ready_up_and_the_host_starts_a_best_of_1_game() {
    let t = three().await;
    let (host_deck, guest_deck) = (uuid(10), uuid(11));
    with_saved_deck(&t.app, &t.host, &host_deck, "Pyre Control").await;
    with_saved_deck(&t.app, &t.guest, &guest_deck, "Tide Tempo").await;
    let code = duel(&t.app, &t.host, "bo1").await;
    let lobby = view(&t.app, &t.host, &code).await;
    assert_eq!(lobby["kind"], "lobby");
    assert_eq!(lobby["you"], "host");
    assert_eq!(lobby["settings"], json!({ "format": "duel", "mode": "bo1" }));
    assert!(lobby["seats"][1]["player"].is_null(), "{lobby}");

    ok_act(&t.app, &t.guest, &code, "enter", Value::Null).await;
    ok_act(
        &t.app,
        &t.host,
        &code,
        "pick",
        json!({ "mode": "bo1", "deckId": host_deck }),
    )
    .await;
    ok_act(
        &t.app,
        &t.guest,
        &code,
        "pick",
        json!({ "mode": "bo1", "deckId": guest_deck }),
    )
    .await;
    let seen = view(&t.app, &t.guest, &code).await;
    assert_eq!(seen["you"], "guest");
    assert_eq!(seen["seats"][0]["player"]["username"], t.host.name);
    assert_eq!(seen["seats"][1]["player"]["username"], t.guest.name);
    assert_eq!(states(&seen), ["chosen", "chosen"]);
    // Rule 7: a 1v1 opponent's deck is shown by its state alone; one's own by its name.
    assert!(seen["seats"][0]["deck"].is_null(), "{seen}");
    assert_eq!(seen["seats"][1]["deck"], "Tide Tempo");
    assert_eq!(seen["yourPick"], json!({ "mode": "bo1", "deckId": guest_deck }));
    assert!(
        !seen.to_string().contains(&playable()[0]),
        "no card leaves: {seen}"
    );

    for who in [&t.host, &t.guest] {
        ok_act(&t.app, who, &code, "ready", json!({ "ready": true })).await;
    }
    let (status, started) = act(&t.app, &t.host, &code, "start", Value::Null).await;
    assert_eq!(status, 200, "{started}");
    let match_id = started["matchId"].as_str().expect("a match id").to_string();
    assert!(started["seriesId"].is_null(), "{started}");
    for who in [&t.host, &t.guest] {
        assert_eq!(t.app.seats.of(&who.id), Some(Seat::Match(match_id.clone())));
    }
    let seen = view(&t.app, &t.guest, &code).await;
    assert_eq!(seen["matchId"], match_id.as_str());
    assert!(seen["startedAt"].is_i64(), "{seen}");
    // One Start wins: a second is refused, and nothing changes a started lobby.
    let (status, body) = act(&t.app, &t.host, &code, "start", Value::Null).await;
    assert_eq!(status, 409, "{body}");
    assert!(refused_for(&body, "started"), "{body}");
    let (status, body) = act(&t.app, &t.guest, &code, "ready", json!({ "ready": false })).await;
    assert!(status == 409 && refused_for(&body, "started"), "{status} {body}");
    // R3730004: nor is it left, by the host or the guest, once its game is recorded: the lobby is
    // what takes the other member to the board, so it stays until its TTL.
    for who in [&t.host, &t.guest] {
        let (status, body) = act(&t.app, who, &code, "leave", Value::Null).await;
        assert!(status == 409 && refused_for(&body, "started"), "{status} {body}");
    }
    let seen = view(&t.app, &t.guest, &code).await;
    assert_eq!(seen["you"], "guest", "{seen}");
    assert_eq!(seen["matchId"], match_id.as_str(), "{seen}");
}

#[tokio::test]
async fn r3730002_a_co_op_lobby_starts_with_the_host_as_a1() {
    let t = three().await;
    let code = coop(&t.app, &t.host).await;
    ok_act(&t.app, &t.guest, &code, "enter", Value::Null).await;
    for who in [&t.host, &t.guest] {
        ok_act(&t.app, who, &code, "pick", json!({ "random": true })).await;
        ok_act(&t.app, who, &code, "ready", json!({ "ready": true })).await;
    }
    // A teammate's deck is no secret, so a co-op seat shows its pick to both.
    let seen = view(&t.app, &t.guest, &code).await;
    assert_eq!(seen["seats"][0]["random"], true, "{seen}");
    let (status, started) = act(&t.app, &t.host, &code, "start", Value::Null).await;
    assert_eq!(status, 200, "{started}");
    let match_id = started["matchId"].as_str().expect("a match id").to_string();
    let row = store!(t.app, tx => tx.duos_matches_get(&match_id).await.expect("duos.matches.get"))
        .expect("the co-op match");
    assert_eq!(
        (row.a1.as_str(), row.a2.as_str()),
        (t.host.id.as_str(), t.guest.id.as_str())
    );
    assert_eq!(
        serde_json::to_value(row.difficulty).expect("a difficulty"),
        "easy"
    );
    assert_eq!(view(&t.app, &t.guest, &code).await["matchId"], match_id.as_str());
}

#[tokio::test]
async fn r3730002_start_is_refused_while_a_seat_is_empty_or_unready() {
    let t = three().await;
    let code = duel(&t.app, &t.host, "random").await;
    ok_act(&t.app, &t.host, &code, "pick", json!({ "mode": "random" })).await;
    ok_act(&t.app, &t.host, &code, "ready", json!({ "ready": true })).await;
    let not_ready = |(status, body): (u16, Value)| {
        assert_eq!(status, 409, "{body}");
        assert!(refused_for(&body, "not_ready"), "{body}");
    };
    // The guest's seat is empty.
    not_ready(act(&t.app, &t.host, &code, "start", Value::Null).await);
    // Filled, with no pick.
    ok_act(&t.app, &t.guest, &code, "enter", Value::Null).await;
    not_ready(act(&t.app, &t.host, &code, "start", Value::Null).await);
    // Picked, not ready.
    ok_act(&t.app, &t.guest, &code, "pick", json!({ "mode": "random" })).await;
    not_ready(act(&t.app, &t.host, &code, "start", Value::Null).await);
    // The guest ready and the host not.
    ok_act(&t.app, &t.guest, &code, "ready", json!({ "ready": true })).await;
    ok_act(&t.app, &t.host, &code, "ready", json!({ "ready": false })).await;
    not_ready(act(&t.app, &t.host, &code, "start", Value::Null).await);
    // Only the host starts.
    ok_act(&t.app, &t.host, &code, "ready", json!({ "ready": true })).await;
    let (status, body) = act(&t.app, &t.guest, &code, "start", Value::Null).await;
    assert!(status == 409 && refused_for(&body, "not_host"), "{status} {body}");
    assert!(view(&t.app, &t.host, &code).await["startedAt"].is_null());
    assert_eq!(t.app.seats.of(&t.host.id), None);
}

#[tokio::test]
async fn r3730001_ready_needs_a_legal_pick_and_a_new_pick_takes_ready_off() {
    let t = three().await;
    let host_deck = uuid(10);
    with_saved_deck(&t.app, &t.host, &host_deck, "Pyre Control").await;
    let code = duel(&t.app, &t.host, "bo1").await;
    let (status, body) = act(&t.app, &t.host, &code, "ready", json!({ "ready": true })).await;
    assert!(status == 409 && refused_for(&body, "no_pick"), "{status} {body}");
    // R264: a pick in another mode names the lobby's.
    let (status, body) = act(&t.app, &t.host, &code, "pick", json!({ "mode": "random" })).await;
    assert_eq!(status, 409, "{body}");
    assert_eq!(body["error"]["details"]["mode"], "bo1", "{body}");
    // R253: a deck that is not the caller's is the deck builder's refusal, and no pick is kept.
    let (status, body) = act(
        &t.app,
        &t.host,
        &code,
        "pick",
        json!({ "mode": "bo1", "deckId": uuid(99) }),
    )
    .await;
    assert_eq!(status, 422, "{body}");
    assert_eq!(states(&view(&t.app, &t.host, &code).await)[0], "choosing");

    ok_act(
        &t.app,
        &t.host,
        &code,
        "pick",
        json!({ "mode": "bo1", "deckId": host_deck }),
    )
    .await;
    ok_act(&t.app, &t.host, &code, "ready", json!({ "ready": true })).await;
    assert_eq!(states(&view(&t.app, &t.host, &code).await)[0], "ready");
    ok_act(
        &t.app,
        &t.host,
        &code,
        "pick",
        json!({ "mode": "bo1", "deckId": host_deck }),
    )
    .await;
    let seen = view(&t.app, &t.host, &code).await;
    assert_eq!(states(&seen)[0], "chosen", "a new pick takes Ready off: {seen}");
    assert_eq!(seen["seats"][0]["deck"], "Pyre Control");
}

#[tokio::test]
async fn r3730003_a_settings_change_takes_every_ready_off() {
    let t = three().await;
    let code = ready_random_duel(&t).await;
    assert_eq!(states(&view(&t.app, &t.host, &code).await), ["ready", "ready"]);
    // Only the host changes the settings, and the format never changes.
    let (status, body) = act(&t.app, &t.guest, &code, "settings", json!({ "mode": "bo1" })).await;
    assert!(status == 409 && refused_for(&body, "not_host"), "{status} {body}");
    let (status, body) = act(
        &t.app,
        &t.host,
        &code,
        "settings",
        json!({ "difficulty": "hard" }),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    // The same mode again changes nothing.
    ok_act(&t.app, &t.host, &code, "settings", json!({ "mode": "random" })).await;
    assert_eq!(states(&view(&t.app, &t.host, &code).await), ["ready", "ready"]);
    // A new mode takes every Ready off, and the picks with it: no pick of one mode plays in another.
    ok_act(&t.app, &t.host, &code, "settings", json!({ "mode": "bo1" })).await;
    let seen = view(&t.app, &t.guest, &code).await;
    assert_eq!(seen["settings"]["mode"], "bo1");
    assert_eq!(states(&seen), ["choosing", "choosing"]);
    assert!(seen["yourPick"].is_null(), "{seen}");

    // Co-op: a new difficulty takes every Ready off and keeps the picks.
    let code = coop(&t.app, &t.host).await;
    ok_act(&t.app, &t.third, &code, "enter", Value::Null).await;
    for who in [&t.host, &t.third] {
        ok_act(&t.app, who, &code, "pick", json!({ "random": true })).await;
        ok_act(&t.app, who, &code, "ready", json!({ "ready": true })).await;
    }
    ok_act(
        &t.app,
        &t.host,
        &code,
        "settings",
        json!({ "difficulty": "hard" }),
    )
    .await;
    let seen = view(&t.app, &t.third, &code).await;
    assert_eq!(
        seen["settings"],
        json!({ "format": "coop", "difficulty": "hard" })
    );
    assert_eq!(states(&seen), ["chosen", "chosen"]);
}

#[tokio::test(start_paused = true)]
async fn r3730004_the_host_leaving_closes_the_lobby() {
    let t = three().await;
    let code = ready_random_duel(&t).await;
    // A guest leaving frees the seat; the host's lobby stays.
    let (status, body) = act(&t.app, &t.guest, &code, "leave", Value::Null).await;
    assert_eq!(status, 204, "{body}");
    let seen = view(&t.app, &t.host, &code).await;
    assert!(seen["seats"][1]["player"].is_null(), "{seen}");
    // The host removes a guest, who may enter again (R3730004).
    ok_act(&t.app, &t.guest, &code, "enter", Value::Null).await;
    let (status, body) = act(
        &t.app,
        &t.guest,
        &code,
        "remove",
        json!({ "profileId": t.host.id }),
    )
    .await;
    assert!(status == 409 && refused_for(&body, "not_host"), "{status} {body}");
    ok_act(
        &t.app,
        &t.host,
        &code,
        "remove",
        json!({ "profileId": t.guest.id }),
    )
    .await;
    let seen = view(&t.app, &t.guest, &code).await;
    assert!(
        seen["you"].is_null(),
        "removed, the guest reads the lobby as a visitor: {seen}"
    );
    ok_act(&t.app, &t.guest, &code, "enter", Value::Null).await;
    let (_, mine) = api(&t.app, "GET", "/api/lobbies", &t.guest.token, Value::Null).await;
    assert_eq!(mine["lobbies"][0]["code"], code.as_str(), "{mine}");
    assert_eq!(mine["lobbies"][0]["you"], "guest", "{mine}");

    // The host leaving closes it for everyone (R110, R1075000).
    let (status, body) = act(&t.app, &t.host, &code, "leave", Value::Null).await;
    assert_eq!(status, 204, "{body}");
    for who in [&t.host, &t.guest] {
        assert_eq!(read(&t.app, who, &code).await.0, 404);
        let (_, mine) = api(&t.app, "GET", "/api/lobbies", &who.token, Value::Null).await;
        assert_eq!(mine["lobbies"], json!([]), "{mine}");
    }

    // An idle lobby closes after its TTL, from its last change.
    let code = duel(&t.app, &t.host, "random").await;
    advance(ROOM_CODE_TTL_SECONDS * 1000 / 2).await;
    ok_act(&t.app, &t.host, &code, "pick", json!({ "mode": "random" })).await;
    advance(ROOM_CODE_TTL_SECONDS * 1000 / 2 + 1).await;
    assert_eq!(
        read(&t.app, &t.host, &code).await.0,
        200,
        "the pick ran the TTL again"
    );
    advance(ROOM_CODE_TTL_SECONDS * 1000 / 2).await;
    assert_eq!(read(&t.app, &t.host, &code).await.0, 404);
}

#[tokio::test]
async fn r3730000_a_blocked_player_can_neither_read_nor_enter() {
    let t = three().await;
    let code = duel(&t.app, &t.host, "random").await;
    let (_, unknown) = read(&t.app, &t.third, "ZZZZZZ").await;
    for (blocker, blocked) in [(&t.host, &t.third), (&t.third, &t.host)] {
        store!(t.app, tx => tx.blocks_add(&blocker.id, &blocked.id, now_ms()).await.expect("blocks.add"));
        let (status, body) = read(&t.app, &t.third, &code).await;
        assert_eq!(
            (status, &body),
            (404, &unknown),
            "a block reads as an unknown code"
        );
        let (status, body) = act(&t.app, &t.third, &code, "enter", Value::Null).await;
        assert_eq!(status, 404, "{body}");
        store!(t.app, tx => tx.blocks_remove(&blocker.id, &blocked.id).await.expect("blocks.remove"));
    }
    // A seated guest a block now separates from the host is dropped from the seat.
    ok_act(&t.app, &t.guest, &code, "enter", Value::Null).await;
    store!(t.app, tx => tx.blocks_add(&t.guest.id, &t.host.id, now_ms()).await.expect("blocks.add"));
    let seen = view(&t.app, &t.host, &code).await;
    assert!(seen["seats"][1]["player"].is_null(), "{seen}");
    assert_eq!(read(&t.app, &t.guest, &code).await.0, 404);
    // Anyone else still may enter.
    ok_act(&t.app, &t.third, &code, "enter", Value::Null).await;
    // Full, a lobby is as gone as an unknown code to anyone not in it, with no block between them.
    store!(t.app, tx => tx.blocks_remove(&t.guest.id, &t.host.id).await.expect("blocks.remove"));
    let (status, body) = read(&t.app, &t.guest, &code).await;
    assert_eq!(
        (status, &body),
        (404, &unknown),
        "a full lobby reads as an unknown code"
    );
    assert_eq!(view(&t.app, &t.third, &code).await["you"], "guest");
}

#[tokio::test]
async fn r890000_r3730004_the_room_cap_holds_per_format() {
    let t = three().await;
    let cap = usize::try_from(MAX_OPEN_ROOMS_PER_HOST).expect("a cap");
    assert!(cap > 0);
    for _ in 0..cap {
        duel(&t.app, &t.host, "random").await;
    }
    let (status, body) = api(
        &t.app,
        "POST",
        "/api/lobbies",
        &t.host.token,
        json!({ "format": "duel", "mode": "bo1" }),
    )
    .await;
    assert_eq!(status, 409, "{body}");
    // The cap counts each format apart, and apart from rooms.
    for _ in 0..cap {
        coop(&t.app, &t.host).await;
    }
    let (status, body) = api(
        &t.app,
        "POST",
        "/api/lobbies",
        &t.host.token,
        json!({ "format": "coop", "difficulty": "easy" }),
    )
    .await;
    assert_eq!(status, 409, "{body}");
    let (status, body) = api(
        &t.app,
        "POST",
        "/api/rooms",
        &t.host.token,
        json!({ "mode": "random" }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    // A lobby closed frees its place.
    let (_, mine) = api(&t.app, "GET", "/api/lobbies", &t.host.token, Value::Null).await;
    let duel_code = mine["lobbies"]
        .as_array()
        .and_then(|lobbies| lobbies.iter().find(|lobby| lobby["settings"]["format"] == "duel"))
        .and_then(|lobby| lobby["code"].as_str())
        .expect("a duel lobby")
        .to_string();
    ok_act(&t.app, &t.host, &duel_code, "leave", Value::Null).await;
    duel(&t.app, &t.host, "bo1").await;
}

#[tokio::test]
async fn r3730000_a_member_holds_no_seat_and_start_re_checks_both() {
    let t = three().await;
    let code = ready_random_duel(&t).await;
    // R1523000: a lobby holds no seat; its members may play elsewhere until the Start.
    assert_eq!(t.app.seats.of(&t.host.id), None);
    assert_eq!(t.app.seats.of(&t.guest.id), None);
    let elsewhere = Seat::Match("match-elsewhere".to_string());
    t.app
        .seats
        .reserve(&[&t.guest.id], elsewhere.clone())
        .expect("the guest's seat is free")
        .commit();
    let (status, body) = act(&t.app, &t.host, &code, "start", Value::Null).await;
    assert_eq!(status, 409, "{body}");
    assert!(refused_for(&body, "busy"), "{body}");
    let seen = view(&t.app, &t.host, &code).await;
    assert!(
        seen["startedAt"].is_null(),
        "a refused Start is taken back: {seen}"
    );
    assert_eq!(states(&seen), ["ready", "ready"]);
    assert_eq!(t.app.seats.of(&t.host.id), None);

    t.app.seats.release(&t.guest.id, &elsewhere);
    let (status, body) = act(&t.app, &t.host, &code, "start", Value::Null).await;
    assert_eq!(status, 200, "{body}");
}

#[tokio::test]
async fn r3730006_an_old_play_room_code_still_joins() {
    let t = three().await;
    let (status, room) = api(
        &t.app,
        "POST",
        "/api/rooms",
        &t.host.token,
        json!({ "mode": "random" }),
    )
    .await;
    assert_eq!(status, 200, "{room}");
    let code = room["code"].as_str().expect("a room code").to_string();
    let seen = view(&t.app, &t.guest, &code).await;
    assert_eq!(seen["kind"], "room", "{seen}");
    assert_eq!(seen["mode"], "random");
    assert_eq!(seen["host"]["username"], t.host.name);
    let (status, body) = api(
        &t.app,
        "POST",
        &format!("/api/rooms/{code}/join"),
        &t.guest.token,
        json!({ "mode": "random" }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert!(body["matchId"].is_string(), "{body}");
    // Joined, the room is no one's to open.
    assert_eq!(read(&t.app, &t.third, &code).await.0, 404);

    // A co-op room's code reads as its room.
    let (status, room) = api(
        &t.app,
        "POST",
        "/api/duos/rooms",
        &t.third.token,
        json!({ "difficulty": "hard", "random": true }),
    )
    .await;
    assert_eq!(status, 201, "{room}");
    let code = room["code"].as_str().expect("a room code").to_string();
    let seen = view(&t.app, &t.guest, &code).await;
    assert_eq!(seen["kind"], "coopRoom", "{seen}");
    assert_eq!(seen["difficulty"], "hard");
}

#[tokio::test]
async fn r3730005_an_invite_needs_a_friend_with_no_block() {
    let t = three().await;
    let code = duel(&t.app, &t.host, "random").await;
    let invite = |who: &Account| json!({ "profileId": who.id });
    let (status, body) = act(&t.app, &t.host, &code, "invite", invite(&t.guest)).await;
    assert_eq!(status, 404, "a stranger is not invited: {body}");
    befriend(&t.app, &t.host, &t.guest).await;
    let (status, body) = act(&t.app, &t.host, &code, "invite", invite(&t.guest)).await;
    assert_eq!(status, 204, "{body}");
    store!(t.app, tx => tx.blocks_add(&t.guest.id, &t.host.id, now_ms()).await.expect("blocks.add"));
    let (status, body) = act(&t.app, &t.host, &code, "invite", invite(&t.guest)).await;
    assert_eq!(status, 404, "a block ends the invite: {body}");
    store!(t.app, tx => tx.blocks_remove(&t.guest.id, &t.host.id).await.expect("blocks.remove"));
    ok_act(&t.app, &t.guest, &code, "enter", Value::Null).await;
    let (status, body) = act(&t.app, &t.host, &code, "invite", invite(&t.guest)).await;
    assert!(
        status == 409 && refused_for(&body, "already_in"),
        "{status} {body}"
    );
    befriend(&t.app, &t.host, &t.third).await;
    let (status, body) = act(&t.app, &t.host, &code, "invite", invite(&t.third)).await;
    assert!(status == 409 && refused_for(&body, "full"), "{status} {body}");
}

// A 2v2 between people (issue #3731, R3731000 to R3731003)

/// A host and four more accounts, the catalog registered.
struct Five {
    app: Arc<App>,
    host: Account,
    guest: Account,
    third: Account,
    fourth: Account,
    fifth: Account,
}

async fn five() -> Five {
    let Three {
        app,
        host,
        guest,
        third,
    } = three().await;
    let fourth = account(&app, 4, "Di").await;
    let fifth = account(&app, 5, "Ed").await;
    Five {
        app,
        host,
        guest,
        third,
        fourth,
        fifth,
    }
}

async fn two_v_two(app: &Arc<App>, host: &Account) -> String {
    create(app, host, json!({ "format": "2v2" })).await
}

/// Each seat's role and who sits there, in the lobby's order.
fn lineup_of(view: &Value) -> Vec<(String, Option<String>)> {
    view["seats"]
        .as_array()
        .expect("the seats")
        .iter()
        .map(|seat| {
            (
                seat["role"].as_str().expect("a role").to_string(),
                seat["player"]["profileId"].as_str().map(str::to_string),
            )
        })
        .collect()
}

/// A 2v2 lobby with every seat filled in entry order, each member on a random deck and ready.
async fn full_two_v_two(t: &Five) -> String {
    let code = two_v_two(&t.app, &t.host).await;
    for who in [&t.guest, &t.third, &t.fourth] {
        ok_act(&t.app, who, &code, "enter", Value::Null).await;
    }
    for who in [&t.host, &t.guest, &t.third, &t.fourth] {
        ok_act(&t.app, who, &code, "pick", json!({ "random": true })).await;
        ok_act(&t.app, who, &code, "ready", json!({ "ready": true })).await;
    }
    code
}

#[tokio::test]
async fn r3731000_a_two_v_two_lobby_seats_four_in_two_teams() {
    let t = five().await;
    let code = two_v_two(&t.app, &t.host).await;
    let seen = view(&t.app, &t.host, &code).await;
    assert_eq!(seen["settings"], json!({ "format": "2v2" }), "{seen}");
    assert_eq!(
        lineup_of(&seen),
        vec![
            ("host".to_string(), Some(t.host.id.clone())),
            ("guest".to_string(), None),
            ("guest2".to_string(), None),
            ("guest3".to_string(), None),
        ]
    );
    // Each entrant takes the first empty seat, and a visitor reads the lobby while one is empty.
    for (who, role) in [(&t.guest, "guest"), (&t.third, "guest2"), (&t.fourth, "guest3")] {
        assert_eq!(view(&t.app, who, &code).await["you"], Value::Null);
        ok_act(&t.app, who, &code, "enter", Value::Null).await;
        assert_eq!(view(&t.app, who, &code).await["you"], role, "{role}");
    }
    let (status, mine) = api(&t.app, "GET", "/api/lobbies", &t.fourth.token, Value::Null).await;
    assert_eq!(status, 200, "{mine}");
    assert_eq!(mine["lobbies"][0]["you"], "guest3", "{mine}");
    // Four is full: a fifth neither reads nor enters it.
    assert_eq!(read(&t.app, &t.fifth, &code).await.0, 404);
    let (status, body) = act(&t.app, &t.fifth, &code, "enter", Value::Null).await;
    assert!(status == 409 && refused_for(&body, "full"), "{status} {body}");
    // A 2v2 has nothing to set.
    let (status, body) = act(
        &t.app,
        &t.host,
        &code,
        "settings",
        json!({ "difficulty": "hard" }),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    // Start waits for all four to be ready.
    for who in [&t.host, &t.guest, &t.third] {
        ok_act(&t.app, who, &code, "pick", json!({ "random": true })).await;
        ok_act(&t.app, who, &code, "ready", json!({ "ready": true })).await;
    }
    let (status, body) = act(&t.app, &t.host, &code, "start", Value::Null).await;
    assert!(
        status == 409 && refused_for(&body, "not_ready"),
        "{status} {body}"
    );
}

#[tokio::test]
async fn r3731000_switch_team_and_the_hosts_move_take_every_ready_off() {
    let t = five().await;
    let code = two_v_two(&t.app, &t.host).await;
    for who in [&t.guest, &t.third] {
        ok_act(&t.app, who, &code, "enter", Value::Null).await;
    }
    for who in [&t.host, &t.guest, &t.third] {
        ok_act(&t.app, who, &code, "pick", json!({ "random": true })).await;
        ok_act(&t.app, who, &code, "ready", json!({ "ready": true })).await;
    }
    let all_off = |seen: &Value| {
        states(seen)
            .iter()
            .all(|state| state == "chosen" || state == "choosing")
    };
    // The guest switches to the other team's empty seat, keeping their pick; every Ready goes off.
    ok_act(&t.app, &t.guest, &code, "team", Value::Null).await;
    let seen = view(&t.app, &t.host, &code).await;
    assert_eq!(
        lineup_of(&seen),
        vec![
            ("host".to_string(), Some(t.host.id.clone())),
            ("guest".to_string(), None),
            ("guest2".to_string(), Some(t.third.id.clone())),
            ("guest3".to_string(), Some(t.guest.id.clone())),
        ]
    );
    assert_eq!(
        states(&seen),
        ["chosen", "choosing", "chosen", "chosen"],
        "{seen}"
    );
    assert!(all_off(&seen));
    // The third crosses to the host's team; the fourth then takes the first empty seat.
    ok_act(&t.app, &t.third, &code, "team", Value::Null).await;
    ok_act(&t.app, &t.fourth, &code, "enter", Value::Null).await;
    let seen = view(&t.app, &t.host, &code).await;
    let seated: Vec<Option<String>> = lineup_of(&seen).into_iter().map(|(_, id)| id).collect();
    assert_eq!(
        seated,
        vec![
            Some(t.host.id.clone()),
            Some(t.third.id.clone()),
            Some(t.fourth.id.clone()),
            Some(t.guest.id.clone()),
        ]
    );
    // Full teams: no one switches, and the host's team is the host's.
    let (status, body) = act(&t.app, &t.guest, &code, "team", Value::Null).await;
    assert!(
        status == 409 && refused_for(&body, "team_full"),
        "{status} {body}"
    );
    let (status, body) = act(&t.app, &t.host, &code, "team", Value::Null).await;
    assert!(
        status == 409 && refused_for(&body, "host_stays"),
        "{status} {body}"
    );
    // The host's Move swaps a member into a seat; it is the host's alone.
    for who in [&t.host, &t.third, &t.fourth, &t.guest] {
        ok_act(&t.app, who, &code, "pick", json!({ "random": true })).await;
        ok_act(&t.app, who, &code, "ready", json!({ "ready": true })).await;
    }
    let moved = json!({ "profileId": t.fourth.id, "seat": "guest" });
    let (status, body) = act(&t.app, &t.third, &code, "move", moved.clone()).await;
    assert!(status == 409 && refused_for(&body, "not_host"), "{status} {body}");
    ok_act(&t.app, &t.host, &code, "move", moved).await;
    let seen = view(&t.app, &t.host, &code).await;
    let seated: Vec<Option<String>> = lineup_of(&seen).into_iter().map(|(_, id)| id).collect();
    assert_eq!(
        seated,
        vec![
            Some(t.host.id.clone()),
            Some(t.fourth.id.clone()),
            Some(t.third.id.clone()),
            Some(t.guest.id.clone()),
        ]
    );
    assert!(all_off(&seen), "{seen}");
    // The host is never moved, nor is anyone into the host's seat.
    let (status, body) = act(
        &t.app,
        &t.host,
        &code,
        "move",
        json!({ "profileId": t.host.id, "seat": "guest2" }),
    )
    .await;
    assert_eq!(status, 404, "{body}");
    let (status, body) = act(
        &t.app,
        &t.host,
        &code,
        "move",
        json!({ "profileId": t.guest.id, "seat": "host" }),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    // Outside a 2v2 there are no teams.
    let other = coop(&t.app, &t.host).await;
    ok_act(&t.app, &t.fifth, &other, "enter", Value::Null).await;
    assert_eq!(act(&t.app, &t.fifth, &other, "team", Value::Null).await.0, 400);
}

#[tokio::test]
async fn r3731000_the_other_teams_deck_is_shown_by_its_state_alone() {
    let t = five().await;
    let code = two_v_two(&t.app, &t.host).await;
    for who in [&t.guest, &t.third, &t.fourth] {
        ok_act(&t.app, who, &code, "enter", Value::Null).await;
    }
    let (host_deck, third_deck) = (uuid(310), uuid(311));
    with_saved_deck(&t.app, &t.host, &host_deck, "Host's deck").await;
    with_saved_deck(&t.app, &t.third, &third_deck, "Third's deck").await;
    ok_act(&t.app, &t.host, &code, "pick", json!({ "deckId": host_deck })).await;
    ok_act(&t.app, &t.guest, &code, "pick", json!({ "random": true })).await;
    ok_act(&t.app, &t.third, &code, "pick", json!({ "deckId": third_deck })).await;
    ok_act(&t.app, &t.fourth, &code, "pick", json!({ "random": true })).await;
    let shown = |seen: &Value| -> Vec<(Value, Value)> {
        seen["seats"]
            .as_array()
            .expect("the seats")
            .iter()
            .map(|seat| (seat["deck"].clone(), seat["random"].clone()))
            .collect()
    };
    let hidden = (Value::Null, json!(false));
    // The host's team sees its own decks, and of the other team only that each has picked.
    let seen = view(&t.app, &t.guest, &code).await;
    assert_eq!(
        shown(&seen),
        vec![
            (json!("Host's deck"), json!(false)),
            (Value::Null, json!(true)),
            hidden.clone(),
            hidden.clone()
        ],
        "{seen}"
    );
    assert_eq!(states(&seen), ["chosen"; 4]);
    // And the other team the other way round.
    let seen = view(&t.app, &t.fourth, &code).await;
    assert_eq!(
        shown(&seen),
        vec![
            hidden.clone(),
            hidden,
            (json!("Third's deck"), json!(false)),
            (Value::Null, json!(true))
        ],
        "{seen}"
    );
    // The caller's own pick comes back as it was frozen, as a co-op lobby's does.
    assert_eq!(seen["yourPick"], json!({ "mode": "random" }), "{seen}");
}

#[tokio::test]
async fn r3731000_a_block_between_two_guests_drops_the_later() {
    let t = five().await;
    let code = two_v_two(&t.app, &t.host).await;
    for who in [&t.guest, &t.third, &t.fourth] {
        ok_act(&t.app, who, &code, "enter", Value::Null).await;
    }
    // A block between the guest and the fourth (guest3) drops the fourth, seated later.
    store!(t.app, tx => tx.blocks_add(&t.fourth.id, &t.guest.id, now_ms()).await.expect("blocks.add"));
    let seen = view(&t.app, &t.host, &code).await;
    assert_eq!(lineup_of(&seen)[3], ("guest3".to_string(), None), "{seen}");
    assert_eq!(read(&t.app, &t.fourth, &code).await.0, 404);
    assert_eq!(act(&t.app, &t.fourth, &code, "enter", Value::Null).await.0, 404);
    // A visitor blocked with any member reads the lobby as an unknown code, and is never invited.
    let (_, unknown) = read(&t.app, &t.fifth, "ZZZZZZ").await;
    store!(t.app, tx => tx.blocks_add(&t.third.id, &t.fifth.id, now_ms()).await.expect("blocks.add"));
    assert_eq!(read(&t.app, &t.fifth, &code).await, (404, unknown));
    assert_eq!(act(&t.app, &t.fifth, &code, "enter", Value::Null).await.0, 404);
    befriend(&t.app, &t.guest, &t.fifth).await;
    let (status, body) = act(
        &t.app,
        &t.guest,
        &code,
        "invite",
        json!({ "profileId": t.fifth.id }),
    )
    .await;
    assert_eq!(status, 404, "{body}");
    // Once the block is gone the visitor reads it, enters the empty seat and is no one's to drop.
    store!(t.app, tx => tx.blocks_remove(&t.third.id, &t.fifth.id).await.expect("blocks.remove"));
    ok_act(
        &t.app,
        &t.guest,
        &code,
        "invite",
        json!({ "profileId": t.fifth.id }),
    )
    .await;
    ok_act(&t.app, &t.fifth, &code, "enter", Value::Null).await;
    assert_eq!(view(&t.app, &t.fifth, &code).await["you"], "guest3");
}

/// The first seeds whose coin lands each way (R1989000's `seat_coin_p1_first`).
fn seeds_each_way() -> (String, String) {
    let seed = |want: bool| {
        (0..)
            .map(|n| format!("seed-2v2-{n}"))
            .find(|seed| jackioh_server::api::queue::seat_coin_p1_first(seed) == want)
            .expect("a seed")
    };
    (seed(true), seed(false))
}

#[tokio::test]
async fn r3731001_the_seeds_coin_picks_which_lobby_team_plays_a() {
    let (heads, tails) = seeds_each_way();
    for (seed, host_team_first) in [(heads, true), (tails, false)] {
        let t = five().await;
        let code = full_two_v_two(&t).await;
        let (status, started) = act(&t.app, &t.host, &code, "start", json!({ "seed": seed })).await;
        assert_eq!(status, 200, "{started}");
        let match_id = started["matchId"].as_str().expect("a match id").to_string();
        assert!(started["seriesId"].is_null(), "{started}");
        let row = store!(t.app, tx => tx.duos_matches_get(&match_id).await.expect("duos.matches.get"))
            .expect("the 2v2 match");
        let (team_a, team_b) = if host_team_first {
            ([&t.host, &t.guest], [&t.third, &t.fourth])
        } else {
            ([&t.third, &t.fourth], [&t.host, &t.guest])
        };
        // Each team's first seat plays duel 0.
        assert_eq!(
            [row.a1.as_str(), row.a2.as_str(), row.b1.as_str(), row.b2.as_str()],
            [
                team_a[0].id.as_str(),
                team_a[1].id.as_str(),
                team_b[0].id.as_str(),
                team_b[1].id.as_str()
            ],
            "{seed}"
        );
        assert_eq!(row.seed, seed);
        // R3731002: team B is two people on the humans' rules, with no handicap and no tier read.
        assert_eq!(row.team_b(), jackioh_shared::host::DuosTeamB::People);
        assert_eq!(row.options.teams.b, jackioh_engine::config::DUOS_HUMAN_TEAM);
        assert_eq!((row.options.handicaps.b1, row.options.handicaps.b2), (None, None));
        assert_eq!(row.options.dealt.len(), 4, "every random pick is dealt");
        assert_eq!(
            serde_json::to_value(row.difficulty).expect("a difficulty"),
            "easy"
        );
        // Every person holds the match's seat until it ends.
        for who in [&t.host, &t.guest, &t.third, &t.fourth] {
            assert_eq!(
                t.app.seats.of(&who.id),
                Some(Seat::Duos(match_id.clone())),
                "{}",
                who.id
            );
        }
    }
}

#[tokio::test]
async fn r3731003_a_two_v_two_offers_no_play_again() {
    let t = five().await;
    let code = full_two_v_two(&t).await;
    let (status, started) = act(&t.app, &t.host, &code, "start", Value::Null).await;
    assert_eq!(status, 200, "{started}");
    let match_id = started["matchId"].as_str().expect("a match id").to_string();
    let draw = json!({ "outcome": "draw", "winner": null, "reason": "ceiling" });
    assert!(
        store!(t.app, tx => tx.duos_matches_finish(&match_id, &draw, now_ms()).await.expect("duos.matches.finish"))
    );
    let path = format!("/api/duos/matches/{match_id}/again");
    for who in [&t.host, &t.fourth] {
        let (status, body) = api(&t.app, "POST", &path, &who.token, json!({})).await;
        assert_eq!(status, 409, "{body}");
        assert_eq!(
            body["error"]["message"], "a 2v2 game cannot be played again",
            "{body}"
        );
        let (status, body) = api(&t.app, "GET", &path, &who.token, Value::Null).await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["unavailable"], true, "{body}");
        assert!(body["matchId"].is_null(), "{body}");
    }
    // Someone who played none of its seats meets the unknown match's answer.
    assert_eq!(
        api(&t.app, "GET", &path, &t.fifth.token, Value::Null).await.0,
        404
    );
}
