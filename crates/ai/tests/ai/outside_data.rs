//! The Meditative+ batch "outside data and the autopilot" (issue #1183) meets the AI. KY's Intuition's
//! Discover offers code blocks (R1183005): the AI needs nothing new for it, since `legal_actions` lists
//! its three answers and the search tries each through `reduce`. Cognitive Offloading's autopilot
//! (R1183002, R1183003) is the engine's, never this crate's: a seat it plays for is never asked to
//! act, and a line that draws into it comes back with that turn already played.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, is_legal, register_cards, scenario};

const INTUITION: &str = "meditativeplus-025";
const OFFLOADING: &str = "meditativeplus-018";
const FILLER: &str = "core-005";

#[test]
fn r1183005_the_ai_answers_a_code_discover_with_one_of_its_places() {
    register_cards();
    let mut s = scenario(json!({
        "seed": "ai-code-discover",
        "p1": { "hand": [INTUITION, FILLER], "library": [FILLER, FILLER, FILLER] },
        "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
    }));
    s.play(INTUITION, json!({}));
    let state = s.state().clone();
    assert_eq!(state.pending.as_ref().map(|pending| pending.player_id), Some(AI));
    let decision = decide(&state, AI, &mut AiOptions::new(create_rng("ai-code-discover", 0)))
        .expect("the AI answers its prompt");
    assert!(is_legal(&state, AI, &decision.action));
    let ActionBody::Answer { selection, .. } = &decision.action else {
        panic!("the AI answers the Discover: {:?}", decision.action);
    };
    assert!(matches!(
        selection.as_slice(),
        [Selection::Mode { option }] if option.starts_with(effects::CODE_OPTION_PREFIX)
    ));
}

#[test]
fn r1183002_a_seat_the_autopilot_plays_for_is_never_asked_to_act() {
    register_cards();
    let _preview = preview_sets(&[SetName::MeditativePlus]);
    let mut s = scenario(json!({
        "seed": "ai-autopilot",
        "active": "p2",
        "p1": {
            "hand": ["core-008", "core-011"],
            "field": ["core-019"],
            "library": [OFFLOADING, FILLER, FILLER, FILLER],
        },
        "p2": { "hand": ["core-011"], "library": [FILLER, FILLER, FILLER] },
    }));
    // p2 ends the turn: p1's draws Cognitive Offloading, and the engine plays p1's turn through.
    s.end_turn();
    let state = s.state().clone();
    assert_eq!(state.active, HUMAN, "the AI's seat had its turn played for it");
    assert!(
        state
            .players
            .p1
            .graveyard
            .iter()
            .any(|card| card.def_id == OFFLOADING)
    );
    assert!(!ai_to_act(&state, AI), "nothing waits on the AI's seat");
}
