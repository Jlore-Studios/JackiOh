//! #1169: a live match's checkpoint (SPEC §9.3, §9.5). The restart drain (`Registry::stop_all`)
//! stores one for a live match of at least `MATCH_CHECKPOINT_MIN_ACTIONS` actions, and the next
//! rebuild resumes from it, folding only the rows after it, to exactly what a whole log's fold gives:
//! the state and its hash, the open prompt, the seq counter and the nonce map, the clocks, both views
//! and the play telemetry. A checkpoint that fails any check is refused, with nothing but its reason
//! logged, and the whole log is folded. Ending the match drops it; the log keeps every row, and the
//! replay (R768) folds them all to the recorded final hash.
//!
//! The log is a real-card game the engine's random policy plays (§10.7), written straight into the
//! fake store as a crashed process would have left it.

use serde_json::{Value, json};

use jackioh_engine::subsystems::ai_policy::choose_action;
use jackioh_engine::{Action, ActionBody, PerPlayer, PlayerId, Rng, fnv1a32_utf16, reduce, seat_to_act};
use jackioh_server::actor::checkpoint::{CheckpointBody, RULES_FINGERPRINT, checkpoint_row, verify};
use jackioh_server::actor::clock::initial_clocks;
use jackioh_server::actor::engine::{EngineState, Pinned, fold, fold_args, hash_state};
use jackioh_server::actor::match_actor::{MatchActor, bot_moved_turns, bot_moved_turns_from};
use jackioh_server::actor::protocol::SERVER_NONCE_PREFIX;
use jackioh_server::actor::telemetry::{LIVE_PILOTS, Recorder, replay, replay_from};
use jackioh_server::app::{App, now_ms};
use jackioh_server::config::{MATCH_CHECKPOINT_MIN_ACTIONS, MATCH_CHECKPOINT_VERSION};
use jackioh_server::db::store::{MatchActionRow, MatchCheckpointRow, MatchRow};

use crate::support::deps::{RecordingLogger, add_user, call, empty_test_app, record_logs};
use crate::support::engine::{begun_game, decks_the_engine_accepts, install_test_cards, real_pool};
use crate::support::store::{fake_ref, to_json};

const MATCH_ID: &str = "match-checkpoints";
/// The rows logged before the drain.
const LOGGED: usize = 40;
/// The rows logged after it, before the next rebuild.
const TAIL: usize = 6;
/// How many seeds `game` tries for one still on at the length it needs.
const SEED_SEARCH: usize = 200;
/// How many whole games `every_state_…` checkpoints, and the most moves it plays of each.
const WHOLE_GAMES: usize = 3;
const WHOLE_GAME_CAP: usize = 400;

type Decks = (Vec<String>, Vec<String>);

fn from<T: serde::de::DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value.clone()).unwrap_or_else(|error| panic!("{error}: {value}"))
}

