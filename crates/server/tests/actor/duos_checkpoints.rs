//! #4169: live co-op and 2v2 checkpoints (SPEC §9.3, §13.9). A co-op actor's rebuild resumes from the
//! checkpoint the restart drain or its periodic write stored and folds only the later rows, to what a
//! full fold of the log rebuilds; a checkpoint that fails a check is refused and the whole log folds,
//! as a 1v1 match's (`checkpoints.rs`).

use std::sync::Arc;

use serde_json::{Value, json};

use jackioh_engine::config::Difficulty;
use jackioh_engine::duos::{DuosReplayInput, DuosState, duos_seats_to_act};
use jackioh_engine::wire::{DUOS_SEATS, DuosAction, DuosActionBody, DuosSeat};
use jackioh_engine::{Rng, fnv1a32_utf16};
use jackioh_server::actor::checkpoint::RULES_FINGERPRINT;
use jackioh_server::app::{App, now_ms};
use jackioh_server::config::{MATCH_CHECKPOINT_MIN_ACTIONS, MATCH_CHECKPOINT_VERSION};
use jackioh_server::db::store::MatchCheckpointRow;
use jackioh_server::duos::actor::DuosActor;
use jackioh_server::duos::checkpoint::{DuosCheckpointBody, checkpoint_row, verify};
use jackioh_server::duos::engine::{self, Pinned, TeamBChoices};
use jackioh_server::duos::registry;
use jackioh_server::duos::store::{DuosActionRow, DuosChoice, DuosMatch, action_json};
use jackioh_shared::host::duos::ai_forbidden;

use crate::support::deps::{RecordingLogger, empty_test_app, record_logs};
use crate::support::store::{account, fail_on, fake_ref, stop_failing, to_json};

const MATCH_ID: &str = "duos-checkpoints";
/// The rows logged before the drain.
const LOGGED: usize = 40;
/// The rows logged after it, before the next rebuild.
const TAIL: usize = 6;
/// How many seeds `game` tries for one still on at the length it needs.
const SEED_SEARCH: usize = 200;
/// The most moves `a_whole_coop_game_…` plays, and every how many rows it checkpoints the state (its
/// last one too): a co-op state is some 200 KB, and each of a whole game's ~400 costs ~25 ms in a
/// debug build.
const WHOLE_GAME_CAP: usize = 600;
const WHOLE_GAME_STRIDE: usize = 6;

/// The ids of `four` (a 2v2's) or two (a co-op match's) people, each with a profile.
async fn people(app: &Arc<App>, four: bool) -> Vec<String> {
    let mut ids = Vec::new();
    for n in 1..=if four { 4 } else { 2 } {
        ids.push(account(app, n, "Duo").await.id);
    }
    ids
}

/// A live match row as a join writes one: a co-op match against the easy AI team for two people, a
/// 2v2 for four, every person's deck dealt.
fn live_row(match_id: &str, seed: &str, people: &[String]) -> DuosMatch {
    let random = DuosChoice::Random { lean: None };
    let team_b = if people.len() == 4 {
        TeamBChoices::People {
            b1: &random,
            b2: &random,
        }
    } else {
        TeamBChoices::Ai(Difficulty::Easy)
    };
    let options = engine::match_options(
        Pinned::default(),
        jackioh_cards::catalog_version(),
        seed,
        team_b,
        &random,
        &random,
    );
    let person = |n: usize| people.get(n).cloned().unwrap_or_default();
    DuosMatch {
        id: match_id.to_string(),
        seed: seed.to_string(),
        a1: person(0),
        a2: person(1),
        b1: person(2),
        b2: person(3),
        difficulty: Difficulty::Easy,
        options,
        created_at: now_ms(),
        ended_at: None,
        result: None,
    }
}

