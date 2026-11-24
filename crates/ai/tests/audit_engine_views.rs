//! Audit: what a player is shown (`view_for`, events) and what a player may send (the validator).
//! Each test asserts the documented behaviour and fails on the bug it proves.

use jackioh_engine::testkit::*;

fn js<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serializable")
}

fn diff_paths(a: &Value, b: &Value, path: &str, out: &mut Vec<(String, String, String)>) {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            let mut keys: Vec<&String> = x.keys().chain(y.keys()).collect();
            keys.sort();
            keys.dedup();
            for key in keys {
                let (l, r) = (x.get(key).unwrap_or(&Value::Null), y.get(key).unwrap_or(&Value::Null));
                diff_paths(l, r, &format!("{path}.{key}"), out);
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (at, (l, r)) in x.iter().zip(y).enumerate() {
                diff_paths(l, r, &format!("{path}[{at}]"), out);
            }
        }
        _ => {
            if a != b {
                let short = |v: &Value| {
                    let s = v.to_string();
                    s.chars().take(160).collect::<String>()
                };
                out.push((path.to_string(), short(a), short(b)));
            }
        }
    }
}

fn strip_indexes(path: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for ch in path.chars() {
        match ch {
            '[' => {
                inside = true;
                out.push_str("[]");
            }
            ']' => inside = false,
            _ if inside => {}
            _ => out.push(ch),
        }
    }
    out
}

fn pool() -> Vec<String> {
    let mut pool: Vec<String> = registered_catalog()
        .iter()
        .filter(|(_, def)| !def.is_token() && !made_by_a_rule(def) && set_ships(def.set))
        .map(|(id, _)| id.clone())
        .collect();
    pool.sort();
    pool
}

fn decks(seed: u32, pool: &[String]) -> (Vec<String>, Vec<String>) {
    let mut rng = Rng::new(&format!("audit-views-decks-{seed}"), 0);
    let shuffled = rng.shuffle(pool);
    let size = DECK_SIZE as usize;
    (shuffled[..size].to_vec(), shuffled[size..2 * size].to_vec())
}

/// The twins of `state` that differ from it only in what `viewer` may not read.
fn twins(state: &GameState, viewer: PlayerId) -> Vec<(&'static str, GameState)> {
    let opp = opponent_of(viewer);
    let mut out = Vec::new();
    if state.players[opp].library.len() > 1 {
        let mut twin = state.clone();
        twin.players[opp].library.reverse();
        out.push(("opp-library-order", twin));
    }
    if state.players[viewer].library.len() > 1 {
        let mut twin = state.clone();
        twin.players[viewer].library.reverse();
        out.push(("own-library-order", twin));
    }
    // The opponent's hand cards trade what they are with library cards: ids and zones stay.
    {
        let mut twin = state.clone();
        let side = &mut twin.players[opp];
        let n = side.hand.len().min(side.library.len());
        if n > 0 && state.result.is_none() {
            for at in 0..n {
                let lib_at = side.library.len() - 1 - at;
                let mut hand_card = side.hand[at].clone();
                let mut lib_card = side.library[lib_at].clone();
                std::mem::swap(&mut hand_card.id, &mut lib_card.id);
                std::mem::swap(&mut hand_card.zone, &mut lib_card.zone);
                side.hand[at] = lib_card;
                side.library[lib_at] = hand_card;
            }
            out.push(("opp-hand-contents", twin));
        }
    }
    if state.players[opp].hand.len() > 1 && state.result.is_none() {
        let mut twin = state.clone();
        twin.players[opp].hand.reverse();
        out.push(("opp-hand-order", twin));
    }
    {
        let mut twin = state.clone();
        let mut changed = false;
        for player in PLAYER_IDS {
            let side = &mut twin.players[player];
            for card in side.hand.iter_mut().chain(side.library.iter_mut()) {
                if card.hidden_nerfs.take().is_some() {
                    changed = true;
                }
            }
        }
        if changed {
            out.push(("hidden-nerfs", twin));
        }
    }
    // R312: a library card its owner was never shown becomes another card.
    {
        let mut twin = state.clone();
        let mut changed = false;
        let catalog = registered_catalog();
        for card in twin.players[viewer].library.iter_mut() {
            if card.known_as.is_some() {
                continue;
            }
            let Some(def) = catalog.get(&card.def_id) else { continue };
            if let Some((id, _)) = catalog.iter().find(|(id, other)| {
                **id != card.def_id && other.type_ == def.type_ && !other.is_token() && set_ships(other.set)
            }) {
                card.def_id = id.clone();
                changed = true;
            }
        }
        if changed {
            out.push(("own-library-unshown", twin));
        }
    }
    // The opponent's face-down traps become other traps of the same type.
    {
        let mut twin = state.clone();
        let mut changed = false;
        let catalog = registered_catalog();
        for lane in 0..twin.players[opp].backrow.len() {
            let Some(card) = state.players[opp].backrow[lane].as_ref() else { continue };
            if backrow_is_public(state, card, viewer) || card.set_as.is_some() {
                continue;
            }
            let Some(def) = catalog.get(&card.def_id) else { continue };
            let other = catalog.iter().find(|(id, other)| {
                **id != card.def_id
                    && other.type_ == def.type_
                    && !other.is_token()
                    && set_ships(other.set)
                    && other.cost == def.cost
            });
            if let Some((id, _)) = other {
                let slot = twin.players[opp].backrow[lane].as_mut().unwrap();
                slot.def_id = id.clone();
                changed = true;
            }
        }
        if changed {
            out.push(("opp-face-down-def", twin));
        }
    }
    out
}

