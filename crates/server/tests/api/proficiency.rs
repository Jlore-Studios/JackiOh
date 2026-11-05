//! Card proficiency and stamps (`src/economy/proficiency.rs`, `src/api/proficiency.rs`, SPEC §14.14,
//! #3013). Counting a game's plays in the payout's transaction (R3013000), and the stamp picks a card's
//! level unlocks, the dev fill among them (R3013001).

use std::sync::Arc;

use jackioh_engine::PlayerId;
use jackioh_engine::set_ships;
use jackioh_engine::wire::{CreatedUse, SeatSummary};
use serde_json::{Value, json};

use jackioh_server::api::http::AuthLevel;
use jackioh_server::app::{self, App, now_ms};
use jackioh_server::config::{PROFICIENCY_LEVEL_PLAYS, PROFICIENCY_LEVELS};
use jackioh_server::db::fake::OnCall;
use jackioh_server::db::store::StoreError;
use jackioh_server::economy::proficiency::{proficiency_plays, top_level, top_level_plays};
use jackioh_server::economy::quests::GameTally;
use jackioh_server::economy::rewards::{GameMode, GameSummary, pay_game_rewards};
use jackioh_server::economy::store::CardProficiency;

use crate::support::deps::{call, test_app as built_app};
use crate::support::store::{fake, store};

/// The caller's bearer token and managed-auth user id (`e2e/support/config.ts`'s `e2e-p1`).
const TOKEN: &str = "e2e-token-p1";
const USER: &str = "e2e-p1";
/// The other active account (`e2e-p2`).
const OTHER_TOKEN: &str = "e2e-token-p2";
const OTHER_USER: &str = "e2e-p2";
/// The pending account (`e2e-pending`), which no `Active` route answers.
const PENDING_TOKEN: &str = "e2e-token-pending";

/// Two cards of the catalog, no token.
const X: &str = "core-008";
const Y: &str = "core-009";

/// The test app with the economy on (§14.12), as `rewards.rs`'s.
async fn test_app() -> Arc<App> {
    let mut app = built_app().await;
    Arc::get_mut(&mut app)
        .expect("the test app has no second handle yet")
        .economy_live = true;
    app
}

async fn profile_id_of(app: &App, user_id: &str) -> String {
    let mut tx = app.db.begin(None).await.expect("a store transaction");
    let profile = tx
        .profiles_get_by_user_id(user_id)
        .await
        .expect("profiles.getByUserId")
        .unwrap_or_else(|| panic!("the fixture account {user_id} has a profile"));
    tx.commit().await.expect("commit");
    profile.id
}

/// A token of the catalog.
fn token_of(app: &App) -> String {
    app.catalog
        .card_ids
        .iter()
        .find(|id| app.catalog.is_token(id))
        .cloned()
        .expect("the catalog ships a token")
}

fn row(card_id: &str, plays: i64, level: i64, stamp: Option<i64>) -> CardProficiency {
    CardProficiency {
        card_id: card_id.to_owned(),
        plays,
        level,
        stamp,
    }
}

async fn rows_of(app: &App, profile_id: &str) -> Vec<CardProficiency> {
    store!(app, |t| t.economy_card_proficiency_get(profile_id))
}

/// `profile_id` holds `rows`, written as a game's pay writes them.
async fn hold(app: &App, profile_id: &str, rows: &[CardProficiency]) {
    store!(app, |t| t.economy_card_proficiency_put(
        profile_id,
        rows,
        now_ms()
    ));
    for held in rows {
        if let Some(stamp) = held.stamp {
            assert!(store!(app, |t| t.economy_card_proficiency_stamp(
                profile_id,
                &held.card_id,
                Some(stamp),
                now_ms()
            )));
        }
    }
}

/// One finished game of `mode` for `profile_id`, counting `plays`.
fn game(mode: GameMode, profile_id: &str, player: PlayerId, plays: Vec<String>) -> GameSummary {
    GameSummary {
        mode,
        profile_id: profile_id.to_owned(),
        player,
        won: Some(true),
        actions: 4,
        match_id: "match-3013".to_owned(),
        at: now_ms(),
        bot: false,
        tally: GameTally::default(),
        proficiency: plays,
    }
}

fn ids(cards: &[&str]) -> Vec<String> {
    cards.iter().map(|card| (*card).to_owned()).collect()
}

async fn get(app: &Arc<App>, bearer: Option<&str>) -> (u16, Value) {
    let (status, _, body) = call(app, "GET", "/api/proficiency", bearer, Value::Null).await;
    (status, body)
}

async fn put(app: &Arc<App>, bearer: &str, body: Value) -> (u16, Value) {
    let (status, _, body) = call(app, "PUT", "/api/proficiency/stamp", Some(bearer), body).await;
    (status, body)
}