/// The policy's first `length` moves of `row`'s match, nonces `cp-<seq>`, each by the first seat the
/// game waits on, or `None` when the game ends before them or right on the last.
fn policy_log(row: &DuosMatch, length: usize) -> Option<Vec<DuosAction>> {
    let pinned = Pinned::default();
    let mut state = engine::create_and_begin(pinned, &engine::create_options(row)).expect("the match begins");
    let mut policy = Rng::new(&format!("{}:policy", row.seed), 0);
    let mut log: Vec<DuosAction> = Vec::new();
    while log.len() < length {
        if state.result.is_some() {
            return None;
        }
        let action = policy_move(&state, &mut policy, log.len() + 1)?;
        let result = engine::reduce(pinned, &state, &action);
        if let Some(error) = result.error {
            panic!("the policy's move was refused under {}: {error}", row.seed);
        }
        state = result.state;
        log.push(action);
    }
    state.result.is_none().then_some(log)
}

/// The policy's move at `seq`: one of the legal moves of the first seat the game waits on, never one
/// the AI may not make (R188).
fn policy_move(state: &DuosState, policy: &mut Rng, seq: usize) -> Option<DuosAction> {
    let seat = *duos_seats_to_act(state).first()?;
    let legal: Vec<DuosActionBody> = engine::legal(Pinned::default(), state, seat)
        .into_iter()
        .filter(|body| !ai_forbidden(body))
        .collect();
    Some(DuosAction {
        seat,
        nonce: format!("cp-{seq}"),
        body: policy.pick(&legal)?.clone(),
    })
}

/// A seed whose game is still on after `length` moves, its row and those moves.
fn game(match_id: &str, people: &[String], length: usize) -> (DuosMatch, Vec<DuosAction>) {
    (0..SEED_SEARCH)
        .map(|k| live_row(match_id, &format!("duos-checkpoints-{k}"), people))
        .find_map(|row| policy_log(&row, length).map(|log| (row, log)))
        .expect("a seed whose game outlasts the log")
}

/// `actions` as the log's rows from `first` on, stamped with the match's creation.
fn rows_of(row: &DuosMatch, first: usize, actions: &[DuosAction]) -> Vec<DuosActionRow> {
    actions
        .iter()
        .enumerate()
        .map(|(n, action)| DuosActionRow {
            match_id: row.id.clone(),
            seq: i64::try_from(first + n).expect("a small seq"),
            nonce: action.nonce.clone(),
            action: action_json(action.seat, &action.nonce, &action.body).expect("an action serializes"),
            at: row.created_at,
        })
        .collect()
}

async fn write_match(app: &App, row: &DuosMatch) {
    let mut tx = app.db.begin(None).await.expect("begin");
    tx.duos_matches_create(row).await.expect("duos_matches_create");
    tx.commit().await.expect("commit");
}

async fn append(app: &App, rows: &[DuosActionRow]) {
    let mut tx = app.db.begin(None).await.expect("begin");
    for row in rows {
        tx.duos_actions_append(row).await.expect("duos_actions_append");
    }
    tx.commit().await.expect("commit");
}

async fn checkpoints(app: &App) -> Vec<MatchCheckpointRow> {
    fake_ref(app).lock().await.tables.duos.checkpoints.clone()
}

async fn set_checkpoint(app: &App, checkpoint: Option<MatchCheckpointRow>) {
    fake_ref(app).lock().await.tables.duos.checkpoints = checkpoint.into_iter().collect();
}

async fn stored_row(app: &App, match_id: &str) -> DuosMatch {
    let mut tx = app.db.begin(None).await.expect("begin");
    let row = tx.duos_matches_get(match_id).await.expect("duos_matches_get");
    tx.commit().await.expect("commit");
    row.expect("the match row")
}

async fn actor_of(app: &Arc<App>, match_id: &str) -> DuosActor {
    registry(app)
        .actor_for(app, match_id)
        .await
        .expect("the rebuilt actor")
}

/// The seq the newest rebuild resumed after (`duos.rebuilt`'s `fromSeq`).
fn from_seq(logs: &RecordingLogger) -> Value {
    logs.named("duos.rebuilt").last().expect("a rebuild").data["fromSeq"].clone()
}

/// The state `actions` fold to from the row's setup, as a whole log's fold, which refuses none.
fn folded(row: &DuosMatch, actions: &[DuosAction]) -> DuosState {
    let result = engine::fold(
        Pinned::default(),
        &DuosReplayInput {
            options: engine::create_options(row),
            log: actions.to_vec(),
        },
    );
    assert_eq!(result.refused_at, None, "the log folds whole");
    result.state
}

