//! Audit scratch: the dungeon run's pipeline, explored end to end.
#![allow(clippy::all, unused)]

use std::panic::{AssertUnwindSafe, catch_unwind};

use jackioh_engine::catalog::registered_catalog;
use jackioh_engine::reduce::{begin_game, legal_actions, reduce, seat_to_act};
use jackioh_engine::replay::ReplayInput;
use jackioh_engine::rng::Rng;
use jackioh_engine::state::{CreateGameOptions, GameState, SetupKind, create_game, validate_setup};
use jackioh_engine::wire::{Action, ActionBody, PerPlayerOpt, PlayerId, Winner};
use jackioh_run::config::*;
use jackioh_run::*;

fn catalog_content() -> RunContent {
    jackioh_cards::register_all();
    run_catalog_content(registered_catalog())
}

fn options_of(setup: &RunFightSetup) -> CreateGameOptions {
    CreateGameOptions {
        seed: setup.seed.clone(),
        decks: (setup.human_deck.clone(), setup.enemy_deck.clone()),
        handicaps: Some(PerPlayerOpt {
            p1: Some(setup.human_handicap),
            p2: Some(setup.enemy_handicap),
        }),
        deck_marks: (!setup.human_marks.is_empty()).then(|| PerPlayerOpt {
            p1: Some(setup.human_marks.clone()),
            p2: None,
        }),
        ..Default::default()
    }
}

/// A random-policy game on `setup`, both seats; its fold input and its final state.
fn play_out(setup: &RunFightSetup, rng: &mut Rng, notes: &mut Vec<String>) -> Result<(ReplayInput, GameState), String> {
    let options = options_of(setup);
    validate_setup(&options, registered_catalog(), SetupKind::New).map_err(|e| format!("setup refused: {e}"))?;
    let mut state = begin_game(&create_game(&options)).state;
    let mut log: Vec<Action> = Vec::new();
    let mut steps = 0;
    while state.result.is_none() {
        steps += 1;
        let seat = seat_to_act(&state).ok_or("no seat acts")?;
        let legal = legal_actions(&state, seat);
        let pool: Vec<&ActionBody> = legal
            .iter()
            .filter(|b| !matches!(b, ActionBody::Concede))
            .collect();
        let body = if steps > 2500 || pool.is_empty() {
            ActionBody::Concede
        } else {
            // Lean off EndTurn so boards fill and fights happen.
            let non_end: Vec<&ActionBody> = pool.iter().copied().filter(|b| !matches!(b, ActionBody::EndTurn)).collect();
            if !non_end.is_empty() && rng.int(100) < 85 {
                (*rng.pick(&non_end).unwrap()).clone()
            } else {
                (*rng.pick(&pool).unwrap()).clone()
            }
        };
        let action = Action::new(body, seat, format!("n{}", log.len()));
        let result = reduce(&state, &action);
        if let Some(error) = &result.error {
            if notes.len() < 20 {
                notes.push(format!("legal action refused: {error}: {:?}", action.body));
            }
            // Fall back to EndTurn or Concede so the game goes on.
            let fallback = Action::new(ActionBody::Concede, seat, format!("n{}c", log.len()));
            let result = reduce(&state, &fallback);
            if result.error.is_some() {
                return Err(format!("concede refused: {:?}", result.error));
            }
            log.push(fallback);
            state = result.state;
            continue;
        }
        log.push(action);
        state = result.state;
    }
    let game = ReplayInput {
        seed: options.seed.clone(),
        decks: options.decks.clone(),
        log,
        handicaps: options.handicaps.clone(),
        deck_marks: options.deck_marks.clone(),
        ..Default::default()
    };
    Ok((game, state))
}

fn pick_index(rng: &mut Rng, len: usize) -> usize {
    rng.int(len as i32) as usize
}

