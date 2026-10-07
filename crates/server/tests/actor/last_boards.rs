//! Last boards on the server (C+ #29 Portal to the Past, R417, R565; BUILD M9-T11: "a finished game
//! writes both seats' last boards, never the opponent's face-down cards").
//!
//! The first block is the real engine under the real registry, actor and results writer, over the
//! fake store: a real game ends with a face-down card on the field, each seat's stored board is
//! exactly the field its own final view showed, and the next match between the same profiles
//! starts from those boards, frozen on its row, so a rebuilt actor folds the same game. The second
//! block is the wiring the real game cannot reach on demand, straight through the results writer.
//!
//! Port of `apps/server/test/match/last-boards.test.ts`. TS's `enginePort()` is the engine itself
//! here (SURFACE §11.3: the server calls `jackioh_engine` directly), and the second block's
//! scripted port, which it never drove, is not needed at all.

use std::sync::Arc;

use serde_json::{json, Value};

use jackioh_engine::wire::PlayerId;
use jackioh_engine::LastBoardEntry;
use jackioh_server::actor::match_actor::MatchActor;
use jackioh_server::api::results::{reap_stuck_matches, record_result};
use jackioh_server::app::App;
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{Db, StoreError};

use crate::support::deps::test_app;
use crate::support::engine::fake_deck;

const P1: &str = "profile-1";
const P2: &str = "profile-2";

/// The epoch-ms stamp the rows below are written at; the results writer only stores it.
const AT: i64 = 1_700_000_000_000;

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

/// TS's `store.onCall`: every store method named `method` fails with `message` until cleared.
async fn fail_on(app: &App, method: &'static str, message: &'static str) {
    fake(app).lock().await.on_call = Some(Arc::new(move |called: &str| {
        if called == method { Err(StoreError::Other(message.to_string())) } else { Ok(()) }
    }));
}

async fn stop_failing(app: &App) {
    fake(app).lock().await.on_call = None;
}

/// A seat's own hand, as its view lists it.
fn hand_of(view: &Value) -> Vec<Value> {
    view["you"]["hand"].as_array().cloned().unwrap_or_else(|| panic!("a seat's own hand is a list"))
}

/// What a seat's own view shows of the field, p1's side then p2's, in board order (R417).
fn shown_field(view: &Value) -> Vec<Value> {
    let mut sides = vec![&view["you"], &view["opponent"]];
    sides.sort_by(|a, b| a["player"].as_str().cmp(&b["player"].as_str()));
    let mut field = Vec::new();
    for side in sides {
        for unit in side["units"].as_array().into_iter().flatten() {
            if !unit.is_null() {
                field.push(json!({ "defId": unit["defId"], "radiant": unit["radiant"] }));
            }
        }
        for (lane, card) in side["backrow"].as_array().into_iter().flatten().enumerate() {
            let carried = &side["carried"][lane];
            if !carried.is_null() {
                field.push(json!({ "defId": carried["defId"], "radiant": carried["radiant"] }));
            }
            if !card.is_null() && card["faceDown"] != json!(true) {
                field.push(json!({ "defId": card["defId"], "radiant": card["radiant"] }));
            }
        }
    }
    field
}

/// The test app with both profiles seeded (TS's `world(engine)`; the engine needs no wiring).
async fn world() -> Arc<App> {
    let app = test_app().await;
    let data = fake(&app);
    let mut data = data.lock().await;
    data.seed_profile(json!({ "id": P1, "rating": 1000 }));
    data.seed_profile(json!({ "id": P2, "rating": 1000 }));
    drop(data);
    app
}

async fn start(app: &Arc<App>, match_id: &str, seed: &str, decks: &(Vec<String>, Vec<String>)) {
    app.matches
        .start(
            app,
            from(json!({
                "matchId": match_id,
                "seed": seed,
                "catalogVersion": jackioh_cards::catalog_version(),
                "ranked": false,
                "seats": [
                    { "profileId": P1, "player": "p1", "deck": decks.0 },
                    { "profileId": P2, "player": "p2", "deck": decks.1 },
                ],
            })),
        )
        .await
        .expect("the registry starts the match");
}

async fn view_of(actor: &MatchActor, player: PlayerId) -> Value {
    to_json(&actor.view_for(player))
}

/// TS's `submit`: one action as `player`, under the next `lb-<n>` nonce; a refusal fails the test.
struct Submitter {
    actor: MatchActor,
    n: u32,
}

impl Submitter {
    async fn submit(&mut self, player: PlayerId, body: Value) {
        self.n += 1;
        let reply = to_json(&self.actor.submit(player, &format!("lb-{}", self.n), from(body.clone())).await);
        if reply["type"] == "error" {
            panic!("{} by {player}: {reply}", body["type"]);
        }
    }
}

/// Cheap Traps that fire only on what the walk below never does — a lethal attack, a death of p1's
/// Unit, a hit to 0, a heal, a Spell — so the one p1 sets stays face-down to the end.
const QUIET_TRAPS: [&str; 5] = ["core-096", "classic-014", "classic-052", "classicplus-022", "classic-017"];

