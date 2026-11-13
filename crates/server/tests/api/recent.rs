//! Recently played (R3732000, issue #3732): `GET /api/friends/recent` lists the opponents of a
//! player's unranked games and never one the queue paired, so a ladder bot stays indistinguishable
//! from a person (R1452).

use std::sync::Arc;

use serde_json::{Value, json};

use jackioh_engine::{Action, ActionBody, PlayerId};
use jackioh_server::actor::clock::initial_clocks;
use jackioh_server::app::App;
use jackioh_server::config::{RECENT_OPPONENTS_MAX, RECENT_OPPONENTS_SCAN};
use jackioh_server::db::store::{MatchActionRow, MatchRow, ResultRow};

use crate::support::deps::{call, empty_test_app};
use crate::support::store::{Account, account, befriend, from, q, to_json};

/// Unit conversion, not configuration.
const MINUTE_MS: i64 = 60_000;
/// When the first seeded game ended, epoch ms; each later one ends a minute after the one before.
const FIRST_ENDED_AT: i64 = 1_760_000_000_000;

/// A finished game between `p1` and `p2` that ended `minutes` after `FIRST_ENDED_AT`: `ranked` is the
/// queue's mark (R604), absent on a room, lobby, challenge or daily game.
async fn finished(app: &Arc<App>, id: &str, p1: &Account, p2: &Account, ranked: bool, minutes: i64) {
    let ended_at = FIRST_ENDED_AT + minutes * MINUTE_MS;
    let created_at = ended_at - MINUTE_MS / 2;
    let mut row = json!({
        "id": id,
        "seed": format!("seed-{id}"),
        "players": [p1.id, p2.id],
        "decks": [[], []],
        "catalogVersion": jackioh_cards::catalog_version(),
        "status": "live",
        "createdAt": created_at,
        "finishedAt": null,
        "clocks": to_json(&initial_clocks(created_at)),
    });
    if ranked {
        row["ranked"] = json!(true);
    }
    let row: MatchRow = from(row);
    q!(app, matches_create(&row));
    let action = MatchActionRow {
        match_id: id.to_string(),
        seq: 1,
        action: Action::new(ActionBody::Concede, PlayerId::P1, format!("nonce-{id}")),
        at: created_at,
    };
    q!(app, matches_append_actions(&[action]));
    let result: ResultRow = from(json!({
        "matchId": id,
        "players": [p1.id, p2.id],
        "winnerProfileId": p2.id,
        "reason": "concede",
        "turns": 3,
        "endedAt": ended_at,
        "ratingBefore": [1000, 1000],
        "ratingAfter": [1000, 1000],
    }));
    q!(app, results_insert(&result));
    q!(app, matches_finish(id, ended_at));
}

/// `GET /api/friends/recent` for `who`: its `recent` list.
async fn recent(app: &Arc<App>, who: &Account) -> Vec<Value> {
    let (status, _headers, body) =
        call(app, "GET", "/api/friends/recent", Some(&who.token), Value::Null).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body.as_object().map(|fields| fields.len()), Some(1), "{body}");
    body["recent"].as_array().expect("a recent list").clone()
}

/// The `profileId`s of a recent list, in its order.
fn ids(list: &[Value]) -> Vec<String> {
    list.iter()
        .map(|entry| entry["profileId"].as_str().expect("a profile id").to_string())
        .collect()
}