/// One walk action; `shrink` leans on removals and skips.
fn walk_action(state: &RunState, content: &RunContent, rng: &mut Rng, shrink: bool) -> Vec<RunAction> {
    match &state.phase {
        RunPhase::Map => {
            let next = run_next_cols(state);
            vec![RunAction::Go { col: *rng.pick(&next).unwrap() }]
        }
        RunPhase::Reward { artifact: Some(_), .. } => vec![RunAction::TakeArtifact { take: true }],
        RunPhase::Reward { packs, .. } => vec![RunAction::OpenPack { index: pick_index(rng, packs.len()) }],
        RunPhase::Pack { cards, keep } => {
            let mut out = Vec::new();
            if shrink {
                out.push(RunAction::Keep { cards: Vec::new() });
            }
            out.push(RunAction::Keep { cards: rng.shuffle(cards).into_iter().take(*keep).collect() });
            out
        }
        RunPhase::ExtractCard => vec![
            RunAction::Extract { index: Some(pick_index(rng, state.deck.len())) },
            RunAction::Extract { index: None },
        ],
        RunPhase::ExtractArtifact => vec![RunAction::Extract { index: None }],
        RunPhase::Shop { stock } => {
            let mut out = Vec::new();
            for _ in 0..3 {
                out.push(match rng.int(5) {
                    0 => RunAction::RemoveCard { index: pick_index(rng, state.deck.len()) },
                    1 => RunAction::MakeRadiant { index: pick_index(rng, state.deck.len()) },
                    2 => RunAction::Buff { index: pick_index(rng, state.deck.len()) },
                    3 if !state.artifacts.is_empty() => RunAction::LevelArtifact { index: pick_index(rng, state.artifacts.len()) },
                    _ if !stock.is_empty() && !shrink => RunAction::Buy { index: pick_index(rng, stock.len()) },
                    _ => RunAction::Buff { index: pick_index(rng, state.deck.len()) },
                });
            }
            if rng.int(100) < 30 {
                out.insert(0, RunAction::Leave);
            }
            out.push(RunAction::Leave);
            out
        }
        RunPhase::Campfire => {
            let len = state.deck.len();
            let mut out = Vec::new();
            if len > 2 && (shrink || rng.int(100) < 30) {
                let a = pick_index(rng, len);
                let mut b = pick_index(rng, len - 1);
                if b >= a {
                    b += 1;
                }
                out.push(RunAction::RemoveCards { indices: vec![a, b] });
            }
            out.push(if rng.coin() {
                RunAction::Buff { index: pick_index(rng, len) }
            } else {
                RunAction::MakeRadiant { index: pick_index(rng, len) }
            });
            out.push(RunAction::GainMaxHp);
            out
        }
        RunPhase::Event { id } => {
            let n = content.events.iter().find(|e| &e.id == id).map_or(0, |e| e.options.len());
            let mut out = vec![RunAction::Choose { index: pick_index(rng, n) }];
            out.push(RunAction::Choose { index: n - 1 });
            out
        }
        RunPhase::Shrine { offers } => vec![RunAction::Bless { index: Some(pick_index(rng, offers.len())) }],
        RunPhase::Fight | RunPhase::Won | RunPhase::Dead => unreachable!(),
    }
}