fn folded_hash(row: &DuosMatch, actions: &[DuosAction]) -> String {
    engine::hash(&folded(row, actions))
}

/// Everything a rebuilt actor shows: its state's hash, the log's next seq and every seat's view.
#[derive(Debug, PartialEq)]
struct Seen {
    hash: String,
    next_seq: i64,
    views: Vec<Value>,
}

async fn seen(actor: &DuosActor) -> Seen {
    actor.idle().await;
    Seen {
        hash: actor.hash(),
        next_seq: actor.next_seq(),
        views: DUOS_SEATS
            .iter()
            .map(|seat| to_json(&actor.view_for(*seat)))
            .collect(),
    }
}

/// What an actor folded from the whole of `actions` shows, read off the fold itself.
fn seen_folded(row: &DuosMatch, actions: &[DuosAction]) -> Seen {
    let state = folded(row, actions);
    Seen {
        hash: engine::hash(&state),
        next_seq: i64::try_from(actions.len() + 1).expect("a small seq"),
        views: DUOS_SEATS
            .iter()
            .map(|seat| to_json(&engine::view(Pinned::default(), &state, *seat)))
            .collect(),
    }
}

/// The answer a logged nonce gets sent again by the seat that sent it: its original ack.
async fn ack_of(actor: &DuosActor, action: &DuosAction) -> Value {
    to_json(
        &actor
            .submit(action.seat, action.nonce.clone(), DuosActionBody::Concede)
            .await,
    )
}

/// A live match whose `LOGGED` rows are written, rebuilt and drained: its checkpoint stored.
async fn drained(app: &Arc<App>, four: bool) -> (DuosMatch, Vec<DuosAction>) {
    let people = people(app, four).await;
    let (row, actions) = game(MATCH_ID, &people, LOGGED + TAIL + 1);
    write_match(app, &row).await;
    append(app, &rows_of(&row, 1, &actions[..LOGGED])).await;
    actor_of(app, MATCH_ID).await;
    assert_eq!(registry(app).stop_all().await, 1);
    (row, actions)
}

#[tokio::test(start_paused = true)]
async fn a_drained_coop_match_rebuilds_from_its_checkpoint_as_a_full_fold_rebuilds_it() {
    jackioh_cards::register_all();
    let logs = record_logs();
    let app = empty_test_app().await;
    let (row, actions) = drained(&app, false).await;

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
    let body: DuosCheckpointBody = serde_json::from_str(&checkpoint.body).expect("the body reads");
    assert_eq!(engine::hash(&body.state), folded_hash(&row, &actions[..LOGGED]));
    assert_eq!(logs.named("duos.checkpoint.written").len(), 1);

    append(&app, &rows_of(&row, LOGGED + 1, &actions[LOGGED..LOGGED + TAIL])).await;
    let resumed = actor_of(&app, MATCH_ID).await;
    assert_eq!(from_seq(&logs), json!(LOGGED));
    let from_checkpoint = seen(&resumed).await;

    registry(&app).stop(MATCH_ID).await;
    set_checkpoint(&app, None).await;
    let whole = actor_of(&app, MATCH_ID).await;
    assert_eq!(from_seq(&logs), json!(0));
    let from_the_log = seen(&whole).await;

    assert_eq!(from_checkpoint, from_the_log);
    assert_eq!(from_checkpoint, seen_folded(&row, &actions[..LOGGED + TAIL]));
    assert_eq!(from_checkpoint.next_seq, (LOGGED + TAIL + 1) as i64);
    assert!(logs.named("duos.checkpoint.refused").is_empty());
    assert!(logs.named("duos.fold.errors").is_empty());
    // §9.3: the nonce map and the seq counter are the whole log's on both.
    for actor in [&resumed, &whole] {
        for (seq, action) in [(1, &actions[0]), (LOGGED + TAIL, &actions[LOGGED + TAIL - 1])] {
            assert_eq!(
                ack_of(actor, action).await,
                json!({ "type": "ack", "nonce": action.nonce, "seq": seq })
            );
        }
    }
}

