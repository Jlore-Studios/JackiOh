//! Audit scratch: the static combat walk against the engine (exploration; trimmed before the report).

#![allow(dead_code)]

use jackioh_ai::*;
use jackioh_engine::testkit::*;

const AI: PlayerId = PlayerId::P1;
const HUMAN: PlayerId = PlayerId::P2;
const WIN: i32 = 1000;

fn register() {
    jackioh_cards::register_all();
}

fn stand_in(attack: i32, health: i32, keywords: &[&str]) -> String {
    if keywords.is_empty() {
        format!("stand-in (1) {attack}/{health}")
    } else {
        format!("stand-in (1) {attack}/{health} {}", keywords.join(", "))
    }
}

/// The most the seat's attacks alone can take off the enemy hero's life from `state`, every order and
/// every target tried through `reduce`; `WIN` when some order wins the game.
fn brute(state: &GameState, seat: PlayerId, root_life: i32, depth: u32, nodes: &mut u64) -> i32 {
    let opp = seat.opponent();
    if let Some(result) = &state.result {
        return if result.winner.player() == Some(seat) { WIN } else { 0 };
    }
    let mut best = root_life - effective_life(state, opp);
    if state.pending.is_some() || state.active != seat || state.phase != Phase::Main {
        return best;
    }
    for action in legal_actions(state, seat) {
        if !matches!(action, ActionBody::Attack { .. }) {
            continue;
        }
        *nodes += 1;
        let stepped = reduce(state, &Action::new(action, seat, format!("b{depth}-{nodes}")));
        if stepped.error.is_some() {
            continue;
        }
        best = best.max(brute(&stepped.state, seat, root_life, depth + 1, nodes));
        if best >= WIN {
            return best;
        }
    }
    best
}

struct Board {
    desc: String,
    state: GameState,
}

const ATT_KW: [&str; 12] = [
    "Windfury",
    "Frenzy",
    "Ranged",
    "Flying",
    "Trample",
    "Pierce",
    "Poisonous",
    "Lifesteal",
    "Cleave",
    "First Strike",
    "Divine Shield",
    "Armor 1",
];
const DEF_KW: [&str; 10] = [
    "Armor 1",
    "Armor 2",
    "Divine Shield",
    "First Strike",
    "Reborn",
    "Poisonous",
    "Indestructible",
    "Flying",
    "Ranged",
    "Pierce",
];

