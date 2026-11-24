//! Audit (decide): exploratory.

use jackioh_ai::{
    AI_BUDGET, AI_DECISION_PRICE_NODES, AiDeckOptions, AiOptions, AiStyle, Decision, DecisionReason, GREEDY_EVAL, NextSwing, Prices,
    SearchBudget, action_key, ai_to_act, build_ai_deck, candidate_actions, create_node_counter, decide,
    decide_styled, decision_prices, det_ground, evaluate, greedy_action, probe_world, redact,
    simulate_first_answers, skill_miss_action,
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


const WEAKEST: SearchBudget = SearchBudget {
    nodes: 150,
    lethal_nodes: 40,
    determinizations: 1,
    beam_width: 2,
    root_branching: 10,
    branching: 4,
    max_depth: 6,
    finalists: 3,
};

fn ai_decks(seed: &str) -> (Vec<String>, Vec<String>) {
    register_cards();
    let mut rng = create_rng(&format!("audit-decks:{seed}"), 0);
    let a = build_ai_deck(&mut rng, DECK_SIZE, &AiDeckOptions::default());
    let b = build_ai_deck(&mut rng, DECK_SIZE, &AiDeckOptions::default());
    (a, b)
}

fn policy_states(seed: &str, every: usize, max_actions: usize) -> Vec<GameState> {
    register_cards();
    let mut state = begin_game(&create_game(&CreateGameOptions {
        seed: seed.to_string(),
        decks: ai_decks(seed),
        ..CreateGameOptions::default()
    }))
    .state;
    let mut policy = create_rng(&format!("audit-policy:{seed}"), 0);
    let mut states: Vec<GameState> = vec![state.clone()];
    let mut n = 0;
    while state.result.is_none() && n < max_actions {
        let Some(player) = seat_to_act(&state) else { break };
        let Some(chosen) = subsystems::choose_action(&state, player, &mut policy) else {
            break;
        };
        let result = reduce(&state, &Action::new(chosen, player, format!("rp-{n}")));
        if result.error.is_some() { break; }
        state = result.state;
        if (n + 1) % every == 0 {
            states.push(state.clone());
        }
        n += 1;
    }
    states
}

#[test]
fn explore_weakest_bot_budget() {
    let mut reasons: std::collections::BTreeMap<String, usize> = Default::default();
    let mut total = 0;
    for g in 0..6 {
        let seed = format!("audit-weak-{g}");
        for (index, state) in policy_states(&seed, 5, 400).into_iter().enumerate() {
            for seat in PLAYER_IDS {
                if !ai_to_act(&state, seat) {
                    continue;
                }
                let public = redact(&state, seat);
                let ground = det_ground(&public, seat);
                let probe = probe_world(&public, seat, &ground);
                let cands = candidate_actions(&probe, seat, Prices::NONE).len();
                let pc = create_node_counter(AI_DECISION_PRICE_NODES, None);
                let _ = decision_prices(&probe, seat, &pc);
                use jackioh_ai::NodeCounter;
                let price_nodes = pc.used();
                let mut options = AiOptions::with_budget(create_rng("audit-weak", 0), WEAKEST);
                let d = decide(&state, seat, &mut options).expect("d");
                let mut again = AiOptions::with_budget(create_rng("audit-weak", 0), WEAKEST);
                let d2 = decide(&state, seat, &mut again).expect("d");
                total += 1;
                *reasons.entry(format!("{:?}/{:?}", d.reason, d.stats.stopped_by)).or_default() += 1;
                let legal = is_legal(&state, seat, &d.action);
                if d != d2 || !legal || d.stats.sim_panics > 0 || d.stats.nodes > WEAKEST.nodes || (d.reason == DecisionReason::Fallback) || price_nodes > 40 {
                    println!(
                        "{seed}#{index}/{seat} turn={} cands={cands} price_nodes={price_nodes}: {:?} {} legal={legal} det={} nodes={} lines={} errs={} panics={} stop={:?} replied={} worlds={} panic={:?}",
                        state.turn, d.reason, action_key(&d.action), d == d2, d.stats.nodes, d.stats.lines, d.stats.sim_errors, d.stats.sim_panics, d.stats.stopped_by, d.stats.replied, d.stats.worlds_scored, d.first_sim_panic
                    );
                }
            }
        }
    }
    println!("total {total} {reasons:?}");
}

#[test]
fn explore_lethal_floor() {
    let state = mid_board();
    for (nodes, lethal) in [(30usize, 0usize), (30, 7), (60, 15), (100, 10), (150, 40), (1200, 10)] {
        let b = SearchBudget { nodes, lethal_nodes: lethal, ..AI_BUDGET };
        let d = decide(&state, AI, &mut AiOptions::with_budget(create_rng("audit-floor", 0), b)).expect("d");
        let b0 = SearchBudget { max_depth: 0, ..b };
        let d0 = decide(&state, AI, &mut AiOptions::with_budget(create_rng("audit-floor", 0), b0)).expect("d");
        println!("nodes={nodes} lethal={lethal}: {:?} {} nodes={} lines={} stop={:?} | depth0 nodes={}", d.reason, action_key(&d.action), d.stats.nodes, d.stats.lines, d.stats.stopped_by, d0.stats.nodes);
    }
}

#[test]
fn explore_one_ply_surcharge() {
    register_cards();
    let state = scenario(json!({
        "seed": "audit-zeal",
        "active": AI,
        "turn": 9,
        "p1": { "hand": ["classicplus-027", "core-008", "core-011"], "mana": 4, "library": ["core-008", "core-011", "core-020"] },
        "p2": { "hand": ["core-011"], "field": ["core-008"], "library": ["core-008", "core-011", "core-020"] },
    }))
    .state()
    .clone();
    let cands = candidate_actions(&state, AI, Prices::NONE);
    println!("cands {}", cands.len());
    let big = create_node_counter(10_000, None);
    let standing = evaluate(&state, AI, NextSwing::Enemy, &GREEDY_EVAL);
    for c in &cands {
        let r = simulate_first_answers(&state, AI, c, &big);
        let score = match &r { Some(Ok(next)) => evaluate(next, AI, NextSwing::Enemy, &GREEDY_EVAL), _ => f64::NAN };
        println!("  {} -> {score} (standing {standing}) ok={}", action_key(c), matches!(r, Some(Ok(_))));
    }
    let g = greedy_action(&state, AI, &mut create_rng("audit-zeal", 0));
    println!("greedy: {:?}", g.as_ref().map(action_key));
    let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
    let mut rng = create_rng("audit-zeal-miss", 0);
    for _ in 0..200 {
        let a = skill_miss_action(&state, AI, &mut rng).expect("a");
        *counts.entry(action_key(&a)).or_default() += 1;
    }
    println!("miss: {counts:?}");
}