#[test]
fn explore_differential() {
    jackioh_cards::register_all();
    let pool = pool();
    let mut seen: IndexMap<String, (String, usize)> = IndexMap::new();
    let seeds: Vec<u32> = std::env::var("AUDIT_SEEDS")
        .ok()
        .map(|s| {
            let (a, b) = s.split_once("..").unwrap();
            (a.parse().unwrap()..b.parse().unwrap()).collect()
        })
        .unwrap_or_else(|| (0..12).collect());
    for seed in seeds {
        let (d1, d2) = decks(seed, &pool);
        let dealt: Vec<&str> = if seed % 2 == 1 { vec!["p1", "p2"] } else { vec![] };
        let options: CreateGameOptions =
            json_as(json!({ "seed": format!("audit-views-{seed}"), "decks": [d1, d2], "dealt": dealt }));
        let begun = begin_game(&create_game(&options));
        let mut state = begun.state;
        let mut policy = Rng::new(&format!("audit-views-policy-{seed}"), 0);
        let mut steps = 0usize;
        while state.result.is_none() && steps < 600 {
            let player = seat_to_act(&state).unwrap_or(state.active);
            let Some(chosen) = subsystems::choose_action(&state, player, &mut policy) else { break };
            let action = Action::new(chosen, player, format!("a-{steps}"));
            let result = reduce(&state, &action);
            if result.error.is_some() {
                break;
            }
            state = result.state;
            steps += 1;
            if steps % 3 != 0 {
                continue;
            }
            if std::env::var("AUDIT_DYNAMIC").is_ok() && state.result.is_none() {
                let actor = seat_to_act(&state).unwrap_or(state.active);
                let listed = legal_actions(&state, actor);
                let stride = (listed.len() / 6).max(1);
                let sample: Vec<&ActionBody> = listed.iter().step_by(stride).collect();
                for viewer in PLAYER_IDS {
                    for (name, twin) in twins(&state, viewer) {
                        for body in &sample {
                            let action = Action::new((*body).clone(), actor, format!("d-{steps}"));
                            let (r1, r2) = (reduce(&state, &action), reduce(&twin, &action));
                            if r1.error.is_some() || r2.error.is_some() {
                                if r1.error.is_some() != r2.error.is_some() {
                                    let key = format!("DYN {name} {} refusal-differs actor-is-viewer={}", action.action_type().as_str(), actor == viewer);
                                    seen.entry(key).or_insert_with(|| (format!("seed {seed} step {steps} viewer {viewer}: {:?} | {:?} for {}", r1.error, r2.error, js(body)), 0)).1 += 1;
                                }
                                continue;
                            }
                            let fired = |events: &[GameEvent]| events.iter().any(|e| matches!(e, GameEvent::TrapFired { .. }));
                            if name == "opp-face-down-def" && (fired(&r1.events) || fired(&r2.events)) {
                                continue;
                            }
                            if name == "own-library-order" || name == "own-library-unshown" || (name == "opp-hand-contents" && actor != viewer) {
                                continue;
                            }
                            let split = |r: &ReduceResult| {
                                let mut view = js(&view_for(&r.state, viewer));
                                let events = view["events"].as_array().cloned().unwrap_or_default();
                                let fresh = viewed_of_newest(&r.state, viewer, r.events.len()).min(events.len());
                                let new: Vec<Value> = events[events.len() - fresh..].to_vec();
                                view.as_object_mut().unwrap().remove("events");
                                (view, new)
                            };
                            let ((v1, e1), (v2, e2)) = (split(&r1), split(&r2));
                            let types = |events: &[Value]| events.iter().map(|e| e["type"].as_str().unwrap_or("?").to_string()).collect::<Vec<_>>();
                            let (t1, t2) = (types(&e1), types(&e2));
                            let kind = action.action_type().as_str().to_string();
                            if t1 != t2 {
                                let at = t1.iter().zip(&t2).position(|(x, y)| x != y).unwrap_or(t1.len().min(t2.len()));
                                let key = format!("DYN {name} {kind} by-viewer={} SHAPE {:?} vs {:?}", actor == viewer, t1.get(at), t2.get(at));
                                seen.entry(key).or_insert_with(|| (format!("seed {seed} step {steps} viewer {viewer} for {}: {} | {}", js(body), Value::from(e1.get(at.saturating_sub(1)..(at + 2).min(e1.len())).unwrap_or(&[]).to_vec()), Value::from(e2.get(at.saturating_sub(1)..(at + 2).min(e2.len())).unwrap_or(&[]).to_vec())), 0)).1 += 1;
                                continue;
                            }
                            let mut diffs = Vec::new();
                            diff_paths(&Value::from(e1.clone()), &Value::from(e2.clone()), "new", &mut diffs);
                            diff_paths(&v1, &v2, "view", &mut diffs);
                            if let Some((path, a, b)) = diffs.first() {
                                let at: usize = path.strip_prefix("new[").and_then(|rest| rest.split(']').next()).and_then(|n| n.parse().ok()).unwrap_or(usize::MAX);
                                let event_type = e1.get(at).map(|e| e["type"].as_str().unwrap_or("?").to_string()).unwrap_or_default();
                                let key = format!("DYN {name} {kind} by-viewer={} {} {event_type}", actor == viewer, strip_indexes(path));
                                seen.entry(key).or_insert_with(|| (format!("seed {seed} step {steps} viewer {viewer} at {path}: {a} | {b} for {} ({} diffs) {}", js(body), diffs.len(), e1.get(at).cloned().unwrap_or(Value::Null)), 0)).1 += 1;
                            }
                        }
                    }
                }
            }
            for viewer in PLAYER_IDS {
                let base_view = js(&view_for(&state, viewer));
                let base_legal = js(&legal_actions(&state, viewer));
                for (name, twin) in twins(&state, viewer) {
                    let mut diffs = Vec::new();
                    diff_paths(&base_view, &js(&view_for(&twin, viewer)), "view", &mut diffs);
                    diff_paths(&base_legal, &js(&legal_actions(&twin, viewer)), "legal", &mut diffs);
                    for (path, a, b) in diffs {
                        let key = format!("{name} {}", strip_indexes(&path));
                        let entry = seen.entry(key).or_insert_with(|| {
                            (format!("seed {seed} step {steps} viewer {viewer} at {path}: {a} | {b}"), 0)
                        });
                        entry.1 += 1;
                    }
                }
            }
        }
        eprintln!("seed {seed}: {steps} steps, turn {}", state.turn);
    }
    for (key, (example, count)) in &seen {
        println!("DIFF {key} x{count}\n     {example}");
    }
}