/// A live match row, as the registry writes one.
fn live_row(seed: &str, decks: &Decks) -> MatchRow {
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

/// The policy's first `length` moves under `seed`, nonces `cp-<seq>`, or `None` when the game ends
/// before them or right on the last.
fn policy_log(seed: &str, decks: &Decks, length: usize) -> Option<Vec<Action>> {
    let mut state = begun_game(seed, decks.clone());
    let mut policy = Rng::new(&format!("{seed}:policy"), 0);
    let mut log: Vec<Action> = Vec::new();
    while log.len() < length {
        if state.result.is_some() {
            return None;
        }
        let seat = seat_to_act(&state).unwrap_or(state.active);
        let body = choose_action(&state, seat, &mut policy)?;
        let action = Action::new(body, seat, format!("cp-{}", log.len() + 1));
        let result = reduce(&state, &action);
        if let Some(error) = result.error {
            panic!("the policy's move was refused under {seed}: {error}");
        }
        state = result.state;
        log.push(action);
    }
    state.result.is_none().then_some(log)
}

/// A seed whose game is still on after `length` moves, its decks and those moves.
fn game(length: usize) -> (String, Decks, Vec<Action>) {
    let decks = decks_the_engine_accepts(&real_pool(), "checkpoints").1;
    (0..SEED_SEARCH)
        .map(|k| format!("checkpoints-{k}"))
        .find_map(|seed| policy_log(&seed, &decks, length).map(|log| (seed, decks.clone(), log)))
        .expect("a seed whose game outlasts the log")
}

/// `actions` as the log's rows from `first` on, stamped with the match's creation.
fn rows_of(row: &MatchRow, first: usize, actions: &[Action]) -> Vec<MatchActionRow> {
    actions
        .iter()
        .enumerate()
        .map(|(n, action)| MatchActionRow {
            match_id: row.id.clone(),
            seq: i64::try_from(first + n).expect("a small seq"),
            action: action.clone(),
            at: row.created_at,
        })
        .collect()
}

/// The row written straight into the fake store with both profiles; its log is `append`'s.
async fn write_match(app: &App, row: &MatchRow) {
    for n in [1, 2] {
        fake_ref(app).lock().await.seed_profile(json!({
            "id": format!("profile-{n}"),
            "userId": format!("user-{n}"),
            "status": "active",
            "username": format!("P{n}"),
            "rating": 1000,
        }));
    }
    let mut tx = app.db.begin(None).await.expect("begin");
    tx.matches_create(row).await.expect("matches_create");
    tx.commit().await.expect("commit");
}

async fn append(app: &App, rows: &[MatchActionRow]) {
    let mut tx = app.db.begin(None).await.expect("begin");
    tx.matches_append_actions(rows)
        .await
        .expect("matches_append_actions");
    tx.commit().await.expect("commit");
}

async fn checkpoints(app: &App) -> Vec<MatchCheckpointRow> {
    fake_ref(app).lock().await.tables.match_checkpoints.clone()
}

async fn set_checkpoint(app: &App, checkpoint: Option<MatchCheckpointRow>) {
    fake_ref(app).lock().await.tables.match_checkpoints = checkpoint.into_iter().collect();
}

async fn logged(app: &App) -> Vec<MatchActionRow> {
    let mut tx = app.db.begin(None).await.expect("begin");
    let rows = tx.matches_actions(MATCH_ID).await.expect("matches_actions");
    tx.commit().await.expect("commit");
    rows
}

async fn actor_of(app: &std::sync::Arc<App>) -> MatchActor {
    app.matches
        .actor_for(app, MATCH_ID)
        .await
        .expect("the rebuilt actor")
}

/// The seq the newest rebuild resumed after (`match.rebuilt`'s `fromSeq`).
fn from_seq(logs: &RecordingLogger) -> Value {
    logs.named("match.rebuilt").last().expect("a rebuild").data["fromSeq"].clone()
}

/// Everything a rebuilt actor shows: its state (the open prompt included) and its hash, both seats'
/// views, its clocks and its play telemetry.
#[derive(Debug, PartialEq)]
struct Seen {
    state: Value,
    hash: String,
    views: (Value, Value),
    clocks: Value,
    telemetry: Value,
}

async fn seen(actor: &MatchActor) -> Seen {
    actor.idle().await;
    let state = actor.engine_state();
    Seen {
        state: to_json(&state),
        hash: hash_state(&state),
        views: (
            to_json(&actor.view_for(PlayerId::P1)),
            to_json(&actor.view_for(PlayerId::P2)),
        ),
        clocks: to_json(&actor.clocks()),
        telemetry: to_json(&actor.telemetry()),
    }
}

/// The state `actions` fold to from the row's setup, as `engine::fold` folds a whole log.
fn folded(row: &MatchRow, actions: &[Action]) -> EngineState {
    fold(
        Pinned::default(),
        &fold_args(&row.seed, &row.decks, actions.to_vec(), None, None, None),
    )
    .state
}

/// The answer a logged nonce gets sent again: its original ack.
async fn ack_of(actor: &MatchActor, nonce: &str) -> Value {
    to_json(
        &actor
            .submit(PlayerId::P1, nonce.to_string(), ActionBody::EndTurn)
            .await,
    )
}

/// A live match whose `LOGGED` rows are written, rebuilt and drained: its checkpoint stored.
async fn drained(app: &std::sync::Arc<App>) -> (MatchRow, Vec<Action>) {
    let (seed, decks, actions) = game(LOGGED + TAIL + 1);
    let row = live_row(&seed, &decks);
    write_match(app, &row).await;
    append(app, &rows_of(&row, 1, &actions[..LOGGED])).await;
    actor_of(app).await;
    assert_eq!(app.matches.stop_all().await, 1);
    (row, actions)
}

#[tokio::test(start_paused = true)]
async fn a_drained_match_rebuilds_from_its_checkpoint_as_a_full_fold_rebuilds_it() {
    install_test_cards();
    let logs = record_logs();
    let app = empty_test_app().await;
    let (row, actions) = drained(&app).await;

    let stored = checkpoints(&app).await;
    assert_eq!(stored.len(), 1, "one checkpoint for the drained match");
    let checkpoint = &stored[0];
    assert_eq!(checkpoint.match_id, MATCH_ID);
    assert_eq!(checkpoint.seq, LOGGED as i64);
    assert_eq!(checkpoint.nonce, format!("cp-{LOGGED}"));
    assert_eq!(checkpoint.catalog_version, jackioh_cards::catalog_version());
    assert_eq!(checkpoint.rules, RULES_FINGERPRINT);
    assert_eq!(checkpoint.checkpoint_version, MATCH_CHECKPOINT_VERSION);
    assert_eq!(checkpoint.body_hash, fnv1a32_utf16(&checkpoint.body));
    assert_eq!(logs.named("match.checkpoint.written").len(), 1);

    append(&app, &rows_of(&row, LOGGED + 1, &actions[LOGGED..LOGGED + TAIL])).await;
    let resumed = actor_of(&app).await;
    assert_eq!(from_seq(&logs), json!(LOGGED));
    let from_checkpoint = seen(&resumed).await;

    app.matches.stop(MATCH_ID).await;
    set_checkpoint(&app, None).await;
    let whole = actor_of(&app).await;
    assert_eq!(from_seq(&logs), json!(0));
    let from_the_log = seen(&whole).await;

    assert_eq!(from_checkpoint, from_the_log);
    assert_eq!(
        from_checkpoint.hash,
        hash_state(&folded(&row, &actions[..LOGGED + TAIL]))
    );
    assert!(logs.named("match.checkpoint.refused").is_empty());
    assert!(logs.named("match.fold.errors").is_empty());
    // §9.3: the nonce map and the seq counter are the whole log's on both.
    for actor in [&resumed, &whole] {
        assert_eq!(
            ack_of(actor, "cp-1").await,
            json!({ "type": "ack", "nonce": "cp-1", "seq": 1 })
        );
        let last = format!("cp-{}", LOGGED + TAIL);
        assert_eq!(
            ack_of(actor, &last).await,
            json!({ "type": "ack", "nonce": last, "seq": LOGGED + TAIL })
        );
    }
    assert_eq!(logged(&app).await.len(), LOGGED + TAIL);
}

#[tokio::test(start_paused = true)]
async fn a_rebuilt_match_carries_its_sequence_on_and_the_next_drain_moves_its_checkpoint_up() {
    install_test_cards();
    let logs = record_logs();
    let app = empty_test_app().await;
    let (row, actions) = drained(&app).await;
    append(&app, &rows_of(&row, LOGGED + 1, &actions[LOGGED..LOGGED + TAIL])).await;
    let resumed = actor_of(&app).await;
    assert_eq!(from_seq(&logs), json!(LOGGED));

    let next = &actions[LOGGED + TAIL];
    let seq = LOGGED + TAIL + 1;
    assert_eq!(
        to_json(
            &resumed
                .submit(next.player_id, next.nonce.clone(), next.body.clone())
                .await
        ),
        json!({ "type": "ack", "nonce": next.nonce, "seq": seq })
    );
    assert_eq!(app.matches.stop_all().await, 1);
    let stored = checkpoints(&app).await;
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].seq, seq as i64);
    assert_eq!(stored[0].nonce, next.nonce);

    let again = actor_of(&app).await;
    assert_eq!(from_seq(&logs), json!(seq));
    assert_eq!(
        hash_state(&again.engine_state()),
        hash_state(&folded(&row, &actions[..seq]))
    );
}

