//! R388, R767000 (#1085): a live 1v1 match is played, rebuilt and reaped under the catalog
//! version its row froze when it started, as Duos is. A start on a version this build neither is
//! nor carries is refused; a rebuild of such a match logs `match.catalog.unpinned` and plays on
//! this build's own catalog.

use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::{Value, json};

use jackioh_engine::testkit::{clear_overrides, register_catalog, register_shipped};
use jackioh_engine::{
    Action, ActionBody, CardCost, CardDefs, PLAYER_IDS, PlayerId, ReplayOpen, ReplayRecord, ReplayRefusal,
    ShippedCatalog,
};
use jackioh_server::actor::engine::{Pinned, fold, fold_args, hash_state, legal_actions, replay_open};
use jackioh_server::actor::match_actor::{MatchActor, last_boards_of};
use jackioh_server::app::App;
use jackioh_server::db::store::{Db, MatchRow, StartMatchInput};

use crate::support::deps::{empty_test_app, record_logs};
use crate::support::engine::{FILLER_PREFIX, fake_deck, hand_ids, install_test_cards, test_catalog};
use crate::support::socket::create_fake_socket;

const MATCH_ID: &str = "match-pinned";
const P1: &str = "profile-1";
const P2: &str = "profile-2";
const SEED: &str = "pinned-seed";
/// The catalog version the matches below started on: carried by this thread's build, not its own.
const PLAYED_ON: &str = "v-pinned";
/// A newer build's price for every filler: past what any opening hand can pay (R388's patch).
const PRICED_OUT: i32 = jackioh_engine::config::MAX_MANA + 1;

/// The next build's catalog: every filler repriced past any mana it could meet.
fn priced_out() -> CardDefs {
    let mut defs = test_catalog();
    for def in defs.values_mut() {
        if def.id.starts_with(FILLER_PREFIX) {
            def.cost = CardCost::Fixed(PRICED_OUT);
        }
    }
    defs
}

/// The deploy the tests play under: the cards as they shipped, and the next build's.
fn deploy() {
    install_test_cards();
    register_shipped(vec![ShippedCatalog::new(
        PLAYED_ON,
        test_catalog(),
        IndexMap::new(),
    )]);
    register_catalog(priced_out());
}

/// The test app over the fake store, both profiles seeded, as `dealt_deck.rs` seeds them.
async fn world() -> Arc<App> {
    jackioh_cards::register_all();
    let app = empty_test_app().await;
    for profile_id in [P1, P2] {
        let Db::Fake(data) = &app.db else {
            panic!("the test app runs on the fake store")
        };
        data.lock()
            .await
            .seed_profile(json!({ "id": profile_id, "rating": 1000 }));
    }
    app
}

/// `dealt_deck.rs`'s start, with `fake_deck(&[])` for both seats, on `version`.
fn start_input(version: &str) -> StartMatchInput {
    serde_json::from_value(json!({
        "matchId": MATCH_ID,
        "seed": SEED,
        "catalogVersion": version,
        "ranked": false,
        "seats": [
            { "profileId": P1, "player": "p1", "deck": fake_deck(&[]) },
            { "profileId": P2, "player": "p2", "deck": fake_deck(&[]) },
        ],
    }))
    .expect("a StartMatchInput")
}

/// Each of `seat`'s hand costs, as its own view shows them (§10.8).
fn hand_costs(actor: &MatchActor, seat: PlayerId) -> Vec<Value> {
    serde_json::to_value(actor.view_for(seat)).expect("PlayerView serialises")["you"]["hand"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|card| card["cost"].clone())
        .collect()
}

/// Both seats keep their whole opening hands (R265), through the actor's own `submit`.
async fn keep_both(actor: &MatchActor) {
    for seat in PLAYER_IDS {
        let body = ActionBody::Mulligan {
            keep: hand_ids(&actor.engine_state(), seat),
        };
        let reply = actor.submit(seat, format!("keep-{seat}"), body).await;
        let reply = serde_json::to_value(&reply).expect("ServerMessage serialises");
        assert_eq!(reply["type"], json!("ack"), "the {seat} mulligan: {reply}");
    }
}

/// The match row and its logged actions in one transaction.
async fn match_log(app: &Arc<App>) -> (MatchRow, Vec<Action>) {
    let mut tx = app.db.begin(None).await.expect("begin");
    let row = tx
        .matches_get(MATCH_ID)
        .await
        .expect("read")
        .expect("a match row");
    let log = tx.matches_actions(MATCH_ID).await.expect("read");
    tx.commit().await.expect("commit");
    (row, log.into_iter().map(|entry| entry.action).collect())
}

