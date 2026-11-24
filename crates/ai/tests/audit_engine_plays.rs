//! Audit of how a card is played (`play_choices.rs`, `play_steps.rs` and their neighbours). Temporary.

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

use jackioh_engine::testkit::*;

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

fn board(name: &str, tested: &str, radiant: bool) -> Option<Scenario> {
    board_seeded(name, tested, radiant, "audit-plays")
}

fn board_seeded(name: &str, tested: &str, radiant: bool, seed: &str) -> Option<Scenario> {
    let t = json!({ "def": tested, "radiant": radiant });
    let lib = json!(["core-008", "core-005", "core-067", "core-030", "core-053", "core-005"]);
    let mixed_p2 = json!({
        "hand": ["core-008", "core-005"],
        "field": [
            { "def": "core-019", "lane": 1 },
            { "def": "classicplus-051", "lane": 2 },
            { "def": "stinkyscape-074", "lane": 3 },
            { "def": "core-053", "lane": 4, "slumber": { "turns": 2 } },
            { "def": "classic-089", "lane": 5 },
        ],
        "backrow": [{ "def": "core-018", "lane": 2 }, { "def": "meditative-066", "lane": 5, "faceUp": true }],
        "library": lib,
        "graveyard": ["core-008", "core-067", "core-005"],
    });
    let mixed_p1 = |mana: i32| {
        json!({
            "hand": [t, "core-008", "core-005", "classic-055"],
            "field": [
                { "def": "core-008", "lane": 1 },
                { "def": "core-t-sheep", "lane": 2 },
                { "def": "core-019", "lane": 3, "damage": 2 },
            ],
            "backrow": [
                { "def": "core-018", "lane": 1 },
                { "def": "meditative-066", "lane": 2, "faceUp": true },
                { "def": "stinkyscapeplus-035", "lane": 4, "row": "backrow" },
            ],
            "library": lib,
            "graveyard": ["core-008", "core-005", "core-067", "core-026"],
            "mana": mana,
        })
    };
    let opts = match name {
        "mixed" => json!({ "seed": "audit-plays", "p1": mixed_p1(10), "p2": mixed_p2 }),
        "poor" => json!({ "seed": "audit-plays", "p1": mixed_p1(1), "p2": mixed_p2 }),
        "empty" => json!({ "seed": "audit-plays", "p1": { "hand": [t], "mana": 10 }, "p2": {} }),
        "full" => json!({
            "seed": "audit-plays",
            "p1": {
                "hand": [t, "core-008"],
                "field": ["core-008", "core-003", "core-092", "core-t-sheep", "core-030"],
                "backrow": [
                    "core-018",
                    { "def": "meditative-066", "faceUp": true },
                    { "def": "classicplus-033", "faceUp": true },
                    { "def": "stinkyscape-016", "faceUp": true },
                    { "def": "core-064", "faceUp": true },
                ],
                "library": lib,
                "graveyard": ["core-008", "core-005"],
                "mana": 10,
            },
            "p2": {
                "hand": ["core-008"],
                "field": ["core-008", "core-019", "classicplus-051", "stinkyscape-116", "core-053"],
                "backrow": [
                    "core-018",
                    { "def": "meditative-066", "faceUp": true },
                    "core-018",
                    { "def": "core-064", "faceUp": true },
                    "core-018",
                ],
                "library": lib,
                "graveyard": ["core-008"],
            },
        }),
        "mayor" => {
            let mut p2 = mixed_p2.clone();
            p2["field"] = json!([
                { "def": "core-019", "lane": 1 },
                { "def": "classicplus-051", "lane": 2 },
                { "def": "meditative-086", "lane": 3 },
                { "def": "classic-089", "lane": 5 },
            ]);
            json!({ "seed": "audit-plays", "p1": mixed_p1(10), "p2": p2 })
        }
        "special" => json!({
            "seed": "audit-plays",
            "p1": {
                "hand": [t, "core-008", "core-056", "classic-033"],
                "field": [
                    { "def": "core-008", "lane": 1 },
                    { "def": "core-092", "lane": 1, "stack": true },
                    { "def": "core-003", "lane": 3 },
                ],
                "backrow": [
                    { "def": "classicplus-033", "lane": 1, "faceUp": true },
                    { "def": "stinkyscape-016", "lane": 2, "faceUp": true },
                    { "def": "meditativeplus-022", "lane": 3, "faceUp": true },
                    { "def": "classic-074", "lane": 4, "faceUp": true, "counters": { "plague": 3 } },
                ],
                "library": lib,
                "graveyard": ["core-008", "core-005", "core-026", "core-067"],
                "mana": 4,
            },
            "p2": mixed_p2,
        }),
        "gy" => json!({
            "seed": "audit-plays",
            "p1": {
                "hand": ["core-008", "core-005"],
                "field": [{ "def": "core-008", "lane": 1 }, { "def": "core-t-sheep", "lane": 2 }],
                "backrow": [
                    { "def": "classic-028", "lane": 1, "faceUp": true },
                    { "def": "classic-074", "lane": 2, "faceUp": true, "counters": { "plague": 3 } },
                ],
                "library": lib,
                "graveyard": [t, "core-008", "core-005"],
                "mana": 10,
            },
            "p2": mixed_p2,
        }),
        "giga" => json!({
            "seed": "audit-plays",
            "p1": {
                "hand": [t, "core-008", "core-005"],
                "field": [{ "def": "core-008", "lane": 5 }],
                "backrow": [{ "def": "stinkyscapeplus-053", "lane": 1, "faceUp": true }],
                "library": lib,
                "graveyard": ["core-008", "core-005"],
                "mana": 10,
            },
            "p2": {
                "hand": ["core-008", "core-005"],
                "field": [{ "def": "stinkyscapeplus-t-ai-04", "lane": 1 }, { "def": "core-019", "lane": 5 }],
                "backrow": [{ "def": "stinkyscapeplus-053", "lane": 3, "faceUp": true }],
                "library": lib,
                "graveyard": ["core-008"],
            },
        }),
        "tyrant" | "tyrant-empty" => {
            let mut p2 = mixed_p2.clone();
            let mut p1 = mixed_p1(10);
            p1["hand"] = json!(["classic-056", "core-008", "core-005"]);
            p1["graveyard"] = json!([t]);
            if name == "tyrant-empty" {
                p1["field"] = json!([]);
                p1["backrow"] = json!([]);
                p2 = json!({ "hand": ["core-008"], "library": lib });
            }
            json!({ "seed": "audit-plays", "p1": p1, "p2": p2 })
        }
        _ => return None,
    };
    let mut opts = opts;
    opts["seed"] = json!(seed);
    catch_unwind(AssertUnwindSafe(|| scenario(opts))).ok()
}

