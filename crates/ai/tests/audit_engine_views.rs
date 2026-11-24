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

#[test]
fn debug_case_b() {
    jackioh_cards::register_all();
    let pool = pool();
    let seed = 8u32;
    let (d1, d2) = decks(seed, &pool);
    let options: CreateGameOptions =
        json_as(json!({ "seed": format!("audit-views-{seed}"), "decks": [d1, d2], "dealt": Vec::<&str>::new() }));
    let begun = begin_game(&create_game(&options));
    let mut state = begun.state;
    let mut policy = Rng::new(&format!("audit-views-policy-{seed}"), 0);
    for step in 0..45 {
        let player = seat_to_act(&state).unwrap_or(state.active);
        let chosen = subsystems::choose_action(&state, player, &mut policy).unwrap();
        state = reduce(&state, &Action::new(chosen, player, format!("a-{step}"))).state;
    }
    let viewer = PlayerId::P1;
    let opp = PlayerId::P2;
    let mut twin = state.clone();
    twin.players[opp].library.reverse();
    let actor = seat_to_act(&state).unwrap_or(state.active);
    println!("actor {actor} active {} turn {}", state.active, state.turn);
    let describe = |s: &GameState| {
        let side = &s.players[opp];
        println!("  hand: {:?}", side.hand.iter().map(|c| (&c.id, &c.def_id, c.cost_mod)).collect::<Vec<_>>());
        println!("  lib top/bottom: {:?} .. {:?}", side.library.first().map(|c| &c.def_id), side.library.last().map(|c| &c.def_id));
        for card in side.backrow.iter().flatten() {
            println!("  backrow: {} {} cost_mod {} override {:?} face_up {:?} memory {:?} eff {}", card.id, card.def_id, card.cost_mod, card.cost_override, card.face_up, card.memory, effective_cost(s, card, Default::default()));
        }
        println!("  mods: {}", js(&side.mods));
    };
    describe(&state);
    let action = Action::new(ActionBody::EndTurn, actor, "x".to_string());
    for (name, s) in [("base", &state), ("twin", &twin)] {
        let r = reduce(s, &action);
        println!("{name}: error {:?}", r.error);
        for e in &r.events {
            println!("   {}", js(e).to_string().chars().take(220).collect::<String>());
        }
        describe(&r.state);
    }
}

fn canon(state: &GameState) -> String {
    let mut value = js(state);
    value.as_object_mut().unwrap().remove("applied");
    value.to_string()
}

fn try_reduce(state: &GameState, body: &ActionBody, player: PlayerId) -> Result<ReduceResult, String> {
    let action = Action::new(body.clone(), player, "forged".to_string());
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| reduce(state, &action))).map_err(|panic| {
        panic
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_else(|| "panic".to_string())
    })
}

