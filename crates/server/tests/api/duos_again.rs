//! Play again after a finished co-op match (SPEC §13.9, R2092000, R1065000).
//! Two seats offering starts one new match; respects TTL, blocks, and active games.

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};

use jackioh_engine::LeanPick;
use jackioh_engine::config::DUOS_HUMAN_TEAM;
use jackioh_server::actor::engine::{deal_random_deck, lean_of};
use jackioh_server::actor::ws_server::Socket;
use jackioh_server::app::{App, now_ms};
use jackioh_server::config::REMATCH_OFFER_TTL_MS;
use jackioh_server::duos::registry::registry;
use jackioh_server::duos::store::DuosMatch;
use jackioh_server::seats::Seat;

use crate::support::deps::{add_user, call, empty_test_app};
use crate::support::engine::playable;
use crate::support::store::{advance, fake, from, store, to_json, uuid};

fn millis<T>(value: T) -> u64
where
    u64: TryFrom<T>,
    <u64 as TryFrom<T>>::Error: std::fmt::Debug,
{
    u64::try_from(value).expect("a duration fits a u64")
}

struct Account {
    id: String,
    token: String,
}

async fn account(app: &Arc<App>, n: u32, base: &str) -> Account {
    let id = uuid(n);
    let user_id = format!("user-{n}");
    fake(app).lock().await.seed_profile(json!({
        "id": id,
        "userId": user_id,
        "status": "active",
        "username": base,
        "usernameTag": i64::from(n),
        "usernamePrompted": true,
    }));
    let token = add_user(app, &user_id, &format!("{user_id}@example.test"), true);
    Account { id, token }
}

async fn api(app: &Arc<App>, method: &str, path: &str, token: &str, body: Value) -> (u16, Value) {
    let (status, _headers, body) = call(app, method, path, Some(token), body).await;
    (status, body)
}

