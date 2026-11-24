//! Audit of move generation and simulation (`candidates.rs`, `simulate.rs`). Exploratory, temporary.

use std::panic::{AssertUnwindSafe, catch_unwind};

use jackioh_ai::*;
use jackioh_engine::testkit::*;

const AI: PlayerId = PlayerId::P1;

fn diff(a: &Value, b: &Value, path: &str, out: &mut Vec<String>) {
    if a == b || out.len() > 14 {
        return;
    }
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            let mut keys: Vec<&String> = x.keys().chain(y.keys()).collect();
            keys.sort();
            keys.dedup();
            for key in keys {
                if ["applied", "rngCursor", "nextId", "nextSeq"].contains(&key.as_str()) && path.is_empty() {
                    continue;
                }
                diff(
                    x.get(key).unwrap_or(&Value::Null),
                    y.get(key).unwrap_or(&Value::Null),
                    &format!("{path}/{key}"),
                    out,
                );
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (i, (p, q)) in x.iter().zip(y).enumerate() {
                diff(p, q, &format!("{path}/{i}"), out);
            }
        }
        _ => {
            let short = |v: &Value| {
                let s = v.to_string();
                s.chars().take(60).collect::<String>()
            };
            out.push(format!("{path}: {} != {}", short(a), short(b)));
        }
    }
}

fn board(tested: &str, on_field: bool, radiant: bool, rich: bool, is_unit: bool) -> Option<Scenario> {
    let mut p1_field = vec![
        json!({ "def": "core-008", "lane": 1 }),
        json!({ "def": "core-067", "lane": 2 }),
    ];
    let mut hand = if rich {
        // a Unit, a Trap, a Field Spell, a Spell, a keyword Unit
        vec![json!("core-025"), json!("core-018"), json!("meditative-066"), json!("classic-055"), json!("core-056")]
    } else {
        vec![json!("core-008"), json!("core-067"), json!("core-005")]
    };
    let mut own_backrow: Vec<Value> = Vec::new();
    if on_field && is_unit {
        p1_field.push(json!({ "def": tested, "lane": 4, "radiant": radiant }));
    } else if on_field {
        own_backrow.push(json!({ "def": tested, "lane": 4, "radiant": radiant, "faceUp": true }));
    } else {
        hand.insert(0, json!({ "def": tested, "radiant": radiant }));
    }
    if rich {
        own_backrow.push(json!({ "def": "core-018", "lane": 1 }));
        own_backrow.push(json!({ "def": "meditative-066", "lane": 3 }));
    }
    let p1_backrow = Value::Array(own_backrow);
    let p2_backrow = if rich { json!([{ "def": "core-018", "lane": 2 }, { "def": "meditative-066", "lane": 5 }]) } else { json!([]) };
    let build = || {
        scenario(json!({
            "seed": "audit-sim",
            "p1": {
                "hand": hand,
                "field": p1_field,
                "backrow": p1_backrow,
                "library": ["core-008", "core-005", "core-067", "core-030", "core-053", "core-005"],
                "graveyard": ["core-008", "core-005", "core-067"],
                "mana": 10,
            },
            "p2": {
                "hand": ["core-008", "core-005"],
                "field": [
                    { "def": "core-053", "lane": 1 },
                    { "def": "core-067", "lane": 3 },
                    { "def": "core-030", "lane": 4 },
                ],
                "backrow": p2_backrow,
                "library": ["core-008", "core-005", "core-067", "core-030", "core-053", "core-005"],
                "graveyard": ["core-008", "core-067"],
            },
        }))
    };
    catch_unwind(AssertUnwindSafe(build)).ok()
}

fn names_instance(action: &ActionBody, id: &str) -> bool {
    match action {
        ActionBody::Play { instance_id, .. }
        | ActionBody::Activate { instance_id, .. }
        | ActionBody::ActivatePower { instance_id, .. }
        | ActionBody::Trade { instance_id }
        | ActionBody::SwitchPosition { instance_id } => instance_id == id,
        ActionBody::Attack { attacker_id, .. } => attacker_id == id,
        _ => false,
    }
}

