//! R2575000 (issue #2575): the practice AI's budget, doubled to 1,200 nodes and 300 lethal nodes in
//! the search shape of before, and what follows it.
//! R4802000 (issue #4802): a decision keeps to its caller's own budget on a state that is not fat, and a
//! budget field at any value overflows nothing.
//! R4791000 (issue #4791): a beam its budget cut short ends its open lines' turns before they are
//! scored.

use jackioh_ai::{
    AI_BUDGET, AI_GATE_BUDGET, AI_SEARCH, Decision, DecisionReason, LETHAL_HINT_DETERMINIZATIONS,
    LETHAL_HINT_NODES, NodeCounter, Prices, SearchBudget, StoppedBy, beam_search, create_beam_counter,
    create_node_counter, decide, hidden_card_ids, redact, simulate_reply,
};
use jackioh_engine::testkit::{ActionBody, CardInstance, GameState, create_rng, json, legal_actions};

use super::support::{AI, TEST_BUDGET, act, ai_options, det, in_graveyard, js, register_cards, scenario};

#[test]
fn r2575000_the_practice_budget_is_1200_nodes_300_of_them_lethal_in_the_search_shape_of_before() {
    assert_eq!(
        AI_BUDGET,
        SearchBudget {
            nodes: 1200,
            lethal_nodes: 300,
            determinizations: 3,
            beam_width: 4,
            root_branching: 20,
            branching: 6,
            max_depth: 8,
            finalists: 3,
        }
    );
}

#[test]
fn r2575000_the_gates_the_sweep_and_the_lethal_hint_follow_the_practice_budget() {
    assert_eq!(AI_GATE_BUDGET, AI_BUDGET);
    assert_eq!(LETHAL_HINT_NODES, AI_BUDGET.lethal_nodes);
    assert_eq!(LETHAL_HINT_DETERMINIZATIONS, AI_BUDGET.determinizations);
}

#[test]
fn r2575000_a_fat_state_scales_from_the_600_nodes_of_before() {
    assert_eq!(AI_SEARCH.fat_budget_nodes, 600);
    assert!((AI_SEARCH.fat_budget_nodes as usize) < AI_BUDGET.nodes);
}

/// R4802000: a mid-game board well under `AI_SEARCH.fat_reference`'s width (12 cards, 7 candidates), with
/// no lethal on it.
fn narrow_board() -> GameState {
    register_cards();
    scenario(json!({
        "seed": "audit-decide-mid", "active": AI, "turn": 9,
        "p1": { "hand": ["core-008", "core-011", "core-019", "core-044"], "field": ["core-008", "core-011"],
                "mana": 5, "library": ["core-008", "core-011", "core-020"] },
        "p2": { "hand": ["core-011", "core-008"], "field": ["core-008", "core-019"],
                "library": ["core-008", "core-011", "core-020"] },
    }))
    .state()
    .clone()
}

fn decided(state: &GameState, budget: SearchBudget) -> Decision {
    decide(state, AI, &mut ai_options("audit-decide", budget)).expect("a decision")
}

/// The nodes the lethal solver spent on `narrow_board` under `budget`: with `max_depth` 0 the beam takes
/// none, so a decision's nodes are its prices' and the solver's, and the prices' are what the same
/// decision spends with `lethal_nodes` 0.
fn solver_nodes(budget: SearchBudget) -> usize {
    let state = narrow_board();
    let no_beam = SearchBudget {
        max_depth: 0,
        ..budget
    };
    let prices = decided(
        &state,
        SearchBudget {
            lethal_nodes: 0,
            ..no_beam
        },
    )
    .stats
    .nodes;
    decided(&state, no_beam).stats.nodes - prices
}

