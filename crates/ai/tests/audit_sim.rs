//! Audit of move generation and simulation (`candidates.rs`, `simulate.rs`): probes.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

const AI: PlayerId = PlayerId::P1;

const WILDFIRE: &str = "classic-055";
const ECHO: &str = "classic-057";
const COIN: &str = "core-t-coin";
const GIGA_IFY: &str = "stinkyscapeplus-023";
const SILENCE: &str = "classicplus-009";
const SECOND_WIND: &str = "classic-028";
const CHURNWALKER: &str = "stinkyscapeplus-007";
const SHIP_SMITH: &str = "stinkyscape-017";
const TRAP: &str = "core-018";
const WARAXE: &str = "meditative-066";
const SEVEN: &str = "core-025";
const VANILLA: &str = "core-008";

fn id_of(state: &GameState, def_id: &str) -> String {
    let side = &state.players[AI];
    side.hand
        .iter()
        .chain(side.graveyard.iter())
        .find(|card| card.def_id == def_id)
        .unwrap_or_else(|| panic!("no {def_id}"))
        .id
        .clone()
}

/// The seat's legal plays of `def_id` (from hand or graveyard) that `pick` accepts.
fn plays(state: &GameState, def_id: &str) -> Vec<ActionBody> {
    let id = id_of(state, def_id);
    legal_actions(state, AI)
        .into_iter()
        .filter(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == id))
        .collect()
}

fn aims_at(action: &ActionBody, target: &str) -> bool {
    match action {
        ActionBody::Play { targets: Some(targets), .. } => targets.iter().any(|picked| match picked {
            Selection::Instance { instance_id } => instance_id == target,
            Selection::Hero { player } => format!("hero-{}", player.as_str()) == target,
            _ => false,
        }),
        _ => false,
    }
}

fn step(state: &GameState, action: &ActionBody, n: usize) -> GameState {
    let next = reduce(state, &Action::new(action.clone(), AI, format!("audit:{n}")));
    assert!(next.error.is_none(), "{:?}", next.error);
    next.state
}

fn keys(actions: &[ActionBody]) -> Vec<String> {
    actions.iter().map(action_key).collect()
}