#[tokio::test]
async fn r3732000_lists_room_lobby_and_daily_opponents_and_never_a_queue_one() {
    let app = empty_test_app().await;
    let ada = account(&app, 1, "Ada").await;
    let room = account(&app, 2, "Room").await;
    let lobby = account(&app, 3, "Lobby").await;
    let queue = account(&app, 4, "Queue").await;
    let daily = account(&app, 5, "Daily").await;

    finished(&app, "m-room", &ada, &room, false, 1).await;
    // A seat the other way round: the list reads the seat Ada did not begin in.
    finished(&app, "m-lobby", &lobby, &ada, false, 2).await;
    finished(&app, "m-queue", &ada, &queue, true, 3).await;
    finished(&app, "m-daily", &ada, &daily, false, 4).await;

    let list = recent(&app, &ada).await;

    // Newest first, and the queue's opponent in none of it.
    assert_eq!(
        ids(&list),
        vec![daily.id.clone(), lobby.id.clone(), room.id.clone()],
        "{list:?}"
    );
    assert_eq!(list[0]["username"], json!(daily.name));
    assert!(list[0]["playedAt"].is_string(), "{:?}", list[0]);
    assert_eq!(
        list[0].as_object().map(|fields| fields.len()),
        Some(3),
        "a profile id, a name and a time, and nothing of the game: {:?}",
        list[0]
    );
    assert!(!ids(&list).contains(&queue.id));
    // The queue's opponent sees the same: Ada is in none of their list.
    assert!(recent(&app, &queue).await.is_empty());
}

#[tokio::test]
async fn r3732000_leaves_out_self_friends_and_blocks() {
    let app = empty_test_app().await;
    let ada = account(&app, 1, "Ada").await;
    let friend = account(&app, 2, "Fay").await;
    let blocked = account(&app, 3, "Blo").await;
    let blocker = account(&app, 4, "Bla").await;
    let stranger = account(&app, 5, "Sam").await;

    finished(&app, "m-friend", &ada, &friend, false, 1).await;
    finished(&app, "m-blocked", &ada, &blocked, false, 2).await;
    finished(&app, "m-blocker", &blocker, &ada, false, 3).await;
    finished(&app, "m-stranger", &ada, &stranger, false, 4).await;
    // A game against oneself (a test fixture's shape) never lists the caller.
    finished(&app, "m-self", &ada, &ada, false, 5).await;
    befriend(&app, &ada, &friend).await;
    for (who, whom) in [(&ada, &blocked), (&blocker, &ada)] {
        let (status, _headers, body) = call(
            &app,
            "POST",
            "/api/blocks",
            Some(&who.token),
            json!({ "profileId": whom.id }),
        )
        .await;
        assert_eq!(status, 204, "{body}");
    }

    assert_eq!(ids(&recent(&app, &ada).await), vec![stranger.id.clone()]);
}

#[tokio::test]
async fn r3732000_lists_each_opponent_once_at_their_newest_game() {
    let app = empty_test_app().await;
    let ada = account(&app, 1, "Ada").await;
    let bea = account(&app, 2, "Bea").await;
    let cy = account(&app, 3, "Cy").await;

    finished(&app, "m-1", &ada, &bea, false, 1).await;
    finished(&app, "m-2", &ada, &cy, false, 2).await;
    finished(&app, "m-3", &bea, &ada, false, 3).await;

    let list = recent(&app, &ada).await;

    assert_eq!(ids(&list), vec![bea.id.clone(), cy.id.clone()], "{list:?}");
    assert_eq!(
        list[0]["playedAt"],
        json!(jackioh_server::social::iso(FIRST_ENDED_AT + 3 * MINUTE_MS)),
        "Bea's newest game ended at minute 3"
    );
}

#[tokio::test]
async fn r3732000_caps_the_list_newest_first() {
    let app = empty_test_app().await;
    let ada = account(&app, 1, "Ada").await;
    let games = RECENT_OPPONENTS_MAX + 5;
    assert!(
        i64::try_from(games).expect("a small count") <= RECENT_OPPONENTS_SCAN,
        "premise: every game is inside the scan"
    );
    let mut opponents: Vec<Account> = Vec::new();
    for n in 0..games {
        let opponent = account(
            &app,
            u32::try_from(n + 2).expect("a small id"),
            &format!("Opp{n}"),
        )
        .await;
        finished(
            &app,
            &format!("m-{n}"),
            &ada,
            &opponent,
            false,
            i64::try_from(n).expect("a small n"),
        )
        .await;
        opponents.push(opponent);
    }

    let list = recent(&app, &ada).await;

    let newest: Vec<String> = opponents
        .iter()
        .rev()
        .take(RECENT_OPPONENTS_MAX)
        .map(|opponent| opponent.id.clone())
        .collect();
    assert_eq!(ids(&list), newest);
}