fn sig(state: &GameState) -> String {
    let mut copy = state.clone();
    copy.applied = Vec::new();
    serde_json::to_string(&copy).unwrap_or_default()
}

fn send(state: &GameState, play: &PlayAction) -> Result<ReduceResult, String> {
    let action = Action::new(play.clone().into_body(), P1, "audit");
    catch_unwind(AssertUnwindSafe(|| reduce(state, &action))).map_err(|payload| {
        payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_else(|| "panic".to_string())
    })
}

fn universe(state: &GameState) -> Vec<Selection> {
    let mut out: Vec<Selection> = Vec::new();
    let mut ids: Vec<String> = Vec::new();
    for player in [P1, P2] {
        let side = &state.players[player];
        ids.extend(side.units.iter().flatten().flatten().map(|c| c.id.clone()));
        ids.extend(side.backrow.iter().flatten().map(|c| c.id.clone()));
        ids.extend(side.backrow_piles.iter().flatten().flatten().map(|c| c.id.clone()));
        ids.extend(side.carried.iter().flatten().flatten().map(|c| c.id.clone()));
        ids.extend(side.hand.iter().map(|c| c.id.clone()));
        ids.extend(side.graveyard.iter().take(3).map(|c| c.id.clone()));
        ids.extend(side.library.iter().take(1).map(|c| c.id.clone()));
    }
    ids.push("c9999".to_string());
    out.extend(ids.into_iter().map(|instance_id| Selection::Instance { instance_id }));
    for player in [P1, P2] {
        out.push(Selection::Hero { player });
        for row in [Row::Units, Row::Backrow] {
            for lane in [0, 1, 2, 3, 4, 5, 6] {
                out.push(Selection::Zone { player, row, lane });
            }
        }
    }
    out.push(Selection::None);
    out
}