#[test]
fn r4802000_lethal_nodes_caps_the_solver_on_a_state_that_is_not_fat() {
    assert_eq!(
        solver_nodes(SearchBudget {
            lethal_nodes: 300,
            ..AI_BUDGET
        }),
        300,
        "PREMISE: the solver spends its allowance here"
    );
    let spent = solver_nodes(SearchBudget {
        lethal_nodes: 10,
        ..AI_BUDGET
    });
    assert!(spent <= 10, "lethal_nodes 10 of 1200: the solver spent {spent}");
}

#[test]
fn r4802000_a_150_node_budget_keeps_its_solver_to_its_40_nodes() {
    let spent = solver_nodes(TEST_BUDGET);
    assert!(spent <= TEST_BUDGET.lethal_nodes, "the solver spent {spent}");
}

#[test]
fn r4802000_a_small_budget_still_leaves_the_beam_its_nodes() {
    let state = narrow_board();
    let alone = decided(
        &state,
        SearchBudget {
            nodes: 30,
            lethal_nodes: 0,
            ..AI_BUDGET
        },
    );
    assert_eq!(
        alone.reason,
        DecisionReason::Search,
        "PREMISE: 30 nodes search this board"
    );
    let small = decided(
        &state,
        SearchBudget {
            nodes: 30,
            lethal_nodes: 7,
            ..AI_BUDGET
        },
    );
    assert!(
        small.stats.lines > 0,
        "7 of 30 nodes were the solver's, yet the beam searched no line"
    );
    assert_ne!(small.reason, DecisionReason::Fallback);
}

#[test]
fn r4802000_a_budget_field_at_its_largest_value_does_not_overflow() {
    let state = narrow_board();
    let big = usize::MAX;
    let cases = [
        (
            "max_depth",
            SearchBudget {
                max_depth: big,
                ..AI_BUDGET
            },
        ),
        (
            "finalists",
            SearchBudget {
                finalists: big,
                ..AI_BUDGET
            },
        ),
        (
            "root_branching",
            SearchBudget {
                root_branching: big,
                ..AI_BUDGET
            },
        ),
        (
            "branching",
            SearchBudget {
                branching: big,
                ..AI_BUDGET
            },
        ),
        (
            "lethal_nodes",
            SearchBudget {
                lethal_nodes: big,
                ..AI_BUDGET
            },
        ),
    ];
    let mut failed: Vec<String> = Vec::new();
    for (name, budget) in cases {
        let decision = decided(&state, budget);
        if decision.stats.sim_panics > 0 {
            failed.push(format!(
                "{name}: {:?}, {:?}",
                decision.reason, decision.first_sim_panic
            ));
        }
    }
    assert!(failed.is_empty(), "{failed:#?}");
}

/// R4791000: a card nobody can cast, so the libraries' draws change nothing.
const INERT: &str = "stand-in (9) 1/1";
/// R4791000: the one-drop whose line leaves two crystals unspent, and which the enemy's 3/5 kills.
const FOUR_ONE: &str = "stand-in (1) 4/1";

/// R4791000: turn 5's three crystals, a (1) 4/1 and a (3) 2/4 in hand, and the enemy's (4) 3/5 on the
/// field, which survives the 4/1 and kills it in the opponent's reply.
fn cut_board() -> GameState {
    register_cards();
    scenario(json!({
        "seed": "ai-cut-beam", "active": AI, "turn": 5,
        "p1": { "hand": [FOUR_ONE, "stand-in (3) 2/4"], "mana": 3, "library": [INERT, INERT, INERT] },
        "p2": { "field": ["stand-in (4) 3/5"], "hand": [INERT], "library": [INERT, INERT] },
    }))
    .state()
    .clone()
}

/// R4791000: the decision on `state` at `nodes`, one sampled world and a lethal solver of 4 nodes, so
/// that 20 and 25 nodes cut the beam at its first depth and 30 do not.
fn cut_decision(state: &GameState, nodes: usize) -> Decision {
    let budget = SearchBudget {
        nodes,
        lethal_nodes: 4,
        determinizations: 1,
        ..AI_BUDGET
    };
    decide(state, AI, &mut ai_options("cut-beam", budget)).expect("a decision")
}