mod r417_r565_last_boards_through_a_real_match {
    use super::*;

    #[tokio::test]
    async fn r565_a_finished_game_writes_both_seats_last_boards_each_the_field_its_own_view_showed_never_the_opponents_face_down_cards_the_next_match_starts_from_them_and_a_rebuild_folds_the_same_game()
     {
        jackioh_cards::register_all();
        let catalog = &*jackioh_cards::CATALOG;
        let core: Vec<String> = catalog
            .iter()
            .filter(|(_, def)| {
                !def.token
                    && !to_json(&def.tags).as_array().is_some_and(|tags| tags.contains(&json!("Token")))
                    && to_json(&def.set) == "Core"
            })
            .map(|(id, _)| id.clone())
            .collect();
        let mut first: Vec<String> = QUIET_TRAPS.iter().map(|id| id.to_string()).collect();
        first.extend(core[..15].iter().cloned());
        let decks = (first, core[15..35].to_vec());
        let app = world().await;
        start(&app, "m-last-1", "last-boards-real", &decks).await;
        let actor = app.matches.actor_for(&app, "m-last-1").await.expect("the match's actor");

        let mut walk = Submitter { actor: actor.clone(), n: 0 };
        let type_of = |view: &Value, instance_id: &Value| -> Option<String> {
            let card = hand_of(view).into_iter().find(|entry| &entry["instanceId"] == instance_id)?;
            let def = catalog.get(card["defId"].as_str()?)?;
            to_json(&def.type_).as_str().map(str::to_string)
        };

        for player in [PlayerId::P1, PlayerId::P2] {
            let keep: Vec<Value> = hand_of(&view_of(&actor, player).await).iter().map(|card| card["instanceId"].clone()).collect();
            walk.submit(player, json!({ "type": "mulligan", "keep": keep })).await;
        }
        // p1 sets a quiet Trap, p2 plays a Unit; everything else ends the turn or answers the prompt.
        let mut step = 0;
        loop {
            if step > 80 {
                panic!("the walk never put a face-down card and a Unit on the field");
            }
            step += 1;
            let seen = view_of(&actor, PlayerId::P2).await;
            let face_down = seen["opponent"]["backrow"].as_array().is_some_and(|row| row.iter().any(|card| card["faceDown"] == json!(true)));
            let unit = seen["you"]["units"].as_array().is_some_and(|row| row.iter().any(|pile| !pile.is_null()));
            if face_down && unit {
                break;
            }
            let snapshot = actor.snapshot();
            assert!(snapshot.result.is_none());
            let who = snapshot.pending_for.unwrap_or(snapshot.active);
            let legal: Vec<Value> = jackioh_engine::legal_actions(&actor.engine_state(), who).iter().map(to_json).collect();
            let own_view = view_of(&actor, who).await;
            let wanted = |want: &[&str]| -> Option<Value> {
                legal
                    .iter()
                    .find(|action| {
                        action["type"] == "play"
                            && type_of(&own_view, &action["instanceId"]).is_some_and(|kind| want.contains(&kind.as_str()))
                    })
                    .cloned()
            };
            let body = legal
                .iter()
                .find(|action| action["type"] == "answer")
                .cloned()
                .or_else(|| if who == PlayerId::P1 && !face_down { wanted(&["Trap", "Field Trap"]) } else { None })
                .or_else(|| if who == PlayerId::P2 && !unit { wanted(&["Unit"]) } else { None })
                .or_else(|| legal.iter().find(|action| action["type"] == "endTurn").cloned())
                .unwrap_or_else(|| panic!("{who} has nothing to do"));
            walk.submit(who, body).await;
        }
        walk.submit(PlayerId::P2, json!({ "type": "concede" })).await;
        actor.idle().await;

        let own = to_json(&must(store!(app, t => t.last_boards_get(P1, from(json!("server"))).await.expect("lastBoards.get"))));
        let theirs = to_json(&must(store!(app, t => t.last_boards_get(P2, from(json!("server"))).await.expect("lastBoards.get"))));
        let p1_view = view_of(&actor, PlayerId::P1).await;
        assert_eq!(own, json!(shown_field(&p1_view)));
        assert_eq!(theirs, json!(shown_field(&view_of(&actor, PlayerId::P2).await)));
        let trap = p1_view["you"]["backrow"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|card| !card.is_null() && card["faceDown"] == json!(false) && card["unrevealed"] == json!(true))
            .cloned()
            .unwrap_or_else(|| panic!("p1 holds no face-down card"));
        let ids = |board: &Value| -> Vec<Value> {
            board.as_array().into_iter().flatten().map(|entry| entry["defId"].clone()).collect()
        };
        assert!(ids(&own).contains(&trap["defId"]));
        assert!(!ids(&theirs).contains(&trap["defId"]));
        assert!(ids(&theirs).len() < ids(&own).len());

        // The next match between them starts from those boards, frozen on its row (R417).
        start(&app, "m-last-2", "last-boards-next", &decks).await;
        let row = store!(app, t => t.matches_get("m-last-2").await.expect("matches.get")).expect("the next match's row");
        assert_eq!(to_json(&row)["lastBoards"], json!([own, theirs]));
        let next = app.matches.actor_for(&app, "m-last-2").await.expect("the next match's actor");
        let mut next_walk = Submitter { actor: next.clone(), n: walk.n };
        for player in [PlayerId::P1, PlayerId::P2] {
            let keep: Vec<Value> = hand_of(&view_of(&next, player).await).iter().map(|card| card["instanceId"].clone()).collect();
            next_walk.submit(player, json!({ "type": "mulligan", "keep": keep })).await;
        }
        let live = jackioh_engine::hash_state(&next.engine_state());
        // A newer board stored meanwhile changes nothing: a rebuilt actor folds the match's own inputs.
        let empty: Vec<LastBoardEntry> = Vec::new();
        store!(app, t => t.last_boards_put(P1, from(json!("server")), &empty, AT as _).await.expect("lastBoards.put"));
        app.matches.stop("m-last-2").await;
        let rebuilt = app.matches.actor_for(&app, "m-last-2").await.expect("the rebuilt actor");
        assert_eq!(jackioh_engine::hash_state(&rebuilt.engine_state()), live);
    }
}

