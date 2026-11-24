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
