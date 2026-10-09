//! Rematch offers after a finished non-series match (SPEC §9.5, R672):
//! `POST`/`GET /api/matches/:matchId/rematch` (`src/api/rematch.rs`), the rating move a
//! double-or-nothing rematch makes (`src/api/ranked.rs`, `src/api/results.rs`), and the presence
//! the registry reports off the live actor's sockets (`src/actor/registry.rs`,
//! `src/actor/match_actor.rs`).
//!
//!  - **R672**: a double-or-nothing rematch is ranked-only and doubles each side's Glicko rating
//!    delta, with deviation and volatility from the single update.
//!  - Equal stakes from both seats create exactly one match with the finished decks, a fresh seed
//!    and the same seats; mismatched stakes create nothing.
//!  - A non-seat learns nothing (404); a live match (422), a series game (422) and a double from
//!    an unranked match (422) are refused.
//!  - Stale offers are ignored past `REMATCH_OFFER_TTL_MS`; `opponentHere` follows the opponent's
//!    socket, false once it closes or the actor is gone.
//!  - **R1372**: an All Random rematch leans the deck of each seat whose latest offer asked, and the
//!    status names the mode the rematch plays.
//!
//! Everything runs on tokio's paused clock through `test_app()`, with the real registry: the
//! created rematch is asserted off the store and off `Registry::has`. Presence and call order are
//! reached for real (sockets on a live actor; the order of the store's calls).
//!
//! The offers are module state shared by every test of this binary, so each test names its own
//! profiles and matches instead of clearing the map, which would wipe a test running beside it.

use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use tokio::sync::MutexGuard;

use jackioh_engine::{PlayerId, newest_shipped_set};
use jackioh_server::actor::engine::deal_random_deck;
use jackioh_server::actor::ws_server::Socket;
use jackioh_server::api::ranked::rate_ranked_game;
use jackioh_server::api::rematch::{STAKE_DOUBLE, STAKE_NORMAL};
use jackioh_server::api::results::record_result;
use jackioh_server::app::{App, now_ms};
use jackioh_server::config::REMATCH_OFFER_TTL_MS;
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{Db, MatchSeat, StartMatchInput};

use crate::support::deps::{add_user, call, test_app};

// Plumbing (private copies: each test file of this binary keeps its own)

/// Builds a value from a JSON literal, so the test depends on the JSON shape only.
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

/// `{ status, body }` of one request.
async fn request(app: &Arc<App>, method: &str, path: &str, token: &str, body: Option<Value>) -> (u16, Value) {
    let (status, _headers, body) = call(app, method, path, Some(token), body.unwrap_or(Value::Null)).await;
    (status, body)
}

