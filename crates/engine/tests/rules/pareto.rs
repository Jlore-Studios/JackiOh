//! ME-PARETO (docs/meditative-set.md M5, MD-D28–MD-D30): the engine-side judge of "the AI optimal
//! move" (R1125) and the Cane's attack (R1126).
//!
//! R1125: while a card with `StaticFlags.judges_plays` acts, each opponent play is scored against
//! their other playable cards, from their own concealed view, on the state before the play. The top
//! score is optimal, ties count, and only plays are judged — no verdict without a judge, none for a
//! card that is not theirs to play. The verdict is stored on the play, follows a face-down Trap to
//! its fresh id (R227), and never reaches a view (§10.8). The same play twice folds identically.
//!
//! Port of `packages/engine/test/pareto.test.ts`.

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat_judge::{cheap, combat_judge_game, dear, judge, snare};
use crate::rules::fixtures::harness::{in_hand, put, slot};

/// P2 to play at turn 4 with 10 mana and the judge watching from P1's backrow.
fn watched(seed: &str) -> GameState {
    let mut state = combat_judge_game(seed, None);
    state.turn = 4;
    state.active = P2;
    state.phase = Phase::Main;
    state.players[P2].mana.current = 10;
    put(&mut state, &judge.id, slot(P1, Row::Backrow, 1), Default::default());
    state
}

fn play_body(state: &GameState, player: PlayerId, instance_id: &str) -> ActionBody {
    jackioh_engine::reduce::legal_actions(state, player)
        .into_iter()
        .find(|body| matches!(body, ActionBody::Play { instance_id: id, .. } if id == instance_id))
        .unwrap_or_else(|| panic!("no play for {instance_id}"))
}

fn play(state: &GameState, player: PlayerId, instance_id: &str, nonce: &str) -> ReduceResult {
    let body = play_body(state, player, instance_id);
    jackioh_engine::reduce::reduce(state, &Action::new(body, player, nonce.to_string()))
}

#[test]
fn r1125_top_score_optimal_ties_count() {
    let mut state = watched("r1125-top");
    let cheap_card = in_hand(&mut state, &cheap.id, P2, 1).into_iter().next().unwrap();
    let dear_card = in_hand(&mut state, &dear.id, P2, 1).into_iter().next().unwrap();
    // The 1/1 for 1 outrates the 5/5 for 6: top score optimal, the other not.
    assert!(jackioh_engine::subsystems::pareto::judge_play(&state, P2, &cheap_card.id));
    assert!(!jackioh_engine::subsystems::pareto::judge_play(&state, P2, &dear_card.id));

    // Two identical bodies tie at the top: both optimal.
    let mut state = watched("r1125-tie");
    let first = in_hand(&mut state, &cheap.id, P2, 1).into_iter().next().unwrap();
    let second = in_hand(&mut state, &cheap.id, P2, 1).into_iter().next().unwrap();
    assert!(jackioh_engine::subsystems::pareto::judge_play(&state, P2, &first.id));
    assert!(jackioh_engine::subsystems::pareto::judge_play(&state, P2, &second.id));
}

#[test]
fn r1125_concealed_pre_play_view() {
    let mut state = watched("r1125-concealed");
    in_hand(&mut state, &cheap.id, P1, 1);
    let cheap_card = in_hand(&mut state, &cheap.id, P2, 1).into_iter().next().unwrap();
    let dear_card = in_hand(&mut state, &dear.id, P2, 1).into_iter().next().unwrap();

    // The copy the judge scores on hides what P2 cannot see, and keeps what they own.
    let base = jackioh_engine::subsystems::scorer::dry_run_base(&state, P2)
        .expect("P2 can play now");
    assert!(
        base.players[P1].hand.iter().all(|card| card.def_id == "zephyrs:hidden-card"),
        "P1's hand is concealed from the copy"
    );
    assert!(base.players[P2].hand.iter().any(|card| card.id == cheap_card.id));
    assert!(base.players[P2].hand.iter().any(|card| card.id == dear_card.id));

    // The verdict is stored on the play, on the pre-play state: this card, this turn, not optimal.
    let out = play(&state, P2, &dear_card.id, "n1");
    assert!(out.error.is_none());
    assert_eq!(
        out.state.play_judgement,
        Some(PlayJudgement {
            instance_id: dear_card.id.clone(),
            optimal: false,
            turn: 4,
        })
    );
    // And it never reaches a view.
    let view = serde_json::to_value(view_for(&out.state, P1)).expect("serialises");
    assert!(view.get("playJudgement").is_none());
}

#[test]
fn r1125_casts_and_unwatched_store_nothing() {
    // No judge on the board: a play stores no verdict.
    let mut state = combat_judge_game("r1125-unwatched", None);
    state.turn = 4;
    state.active = P2;
    state.phase = Phase::Main;
    state.players[P2].mana.current = 10;
    let cheap_card = in_hand(&mut state, &cheap.id, P2, 1).into_iter().next().unwrap();
    let out = play(&state, P2, &cheap_card.id, "n1");
    assert!(out.error.is_none());
    assert_eq!(out.state.play_judgement, None);

    // A judge watches, but an attack is no play: nothing stored.
    let mut state = watched("r1125-attack");
    let attacker = put(&mut state, &cheap_card.id, slot(P2, Row::Units, 1), Default::default());
    let out = jackioh_engine::reduce::reduce(
        &state,
        &Action::new(
            ActionBody::Attack {
                attacker_id: attacker.id.clone(),
                target_id: "hero-p1".to_string(),
            },
            P2,
            "n2".to_string(),
        ),
    );
    assert!(out.error.is_none());
    assert_eq!(out.state.play_judgement, None);
}

#[test]
fn r1125_face_down_trap_keeps_judgement() {
    let mut state = watched("r1125-trap");
    let snare_card = in_hand(&mut state, &snare.id, P2, 1).into_iter().next().unwrap();
    let out = play(&state, P2, &snare_card.id, "n1");
    assert!(out.error.is_none());
    // R227 gave the face-down Trap a fresh id, and the judgement followed it there.
    let judgement = out.state.play_judgement.as_ref().expect("a verdict was stored");
    assert_ne!(judgement.instance_id, snare_card.id);
    assert_eq!(judgement.turn, 4);
    let played: Vec<&GameEvent> = out
        .events
        .iter()
        .filter(|event| matches!(event, GameEvent::CardPlayed { .. }))
        .collect();
    assert_eq!(played.len(), 1);
    let GameEvent::CardPlayed {
        instance_id,
        former_id,
        ..
    } = played[0]
    else {
        panic!("a cardPlayed was collected")
    };
    assert_eq!(instance_id, &judgement.instance_id);
    assert_eq!(former_id.as_ref(), Some(&snare_card.id));
}

#[test]
fn r1125_replay_fold_matches_hash() {
    let mut state = watched("r1125-replay");
    // The cheap body stays the better alternative, so the dear play is judged both times.
    in_hand(&mut state, &cheap.id, P2, 1);
    let dear_card = in_hand(&mut state, &dear.id, P2, 1).into_iter().next().unwrap();
    let first = play(&state, P2, &dear_card.id, "n1");
    let second = play(&state, P2, &dear_card.id, "n2");
    assert!(first.error.is_none());
    assert_eq!(
        serde_json::to_value(&first.state).expect("serialises"),
        serde_json::to_value(&second.state).expect("serialises")
    );
    assert_eq!(
        serde_json::to_value(&first.events).expect("serialises"),
        serde_json::to_value(&second.events).expect("serialises")
    );
}