#[tokio::test(start_paused = true)]
async fn a_two_v_two_match_resumed_from_its_checkpoint_shows_all_four_seats_a_full_folds_views_and_the_next_drain_moves_it_up()
 {
    jackioh_cards::register_all();
    let logs = record_logs();
    let app = empty_test_app().await;
    let (row, actions) = drained(&app, true).await;
    assert_eq!(checkpoints(&app).await[0].seq, LOGGED as i64);

    append(&app, &rows_of(&row, LOGGED + 1, &actions[LOGGED..LOGGED + TAIL])).await;
    let resumed = actor_of(&app, MATCH_ID).await;
    assert_eq!(from_seq(&logs), json!(LOGGED));
    assert_eq!(seen(&resumed).await, seen_folded(&row, &actions[..LOGGED + TAIL]));

    // R3731002: team B's people play on from the resumed state too.
    let next = &actions[LOGGED + TAIL];
    assert!(matches!(
        next.seat,
        DuosSeat::B1 | DuosSeat::B2 | DuosSeat::A1 | DuosSeat::A2
    ));
    let seq = LOGGED + TAIL + 1;
    assert_eq!(
        to_json(
            &resumed
                .submit(next.seat, next.nonce.clone(), next.body.clone())
                .await
        ),
        json!({ "type": "ack", "nonce": next.nonce, "seq": seq })
    );
    assert_eq!(registry(&app).stop_all().await, 1);
    let stored = checkpoints(&app).await;
    assert_eq!(
        stored.len(),
        1,
        "the later checkpoint takes the earlier one's place"
    );
    assert_eq!(stored[0].seq, seq as i64);
    assert_eq!(stored[0].nonce, next.nonce);

    let again = actor_of(&app, MATCH_ID).await;
    assert_eq!(from_seq(&logs), json!(seq));
    let from_checkpoint = seen(&again).await;
    assert_eq!(from_checkpoint, seen_folded(&row, &actions[..seq]));

    registry(&app).stop(MATCH_ID).await;
    set_checkpoint(&app, None).await;
    let whole = actor_of(&app, MATCH_ID).await;
    assert_eq!(from_seq(&logs), json!(0));
    assert_eq!(seen(&whole).await, from_checkpoint);
    assert!(logs.named("duos.checkpoint.refused").is_empty());
}

#[tokio::test(start_paused = true)]
async fn a_coop_checkpoint_that_fails_a_check_is_refused_and_the_whole_log_is_folded() {
    jackioh_cards::register_all();
    let logs = record_logs();
    let app = empty_test_app().await;
    let (row, actions) = drained(&app, false).await;
    let good = checkpoints(&app).await.pop().expect("the drained checkpoint");

    registry(&app).stop(MATCH_ID).await;
    set_checkpoint(&app, None).await;
    let reference = seen(&actor_of(&app, MATCH_ID).await).await;
    assert_eq!(from_seq(&logs), json!(0));
    assert_eq!(reference, seen_folded(&row, &actions[..LOGGED]));

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
        registry(&app).stop(MATCH_ID).await;
        let seq = poison.seq;
        set_checkpoint(&app, Some(poison)).await;
        let rebuilt = seen(&actor_of(&app, MATCH_ID).await).await;
        assert_eq!(rebuilt, reference, "a checkpoint refused for its {reason}");
        assert_eq!(from_seq(&logs), json!(0), "{reason}");
        let refused = logs.named("duos.checkpoint.refused");
        assert_eq!(refused.len(), n + 1, "{reason}");
        let entry = refused.last().expect("the refusal");
        assert_eq!(entry.level, "warn");
        assert_eq!(
            entry.data,
            json!({ "matchId": MATCH_ID, "seq": seq, "reason": reason }),
            "only the reason, never the body"
        );
    }
    assert!(logs.named("duos.fold.errors").is_empty());
}

