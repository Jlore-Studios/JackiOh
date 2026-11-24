//! Audit of the engine's work queue, prompts, traps and counters. Each test asserts the ruled behaviour.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

fn games(count: usize, tag: &str, max_actions: usize) -> Vec<(GameState, GameState, PlayerId, ActionBody)> {
    jackioh_cards::register_all();
    let mut out = Vec::new();
    for game in 0..count {
        let seed = format!("audit-queue:{tag}:{game}");
        let deck = |seat: PlayerId| {
            build_ai_deck(
                &mut Rng::new(&format!("{seed}:deck:{}", seat.as_str()), 0),
                DECK_SIZE,
                &AiDeckOptions {
                    banned: Some(vec![]),
                    gated_sets: Some(vec![]),
                    ..AiDeckOptions::default()
                },
            )
        };
        let config = MatchConfig {
            seed: seed.clone(),
            decks: (deck(PlayerId::P1), deck(PlayerId::P2)),
            handicaps: None,
            controllers: PerPlayer {
                p1: SeatController::Random,
                p2: SeatController::Random,
            },
            max_actions: Some(max_actions),
            setup: MatchSetup::default(),
        };
        let mut hooks = MatchHooks {
            after_action: Some(Box::new(
                |before: &GameState, after: &GameState, seat: PlayerId, action: &ActionBody| {
                    out.push((before.clone(), after.clone(), seat, action.clone()));
                },
            )),
            ..MatchHooks::default()
        };
        play_match(&config, &mut hooks);
    }
    out
}

fn sel_key(selection: &[Selection]) -> Vec<String> {
    let mut keys: Vec<String> = selection
        .iter()
        .map(|pick| serde_json::to_string(pick).unwrap())
        .collect();
    keys.sort();
    keys
}

fn all_instance_ids(state: &GameState) -> Vec<String> {
    let mut out = Vec::new();
    for player in PLAYER_IDS {
        let side = &state.players[player];
        for card in side
            .hand
            .iter()
            .chain(side.graveyard.iter())
            .chain(side.library.iter().take(3))
            .chain(side.exile.iter().take(3))
            .chain(side.units.iter().flatten().flatten())
            .chain(side.backrow.iter().flatten())
        {
            out.push(card.id.clone());
        }
    }
    out
}