/// A refused write's message, after checking it is a 400 `bad_request`.
#[track_caller]
fn refusal((status, body): (u16, Value)) -> String {
    assert_eq!(status, 400, "{body}");
    assert_eq!(body["error"]["code"], "bad_request", "{body}");
    body["error"]["message"].as_str().unwrap_or_default().to_owned()
}

/// Every store method the fake is asked to run from here on, by name, in order
/// (`tests/api/card_picks.rs`'s).
async fn record_store_calls(app: &App) -> Arc<std::sync::Mutex<Vec<String>>> {
    let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = Arc::clone(&calls);
    let hook: OnCall = Arc::new(move |method: &str| {
        sink.lock().expect("the call log").push(method.to_string());
        Ok(())
    });
    fake(app).lock().await.on_call = Some(hook);
    calls
}

/// The proficiency methods among `calls`, in order.
fn proficiency_calls(calls: &std::sync::Mutex<Vec<String>>) -> Vec<String> {
    calls
        .lock()
        .expect("the call log")
        .iter()
        .filter(|method| method.starts_with("economy.cardProficiency."))
        .cloned()
        .collect()
}

// Counting (R3013000)

#[tokio::test]
async fn r3013000_a_ladder_game_adds_the_seats_own_plays_and_nothing_else() {
    let app = test_app().await;
    let (a, b) = (
        profile_id_of(&app, USER).await,
        profile_id_of(&app, OTHER_USER).await,
    );
    let token = token_of(&app);
    // Seat A's fold: X played from hand three times, one of them Radiant, and a token and a Created
    // copy of X played from hand, which count for nothing.
    let seat_a = SeatSummary {
        deck: ids(&[X, X, X, Y]),
        played: ids(&[X, X, X]),
        played_radiant: Some(vec![false, true, false]),
        created_played: Some(vec![
            CreatedUse {
                card: token.clone(),
                turn: 2,
                radiant: false,
            },
            CreatedUse {
                card: X.to_owned(),
                turn: 3,
                radiant: false,
            },
        ]),
        ..SeatSummary::default()
    };
    let seat_b = SeatSummary {
        deck: ids(&[X, Y]),
        played: ids(&[X, Y]),
        ..SeatSummary::default()
    };
    let plays_a = proficiency_plays(Some(&seat_a), &app.catalog.defs);
    assert_eq!(plays_a, ids(&[X, X, X]));
    let plays_b = proficiency_plays(Some(&seat_b), &app.catalog.defs);
    {
        let mut tx = app.db.begin(None).await.expect("a store transaction");
        pay_game_rewards(&mut tx, &game(GameMode::Ladder, &a, PlayerId::P1, plays_a))
            .await
            .expect("seat A's payout");
        pay_game_rewards(&mut tx, &game(GameMode::Ladder, &b, PlayerId::P2, plays_b))
            .await
            .expect("seat B's payout");
        tx.commit().await.expect("commit");
    }
    assert_eq!(rows_of(&app, &a).await, vec![row(X, 3, 1, None)]);
    assert_eq!(
        rows_of(&app, &b).await,
        vec![row(X, 1, 0, None), row(Y, 1, 0, None)]
    );

    // Unranked and Conquest count as the ladder does; a bot's side counts nothing (R1454).
    for mode in [GameMode::Unranked, GameMode::Conquest] {
        store!(app, |t| pay_game_rewards(
            &mut t,
            &game(mode, &b, PlayerId::P2, ids(&[Y]))
        ));
    }
    assert_eq!(
        rows_of(&app, &b).await,
        vec![row(X, 1, 0, None), row(Y, 3, 1, None)]
    );
    let bot = GameSummary {
        bot: true,
        ..game(GameMode::Ladder, &b, PlayerId::P2, ids(&[X]))
    };
    store!(app, |t| pay_game_rewards(&mut t, &bot));
    assert_eq!(
        rows_of(&app, &b).await,
        vec![row(X, 1, 0, None), row(Y, 3, 1, None)]
    );
}

#[tokio::test]
async fn r3013000_a_practice_game_adds_nothing() {
    let app = test_app().await;
    let a = profile_id_of(&app, USER).await;
    // Neither a game against the AI nor a co-op game counts, whatever its summary says.
    for mode in [GameMode::Practice, GameMode::Coop] {
        let paid = store!(app, |t| pay_game_rewards(
            &mut t,
            &game(mode, &a, PlayerId::P1, ids(&[X, X, X]))
        ));
        assert!(paid.xp > 0, "{mode:?} still pays");
    }
    assert_eq!(rows_of(&app, &a).await, Vec::new());
}

