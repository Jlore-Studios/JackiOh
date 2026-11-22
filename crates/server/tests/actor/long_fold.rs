//! #1035: whole log's fold runs on tokio's blocking pool (SPEC §9.3, §9.5).

use std::sync::Arc;

use serde_json::{Value, json};

use jackioh_engine::config::Difficulty;
use jackioh_engine::duos::DuosReplayInput;
use jackioh_engine::{Action, GameState, PlayerId, reduce};
use jackioh_server::actor::clock::initial_clocks;
use jackioh_server::actor::engine::hash_state;
use jackioh_server::actor::registry::AttachError;
use jackioh_server::app::{App, now_ms};
use jackioh_server::db::store::{Db, MatchActionRow, MatchRow};
use jackioh_server::duos::engine::{self as duos_engine, Pinned, TeamBChoices};
use jackioh_server::duos::registry::{DuosAttachError, registry as duos_registry};
use jackioh_server::duos::store::{DuosChoice, DuosMatch};

use crate::support::deps::{call, empty_test_app};
use crate::support::engine::{
    FOLD_GATE_WAIT, FoldGate, TEST_FOLD_GATE, begun_game, fake_deck, hand_card, install_gated_test_cards,
    install_test_cards, keep_whole_hand, past_the_mulligans,
};
use crate::support::store::from;

const MATCH_ID: &str = "match-long-fold";

/// The co-op match `#4430`'s tests rebuild.
const DUOS_MATCH_ID: &str = "match-duos-long-fold";

/// How many seeds `gate_seed` tries before it gives up.
const SEED_SEARCH: usize = 5_000;

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
        "clocks": serde_json::to_value(initial_clocks(created_at)).expect("serializes"),
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

/// A live co-op match row as a join writes one: two people against the easy AI team, its decks as
/// given.
fn duos_row(seed: &str, a1: Vec<String>, a2: Vec<String>) -> DuosMatch {
    let random = DuosChoice::Random { lean: None };
    let mut options = duos_engine::match_options(
        Pinned::default(),
        jackioh_cards::catalog_version(),
        seed,
        TeamBChoices::Ai(Difficulty::Easy),
        &random,
        &random,
    );
    options.decks.a1 = a1;
    options.decks.a2 = a2;
    DuosMatch {
        id: DUOS_MATCH_ID.to_string(),
        seed: seed.to_string(),
        a1: "profile-1".to_string(),
        a2: "profile-2".to_string(),
        b1: String::new(),
        b2: String::new(),
        difficulty: Difficulty::Easy,
        options,
        created_at: now_ms(),
        ended_at: None,
        result: None,
    }
}

/// The co-op row written straight into the fake store.
async fn write_duos_match(app: &App, row: &DuosMatch) {
    let mut tx = app.db.begin(None).await.expect("begin");
    tx.duos_matches_create(row).await.expect("duos_matches_create");
    tx.commit().await.expect("commit");
}

/// #4430: a co-op rebuild folds off the runtime (#1035), so a panic in the fold is an internal
/// error and registers no actor, as a 1v1 rebuild's is.
#[tokio::test]
async fn a_duos_rebuild_whose_fold_panics_is_an_internal_error_and_registers_no_actor() {
    install_test_cards();
    let app = empty_test_app().await;
    let bad = vec!["not-a-card".to_string()];
    write_duos_match(&app, &duos_row("duos-long-fold-panics", bad.clone(), bad)).await;

    match duos_registry(&app).actor_for(&app, DUOS_MATCH_ID).await {
        Err(DuosAttachError::Internal(message)) => {
            assert!(message.contains("deck must hold exactly"), "{message}");
        }
        Err(other) => panic!("an internal error was expected, not {other}"),
        Ok(_) => panic!("a fold that panics registered an actor"),
    }
    assert!(duos_registry(&app).live().is_empty());
    let (status, _headers, _body) = call(&app, "GET", "/healthz", None, Value::Null).await;
    assert_eq!(status, 200);
}

/// #4430: a co-op rebuild runs its fold through `app.engine_thread`, so the test harness's card
/// overrides apply on the worker thread: decks of test-only cards fold to what a direct fold
/// reads. On bare `spawn_blocking` the worker thread holds no overrides and the fold panics.
#[tokio::test]
async fn a_duos_rebuild_installs_the_test_cards_before_folding() {
    install_test_cards();
    let app = empty_test_app().await;
    let row = duos_row("duos-long-fold-test-cards", fake_deck(&[]), fake_deck(&[]));
    write_duos_match(&app, &row).await;

    let actor = duos_registry(&app)
        .actor_for(&app, DUOS_MATCH_ID)
        .await
        .expect("the rebuilt actor");
    actor.idle().await;
    assert_eq!(actor.next_seq(), 1);
    let folded = duos_engine::fold(
        Pinned::default(),
        &DuosReplayInput {
            options: duos_engine::create_options(&row),
            log: Vec::new(),
        },
    );
    assert_eq!(folded.refused_at, None, "the empty log folds whole");
    assert_eq!(actor.hash(), duos_engine::hash(&folded.state));
}