/// PROBE: crafted answers against every prompt a random game opens.
#[test]
fn probe_crafted_answers() {
    let steps = games(40, "crafted", 500);
    let mut prompts = 0usize;
    let mut kinds: IndexMap<String, usize> = IndexMap::new();
    let mut problems: Vec<String> = Vec::new();
    for (index, (_before, after, _seat, _action)) in steps.iter().enumerate() {
        let Some(pending) = after.pending.clone() else {
            continue;
        };
        if after.result.is_some() {
            continue;
        }
        prompts += 1;
        *kinds
            .entry(format!("{:?}/{}", pending.kind, pending.resume.hook))
            .or_default() += 1;
        let holder = pending.player_id;
        let legal: Vec<ActionBody> = legal_actions(after, holder);
        let listed: IndexSet<Vec<String>> = legal
            .iter()
            .filter_map(|body| match body {
                ActionBody::Answer { selection, .. } => Some(sel_key(selection)),
                _ => None,
            })
            .collect();
        let capped = listed.len() >= 256;
        if listed.is_empty() {
            problems.push(format!(
                "step {index}: prompt {:?} hook {} of {} has NO legal answer (min {} max {} options {})",
                pending.kind,
                pending.resume.hook,
                pending.resume.def_id,
                pending.min,
                pending.max,
                pending.options.len()
            ));
        }
        // Candidate picks: the offered ones, every instance in the game, heroes, none, modes, zones.
        let mut picks: Vec<Selection> = pending.options.iter().map(|o| o.selection.clone()).collect();
        for id in all_instance_ids(after) {
            picks.push(Selection::Instance { instance_id: id });
        }
        picks.push(Selection::Hero { player: PlayerId::P1 });
        picks.push(Selection::Hero { player: PlayerId::P2 });
        picks.push(Selection::None);
        picks.push(Selection::Mode { option: "zzz".into() });
        picks.push(Selection::Mode { option: "0".into() });
        picks.push(Selection::Mode { option: "-1".into() });
        picks.push(Selection::Mode { option: "99999999999".into() });
        picks.push(Selection::Zone { player: holder, row: Row::Units, lane: 1 });
        picks.push(Selection::Zone { player: holder, row: Row::Backrow, lane: 99 });
        picks.push(Selection::Zone { player: holder, row: Row::Units, lane: -1 });
        let mut crafted: Vec<(String, PlayerId, String, Vec<Selection>)> = Vec::new();
        crafted.push(("empty".into(), holder, pending.id.clone(), vec![]));
        for pick in &picks {
            crafted.push(("single".into(), holder, pending.id.clone(), vec![pick.clone()]));
            crafted.push(("double".into(), holder, pending.id.clone(), vec![pick.clone(), pick.clone()]));
        }
        if let Some(first) = pending.options.first() {
            for pick in picks.iter().take(pending.options.len() + 12) {
                crafted.push((
                    "pair".into(),
                    holder,
                    pending.id.clone(),
                    vec![first.selection.clone(), pick.clone()],
                ));
            }
            crafted.push((
                "wrong-id".into(),
                holder,
                "q999999".into(),
                vec![first.selection.clone()],
            ));
            crafted.push((
                "wrong-seat".into(),
                opponent_of(holder),
                pending.id.clone(),
                vec![first.selection.clone()],
            ));
        }
        crafted.push((
            "all".into(),
            holder,
            pending.id.clone(),
            pending.options.iter().map(|o| o.selection.clone()).collect(),
        ));
        for (label, seat, choice_id, selection) in crafted {
            let action = Action::new(
                ActionBody::Answer {
                    choice_id: choice_id.clone(),
                    selection: selection.clone(),
                },
                seat,
                format!("crafted:{index}:{label}"),
            );
            let state = after.clone();
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| reduce(&state, &action)));
            match outcome {
                Err(_) => problems.push(format!(
                    "step {index}: PANIC on {label} answer {:?} to {:?} hook {} of {}",
                    selection, pending.kind, pending.resume.hook, pending.resume.def_id
                )),
                Ok(result) => {
                    if result.error.is_some() {
                        if result.state != *after {
                            problems.push(format!("step {index}: refused {label} changed the state"));
                        }
                        continue;
                    }
                    let key = sel_key(&selection);
                    let ok = seat == holder && choice_id == pending.id && (listed.contains(&key) || capped);
                    if !ok && pending.kind != PromptKind::Craft {
                        problems.push(format!(
                            "step {index}: ACCEPTED unlisted {label} answer {:?} to {:?} hook {} of {} (min {} max {} options {:?})",
                            selection,
                            pending.kind,
                            pending.resume.hook,
                            pending.resume.def_id,
                            pending.min,
                            pending.max,
                            pending.options.iter().map(|o| o.key.clone()).collect::<Vec<_>>()
                        ));
                    }
                }
            }
        }
        // R221: the order an answer lists its picks in must not change what it does.
        for body in legal.iter().take(40) {
            let ActionBody::Answer { choice_id, selection } = body else {
                continue;
            };
            if selection.len() < 2 {
                continue;
            }
            let mut reversed = selection.clone();
            reversed.reverse();
            let a = reduce(after, &Action::new(body.clone(), holder, "n"));
            let b = reduce(
                after,
                &Action::new(
                    ActionBody::Answer {
                        choice_id: choice_id.clone(),
                        selection: reversed,
                    },
                    holder,
                    "n",
                ),
            );
            if a.error != b.error || (a.error.is_none() && hash_state(&a.state) != hash_state(&b.state)) {
                problems.push(format!(
                    "step {index}: answer order matters for {:?} hook {} of {}: {:?} vs {:?}",
                    pending.kind, pending.resume.hook, pending.resume.def_id, a.error, b.error
                ));
            }
        }
    }
    println!("prompts seen: {prompts}; kinds: {kinds:?}");
    problems.sort();
    problems.dedup();
    for problem in problems.iter().take(60) {
        println!("{problem}");
    }
    assert!(problems.is_empty(), "{} problems", problems.len());
}

