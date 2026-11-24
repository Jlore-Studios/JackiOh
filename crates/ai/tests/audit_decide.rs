//! Audit (decide): exploratory.

use jackioh_ai::{
    AI_BUDGET, AiOptions, AiStyle, Decision, SearchBudget, action_key, ai_to_act, decide, decide_styled,
};
use jackioh_engine::testkit::{
    Action, ActionBody, CatalogQueryArgs, CreateGameOptions, DECK_SIZE, GameState, PLAYER_IDS, PlayerId,
    begin_game, create_game, create_rng, json, json_as, legal_actions, query, reduce, scenario, seat_to_act,
    subsystems,
};

const AI: PlayerId = PlayerId::P1;

fn register_cards() {
    jackioh_cards::register_all();
}

fn is_legal(state: &GameState, seat: PlayerId, action: &ActionBody) -> bool {
    let key = action_key(action);
    legal_actions(state, seat)
        .iter()
        .any(|legal| action_key(legal) == key)
}

fn random_decks(seed: &str) -> (Vec<String>, Vec<String>) {
    register_cards();
    let mut core: Vec<String> = query(&json_as::<CatalogQueryArgs>(json!({ "set": "Core" })))
        .iter()
        .map(|def| def.id.clone())
        .collect();
    core.sort();
    let shuffled = create_rng(&format!("ai-test-decks:{seed}"), 0).shuffle(&core);
    let n = DECK_SIZE as usize;
    (shuffled[..n].to_vec(), shuffled[n..2 * n].to_vec())
}

fn dealt_game(seed: &str) -> GameState {
    register_cards();
    begin_game(&create_game(&CreateGameOptions {
        seed: seed.to_string(),
        decks: random_decks(seed),
        ..CreateGameOptions::default()
    }))
    .state
}

fn random_policy_states(seed: &str, every: usize, max_actions: usize) -> Vec<GameState> {
    let mut state = dealt_game(seed);
    let mut policy = create_rng(&format!("ai-test-policy:{seed}"), 0);
    let mut states: Vec<GameState> = vec![state.clone()];
    let mut n = 0;
    while state.result.is_none() && n < max_actions {
        let player = seat_to_act(&state).expect("a live game has a seat to act");
        let Some(chosen) = subsystems::choose_action(&state, player, &mut policy) else {
            break;
        };
        let result = reduce(&state, &Action::new(chosen, player, format!("rp-{n}")));
        assert!(result.error.is_none());
        state = result.state;
        if (n + 1) % every == 0 {
            states.push(state.clone());
        }
        n += 1;
    }
    states
}

fn mid_board() -> GameState {
    register_cards();
    scenario(json!({
        "seed": "audit-decide-mid",
        "active": AI,
        "turn": 9,
        "p1": {
            "hand": ["core-008", "core-011", "core-019", "core-044"],
            "field": ["core-008", "core-011"],
            "mana": 5,
            "library": ["core-008", "core-011", "core-020"],
        },
        "p2": {
            "hand": ["core-011", "core-008"],
            "field": ["core-008", "core-019"],
            "library": ["core-008", "core-011", "core-020"],
        },
    }))
    .state()
    .clone()
}

fn budgets() -> Vec<(&'static str, SearchBudget)> {
    let b = AI_BUDGET;
    let big = usize::MAX;
    vec![
        ("nodes0", SearchBudget { nodes: 0, ..b }),
        ("nodes1", SearchBudget { nodes: 1, ..b }),
        ("nodes2", SearchBudget { nodes: 2, ..b }),
        ("nodes5", SearchBudget { nodes: 5, ..b }),
        ("nodes30", SearchBudget { nodes: 30, ..b }),
        ("dets0", SearchBudget { determinizations: 0, ..b }),
        ("dets1", SearchBudget { determinizations: 1, ..b }),
        ("lethal0", SearchBudget { lethal_nodes: 0, ..b }),
        ("lethal1", SearchBudget { lethal_nodes: 1, ..b }),
        ("lethal_gt_nodes", SearchBudget { lethal_nodes: 5000, ..b }),
        ("beam0", SearchBudget { beam_width: 0, ..b }),
        ("depth0", SearchBudget { max_depth: 0, ..b }),
        ("depth1", SearchBudget { max_depth: 1, ..b }),
        ("root0", SearchBudget { root_branching: 0, ..b }),
        ("branch0", SearchBudget { branching: 0, ..b }),
        ("fin0", SearchBudget { finalists: 0, ..b }),
        ("fin1", SearchBudget { finalists: 1, ..b }),
        ("depth_max", SearchBudget { max_depth: big, ..b }),
        ("fin_max", SearchBudget { finalists: big, ..b }),
        ("root_max", SearchBudget { root_branching: big, ..b }),
        ("branch_max", SearchBudget { branching: big, ..b }),
        ("beam_max", SearchBudget { beam_width: big, ..b }),
        ("lethal_max", SearchBudget { lethal_nodes: big, ..b }),
        ("nodes_max_small", SearchBudget { nodes: big, lethal_nodes: 300, max_depth: 2, root_branching: 3, branching: 2, ..b }),
    ]
}

