//! R747000 (SPEC §9.12, R1452): the skill knob on a seat. At skill 1.0 a seat takes no extra draw, at
//! 0.0 it plays the skill miss (R2214000) on the same stream, in between each decision rolls the seat's
//! own seeded stream, and every run reproduces.
//! R4216000: the miss reads only what its seat may know.

use jackioh_ai::{
    AI_BUDGET, AI_SKILL_MAX, AI_SKILL_MIN, MatchConfig, MatchHooks, MatchSetup, SearchBudget, SeatController,
    action_key, candidate_set, clamp_skill, det_ground, play_match, plays_best, probe_world, redact,
    skill_miss_action,
};
use jackioh_engine::testkit::{ActionBody, GameState, PerPlayer, create_rng, json};

use super::support::{AI, random_decks, register_cards, scenario};

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
        setup: MatchSetup::default(),
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
fn r2214000_zero_skill_plays_the_miss_not_the_random_policy() {
    let skilled = config(
        "r2214000-zero",
        seat(Some(AI_SKILL_MIN)),
        seat(Some(AI_SKILL_MIN)),
        Some(SHORT_GAME),
    );
    let random = config(
        "r2214000-zero",
        SeatController::Random,
        SeatController::Random,
        Some(SHORT_GAME),
    );
    let at_zero = play_match(&skilled, &mut MatchHooks::default());
    let again = play_match(&skilled, &mut MatchHooks::default());
    let plain = play_match(&random, &mut MatchHooks::default());

    // The miss draws only on the seat's own stream, so the game replays.
    assert_eq!(again.log, at_zero.log);
    assert_eq!(again.hash, at_zero.hash);
    // No search ran, and every miss was a legal move: the AI was never asked to decide.
    assert_eq!(at_zero.decisions, 0);
    assert_eq!(at_zero.nodes, 0);
    assert_eq!(at_zero.fallbacks, 0);
    assert!(at_zero.rejected.is_empty());
    assert!(at_zero.thrown.is_empty());
    // A miss is not §10.7's uniform draw.
    assert_ne!(at_zero.log, plain.log);
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
        serde_json::to_value(bare).expect("serializes"),
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
        serde_json::to_value(skilled).expect("serializes"),
        json!({ "kind": "ai", "skill": 0.25 })
    );
}

/// R4216000: p1 holds Core #50 K-Pop Fanatic, whose Allure may aim at p2's face-down card; `trap`,
/// `hand` and `library` are p2's hidden cards, and S #174 lies seen in p2's graveyard.
fn masked(trap: &str, hand: &str, library: [&str; 2]) -> GameState {
    register_cards();
    scenario(json!({
        "seed": "r4216000-masked",
        "active": AI,
        "turn": 9,
        "p1": {
            "field": ["core-008", "core-011"],
            "hand": ["core-050"],
            "mana": 3,
            "library": ["core-020", "core-053"],
        },
        "p2": {
            "field": ["core-019"],
            "backrow": [{ "def": trap, "faceUp": false }],
            "graveyard": ["stinkyscape-174"],
            "hand": [hand],
            "library": library,
        },
    }))
    .state()
    .clone()
}

/// R4216000, R602: under v0.3.0c's kept script (R767000) p2's face-down Radiant C #88 Siphon Squad is
/// live, so p1's units show 0 Attack; `hand` is p2's hidden hand.
fn siphoned_live(hand: &str) -> GameState {
    register_cards();
    scenario(json!({
        "seed": "redact-live-face-down",
        "active": AI,
        "turn": 9,
        "p1": {
            "field": ["core-019", "core-011"],
            "hand": ["core-008"],
            "mana": 3,
            "library": ["core-020", "core-053"],
        },
        "p2": {
            "backrow": [{ "def": "classic-088", "faceUp": false, "radiant": true }],
            "hand": [hand],
            "library": ["core-005", "core-016"],
        },
    }))
    .state()
    .clone()
}

/// R4216000, R762: `determinize_shown_cost.rs`'s `shroomed(true)`, copied. Vanilla 4 + Timmy 3 is
/// exactly p2's 7 Health, and p2's one face-down card is Doom Shroom, showing (4); the other Traps at
/// (4), S #22 and S #174, lie seen in p2's graveyard.
fn shroomed() -> GameState {
    register_cards();
    scenario(json!({
        "seed": "shown-cost-doom",
        "active": AI,
        "turn": 9,
        "p1": { "field": ["core-008", "core-011"], "library": ["core-020", "core-053"] },
        "p2": {
            "health": 7,
            "backrow": [{ "def": "classicplus-001", "faceUp": false }],
            "graveyard": ["stinkyscape-022", "stinkyscape-174"],
            "library": ["core-005", "core-016"],
        },
    }))
    .state()
    .clone()
}