#[test]
fn explore_forged() {
    jackioh_cards::register_all();
    std::panic::set_hook(Box::new(|_| {}));
    let pool = pool();
    let mut seen: IndexMap<String, (String, usize)> = IndexMap::new();
    let seeds: Vec<u32> = std::env::var("AUDIT_SEEDS")
        .ok()
        .map(|s| {
            let (a, b) = s.split_once("..").unwrap();
            (a.parse().unwrap()..b.parse().unwrap()).collect()
        })
        .unwrap_or_else(|| (0..6).collect());
    let mut probes = 0usize;
    for seed in seeds {
        let (d1, d2) = decks(seed, &pool);
        let options: CreateGameOptions =
            json_as(json!({ "seed": format!("audit-views-{seed}"), "decks": [d1, d2] }));
        let begun = begin_game(&create_game(&options));
        let mut state = begun.state;
        let mut policy = Rng::new(&format!("audit-views-policy-{seed}"), 0);
        let mut steps = 0usize;
        while state.result.is_none() && steps < 600 {
            let player = seat_to_act(&state).unwrap_or(state.active);
            let Some(chosen) = subsystems::choose_action(&state, player, &mut policy) else { break };
            let result = reduce(&state, &Action::new(chosen, player, format!("a-{steps}")));
            if result.error.is_some() {
                break;
            }
            state = result.state;
            steps += 1;
            if steps % 4 != 0 || state.result.is_some() {
                continue;
            }
            let me = seat_to_act(&state).unwrap_or(state.active);
            let opp = opponent_of(me);
            let legal = legal_actions(&state, me);
            let legal_json: Vec<Value> = legal.iter().map(|b| js(b)).collect();
            let mut forged: Vec<(String, Value)> = Vec::new();
            // A. ids from every zone, in every action that names a card.
            let mut ids: Vec<(String, String)> = Vec::new();
            let mut take = |zone: &str, cards: Vec<&CardInstance>| {
                for card in cards.into_iter().take(2) {
                    ids.push((zone.to_string(), card.id.clone()));
                }
            };
            take("own-library", state.players[me].library.iter().collect());
            take("own-exile", state.players[me].exile.iter().collect());
            take("own-graveyard", state.players[me].graveyard.iter().collect());
            take("own-units", state.players[me].units.iter().flatten().flatten().collect());
            take("own-backrow", state.players[me].backrow.iter().flatten().collect());
            take("opp-hand", state.players[opp].hand.iter().collect());
            take("opp-library", state.players[opp].library.iter().collect());
            take("opp-graveyard", state.players[opp].graveyard.iter().collect());
            take("opp-units", state.players[opp].units.iter().flatten().flatten().collect());
            take("opp-backrow", state.players[opp].backrow.iter().flatten().collect());
            for bogus in ["c999999", "", "hidden", "hero-p1", "hero-p2", "random"] {
                ids.push(("bogus".to_string(), bogus.to_string()));
            }
            for (zone, id) in &ids {
                forged.push((format!("play {zone}"), json!({"type":"play","instanceId":id})));
                forged.push((format!("play-zone {zone}"), json!({"type":"play","instanceId":id,"zone":{"row":"units","lane":1}})));
                forged.push((format!("play-backrow {zone}"), json!({"type":"play","instanceId":id,"zone":{"row":"backrow","lane":1}})));
                forged.push((format!("play-facedown {zone}"), json!({"type":"play","instanceId":id,"faceDown":"cloaked"})));
                forged.push((format!("trade {zone}"), json!({"type":"trade","instanceId":id})));
                forged.push((format!("switch {zone}"), json!({"type":"switchPosition","instanceId":id})));
                forged.push((format!("activate {zone}"), json!({"type":"activate","instanceId":id})));
                forged.push((format!("activatePower {zone}"), json!({"type":"activatePower","instanceId":id})));
                forged.push((format!("attack-by {zone}"), json!({"type":"attack","attackerId":id,"targetId":format!("hero-{opp}")})));
                if let Some(mine) = state.players[me].units.iter().flatten().flatten().next() {
                    forged.push((format!("attack-on {zone}"), json!({"type":"attack","attackerId":mine.id,"targetId":id})));
                }
                forged.push((format!("mulligan {zone}"), json!({"type":"mulligan","keep":[id]})));
            }
            for offer in [-1i64, i32::MIN as i64, i32::MAX as i64, 6, 64] {
                forged.push(("marketBuy".to_string(), json!({"type":"marketBuy","offer":offer})));
            }
            forged.push(("answerDraw".to_string(), json!({"type":"answerDraw","accept":true})));
            forged.push(("sandbox".to_string(), json!({"type":"sandbox","op":{"op":"draw","player":me,"count":1}})));
            // B. listed plays with one field forged.
            let plays: Vec<&Value> = legal_json.iter().filter(|j| j["type"] == "play").collect();
            let stride = (plays.len() / 6).max(1);
            let own_units: Vec<String> = state.players[me].units.iter().flatten().flatten().map(|c| c.id.clone()).collect();
            let any_ids: Vec<String> = ids.iter().map(|(_, id)| id.clone()).collect();
            for play in plays.iter().step_by(stride) {
                let with = |key: &str, value: Value| {
                    let mut body = (*play).clone();
                    body[key] = value;
                    body
                };
                for lane in [-1i64, 0, 7, 8, 1000, i32::MAX as i64, i32::MIN as i64] {
                    for row in ["units", "backrow"] {
                        forged.push((format!("lane {row}"), with("zone", json!({"row":row,"lane":lane}))));
                    }
                }
                for x in [i32::MAX as i64, 1 << 30, 1000, 3, 0, -1, i32::MIN as i64] {
                    forged.push(("x".to_string(), with("x", json!(x))));
                }
                forged.push(("embiggen".to_string(), with("embiggen", json!(true))));
                forged.push(("equip".to_string(), with("equip", json!(true))));
                for reveal in ["cloaked", "endOfThisTurn", "startOfNextTurn", "endOfNextTurn"] {
                    forged.push(("faceDown".to_string(), with("faceDown", json!(reveal))));
                }
                if let Some(first) = own_units.first() {
                    forged.push(("tributes-one".to_string(), with("tributes", json!([first]))));
                    forged.push(("tributes-dup".to_string(), with("tributes", json!([first, first]))));
                    forged.push(("tributes-all".to_string(), with("tributes", json!(own_units))));
                }
                if let Some(listed) = play["tributes"].as_array().filter(|t| t.len() >= 2) {
                    forged.push(("tributes-listed-dup".to_string(), with("tributes", json!(vec![listed[0].clone(); listed.len()]))));
                }
                if let Some(listed) = play["targets"].as_array().filter(|t| !t.is_empty()) {
                    forged.push(("targets-dup".to_string(), with("targets", json!(vec![listed[0].clone(); listed.len() + 1]))));
                    if listed.len() >= 2 {
                        forged.push(("targets-same".to_string(), with("targets", json!(vec![listed[0].clone(); listed.len()]))));
                    }
                    forged.push(("targets-none".to_string(), with("targets", json!([]))));
                    for id in &any_ids {
                        let mut targets = listed.clone();
                        targets[0] = json!({"pick":"instance","instanceId":id});
                        forged.push(("targets-any".to_string(), with("targets", json!(targets))));
                    }
                    for hero in ["p1", "p2"] {
                        let mut targets = listed.clone();
                        targets[0] = json!({"pick":"hero","player":hero});
                        forged.push(("targets-hero".to_string(), with("targets", json!(targets))));
                    }
                } else {
                    for id in any_ids.iter().take(6) {
                        forged.push(("targets-added".to_string(), with("targets", json!([{"pick":"instance","instanceId":id}]))));
                    }
                }
                if let Some(listed) = play["modes"].as_array().filter(|m| !m.is_empty()) {
                    forged.push(("modes-dup".to_string(), with("modes", json!(vec![listed[0].clone(); listed.len() + 1]))));
                    forged.push(("modes-none".to_string(), with("modes", json!([]))));
                } else {
                    forged.push(("modes-added".to_string(), with("modes", json!(["draw"]))));
                }
                for id in any_ids.iter().take(8) {
                    for tokens in [1i64, -1, 0, i32::MAX as i64] {
                        forged.push(("plague".to_string(), with("plague", json!({"from":id,"tokens":tokens}))));
                    }
                }
            }
            // C. forged answers to the open prompt.
            if let Some(pending) = state.pending.as_ref().filter(|p| p.player_id == me) {
                let answer = |selection: Value| json!({"type":"answer","choiceId":pending.id,"selection":selection});
                for id in &any_ids {
                    forged.push(("answer-any-id".to_string(), answer(json!([{"pick":"instance","instanceId":id}]))));
                }
                forged.push(("answer-none".to_string(), answer(json!([{"pick":"none"}]))));
                forged.push(("answer-empty".to_string(), answer(json!([]))));
                for lane in [-1i64, 0, 1, 8, i32::MAX as i64, i32::MIN as i64] {
                    for row in ["units", "backrow"] {
                        for who in ["p1", "p2"] {
                            forged.push(("answer-zone".to_string(), answer(json!([{"pick":"zone","player":who,"row":row,"lane":lane}]))));
                        }
                    }
                }
                for who in ["p1", "p2"] {
                    forged.push(("answer-hero".to_string(), answer(json!([{"pick":"hero","player":who}]))));
                }
                forged.push(("answer-mode".to_string(), answer(json!([{"pick":"mode","option":"draw"}]))));
                let all: Vec<Value> = pending.options.iter().map(|o| js(&o.selection)).collect();
                forged.push(("answer-all".to_string(), answer(json!(all))));
                if all.len() >= 2 {
                    forged.push(("answer-two".to_string(), answer(json!([all[0], all[1]]))));
                    forged.push(("answer-two-rev".to_string(), answer(json!([all[1], all[0]]))));
                }
            }
            // The outcomes the listing allows, per card.
            let mut listed_outcomes: IndexMap<String, IndexSet<String>> = IndexMap::new();
            for (kind, body) in forged {
                let Ok(parsed) = serde_json::from_value::<ActionBody>(body.clone()) else { continue };
                if legal.contains(&parsed) {
                    continue;
                }
                probes += 1;
                match try_reduce(&state, &parsed, me) {
                    Err(panic) => {
                        let key = format!("PANIC {kind}: {}", panic.chars().take(120).collect::<String>());
                        seen.entry(key).or_insert_with(|| (format!("seed {seed} step {steps} {me}: {body}"), 0)).1 += 1;
                    }
                    Ok(result) if result.error.is_none() => {
                        let card = body["instanceId"].as_str().or(body["attackerId"].as_str()).unwrap_or("").to_string();
                        let outcomes = listed_outcomes.entry(format!("{}:{card}", body["type"])).or_insert_with(|| {
                            legal
                                .iter()
                                .zip(&legal_json)
                                .filter(|(_, j)| (j["type"] == body["type"] || (body["type"] == "activatePower" && j["type"] == "activate")) && (j["instanceId"].as_str().or(j["attackerId"].as_str()).unwrap_or("") == card))
                                .filter_map(|(b, _)| try_reduce(&state, b, me).ok())
                                .map(|r| canon(&r.state))
                                .collect()
                        });
                        if !outcomes.contains(&canon(&result.state)) {
                            let def = find_instance(&state, &card).map(|c| c.def_id.clone()).unwrap_or_default();
                            let key = format!("ACCEPTED {kind} ({} listed outcomes)", outcomes.len());
                            seen.entry(key).or_insert_with(|| (format!("seed {seed} step {steps} {me} def {def}: {body}"), 0)).1 += 1;
                        }
                    }
                    Ok(_) => {}
                }
            }
        }
        eprintln!("seed {seed}: {steps} steps, turn {}", state.turn);
    }
    println!("{probes} probes");
    for (key, (example, count)) in &seen {
        println!("FOUND {key} x{count}\n     {example}");
    }
}

