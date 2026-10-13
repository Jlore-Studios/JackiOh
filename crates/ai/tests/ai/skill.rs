//! R747000: the skill mixture. Each decision follows the best AI with chance `skill` and the
//! random policy otherwise: 1.0 is the exact AI, draw for draw, and 0.0 is the random policy on
//! the same stream. The Elo helpers map a skill to its expected rating below the reference.

use jackioh_ai::{
    AiDeckOptions, MatchConfig, MatchHooks, SearchBudget, SeatController, build_ai_deck,
    clamp_skill, elo_delta_from_score, follows_best, play_match, skill_elo_offset,
    skill_expected_elo,
};
use jackioh_engine::testkit::{DECK_SIZE, PerPlayer, create_rng};

use super::support::register_cards;

const QUICK: SearchBudget = SearchBudget {
    nodes: 30,
    lethal_nodes: 10,
    determinizations: 1,
    beam_width: 1,
    root_branching: 6,
    branching: 2,
    max_depth: 2,
    finalists: 1,
};

/// One quick game on `seed`, for parity: the same decks whatever seats them.
fn quick_game(p1: SeatController, p2: SeatController) -> MatchConfig {
    register_cards();
    let seed = "skill-parity";
    let deal = |seat: &str| {
        build_ai_deck(
            &mut create_rng(&format!("{seed}:{seat}-deck"), 0),
            DECK_SIZE,
            &AiDeckOptions::default(),
        )
    };
    MatchConfig {
        seed: seed.to_string(),
        decks: (deal("p1"), deal("p2")),
        handicaps: None,
        controllers: PerPlayer::new(p1, p2),
        max_actions: None,
    }
}

fn ai(budget: SearchBudget) -> SeatController {
    SeatController::Ai {
        budget: Some(budget),
    }
}

fn skilled(budget: SearchBudget, skill: f64) -> SeatController {
    SeatController::Skill {
        budget: Some(budget),
        skill: Some(skill),
    }
}

#[test]
fn r747000_skill_clamps_into_zero_to_one_and_nan_is_zero() {
    assert_eq!(clamp_skill(1.0), 1.0);
    assert_eq!(clamp_skill(0.0), 0.0);
    assert_eq!(clamp_skill(0.4), 0.4);
    assert_eq!(clamp_skill(1.5), 1.0);
    assert_eq!(clamp_skill(-0.2), 0.0);
    assert_eq!(clamp_skill(f64::INFINITY), 1.0);
    assert_eq!(clamp_skill(f64::NEG_INFINITY), 0.0);
    assert_eq!(clamp_skill(f64::NAN), 0.0);
}

#[test]
fn r747000_skill_endpoints_draw_nothing_and_a_middle_skill_draws_once() {
    for skill in [0.0, 1.0] {
        let mut rng = create_rng("skill-draws", 0);
        assert_eq!(follows_best(skill, &mut rng), skill >= 1.0);
        assert_eq!(rng.cursor(), 0, "skill {skill} took a draw of its own");
    }
    let mut rng = create_rng("skill-draws", 0);
    let _ = follows_best(0.4, &mut rng);
    assert_eq!(rng.cursor(), 1);
}

#[test]
fn r747000_skill_one_plays_the_exact_ai() {
    let played = quick_game(ai(QUICK), SeatController::Random);
    let degraded = quick_game(skilled(QUICK, 1.0), SeatController::Random);
    let a = play_match(&played, &mut MatchHooks::default());
    let b = play_match(&degraded, &mut MatchHooks::default());
    assert_eq!(a.hash, b.hash);
    assert_eq!(a.log.len(), b.log.len());
}

#[test]
fn r747000_skill_zero_plays_the_random_policy() {
    let plain = quick_game(SeatController::Random, SeatController::Random);
    let degraded = quick_game(skilled(QUICK, 0.0), skilled(QUICK, 0.0));
    let a = play_match(&plain, &mut MatchHooks::default());
    let b = play_match(&degraded, &mut MatchHooks::default());
    assert_eq!(a.hash, b.hash);
    assert_eq!(a.log.len(), b.log.len());
}

#[test]
fn r747000_skill_expected_elo_interpolates_the_measured_table() {
    // On the knots it is exact.
    assert_eq!(skill_elo_offset(1.0), 0.0);
    assert_eq!(skill_elo_offset(0.0), 720.0);
    assert_eq!(skill_elo_offset(0.5), 320.0);
    // Between knots it runs straight, and past the ends it holds.
    assert_eq!(skill_elo_offset(0.625), 235.0);
    assert_eq!(skill_elo_offset(1.5), 0.0);
    assert_eq!(skill_elo_offset(-0.5), 720.0);
    assert_eq!(skill_expected_elo(1.0, 1200.0), 1200.0);
    assert_eq!(skill_expected_elo(0.5, 1200.0), 880.0);
}

#[test]
fn r747000_elo_delta_from_score_inverts_the_logistic() {
    assert_eq!(elo_delta_from_score(0.5), 0.0);
    let upper = elo_delta_from_score(0.75);
    assert!((upper - 190.85).abs() < 0.01, "{upper}");
    assert!((elo_delta_from_score(0.25) + upper).abs() < 1e-9);
    assert_eq!(elo_delta_from_score(f64::NAN), 0.0);
    assert_eq!(elo_delta_from_score(f64::INFINITY), 0.0);
}
