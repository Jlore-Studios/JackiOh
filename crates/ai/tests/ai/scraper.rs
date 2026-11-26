//! Scraper AI tests (C8 #4592, R4592003, R4592004, R4592005).

use jackioh_ai::observe::hidden_instance_ids;
use jackioh_ai::{AI_BUDGET, DecisionReason, decide};
use jackioh_engine::testkit::PlayerId::P1;
use jackioh_engine::testkit::*;

use super::support::{AI, ai_options, register_cards, scenario};

const GROUND_TRUTH: &str = "infiniteassault-191";
const DATA_POINT: &str = "infiniteassault-170";
const PATTERN_RECOGNITION: &str = "infiniteassault-184";
const FILLER: &str = "core-005";

#[test]
fn r4592003_a_hand_revealed_by_ground_truth_is_not_hidden_from_the_seat() {
    register_cards();
    let s = scenario(json!({
        "p1": { "field": [{ "def": GROUND_TRUTH, "lane": 1 }], "mana": 10 },
        "p2": { "hand": [FILLER, FILLER], "mana": 10 },
    }));

    let hidden = hidden_instance_ids(s.state(), P1);
    for card in s.hand(PlayerId::P2) {
        assert!(
            !hidden.contains(&card.id),
            "card {} should not be hidden when opponent hand is revealed",
            card.id
        );
    }
}

#[test]
fn r4592004_the_ai_answers_pattern_recognitions_picks() {
    register_cards();
    let mut s = scenario(json!({
        "p1": { "hand": [PATTERN_RECOGNITION], "mana": 10 },
        "p2": { "library": ["core-001", "core-008", "core-010"] },
    }));
    s.play(PATTERN_RECOGNITION, json!({}));
    assert!(s.state().pending.is_some());

    let decision = decide(s.state(), AI, &mut ai_options("scraper-pattern-1", AI_BUDGET))
        .expect("a decision on first pick");
    assert_eq!(decision.reason, DecisionReason::Prompt);
    let ActionBody::Answer {
        selection: target, ..
    } = decision.action
    else {
        panic!("expected answer action");
    };
    s.answer(serde_json::to_value(&target).unwrap());

    assert!(s.state().pending.is_some());
    let decision2 = decide(s.state(), AI, &mut ai_options("scraper-pattern-2", AI_BUDGET))
        .expect("a decision on exile pick");
    assert_eq!(decision2.reason, DecisionReason::Prompt);
    let ActionBody::Answer {
        selection: target2, ..
    } = decision2.action
    else {
        panic!("expected answer action");
    };
    s.answer(serde_json::to_value(&target2).unwrap());
    assert!(s.state().pending.is_none());
}

#[test]
fn r4592005_the_ai_answers_data_points_discover() {
    register_cards();
    let mut s = scenario(json!({
        "p1": { "hand": [DATA_POINT], "mana": 10 },
        "p2": { "library": ["core-001", "core-008", "core-010"] },
    }));
    s.play(DATA_POINT, json!({}));
    assert!(s.state().pending.is_some());

    let decision = decide(s.state(), AI, &mut ai_options("scraper-data-point", AI_BUDGET))
        .expect("a decision on discover");
    assert_eq!(decision.reason, DecisionReason::Prompt);
    let ActionBody::Answer {
        selection: target, ..
    } = decision.action
    else {
        panic!("expected answer action");
    };
    s.answer(serde_json::to_value(&target).unwrap());
    assert!(s.state().pending.is_none());
    assert_eq!(s.hand(P1).len(), 1);
}