/// Every card played once from a fixed position; the opponent's hidden order (library, hand) reversed
/// must not change what the other seat is shown unless the play makes a card public.
#[test]
fn explore_card_sweep() {
    jackioh_cards::register_all();
    std::panic::set_hook(Box::new(|_| {}));
    let pool = pool();
    let pick = |offset: usize, n: usize| -> Vec<String> {
        let stride = pool.len() / n;
        (0..n).map(|k| pool[(offset + k * stride) % pool.len()].clone()).collect()
    };
    let (lib1, lib2) = (pick(3, 12), pick(41, 12));
    let (hand1, hand2) = (pick(17, 3), pick(29, 4));
    let mut seen: IndexMap<String, (String, usize)> = IndexMap::new();
    let only = std::env::var("AUDIT_CARD").ok();
    for def in &pool {
        if only.as_ref().is_some_and(|o| o != def) {
            continue;
        }
        let built = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut hand = vec![def.clone()];
            hand.extend(hand1.clone());
            scenario(json!({
                "seed": format!("sweep-{def}"),
                "turn": 9,
                "p1": { "hand": hand, "mana": 10, "library": lib1, "field": ["core-008", "core-019"], "graveyard": ["core-025", "core-005"] },
                "p2": { "hand": hand2, "library": lib2, "field": ["core-025", "core-011"], "graveyard": ["core-008", "core-020"],
                        "backrow": [{ "def": "core-096", "faceUp": false }] },
            }))
            .state()
            .clone()
        }));
        let Ok(base) = built else { continue };
        let me = PlayerId::P1;
        let card_id = base.players[me].hand[0].id.clone();
        let plays: Vec<ActionBody> = legal_actions(&base, me)
            .into_iter()
            .filter(|b| matches!(b, ActionBody::Play { instance_id, .. } if *instance_id == card_id))
            .collect();
        let stride = (plays.len() / 3).max(1);
        for play in plays.iter().step_by(stride) {
            for (name, viewer) in [("opp-library", PlayerId::P2), ("opp-hand", PlayerId::P2), ("opp-library", PlayerId::P1), ("opp-hand", PlayerId::P1), ("own-library", PlayerId::P1)] {
                let hidden_side = if name == "own-library" { viewer } else { opponent_of(viewer) };
                let mut twin = base.clone();
                if name == "opp-hand" {
                    twin.players[hidden_side].hand.reverse();
                } else {
                    twin.players[hidden_side].library.reverse();
                }
                let run = |start: &GameState| -> Result<(GameState, usize, Vec<String>), String> {
                    let mut r = try_reduce(start, play, me)?;
                    if let Some(e) = r.error.take() {
                        return Err(format!("refused: {e}"));
                    }
                    let mut fresh = r.events.len();
                    let mut state = r.state;
                    let mut answers = Vec::new();
                    for step in 0..6 {
                        let Some(holder) = state.pending.as_ref().map(|p| p.player_id) else { break };
                        if state.result.is_some() {
                            break;
                        }
                        let legal = legal_actions(&state, holder);
                        let Some(first) = legal.iter().find(|b| matches!(b, ActionBody::Answer { .. })) else { break };
                        answers.push(js(&legal).to_string());
                        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            reduce(&state, &Action::new(first.clone(), holder, format!("ans-{step}")))
                        }))
                        .map_err(|_| "panic in answer".to_string())?;
                        fresh += r.events.len();
                        state = r.state;
                    }
                    Ok((state, fresh, answers))
                };
                let (a, b) = (run(&base), run(&twin));
                let (Ok((s1, n1, ans1)), Ok((s2, n2, ans2))) = (&a, &b) else {
                    if let (Err(e), _) | (_, Err(e)) = (&a, &b)
                        && !e.starts_with("refused")
                    {
                        seen.entry(format!("PANIC {def}: {}", e.chars().take(100).collect::<String>())).or_insert_with(|| (js(play).to_string(), 0)).1 += 1;
                    }
                    continue;
                };
                if ans1 != ans2 {
                    continue;
                }
                let split = |state: &GameState, fresh: usize| {
                    let mut view = js(&view_for(state, viewer));
                    let events = view["events"].as_array().cloned().unwrap_or_default();
                    let shown = viewed_of_newest(state, viewer, fresh).min(events.len());
                    let new: Vec<Value> = events[events.len() - shown..].to_vec();
                    view.as_object_mut().unwrap().remove("events");
                    (view, new)
                };
                let ((v1, e1), (v2, e2)) = (split(s1, *n1), split(s2, *n2));
                let board_same = v1 == v2;
                let types = |events: &[Value]| events.iter().map(|e| e["type"].as_str().unwrap_or("?").to_string()).collect::<Vec<_>>();
                if name == "own-library" && types(&e1).iter().any(|t| t == "drawn" || t == "addedToHand" || t == "promptOpened") {
                    continue;
                }
                if e1 == e2 && board_same {
                    continue;
                }
                let mut diffs = Vec::new();
                if types(&e1) != types(&e2) {
                    diffs.push(("SHAPE".to_string(), types(&e1).join(","), types(&e2).join(",")));
                } else {
                    diffs_into(&e1, &e2, &mut diffs);
                }
                diff_paths(&v1, &v2, "view", &mut diffs);
                let (path, x, y) = diffs.first().cloned().unwrap_or_default();
                let key = format!("{name} viewer={viewer} board_same={board_same} {def} {}", strip_indexes(&path));
                seen.entry(key).or_insert_with(|| (format!("{path}: {x} | {y} for {}", js(play)), 0)).1 += 1;
            }
        }
    }
    for (key, (example, count)) in &seen {
        println!("SWEEP {key} x{count}\n     {example}");
    }
}

