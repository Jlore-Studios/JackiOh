//! R747000 (SPEC §9.12, R1452): the skill knob on a seat. At skill 1.0 a seat decides exactly as it
//! did before the knob and takes no extra draw, at 0.0 it plays §10.7's random policy under the same
//! stream, in between each decision rolls the seat's own seeded stream, and every run reproduces.

use jackioh_ai::{
    AI_BUDGET, AI_SKILL_MAX, AI_SKILL_MIN, MatchConfig, MatchHooks, SearchBudget, SeatController,
    clamp_skill, play_match, plays_best,
};
use jackioh_engine::testkit::{PerPlayer, create_rng, json};

use super::support::random_decks;

/// A budget small enough for whole games in a test.
const QUICK: SearchBudget = SearchBudget {
    nodes: 60,
    lethal_nodes: 15,
    ..AI_BUDGET
};

/// Actions a skilled game plays before it stops: enough for several turns of both seats' decisions.
const SHORT_GAME: usize = 120;

fn seat(skill: Option<f64>) -> SeatController {
    SeatController::Ai {
        budget: Some(QUICK),
        skill,
    }
}

fn config(seed: &str, p1: SeatController, p2: SeatController, max_actions: Option<usize>) -> MatchConfig {
    MatchConfig {
        seed: seed.to_string(),
        decks: random_decks(seed),
        handicaps: None,
        controllers: PerPlayer::new(p1, p2),
        max_actions,
    }
}

#[test]
fn r747000_full_skill_matches_unskilled() {
    let unskilled = config("r747000-full", seat(None), seat(None), Some(SHORT_GAME));
    let full = config(
        "r747000-full",
        seat(Some(AI_SKILL_MAX)),
        seat(Some(AI_SKILL_MAX)),
        Some(SHORT_GAME),
    );
    let before = play_match(&unskilled, &mut MatchHooks::default());
    let after = play_match(&full, &mut MatchHooks::default());

    assert!(before.thrown.is_empty());
    assert!(before.decisions > 0);
    // The same actions, the same hash, and the same AI decisions: no draw was spent on a roll.
    assert_eq!(after.log, before.log);
    assert_eq!(after.hash, before.hash);
    assert_eq!(after.decisions, before.decisions);
    assert_eq!(after.nodes, before.nodes);

    // A draw spent on a roll would move every later decision; the bare draw discipline is pinned too.
    let mut rng = create_rng("r747000-no-draw", 0);
    assert!(plays_best(AI_SKILL_MAX, &mut rng));
    assert!(!plays_best(AI_SKILL_MIN, &mut rng));
    assert_eq!(rng.cursor(), 0);
}

#[test]
fn r747000_zero_skill_equals_random_controller() {
    let skilled = config(
        "r747000-zero",
        seat(Some(AI_SKILL_MIN)),
        seat(Some(AI_SKILL_MIN)),
        None,
    );
    let random = config(
        "r747000-zero",
        SeatController::Random,
        SeatController::Random,
        None,
    );
    let at_zero = play_match(&skilled, &mut MatchHooks::default());
    let plain = play_match(&random, &mut MatchHooks::default());

    assert!(plain.result.is_some());
    assert_eq!(at_zero.log, plain.log);
    assert_eq!(at_zero.hash, plain.hash);
    assert_eq!(at_zero.result, plain.result);
    // No search ran: the AI was never asked to decide.
    assert_eq!(at_zero.decisions, 0);
    assert_eq!(at_zero.nodes, 0);
    assert!(at_zero.rejected.is_empty());
    assert!(at_zero.thrown.is_empty());
}

#[test]
fn r747000_mid_skill_mixes_and_reproduces() {
    let mut rng = create_rng("r747000-mid", 0);
    let best = (0..2000).filter(|_| plays_best(0.5, &mut rng)).count();
    assert!((900..=1100).contains(&best), "{best}");
    // One draw a roll, so a seat's stream advances by its decisions and nothing else.
    assert_eq!(rng.cursor(), 2000);

    let mixed = config("r747000-mix", seat(Some(0.5)), seat(Some(0.5)), Some(SHORT_GAME));
    let first = play_match(&mixed, &mut MatchHooks::default());
    let second = play_match(&mixed, &mut MatchHooks::default());
    assert!(first.thrown.is_empty());
    assert_eq!(second.log, first.log);
    assert_eq!(second.hash, first.hash);
    assert_eq!(second.decisions, first.decisions);

    // Some decisions searched and some did not: fewer than the full-skill game's, more than none.
    let full = play_match(
        &config("r747000-mix", seat(None), seat(None), Some(SHORT_GAME)),
        &mut MatchHooks::default(),
    );
    assert!(first.decisions > 0, "{}", first.decisions);
    assert!(
        first.decisions < i32::try_from(first.log.len()).expect("a short log"),
        "{} decisions in {} actions",
        first.decisions,
        first.log.len()
    );
    assert_ne!(first.log, full.log);
}

#[test]
fn r747000_clamp_skill_pins_range_and_nan() {
    assert_eq!(clamp_skill(-0.5), AI_SKILL_MIN);
    assert_eq!(clamp_skill(1.5), AI_SKILL_MAX);
    assert_eq!(clamp_skill(f64::NAN), AI_SKILL_MAX);
    assert_eq!(clamp_skill(f64::NEG_INFINITY), AI_SKILL_MIN);
    assert_eq!(clamp_skill(f64::INFINITY), AI_SKILL_MAX);
    assert_eq!(clamp_skill(0.25), 0.25);

    // Out of range plays as the end it is past, still with no draw.
    let mut rng = create_rng("r747000-clamp", 0);
    assert!(plays_best(3.0, &mut rng));
    assert!(!plays_best(-3.0, &mut rng));
    assert!(plays_best(f64::NAN, &mut rng));
    assert_eq!(rng.cursor(), 0);
}

#[test]
fn r747000_a_seats_skill_is_optional_on_the_wire() {
    let bare: SeatController = serde_json::from_value(json!({ "kind": "ai" })).expect("a bare ai seat");
    assert_eq!(
        bare,
        SeatController::Ai {
            budget: None,
            skill: None
        }
    );
    // A seat that sets none writes none, so a record written before the knob reads back unchanged.
    assert_eq!(
        serde_json::to_value(bare).expect("serialises"),
        json!({ "kind": "ai" })
    );

    let skilled: SeatController =
        serde_json::from_value(json!({ "kind": "ai", "skill": 0.25 })).expect("a skilled ai seat");
    assert_eq!(
        skilled,
        SeatController::Ai {
            budget: None,
            skill: Some(0.25)
        }
    );
    assert_eq!(
        serde_json::to_value(skilled).expect("serialises"),
        json!({ "kind": "ai", "skill": 0.25 })
    );
}
