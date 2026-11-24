//! The Commander prompts the AI must answer (R4594011, R4594014, R4594015): Unpatched's
//! recast target, Spaghettification's aim, and a targeted Spell under Zero-Day, each open for the
//! AI seat with a legal answer.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, ai_options, is_legal, register_cards};

const UNPATCHED: &str = "infiniteassault-240";
const SPAGHETTI: &str = "infiniteassault-232";
const ZERO_DAY: &str = "infiniteassault-239";
const LUNAR: &str = "core-035"; // Lunar Eclipse: deal 3 damage to a target.
const STATS: &str = "classicplus-057"; // Book of Stats: give a Unit +5/+5.
const VANILLA: &str = "core-008"; // Mr. Vanilla: a 4/4.
const LIBRARY: &[&str] = &["core-008", "core-005"];

/// The AI seat's Unpatched set, sprung by the human's Book of Stats: the recast's target prompt
/// is open for the AI seat.
fn unpatched_open(seed: &str) -> GameState {
    register_cards();
    let mut s = scenario(json!({
        "seed": seed,
        "active": "p2",
        "p1": { "field": [VANILLA], "backrow": [UNPATCHED], "hand": ["core-005"], "library": LIBRARY },
        "p2": { "field": [VANILLA], "hand": [STATS, "core-005"], "library": LIBRARY },
    }));
    let theirs = s.unit_at(PlayerId::P2, 1);
    s.play(STATS, json!({ "targets": pick::instance(&theirs) }));
    assert!(s.state().pending.is_some(), "the recast's target is open");
    s.state().clone()
}

/// The AI seat holding Spaghettification against three enemy Units.
fn spaghetti_held(seed: &str) -> GameState {
    register_cards();
    let s = scenario(json!({
        "seed": seed,
        "active": "p1",
        "p1": { "hand": [SPAGHETTI, "core-005"], "library": LIBRARY },
        "p2": { "field": [VANILLA, VANILLA, VANILLA], "hand": ["core-005"], "library": LIBRARY },
    }));
    s.state().clone()
}

/// The human's Zero-Day standing while the AI seat holds Lunar Eclipse.
fn zero_day_standing(seed: &str) -> GameState {
    register_cards();
    let s = scenario(json!({
        "seed": seed,
        "active": "p1",
        "p1": { "hand": [LUNAR, "core-005"], "library": LIBRARY },
        "p2": { "field": [{ "def": ZERO_DAY }], "hand": ["core-005"], "library": LIBRARY },
    }));
    s.state().clone()
}

fn answered(seed: &str, state: &GameState) {
    let decision = decide(state, AI, &mut ai_options(seed, AI_BUDGET)).expect("a decision");
    assert!(
        is_legal(state, AI, &decision.action),
        "{seed}: {:?}",
        decision.action
    );
    assert_ne!(decision.reason, DecisionReason::Fallback, "{seed}");
    assert_eq!(decision.stats.sim_panics, 0, "{seed}");
}

#[test]
fn r4594015_the_ai_answers_unpatcheds_cast_target() {
    for k in ["ai-unpatched-k1", "ai-unpatched-k2"] {
        answered(k, &unpatched_open(k));
    }
}

#[test]
fn r4594011_the_ai_plays_spaghettification() {
    for k in ["ai-spaghetti-k1", "ai-spaghetti-k2"] {
        let state = spaghetti_held(k);
        let decision = decide(&state, AI, &mut ai_options(k, AI_BUDGET)).expect("a decision");
        let ActionBody::Play { instance_id, .. } = &decision.action else {
            panic!("{k}: the AI plays, not {:?}", decision.action);
        };
        let played = find_instance(&state, instance_id).expect("a played card");
        assert_eq!(played.def_id, SPAGHETTI, "{k}");
        assert!(is_legal(&state, AI, &decision.action), "{k}");
        assert_ne!(decision.reason, DecisionReason::Fallback, "{k}");
        assert_eq!(decision.stats.sim_panics, 0, "{k}");
    }
}

#[test]
fn r4594014_the_ai_casts_a_targeted_spell_against_zero_day() {
    for k in ["ai-zero-day-k1", "ai-zero-day-k2"] {
        let state = zero_day_standing(k);
        let lunar = state.players[AI]
            .hand
            .iter()
            .find(|card| card.def_id == LUNAR)
            .expect("lunar")
            .id
            .clone();
        let plays: Vec<ActionBody> = legal_actions(&state, AI)
            .into_iter()
            .filter(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == lunar))
            .collect();
        assert_eq!(plays.len(), 1, "{k}: listed once");
        assert!(
            matches!(&plays[0], ActionBody::Play { targets: None, .. }),
            "{k}: with no targets"
        );
        answered(k, &state);
    }
}