#[test]
fn explore_collisions() {
    jackioh_cards::register_all();
    std::panic::set_hook(Box::new(|_| {}));
    let filter = std::env::var("AUDIT_FILTER").unwrap_or_default();
    let defs: Vec<(String, bool)> = registered_catalog()
        .iter()
        .filter(|(id, _)| filter.is_empty() || id.contains(&filter))
        .map(|(id, def)| (id.clone(), def.type_ == CardType::Unit))
        .collect();
    let mut reports = 0;
    for (def_id, is_unit) in &defs {
        for (on_field, radiant, rich) in [
            (false, false, false), (true, false, false), (false, true, false), (true, true, false),
            (false, false, true), (true, false, true), (false, true, true), (true, true, true),
        ] {
            let Some(s) = board(def_id, on_field, radiant, rich, *is_unit) else {
                continue;
            };
            let root = s.state().clone();
            let tested_id = if on_field {
                let on_units = active_units_of(&root, AI).into_iter().find(|u| u.def_id == *def_id).map(|u| u.id.clone());
                let on_backrow = root.players[AI].backrow.iter().flatten().find(|c| c.def_id == *def_id && c.radiant == radiant).map(|c| c.id.clone());
                match on_units.or(on_backrow) {
                    Some(id) => id,
                    None => continue,
                }
            } else {
                root.players[AI].hand[0].id.clone()
            };
            let counter = create_node_counter(10_000_000, None);
            // depth -> states with their action paths
            let mut frontier: Vec<(GameState, Vec<String>)> = vec![(root.clone(), Vec::new())];
            for depth in 0..4 {
                let mut next: Vec<(GameState, Vec<String>)> = Vec::new();
                for (state, path) in &frontier {
                    let actions: Vec<ActionBody> = legal_actions(state, AI)
                        .into_iter()
                        .filter(|action| {
                            if depth == 0 {
                                names_instance(action, &tested_id)
                            } else {
                                matches!(action, ActionBody::Answer { .. })
                            }
                        })
                        .take(40)
                        .collect();
                    for action in actions {
                        let Some(Ok(child)) = simulate(state, AI, &action, &counter, Prices::NONE) else {
                            continue;
                        };
                        let mut p = path.clone();
                        p.push(action_key(&action));
                        next.push((child, p));
                    }
                }
                if next.is_empty() {
                    break;
                }
                next.truncate(400);
                // group by signature
                let mut groups: IndexMap<(String, String), Vec<usize>> = IndexMap::new();
                for (i, (state, _)) in next.iter().enumerate() {
                    if line_status(state, AI, root.turn) != LineStatus::Open {
                        continue;
                    }
                    groups
                        .entry((search_signature(state, AI), unit_lanes(state)))
                        .or_default()
                        .push(i);
                }
                let mut said = 0;
                for members in groups.values() {
                    let first = &next[members[0]];
                    let a = serde_json::to_value(&first.0).unwrap();
                    for other in &members[1..] {
                        let second = &next[*other];
                        let b = serde_json::to_value(&second.0).unwrap();
                        let mut out = Vec::new();
                        diff(&a, &b, "", &mut out);
                        if out.is_empty() {
                            continue;
                        }
                        let ea = evaluate(&first.0, AI, NextSwing::Enemy, &AI_EVAL);
                        let eb = evaluate(&second.0, AI, NextSwing::Enemy, &AI_EVAL);
                        let la = legal_actions(&first.0, AI).len();
                        let lb = legal_actions(&second.0, AI).len();
                        if said < 2 {
                            said += 1;
                            reports += 1;
                            println!(
                                "COLLISION {def_id} field={on_field} rad={radiant} rich={rich} depth={depth} eval {ea} vs {eb} legal {la} vs {lb}\n  A: {:?}\n  B: {:?}\n  {}",
                                first.1,
                                second.1,
                                out.join("\n  ")
                            );
                        }
                    }
                }
                frontier = next
                    .into_iter()
                    .filter(|(state, _)| {
                        state.result.is_none()
                            && state.pending.as_ref().is_some_and(|pending| pending.player_id == AI)
                    })
                    .collect();
                if frontier.is_empty() {
                    break;
                }
            }
        }
    }
    println!("{reports} reports over {} defs", defs.len());
}

/// C #55 Book of Wildfire: (1) Spell, "Deal 4 damage" to a declared target.
const WILDFIRE: &str = "classic-055";
/// C #57 Echo: (1) Spell, "This has the text of the last Spell either player played".
const ECHO: &str = "classic-057";
/// The Coin: (0) Spell, "Gain 1 mana this turn".
const COIN: &str = "core-t-coin";

fn play_of(state: &GameState, def_id: &str, at_enemy_hero: bool) -> ActionBody {
    let id = state.players[AI]
        .hand
        .iter()
        .find(|card| card.def_id == def_id)
        .expect("in hand")
        .id
        .clone();
    legal_actions(state, AI)
        .into_iter()
        .find(|action| match action {
            ActionBody::Play {
                instance_id, targets, ..
            } if *instance_id == id => {
                !at_enemy_hero
                    || targets.as_ref().is_some_and(|targets| {
                        matches!(targets.first(), Some(Selection::Hero { player }) if *player == PlayerId::P2)
                    })
            }
            _ => false,
        })
        .unwrap_or_else(|| panic!("no play of {def_id}"))
}