fn check(tag: &str, state: &GameState, seat: PlayerId, budget: SearchBudget) -> Option<Decision> {
    let mut options = AiOptions::with_budget(create_rng("audit-decide", 0), budget);
    let first = decide(state, seat, &mut options);
    let mut again = AiOptions::with_budget(create_rng("audit-decide", 0), budget);
    let second = decide(state, seat, &mut again);
    let Some(decision) = first.clone() else {
        println!("{tag}: NONE");
        return None;
    };
    let legal = is_legal(state, seat, &decision.action);
    let det = first == second && options.rng.cursor() == again.rng.cursor();
    println!(
        "{tag}: {:?} {} legal={legal} det={det} nodes={} dets={} lines={} errs={} panics={} stop={:?} replied={} worlds={} cuts={} score={} panic={:?}",
        decision.reason,
        action_key(&decision.action),
        decision.stats.nodes,
        decision.stats.determinizations,
        decision.stats.lines,
        decision.stats.sim_errors,
        decision.stats.sim_panics,
        decision.stats.stopped_by,
        decision.stats.replied,
        decision.stats.worlds_scored,
        decision.stats.replays_cut,
        decision.stats.score,
        decision.first_sim_panic,
    );
    if !legal || !det || decision.stats.sim_panics > 0 || decision.stats.nodes > budget.nodes {
        println!("  ^^^^ SUSPECT {tag}");
    }
    Some(decision)
}

#[test]
fn explore_budget_edges_mid_board() {
    let state = mid_board();
    for (name, budget) in budgets() {
        check(name, &state, AI, budget);
    }
}

#[test]
fn explore_clock_edges() {
    let state = mid_board();
    let stop = || true;
    let mut options = AiOptions::with_budget(create_rng("audit-decide", 0), AI_BUDGET);
    options.should_stop = Some(&stop);
    let d = decide(&state, AI, &mut options).expect("a decision");
    println!("should_stop: {:?} {} nodes={} stop={:?}", d.reason, action_key(&d.action), d.stats.nodes, d.stats.stopped_by);
    let mut options = AiOptions::with_budget(create_rng("audit-decide", 0), AI_BUDGET);
    options.wrap_up = Some(&stop);
    let d = decide(&state, AI, &mut options).expect("a decision");
    println!(
        "wrap_up: {:?} {} nodes={} stop={:?} lines={} replied={} worlds={} fell_short={}",
        d.reason, action_key(&d.action), d.stats.nodes, d.stats.stopped_by, d.stats.lines, d.stats.replied, d.stats.worlds_scored, d.fell_short()
    );
    let plain = decide(&state, AI, &mut AiOptions::with_budget(create_rng("audit-decide", 0), AI_BUDGET)).expect("d");
    println!("plain: {:?} {} nodes={} stop={:?} lines={}", plain.reason, action_key(&plain.action), plain.stats.nodes, plain.stats.stopped_by, plain.stats.lines);
}

#[test]
fn explore_budget_edges_real_states() {
    let mut count = 0;
    for seed in ["audit-decide-a", "audit-decide-b"] {
        for (index, state) in random_policy_states(seed, 9, 160).into_iter().enumerate() {
            for seat in PLAYER_IDS {
                if !ai_to_act(&state, seat) {
                    continue;
                }
                for (name, budget) in budgets() {
                    if matches!(name, "nodes0" | "nodes1" | "nodes5" | "dets0" | "lethal0" | "depth0" | "depth1" | "root0" | "fin0" | "depth_max" | "fin_max" | "root_max" | "lethal_max") {
                        let small = SearchBudget { nodes: budget.nodes.min(200), ..budget };
                        check(&format!("{seed}#{index}/{seat}/{name}"), &state, seat, small);
                        count += 1;
                    }
                }
            }
        }
    }
    println!("checked {count}");
}

#[test]
fn explore_styles() {
    let state = mid_board();
    for (t, m) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0), (f64::NAN, f64::NAN), (f64::INFINITY, 2.0), (-1.0, -1.0), (1e-300, 0.5)] {
        let style = AiStyle { temperature: t, missed_lethal_chance: m };
        let mut options = AiOptions::with_budget(create_rng("audit-decide", 0), AI_BUDGET);
        let d = decide_styled(&state, AI, &mut options, &style).expect("d");
        println!("style t={t} m={m}: {:?} {} cursor={} dets={} lines={}", d.reason, action_key(&d.action), options.rng.cursor(), d.stats.determinizations, d.stats.lines);
    }
}