/// Lets the actors run what the last step queued for them.
async fn settle() {
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

/// This test's own profiles A and B and its finished match's id.
fn names(tag: &str) -> (String, String, String) {
    (format!("{tag}-a"), format!("{tag}-b"), format!("{tag}-finished"))
}

/// An active profile with a token that verifies as it (`queue.rs`'s fixture).
async fn active_profile(app: &App, id: &str, rating: f64) -> String {
    let user_id = format!("user-{id}");
    fake(app)
        .await
        .seed_profile(json!({ "id": id, "userId": user_id, "status": "active", "rating": rating }));
    add_user(app, &user_id, &format!("{id}@example.test"), true)
}

/// Two legal decks of real cards, so the real registry can start a rematch on them.
fn real_decks() -> (Vec<String>, Vec<String>) {
    jackioh_cards::register_all();
    let size = usize::try_from(jackioh_engine::config::DECK_SIZE).expect("a deck size");
    let pool: Vec<String> = jackioh_cards::CATALOG
        .values()
        .filter(|def| !def.token)
        .map(|def| def.id.clone())
        .collect();
    (pool[..size].to_vec(), pool[size..size * 2].to_vec())
}

fn clocks(now: i64) -> Value {
    json!({
        "turnDeadline": null,
        "promptDeadline": null,
        "graceDeadline": { "p1": null, "p2": null },
        "ceilingAt": now + 3_600_000,
    })
}

/// A finished Best-of-1 match between the two profiles, ranked unless `extra` says otherwise.
async fn finished_match(app: &App, id: &str, p1: &str, p2: &str, extra: Value) -> Value {
    let now = now_ms();
    let (one, two) = real_decks();
    let mut row = json!({
        "id": id,
        "seed": format!("seed-{id}"),
        "players": [p1, p2],
        "decks": [one, two],
        "catalogVersion": app.catalog.version,
        "ranked": true,
        "status": "finished",
        "createdAt": now,
        "finishedAt": now,
        "clocks": clocks(now),
        "portraits": ["vanilla", "vanilla"],
    });
    if let (Some(fields), Some(extra)) = (row.as_object_mut(), extra.as_object()) {
        for (key, value) in extra {
            fields.insert(key.clone(), value.clone());
        }
    }
    q!(app, matches_create(&from(row.clone())));
    row
}

/// Queue tickets the finished match paired, so `matches_mode_of` reads its mode.
async fn paired_tickets(app: &App, match_id: &str, profiles: [&str; 2], mode: &str) {
    let now = now_ms();
    for profile_id in profiles {
        q!(
            app,
            tickets_insert(&from(json!({
                "id": format!("ticket-{match_id}-{profile_id}"),
                "profileId": profile_id,
                "rating": 1000,
                "mode": mode,
                "deck": [],
                "portrait": null,
                "trio": null,
                "catalogVersion": app.catalog.version,
                "enqueuedAt": now,
                "status": "matched",
                "matchId": match_id,
            })))
        );
    }
}

async fn offer(app: &Arc<App>, token: &str, match_id: &str, stakes: impl Serialize) -> (u16, Value) {
    request(
        app,
        "POST",
        &format!("/api/matches/{match_id}/rematch"),
        token,
        Some(json!({ "stakes": stakes })),
    )
    .await
}

/// R1372: a normal offer that asks, or does not ask, for "More cards from the newest set".
async fn lean_offer(app: &Arc<App>, token: &str, match_id: &str, lean_newest: bool) -> (u16, Value) {
    request(
        app,
        "POST",
        &format!("/api/matches/{match_id}/rematch"),
        token,
        Some(json!({ "stakes": STAKE_NORMAL, "leanNewest": lean_newest })),
    )
    .await
}

async fn status(app: &Arc<App>, token: &str, match_id: &str) -> (u16, Value) {
    request(
        app,
        "GET",
        &format!("/api/matches/{match_id}/rematch"),
        token,
        None,
    )
    .await
}

async fn in_match_of(app: &App, profile_id: &str) -> Value {
    let profile = q!(app, profiles_get_by_id(profile_id)).map(|profile| to_json(&profile));
    profile
        .map(|profile| profile["inMatchId"].clone())
        .unwrap_or(Value::Null)
}

async fn live_ids(app: &App) -> Vec<Value> {
    q!(app, matches_live())
        .iter()
        .map(|row| to_json(row)["id"].clone())
        .collect()
}

async fn match_row(app: &App, match_id: &str) -> Value {
    q!(app, matches_get(match_id))
        .map(|row| to_json(&row))
        .unwrap_or(Value::Null)
}

/// A live match on the real registry, A as p1 and B as p2.
async fn start_live(app: &Arc<App>, match_id: &str, a: &str, b: &str) {
    let (one, two) = real_decks();
    app.matches
        .start(
            app,
            StartMatchInput {
                match_id: match_id.to_string(),
                seed: format!("seed-{match_id}"),
                catalog_version: app.catalog.version.clone(),
                ranked: false,
                seats: (
                    MatchSeat {
                        profile_id: a.to_string(),
                        player: PlayerId::P1,
                        deck: one,
                        portrait: None,
                    },
                    MatchSeat {
                        profile_id: b.to_string(),
                        player: PlayerId::P2,
                        deck: two,
                        portrait: None,
                    },
                ),
                mode: None,
                stake: None,
            },
        )
        .await
        .expect("the match starts");
}

// R672 — the rating move

mod r672_double_or_nothing {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r672_doubles_each_sides_rating_movement_with_deviation_and_volatility_from_the_single_update() {
        let app = test_app().await;
        let (a, b, _) = names("rate-unit");
        let (c, d) = ("rate-unit-c".to_string(), "rate-unit-d".to_string());
        {
            let mut data = fake(&app).await;
            for profile_id in [&a, &b, &c, &d] {
                data.seed_profile(json!({ "id": profile_id }));
            }
        }
        let sides = |x: &str, y: &str| json!([{ "kind": "player", "profileId": x }, { "kind": "player", "profileId": y }]);
        let single = {
            let mut tx = app.db.begin(None).await.expect("begin");
            let rated = rate_ranked_game(
                &mut tx,
                &app,
                &from(json!({
                    "id": "rate-unit-single",
                    "kind": "match",
                    "catalogVersion": app.catalog.version,
                    "sides": sides(&a, &b),
                    "winnerSide": 0,
                    "reason": "hero-death",
                    "at": now_ms(),
                })),
            )
            .await
            .expect("rates");
            tx.commit().await.expect("commit");
            to_json(&rated)
        };
        let doubled = {
            let mut tx = app.db.begin(None).await.expect("begin");
            let rated = rate_ranked_game(
                &mut tx,
                &app,
                &from(json!({
                    "id": "rate-unit-doubled",
                    "kind": "match",
                    "catalogVersion": app.catalog.version,
                    "sides": sides(&c, &d),
                    "winnerSide": 0,
                    "reason": "hero-death",
                    "at": now_ms(),
                    "stake": STAKE_DOUBLE,
                })),
            )
            .await
            .expect("rates");
            tx.commit().await.expect("commit");
            to_json(&rated)
        };

        for side in [0usize, 1] {
            let rating =
                |row: &Value, when: &str| row["sides"][side][when]["rating"].as_f64().expect("a rating");
            let before = rating(&single, "before");
            let single_delta = rating(&single, "after") - before;
            // The same starting ratings, so the single update's delta is the doubled game's unit.
            assert_eq!(before, rating(&doubled, "before"));
            assert!(((rating(&doubled, "after") - before) - 2.0 * single_delta).abs() < 1e-10);
            // Confidence is not doubled: the same update's deviation and volatility.
            assert_eq!(
                doubled["sides"][side]["after"]["deviation"],
                single["sides"][side]["after"]["deviation"]
            );
            assert_eq!(
                doubled["sides"][side]["after"]["volatility"],
                single["sides"][side]["after"]["volatility"]
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r672_a_finished_double_or_nothing_rematch_moves_each_sides_rating_twice_as_far() {
        let app = test_app().await;
        let (a, b, _) = names("rate-game");
        let (c, d) = ("rate-game-c".to_string(), "rate-game-d".to_string());
        for (pair, id, stake) in [
            ([&a, &b], "rate-game-double", STAKE_DOUBLE),
            ([&c, &d], "rate-game-single", STAKE_NORMAL),
        ] {
            let [p1, p2] = pair;
            {
                let mut data = fake(&app).await;
                data.seed_profile(json!({ "id": p1 }));
                data.seed_profile(json!({ "id": p2 }));
            }
            let extra = if stake == STAKE_DOUBLE {
                json!({ "stake": stake })
            } else {
                json!({})
            };
            finished_match(&app, id, p1, p2, extra).await;
            record_result(
                &app,
                from(json!({
                    "matchId": id,
                    "seats": [
                        { "profileId": p1, "player": "p1", "deck": [] },
                        { "profileId": p2, "player": "p2", "deck": [] },
                    ],
                    "outcome": { "winner": "p1", "reason": "hero-death" },
                    "turns": 9,
                    "at": now_ms(),
                })),
            )
            .await
            .expect("the result is written");
        }
        let ids = vec![a.clone(), b.clone(), c.clone(), d.clone()];
        let profiles = to_json(&q!(app, profiles_get_many(&ids)));
        let rating_of = |id: &str| {
            profiles
                .as_array()
                .and_then(|rows| rows.iter().find(|row| row["id"] == id))
                .and_then(|row| row["rating"].as_f64())
                .expect("premise: all four profiles exist")
        };
        // Both pairs started at the same rating, so the doubled game's movement is twice the single's.
        assert!(((rating_of(&a) - 1000.0) - 2.0 * (rating_of(&c) - 1000.0)).abs() < 1e-8);
        assert!(((1000.0 - rating_of(&b)) - 2.0 * (1000.0 - rating_of(&d))).abs() < 1e-8);
    }

    #[tokio::test(start_paused = true)]
    async fn r672_a_double_from_an_unranked_match_is_refused() {
        let app = test_app().await;
        let (a, b, finished) = names("double-unranked");
        let token_a = active_profile(&app, &a, 1000.0).await;
        active_profile(&app, &b, 1000.0).await;
        finished_match(&app, &finished, &a, &b, json!({ "ranked": false })).await;

        let (code, body) = offer(&app, &token_a, &finished, STAKE_DOUBLE).await;
        assert_eq!(code, 422);
        assert_eq!(body["error"]["code"], "double_requires_ranked");
        let (_, seen) = status(&app, &token_a, &finished).await;
        assert_eq!(seen["youOffered"], Value::Null);
    }
}

// Offers and creation

mod rematch_offers {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn upserts_one_seats_offer_and_reports_both_seats_offers_back() {
        let app = test_app().await;
        let (a, b, finished) = names("upserts");
        let token_a = active_profile(&app, &a, 1000.0).await;
        let token_b = active_profile(&app, &b, 1000.0).await;
        finished_match(&app, &finished, &a, &b, json!({})).await;

        assert_eq!(
            offer(&app, &token_a, &finished, STAKE_NORMAL).await.1["matchId"],
            Value::Null
        );

        let (_, seen_by_a) = status(&app, &token_a, &finished).await;
        assert_eq!(seen_by_a["youOffered"], json!(STAKE_NORMAL));
        assert_eq!(seen_by_a["opponentOffer"], Value::Null);
        assert_eq!(seen_by_a["matchId"], Value::Null);
        let (_, seen_by_b) = status(&app, &token_b, &finished).await;
        assert_eq!(seen_by_b["youOffered"], Value::Null);
        assert_eq!(seen_by_b["opponentOffer"], json!(STAKE_NORMAL));
        assert_eq!(seen_by_b["matchId"], Value::Null);
    }

    #[tokio::test(start_paused = true)]
    async fn matching_offers_create_exactly_one_match_with_the_finished_decks_a_fresh_seed_and_the_same_seats()
     {
        let app = test_app().await;
        let (a, b, finished_id) = names("matching");
        let token_a = active_profile(&app, &a, 1000.0).await;
        let token_b = active_profile(&app, &b, 1000.0).await;
        let finished = finished_match(&app, &finished_id, &a, &b, json!({})).await;
        paired_tickets(&app, &finished_id, [&a, &b], "bo1").await;

        assert_eq!(
            offer(&app, &token_a, &finished_id, STAKE_NORMAL).await.1["matchId"],
            Value::Null
        );
        let (_, created) = offer(&app, &token_b, &finished_id, STAKE_NORMAL).await;
        let rematch_id = created["matchId"].as_str().expect("a match id").to_string();

        let rematch = match_row(&app, &rematch_id).await;
        assert_eq!(rematch["players"], json!([a, b]));
        assert_eq!(rematch["decks"], finished["decks"]);
        assert_eq!(rematch["ranked"], true);
        assert_eq!(rematch["mode"], "bo1");
        assert_eq!(rematch["status"], "live");
        assert!(
            rematch.get("stake").is_none_or(Value::is_null),
            "stake: {}",
            rematch["stake"]
        );
        assert_ne!(rematch["seed"], finished["seed"]);
        assert_eq!(in_match_of(&app, &a).await, json!(rematch_id));
        assert_eq!(in_match_of(&app, &b).await, json!(rematch_id));
        // The registry started it, so a reconnecting socket finds a live actor's match row.
        assert!(app.matches.has(&rematch_id));

        let (_, again) = offer(&app, &token_a, &finished_id, STAKE_NORMAL).await;
        assert_eq!(again["matchId"], json!(rematch_id));
        assert_eq!(live_ids(&app).await, vec![json!(rematch_id)]);
        // A late poller still learns where to go.
        assert_eq!(
            status(&app, &token_a, &finished_id).await.1["matchId"],
            json!(rematch_id)
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_double_or_nothing_both_seats_want_is_ranked_with_stakes_on_its_row() {
        let app = test_app().await;
        let (a, b, finished) = names("double-wanted");
        let token_a = active_profile(&app, &a, 1000.0).await;
        let token_b = active_profile(&app, &b, 1000.0).await;
        finished_match(&app, &finished, &a, &b, json!({})).await;
        paired_tickets(&app, &finished, [&a, &b], "bo1").await;

        offer(&app, &token_a, &finished, STAKE_DOUBLE).await;
        let (_, created) = offer(&app, &token_b, &finished, STAKE_DOUBLE).await;
        let rematch = match_row(&app, created["matchId"].as_str().expect("a match id")).await;
        assert_eq!(rematch["ranked"], true);
        assert_eq!(rematch["stake"], json!(STAKE_DOUBLE));
        assert_eq!(rematch["mode"], "bo1");
    }

    #[tokio::test(start_paused = true)]
    async fn mismatched_stakes_create_nothing() {
        let app = test_app().await;
        let (a, b, finished) = names("mismatched");
        let token_a = active_profile(&app, &a, 1000.0).await;
        let token_b = active_profile(&app, &b, 1000.0).await;
        finished_match(&app, &finished, &a, &b, json!({})).await;

        assert_eq!(
            offer(&app, &token_a, &finished, STAKE_NORMAL).await.1["matchId"],
            Value::Null
        );
        assert_eq!(
            offer(&app, &token_b, &finished, STAKE_DOUBLE).await.1["matchId"],
            Value::Null
        );
        assert!(live_ids(&app).await.is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn an_all_random_rematch_deals_fresh_decks_from_the_new_seed() {
        let app = test_app().await;
        let (a, b, finished_id) = names("random-rematch");
        let token_a = active_profile(&app, &a, 1000.0).await;
        let token_b = active_profile(&app, &b, 1000.0).await;
        let finished = finished_match(&app, &finished_id, &a, &b, json!({})).await;
        paired_tickets(&app, &finished_id, [&a, &b], "random").await;

        offer(&app, &token_a, &finished_id, STAKE_NORMAL).await;
        let (_, created) = offer(&app, &token_b, &finished_id, STAKE_NORMAL).await;
        let rematch = match_row(
            &app,
            created["matchId"]
                .as_str()
                .expect("premise: the rematch was created"),
        )
        .await;
        assert_eq!(rematch["mode"], "random");
        // Fresh decks from the new seed (`queue.rs`'s suffix scheme), not the finished ones.
        let seed = rematch["seed"].as_str().expect("a seed").to_string();
        assert_eq!(
            rematch["decks"],
            json!([
                deal_random_deck(&format!("{seed}:p1-deck"), None),
                deal_random_deck(&format!("{seed}:p2-deck"), None)
            ])
        );
        assert_ne!(rematch["decks"], finished["decks"]);
    }

    #[tokio::test(start_paused = true)]
    async fn r1372_an_all_random_rematch_leans_the_deck_of_exactly_the_seat_whose_offer_asked() {
        let app = test_app().await;
        let (a, b, finished_id) = names("lean-rematch");
        let token_a = active_profile(&app, &a, 1000.0).await;
        let token_b = active_profile(&app, &b, 1000.0).await;
        finished_match(&app, &finished_id, &a, &b, json!({})).await;
        paired_tickets(&app, &finished_id, [&a, &b], "random").await;
        // R1372: the death screen learns the rematch's mode, to offer the lean beside All Random only.
        assert_eq!(status(&app, &token_a, &finished_id).await.1["mode"], "random");

        // A (p1) asks first and then changes their mind: the latest offer's lean stands. B (p2) asks.
        lean_offer(&app, &token_a, &finished_id, true).await;
        offer(&app, &token_a, &finished_id, STAKE_NORMAL).await;
        let (_, created) = lean_offer(&app, &token_b, &finished_id, true).await;
        let rematch = match_row(
            &app,
            created["matchId"]
                .as_str()
                .expect("premise: the rematch was created"),
        )
        .await;
        let seed = rematch["seed"].as_str().expect("a seed").to_string();
        assert_eq!(
            rematch["decks"],
            json!([
                deal_random_deck(&format!("{seed}:p1-deck"), None),
                deal_random_deck(&format!("{seed}:p2-deck"), Some(newest_shipped_set()))
            ])
        );
    }

    #[tokio::test(start_paused = true)]
    async fn r1372_a_best_of_1_rematch_replays_its_decks_whatever_an_offer_asks() {
        let app = test_app().await;
        let (a, b, finished_id) = names("lean-bo1");
        let token_a = active_profile(&app, &a, 1000.0).await;
        let token_b = active_profile(&app, &b, 1000.0).await;
        let finished = finished_match(&app, &finished_id, &a, &b, json!({})).await;
        paired_tickets(&app, &finished_id, [&a, &b], "bo1").await;
        assert_eq!(status(&app, &token_a, &finished_id).await.1["mode"], "bo1");

        lean_offer(&app, &token_a, &finished_id, true).await;
        let (_, created) = lean_offer(&app, &token_b, &finished_id, true).await;
        let rematch = match_row(&app, created["matchId"].as_str().expect("the rematch")).await;
        assert_eq!(rematch["decks"], finished["decks"]);
        // And a lean that is not a boolean is refused like a malformed stake.
        let (code, _) = request(
            &app,
            "POST",
            &format!("/api/matches/{finished_id}/rematch"),
            &token_a,
            Some(json!({ "stakes": STAKE_NORMAL, "leanNewest": "please" })),
        )
        .await;
        assert_eq!(code, 400);
    }

    #[tokio::test(start_paused = true)]
    async fn a_stale_offer_is_ignored_past_the_ttl() {
        let app = test_app().await;
        let (a, b, finished) = names("stale");
        let token_a = active_profile(&app, &a, 1000.0).await;
        let token_b = active_profile(&app, &b, 1000.0).await;
        finished_match(&app, &finished, &a, &b, json!({})).await;

        offer(&app, &token_a, &finished, STAKE_NORMAL).await;
        tokio::time::advance(Duration::from_millis(
            u64::try_from(REMATCH_OFFER_TTL_MS).expect("a ttl") + 1,
        ))
        .await;

        // A's offer lapsed, so B's is the first live one: no game, and A must offer again.
        assert_eq!(
            offer(&app, &token_b, &finished, STAKE_NORMAL).await.1["matchId"],
            Value::Null
        );
        assert!(live_ids(&app).await.is_empty());
        assert_eq!(
            status(&app, &token_a, &finished).await.1["youOffered"],
            Value::Null
        );

        offer(&app, &token_a, &finished, STAKE_NORMAL).await;
        let (_, created) = status(&app, &token_b, &finished).await;
        assert_ne!(created["matchId"], Value::Null);
    }
}

// ---------------------------------------------------------------------------------------------
// Creation guards: the flags point at the created row, and a seat in another
// game is never stolen (`profiles_current_match_id_fkey` on Postgres)
// ---------------------------------------------------------------------------------------------

mod rematch_creation_guards {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn flags_the_seats_only_once_the_row_exists_which_the_foreign_key_demands() {
        let app = test_app().await;
        let (a, b, finished) = names("fk-order");
        let token_a = active_profile(&app, &a, 1000.0).await;
        let token_b = active_profile(&app, &b, 1000.0).await;
        finished_match(&app, &finished, &a, &b, json!({})).await;
        paired_tickets(&app, &finished, [&a, &b], "bo1").await;

        offer(&app, &token_a, &finished, STAKE_NORMAL).await;
        // Postgres refuses `current_match_id` pointing at no row; the fake does not, so the order
        // of the store's calls stands in for the foreign key: no flag before the row is written.
        let calls: Arc<StdMutex<Vec<String>>> = Arc::default();
        let seen = Arc::clone(&calls);
        fake(&app).await.on_call = Some(Arc::new(move |method: &str| {
            seen.lock().expect("calls").push(method.to_string());
            Ok(())
        }));
        let (_, created) = offer(&app, &token_b, &finished, STAKE_NORMAL).await;
        fake(&app).await.on_call = None;
        let created_id = created["matchId"].as_str().expect("a match id").to_string();

        let calls = calls.lock().expect("calls").clone();
        let created_at = calls.iter().position(|method| method == "matches.create");
        let flagged_at = calls.iter().position(|method| method == "profiles.setInMatch");
        assert!(created_at.is_some() && flagged_at.is_some(), "calls: {calls:?}");
        assert!(
            created_at < flagged_at,
            "a seat was flagged before its match row existed: {calls:?}"
        );
        assert_eq!(in_match_of(&app, &a).await, json!(created_id));
        assert_eq!(in_match_of(&app, &b).await, json!(created_id));
    }

    #[tokio::test(start_paused = true)]
    async fn refuses_with_409_when_a_seat_found_another_game_creating_nothing() {
        let app = test_app().await;
        let (a, b, finished) = names("seat-taken");
        let token_a = active_profile(&app, &a, 1000.0).await;
        let token_b = active_profile(&app, &b, 1000.0).await;
        finished_match(&app, &finished, &a, &b, json!({})).await;
        q!(app, profiles_set_in_match(&a, Some("match-elsewhere")));

        offer(&app, &token_a, &finished, STAKE_NORMAL).await;
        let (code, refused) = offer(&app, &token_b, &finished, STAKE_NORMAL).await;
        assert_eq!(code, 409);
        assert_eq!(refused["error"]["code"], "already_in_match");
        assert!(live_ids(&app).await.is_empty());
        // Neither seat moved: A stays where it is, B stays out.
        assert_eq!(in_match_of(&app, &a).await, json!("match-elsewhere"));
        assert_eq!(in_match_of(&app, &b).await, Value::Null);
    }

    #[tokio::test(start_paused = true)]
    async fn a_ranked_rematch_can_itself_spawn_a_double_while_an_unranked_rematch_stays_undoubled() {
        let app = test_app().await;
        let (a, b, finished) = names("chained");
        let token_a = active_profile(&app, &a, 1000.0).await;
        let token_b = active_profile(&app, &b, 1000.0).await;
        finished_match(&app, &finished, &a, &b, json!({})).await;
        paired_tickets(&app, &finished, [&a, &b], "bo1").await;

        offer(&app, &token_a, &finished, STAKE_NORMAL).await;
        let (_, first) = offer(&app, &token_b, &finished, STAKE_NORMAL).await;
        let chained = first["matchId"].as_str().expect("a match id").to_string();
        // The game ends, and the ending clears the flags (`results.rs`).
        q!(app, matches_finish(&chained, now_ms()));
        q!(app, profiles_set_in_match(&a, None));
        q!(app, profiles_set_in_match(&b, None));

        // The rematch is ranked exactly when the finished match was, so it can double (R672).
        offer(&app, &token_a, &chained, STAKE_DOUBLE).await;
        let (_, second) = offer(&app, &token_b, &chained, STAKE_DOUBLE).await;
        let second_id = second["matchId"].as_str().expect("a match id").to_string();
        let doubled = match_row(&app, &second_id).await;
        assert_eq!(doubled["ranked"], true);
        assert_eq!(doubled["stake"], json!(STAKE_DOUBLE));

        // An unranked rematch refuses the double like any unranked match.
        q!(app, profiles_set_in_match(&a, None));
        q!(app, profiles_set_in_match(&b, None));
        let casual_id = "chained-casual";
        finished_match(&app, casual_id, &a, &b, json!({ "ranked": false })).await;
        offer(&app, &token_a, casual_id, STAKE_NORMAL).await;
        let (_, casual) = offer(&app, &token_b, casual_id, STAKE_NORMAL).await;
        let casual_rematch = casual["matchId"].as_str().expect("a match id").to_string();
        q!(app, matches_finish(&casual_rematch, now_ms()));
        q!(app, profiles_set_in_match(&a, None));
        q!(app, profiles_set_in_match(&b, None));
        offer(&app, &token_a, &casual_rematch, STAKE_DOUBLE).await;
        let (code, refused) = offer(&app, &token_b, &casual_rematch, STAKE_DOUBLE).await;
        assert_eq!(code, 422);
        assert_eq!(refused["error"]["code"], "double_requires_ranked");
    }
}

// Refusals

mod rematch_refusals {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn a_non_seat_gets_404_on_both_endpoints_like_for_a_missing_match() {
        let app = test_app().await;
        let (a, b, finished) = names("non-seat");
        let token_a = active_profile(&app, &a, 1000.0).await;
        active_profile(&app, &b, 1000.0).await;
        let token_c = active_profile(&app, "non-seat-c", 1000.0).await;
        finished_match(&app, &finished, &a, &b, json!({})).await;

        let (code, posted) = offer(&app, &token_c, &finished, STAKE_NORMAL).await;
        assert_eq!(code, 404);
        assert_eq!(posted["error"]["code"], "not_found");
        let (code, read) = status(&app, &token_c, &finished).await;
        assert_eq!(code, 404);
        assert_eq!(read["error"]["code"], "not_found");

        let (code, _) = offer(&app, &token_a, "no-such-match", STAKE_NORMAL).await;
        assert_eq!(code, 404);
    }

    #[tokio::test(start_paused = true)]
    async fn an_unfinished_match_gets_422() {
        let app = test_app().await;
        let (a, b, finished) = names("unfinished");
        let token_a = active_profile(&app, &a, 1000.0).await;
        active_profile(&app, &b, 1000.0).await;
        finished_match(
            &app,
            &finished,
            &a,
            &b,
            json!({ "status": "live", "finishedAt": null }),
        )
        .await;

        let (code, refused) = offer(&app, &token_a, &finished, STAKE_NORMAL).await;
        assert_eq!(code, 422);
        assert_eq!(refused["error"]["code"], "match_not_finished");
    }

    #[tokio::test(start_paused = true)]
    async fn a_series_game_is_refused_whatever_its_status() {
        let app = test_app().await;
        let (a, b, finished) = names("series-game");
        let token_a = active_profile(&app, &a, 1000.0).await;
        active_profile(&app, &b, 1000.0).await;
        finished_match(&app, &finished, &a, &b, json!({})).await;
        let trio = |name: &str| {
            json!({
                "name": name,
                "decks": [
                    { "name": "one", "cards": [] },
                    { "name": "two", "cards": [] },
                    { "name": "three", "cards": [] },
                ],
            })
        };
        let now = now_ms();
        q!(
            app,
            series_create(&from(json!({
                "id": "series-game-series",
                "sides": [
                    { "profileId": a, "trio": trio("a"), "wins": 0, "pick": null },
                    { "profileId": b, "trio": trio("b"), "wins": 0, "pick": null },
                ],
                "catalogVersion": app.catalog.version,
                "seedBase": "seed-base",
                "status": "playing",
                "games": [{ "gameNo": 1, "matchId": finished, "slots": [0, 0], "first": "p1", "winner": null, "reason": null }],
                "nextMatchId": finished,
                "pickDeadline": null,
                "winner": null,
                "endReason": null,
                "ratingBefore": null,
                "ratingAfter": null,
                "createdAt": now,
                "updatedAt": now,
                "endedAt": null,
                "version": 0,
            })))
        );

        let (code, refused) = offer(&app, &token_a, &finished, STAKE_NORMAL).await;
        assert_eq!(code, 422);
        assert_eq!(refused["error"]["code"], "series_game");

        // Reading the status is refused the same way, so the death screen never polls one into view.
        let (code, read) = status(&app, &token_a, &finished).await;
        assert_eq!(code, 422);
        assert_eq!(read["error"]["code"], "series_game");
    }

    #[tokio::test(start_paused = true)]
    async fn a_bad_stakes_value_is_a_400() {
        let app = test_app().await;
        let (a, b, finished) = names("bad-stakes");
        let token_a = active_profile(&app, &a, 1000.0).await;
        active_profile(&app, &b, 1000.0).await;
        finished_match(&app, &finished, &a, &b, json!({})).await;

        let (code, _) = offer(&app, &token_a, &finished, 3).await;
        assert_eq!(code, 400);
    }
}

// Presence

mod rematch_presence {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn reports_the_opponents_offer_only_while_their_socket_is_open_via_the_status() {
        let app = test_app().await;
        let (a, b, finished) = names("presence-status");
        let token_a = active_profile(&app, &a, 1000.0).await;
        let token_b = active_profile(&app, &b, 1000.0).await;

        // No live actor: nobody is here.
        let gone = format!("{finished}-no-actor");
        finished_match(&app, &gone, &a, &b, json!({})).await;
        assert_eq!(status(&app, &token_a, &gone).await.1["opponentHere"], false);

        // A live actor with both seats' sockets open, whose game is then over.
        start_live(&app, &finished, &a, &b).await;
        let (p1, _p1_frames) = Socket::channel();
        let (p2, _p2_frames) = Socket::channel();
        app.matches
            .attach(&app, &finished, &a, p1.clone())
            .await
            .expect("A attaches");
        app.matches
            .attach(&app, &finished, &b, p2.clone())
            .await
            .expect("B attaches");
        q!(app, matches_finish(&finished, now_ms()));
        settle().await;
        assert_eq!(status(&app, &token_a, &finished).await.1["opponentHere"], true);

        // Only the opponent's seat counts: mine open alone is still gone.
        p2.transport_closed();
        settle().await;
        assert_eq!(status(&app, &token_b, &finished).await.1["opponentHere"], true);
        assert_eq!(status(&app, &token_a, &finished).await.1["opponentHere"], false);
    }

    #[tokio::test(start_paused = true)]
    async fn the_registrys_presence_follows_the_live_actors_sockets_and_dies_with_it() {
        let app = test_app().await;
        let (a, b, _) = names("presence-registry");
        {
            let mut data = fake(&app).await;
            data.seed_profile(json!({ "id": a }));
            data.seed_profile(json!({ "id": b }));
        }
        let match_id = "presence-registry-match";
        start_live(&app, match_id, &a, &b).await;

        // No sockets yet: neither seat is here.
        assert_eq!(
            app.matches.presence_of(match_id).map(|seats| to_json(&seats)),
            Some(json!({ "p1": false, "p2": false }))
        );
        // An id with no actor is nobody, not an empty room.
        assert!(app.matches.presence_of("no-such-match").is_none());

        let (p1, _p1_frames) = Socket::channel();
        let (p2, _p2_frames) = Socket::channel();
        app.matches
            .attach(&app, match_id, &a, p1.clone())
            .await
            .expect("A attaches");
        app.matches
            .attach(&app, match_id, &b, p2.clone())
            .await
            .expect("B attaches");
        settle().await;
        assert_eq!(
            app.matches.presence_of(match_id).map(|seats| to_json(&seats)),
            Some(json!({ "p1": true, "p2": true }))
        );

        // The opponent closes the tab: their seat reads as gone.
        p2.transport_closed();
        settle().await;
        assert_eq!(
            app.matches.presence_of(match_id).map(|seats| to_json(&seats)),
            Some(json!({ "p1": true, "p2": false }))
        );

        app.matches.stop(match_id).await;
        assert!(app.matches.presence_of(match_id).is_none());
    }
}