#[tokio::test]
async fn r3013000_crossing_a_threshold_raises_the_level() {
    let app = test_app().await;
    let a = profile_id_of(&app, USER).await;
    let first = PROFICIENCY_LEVEL_PLAYS[0];
    let second = PROFICIENCY_LEVEL_PLAYS[1];
    hold(
        &app,
        &a,
        &[row(X, first - 1, 0, None), row(Y, second - 1, 1, Some(1))],
    )
    .await;
    let plays = ids(&[X, Y]);
    store!(app, |t| pay_game_rewards(
        &mut t,
        &game(GameMode::Ladder, &a, PlayerId::P1, plays)
    ));
    // Each crossed its next threshold; Y keeps the stamp it wears.
    assert_eq!(
        rows_of(&app, &a).await,
        vec![row(X, first, 1, None), row(Y, second, 2, Some(1))]
    );
    // A level never falls: a row already past what its plays reach keeps its level.
    hold(&app, &a, &[row(X, first, 3, None)]).await;
    store!(app, |t| pay_game_rewards(
        &mut t,
        &game(GameMode::Ladder, &a, PlayerId::P1, ids(&[X]))
    ));
    assert_eq!(rows_of(&app, &a).await[0], row(X, first + 1, 3, None));
}

#[tokio::test]
async fn r3013000_a_rolled_back_result_changes_nothing() {
    let app = test_app().await;
    let a = profile_id_of(&app, USER).await;
    let jlore = store!(app, |t| t.economy_jlore_get(&a));

    // A payout whose transaction never commits leaves no play behind.
    {
        let mut tx = app.db.begin(None).await.expect("a store transaction");
        pay_game_rewards(&mut tx, &game(GameMode::Ladder, &a, PlayerId::P1, ids(&[X])))
            .await
            .expect("the payout");
        // Dropped without a commit: the rollback.
    }
    assert_eq!(rows_of(&app, &a).await, Vec::new());
    assert_eq!(store!(app, |t| t.economy_jlore_get(&a)), jlore);

    // A count that fails fails the payout, and its Jlore goes with it.
    let hook: OnCall = Arc::new(|method: &str| {
        if method == "economy.cardProficiency.put" {
            return Err(StoreError::Other("injected".to_owned()));
        }
        Ok(())
    });
    fake(&app).lock().await.on_call = Some(hook);
    let failed = {
        let mut tx = app.db.begin(None).await.expect("a store transaction");
        pay_game_rewards(&mut tx, &game(GameMode::Ladder, &a, PlayerId::P1, ids(&[X])))
            .await
            .is_err()
    };
    fake(&app).lock().await.on_call = None;
    assert!(failed, "the injected fault fails the payout");
    assert_eq!(rows_of(&app, &a).await, Vec::new());
    assert_eq!(store!(app, |t| t.economy_jlore_get(&a)), jlore);
}

// Picking a stamp (R3013001)

#[tokio::test]
async fn r3013001_routes_are_active_only() {
    let routes: Vec<(String, bool)> = app::ROUTES
        .iter()
        .filter(|(_, path, _, _)| path.starts_with("/api/proficiency"))
        .map(|(method, path, auth, _)| (format!("{method} {path}"), matches!(auth, AuthLevel::Active)))
        .collect();
    assert_eq!(
        routes,
        vec![
            ("GET /api/proficiency".to_owned(), true),
            ("PUT /api/proficiency/stamp".to_owned(), true),
        ]
    );
    let app = test_app().await;
    assert_eq!(get(&app, None).await.0, 401);
    assert_eq!(get(&app, Some(PENDING_TOKEN)).await.0, 403);
    assert_eq!(
        put(&app, PENDING_TOKEN, json!({ "cardId": X, "stamp": null }))
            .await
            .0,
        403
    );
    assert_eq!(get(&app, Some(TOKEN)).await, (200, json!({ "cards": [] })));
}

#[tokio::test]
async fn r3013001_a_stamp_above_the_cards_level_is_400_and_writes_nothing() {
    let app = test_app().await;
    let a = profile_id_of(&app, USER).await;
    hold(&app, &a, &[row(X, PROFICIENCY_LEVEL_PLAYS[1], 2, Some(1))]).await;
    let before = rows_of(&app, &a).await;
    let calls = record_store_calls(&app).await;

    assert_eq!(
        refusal(put(&app, TOKEN, json!({ "cardId": X, "stamp": 3 })).await),
        "\"stamp\" needs level 3 of this card"
    );
    // A card never played is at level 0: every stamp is locked.
    assert_eq!(
        refusal(put(&app, TOKEN, json!({ "cardId": Y, "stamp": 1 })).await),
        "\"stamp\" needs level 1 of this card"
    );
    let top = i64::try_from(PROFICIENCY_LEVELS).expect("a few levels");
    for body in [
        json!({ "cardId": X }),
        json!({ "cardId": X, "stamp": 0 }),
        json!({ "cardId": X, "stamp": top + 1 }),
        json!({ "cardId": X, "stamp": "1" }),
        json!({ "cardId": X, "stamp": 1.5 }),
        json!({ "stamp": 1 }),
        json!({ "cardId": "core-999", "stamp": 1 }),
        json!({ "cardId": token_of(&app), "stamp": null }),
    ] {
        refusal(put(&app, TOKEN, body).await);
    }
    fake(&app).lock().await.on_call = None;
    assert!(
        !proficiency_calls(&calls)
            .iter()
            .any(|method| method == "economy.cardProficiency.stamp"),
        "{:?}",
        proficiency_calls(&calls)
    );
    assert_eq!(rows_of(&app, &a).await, before);
}