fn diffs_into(a: &[Value], b: &[Value], out: &mut Vec<(String, String, String)>) {
    diff_paths(&Value::from(a.to_vec()), &Value::from(b.to_vec()), "new", out);
}

#[test]
fn debug_card() {
    jackioh_cards::register_all();
    let pool = pool();
    let pick = |offset: usize, n: usize| -> Vec<String> {
        let stride = pool.len() / n;
        (0..n).map(|k| pool[(offset + k * stride) % pool.len()].clone()).collect()
    };
    let (lib1, lib2) = (pick(3, 12), pick(41, 12));
    let (hand1, hand2) = (pick(17, 3), pick(29, 4));
    let def = std::env::var("AUDIT_CARD").unwrap();
    let which = std::env::var("AUDIT_TWIN").unwrap_or("opp-hand".to_string());
    let viewer = if std::env::var("AUDIT_VIEWER").ok().as_deref() == Some("p1") { PlayerId::P1 } else { PlayerId::P2 };
    let mut hand = vec![def.clone()];
    hand.extend(hand1.clone());
    let base = scenario(json!({
        "seed": format!("sweep-{def}"),
        "turn": 9,
        "p1": { "hand": hand, "mana": 10, "library": lib1, "field": ["core-008", "core-019"], "graveyard": ["core-025", "core-005"] },
        "p2": { "hand": hand2, "library": lib2, "field": ["core-025", "core-011"], "graveyard": ["core-008", "core-020"],
                "backrow": [{ "def": "core-096", "faceUp": false }] },
    }))
    .state()
    .clone();
    let me = PlayerId::P1;
    println!("p1 hand {:?}", base.players[me].hand.iter().map(|c| (&c.id, &c.def_id)).collect::<Vec<_>>());
    println!("p1 lib {:?}", base.players[me].library.iter().map(|c| (&c.id, &c.def_id)).collect::<Vec<_>>());
    let card_id = base.players[me].hand[0].id.clone();
    let play = legal_actions(&base, me)
        .into_iter()
        .find(|b| matches!(b, ActionBody::Play { instance_id, .. } if *instance_id == card_id))
        .unwrap();
    let mut twin = base.clone();
    let hidden_side = if which == "own-library" { viewer } else { opponent_of(viewer) };
    if which == "opp-hand" {
        twin.players[hidden_side].hand.reverse();
    } else {
        twin.players[hidden_side].library.reverse();
    }
    for (name, s) in [("base", &base), ("twin", &twin)] {
        let r = reduce(s, &Action::new(play.clone(), me, "x".to_string()));
        println!("== {name} raw");
        for e in &r.events {
            println!("   {}", js(e).to_string().chars().take(260).collect::<String>());
        }
        println!("== {name} as {viewer} sees");
        let view = view_for(&r.state, viewer);
        let shown = viewed_of_newest(&r.state, viewer, r.events.len());
        for e in &view.events[view.events.len() - shown..] {
            println!("   {}", js(e).to_string().chars().take(260).collect::<String>());
        }
    }
}