#[tokio::test(start_paused = true)]
async fn a_two_v_two_match_checkpoints_every_min_actions_rows_as_it_runs_and_a_crash_resumes_there() {
    jackioh_cards::register_all();
    let logs = record_logs();
    let app = empty_test_app().await;
    let people = people(&app, true).await;
    let (row, actions) = game(MATCH_ID, &people, LOGGED);
    write_match(&app, &row).await;
    let actor = actor_of(&app, MATCH_ID).await;
    assert_eq!(from_seq(&logs), json!(0));

    let every = usize::try_from(MATCH_CHECKPOINT_MIN_ACTIONS).expect("a small count");
    for (n, action) in actions.iter().enumerate() {
        let seq = n + 1;
        assert_eq!(
            to_json(
                &actor
                    .submit(action.seat, action.nonce.clone(), action.body.clone())
                    .await
            ),
            json!({ "type": "ack", "nonce": action.nonce, "seq": seq }),
            "at seq {seq}"
        );
        if seq == every - 1 {
            actor.idle().await;
            assert!(checkpoints(&app).await.is_empty(), "none before the interval");
        }
        if seq == every {
            actor.idle().await;
            let stored = checkpoints(&app).await;
            assert_eq!(stored.len(), 1);
            assert_eq!(stored[0].seq, MATCH_CHECKPOINT_MIN_ACTIONS);
            assert_eq!(stored[0].nonce, format!("cp-{every}"));
        }
    }
    actor.idle().await;
    let stored = checkpoints(&app).await;
    assert_eq!(stored.len(), 1);
    assert_eq!(
        stored[0].seq, MATCH_CHECKPOINT_MIN_ACTIONS,
        "the next is due at twice the interval"
    );
    assert_eq!(logs.named("duos.checkpoint.written").len(), 1);

    // A crash, or the reaper's stop, writes none: the rebuild resumes from the periodic one.
    registry(&app).stop(MATCH_ID).await;
    let rebuilt = actor_of(&app, MATCH_ID).await;
    assert_eq!(from_seq(&logs), json!(MATCH_CHECKPOINT_MIN_ACTIONS));
    let resumed = seen(&rebuilt).await;
    assert_eq!(resumed, seen_folded(&row, &actions));
    assert_eq!(resumed.next_seq, (LOGGED + 1) as i64);
    assert_eq!(resumed.hash, folded_hash(&row, &actions));
}

#[tokio::test(start_paused = true)]
async fn the_restart_drain_checkpoints_each_long_coop_match_and_passes_over_a_short_one() {
    jackioh_cards::register_all();
    let logs = record_logs();
    let app = empty_test_app().await;
    let people = people(&app, false).await;
    let short = usize::try_from(MATCH_CHECKPOINT_MIN_ACTIONS - 1).expect("a small count");
    let mut ids = Vec::new();
    for (match_id, length) in [
        ("duos-checkpoints-long", LOGGED),
        ("duos-checkpoints-short", short),
    ] {
        let (row, actions) = game(match_id, &people, length);
        write_match(&app, &row).await;
        append(&app, &rows_of(&row, 1, &actions)).await;
        actor_of(&app, match_id).await;
        ids.push(match_id);
    }
    let held = registry(&app);
    assert_eq!(held.drain().await, 2);

    let stored = checkpoints(&app).await;
    assert_eq!(stored.len(), 1, "{stored:?}");
    assert_eq!(stored[0].match_id, "duos-checkpoints-long");
    assert_eq!(stored[0].seq, LOGGED as i64);
    assert_eq!(logs.named("duos.checkpoint.written").len(), 1);
    for match_id in ids {
        assert!(!held.has(match_id), "{match_id} left live after the drain");
    }
}

#[tokio::test(start_paused = true)]
async fn a_coop_checkpoint_the_store_fails_to_write_costs_only_the_shortcut() {
    jackioh_cards::register_all();
    let logs = record_logs();
    let app = empty_test_app().await;
    let people = people(&app, false).await;
    let (row, actions) = game(MATCH_ID, &people, LOGGED);
    write_match(&app, &row).await;
    append(&app, &rows_of(&row, 1, &actions)).await;
    actor_of(&app, MATCH_ID).await;

    fail_on(&app, "duos.matches.putCheckpoint", "the store is down").await;
    assert_eq!(registry(&app).stop_all().await, 1);
    let failed = logs.named("duos.checkpoint.failed");
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].level, "warn");
    assert!(failed[0].data.get("body").is_none(), "{:?}", failed[0].data);
    assert!(checkpoints(&app).await.is_empty());

    stop_failing(&app).await;
    let rebuilt = actor_of(&app, MATCH_ID).await;
    assert_eq!(from_seq(&logs), json!(0));
    assert_eq!(seen(&rebuilt).await, seen_folded(&row, &actions));
}