#[tokio::test]
async fn r3013001_an_unlocked_stamp_round_trips_and_clears() {
    let app = test_app().await;
    let a = profile_id_of(&app, USER).await;
    hold(&app, &a, &[row(X, PROFICIENCY_LEVEL_PLAYS[1], 2, None)]).await;

    let worn =
        json!({ "cards": [{ "cardId": X, "plays": PROFICIENCY_LEVEL_PLAYS[1], "level": 2, "stamp": 2 }] });
    assert_eq!(
        put(&app, TOKEN, json!({ "cardId": X, "stamp": 2 })).await,
        (200, worn.clone())
    );
    assert_eq!(get(&app, Some(TOKEN)).await, (200, worn));
    // Another account's read is its own.
    assert_eq!(get(&app, Some(OTHER_TOKEN)).await, (200, json!({ "cards": [] })));
    assert_eq!(
        put(&app, OTHER_TOKEN, json!({ "cardId": X, "stamp": null })).await,
        (200, json!({ "cards": [] }))
    );

    let cleared =
        json!({ "cards": [{ "cardId": X, "plays": PROFICIENCY_LEVEL_PLAYS[1], "level": 2, "stamp": null }] });
    assert_eq!(
        put(&app, TOKEN, json!({ "cardId": X, "stamp": null })).await,
        (200, cleared.clone())
    );
    assert_eq!(get(&app, Some(TOKEN)).await, (200, cleared));
    assert_eq!(
        rows_of(&app, &a).await,
        vec![row(X, PROFICIENCY_LEVEL_PLAYS[1], 2, None)]
    );
}

/// The test app with `e2e-p1` listed in `DEV_PROFILE_IDS` (`quests.rs`'s), and its profile id.
async fn dev_app() -> (Arc<App>, String) {
    let mut app = built_app().await;
    let p1 = profile_id_of(&app, USER).await;
    Arc::get_mut(&mut app)
        .expect("the test app has no second handle yet")
        .env
        .dev_profile_ids = vec![p1.clone()];
    (app, p1)
}

#[tokio::test]
async fn r3013001_the_dev_fill_sets_every_card_to_its_top_level() {
    let (app, dev) = dev_app().await;
    let past_top = top_level_plays() + 7;
    hold(&app, &dev, &[row(X, past_top, top_level(), Some(3))]).await;
    let ledger = store!(app, |t| t.economy_ledger_get(&dev));

    let (status, _, answer) = call(&app, "POST", "/api/dev/fill", Some(TOKEN), json!({})).await;
    assert_eq!(status, 200, "{answer}");
    let rows = rows_of(&app, &dev).await;
    let mut collectable: Vec<String> = app
        .catalog
        .card_ids
        .iter()
        .filter(|id| {
            !app.catalog.is_token(id)
                && !app.catalog.is_banned(id)
                && app.catalog.defs.get(*id).is_some_and(|def| set_ships(def.set))
        })
        .cloned()
        .collect();
    collectable.sort();
    assert_eq!(
        rows.iter().map(|held| held.card_id.clone()).collect::<Vec<_>>(),
        collectable
    );
    for held in &rows {
        assert_eq!(held.level, top_level(), "{held:?}");
        let expected = if held.card_id == X {
            (past_top, Some(3))
        } else {
            (top_level_plays(), None)
        };
        assert_eq!((held.plays, held.stamp), expected, "{held:?}");
    }
    // Progress, no holding: the fill writes no proficiency row to any ledger, and a second fill
    // leaves every row as it is.
    let after = store!(app, |t| t.economy_ledger_get(&dev));
    assert_eq!(after.jlore, ledger.jlore);
    let (status, _, _) = call(&app, "POST", "/api/dev/fill", Some(TOKEN), json!({})).await;
    assert_eq!(status, 200);
    assert_eq!(rows_of(&app, &dev).await, rows);
    // A topped card's every stamp is unlocked.
    let top = top_level();
    assert_eq!(
        put(&app, TOKEN, json!({ "cardId": Y, "stamp": top })).await.0,
        200
    );
}