/// PROBE: a timeout from either seat at every state; a JSON round trip of every state before its action.
#[test]
fn probe_timeout_and_round_trip() {
    let steps = games(24, "timeout", 500);
    let mut problems: Vec<String> = Vec::new();
    let (mut timeouts, mut trips) = (0usize, 0usize);
    for (index, (before, after, seat, action)) in steps.iter().enumerate() {
        // JSON round trip (§10.1, work.rs header: "a paused state survives a JSON round trip and replays exactly").
        let text = serde_json::to_string(before).unwrap();
        let back: GameState = serde_json::from_str(&text).unwrap();
        let act = Action::new(action.clone(), *seat, "trip");
        let direct = reduce(before, &act);
        let tripped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| reduce(&back, &act)));
        trips += 1;
        match tripped {
            Err(_) => problems.push(format!("step {index}: PANIC after JSON round trip on {action:?}")),
            Ok(tripped) => {
                if tripped.error != direct.error || hash_state(&tripped.state) != hash_state(&direct.state) {
                    problems.push(format!(
                        "step {index}: JSON round trip changes {:?} (pending {:?}, work {:?}): {:?} vs {:?}",
                        action.action_type(),
                        before.pending.as_ref().map(|p| (p.kind, p.resume.hook.clone(), p.resume.def_id.clone())),
                        before.work.iter().map(|w| w.resume.hook.clone()).collect::<Vec<_>>(),
                        direct.error,
                        tripped.error
                    ));
                }
            }
        }
        if after.result.is_some() || after.mulligan.is_some() {
            continue;
        }
        for who in PLAYER_IDS {
            timeouts += 1;
            let act = Action::new(ActionBody::Timeout, who, "timeout");
            let state = after.clone();
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| reduce(&state, &act)));
            let Ok(result) = outcome else {
                problems.push(format!(
                    "step {index}: PANIC on timeout by {who} (pending {:?})",
                    after.pending.as_ref().map(|p| (p.kind, p.resume.hook.clone(), p.resume.def_id.clone()))
                ));
                continue;
            };
            if let Some(error) = &result.error {
                problems.push(format!("step {index}: timeout refused: {error}"));
                continue;
            }
            let next = &result.state;
            let held = after.pending.as_ref().map(|p| p.player_id);
            if who == after.active {
                // R79: every prompt of theirs is answered, then the turn ends; it stops at a prompt the
                // other player holds and once the turn has passed.
                let stopped = next.result.is_some()
                    || next.turn != after.turn
                    || next.pending.as_ref().is_some_and(|p| p.player_id != who);
                if !stopped {
                    problems.push(format!(
                        "step {index}: active timeout left turn {} open: phase {:?} pending {:?} (was {:?}) work {:?}",
                        after.turn,
                        next.phase,
                        next.pending.as_ref().map(|p| (p.kind, p.resume.hook.clone(), p.resume.def_id.clone(), p.player_id)),
                        after.pending.as_ref().map(|p| (p.kind, p.resume.hook.clone(), p.resume.def_id.clone(), p.player_id)),
                        next.work.iter().map(|w| w.resume.hook.clone()).collect::<Vec<_>>(),
                    ));
                }
            } else if held == Some(who) {
                let same = next.pending.as_ref().map(|p| p.id.clone()) == after.pending.as_ref().map(|p| p.id.clone());
                if same && next.result.is_none() {
                    problems.push(format!(
                        "step {index}: non-active timeout did not answer {:?}",
                        after.pending.as_ref().map(|p| (p.kind, p.resume.hook.clone(), p.resume.def_id.clone()))
                    ));
                }
            } else if hash_state(next) != hash_state(after) {
                problems.push(format!("step {index}: non-active timeout with no prompt of theirs changed the state"));
            }
        }
    }
    println!("timeouts {timeouts}, round trips {trips}");
    problems.sort();
    problems.dedup();
    for problem in problems.iter().take(40) {
        println!("{problem}");
    }
    assert!(problems.is_empty(), "{} problems", problems.len());
}