/// Lets the actors run what the last step queued for them.
async fn settle() {
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

/// The server, with its cards registered and two active accounts on it.
async fn app_with_pair() -> (Arc<App>, Account, Account) {
    jackioh_cards::register_all();
    let app = empty_test_app().await;
    let a1 = account(&app, 1, "Hana").await;
    let a2 = account(&app, 2, "Gus").await;
    (app, a1, a2)
}

/// The first `DECK_SIZE` playable ids: a legal deck.
fn deck() -> Vec<String> {
    let size = usize::try_from(jackioh_engine::config::DECK_SIZE).expect("a deck size");
    playable()[..size].to_vec()
}

/// The finished row's `options` by default: `a1` frozen from a legal deck, `a2` dealt, under the
/// server's catalog version.
fn default_options(app: &App) -> Value {
    let cards = deck();
    json!({
        "decks": { "a1": cards.clone(), "a2": cards.clone(), "b1": cards.clone(), "b2": cards.clone() },
        "dealt": ["a2"],
        "teams": { "a": to_json(&DUOS_HUMAN_TEAM), "b": to_json(&DUOS_HUMAN_TEAM) },
        "handicaps": { "a1": null, "a2": null, "b1": null, "b2": null },
        "catalogVersion": app.catalog.version,
    })
}

/// Seeds a finished co-op match row directly into the store with optional `extra` overrides.
async fn finished_row(app: &Arc<App>, id: &str, a1: &str, a2: &str, extra: Value) -> Value {
    let now = now_ms();
    let mut row = json!({
        "id": id,
        "seed": format!("seed-{id}"),
        "a1": a1,
        "a2": a2,
        "difficulty": "medium",
        "options": default_options(app),
        "createdAt": now,
        "endedAt": now,
        "result": { "outcome": "draw", "winner": null, "reason": "test" },
    });
    if let (Some(fields), Some(over)) = (row.as_object_mut(), extra.as_object()) {
        for (key, value) in over {
            fields.insert(key.clone(), value.clone());
        }
    }
    let duos_match: DuosMatch = from(row.clone());
    let (result, ended_at) = (
        duos_match.result.clone().expect("a finished match's result"),
        duos_match.ended_at.expect("a finished match's end"),
    );
    store!(app, t => {
        t.duos_matches_create(&duos_match).await.expect("duos.matches.create");
        t.duos_matches_finish(id, &result, ended_at).await.expect("duos.matches.finish");
    });
    row
}

async fn offer(app: &Arc<App>, token: &str, match_id: &str, body: Value) -> (u16, Value) {
    api(
        app,
        "POST",
        &format!("/api/duos/matches/{match_id}/again"),
        token,
        body,
    )
    .await
}

async fn status(app: &Arc<App>, token: &str, match_id: &str) -> (u16, Value) {
    api(
        app,
        "GET",
        &format!("/api/duos/matches/{match_id}/again"),
        token,
        Value::Null,
    )
    .await
}

async fn live_duos(app: &Arc<App>) -> Vec<String> {
    store!(app, t => t.duos_matches_live().await.expect("duos.matches.live"))
}

async fn new_match(app: &Arc<App>, id: &str) -> DuosMatch {
    store!(app, t => t
        .duos_matches_get(id)
        .await
        .expect("duos.matches.get")
        .expect("the new match"))
}

/// An Easy room the host holds, dealt at random; answers its code.
async fn room(app: &Arc<App>, host: &Account) -> String {
    let (status, body) = api(
        app,
        "POST",
        "/api/duos/rooms",
        &host.token,
        json!({ "difficulty": "easy", "random": true }),
    )
    .await;
    assert_eq!(status, 201, "{body}");
    body["code"].as_str().expect("a room code").to_string()
}

async fn join(app: &Arc<App>, who: &Account, code: &str) -> (u16, Value) {
    api(
        app,
        "POST",
        &format!("/api/duos/rooms/{code}/join"),
        &who.token,
        json!({ "random": true }),
    )
    .await
}

#[tokio::test(start_paused = true)]
async fn r2092000_both_seats_offering_starts_one_match_with_the_same_seats_difficulty_and_a_fresh_seed() {
    let (app, a1, a2) = app_with_pair().await;
    let finished_id = uuid(100);
    let finished = finished_row(&app, &finished_id, &a1.id, &a2.id, json!({})).await;

    assert_eq!(
        offer(&app, &a1.token, &finished_id, json!({})).await.1["matchId"],
        Value::Null
    );
    let (_, created) = offer(&app, &a2.token, &finished_id, json!({})).await;
    let new_id = created["matchId"].as_str().expect("a match id").to_string();

    // The second offer's answer and both seats' GET give the new game's id.
    assert_eq!(
        status(&app, &a1.token, &finished_id).await.1["matchId"],
        json!(new_id)
    );
    assert_eq!(
        status(&app, &a2.token, &finished_id).await.1["matchId"],
        json!(new_id)
    );

    let new = new_match(&app, &new_id).await;
    assert_eq!(new.a1, a1.id);
    assert_eq!(new.a2, a2.id);
    assert_eq!(to_json(&new)["difficulty"], "medium");
    assert_ne!(new.seed, finished["seed"].as_str().expect("a seed"));
    assert_eq!(new.options.catalog_version, app.catalog.version);

    // Exactly one live co-op match, started on the registry, and a late poller still finds it.
    assert_eq!(live_duos(&app).await, vec![new_id.clone()]);
    assert!(registry(&app).has(&new_id));
    assert_eq!(
        offer(&app, &a1.token, &finished_id, json!({})).await.1["matchId"],
        json!(new_id)
    );
}

#[tokio::test(start_paused = true)]
async fn r2092000_one_offer_answers_null_and_each_side_sees_its_own_offer() {
    let (app, a1, a2) = app_with_pair().await;
    let finished_id = uuid(101);
    finished_row(&app, &finished_id, &a1.id, &a2.id, json!({})).await;

    assert_eq!(
        offer(&app, &a1.token, &finished_id, json!({})).await.1["matchId"],
        Value::Null
    );

    let (_, seen_by_a1) = status(&app, &a1.token, &finished_id).await;
    assert_eq!(seen_by_a1["youOffered"], true);
    assert_eq!(seen_by_a1["teammateOffered"], false);
    assert_eq!(seen_by_a1["teammateHere"], false);
    assert_eq!(seen_by_a1["matchId"], Value::Null);

    let (_, seen_by_a2) = status(&app, &a2.token, &finished_id).await;
    assert_eq!(seen_by_a2["youOffered"], false);
    assert_eq!(seen_by_a2["teammateOffered"], true);
    assert_eq!(seen_by_a2["teammateHere"], false);
    assert_eq!(seen_by_a2["matchId"], Value::Null);

    assert!(live_duos(&app).await.is_empty());
}

#[tokio::test(start_paused = true)]
async fn r2092000_a_saved_deck_seat_replays_its_frozen_deck() {
    let (app, a1, a2) = app_with_pair().await;
    let finished_id = uuid(102);
    // By default `a1` played a saved deck and `a2` was dealt.
    let finished = finished_row(&app, &finished_id, &a1.id, &a2.id, json!({})).await;

    offer(&app, &a1.token, &finished_id, json!({})).await;
    let (_, created) = offer(&app, &a2.token, &finished_id, json!({})).await;
    let new_id = created["matchId"].as_str().expect("a match id").to_string();

    let new = to_json(&new_match(&app, &new_id).await);
    assert_eq!(new["options"]["decks"]["a1"], finished["options"]["decks"]["a1"]);
    assert_eq!(new["options"]["dealt"], json!(["a2"]));
}

#[tokio::test(start_paused = true)]
async fn r2092000_a_dealt_seat_is_dealt_afresh_on_its_latest_lean() {
    let (app, a1, a2) = app_with_pair().await;
    let finished_id = uuid(103);
    finished_row(&app, &finished_id, &a1.id, &a2.id, json!({})).await;

    offer(&app, &a1.token, &finished_id, json!({})).await;
    let (_, created) = offer(&app, &a2.token, &finished_id, json!({ "leanSet": "newest" })).await;
    let new_id = created["matchId"].as_str().expect("a match id").to_string();

    let new = to_json(&new_match(&app, &new_id).await);
    let seed = new["seed"].as_str().expect("a seed").to_string();
    assert_eq!(
        new["options"]["decks"]["a2"],
        json!(deal_random_deck(
            &format!("{seed}:a2-deck"),
            lean_of(Some(LeanPick::Newest))
        ))
    );
}

#[tokio::test(start_paused = true)]
async fn r2092000_a_stranger_an_unknown_id_and_a_live_match_get_the_replay_answers() {
    let (app, a1, a2) = app_with_pair().await;
    let third = account(&app, 3, "Tess").await;
    let finished_id = uuid(104);
    finished_row(&app, &finished_id, &a1.id, &a2.id, json!({})).await;

    // A stranger learns nothing beyond what an unknown id answers, on both endpoints.
    let unknown_id = uuid(904);
    for (token, id) in [
        (&third.token, finished_id.as_str()),
        (&a1.token, unknown_id.as_str()),
    ] {
        let (code, refused) = offer(&app, token, id, json!({})).await;
        assert_eq!(code, 404, "{refused}");
        assert_eq!(refused["error"]["message"], "no such co-op match");
        let (code, read) = status(&app, token, id).await;
        assert_eq!(code, 404, "{read}");
        assert_eq!(read["error"]["message"], "no such co-op match");
    }

    // A live match has no Play again until it ends, as it has no replay.
    let code = room(&app, &a1).await;
    let (_, joined) = join(&app, &a2, &code).await;
    let live_id = joined["matchId"].as_str().expect("a match id").to_string();
    let (status_code, refused) = offer(&app, &a1.token, &live_id, json!({})).await;
    assert_eq!(status_code, 409, "{refused}");
    assert_eq!(refused["error"]["message"], "this co-op match has not finished");
    let (status_code, read) = status(&app, &a1.token, &live_id).await;
    assert_eq!(status_code, 409, "{read}");
    assert_eq!(read["error"]["message"], "this co-op match has not finished");
}

#[tokio::test(start_paused = true)]
async fn r2092000_an_offer_past_the_window_makes_nothing_and_answers_null() {
    let (app, a1, a2) = app_with_pair().await;
    let finished_id = uuid(105);
    finished_row(&app, &finished_id, &a1.id, &a2.id, json!({})).await;

    // `a1`'s offer lapses with the window: past it nothing is recorded, so `a2`'s offer finds no
    // pair and answers null, and the entry is gone.
    offer(&app, &a1.token, &finished_id, json!({})).await;
    advance(millis(REMATCH_OFFER_TTL_MS) + 1).await;
    assert_eq!(
        offer(&app, &a2.token, &finished_id, json!({})).await.1["matchId"],
        Value::Null
    );
    assert!(live_duos(&app).await.is_empty());
    let (_, seen) = status(&app, &a1.token, &finished_id).await;
    assert_eq!(seen["youOffered"], false);
    assert_eq!(seen["teammateOffered"], false);
    assert_eq!(seen["matchId"], Value::Null);

    // An entry that made a game keeps its id past the window, so the seat that polls late still
    // learns where to go.
    let late_id = uuid(106);
    finished_row(&app, &late_id, &a1.id, &a2.id, json!({})).await;
    offer(&app, &a1.token, &late_id, json!({})).await;
    let (_, created) = offer(&app, &a2.token, &late_id, json!({})).await;
    let new_id = created["matchId"].as_str().expect("a match id").to_string();
    advance(millis(REMATCH_OFFER_TTL_MS) + 1).await;
    assert_eq!(
        offer(&app, &a1.token, &late_id, json!({})).await.1["matchId"],
        json!(new_id)
    );
    assert_eq!(
        status(&app, &a2.token, &late_id).await.1["matchId"],
        json!(new_id)
    );
}

#[tokio::test(start_paused = true)]
async fn r2092000_a_stale_catalog_answers_update_required_and_makes_nothing() {
    let (app, a1, a2) = app_with_pair().await;
    let finished_id = uuid(106);
    let mut options = default_options(&app);
    options["catalogVersion"] = json!("0.0.0-stale");
    finished_row(&app, &finished_id, &a1.id, &a2.id, json!({ "options": options })).await;

    // Refused before anything is recorded, so the second offer is refused the same way.
    for token in [&a1.token, &a2.token] {
        let (code, refused) = offer(&app, token, &finished_id, json!({})).await;
        assert_eq!(code, 409, "{refused}");
        assert_eq!(refused["error"]["code"], "update_required");
        assert_eq!(
            refused["error"]["details"]["expected"],
            json!(app.catalog.version)
        );
        assert_eq!(refused["error"]["details"]["received"], json!("0.0.0-stale"));
    }
    assert!(live_duos(&app).await.is_empty());
    let (_, seen) = status(&app, &a1.token, &finished_id).await;
    assert_eq!(seen["matchId"], Value::Null);
}

#[tokio::test(start_paused = true)]
async fn r2092000_a_custom_card_in_a_frozen_deck_is_refused_l7() {
    let (app, a1, a2) = app_with_pair().await;
    let finished_id = uuid(107);
    let custom = "custom-0123456789abcdef0123456789abcdef";
    let mut options = default_options(&app);
    options["decks"]["a1"][0] = json!(custom);
    finished_row(&app, &finished_id, &a1.id, &a2.id, json!({ "options": options })).await;

    offer(&app, &a1.token, &finished_id, json!({})).await;
    let (code, refused) = offer(&app, &a2.token, &finished_id, json!({})).await;
    assert_eq!(code, 422, "{refused}");
    assert_eq!(refused["error"]["code"], "loadout_invalid");
    let details = refused["error"]["details"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(details.len(), 1, "{refused}");
    assert_eq!(details[0]["rule"], "L7");
    assert_eq!(details[0]["cardId"], custom);
    assert_eq!(refused["error"]["message"], details[0]["message"]);
    assert!(live_duos(&app).await.is_empty());
    // The claim was let go: the status shows no game made.
    assert_eq!(
        status(&app, &a1.token, &finished_id).await.1["matchId"],
        Value::Null
    );
}

#[tokio::test(start_paused = true)]
async fn r2092000_a_seat_in_another_game_is_refused_with_no_details() {
    let (app, a1, a2) = app_with_pair().await;
    let finished_id = uuid(108);
    finished_row(&app, &finished_id, &a1.id, &a2.id, json!({})).await;

    // `a1` holds a live co-op game elsewhere: the seat ledger's seat, as a door that seated it holds.
    app.seats
        .reserve(&[a1.id.as_str()], Seat::Duos("match-elsewhere".to_string()))
        .expect("premise: the seat reserves")
        .commit();

    // R1065000: the refusal names no game, since it may be the teammate's. `a2`, whose offer
    // completes the pair, never learns where `a1` plays, and `a1`'s own re-offer is told no more.
    offer(&app, &a1.token, &finished_id, json!({})).await;
    for who in [&a2, &a1] {
        let (code, refused) = offer(&app, &who.token, &finished_id, json!({})).await;
        assert_eq!(code, 409, "{refused}");
        assert_eq!(refused["error"]["code"], "already_in_match");
        assert!(refused["error"].get("details").is_none(), "{refused}");
        assert!(!refused.to_string().contains("match-elsewhere"), "{refused}");
    }
    assert!(live_duos(&app).await.is_empty());
    assert_eq!(
        status(&app, &a2.token, &finished_id).await.1["matchId"],
        Value::Null
    );
}

#[tokio::test(start_paused = true)]
async fn r2092000_a_block_made_after_the_game_answers_as_an_unknown_code() {
    let (app, a1, a2) = app_with_pair().await;
    let finished_id = uuid(109);
    finished_row(&app, &finished_id, &a1.id, &a2.id, json!({})).await;

    store!(app, t => t.blocks_add(&a1.id, &a2.id, now_ms()).await.expect("blocks.add"));

    offer(&app, &a1.token, &finished_id, json!({})).await;
    let (code, refused) = offer(&app, &a2.token, &finished_id, json!({})).await;
    assert_eq!(code, 404, "{refused}");
    assert_eq!(refused["error"]["message"], "that co-op room is not open");
    assert!(live_duos(&app).await.is_empty());
}

#[tokio::test(start_paused = true)]
async fn r2092000_two_offers_racing_make_exactly_one_match() {
    let (app, a1, a2) = app_with_pair().await;
    let finished_id = uuid(110);
    finished_row(&app, &finished_id, &a1.id, &a2.id, json!({})).await;

    // Concurrent offers make exactly one match under lock; retry reads created match id.
    assert_eq!(
        offer(&app, &a1.token, &finished_id, json!({})).await.1["matchId"],
        Value::Null
    );
    let (second, retry) = tokio::join!(
        offer(&app, &a2.token, &finished_id, json!({})),
        offer(&app, &a1.token, &finished_id, json!({})),
    );
    let second_id = second.1["matchId"].as_str().expect("a match id").to_string();
    let retry_id = retry.1["matchId"].as_str().expect("a match id").to_string();
    assert_eq!(second_id, retry_id);
    assert_eq!(live_duos(&app).await, vec![second_id.clone()]);
    assert!(registry(&app).has(&second_id));
}

#[tokio::test(start_paused = true)]
async fn r2092000_teammate_here_follows_the_teammates_socket() {
    let (app, a1, a2) = app_with_pair().await;
    let code = room(&app, &a1).await;
    let (_, joined) = join(&app, &a2, &code).await;
    let live_id = joined["matchId"].as_str().expect("a match id").to_string();
    // The game ends, but its actor stays: both people are still on it.
    let result = json!({ "outcome": "draw", "winner": null, "reason": "test" });
    store!(app, t => t.duos_matches_finish(&live_id, &result, now_ms()).await.expect("duos.matches.finish"));

    let (socket_a1, _frames_a1) = Socket::channel();
    let (socket_a2, _frames_a2) = Socket::channel();
    registry(&app)
        .attach(&app, &live_id, &a1.id, socket_a1.clone())
        .await
        .expect("a1 attaches");
    registry(&app)
        .attach(&app, &live_id, &a2.id, socket_a2.clone())
        .await
        .expect("a2 attaches");
    settle().await;

    assert_eq!(status(&app, &a1.token, &live_id).await.1["teammateHere"], true);
    assert_eq!(status(&app, &a2.token, &live_id).await.1["teammateHere"], true);

    // Only the teammate's seat counts: mine open alone is still gone.
    socket_a2.transport_closed();
    settle().await;
    assert_eq!(status(&app, &a2.token, &live_id).await.1["teammateHere"], true);
    assert_eq!(status(&app, &a1.token, &live_id).await.1["teammateHere"], false);

    registry(&app).stop(&live_id).await;
    assert_eq!(status(&app, &a1.token, &live_id).await.1["teammateHere"], false);
}

#[tokio::test(start_paused = true)]
async fn r2092000_an_emptied_seat_refuses_the_remaining_seats_offer() {
    let (app, _a1, a2) = app_with_pair().await;
    let finished_id = uuid(111);
    // R1497000: the deleted account's seat emptied when the game was over.
    finished_row(&app, &finished_id, "", &a2.id, json!({})).await;

    let (code, refused) = offer(&app, &a2.token, &finished_id, json!({})).await;
    assert_eq!(code, 409, "{refused}");
    assert_eq!(
        refused["error"]["message"],
        "that co-op game cannot be played again"
    );
    assert!(live_duos(&app).await.is_empty());
}