#[test]
fn probe() {
    jackioh_cards::register_all();
    // B1 GIGA-ify
    let s = scenario(json!({
        "seed": "audit-sim",
        "p1": { "hand": [GIGA_IFY, TRAP, SEVEN], "library": [VANILLA, VANILLA, VANILLA], "mana": 10 },
        "p2": { "hand": [VANILLA], "library": [VANILLA, VANILLA, VANILLA] },
    }));
    let root = s.state().clone();
    let (trap, seven) = (id_of(&root, TRAP), id_of(&root, SEVEN));
    let all = plays(&root, GIGA_IFY);
    println!("gigaify plays {:?}", keys(&all));
    let on_trap = all.iter().find(|a| aims_at(a, &trap)).unwrap();
    let on_seven = all.iter().find(|a| aims_at(a, &seven)).unwrap();
    let (a, b) = (step(&root, on_trap, 1), step(&root, on_seven, 2));
    println!("B1 sig eq {} eval {} vs {} legal {} vs {}", search_signature(&a, AI) == search_signature(&b, AI),
        evaluate(&a, AI, NextSwing::Enemy, &AI_EVAL), evaluate(&b, AI, NextSwing::Enemy, &AI_EVAL),
        legal_actions(&a, AI).len(), legal_actions(&b, AI).len());
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let lines = beam_search(&root, AI, &counter, AI_BUDGET, Prices::NONE);
    for line in lines.iter().take(8) { println!("  line {:.2} {:?}", line.score, keys(&line.actions)); }
    println!("  any line starting on seven: {}", lines.iter().any(|l| l.actions.first().is_some_and(|a| aims_at(a, &seven))));
    let mut options = AiOptions::with_budget(create_rng("audit", 0), AI_BUDGET);
    let d = decide(&root, AI, &mut options).unwrap();
    println!("  decide {:?} {:?}", d.reason, keys(&d.line));

    // B2 Radiant Silence
    let s = scenario(json!({
        "seed": "audit-sim",
        "p1": { "hand": [{ "def": SILENCE, "radiant": true }], "library": [VANILLA, VANILLA, VANILLA], "mana": 10 },
        "p2": { "hand": [VANILLA], "library": [VANILLA, VANILLA, VANILLA],
            "backrow": [{ "def": TRAP, "lane": 1 }, { "def": WARAXE, "lane": 2, "faceUp": true }] },
    }));
    let root = s.state().clone();
    let trap = root.players[PlayerId::P2].backrow[0].as_ref().unwrap().id.clone();
    let axe = root.players[PlayerId::P2].backrow[1].as_ref().unwrap().id.clone();
    let all = plays(&root, SILENCE);
    println!("silence plays {:?}", keys(&all));
    let on_trap = all.iter().find(|a| aims_at(a, &trap)).unwrap();
    let on_axe = all.iter().find(|a| aims_at(a, &axe)).unwrap();
    let (a, b) = (step(&root, on_trap, 1), step(&root, on_axe, 2));
    println!("B2 sig eq {} eval {} vs {}", search_signature(&a, AI) == search_signature(&b, AI),
        evaluate(&a, AI, NextSwing::Enemy, &AI_EVAL), evaluate(&b, AI, NextSwing::Enemy, &AI_EVAL));
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let lines = beam_search(&root, AI, &counter, AI_BUDGET, Prices::NONE);
    for line in lines.iter().take(8) { println!("  line {:.2} {:?}", line.score, keys(&line.actions)); }
    let mut options = AiOptions::with_budget(create_rng("audit", 0), AI_BUDGET);
    let d = decide(&root, AI, &mut options).unwrap();
    println!("  decide {:?} {:?}", d.reason, keys(&d.line));

    // B3 Second Wind
    let s = scenario(json!({
        "seed": "audit-sim",
        "p1": { "hand": [SECOND_WIND, WILDFIRE], "library": [VANILLA, VANILLA, VANILLA], "mana": 10 },
        "p2": { "hand": [VANILLA], "library": [VANILLA, VANILLA, VANILLA] },
    }));
    let root = s.state().clone();
    let wind = plays(&root, SECOND_WIND);
    println!("wind plays {:?}", keys(&wind));
    let fire_hero = |state: &GameState| plays(state, WILDFIRE).into_iter().find(|a| aims_at(a, "hero-p2"));
    let a1 = step(&root, &wind[0], 1);
    let a2 = step(&a1, &fire_hero(&a1).expect("flashback play"), 2);
    let b1 = step(&root, &fire_hero(&root).unwrap(), 3);
    let wind_b = plays(&b1, SECOND_WIND).into_iter().find(|a| action_key(a) == action_key(&wind[0])).unwrap();
    let b2 = step(&b1, &wind_b, 4);
    println!("B3 sig eq {} lanes eq {} eval {} vs {} legal {:?} vs {:?}", search_signature(&a2, AI) == search_signature(&b2, AI), unit_lanes(&a2) == unit_lanes(&b2),
        evaluate(&a2, AI, NextSwing::Enemy, &AI_EVAL), evaluate(&b2, AI, NextSwing::Enemy, &AI_EVAL),
        keys(&legal_actions(&a2, AI)).len(), keys(&legal_actions(&b2, AI)).len());

    // C Churnwalker
    let s = scenario(json!({
        "seed": "audit-sim",
        "p1": { "hand": [CHURNWALKER], "field": [{ "def": VANILLA, "lane": 1 }, { "def": SEVEN, "lane": 2 }], "library": [VANILLA, VANILLA, VANILLA], "mana": 10 },
        "p2": { "hand": [VANILLA], "library": [VANILLA, VANILLA, VANILLA],
            "field": [{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 2 }] },
    }));
    let root = s.state().clone();
    let all = plays(&root, CHURNWALKER);
    println!("churn plays {} first {:?}", all.len(), keys(&all[..2.min(all.len())]));
    let with_mode = |mode: &str| all.iter().find(|a| matches!(a, ActionBody::Play { modes: Some(m), .. } if m.iter().any(|x| x == mode))).cloned().unwrap();
    let enemy = with_mode("enemy");
    let friendly = all.iter().find(|a| match (a, &enemy) {
        (ActionBody::Play { modes: Some(m), zone, tributes, .. }, ActionBody::Play { zone: z2, tributes: t2, .. }) => m.iter().any(|x| x == "friendly") && zone == z2 && tributes == t2,
        _ => false }).cloned().unwrap();
    let (a, b) = (step(&root, &enemy, 1), step(&root, &friendly, 2));
    println!("C sig eq {} lanes eq {} eval {} vs {}", search_signature(&a, AI) == search_signature(&b, AI), unit_lanes(&a) == unit_lanes(&b),
        evaluate(&a, AI, NextSwing::Enemy, &AI_EVAL), evaluate(&b, AI, NextSwing::Enemy, &AI_EVAL));
    let attacks_a: Vec<ActionBody> = legal_actions(&a, AI).into_iter().filter(|x| matches!(x, ActionBody::Attack { target_id, .. } if target_id != "hero-p2")).collect();
    println!("  attacks {:?}", keys(&attacks_a));
    if let Some(attack) = attacks_a.first() {
        let (a2, b2) = (step(&a, attack, 3), step(&b, attack, 4));
        println!("  after same attack: sig eq {} eval {} vs {}", search_signature(&a2, AI) == search_signature(&b2, AI),
            evaluate(&a2, AI, NextSwing::Enemy, &AI_EVAL), evaluate(&b2, AI, NextSwing::Enemy, &AI_EVAL));
    }

    // D Dormant lane variants
    for def in [VANILLA, SHIP_SMITH, WARAXE] {
        let s = scenario(json!({
            "seed": "audit-sim",
            "p1": { "hand": [def], "library": [VANILLA, VANILLA, VANILLA], "mana": 10 },
            "p2": { "hand": [VANILLA], "library": [VANILLA, VANILLA, VANILLA] },
        }));
        let root = s.state().clone();
        let counter = create_node_counter(AI_BUDGET.nodes, None);
        let lines = beam_search(&root, AI, &counter, AI_BUDGET, Prices::NONE);
        let firsts: Vec<String> = lines.iter().filter_map(|l| l.actions.first().map(action_key)).collect();
        println!("D {def}: root candidates {} lines {:?}", root_candidate_actions(&root, AI, Prices::NONE).len(), firsts);
    }
}