#[tokio::test(start_paused = true)]
async fn a_checkpoint_that_fails_a_check_is_refused_and_the_whole_log_is_folded() {
    install_test_cards();
    let logs = record_logs();
    let app = empty_test_app().await;
    let (_row, _actions) = drained(&app).await;
    let good = checkpoints(&app).await.pop().expect("the drained checkpoint");

    set_checkpoint(&app, None).await;
    let reference = seen(&actor_of(&app).await).await;
    assert_eq!(from_seq(&logs), json!(0));

    let rehashed = |body: String| MatchCheckpointRow {
        body_hash: fnv1a32_utf16(&body),
        body,
        ..good.clone()
    };
    let poisons: Vec<(&str, MatchCheckpointRow)> = vec![
        (
            "version",
            MatchCheckpointRow {
                checkpoint_version: MATCH_CHECKPOINT_VERSION + 1,
                ..good.clone()
            },
        ),
        (
            "rules",
            MatchCheckpointRow {
                rules: "0000000000000000".to_string(),
                ..good.clone()
            },
        ),
        (
            "catalog",
            MatchCheckpointRow {
                catalog_version: "v0.0.1-old".to_string(),
                ..good.clone()
            },
        ),
        (
            "seq",
            MatchCheckpointRow {
                seq: LOGGED as i64 + 5,
                ..good.clone()
            },
        ),
        (
            "nonce",
            MatchCheckpointRow {
                nonce: "not-logged".to_string(),
                ..good.clone()
            },
        ),
        (
            "hash",
            MatchCheckpointRow {
                body_hash: "00000000".to_string(),
                ..good.clone()
            },
        ),
        ("body", rehashed("{".to_string())),
        ("shape", rehashed(good.body.replacen('{', "{\"stale\":1,", 1))),
    ];
    for (n, (reason, poison)) in poisons.into_iter().enumerate() {
        app.matches.stop(MATCH_ID).await;
        let seq = poison.seq;
        set_checkpoint(&app, Some(poison)).await;
        let rebuilt = seen(&actor_of(&app).await).await;
        assert_eq!(rebuilt, reference, "a checkpoint refused for its {reason}");
        assert_eq!(from_seq(&logs), json!(0), "{reason}");
        let refused = logs.named("match.checkpoint.refused");
        assert_eq!(refused.len(), n + 1, "{reason}");
        let entry = refused.last().expect("the refusal");
        assert_eq!(entry.level, "warn");
        assert_eq!(
            entry.data,
            json!({ "matchId": MATCH_ID, "seq": seq, "reason": reason }),
            "only the reason, never the body"
        );
    }
}