#[tokio::test(start_paused = true)]
async fn r388_a_live_match_plays_on_the_catalog_it_started_on() {
    deploy();
    let app = world().await;
    app.matches
        .start(&app, start_input(PLAYED_ON))
        .await
        .expect("the match starts");
    let actor = app.matches.actor_for(&app, MATCH_ID).await.expect("the actor");
    // `view_for` reads the shipped catalog: every filler costs nothing, as it shipped.
    for seat in PLAYER_IDS {
        assert!(
            hand_costs(&actor, seat).iter().all(|cost| *cost == json!(0)),
            "a priced hand under the shipped catalog"
        );
    }
    keep_both(&actor).await;
    let seat = actor.engine_state().active;

    let socket = create_fake_socket();
    actor.attach(seat, socket.socket());
    actor.idle().await;
    // `legal_actions` reads it too: the view frame lists a play of a 0-cost filler.
    let legal = socket.of_type("view").last().cloned().expect("a view frame")["legal"]
        .as_array()
        .cloned()
        .expect("the view frame's legal array");
    let play = legal
        .iter()
        .find(|entry| entry["type"] == "play")
        .cloned()
        .expect("a play in legal");
    // Under this build's own (priced) catalog no play is legal: the two legal sets differ.
    assert!(
        legal_actions(Pinned::default(), &actor.engine_state(), seat)
            .iter()
            .all(|body| !matches!(body, ActionBody::Play { .. })),
        "a play is still legal when priced out"
    );
    // And `reduce` reads it: the shipped play is accepted.
    let body: ActionBody = serde_json::from_value(play).expect("an ActionBody");
    let reply = actor.submit(seat, "pinned-play".to_string(), body).await;
    let reply = serde_json::to_value(&reply).expect("ServerMessage serialises");
    assert_eq!(reply["type"], json!("ack"), "the pinned play: {reply}");
    clear_overrides();
}

#[tokio::test(start_paused = true)]
async fn r388_r767000_a_rebuilt_match_folds_under_the_catalog_it_started_on_as_its_replay_does() {
    deploy();
    let app = world().await;
    app.matches
        .start(&app, start_input(PLAYED_ON))
        .await
        .expect("the match starts");
    let actor = app.matches.actor_for(&app, MATCH_ID).await.expect("the actor");
    keep_both(&actor).await;
    let seat = actor.engine_state().active;
    let pinned = Pinned::of(PLAYED_ON).expect("the played-on catalog");
    let body = legal_actions(pinned, &actor.engine_state(), seat)
        .into_iter()
        .find(|body| matches!(body, ActionBody::Play { .. }))
        .expect("a play in legal");
    let reply = actor.submit(seat, "pinned-play".to_string(), body).await;
    assert_eq!(
        serde_json::to_value(&reply).expect("ServerMessage serialises")["type"],
        json!("ack"),
        "the pinned play: {reply:?}"
    );
    let live = hash_state(&actor.engine_state());

    app.matches.stop(MATCH_ID).await;
    let rebuilt = app
        .matches
        .actor_for(&app, MATCH_ID)
        .await
        .expect("the rebuilt actor");
    assert_eq!(hash_state(&rebuilt.engine_state()), live);

    let (row, log) = match_log(&app).await;
    let (last_boards, glitch_boards) = last_boards_of(&row);
    let args = fold_args(&row.seed, &row.decks, log, last_boards, glitch_boards, None);
    assert!(
        matches!(
            replay_open(
                &args,
                &ReplayRecord {
                    catalog_version: PLAYED_ON.into(),
                    final_hash: live.clone(),
                },
            ),
            ReplayOpen::Ready { .. }
        ),
        "the replay opens under the started-on catalog"
    );
    // What the rebuild folded before this change: under this build's own catalog.
    assert_ne!(hash_state(&fold(Pinned::default(), &args).state), live);
    clear_overrides();
}

#[tokio::test(start_paused = true)]
async fn r388_a_start_on_a_catalog_this_build_does_not_carry_is_refused_and_a_rebuild_of_one_plays_on_this_builds_own()
 {
    deploy();
    let logs = record_logs();
    assert_eq!(Pinned::of("v-gone").err(), Some(ReplayRefusal::EarlierPatch));
    let app = world().await;
    assert!(app.matches.start(&app, start_input("v-gone")).await.is_err());
    assert!(!app.matches.has(MATCH_ID));
    {
        let mut tx = app.db.begin(None).await.expect("begin");
        assert!(tx.matches_get(MATCH_ID).await.expect("read").is_none());
        tx.commit().await.expect("commit");
    }

    app.matches
        .start(&app, start_input(PLAYED_ON))
        .await
        .expect("the match starts");
    app.matches.stop(MATCH_ID).await;
    // The next build no longer carries the started-on catalog.
    register_shipped(Vec::new());
    let rebuilt = app
        .matches
        .actor_for(&app, MATCH_ID)
        .await
        .expect("the rebuilt actor");
    let unpinned = logs.named("match.catalog.unpinned");
    assert_eq!(unpinned.len(), 1);
    assert_eq!(unpinned[0].level, "alert");
    assert_eq!(unpinned[0].data["matchId"], json!(MATCH_ID));
    assert_eq!(unpinned[0].data["catalogVersion"], json!(PLAYED_ON));
    assert_eq!(
        unpinned[0].data["reason"],
        json!(ReplayRefusal::EarlierPatch.to_string())
    );
    // The rebuild plays on this build's own catalog: every filler repriced.
    for seat in PLAYER_IDS {
        assert!(
            hand_costs(&rebuilt, seat)
                .iter()
                .all(|cost| *cost == json!(PRICED_OUT)),
            "a shipped hand under the repriced catalog"
        );
    }
    clear_overrides();
}
