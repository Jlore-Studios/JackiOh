//! #1035: a whole log's fold runs on tokio's blocking pool, not on the async runtime (SPEC §9.3,
//! §9.5), as Duos's rebuild and the bot seat already run theirs.
//!
//! `TEST_FOLD_GATE`'s play parks the fold that replays it on a channel (`FoldGate`), and the test
//! asks `GET /healthz` while it is parked. `#[tokio::test]` is a current-thread runtime, so a fold
//! that held it could not answer: the probe only runs once the fold has ended, and the recorded order
//! says so. The card waits with a bounded `recv_timeout`, so that failure is an order and not a hang.
//! A fold that panics is still the rebuild's internal error.

use std::sync::Arc;

use serde_json::{Value, json};

use jackioh_engine::{Action, GameState, PlayerId, reduce};
use jackioh_server::actor::clock::initial_clocks;
use jackioh_server::actor::engine::hash_state;
use jackioh_server::actor::registry::AttachError;
use jackioh_server::app::{App, now_ms};
use jackioh_server::db::store::{Db, MatchActionRow, MatchRow};

use crate::support::deps::{call, empty_test_app};
use crate::support::engine::{
    FOLD_GATE_WAIT, FoldGate, TEST_FOLD_GATE, begun_game, fake_deck, hand_card, install_gated_test_cards,
    install_test_cards, keep_whole_hand, past_the_mulligans,
};

const MATCH_ID: &str = "match-long-fold";

/// How many seeds `gate_seed` tries before it gives up.
const SEED_SEARCH: usize = 5_000;

fn from<T: serde::de::DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value.clone()).unwrap_or_else(|error| panic!("{error}: {value}"))
}

/// A live match row, as the registry writes one.
fn live_row(seed: &str, decks: &(Vec<String>, Vec<String>)) -> MatchRow {
    let created_at = now_ms();
    from(json!({
        "id": MATCH_ID,
        "seed": seed,
        "players": ["profile-1", "profile-2"],
        "decks": [decks.0, decks.1],
        "catalogVersion": jackioh_cards::catalog_version(),
        "status": "live",
        "createdAt": created_at,
        "finishedAt": null,
        "clocks": serde_json::to_value(initial_clocks(created_at)).expect("serialises"),
    }))
}

/// The row and its log written straight into the fake store, both profiles seeded.
async fn write_match(app: &App, row: &MatchRow, log: &[Action]) {
    let Db::Fake(data) = &app.db else {
        panic!("the test app runs on the fake store")
    };
    for id in ["profile-1", "profile-2"] {
        data.lock()
            .await
            .seed_profile(json!({ "id": id, "rating": 1000 }));
    }
    let rows: Vec<MatchActionRow> = log
        .iter()
        .enumerate()
        .map(|(n, action)| MatchActionRow {
            match_id: row.id.clone(),
            seq: i64::try_from(n + 1).expect("a small seq"),
            action: action.clone(),
            at: row.created_at,
        })
        .collect();
    let mut tx = app.db.begin(None).await.expect("begin");
    tx.matches_create(row).await.expect("matches_create");
    tx.matches_append_actions(&rows)
        .await
        .expect("matches_append_actions");
    tx.commit().await.expect("commit");
}

/// The first seed that deals `TEST_FOLD_GATE` to p1.
fn gate_seed(decks: &(Vec<String>, Vec<String>)) -> String {
    (0..SEED_SEARCH)
        .map(|k| format!("long-fold-{k}"))
        .find(|seed| {
            hand_card(
                &past_the_mulligans(&begun_game(seed, decks.clone())),
                PlayerId::P1,
                TEST_FOLD_GATE,
            )
            .is_some()
        })
        .expect("a seed that deals the gate to p1")
}

/// Both mulligans keeping the whole hand, then p1 playing the gate: the log, and the state it ends on.
fn gate_log(seed: &str, decks: &(Vec<String>, Vec<String>)) -> (Vec<Action>, GameState) {
    let mut state = begun_game(seed, decks.clone());
    let mut log = Vec::new();
    for (seat, nonce) in [(PlayerId::P1, "mulligan-p1"), (PlayerId::P2, "mulligan-p2")] {
        let keep = keep_whole_hand(&state, seat, nonce);
        let result = reduce(&state, &keep);
        assert!(
            result.error.is_none(),
            "the mulligan was refused: {:?}",
            result.error
        );
        state = result.state;
        log.push(keep);
    }
    let card = hand_card(&state, PlayerId::P1, TEST_FOLD_GATE).expect("the gate in hand");
    let play = Action::new(
        from(json!({ "type": "play", "instanceId": card })),
        PlayerId::P1,
        "play-gate",
    );
    let result = reduce(&state, &play);
    assert!(
        result.error.is_none(),
        "the gate's play was refused: {:?}",
        result.error
    );
    log.push(play);
    (log, result.state)
}

#[tokio::test]
async fn a_rebuild_folds_off_the_runtime_so_healthz_answers_while_it_folds() {
    let (gate, release, parked) = FoldGate::new();
    install_gated_test_cards(Arc::clone(&gate));
    let app = empty_test_app().await;
    let decks = (fake_deck(&[TEST_FOLD_GATE]), fake_deck(&[]));
    let seed = gate_seed(&decks);
    let (log, state) = gate_log(&seed, &decks);
    write_match(&app, &live_row(&seed, &decks), &log).await;
    gate.arm();

    let probe = async {
        tokio::time::timeout(FOLD_GATE_WAIT, parked)
            .await
            .expect("the fold parks in time")
            .expect("the fold reports it parked");
        let (status, _headers, _body) = call(&app, "GET", "/healthz", None, Value::Null).await;
        assert_eq!(status, 200);
        gate.note("healthz");
        release.send(()).expect("the fold still waits");
    };
    let (actor, ()) = tokio::join!(app.matches.actor_for(&app, MATCH_ID), probe);
    let actor = actor.expect("the rebuilt actor");

    assert_eq!(
        gate.order(),
        vec!["healthz", "fold"],
        "the fold held the runtime: /healthz waited for it"
    );
    assert_eq!(hash_state(&actor.engine_state()), hash_state(&state));
}

#[tokio::test]
async fn a_rebuild_whose_fold_panics_is_an_internal_error_and_registers_no_actor() {
    install_test_cards();
    let app = empty_test_app().await;
    let decks = (vec!["not-a-card".to_string()], vec!["not-a-card".to_string()]);
    write_match(&app, &live_row("long-fold-panics", &decks), &[]).await;

    match app.matches.actor_for(&app, MATCH_ID).await {
        Err(AttachError::Internal(message)) => {
            assert!(message.contains("deck must hold exactly"), "{message}");
        }
        Err(other) => panic!("an internal error was expected, not {other}"),
        Ok(_) => panic!("a fold that panics registered an actor"),
    }
    assert!(app.matches.live().is_empty());
    let (status, _headers, _body) = call(&app, "GET", "/healthz", None, Value::Null).await;
    assert_eq!(status, 200);
}