#[test]
fn explore_pipeline() {
    let content = catalog_content();
    let runs: usize = std::env::var("AUDIT_RUNS").ok().and_then(|v| v.parse().ok()).unwrap_or(6);
    let mut notes: Vec<String> = Vec::new();
    let mut ends: std::collections::BTreeMap<String, usize> = Default::default();
    let mut fights = 0;
    for n in 0..runs {
        let shrink = n % 2 == 1;
        let mut rng = Rng::new(&format!("audit-run-{n}"), 0);
        let mut artifacts: Vec<String> = Vec::new();
        if shrink {
            artifacts = vec!["temp-picky".into(), "temp-thick-skin".into(), "temp-big-heart".into()];
        } else if n % 4 == 2 {
            artifacts = vec!["temp-greedy".into(), "temp-wide-net".into(), "temp-thick-skin".into()];
        }
        let setup = RunSetup {
            seed: run_seed(&format!("audit-run-{n}")),
            starter: RUN_STARTER_CARDS[..RUN_STARTER_SIZE].iter().map(|s| s.to_string()).collect(),
            artifacts,
            curses: content.curses.iter().filter(|_| rng.coin()).map(|c| c.id.clone()).collect(),
        };
        let mut state = run_create(&setup, &RunUnlocks::default(), &content).unwrap();
        let mut steps = 0;
        while !matches!(state.phase, RunPhase::Won | RunPhase::Dead) {
            steps += 1;
            assert!(steps < 3000, "run {n} loops");
            if state.phase == RunPhase::Fight {
                fights += 1;
                let fight = run_fight_setup(&state, &content).expect("a fight has a setup");
                let at = format!("run {n} row {:?} deck {} hp {} marks {}", state.row, state.deck.len(), state.hp, fight.human_marks.len());
                let played = catch_unwind(AssertUnwindSafe(|| play_out(&fight, &mut rng, &mut notes)));
                let report = match played {
                    Err(payload) => {
                        let text = payload.downcast_ref::<String>().cloned().or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
                        notes.push(format!("PANIC at {at}: {text}"));
                        None
                    }
                    Ok(Err(e)) => {
                        notes.push(format!("ERR at {at}: {e}"));
                        None
                    }
                    Ok(Ok((game, end))) => {
                        let result = end.result.unwrap();
                        *ends.entry(format!("{:?}", result.reason)).or_default() += 1;
                        match run_fight_report(&game, PlayerId::P1) {
                            Err(e) => {
                                notes.push(format!("REPORT ERR at {at}: {e}"));
                                None
                            }
                            Ok(action) => {
                                if let RunAction::FightOver { won, hp, damage_dealt, healing_done, cards_drawn, turns } = &action {
                                    if *won != (result.winner == Winner::P1) || *hp != end.players[PlayerId::P1].hero.health || *turns != end.turn {
                                        notes.push(format!("REPORT MISMATCH at {at}: {action:?} vs {result:?} hp {} turn {}", end.players[PlayerId::P1].hero.health, end.turn));
                                    }
                                    if *damage_dealt < 0 || *healing_done < 0 || *cards_drawn < 0 {
                                        notes.push(format!("NEGATIVE REPORT at {at}: {action:?}"));
                                    }
                                }
                                Some(action)
                            }
                        }
                    }
                };
                // Go on as a win, with the report's numbers when there is one.
                let (hp, dd, hd, cd, t) = match report {
                    Some(RunAction::FightOver { hp, damage_dealt, healing_done, cards_drawn, turns, .. }) => (hp, damage_dealt, healing_done, cards_drawn, turns),
                    _ => (1, 0, 0, 0, 0),
                };
                state = run_reduce(&state, &content, &RunAction::FightOver { won: true, hp, damage_dealt: dd.max(0), healing_done: hd.max(0), cards_drawn: cd.max(0), turns: t }).unwrap();
                continue;
            }
            let tries = walk_action(&state, &content, &mut rng, shrink);
            let mut moved = false;
            for action in tries {
                if let Ok(next) = run_reduce(&state, &content, &action) {
                    state = next;
                    moved = true;
                    break;
                }
            }
            assert!(moved, "run {n}: no way on from {:?}", state.phase);
        }
        eprintln!("run {n}: {:?} deck {} hp {} artifacts {:?} score {}", state.phase, state.deck.len(), state.hp, state.artifacts, run_score(&state, &content));
    }
    eprintln!("fights {fights} ends {ends:?}");
    for note in &notes {
        eprintln!("NOTE {note}");
    }
    assert!(notes.is_empty(), "{} notes", notes.len());
}