fn mutations(state: &GameState, card: &CardInstance, base: &PlayAction) -> Vec<(&'static str, PlayAction)> {
    let mut out: Vec<(&'static str, PlayAction)> = Vec::new();
    let all = universe(state);
    let mana = state.players[P1].mana.current;
    out.push(("zone", PlayAction { zone: None, ..base.clone() }));
    for row in [Row::Units, Row::Backrow] {
        for lane in 0..=6 {
            let zone = Some(ZoneChoice { row, lane });
            out.push(("zone", PlayAction { zone, ..base.clone() }));
            out.push(("equip", PlayAction { zone, equip: Some(true), ..base.clone() }));
        }
    }
    out.push(("equip", PlayAction { equip: Some(true), ..base.clone() }));
    out.push(("equip", PlayAction { equip: Some(false), ..base.clone() }));
    out.push(("equip", PlayAction { equip: None, ..base.clone() }));
    for x in [None, Some(-1), Some(0), Some(1), Some(2), Some(mana), Some(mana + 1), Some(99), Some(i32::MAX)] {
        out.push(("x", PlayAction { x, ..base.clone() }));
    }
    for embiggen in [None, Some(true), Some(false)] {
        out.push(("embiggen", PlayAction { embiggen, ..base.clone() }));
    }
    let field: Vec<String> = all
        .iter()
        .filter_map(|s| match s {
            Selection::Instance { instance_id } => Some(instance_id.clone()),
            _ => None,
        })
        .collect();
    out.push(("tributes", PlayAction { tributes: None, ..base.clone() }));
    out.push(("tributes", PlayAction { tributes: Some(Vec::new()), ..base.clone() }));
    let had = base.tributes.clone().unwrap_or_default();
    for id in &field {
        out.push(("tributes", PlayAction { tributes: Some(vec![id.clone()]), ..base.clone() }));
        out.push(("tributes", PlayAction { tributes: Some(vec![id.clone(), id.clone()]), ..base.clone() }));
        let mut more = had.clone();
        more.push(id.clone());
        out.push(("tributes", PlayAction { tributes: Some(more), ..base.clone() }));
        if let Some(first) = had.first() {
            let mut swapped = had.clone();
            swapped[0] = id.clone();
            if *first != *id {
                out.push(("tributes", PlayAction { tributes: Some(swapped), ..base.clone() }));
            }
        }
    }
    let own: Vec<String> = active_units_of(state, P1).into_iter().map(|u| u.id.clone()).collect();
    for a in &own {
        for b in &own {
            if a != b {
                out.push(("tributes", PlayAction { tributes: Some(vec![a.clone(), b.clone()]), ..base.clone() }));
            }
        }
    }
    let had = base.targets.clone().unwrap_or_default();
    out.push(("targets", PlayAction { targets: None, ..base.clone() }));
    out.push(("targets", PlayAction { targets: Some(Vec::new()), ..base.clone() }));
    for pick in &all {
        out.push(("targets", PlayAction { targets: Some(vec![pick.clone()]), ..base.clone() }));
        let mut more = had.clone();
        more.push(pick.clone());
        out.push(("targets", PlayAction { targets: Some(more), ..base.clone() }));
        for at in 0..had.len().min(3) {
            let mut swapped = had.clone();
            swapped[at] = pick.clone();
            out.push(("targets", PlayAction { targets: Some(swapped), ..base.clone() }));
        }
    }
    for at in 0..had.len().min(3) {
        let mut doubled = had.clone();
        doubled.push(had[at].clone());
        out.push(("targets", PlayAction { targets: Some(doubled), ..base.clone() }));
        let mut dropped = had.clone();
        dropped.remove(at);
        out.push(("targets", PlayAction { targets: Some(dropped), ..base.clone() }));
    }
    let mut options: Vec<String> = vec!["bogus".to_string()];
    for radiant in [false, true] {
        let face = CardInstance { radiant, ..card.clone() };
        for decl in declared_modes(state, &face) {
            options.extend(decl.options.iter().cloned());
        }
    }
    out.push(("modes", PlayAction { modes: None, ..base.clone() }));
    out.push(("modes", PlayAction { modes: Some(Vec::new()), ..base.clone() }));
    for a in &options {
        out.push(("modes", PlayAction { modes: Some(vec![a.clone()]), ..base.clone() }));
        for b in &options {
            out.push(("modes", PlayAction { modes: Some(vec![a.clone(), b.clone()]), ..base.clone() }));
        }
    }
    for reveal in [RevealAt::EndOfThisTurn, RevealAt::StartOfNextTurn, RevealAt::EndOfNextTurn, RevealAt::Cloaked] {
        out.push(("faceDown", PlayAction { face_down: Some(reveal), ..base.clone() }));
        out.push((
            "faceDown",
            PlayAction { face_down: Some(reveal), targets: None, modes: None, zone: None, ..base.clone() },
        ));
    }
    out.push(("plague", PlayAction { plague: None, ..base.clone() }));
    for id in field.iter().take(14) {
        for tokens in [-1, 0, 1, 2, 99] {
            out.push((
                "plague",
                PlayAction { plague: Some(PlagueSpend { from: id.clone(), tokens }), ..base.clone() },
            ));
        }
    }
    out
}

#[test]
fn explore() {
    jackioh_cards::register_all();
    std::panic::set_hook(Box::new(|_| {}));
    let filter = std::env::var("AUDIT_FILTER").unwrap_or_default();
    let only_board = std::env::var("AUDIT_BOARD").unwrap_or_default();
    let out_path = std::env::var("AUDIT_OUT").unwrap_or_default();
    if filter.is_empty() {
        return;
    }
    let defs: Vec<String> = registered_catalog()
        .keys()
        .filter(|id| !id.starts_with("infiniteassault") && id.starts_with(&filter))
        .cloned()
        .collect();
    let mut groups: BTreeMap<String, (usize, Vec<String>)> = BTreeMap::new();
    let mut note = |key: String, example: String| {
        let entry = groups.entry(key).or_insert((0, Vec::new()));
        entry.0 += 1;
        if entry.1.len() < 4 {
            entry.1.push(example);
        }
    };
    let mut reduces = 0usize;
    for def_id in &defs {
        for radiant in [false, true] {
            for name in ["mixed", "poor", "empty", "full", "mayor", "special", "gy", "giga"] {
                if !only_board.is_empty() && only_board != name {
                    continue;
                }
                let Some(s) = board(name, def_id, radiant) else {
                    note(format!("SETUP {name}"), def_id.clone());
                    continue;
                };
                let mut state = s.state().clone();
                state.applied = Vec::new();
                let card = if name == "gy" {
                    state.players[P1].graveyard[0].clone()
                } else {
                    state.players[P1].hand[0].clone()
                };
                let legal = match catch_unwind(AssertUnwindSafe(|| legal_actions(&state, P1))) {
                    Ok(legal) => legal,
                    Err(_) => {
                        note("LEGAL_ACTIONS PANIC".to_string(), format!("{def_id} r={radiant} {name}"));
                        continue;
                    }
                };
                let plays: Vec<PlayAction> = legal
                    .iter()
                    .filter_map(PlayAction::from_body)
                    .filter(|play| play.instance_id == card.id)
                    .collect();
                let truncated = hand_play_choices_truncated(&state, P1, &card);
                let mut listed_sigs: Vec<String> = Vec::new();
                let whole = plays.len() <= 1500;
                for play in plays.iter().take(1500) {
                    reduces += 1;
                    match send(&state, play) {
                        Err(panic) => note(
                            format!("LISTED PANIC {}", panic.chars().take(90).collect::<String>()),
                            format!("{def_id} r={radiant} {name} {}", serde_json::to_string(play).unwrap()),
                        ),
                        Ok(result) => match result.error {
                            Some(error) => note(
                                format!("LISTED REFUSED {}", error.chars().take(60).collect::<String>()),
                                format!("{def_id} r={radiant} {name} {} :: {error}", serde_json::to_string(play).unwrap()),
                            ),
                            None => listed_sigs.push(sig(&result.state)),
                        },
                    }
                }
                let mut templates: Vec<PlayAction> = Vec::new();
                if plays.is_empty() {
                    templates.push(PlayAction { instance_id: card.id.clone(), ..Default::default() });
                } else {
                    for at in [0, plays.len() / 2, plays.len() - 1] {
                        if !templates.contains(&plays[at]) {
                            templates.push(plays[at].clone());
                        }
                    }
                }
                let mut tried: IndexSet<PlayAction> = IndexSet::new();
                for template in &templates {
                    for (label, forged) in mutations(&state, &card, template) {
                        if plays.contains(&forged) || !tried.insert(forged.clone()) {
                            continue;
                        }
                        reduces += 1;
                        match send(&state, &forged) {
                            Err(panic) => note(
                                format!("FORGED PANIC {label} {}", panic.chars().take(90).collect::<String>()),
                                format!("{def_id} r={radiant} {name} {}", serde_json::to_string(&forged).unwrap()),
                            ),
                            Ok(result) => {
                                if result.error.is_some() {
                                    continue;
                                }
                                if listed_sigs.contains(&sig(&result.state)) {
                                    continue;
                                }
                                let why = if !whole {
                                    "manyplays"
                                } else if truncated {
                                    "truncated"
                                } else if plays.is_empty() {
                                    "NOPLAYS"
                                } else {
                                    "distinct"
                                };
                                note(
                                    format!("UNLISTED ACCEPTED {label} {why} {def_id}"),
                                    format!(
                                        "r={radiant} {name} forged={} template={}",
                                        serde_json::to_string(&forged).unwrap(),
                                        serde_json::to_string(template).unwrap()
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    let mut text = format!("defs {} reduces {reduces}\n", defs.len());
    for (key, (count, examples)) in &groups {
        text.push_str(&format!("== {key} x{count}\n"));
        for example in examples {
            text.push_str(&format!("   {example}\n"));
        }
    }
    if !out_path.is_empty() {
        std::fs::write(&out_path, &text).unwrap();
    }
    println!("{}", text.chars().take(6000).collect::<String>());
}

#[test]
fn dump_decls() {
    jackioh_cards::register_all();
    let out_path = std::env::var("AUDIT_DUMP").unwrap_or_default();
    if out_path.is_empty() {
        return;
    }
    std::panic::set_hook(Box::new(|_| {}));
    let mut text = String::new();
    for (def_id, def) in registered_catalog().iter() {
        if def_id.starts_with("infiniteassault") {
            continue;
        }
        for radiant in [false, true] {
            let Some(s) = board("empty", def_id, radiant) else { continue };
            let card = s.state().players[P1].hand[0].clone();
            let decls = declared_targets(s.state(), &card);
            let modes = declared_modes(s.state(), &card);
            if decls.is_empty() && modes.is_empty() {
                continue;
            }
            let multi = decls.len() > 1;
            let wide = decls.iter().enumerate().any(|(at, d)| at + 1 < decls.len() && d.max > d.min);
            text.push_str(&format!(
                "{def_id} r={radiant} {:?} \"{}\" multi={multi} wide={wide} decls={} modes={}\n",
                def.type_,
                def.name,
                serde_json::to_string(&decls).unwrap(),
                serde_json::to_string(&modes).unwrap()
            ));
        }
    }
    std::fs::write(&out_path, text).unwrap();
}

fn short(text: &str) -> String {
    let cut: String = text.chars().take(64).collect();
    cut.chars().map(|c| if c.is_ascii_digit() { '#' } else { c }).collect()
}

fn walk(state: &GameState, depth: usize, what: &str, note: &mut dyn FnMut(String, String)) {
    if state.result.is_some() || depth > 6 {
        return;
    }
    let Some(pending) = &state.pending else { return };
    for finding in offered_target_findings(state) {
        note(format!("PROMPT {}", short(&finding)), format!("{what} :: {finding}"));
    }
    let who = pending.player_id;
    let answers: Vec<ActionBody> = legal_actions(state, who)
        .into_iter()
        .filter(|action| matches!(action, ActionBody::Answer { .. }))
        .collect();
    if answers.is_empty() {
        note("PROMPT NO ANSWER".to_string(), format!("{what} kind={:?} {}", pending.kind, pending.prompt));
        return;
    }
    let stride = (answers.len() / 6).max(1);
    for (at, answer) in answers.iter().enumerate().filter(|(at, _)| at % stride == 0).take(7) {
        let action = Action::new(answer.clone(), who, format!("walk{depth}"));
        let sent = serde_json::to_string(answer).unwrap_or_default();
        match catch_unwind(AssertUnwindSafe(|| reduce(state, &action))) {
            Err(payload) => {
                let text = payload
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default();
                note(format!("ANSWER PANIC {}", short(&text)), format!("{what} -> {sent} :: {text}"));
            }
            Ok(result) => {
                if let Some(error) = result.error {
                    note(format!("ANSWER REFUSED {}", short(&error)), format!("{what} -> {sent} :: {error}"));
                    continue;
                }
                for finding in branch_state_findings(&result.events, &result.state) {
                    note(format!("BRANCH {}", short(&finding)), format!("{what} -> {sent} :: {finding}"));
                }
                if at == 0 || at + stride >= answers.len() {
                    walk(&result.state, depth + 1, &format!("{what} -> {sent}"), note);
                }
            }
        }
    }
}

#[test]
fn explore_deep() {
    jackioh_cards::register_all();
    let filter = std::env::var("AUDIT_DEEP").unwrap_or_default();
    let out_path = std::env::var("AUDIT_OUT").unwrap_or_default();
    if filter.is_empty() {
        return;
    }
    std::panic::set_hook(Box::new(|_| {}));
    let defs: Vec<(String, bool)> = registered_catalog()
        .iter()
        .filter(|(id, _)| !id.starts_with("infiniteassault") && id.starts_with(&filter))
        .map(|(id, def)| (id.clone(), def.token))
        .collect();
    let mut groups: BTreeMap<String, (usize, Vec<String>)> = BTreeMap::new();
    let mut note = |key: String, example: String| {
        let entry = groups.entry(key).or_insert((0, Vec::new()));
        entry.0 += 1;
        if entry.1.len() < 3 {
            entry.1.push(example.chars().take(900).collect());
        }
    };
    for (def_id, token) in &defs {
        for radiant in [false, true] {
            for name in ["mixed", "giga", "special", "gy", "full"] {
                if name == "gy" && *token {
                    continue;
                }
                let Some(s) = board(name, def_id, radiant) else { continue };
                let mut state = s.state().clone();
                state.applied = Vec::new();
                let card = if name == "gy" {
                    state.players[P1].graveyard[0].clone()
                } else {
                    state.players[P1].hand[0].clone()
                };
                let Ok(legal) = catch_unwind(AssertUnwindSafe(|| legal_actions(&state, P1))) else { continue };
                let plays: Vec<PlayAction> = legal
                    .iter()
                    .filter_map(PlayAction::from_body)
                    .filter(|play| play.instance_id == card.id)
                    .collect();
                let stride = (plays.len() / 14).max(1);
                for play in plays.iter().step_by(stride).take(16) {
                    let what = format!("{def_id} r={radiant} {name} {}", serde_json::to_string(play).unwrap());
                    let Ok(result) = send(&state, play) else { continue };
                    if result.error.is_some() {
                        continue;
                    }
                    for finding in branch_state_findings(&result.events, &result.state) {
                        note(format!("BRANCH {}", short(&finding)), format!("{what} :: {finding}"));
                    }
                    walk(&result.state, 0, &what, &mut note);
                }
            }
        }
    }
    let mut text = format!("defs {}\n", defs.len());
    for (key, (count, examples)) in &groups {
        text.push_str(&format!("== {key} x{count}\n"));
        for example in examples {
            text.push_str(&format!("   {example}\n"));
        }
    }
    if !out_path.is_empty() {
        std::fs::write(&out_path, &text).unwrap();
    }
    println!("{}", text.chars().take(5000).collect::<String>());
}

fn diff(a: &Value, b: &Value, path: &str, out: &mut Vec<String>) {
    if a == b || out.len() > 40 {
        return;
    }
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            let mut keys: Vec<&String> = x.keys().chain(y.keys()).collect();
            keys.sort();
            keys.dedup();
            for key in keys {
                diff(x.get(key).unwrap_or(&Value::Null), y.get(key).unwrap_or(&Value::Null), &format!("{path}/{key}"), out);
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (i, (p, q)) in x.iter().zip(y).enumerate() {
                diff(p, q, &format!("{path}/{i}"), out);
            }
        }
        _ => {
            let cut = |v: &Value| v.to_string().chars().take(160).collect::<String>();
            out.push(format!("{path}: {} != {}", cut(a), cut(b)));
        }
    }
}

#[test]
fn probe_tribute_order() {
    jackioh_cards::register_all();
    if std::env::var("AUDIT_PROBE").unwrap_or_default() != "tribute" {
        return;
    }
    let s = board("mixed", "stinkyscape-085", false).unwrap();
    let mut state = s.state().clone();
    state.applied = Vec::new();
    let play = |tributes: &[&str]| PlayAction {
        instance_id: "c1".to_string(),
        zone: Some(ZoneChoice { row: Row::Units, lane: 1 }),
        tributes: Some(tributes.iter().map(|s| s.to_string()).collect()),
        ..Default::default()
    };
    let a = send(&state, &play(&["c5", "c7"])).unwrap();
    let b = send(&state, &play(&["c7", "c5"])).unwrap();
    println!("errors {:?} {:?}", a.error, b.error);
    let mut out = Vec::new();
    let (mut sa, mut sb) = (a.state.clone(), b.state.clone());
    sa.applied = Vec::new();
    sb.applied = Vec::new();
    diff(&serde_json::to_value(&sa).unwrap(), &serde_json::to_value(&sb).unwrap(), "", &mut out);
    for line in out {
        println!("DIFF {line}");
    }
}

#[test]
fn probe_giga_setup() {
    jackioh_cards::register_all();
    if std::env::var("AUDIT_PROBE").unwrap_or_default() != "giga" {
        return;
    }
    let s = scenario(json!({
        "p1": { "hand": ["core-008"], "backrow": [{ "def": "stinkyscapeplus-053", "lane": 1, "faceUp": true }], "mana": 10 },
        "p2": { "field": [{ "def": "stinkyscapeplus-t-ai-04", "lane": 1 }, { "def": "core-019", "lane": 4 }] },
    }));
    println!("{}", serde_json::to_string(&s.state().players[P2].units).unwrap().chars().take(600).collect::<String>());
    println!("{}", serde_json::to_string(&s.state().players[P1].backrow).unwrap().chars().take(900).collect::<String>());
}

#[test]
fn explore_casts() {
    jackioh_cards::register_all();
    let mode = std::env::var("AUDIT_CASTS").unwrap_or_default();
    let out_path = std::env::var("AUDIT_OUT").unwrap_or_default();
    if mode.is_empty() {
        return;
    }
    std::panic::set_hook(Box::new(|_| {}));
    let mut groups: BTreeMap<String, (usize, Vec<String>)> = BTreeMap::new();
    let mut note = |key: String, example: String| {
        let entry = groups.entry(key).or_insert((0, Vec::new()));
        entry.0 += 1;
        if entry.1.len() < 3 {
            entry.1.push(example.chars().take(1200).collect());
        }
    };
    let mut runs: Vec<(String, bool, &str, String)> = Vec::new();
    if mode == "tyrant" {
        for (id, def) in registered_catalog().iter() {
            if id.starts_with("infiniteassault") || def.type_ != CardType::Spell {
                continue;
            }
            for radiant in [false, true] {
                for name in ["tyrant", "tyrant-empty"] {
                    runs.push((id.clone(), radiant, name, "audit-plays".to_string()));
                }
            }
        }
    } else {
        let seeds: usize = std::env::var("AUDIT_SEEDS").ok().and_then(|n| n.parse().ok()).unwrap_or(60);
        for id in mode.split(',') {
            for radiant in [false, true] {
                for name in ["mixed", "giga", "full", "special", "empty"] {
                    for seed in 0..seeds {
                        runs.push((id.to_string(), radiant, name, format!("s{seed}")));
                    }
                }
            }
        }
    }
    for (def_id, radiant, name, seed) in &runs {
        let Some(s) = board_seeded(name, def_id, *radiant, seed) else {
            note(format!("SETUP {name}"), def_id.clone());
            continue;
        };
        let mut state = s.state().clone();
        state.applied = Vec::new();
        let card = state.players[P1].hand[0].clone();
        let Ok(legal) = catch_unwind(AssertUnwindSafe(|| legal_actions(&state, P1))) else { continue };
        let plays: Vec<PlayAction> = legal
            .iter()
            .filter_map(PlayAction::from_body)
            .filter(|play| play.instance_id == card.id)
            .collect();
        let stride = (plays.len() / 3).max(1);
        for play in plays.iter().step_by(stride).take(4) {
            let what = format!("{def_id} r={radiant} {name} seed={seed} {}", serde_json::to_string(play).unwrap());
            match send(&state, play) {
                Err(panic) => note(format!("PLAY PANIC {}", short(&panic)), format!("{what} :: {panic}")),
                Ok(result) => {
                    if let Some(error) = result.error {
                        note(format!("PLAY REFUSED {}", short(&error)), format!("{what} :: {error}"));
                        continue;
                    }
                    for finding in branch_state_findings(&result.events, &result.state) {
                        note(format!("BRANCH {}", short(&finding)), format!("{what} :: {finding}"));
                    }
                    walk(&result.state, 0, &what, &mut note);
                }
            }
        }
    }
    let mut text = format!("runs {}\n", runs.len());
    for (key, (count, examples)) in &groups {
        text.push_str(&format!("== {key} x{count}\n"));
        for example in examples {
            text.push_str(&format!("   {example}\n"));
        }
    }
    if !out_path.is_empty() {
        std::fs::write(&out_path, &text).unwrap();
    }
    println!("{}", text.chars().take(5000).collect::<String>());
}

#[test]
fn probe_equip_hand_over() {
    jackioh_cards::register_all();
    if std::env::var("AUDIT_PROBE").unwrap_or_default() != "equip" {
        return;
    }
    for tested in ["core-055", "core-052"] {
        let s = scenario(json!({
            "p1": { "hand": [tested], "field": [{ "def": "core-008", "lane": 5 }],
                    "backrow": [{ "def": "stinkyscape-016", "lane": 1, "faceUp": true }], "mana": 10 },
            "p2": { "field": [{ "def": "core-008", "lane": 1 }, { "def": "core-008", "lane": 2 }, { "def": "core-008", "lane": 3 }] },
        }));
        let state = s.state().clone();
        let id = state.players[P1].hand[0].id.clone();
        let plays: Vec<PlayAction> = legal_actions(&state, P1)
            .iter()
            .filter_map(PlayAction::from_body)
            .filter(|play| play.instance_id == id && play.equip == Some(true))
            .collect();
        println!("{tested}: {} equip plays", plays.len());
        let Some(play) = plays.iter().find(|p| tested != "core-055" || p.tributes.as_ref().is_some_and(|t| t.iter().all(|id| state.players[P2].units.iter().flatten().flatten().any(|u| u.id == *id)))) else { continue };
        println!("play {}", serde_json::to_string(play).unwrap());
        let result = send(&state, play).unwrap();
        println!("error {:?} pending {:?}", result.error, result.state.pending.as_ref().map(|p| p.prompt.clone()));
        for event in &result.events {
            let text = serde_json::to_string(event).unwrap();
            if ["cardPlayed", "summoned", "fused", "cardResolved", "destroyed", "controlChanged"].iter().any(|k| text.contains(&format!("\"type\":\"{k}\""))) {
                println!("  EV {}", text.chars().take(260).collect::<String>());
            }
        }
        for player in [P1, P2] {
            let side = &result.state.players[player];
            let units: Vec<String> = side.units.iter().map(|pile| pile.as_ref().map_or("-".to_string(), |p| p.iter().map(|c| format!("{}:{}", c.id, c.def_id)).collect::<Vec<_>>().join("+"))).collect();
            let back: Vec<String> = side.backrow.iter().map(|c| c.as_ref().map_or("-".to_string(), |c| format!("{}:{}:ctl={}", c.id, c.def_id, c.controller))).collect();
            println!("  {player} units {units:?} backrow {back:?}");
        }
    }
}

#[test]
fn probe_ticketed() {
    jackioh_cards::register_all();
    if std::env::var("AUDIT_PROBE").unwrap_or_default() != "ticketed" {
        return;
    }
    for name in ["giga", "mixed", "special"] {
        let s = board(name, "stinkyscape-009", false).unwrap();
        let state = s.state().clone();
        let id = state.players[P1].hand[0].id.clone();
        let plays: Vec<PlayAction> = legal_actions(&state, P1)
            .iter()
            .filter_map(PlayAction::from_body)
            .filter(|play| play.instance_id == id)
            .collect();
        println!("{name}: {} plays", plays.len());
        for play in &plays {
            let target = match play.targets.as_deref() {
                Some([Selection::Instance { instance_id }]) => instance_id.clone(),
                other => format!("{other:?}"),
            };
            let named = find_instance(&state, &target).map(|c| format!("{} {:?} giga={:?} slumber={}", c.def_id, c.zone, c.giga, c.slumber.is_some())).unwrap_or_default();
            let result = send(&state, play).unwrap();
            let after = find_instance(&result.state, &id).map(|c| format!("{:?} ctl={}", c.zone, c.controller)).unwrap_or("gone".to_string());
            let mana = result.state.players[P1].mana.current;
            println!("  -> {target} [{named}] err={:?} ticketed now {after} mana {mana}", result.error);
        }
    }
}

// ---- Findings ----

fn plays_of(state: &GameState, id: &str) -> Vec<PlayAction> {
    legal_actions(state, P1)
        .iter()
        .filter_map(PlayAction::from_body)
        .filter(|play| play.instance_id == id)
        .collect()
}

/// R4665000: Ticketed's play "lands on top of that permanent's pile, in its zone"; its graveyard is
/// only for "a permanent that has left the field or been covered by the time Ticketed is placed" or
/// "no permanent on the field to name". R3546000: "Nothing stacks ... onto ... a GIGA"; R688: "a
/// play's Stack entry still honors a Lock". So a permanent Ticketed can never stack on is no legal
/// pick, as a Stack play's zones (`legal_zones_for`) and an Equip play's hosts (`equip_host_at`) read
/// `zones::accepts_stack_card`: every play of Ticketed `legal_actions` lists must put it on the field.
#[test]
fn finding_ticketed_is_offered_permanents_it_cannot_stack_on_and_is_lost_with_its_cost() {
    jackioh_cards::register_all();
    drop(std::panic::take_hook());
    let mut s = scenario(json!({
        "p1": {
            "hand": ["stinkyscape-009", "classic-084"],
            "field": [{ "def": "core-008", "lane": 5 }],
            "backrow": [{ "def": "stinkyscapeplus-053", "lane": 1, "faceUp": true }],
            "mana": 10,
        },
        "p2": {
            "hand": ["core-008"],
            "field": [{ "def": "stinkyscapeplus-t-ai-04", "lane": 1 }, { "def": "core-019", "lane": 5 }],
        },
    }));
    // Classic #84 Lockdown has Crater: it Locks its own backrow zone as it enters (R1850006).
    s.play("classic-084", json!({ "zone": 4 }));
    let state = s.state().clone();
    let ticketed = state.players[P1].hand[0].id.clone();
    let mut lost: Vec<String> = Vec::new();
    for play in nonempty(plays_of(&state, &ticketed), "plays of Ticketed") {
        let Some([Selection::Instance { instance_id: named }]) = play.targets.as_deref() else {
            panic!("Ticketed's play names one permanent: {play:?}");
        };
        let result = reduce(&state, &Action::new(play.clone().into_body(), P1, "ticketed"));
        assert_eq!(result.error, None, "a listed play is accepted");
        let card = must(find_instance(&result.state, &ticketed), "Ticketed after its play");
        if !matches!(card.zone, Zone::Field { .. }) {
            let target = must(find_instance(&state, named), "the named permanent");
            lost.push(format!(
                "onto {} ({}): Ticketed is in {:?}, mana {} -> {}",
                target.def_id,
                named,
                card.zone,
                state.players[P1].mana.current,
                result.state.players[P1].mana.current
            ));
        }
    }
    assert!(lost.is_empty(), "legal_actions offered Ticketed targets it cannot stack on:\n{}", lost.join("\n"));
}

/// R360: a play of #55 Lava Golem's base face "whose Tribute takes at least one unit the opponent
/// controls ... puts the Golem, at step 4, into the opponent's zone ... under the opponent's control".
/// R1086, R1869000: an Equip play fuses "the card on top" into its host, the Unit "standing on Rod";
/// "If either card left first, nothing is fused". Played as an Equip play onto Stinkyscape #16 Rod of
/// Ages with enemy Tributes, the Golem is summoned for the opponent and never stands on Rod, so it
/// must still be the opponent's once the play has resolved.
#[test]
fn finding_r360_a_lava_golem_handed_to_the_opponent_is_not_fused_back_into_its_players_rod_of_ages() {
    jackioh_cards::register_all();
    drop(std::panic::take_hook());
    let s = scenario(json!({
        "p1": {
            "hand": ["core-055"],
            "field": [{ "def": "core-008", "lane": 5 }],
            "backrow": [{ "def": "stinkyscape-016", "lane": 1, "faceUp": true }],
            "mana": 10,
        },
        "p2": { "hand": ["core-008"], "field": ["core-008", "core-008", "core-008"] },
    }));
    let state = s.state().clone();
    let golem = state.players[P1].hand[0].id.clone();
    let rod = must(state.players[P1].backrow[0].clone(), "Rod of Ages").id;
    let enemy: Vec<String> = active_units_of(&state, P2).into_iter().map(|unit| unit.id.clone()).collect();
    let play = must(
        plays_of(&state, &golem).into_iter().find(|play| {
            play.equip == Some(true) && play.tributes.as_ref().is_some_and(|paid| paid.iter().all(|id| enemy.contains(id)))
        }),
        "a listed Equip play of Lava Golem onto Rod of Ages that tributes three enemy Units",
    );
    let result = reduce(&state, &Action::new(play.into_body(), P1, "golem"));
    assert_eq!(result.error, None);
    assert!(result.state.pending.is_none());
    // R360's hand-over happened: step 4 summoned it for the opponent.
    assert!(result.events.iter().any(|event| matches!(
        event,
        GameEvent::Summoned { player: P2, instance_id, .. } if *instance_id == golem
    )));
    let after = find_instance(&result.state, &golem);
    let rod_after = must(find_instance(&result.state, &rod), "Rod of Ages after the play");
    assert!(
        after.is_some_and(|card| matches!(card.zone, Zone::Field { player: P2, .. }) && card.controller == P2),
        "R360: the Golem is the opponent's; it is {:?}, and Rod of Ages is now {} under {}",
        after.map(|card| card.zone.clone()),
        rod_after.def_id,
        rod_after.controller
    );
}

/// §9.3, §10.2: "A refused action comes back with the input state, no events and its message".
/// §2.3, R348: an X above the mana the player can spend is refused ("X is above your current mana").
/// While a Mayor acts (R1200) `validate_play` draws the targets before it checks X, so Stinkyscape #99
/// No-No Words' target check multiplies the unchecked X (`per * x`, c099_no_no_words.rs:29).
#[test]
fn finding_a_play_naming_a_huge_x_is_refused_not_a_panic_while_a_mayor_acts() {
    jackioh_cards::register_all();
    drop(std::panic::take_hook());
    let s = scenario(json!({
        "p1": { "hand": ["stinkyscape-099"], "mana": 4 },
        "p2": { "hand": ["core-008"], "field": ["meditative-086", "core-008"] },
    }));
    let state = s.state().clone();
    let play = PlayAction {
        instance_id: state.players[P1].hand[0].id.clone(),
        x: Some(i32::MAX),
        ..Default::default()
    };
    let action = Action::new(play.into_body(), P1, "huge-x");
    let outcome = catch_unwind(AssertUnwindSafe(|| reduce(&state, &action)));
    assert!(
        outcome.as_ref().is_ok_and(|result| result.error.is_some()),
        "reduce must refuse X = i32::MAX, not panic: {:?}",
        outcome.map(|result| result.error).map_err(|payload| payload.downcast_ref::<String>().cloned())
    );
}