#[tokio::test(start_paused = true)]
async fn a_short_match_writes_no_checkpoint() {
    install_test_cards();
    let logs = record_logs();
    let app = empty_test_app().await;
    let short = usize::try_from(MATCH_CHECKPOINT_MIN_ACTIONS - 1).expect("a small count");
    let (seed, decks, actions) = game(short);
    let row = live_row(&seed, &decks);
    write_match(&app, &row).await;
    append(&app, &rows_of(&row, 1, &actions)).await;
    actor_of(&app).await;
    assert_eq!(app.matches.stop_all().await, 1);
    assert!(checkpoints(&app).await.is_empty());
    assert!(logs.named("match.checkpoint.written").is_empty());
}

#[tokio::test(start_paused = true)]
async fn a_match_ending_after_a_checkpoint_rebuild_drops_it_and_its_replay_folds_the_whole_log() {
    install_test_cards();
    let logs = record_logs();
    let app = empty_test_app().await;
    let (row, actions) = drained(&app).await;
    append(&app, &rows_of(&row, LOGGED + 1, &actions[LOGGED..LOGGED + TAIL])).await;
    let resumed = actor_of(&app).await;
    assert_eq!(from_seq(&logs), json!(LOGGED));

    let seat = seat_to_act(&resumed.engine_state()).expect("a seat to act");
    let conceded = resumed
        .submit(seat, "cp-concede".to_string(), ActionBody::Concede)
        .await;
    assert_eq!(
        to_json(&conceded),
        json!({ "type": "ack", "nonce": "cp-concede", "seq": LOGGED + TAIL + 1 })
    );
    resumed.idle().await;
    assert!(resumed.engine_state().result.is_some());
    assert!(checkpoints(&app).await.is_empty(), "the ending drops it");
    let seqs: Vec<i64> = logged(&app).await.iter().map(|row| row.seq).collect();
    assert_eq!(seqs, (1..=(LOGGED + TAIL + 1) as i64).collect::<Vec<i64>>());

    // R768: the replay folds every row to the final hash the result recorded, or it answers 409.
    let token = add_user(&app, "user-1", "user-1@example.test", true);
    let (status, _headers, body) = call(
        &app,
        "GET",
        &format!("/api/replays/{MATCH_ID}"),
        Some(&token),
        Value::Null,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["total"], json!(LOGGED + TAIL + 2), "{body}");
}