fn step(state: &GameState, action: &ActionBody, n: usize) -> GameState {
    let next = reduce(state, &Action::new(action.clone(), AI, format!("audit:{n}")));
    assert!(next.error.is_none(), "{:?}", next.error);
    next.state
}

#[test]
fn echo_probe() {
    jackioh_cards::register_all();
    let s = scenario(json!({
        "seed": "audit-sim-echo",
        "p1": { "hand": [WILDFIRE, ECHO, COIN], "library": ["core-008", "core-008", "core-008"], "mana": 1 },
        "p2": { "hand": ["core-008"], "library": ["core-008", "core-008", "core-008"], "health": 8 },
    }));
    let root = s.state().clone();
    // Coin, Wildfire at the hero, Echo (now Wildfire) at the hero.
    let a1 = step(&root, &play_of(&root, COIN, false), 1);
    let a2 = step(&a1, &play_of(&a1, WILDFIRE, true), 2);
    // Wildfire at the hero, Coin.
    let b1 = step(&root, &play_of(&root, WILDFIRE, true), 3);
    let b2 = step(&b1, &play_of(&b1, COIN, false), 4);
    println!("sig a2 {}", search_signature(&a2, AI));
    println!("sig b2 {}", search_signature(&b2, AI));
    println!("legal a2 {:?}", legal_actions(&a2, AI).iter().map(action_key).collect::<Vec<_>>());
    println!("legal b2 {:?}", legal_actions(&b2, AI).iter().map(action_key).collect::<Vec<_>>());
    let a3 = step(&a2, &play_of(&a2, ECHO, true), 5);
    println!("a3 result {:?}", a3.result);
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let lethal = find_lethal(std::slice::from_ref(&root), AI, &counter, AI_BUDGET.nodes, Prices::NONE);
    println!("lethal {:?} used {}", lethal.map(|l| l.iter().map(action_key).collect::<Vec<_>>()), counter.used());
    let mut options = AiOptions::with_budget(create_rng("audit", 0), AI_BUDGET);
    let decision = decide(&root, AI, &mut options).expect("a decision");
    println!("decision {:?} {} line {:?}", decision.reason, action_key(&decision.action), decision.line.iter().map(action_key).collect::<Vec<_>>());
}

fn find_action(state: &GameState, key: &str) -> Option<ActionBody> {
    legal_actions(state, AI).into_iter().find(|action| action_key(action) == key)
}

