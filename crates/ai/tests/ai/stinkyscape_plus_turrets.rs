//! S+ #109 Lancer's tile Discover (issue #3559, R3554005): Lancer's Activate opens a `mode` prompt of
//! `DISCOVER_OPTIONS` enemy tiles for its controller, and the AI answers it like any other prompt:
//! `legalActions` lists every tile, the beam scores each through `reduce`, and the decision's reason is
//! `Prompt`. The random policy and the timeout answer it too, which the card's own tests prove.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, ai_options, clone, is_legal, register_cards, scenario};

const LANCER: &str = "stinkyscapeplus-109";
const LIBRARY: &[&str] = &["core-053", "core-030"];
const HAND: &str = "core-005";

#[test]
fn the_ai_answers_lancers_tile_discover_with_an_offered_tile() {
    register_cards();
    let seed = "stinkyscape-plus-lancer-ai";
    let mut s = scenario(json!({
        "seed": seed,
        "p1": { "field": [{ "def": LANCER, "lane": 3 }], "library": LIBRARY },
        "p2": { "hand": [HAND], "library": LIBRARY },
    }));
    s.activate(LANCER, json!({}));
    let state = s.state().clone();
    let pending = state.pending.clone().expect("the tile Discover");
    assert_eq!(
        pending.player_id, AI,
        "the Activate's controller holds the prompt"
    );
    assert_eq!(pending.options.len(), DISCOVER_OPTIONS as usize);

    let decision = decide(&clone(&state), AI, &mut ai_options(seed, AI_BUDGET)).expect("a decision");
    assert_eq!(decision.reason, DecisionReason::Prompt);
    assert!(decision.stats.nodes <= AI_BUDGET.nodes);
    assert_eq!(decision.stats.sim_errors, 0);
    assert!(is_legal(&state, AI, &decision.action));
    let ActionBody::Answer { selection, .. } = &decision.action else {
        panic!("the AI should answer the Discover: {:?}", decision.action);
    };
    assert!(
        pending
            .options
            .iter()
            .any(|option| selection.as_slice() == [option.selection.clone()]),
        "its selection is one of the offered tiles: {selection:?}"
    );
}