#[tokio::test(start_paused = true)]
async fn a_recorder_read_back_from_a_checkpoint_carries_on_as_replaying_the_whole_log() {
    install_test_cards();
    let _logs = record_logs();
    let (seed, decks, actions) = game(LOGGED);
    let row = live_row(&seed, &decks);
    let rows = rows_of(&row, 1, &actions);
    let at = 20;

    let saved: Recorder = serde_json::from_str(
        &serde_json::to_string(&replay(Pinned::default(), &row, &rows[..at], None).0).expect("serialises"),
    )
    .expect("reads back");
    let resumed = replay_from(
        Pinned::default(),
        saved,
        folded(&row, &actions[..at]),
        &rows[at..],
    );
    let whole = replay(Pinned::default(), &row, &rows, None);
    assert_eq!(resumed.1, 0);
    assert_eq!(whole.1, 0);
    assert_eq!(
        resumed.0.finish(&LIVE_PILOTS, None),
        whole.0.finish(&LIVE_PILOTS, None)
    );
    assert!(!whole.0.finish(&LIVE_PILOTS, None).action_timings.is_empty());
}

#[tokio::test(start_paused = true)]
async fn bot_turns_carried_on_from_a_checkpoint_equal_the_whole_logs() {
    install_test_cards();
    let _logs = record_logs();
    let (seed, decks, actions) = game(LOGGED);
    let row = live_row(&seed, &decks);
    // R1452: p2 is a bot: its moves are the server's `srv-bot-…` actions.
    let actions: Vec<Action> = actions
        .into_iter()
        .enumerate()
        .map(|(n, action)| match action.player_id {
            PlayerId::P2 => Action {
                nonce: format!("{SERVER_NONCE_PREFIX}bot-{}", n + 1),
                ..action
            },
            PlayerId::P1 => action,
        })
        .collect();
    let rows = rows_of(&row, 1, &actions);
    let at = 20;

    let saved = bot_moved_turns(Pinned::default(), &row, &rows[..at], None);
    let resumed = bot_moved_turns_from(
        Pinned::default(),
        folded(&row, &actions[..at]),
        &rows[at..],
        saved,
    );
    let whole = bot_moved_turns(Pinned::default(), &row, &rows, None);
    assert_eq!(resumed, whole);
    assert!(whole.p2.is_some());
}

/// Every state a game passes through, with its recorder and bot turns, is written as a checkpoint
/// that reads back to its own text, so `verify` passes it: a body this build cannot write back
/// byte for byte would be refused as `shape` on every rebuild.
#[tokio::test(start_paused = true)]
async fn every_state_of_a_whole_game_checkpoints_and_passes_its_checks() {
    install_test_cards();
    let _logs = record_logs();
    let decks = decks_the_engine_accepts(&real_pool(), "checkpoints").1;
    for k in 0..WHOLE_GAMES {
        let seed = format!("checkpoints-whole-{k}");
        let row = live_row(&seed, &decks);
        let mut state = begun_game(&seed, decks.clone());
        let mut policy = Rng::new(&format!("{seed}:policy"), 0);
        let mut recorder = replay(Pinned::default(), &row, &[], None).0;
        let mut rows: Vec<MatchActionRow> = Vec::new();
        while state.result.is_none() && rows.len() < WHOLE_GAME_CAP {
            let seat = seat_to_act(&state).unwrap_or(state.active);
            let body = choose_action(&state, seat, &mut policy).expect("a move");
            let seq = rows.len() + 1;
            let action = Action::new(body, seat, format!("cp-{seq}"));
            rows.extend(rows_of(&row, seq, std::slice::from_ref(&action)));
            recorder = replay_from(Pinned::default(), recorder, state.clone(), &rows[seq - 1..]).0;
            let result = reduce(&state, &action);
            assert!(result.error.is_none(), "{seed} at seq {seq}: {:?}", result.error);
            state = result.state;

            let body = CheckpointBody {
                state: state.clone(),
                telemetry: recorder.clone(),
                bot_moved_turn: PerPlayer::new(None, Some(state.turn)),
            };
            let checkpoint = checkpoint_row(&row, seq as i64, &action.nonce, &body, row.created_at)
                .expect("a checkpoint serialises");
            let read = verify(&checkpoint, &row, &rows).unwrap_or_else(|refusal| {
                panic!("{seed} at seq {seq}: refused for its {}", refusal.as_str())
            });
            assert_eq!(hash_state(&read.state), hash_state(&state), "{seed} at seq {seq}");
        }
    }
}