/// PROBE: Income Tax firing inside a random cast (Jogg's Box) of the drawer's.
#[test]
fn probe_income_tax_inside_random_cast() {
    jackioh_cards::register_all();
    const TAX: &str = "classic-009";
    const JOGG: &str = "classicplus-047";
    let filler: Vec<&str> = vec!["core-008"; 30];
    let (mut fired, mut wrong) = (0, 0);
    for n in 0..120 {
        let s = scenario(json!({
            "seed": format!("audit-tax:{n}"), "active": "p1", "turn": 9,
            "p1": { "hand": [JOGG, "core-008", "core-008", "core-008"], "mana": 10, "library": filler },
            "p2": { "hand": ["core-019", "core-012", "core-025"],
                    "backrow": [{ "def": TAX, "faceUp": false }], "library": filler },
        }));
        let before = s.state().clone();
        let p2_hand: Vec<String> = before.players[PlayerId::P2].hand.iter().map(|c| c.id.clone()).collect();
        let Some(play) = s.plays_of(JOGG).into_iter().next() else {
            continue;
        };
        let act = Action::new(play, PlayerId::P1, "jogg");
        let Ok(result) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| reduce(&before, &act))) else {
            println!("seed {n}: PANIC");
            continue;
        };
        if result.error.is_some() {
            continue;
        }
        let at = result.events.iter().position(
            |event| matches!(event, GameEvent::TrapFired { def_id, .. } if def_id == TAX),
        );
        let Some(at) = at else {
            continue;
        };
        fired += 1;
        let after = &result.state;
        let taken: Vec<&String> = p2_hand
            .iter()
            .filter(|id| after.players[PlayerId::P1].hand.iter().any(|c| &c.id == *id)
                || after.players[PlayerId::P1].graveyard.iter().any(|c| &c.id == *id))
            .collect();
        if !taken.is_empty() {
            wrong += 1;
            if wrong <= 2 {
                println!(
                    "seed {n}: pending {:?}; P2's hand cards now P1's: {taken:?}",
                    after.pending.as_ref().map(|p| (p.kind, p.player_id, p.resume.def_id.clone()))
                );
                for event in result.events.iter().skip(at.saturating_sub(3)).take(14) {
                    let text = serde_json::to_string(event).unwrap();
                    println!("   {}", &text[..text.len().min(230)]);
                }
            }
        } else if fired <= 3 {
            println!(
                "seed {n}: fired, pending {:?}, P1 hand {} P2 hand {}",
                after.pending.as_ref().map(|p| (p.kind, p.player_id, p.resume.def_id.clone())),
                after.players[PlayerId::P1].hand.len(),
                after.players[PlayerId::P2].hand.len()
            );
        }
    }
    println!("income tax fired in {fired} plays; P2's hand went to P1 in {wrong}");
    assert_eq!(wrong, 0);
}

/// PROBE: a triggered Cry's target prompt naming Paul Allen's Ghost (R450: "a prompt answer naming such
/// a card pays before it goes on").
#[test]
fn probe_triggered_cry_targeting_cost() {
    jackioh_cards::register_all();
    const REWIND: &str = "classic-054";
    const SORCERER: &str = "core-068";
    const GHOST: &str = "classic-089";
    let mut s = scenario(json!({
        "seed": "audit-rewind", "active": "p1", "turn": 9,
        "p1": { "hand": [REWIND, "core-008", "core-008", "core-008"], "mana": 10,
                "field": [SORCERER], "library": ["core-008", "core-008", "core-008"] },
        "p2": { "field": [GHOST], "library": ["core-008", "core-008", "core-008"] },
    }));
    let sorcerer = s.card(SORCERER).clone();
    let ghost = s.card(GHOST).clone();
    s.play(REWIND, json!({ "targets": [{ "pick": "instance", "instanceId": sorcerer.id }] }));
    let pending = s.state().pending.clone().expect("the triggered Cry asks its target");
    println!("prompt {:?} hook {} options {:?}", pending.kind, pending.resume.hook,
        pending.options.iter().map(|o| o.key.clone()).collect::<Vec<_>>());
    let hand_before = s.hand(PlayerId::P1).len();
    s.answer(json!([{ "pick": "instance", "instanceId": ghost.id }]));
    let discards = s
        .last_events()
        .iter()
        .filter(|event| matches!(event, GameEvent::Discarded { .. }))
        .count();
    println!("hand {hand_before} -> {}, discards {discards}", s.hand(PlayerId::P1).len());
    for event in s.last_events() {
        let text = serde_json::to_string(event).unwrap();
        println!("   {}", &text[..text.len().min(200)]);
    }
    assert_eq!(discards, 2, "R450: a prompt answer naming Paul Allen's Ghost pays its 2 discards");
}