/// R2739000: the count the other seat sees on a picker must not move with the chooser's hidden zones.
#[test]
fn explore_option_counts() {
    jackioh_cards::register_all();
    std::panic::set_hook(Box::new(|_| {}));
    let pool = pool();
    let pick = |offset: usize, n: usize| -> Vec<String> {
        let stride = pool.len() / n;
        (0..n).map(|k| pool[(offset + k * stride) % pool.len()].clone()).collect()
    };
    let (lib1, lib2) = (pick(3, 12), pick(41, 12));
    let (hand1, hand2) = (pick(17, 3), pick(29, 4));
    let mut seen: IndexMap<String, (String, usize)> = IndexMap::new();
    let build = |def: &str, hand_rest: &[String], library: &[String], opp_hand: &[String], opp_library: &[String]| {
        let mut hand = vec![def.to_string()];
        hand.extend(hand_rest.iter().cloned());
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            scenario(json!({
                "seed": format!("sweep-{def}"),
                "turn": 9,
                "p1": { "hand": hand, "mana": 10, "library": library, "field": ["core-008", "core-019"], "graveyard": ["core-025", "core-005"] },
                "p2": { "hand": opp_hand, "library": opp_library, "field": ["core-025", "core-011"], "graveyard": ["core-008", "core-020"],
                        "backrow": [{ "def": "core-096", "faceUp": false }] },
            }))
            .state()
            .clone()
        }))
        .ok()
    };
    let vanillas: Vec<String> = vec!["core-008".to_string(); 12];
    let spells: Vec<String> = vec!["core-005".to_string(); 12];
    for def in &pool {
        let Some(base) = build(def, &hand1, &lib1, &hand2, &lib2) else { continue };
        let twins: Vec<(&str, Option<GameState>)> = vec![
            ("own-library-1", build(def, &hand1, &lib1[..1], &hand2, &lib2)),
            ("own-library-vanillas", build(def, &hand1, &vanillas, &hand2, &lib2)),
            ("own-library-spells", build(def, &hand1, &spells, &hand2, &lib2)),
            ("own-library-other", build(def, &hand1, &pick(100, 12), &hand2, &lib2)),
            ("own-hand-alone", build(def, &[], &lib1, &hand2, &lib2)),
            ("own-hand-vanillas", build(def, &vanillas[..3], &lib1, &hand2, &lib2)),
            ("own-hand-spells", build(def, &spells[..3], &lib1, &hand2, &lib2)),
        ];
        let me = PlayerId::P1;
        let plays: Vec<ActionBody> = legal_actions(&base, me)
            .into_iter()
            .filter(|b| matches!(b, ActionBody::Play { instance_id, .. } if instance_id == "c1"))
            .collect();
        let stride = (plays.len() / 4).max(1);
        for play in plays.iter().step_by(stride) {
            // The pickers p1 holds along the way, as p2 sees each.
            let walk = |start: &GameState| -> Option<Vec<Value>> {
                let r = try_reduce(start, play, me).ok()?;
                if r.error.is_some() {
                    return None;
                }
                let mut state = r.state;
                let mut out = Vec::new();
                for step in 0..4 {
                    let Some(holder) = state.pending.as_ref().map(|p| p.player_id) else { break };
                    if holder != me || state.result.is_some() {
                        break;
                    }
                    let kind = state.pending.as_ref().map(|p| p.kind.as_str().to_string()).unwrap_or_default();
                    out.push(json!({ "kind": kind, "seen": js(&view_for(&state, PlayerId::P2))["pending"] }));
                    let legal = legal_actions(&state, holder);
                    let first = legal.iter().find(|b| matches!(b, ActionBody::Answer { .. }))?;
                    let r = try_reduce(&state, first, holder).ok()?;
                    if r.error.is_some() {
                        break;
                    }
                    let _ = step;
                    state = r.state;
                }
                Some(out)
            };
            let Some(base_seen) = walk(&base) else { continue };
            for seen_prompt in &base_seen {
                let counted = !seen_prompt["seen"]["optionCount"].is_null();
                let key = format!("STAT prompts kind={} counted={counted}", seen_prompt["kind"]);
                seen.entry(key).or_insert_with(|| (def.clone(), 0)).1 += 1;
            }
            for (name, twin) in &twins {
                let Some(twin) = twin else { continue };
                let Some(twin_seen) = walk(twin) else { continue };
                for (at, (a, b)) in base_seen.iter().zip(&twin_seen).enumerate() {
                    if a["kind"] == b["kind"] && a["seen"] != b["seen"] {
                        let key = format!("COUNT {def} prompt#{at} {} {name}", a["kind"]);
                        seen.entry(key).or_insert_with(|| (format!("{} | {} for {}", a["seen"], b["seen"], js(play)), 0)).1 += 1;
                    }
                }
            }
        }
    }
    for (key, (example, count)) in &seen {
        println!("OPT {key} x{count}\n     {example}");
    }
}

#[test]
fn debug_traps() {
    jackioh_cards::register_all();
    for (id, def) in registered_catalog() {
        if (def.type_ == CardType::Trap || def.type_ == CardType::FieldTrap) && set_ships(def.set) {
            let kw = js(&def.base.keywords).to_string();
            let rk = js(&def.radiant.keywords).to_string();
            if kw != "[]" || rk != "[]" {
                println!("{id} cost {:?} {:?} {} base {kw} radiant {rk}", def.cost, def.type_, def.name);
            }
        }
    }
}