/// R4216000: the miss's candidates, as it lists them: on the probe world dealt from what the seat sees.
fn listed(state: &GameState) -> Vec<String> {
    let public = redact(state, AI);
    candidate_set(&probe_world(&public, AI, &det_ground(&public, AI)), AI)
        .iter()
        .map(action_key)
        .collect()
}

/// R4216000: `a` and `b` show the seat the same thing, and the miss lists and draws alike on both, and
/// on the redaction itself: the same candidates (the true state's, in its order), the same picks and the
/// same draws.
fn holds_the_miss_alike(a: &GameState, b: &GameState, label: &str) {
    assert_eq!(
        serde_json::to_value(redact(a, AI)).expect("serializes"),
        serde_json::to_value(redact(b, AI)).expect("serializes"),
        "PREMISE {label}: the two positions' redactions match"
    );
    let true_list: Vec<String> = candidate_set(a, AI).iter().map(action_key).collect();
    assert_eq!(listed(a), true_list, "{label}: a's listed candidates");
    assert_eq!(listed(b), true_list, "{label}: b's listed candidates");
    let public = redact(a, AI);
    for k in 0..50 {
        let start = create_rng(&format!("r4216000-miss:{k}"), 0);
        let (mut on_a, mut on_b, mut on_public) = (start.clone(), start.clone(), start.clone());
        let picked = skill_miss_action(a, AI, &mut on_a);
        assert!(picked.is_some(), "{label} k={k}: the miss plays on a");
        assert_eq!(
            skill_miss_action(b, AI, &mut on_b),
            picked,
            "{label} k={k}: the miss on b"
        );
        assert_eq!(
            skill_miss_action(&public, AI, &mut on_public),
            picked,
            "{label} k={k}: the miss on the redaction"
        );
        assert_eq!(on_b.cursor(), on_a.cursor(), "{label} k={k}: b's draws");
        assert_eq!(
            on_public.cursor(),
            on_a.cursor(),
            "{label} k={k}: the redaction's draws"
        );
    }
}

#[test]
fn r4216000_the_miss_plays_alike_on_positions_whose_redactions_match() {
    // C+ #1 Doom Shroom and S #22 Gatinsir both show (4) (R351), and p2's hand and library differ too.
    // Before #4216 this pair failed: the miss handed the redaction panicked listing K-Pop Fanatic's
    // targets on the face-down placeholder.
    holds_the_miss_alike(
        &masked("classicplus-001", "core-005", ["core-016", "core-020"]),
        &masked("stinkyscape-022", "core-030", ["core-041", "core-001"]),
        "shown cost",
    );
    let shipped = jackioh_cards::shipped_catalog("v0.3.0c").expect("v0.3.0c is carried");
    jackioh_engine::with_shipped(Some(shipped), || {
        let a = siphoned_live("core-030");
        assert!(
            !candidate_set(&a, AI)
                .iter()
                .any(|action| matches!(action, ActionBody::Attack { .. })),
            "PREMISE live aura: the face-down Siphon Squad leaves p1 nothing to swing with"
        );
        holds_the_miss_alike(&a, &siphoned_live("core-041"), "live aura");
    });
}

#[test]
fn r4216000_the_miss_never_swings_into_a_face_down_doom_shroom_showing_4() {
    let state = shroomed();
    let attacks = candidate_set(&state, AI)
        .iter()
        .filter(|action| matches!(action, ActionBody::Attack { .. }))
        .count();
    assert_eq!(attacks, 2, "PREMISE: both of p1's units may swing");
    // Every world at the shown (4) holds Doom Shroom, which exiles both attackers on the first swing, so
    // the miss priced on such worlds never swings. The cost-blind sampler swung in 158 of these 200.
    let mut swings = 0;
    for k in 0..200 {
        let picked = skill_miss_action(&state, AI, &mut create_rng(&format!("r762-miss:{k}"), 0));
        assert!(picked.is_some(), "k={k}: the miss plays");
        if matches!(picked, Some(ActionBody::Attack { .. })) {
            swings += 1;
        }
    }
    assert_eq!(
        swings, 0,
        "the miss swung into the face-down card showing (4) in {swings} of 200"
    );
}