fn must<T>(value: Option<T>) -> T {
    value.unwrap_or_else(|| panic!("expected a value"))
}

mod r565_the_results_writer_and_the_boards {
    use super::*;

    fn board_1() -> Value {
        json!([{ "defId": "core-012", "radiant": true }])
    }

    fn board_2() -> Value {
        json!([{ "defId": "core-025", "radiant": false }])
    }

    async fn live_match(app: &Arc<App>, id: &str, ceiling_at: i64) {
        let row = from(json!({
            "id": id,
            "seed": "s",
            "players": [P1, P2],
            "decks": [fake_deck(&[]), fake_deck(&[])],
            "catalogVersion": jackioh_cards::catalog_version(),
            "ranked": false,
            "status": "live",
            "createdAt": AT,
            "finishedAt": null,
            "clocks": { "turnDeadline": null, "promptDeadline": null, "graceDeadline": { "p1": null, "p2": null }, "ceilingAt": ceiling_at },
        }));
        store!(app, t => t.matches_create(&row).await.expect("matches.create"));
    }

    fn input(match_id: &str, at: i64) -> Value {
        json!({
            "matchId": match_id,
            "seats": [
                { "profileId": P1, "player": "p1", "deck": fake_deck(&[]) },
                { "profileId": P2, "player": "p2", "deck": fake_deck(&[]) },
            ],
            "outcome": { "winner": "p1", "reason": "concede" },
            "turns": 3,
            "at": at,
            "lastBoards": [board_1(), board_2()],
        })
    }

    async fn board_of(app: &Arc<App>, profile: &str) -> Option<Value> {
        store!(app, t => t.last_boards_get(profile, from(json!("server"))).await.expect("lastBoards.get")).map(|board| to_json(&board))
    }

    #[tokio::test]
    async fn r565_writes_each_seats_board_once_in_the_results_transaction_a_failed_board_write_leaves_no_result() {
        let app = world().await;
        live_match(&app, "m-tx", AT + 60_000).await;
        fail_on(&app, "lastBoards.put", "injected").await;
        let failed = record_result(&app, from(input("m-tx", AT))).await.expect_err("the board write fails the result");
        assert!(format!("{failed:?}").contains("injected"), "{failed:?}");
        assert!(store!(app, t => t.results_get_by_match("m-tx").await.expect("results.getByMatch")).is_none());
        stop_failing(&app).await;

        record_result(&app, from(input("m-tx", AT))).await.expect("the result lands");
        assert_eq!(board_of(&app, P1).await, Some(board_1()));
        assert_eq!(board_of(&app, P2).await, Some(board_2()));
        // A second write of the same ending is the first one's row and touches no board.
        let empty: Vec<LastBoardEntry> = Vec::new();
        store!(app, t => t.last_boards_put(P1, from(json!("server")), &empty, AT as _).await.expect("lastBoards.put"));
        record_result(&app, from(input("m-tx", AT))).await.expect("the repeat is the first row");
        assert_eq!(board_of(&app, P1).await, Some(json!([])));
    }

    #[tokio::test]
    async fn r565_r112_the_reaper_reads_no_state_so_a_reaped_ceiling_draw_writes_no_board_and_the_last_one_stays() {
        let app = world().await;
        let first: Vec<LastBoardEntry> = from(board_1());
        store!(app, t => t.last_boards_put(P1, from(json!("server")), &first, AT as _).await.expect("lastBoards.put"));
        // A ceiling long past (TS: one millisecond ago).
        live_match(&app, "m-reaped", 0).await;
        assert_eq!(reap_stuck_matches(&app).await.expect("the reaper runs"), vec!["m-reaped".to_string()]);
        assert!(store!(app, t => t.results_get_by_match("m-reaped").await.expect("results.getByMatch")).is_some());
        assert_eq!(board_of(&app, P1).await, Some(board_1()));
        assert_eq!(board_of(&app, P2).await, None);
    }
}