fn plays(action: &ActionBody, card: &CardInstance) -> bool {
    matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == card.id)
}

#[test]
fn r4791000_a_beam_its_budget_cuts_plays_what_a_full_budget_plays_not_its_open_lines_first_action() {
    let state = cut_board();
    let four_one = &state.players[AI].hand[0];
    let two_four = &state.players[AI].hand[1];

    // PREMISE: the 4/1's line leaves two of the three crystals unspent, and the reply kills it.
    let play = legal_actions(&state, AI)
        .into_iter()
        .find(|action| plays(action, four_one))
        .expect("the 4/1 is playable");
    let ended = act(&act(&state, AI, &play), AI, &ActionBody::EndTurn);
    assert_eq!(ended.players[AI].turn_log.unspent_at_end, Some(2));
    let replied = simulate_reply(
        &ended,
        AI,
        &create_node_counter(50, None),
        &hidden_card_ids(&ended, AI),
        Prices::NONE,
    )
    .expect("a reply");
    assert!(in_graveyard(&replied, AI, FOUR_ONE), "the reply kills the 4/1");

    let full = cut_decision(&state, AI_BUDGET.nodes);
    assert_eq!(js(full.stats.stopped_by), json!("exhausted"));
    assert!(
        plays(&full.action, two_four),
        "a full budget plays the 2/4: {:?}",
        full.action
    );

    // Cut at its first depth, the beam left the 4/1's line open, unpriced and unreplied, and played it.
    for nodes in [20, 25] {
        let cut = cut_decision(&state, nodes);
        assert_eq!(
            js(cut.stats.stopped_by),
            json!("budget"),
            "{nodes} nodes cut the beam"
        );
        assert!(cut.stats.replied > 0, "{nodes} nodes: {:?}", cut.stats);
        assert!(
            plays(&cut.action, two_four),
            "{nodes} nodes play what a full budget plays: {:?}",
            cut.action
        );
    }
}

#[test]
fn r4791000_a_decision_whose_beam_is_not_cut_spends_the_nodes_it_did() {
    let state = cut_board();
    for nodes in [30, 40, 80, AI_BUDGET.nodes] {
        let decision = cut_decision(&state, nodes);
        assert_eq!(js(decision.stats.stopped_by), json!("exhausted"), "{nodes} nodes");
        // Measured on the tree before R4791000.
        assert_eq!(decision.stats.nodes, 29, "{nodes} nodes");
        assert_eq!(decision.stats.replied, 6, "{nodes} nodes");
    }
}

#[test]
fn r4791000_a_beam_cut_mid_depth_files_no_line_twice() {
    register_cards();
    let state = scenario(json!({
        "seed": "ai-cut-beam-wide", "active": AI, "turn": 5,
        "p1": { "hand": [FOUR_ONE, "stand-in (2) 2/3", "stand-in (3) 3/3"], "mana": 3,
                "library": [INERT, INERT, INERT] },
        "p2": { "field": ["stand-in (4) 2/4"], "hand": [INERT], "library": [INERT, INERT] },
    }))
    .state()
    .clone();
    let world = det(&redact(&state, AI), AI, &mut create_rng("r4791000", 0));
    let mut cuts = 0;
    for slice in 1..=40 {
        let root = create_node_counter(10_000, None);
        let beam = create_beam_counter(&root, slice, None);
        let lines = beam_search(&world, AI, &beam, AI_BUDGET, Prices::NONE);
        if beam.stopped_by() != StoppedBy::Budget {
            continue;
        }
        cuts += 1;
        // A line the beam finished expanding filed its End turn; filed again, it takes a second place.
        for line in &lines {
            let mut ended = line.actions.clone();
            ended.push(ActionBody::EndTurn);
            assert!(
                !lines.iter().any(|other| other.actions == ended),
                "a slice of {slice}: {:?} filed twice",
                line.actions
            );
        }
    }
    assert!(cuts > 0, "PREMISE: a slice cuts the beam");
}
