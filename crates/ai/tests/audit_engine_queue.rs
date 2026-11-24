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
            let a = reduce(after, &Action::new(body.clone(), holder, "n".into()));
            let b = reduce(
                after,
                &Action::new(
                    ActionBody::Answer {
                        choice_id: choice_id.clone(),
                        selection: reversed,
                    },
                    holder,
                    "n".into(),
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