fn random_board(seed: &str, max_att: i32, multi: bool) -> Board {
    let mut rng = create_rng(seed, 0);
    let mut desc = String::new();
    let mut lanes: Vec<i32> = rng.shuffle(&[1, 2, 3, 4, 5]);
    let n_att = 1 + rng.int(max_att);
    let mut p1: Vec<Value> = Vec::new();
    for _ in 0..n_att {
        let attack = 1 + rng.int(6);
        let health = 1 + rng.int(6);
        let mut kws: Vec<&str> = Vec::new();
        for kw in ATT_KW {
            if rng.chance(0.18) {
                if !multi && (kw == "Windfury" || kw == "Frenzy") {
                    continue;
                }
                kws.push(kw);
            }
        }
        let lane = lanes.pop().unwrap();
        let def = stand_in(attack, health, &kws);
        desc.push_str(&format!("A[{lane}] {def}; "));
        p1.push(json!({ "def": def, "lane": lane }));
    }
    let mut lanes: Vec<i32> = rng.shuffle(&[1, 2, 3, 4, 5]);
    let n_def = rng.int(4);
    let mut p2: Vec<Value> = Vec::new();
    for _ in 0..n_def {
        let attack = rng.int(6);
        let health = 1 + rng.int(7);
        let mut kws: Vec<&str> = Vec::new();
        if rng.chance(0.75) {
            kws.push("Taunt");
        }
        for kw in DEF_KW {
            if rng.chance(0.15) {
                if kw.starts_with("Armor") && kws.iter().any(|k| k.starts_with("Armor")) {
                    continue;
                }
                kws.push(kw);
            }
        }
        let lane = lanes.pop().unwrap();
        let def = stand_in(attack, health, &kws);
        let position = if rng.chance(0.2) { "DEF" } else { "ATK" };
        desc.push_str(&format!("D[{lane}{}] {def}; ", if position == "DEF" { " DEF" } else { "" }));
        p2.push(json!({ "def": def, "lane": lane, "position": position }));
    }
    let health = 4 + rng.int(24);
    let armor = if rng.chance(0.2) { 1 + rng.int(2) } else { 0 };
    desc.push_str(&format!("hero {health} armor {armor}"));
    let mut s = scenario(json!({
        "seed": seed,
        "active": AI,
        "turn": 9,
        "p1": { "field": p1, "library": ["core-008", "core-008"] },
        "p2": { "field": p2, "health": health, "armor": armor, "library": ["core-008", "core-008"] },
    }));
    for player in PLAYER_IDS {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    Board {
        desc,
        state: s.state().clone(),
    }
}


fn explore(tag: &str, max_att: i32, multi: bool, over_only: bool) {
    register();
    let mut over = 0;
    let mut under = 0;
    let total: u32 = std::env::var("AUDIT_N").ok().and_then(|n| n.parse().ok()).unwrap_or(300);
    for n in 0..total {
        let board = random_board(&format!("audit-{tag}-{n}"), max_att, multi);
        let life = effective_life(&board.state, HUMAN);
        let gap = ready_gap(&board.state, AI);
        let walk = life - gap as i32;
        let mut nodes = 0;
        let truth = brute(&board.state, AI, life, 0, &mut nodes);
        let walk_wins = walk >= life;
        let truth_wins = truth >= WIN;
        if walk_wins && truth_wins {
            continue;
        }
        if walk > truth && !(truth_wins) {
            over += 1;
            println!("OVER  {tag}#{n}: walk {walk} > truth {truth} (life {life}) :: {}", board.desc);
        } else if walk < truth.min(life) && !over_only {
            under += 1;
            println!("UNDER {tag}#{n}: walk {walk} < truth {truth} (life {life}) :: {}", board.desc);
        }
    }
    println!("{tag}: over {over}, under {under} of {total}");
}

#[test]
fn explore_one_attacker() {
    explore("one", 1, false, false);
}

#[test]
fn explore_one_attacker_multi() {
    explore("onemulti", 1, true, false);
}

#[test]
fn explore_many_over() {
    explore("many", 3, true, true);
}

/// `brute`, returning the best line too.
fn brute_line(state: &GameState, seat: PlayerId, root_life: i32, depth: u32) -> (i32, Vec<String>) {
    let opp = seat.opponent();
    if let Some(result) = &state.result {
        return (if result.winner.player() == Some(seat) { WIN } else { 0 }, vec![]);
    }
    let mut best = (root_life - effective_life(state, opp), vec![]);
    if state.pending.is_some() || state.active != seat || state.phase != Phase::Main {
        return best;
    }
    for action in legal_actions(state, seat) {
        let ActionBody::Attack { attacker_id, target_id } = &action else {
            continue;
        };
        let label = format!("{attacker_id}->{target_id}");
        let stepped = reduce(state, &Action::new(action.clone(), seat, format!("bl{depth}")));
        if stepped.error.is_some() {
            continue;
        }
        let (value, mut line) = brute_line(&stepped.state, seat, root_life, depth + 1);
        if value > best.0 {
            line.insert(0, label);
            best = (value, line);
        }
    }
    best
}

#[test]
fn trace_one() {
    register();
    let tag = std::env::var("AUDIT_TAG").unwrap_or("onemulti".into());
    let n: u32 = std::env::var("AUDIT_ONE").ok().and_then(|n| n.parse().ok()).unwrap_or(97);
    let multi = tag != "one";
    let max_att = if tag == "many" { 3 } else { 1 };
    let board = random_board(&format!("audit-{tag}-{n}"), max_att, multi);
    let life = effective_life(&board.state, HUMAN);
    println!("{}", board.desc);
    for player in PLAYER_IDS {
        for unit in active_units_of(&board.state, player) {
            let view = unit_view(&board.state, unit);
            println!("  {player} {} {} {}/{} armor {} {:?}", unit.id, unit.def_id, view.attack, view.health, view.armor, view.keywords.iter().map(|k| k.kind().as_str()).collect::<Vec<_>>());
        }
    }
    println!("walk {} truth {:?}", life - ready_gap(&board.state, AI) as i32, brute_line(&board.state, AI, life, 0));
}

/// The same random board mirrored: p2's units attack next turn, p1 (on turn) holds the defenders.
fn random_threat_board(seed: &str, max_att: i32, multi: bool) -> Board {
    let mut rng = create_rng(seed, 0);
    let mut desc = String::new();
    let mut lanes: Vec<i32> = rng.shuffle(&[1, 2, 3, 4, 5]);
    let n_att = 1 + rng.int(max_att);
    let mut att: Vec<Value> = Vec::new();
    for _ in 0..n_att {
        let attack = rng.int(7);
        let health = 1 + rng.int(6);
        let mut kws: Vec<&str> = Vec::new();
        for kw in ATT_KW.iter().chain(["Deft", "Can't attack", "Rush", "Charge", "Brittle 1", "Reborn"].iter()) {
            if rng.chance(0.15) {
                if !multi && (*kw == "Windfury" || *kw == "Frenzy") {
                    continue;
                }
                kws.push(kw);
            }
        }
        let lane = lanes.pop().unwrap();
        let def = stand_in(attack, health, &kws);
        let position = if rng.chance(0.15) { "DEF" } else { "ATK" };
        desc.push_str(&format!("A[{lane}{}] {def}; ", if position == "DEF" { " DEF" } else { "" }));
        att.push(json!({ "def": def, "lane": lane, "position": position }));
    }
    let mut lanes: Vec<i32> = rng.shuffle(&[1, 2, 3, 4, 5]);
    let n_def = rng.int(4);
    let mut dfd: Vec<Value> = Vec::new();
    for _ in 0..n_def {
        let attack = rng.int(6);
        let health = 1 + rng.int(7);
        let mut kws: Vec<&str> = Vec::new();
        if rng.chance(0.75) {
            kws.push("Taunt");
        }
        for kw in DEF_KW {
            if rng.chance(0.15) {
                if kw.starts_with("Armor") && kws.iter().any(|k| k.starts_with("Armor")) {
                    continue;
                }
                kws.push(kw);
            }
        }
        let lane = lanes.pop().unwrap();
        let def = stand_in(attack, health, &kws);
        let position = if rng.chance(0.2) { "DEF" } else { "ATK" };
        desc.push_str(&format!("D[{lane}{}] {def}; ", if position == "DEF" { " DEF" } else { "" }));
        dfd.push(json!({ "def": def, "lane": lane, "position": position }));
    }
    let health = 4 + rng.int(24);
    let armor = if rng.chance(0.2) { 1 + rng.int(2) } else { 0 };
    desc.push_str(&format!("hero {health} armor {armor}"));
    let mut s = scenario(json!({
        "seed": seed,
        "active": AI,
        "turn": 9,
        "p1": { "field": dfd, "health": health, "armor": armor, "library": ["core-008", "core-008"] },
        "p2": { "field": att, "library": ["core-008", "core-008"] },
    }));
    for player in PLAYER_IDS {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    Board {
        desc,
        state: s.state().clone(),
    }
}

fn explore_threat(tag: &str, max_att: i32, multi: bool, over_only: bool) {
    register();
    let mut over = 0;
    let mut under = 0;
    let total: u32 = std::env::var("AUDIT_N").ok().and_then(|n| n.parse().ok()).unwrap_or(300);
    for n in 0..total {
        let board = random_threat_board(&format!("audit-{tag}-{n}"), max_att, multi);
        let life = effective_life(&board.state, AI);
        let walk = face_threat(&board.state, HUMAN);
        let ended = reduce(&board.state, &Action::new(ActionBody::EndTurn, AI, "end"));
        assert!(ended.error.is_none(), "{:?}", ended.error);
        let next = ended.state;
        assert!(next.active == HUMAN && next.phase == Phase::Main && next.pending.is_none(), "{}", board.desc);
        let mut nodes = 0;
        let truth = brute(&next, HUMAN, life, 0, &mut nodes);
        let walk_wins = walk >= life;
        let truth_wins = truth >= WIN;
        if walk_wins && truth_wins {
            continue;
        }
        if walk > truth && !(truth_wins) {
            over += 1;
            println!("OVER  {tag}#{n}: walk {walk} > truth {truth} (life {life}) :: {}", board.desc);
        } else if walk < truth.min(life) && !over_only {
            under += 1;
            println!("UNDER {tag}#{n}: walk {walk} < truth {truth} (life {life}) :: {}", board.desc);
        }
    }
    println!("{tag}: over {over}, under {under} of {total}");
}

#[test]
fn explore_threat_one() {
    explore_threat("t-one", 1, false, false);
}

#[test]
fn explore_threat_many_over() {
    explore_threat("t-many", 3, true, true);
}

// ---- lead 1: the solver's visited set reads no lane ----

const DUELIST: &str = "core-045"; // Deft Duelist, 4/3 Charge, Deft
const DFENDER: &str = "core-001"; // Big D-fender, 0/7: declares no attack (R7)
const MENACE: &str = "core-019"; // Midrange Menace, 9/9 Taunt
const GAUNTLET: &str = "stinkyscape-157";

fn play_to(state: &GameState, def: &str, lane: i32) -> ActionBody {
    let card = state.players[AI].hand.iter().find(|card| card.def_id == def).expect("in hand");
    legal_actions(state, AI)
        .into_iter()
        .find(|action| {
            matches!(action, ActionBody::Play { instance_id, zone: Some(zone), face_down: None, .. }
                if *instance_id == card.id && zone.lane == lane && zone.row == Row::Units)
        })
        .unwrap_or_else(|| panic!("no play of {def} into lane {lane}"))
}

fn wins_by(state: &GameState, lane: i32) -> bool {
    let played = reduce(state, &Action::new(play_to(state, DUELIST, lane), AI, "p"));
    assert!(played.error.is_none(), "{:?}", played.error);
    let duelist = active_units_of(&played.state, AI).into_iter().find(|u| u.def_id == DUELIST).unwrap().id.clone();
    let hit = reduce(
        &played.state,
        &Action::new(ActionBody::Attack { attacker_id: duelist, target_id: "hero-p2".into() }, AI, "a"),
    );
    hit.error.is_none() && hit.state.result.as_ref().is_some_and(|r| r.winner.player() == Some(AI))
}

fn slashed_board() -> GameState {
    register();
    let mut s = scenario(json!({
        "seed": "audit-lanes", "active": AI, "turn": 9,
        "p1": { "hand": [DUELIST], "field": [{ "def": DFENDER, "lane": 1 }], "library": ["core-008", "core-008"] },
        "p2": { "field": [{ "def": MENACE, "lane": 2 }], "health": 4, "library": ["core-008", "core-008"] },
    }));
    s.state_mut().slashed_lane = Some(3);
    s.state().clone()
}

fn gauntlet_board() -> GameState {
    register();
    scenario(json!({
        "seed": "audit-lanes", "active": AI, "turn": 9,
        "p1": { "hand": [DUELIST], "backrow": [GAUNTLET],
                "field": [{ "def": DFENDER, "lane": 1 }, { "def": DFENDER, "lane": 2 }, { "def": DFENDER, "lane": 3 }],
                "library": ["core-008", "core-008"] },
        "p2": { "health": 4, "library": ["core-008", "core-008"] },
    }))
    .state()
    .clone()
}

fn report(tag: &str, state: &GameState, good: i32, bad: i32) {
    println!("{tag}: wins by lane {good}: {}, by lane {bad}: {}", wins_by(state, good), wins_by(state, bad));
    let plays: Vec<String> = candidate_actions(state, AI, Prices::NONE)
        .iter()
        .filter(|a| matches!(a, ActionBody::Play { .. }))
        .map(action_key)
        .collect();
    println!("{tag}: candidate plays {plays:?}");
    let counter = create_node_counter(AI_BUDGET.lethal_nodes, None);
    let line = find_lethal(std::slice::from_ref(state), AI, &counter, AI_BUDGET.lethal_nodes, Prices::NONE);
    println!("{tag}: find_lethal {:?} used {}", line.as_ref().map(|l| l.iter().map(action_key).collect::<Vec<_>>()), counter.used());
    println!("{tag}: lethal_hint {}", lethal_hint(state, AI, None));
    let mut options = AiOptions::with_budget(create_rng("audit-lanes", 0), AI_BUDGET);
    let decision = decide(state, AI, &mut options).expect("a decision");
    println!("{tag}: decide {:?} {}", decision.reason, action_key(&decision.action));
}

#[test]
fn lead_lanes_report() {
    report("slash", &slashed_board(), 4, 2);
    report("gauntlet", &gauntlet_board(), 5, 4);
}

#[test]
fn lead_lanes_turn() {
    for (tag, state) in [("slash", slashed_board()), ("gauntlet", gauntlet_board())] {
        let mut ai = AiOptions::with_budget(create_rng("audit-lanes-turn", 0), AI_BUDGET);
        let turn = play_ai_turn(&state, AI, &mut ai);
        let trace: Vec<String> = turn.decisions.iter().map(|d| format!("{:?}:{}", d.reason, action_key(&d.action))).collect();
        println!("{tag}: result {:?} life {} :: {trace:?}", turn.state.result.as_ref().map(|r| r.winner.player()), turn.state.players[HUMAN].hero.health);
    }
}