#[test]
fn explore_orders() {
    jackioh_cards::register_all();
    std::panic::set_hook(Box::new(|_| {}));
    let filter = std::env::var("AUDIT_FILTER").unwrap_or_default();
    let defs: Vec<(String, bool)> = registered_catalog()
        .iter()
        .filter(|(id, _)| filter.is_empty() || id.contains(&filter))
        .map(|(id, def)| (id.clone(), def.type_ == CardType::Unit))
        .collect();
    let mut kinds: IndexMap<String, Vec<String>> = IndexMap::new();
    for (def_id, is_unit) in &defs {
        for (on_field, radiant) in [(false, false), (false, true), (true, false)] {
            let build = || {
                let mut hand = vec![json!("core-t-coin"), json!("classic-055"), json!("core-008"), json!("core-025")];
                let mut field = vec![json!({ "def": "core-008", "lane": 1 }), json!({ "def": "core-056", "lane": 2 })];
                let mut backrow: Vec<Value> = Vec::new();
                if on_field && *is_unit {
                    field.push(json!({ "def": def_id, "lane": 4, "radiant": radiant }));
                } else if on_field {
                    backrow.push(json!({ "def": def_id, "lane": 4, "radiant": radiant, "faceUp": true }));
                } else {
                    hand.insert(0, json!({ "def": def_id, "radiant": radiant }));
                }
                scenario(json!({
                    "seed": "audit-sim-order",
                    "p1": { "hand": hand, "field": field, "backrow": backrow,
                        "library": ["core-008", "core-005", "core-067", "core-030", "core-053", "core-005"],
                        "graveyard": ["core-008", "core-005"], "mana": 10 },
                    "p2": { "hand": ["core-008", "core-005"],
                        "field": [{ "def": "core-053", "lane": 1 }, { "def": "core-030", "lane": 4 }],
                        "library": ["core-008", "core-005", "core-067", "core-030", "core-053", "core-005"],
                        "graveyard": ["core-008"] },
                }))
            };
            let Ok(s) = catch_unwind(AssertUnwindSafe(build)) else { continue };
            let root = s.state().clone();
            let tested_id = if on_field {
                let u = active_units_of(&root, AI).into_iter().find(|u| u.def_id == *def_id).map(|u| u.id.clone());
                let b = root.players[AI].backrow.iter().flatten().find(|c| c.def_id == *def_id).map(|c| c.id.clone());
                match u.or(b) { Some(id) => id, None => continue }
            } else {
                root.players[AI].hand[0].id.clone()
            };
            let counter = create_node_counter(10_000_000, None);
            let all = legal_actions(&root, AI);
            let tested: Vec<ActionBody> = all.iter().filter(|a| names_instance(a, &tested_id)).take(6).cloned().collect();
            // fillers: one per other source, first variant aimed at the enemy hero when it has one
            let mut fillers: Vec<ActionBody> = Vec::new();
            let mut seen_src: Vec<String> = Vec::new();
            for a in &all {
                if names_instance(a, &tested_id) { continue; }
                let src = match a {
                    ActionBody::Play { instance_id, .. } => format!("p{instance_id}"),
                    ActionBody::Attack { attacker_id, target_id } if target_id == "hero-p2" => format!("a{attacker_id}"),
                    _ => continue,
                };
                if seen_src.contains(&src) { continue; }
                let aimed = match a {
                    ActionBody::Play { targets: Some(t), .. } => matches!(t.first(), Some(Selection::Hero { player }) if *player == PlayerId::P2),
                    _ => true,
                };
                if !aimed { continue; }
                seen_src.push(src);
                fillers.push(a.clone());
            }
            for t in &tested {
                for f in &fillers {
                    let run = |first: &ActionBody, second: &ActionBody| -> Option<GameState> {
                        let a = simulate(&root, AI, first, &counter, Prices::NONE)?.ok()?;
                        if a.pending.is_some() || line_status(&a, AI, root.turn) != LineStatus::Open { return None; }
                        let second = find_action(&a, &action_key(second))?;
                        let b = simulate(&a, AI, &second, &counter, Prices::NONE)?.ok()?;
                        if line_status(&b, AI, root.turn) != LineStatus::Open { return None; }
                        Some(b)
                    };
                    let (Some(x), Some(y)) = (run(t, f), run(f, t)) else { continue };
                    if search_signature(&x, AI) != search_signature(&y, AI) || unit_lanes(&x) != unit_lanes(&y) { continue; }
                    let mut out = Vec::new();
                    diff(&serde_json::to_value(&x).unwrap(), &serde_json::to_value(&y).unwrap(), "", &mut out);
                    let mut paths: Vec<String> = out.iter().map(|line| {
                        let p = line.split(':').next().unwrap_or("").to_string();
                        let mut norm = String::new();
                        for part in p.split('/') { if part.chars().all(|c| c.is_ascii_digit()) && !part.is_empty() { norm.push_str("/#"); } else if !part.is_empty() { norm.push('/'); norm.push_str(part); } }
                        norm
                    }).filter(|p| !p.contains("turnLog/playedIds") && !p.contains("turnLog/costsPaid") && !p.contains("/graveyard/") && !p.contains("/fieldExits") && !p.starts_with("/counters") && !p.contains("/exile/")).collect();
                    paths.sort(); paths.dedup();
                    if paths.is_empty() { continue; }
                    let ex = evaluate(&x, AI, NextSwing::Enemy, &AI_EVAL);
                    let ey = evaluate(&y, AI, NextSwing::Enemy, &AI_EVAL);
                    let lx: Vec<String> = legal_actions(&x, AI).iter().map(action_key).collect();
                    let ly: Vec<String> = legal_actions(&y, AI).iter().map(action_key).collect();
                    let tag = format!("{}{}", if ex != ey { "EVAL " } else { "" }, if lx != ly { "LEGAL " } else { "" });
                    kinds.entry(format!("{tag}{}", paths.join(" "))).or_default().push(format!("{def_id}{}{} + {}", if radiant {"*"} else {""}, if on_field {"@field"} else {""}, action_key(f)));
                }
            }
        }
    }
    for (kind, cards) in &kinds {
        let mut names: Vec<&String> = cards.iter().collect();
        names.dedup();
        println!("ORDER {} x{}\n    {:?}", kind, cards.len(), names.iter().take(6).collect::<Vec<_>>());
    }
}