#[tokio::test(start_paused = true)]
async fn a_coop_match_ending_after_a_checkpoint_rebuild_drops_its_checkpoint() {
    jackioh_cards::register_all();
    let logs = record_logs();
    let app = empty_test_app().await;
    let (row, actions) = drained(&app, false).await;
    append(&app, &rows_of(&row, LOGGED + 1, &actions[LOGGED..LOGGED + TAIL])).await;
    let resumed = actor_of(&app, MATCH_ID).await;
    assert_eq!(from_seq(&logs), json!(LOGGED));
    assert_eq!(checkpoints(&app).await.len(), 1);

    // B11: a team concedes once both its people have.
    for (n, (seat, nonce)) in [(DuosSeat::A1, "a1-concede"), (DuosSeat::A2, "a2-concede")]
        .into_iter()
        .enumerate()
    {
        assert_eq!(
            to_json(
                &resumed
                    .submit(seat, nonce.to_string(), DuosActionBody::Concede)
                    .await
            ),
            json!({ "type": "ack", "nonce": nonce, "seq": LOGGED + TAIL + 1 + n })
        );
    }
    resumed.idle().await;
    assert!(resumed.is_finished());
    assert!(checkpoints(&app).await.is_empty(), "the ending drops it");
    assert!(stored_row(&app, MATCH_ID).await.ended_at.is_some());
    // A drain after the end writes none back.
    assert_eq!(registry(&app).stop_all().await, 1);
    assert!(checkpoints(&app).await.is_empty());
}

/// The states of a whole co-op game, every `WHOLE_GAME_STRIDE`-th and its last, each written as a
/// checkpoint, pass their checks and read back byte for byte, so a drain at any of them could be resumed.
#[tokio::test(start_paused = true)]
async fn a_whole_coop_games_states_checkpoint_and_pass_their_checks() {
    jackioh_cards::register_all();
    let _logs = record_logs();
    let people = vec!["a1".to_string(), "a2".to_string()];
    let row = live_row(MATCH_ID, "duos-checkpoints-whole", &people);
    let mut state =
        engine::create_and_begin(Pinned::default(), &engine::create_options(&row)).expect("the match begins");
    let mut policy = Rng::new(&format!("{}:policy", row.seed), 0);
    let mut rows: Vec<DuosActionRow> = Vec::new();
    let mut checked = 0;
    while state.result.is_none() && rows.len() < WHOLE_GAME_CAP {
        let seq = rows.len() + 1;
        let action = policy_move(&state, &mut policy, seq).expect("a move");
        rows.extend(rows_of(&row, seq, std::slice::from_ref(&action)));
        let result = engine::reduce(Pinned::default(), &state, &action);
        assert!(
            result.error.is_none(),
            "{} at seq {seq}: {:?}",
            row.seed,
            result.error
        );
        state = result.state;
        if !seq.is_multiple_of(WHOLE_GAME_STRIDE) && state.result.is_none() {
            continue;
        }

        let body = DuosCheckpointBody { state: state.clone() };
        let checkpoint = checkpoint_row(&row, seq as i64, &action.nonce, &body, row.created_at)
            .expect("a checkpoint serializes");
        let read = verify(&checkpoint, &row, &rows).unwrap_or_else(|refusal| {
            panic!("{} at seq {seq}: refused for its {}", row.seed, refusal.as_str())
        });
        assert_eq!(
            engine::hash(&read.state),
            engine::hash(&state),
            "{} at seq {seq}",
            row.seed
        );
        checked += 1;
    }
    assert!(
        state.result.is_some(),
        "the game ends within {WHOLE_GAME_CAP} moves"
    );
    assert!(
        checked >= rows.len() / WHOLE_GAME_STRIDE,
        "{checked} of {} rows",
        rows.len()
    );
}