/// Every route's fights and tiles, by dynamic programming, and the graph's shape.
fn check_map(state: &RunState, tag: &str) -> Result<(), String> {
    let map = &state.map;
    let rows = RUN_ACTS * (RUN_ACT_ROWS + 1);
    if map.len() != rows {
        return Err(format!("{tag}: {} rows", map.len()));
    }
    // (fewest fights, most fights) on any route from the first row to each node.
    let mut counts: Vec<(i32, i32)> = Vec::new();
    for (r, row) in map.iter().enumerate() {
        let boss_row = (r + 1) % (RUN_ACT_ROWS + 1) == 0;
        if row.is_empty() {
            return Err(format!("{tag}: row {r} is empty"));
        }
        if boss_row && (row.len() != 1 || row[0].column != RUN_MAP_COLUMNS / 2) {
            return Err(format!("{tag}: boss row {r} is {} nodes", row.len()));
        }
        let above = map.get(r + 1).map_or(0, Vec::len);
        let mut reached = vec![false; above];
        for (c, node) in row.iter().enumerate() {
            if c > 0 && row[c - 1].column >= node.column {
                return Err(format!("{tag}: row {r} columns out of order"));
            }
            if node.column >= RUN_MAP_COLUMNS {
                return Err(format!("{tag}: row {r} column {}", node.column));
            }
            let want = if boss_row {
                if r + 1 == rows { RunNodeKind::Boss } else { RunNodeKind::MiniBoss }
            } else {
                node.kind
            };
            if node.kind != want || (!boss_row && node.kind.is_boss()) {
                return Err(format!("{tag}: row {r} node {c} is {:?}", node.kind));
            }
            if r == 0 && !node.kind.is_fight() {
                return Err(format!("{tag}: first row holds a tile"));
            }
            if node.kind.is_fight() != node.enemy.is_some() || (node.kind == RunNodeKind::Event) != node.event.is_some() {
                return Err(format!("{tag}: row {r} node {c} holds the wrong things"));
            }
            if r + 1 == rows {
                if !node.next.is_empty() {
                    return Err(format!("{tag}: the final boss leads on"));
                }
            } else if node.next.is_empty() {
                return Err(format!("{tag}: dead end at row {r} node {c}"));
            }
            for (i, &n) in node.next.iter().enumerate() {
                if n >= above || (i > 0 && node.next[i - 1] >= n) {
                    return Err(format!("{tag}: row {r} node {c} next {:?}", node.next));
                }
                reached[n] = true;
                let wide = (map[r + 1][n].column as i64 - node.column as i64).abs() > 1;
                let fan = boss_row || (r + 2) % (RUN_ACT_ROWS + 1) == 0;
                if wide && !fan {
                    return Err(format!("{tag}: a wide edge from row {r} node {c}"));
                }
            }
        }
        if reached.iter().any(|r| !r) {
            return Err(format!("{tag}: an orphan on row {}", r + 1));
        }
        // No two edges cross.
        for (a, na) in row.iter().enumerate() {
            for (b, nb) in row.iter().enumerate().skip(a + 1) {
                for &x in &na.next {
                    for &y in &nb.next {
                        if x > y {
                            return Err(format!("{tag}: edges cross above row {r}"));
                        }
                    }
                }
            }
        }
        let own = |n: &RunNode| i32::from(n.kind.is_fight());
        counts = if r == 0 {
            row.iter().map(|n| (own(n), own(n))).collect()
        } else {
            let below = &map[r - 1];
            row.iter()
                .enumerate()
                .map(|(i, n)| {
                    let from: Vec<(i32, i32)> = below.iter().zip(&counts).filter(|(b, _)| b.next.contains(&i)).map(|(_, c)| *c).collect();
                    (from.iter().map(|c| c.0).min().unwrap() + own(n), from.iter().map(|c| c.1).max().unwrap() + own(n))
                })
                .collect()
        };
    }
    let (fewest, most) = counts[0];
    let tiles_fewest = rows as i32 - most;
    if fewest < RUN_ROUTE_FIGHTS_MIN || most > RUN_ROUTE_FIGHTS_MAX || tiles_fewest < RUN_ROUTE_TILES_MIN {
        return Err(format!("{tag}: routes hold {fewest} to {most} fights, {tiles_fewest} tiles at least"));
    }
    Ok(())
}

#[test]
fn explore_maps() {
    let content = run_placeholder_content();
    let seeds: usize = std::env::var("AUDIT_SEEDS").ok().and_then(|v| v.parse().ok()).unwrap_or(2000);
    let mut bad = Vec::new();
    for n in 0..seeds {
        let setup = RunSetup {
            seed: run_seed(&format!("audit-map-{n}")),
            starter: content.starter_access[..RUN_STARTER_SIZE].to_vec(),
            artifacts: Vec::new(),
            curses: Vec::new(),
        };
        let made = catch_unwind(AssertUnwindSafe(|| run_create(&setup, &RunUnlocks::default(), &content)));
        match made {
            Err(_) => bad.push(format!("{}: run_create panics", setup.seed)),
            Ok(Err(e)) => bad.push(format!("{}: {e}", setup.seed)),
            Ok(Ok(state)) => {
                if let Err(e) = check_map(&state, &setup.seed) {
                    bad.push(e);
                }
                let again = run_create(&setup, &RunUnlocks::default(), &content).unwrap();
                if again != state {
                    bad.push(format!("{}: the same seed deals another run", setup.seed));
                }
            }
        }
    }
    for b in bad.iter().take(20) {
        eprintln!("BAD {b}");
    }
    assert!(bad.is_empty(), "{} bad maps of {seeds}", bad.len());
}
