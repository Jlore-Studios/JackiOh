//! The fuzz monitor's I7 (`testkit/invariants.rs`): a live game with no prompt open owes no paused
//! sequence, so `work` is empty and `work_cursor` is 0 (§9.3, §10.3, R113). A prompt, the open
//! mulligans (§10.6, R265) or a game over each excuse what is still owed.

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::new_game;

/// Past both mulligans, in the main phase of p1's first turn: nothing owed, no prompt open.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&new_game(seed, None)).state;
    for player in [P1, P2] {
        let keep: Vec<String> = state.players[player]
            .hand
            .iter()
            .map(|card| card.id.clone())
            .collect();
        let action = Action::new(ActionBody::Mulligan { keep }, player, format!("i7-{player}"));
        let result = reduce(&state, &action);
        assert!(result.error.is_none(), "{:?}", result.error);
        state = result.state;
    }
    state
}

/// A paused sequence nothing will resume: no fixture script has the hook.
fn orphan() -> WorkItem {
    WorkItem {
        id: "w-orphan".to_string(),
        seq: 0,
        owner: P1,
        resume: Resume {
            def_id: "fx-1".to_string(),
            hook: "resume".to_string(),
            step: "orphaned".to_string(),
            radiant: false,
            instance_id: None,
            data: IndexMap::new(),
        },
    }
}

fn orphaned(found: &[String]) -> Vec<&String> {
    found
        .iter()
        .filter(|finding| finding.starts_with("I7 orphaned work"))
        .collect()
}

#[test]
fn i7_a_settled_game_with_no_prompt_open_owes_nothing_and_the_monitor_is_clean() {
    let state = playing("i7-clean");
    assert!(state.pending.is_none() && state.mulligan.is_none() && state.result.is_none());
    assert!(state.work.is_empty());
    assert_eq!(state.work_cursor, 0);
    let mut monitor = create_invariant_monitor(&state);
    assert_eq!(monitor.after(&[], &state), Vec::<String>::new());
}

#[test]
fn i7_a_work_item_left_with_no_prompt_open_is_orphaned_work() {
    let mut state = playing("i7-item");
    state.work.push(orphan());
    let mut monitor = create_invariant_monitor(&state);
    let found = monitor.after(&[], &state);
    let i7 = orphaned(&found);
    assert_eq!(i7.len(), 1, "{found:?}");
    assert!(i7[0].contains("w-orphan"), "{}", i7[0]);
    assert!(i7[0].contains("R113"), "{}", i7[0]);
}

#[test]
fn i7_a_work_cursor_left_off_zero_with_nothing_owed_is_orphaned_work() {
    let mut state = playing("i7-cursor");
    state.work_cursor = 1;
    let mut monitor = create_invariant_monitor(&state);
    let found = monitor.after(&[], &state);
    assert_eq!(orphaned(&found).len(), 1, "{found:?}");
}

#[test]
fn i7_work_owed_behind_an_open_prompt_is_not_orphaned() {
    let begun = begin_game(&new_game("i7-prompt", None)).state;
    // The mulligans are open (§10.6, R265): owed work waits for them.
    let mut mulligan = begun.clone();
    mulligan.work.push(orphan());
    mulligan.work_cursor = 1;
    let mut monitor = create_invariant_monitor(&mulligan);
    assert_eq!(orphaned(&monitor.after(&[], &mulligan)), Vec::<&String>::new());

    // A prompt in `pending` (§10.6): the answer resumes the work.
    let mut prompted = playing("i7-prompt");
    let prompt = begun.mulligan.as_ref().expect("the mulligans are open")[P1]
        .prompt
        .clone();
    prompted.pending = Some(prompt);
    prompted.work.push(orphan());
    prompted.work_cursor = 1;
    let mut monitor = create_invariant_monitor(&prompted);
    assert_eq!(orphaned(&monitor.after(&[], &prompted)), Vec::<&String>::new());
}

#[test]
fn i7_work_a_game_over_left_owed_is_not_orphaned() {
    let mut state = playing("i7-over");
    state.result = Some(GameResult {
        winner: Winner::P1,
        reason: GameOverReason::Concede,
    });
    state.work.push(orphan());
    let mut monitor = create_invariant_monitor(&state);
    assert_eq!(orphaned(&monitor.after(&[], &state)), Vec::<&String>::new());
}
