//! R4595003: a simulated step pays one more node for every `AI_SEARCH.events_per_node` events it
//! emitted past `AI_SEARCH.free_events`. Behind a Suppressive Aura a summoned 1/1 Drone dies at once, Hive Overseer summons
//! another, and the loop runs to the engine's cap: one Play, thousands of events (the sweep's
//! `sweep:easy:infiniteassault-029:6`, turn 18).

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, register_cards, scenario};

const DRONE: &str = "infiniteassault-004-1";
const OVERSEER: &str = "infiniteassault-092";
const SUPPRESSIVE: &str = "core-046";
const DECK: [&str; 3] = ["core-008", "core-011", "core-020"];

#[test]
fn r4595003_a_step_that_runs_a_summon_loop_pays_a_node_per_hundred_events() {
    register_cards();
    let s = scenario(json!({
        "seed": "r4595003", "active": AI, "turn": 9,
        "p1": { "hand": [DRONE], "field": [OVERSEER], "mana": 3, "library": DECK },
        "p2": { "field": [SUPPRESSIVE], "library": DECK },
    }));
    let state = s.state();
    let id = s.card(DRONE).id.clone();
    let play = legal_actions(state, AI)
        .into_iter()
        .find(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == id))
        .expect("the Drone can be played");
    let stepped = reduce_unpiloted(state, &Action::new(play.clone(), AI, "loop".to_string()));
    assert!(stepped.error.is_none());
    let per = AI_SEARCH.events_per_node as usize;
    let free = AI_SEARCH.free_events as usize;
    assert!(
        stepped.events.len() >= free + 10 * per,
        "PREMISE: the loop runs long: {} events",
        stepped.events.len()
    );
    assert_eq!(
        event_surcharge_nodes(&stepped),
        (stepped.events.len() - free) / per
    );

    let counter = create_node_counter(AI_BUDGET.nodes, None);
    simulate(state, AI, &play, &counter, Prices::NONE)
        .expect("the counter gives the step a node")
        .expect("the step is accepted");
    assert_eq!(
        counter.used(),
        1 + (stepped.events.len() - free) / per,
        "the step's node and its events'"
    );
}
