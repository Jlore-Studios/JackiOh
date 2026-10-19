//! #1347: a Zephyrs that a step casts without playing it, from the graveyard (Spell Tyrant) or at
//! random (Call to Chaos, Jogg's Box), ranks every shipped card as a played one does (§10.7, R29),
//! and the step pays for it once it has run (`announced_surcharge_nodes`): its `cardAnnounced` says
//! what nothing in the action did. A played Zephyrs pays up front instead (`surcharge_nodes`), once.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, register_cards, scenario};

const TYRANT: &str = "classic-056";
const ZEPHYRS: &str = "core-097";
/// (1) Spell: draw 2, heal your hero 2. A cast that ranks nothing.
const STOCKPILE: &str = "core-005";
const DECK: [&str; 4] = [STOCKPILE, STOCKPILE, STOCKPILE, STOCKPILE];

/// The AI's Spell Tyrant asks which Spells of its graveyard to cast: a Zephyrs and a Stockpile.
fn tyrant_asking() -> Scenario {
    register_cards();
    let mut s = scenario(json!({
        "seed": "ai-scorer-casts",
        "p1": { "hand": [TYRANT], "graveyard": [ZEPHYRS, STOCKPILE], "library": DECK, "mana": 5 },
        "p2": { "hand": [STOCKPILE], "library": DECK },
    }));
    s.play(TYRANT, json!({}));
    assert_eq!(
        s.state().pending.as_ref().map(|pending| pending.player_id),
        Some(AI),
        "Spell Tyrant asks"
    );
    s
}

/// The listed answer that casts exactly `def_id`'s card from the graveyard.
fn casting(s: &Scenario, def_id: &str) -> ActionBody {
    let id = s.card(def_id).id.clone();
    legal_actions(s.state(), AI)
        .into_iter()
        .find(|action| {
            matches!(action, ActionBody::Answer { selection, .. }
                if serde_json::to_value(selection).ok() == Some(json!([{ "pick": "instance", "instanceId": id }])))
        })
        .expect("the answer is listed")
}

/// The nodes `simulate` takes for `action` on `state`.
fn nodes_for(state: &GameState, action: &ActionBody) -> usize {
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    simulate(state, AI, action, &counter)
        .expect("the counter gives the step a node")
        .expect("the step is accepted");
    counter.used()
}

#[test]
fn a_zephyrs_cast_from_the_graveyard_pays_what_a_played_one_does() {
    let s = tyrant_asking();
    let answer = casting(&s, ZEPHYRS);
    assert_eq!(
        surcharge_nodes(s.state(), AI, &answer),
        0,
        "nothing in the answer says so up front"
    );
    let stepped = reduce_unpiloted(s.state(), &Action::new(answer.clone(), AI, "charge".to_string()));
    assert!(stepped.error.is_none());
    let expensive = AI_SEARCH.expensive_def_nodes as usize;
    assert_eq!(announced_surcharge_nodes(s.state(), &answer, &stepped), expensive);
    assert_eq!(
        nodes_for(s.state(), &answer),
        1 + expensive,
        "the step's node and the ranking's"
    );
}

#[test]
fn a_cast_that_ranks_nothing_pays_its_one_node() {
    let s = tyrant_asking();
    let answer = casting(&s, STOCKPILE);
    let stepped = reduce_unpiloted(s.state(), &Action::new(answer.clone(), AI, "charge".to_string()));
    assert!(stepped.error.is_none());
    assert_eq!(announced_surcharge_nodes(s.state(), &answer, &stepped), 0);
    assert_eq!(nodes_for(s.state(), &answer), 1);
}

#[test]
fn a_played_zephyrs_pays_up_front_and_only_once() {
    register_cards();
    let s = scenario(json!({
        "seed": "ai-scorer-played",
        "p1": { "hand": [ZEPHYRS], "library": DECK, "mana": 5 },
        "p2": { "hand": [STOCKPILE], "library": DECK },
    }));
    let id = s.card(ZEPHYRS).id.clone();
    let play = legal_actions(s.state(), AI)
        .into_iter()
        .find(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == id))
        .expect("Zephyrs can be played");
    let expensive = AI_SEARCH.expensive_def_nodes as usize;
    assert_eq!(surcharge_nodes(s.state(), AI, &play), expensive);
    let stepped = reduce_unpiloted(s.state(), &Action::new(play.clone(), AI, "charge".to_string()));
    assert_eq!(
        announced_surcharge_nodes(s.state(), &play, &stepped),
        0,
        "the up-front charge paid for it"
    );
    assert_eq!(nodes_for(s.state(), &play), 1 + expensive);
}
