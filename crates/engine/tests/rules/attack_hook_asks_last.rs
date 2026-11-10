//! An attack hook whose last effect asks: nothing is left to park, so the sequence owes only the
//! check after it (`ListStatus::Asked` in `combat.rs`). One card holds all five hooks, each ending on
//! its prompt: each runs once and the attack goes on, through a JSON round trip and a replay alike.

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::damage_combat::{
    Recorder, answer, grunt, in_pile, last_asker, notes, playing, recorder, replays_to, round_trip,
};
use crate::rules::fixtures::harness::{input, put, slot};

/// More answers than either test needs: a hook that asks again after its answer stops here.
const MOST_ANSWERS: usize = 8;

/// Answers every prompt in turn from a JSON round trip of the state, so each owed item is read
/// back from what was saved: how many it answered (at most `MOST_ANSWERS`), and the state after.
fn answer_all(mut state: GameState) -> (usize, GameState) {
    let mut asked = 0;
    while state.pending.is_some() && asked < MOST_ANSWERS {
        state = answer(&round_trip(&state)).state;
        asked += 1;
    }
    (asked, state)
}

/// Answers every prompt in the recorded game too, which must replay to the same state.
fn answer_live(game: &mut Recorder) {
    while let Some(pending) = game.state().pending.clone() {
        game.play(input(json!({
            "type": "answer", "choiceId": pending.id, "selection": [{ "pick": "none" }],
            "playerId": pending.player_id
        })));
    }
    assert!(replays_to(&game.start, &game.log, game.state()));
}

#[test]
fn an_attackers_hooks_that_each_end_on_a_question_run_once_and_the_attack_lands_once() {
    let mut state = playing("ask-last-attacker");
    let asker = put(
        &mut state,
        &last_asker.id,
        slot(P1, Row::Units, 1),
        Default::default(),
    );
    let health = state.players.p2.hero.health;
    let mut game = recorder(&state);
    game.play(input(json!({
        "type": "attack", "attackerId": asker.id, "targetId": "hero-p2", "playerId": "p1"
    })));
    let paused = game.state().clone();
    assert_eq!(notes(&paused), vec!["when:ask"]);
    assert_eq!(paused.players.p2.hero.health, health);
    let (asked, done) = answer_all(paused);
    assert_eq!(asked, 3);
    assert_eq!(
        notes(&done),
        vec![
            "when:ask",
            "when:answered",
            "after:ask",
            "after:answered",
            "ally:ask",
            "ally:answered"
        ]
    );
    assert_eq!(done.players.p2.hero.health, health - 2);
    assert!(done.work.is_empty());
    answer_live(&mut game);
}

#[test]
fn a_defenders_hooks_that_each_end_on_a_question_run_once_and_the_combat_follows() {
    let mut state = playing("ask-last-defender");
    let attacker = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), Default::default());
    let asker = put(
        &mut state,
        &last_asker.id,
        slot(P2, Row::Units, 1),
        Default::default(),
    );
    let mut game = recorder(&state);
    game.play(input(json!({
        "type": "attack", "attackerId": attacker.id, "targetId": asker.id, "playerId": "p1"
    })));
    let paused = game.state().clone();
    assert_eq!(notes(&paused), vec!["ally-attacked:ask"]);
    assert_eq!(paused.pending.as_ref().map(|pending| pending.player_id), Some(P2));
    let (asked, done) = answer_all(paused);
    assert_eq!(asked, 2);
    assert_eq!(
        notes(&done),
        vec![
            "ally-attacked:ask",
            "ally-attacked:answered",
            "attacked:ask",
            "attacked:answered"
        ]
    );
    assert!(in_pile(&done, P1, "graveyard", &attacker.id));
    assert!(done.work.is_empty());
    answer_live(&mut game);
}
