//! Seeded whole-run walks on both contents: a few hundred random full runs that check every
//! fight starts and every count stays in bounds (§15.7, §15.8, R2701000, R2922001).
//!
//! Each walk draws its choices from the engine's `Rng` on a fixed seed per walk (CLAUDE.md rule 4),
//! never touching the run's own `rng_cursor`, so the accepted log still `run_fold`s to the same end
//! state. A refused random pick is retried; the refusal-free `free_action` is the fallback, so no
//! phase is a dead end.

use super::*;
use crate::config::{
    RUN_STARTER_CARDS, WALK_DAMAGE_MAX, WALK_DRAWN_MAX, WALK_HEALING_MAX, WALK_LOSS_PERCENT, WALK_MAX_STEPS,
    WALK_SEEDS, WALK_TRIES, WALK_TURNS_MAX, WALK_WAY_ON_PERCENT,
};
use crate::fight_tests::{catalog_content, engine_takes, fight_options};
use jackioh_engine::reduce::begin_game;
use jackioh_engine::rng::Rng;
use jackioh_engine::state::create_recorded_game;
use jackioh_engine::wire::PlayerId;

/// A walk's start: the placeholder's first starting cards with one Greedy (as `tests.rs`), or the
/// catalog's first starting cards with none (as `catalog_run`); both under a random subset of the
/// content's curses, which need no unlock and stack in every fight's setup (R2922001).
fn walk_setup(tag: &str, n: usize, content: &RunContent, rng: &mut Rng) -> (RunSetup, RunUnlocks) {
    let starter = if tag == "placeholder" {
        content.starter_access[..RUN_STARTER_SIZE].to_vec()
    } else {
        RUN_STARTER_CARDS[..RUN_STARTER_SIZE]
            .iter()
            .map(|id| (*id).to_string())
            .collect()
    };
    let artifacts = if tag == "placeholder" {
        vec!["temp-greedy".to_string()]
    } else {
        Vec::new()
    };
    let curses = content
        .curses
        .iter()
        .filter(|_| rng.coin())
        .map(|curse| curse.id.clone())
        .collect();
    let setup = RunSetup {
        seed: run_seed(&format!("walk-2979-{tag}-{n}")),
        starter,
        artifacts,
        curses,
    };
    (setup, RunUnlocks::default())
}

/// The refusal-free way on from each phase: every clause is accepted while the run lives, so a
/// refusal names a dead end. The event's is the last option that costs nothing (the placeholder's
/// "Walk away"); content without one fails loudly here instead of hanging.
fn free_action(state: &RunState, content: &RunContent) -> RunAction {
    match &state.phase {
        RunPhase::Map => RunAction::Go {
            col: run_next_cols(state).first().copied().unwrap_or(0),
        },
        // `fight_won` clamps HP, so any won report is accepted.
        RunPhase::Fight => RunAction::FightOver {
            won: true,
            hp: state.hp,
            damage_dealt: 0,
            healing_done: 0,
            cards_drawn: 0,
            turns: 0,
        },
        RunPhase::Reward {
            artifact: Some(_), ..
        } => RunAction::TakeArtifact { take: false },
        RunPhase::Reward { .. } => RunAction::OpenPack { index: 0 },
        RunPhase::Pack { cards, keep } => RunAction::Keep {
            cards: cards[..*keep].to_vec(),
        },
        RunPhase::ExtractCard | RunPhase::ExtractArtifact => RunAction::Extract { index: None },
        RunPhase::Shop { .. } => RunAction::Leave,
        RunPhase::Campfire => RunAction::Heal,
        RunPhase::Event { id } => {
            let options = &content
                .events
                .iter()
                .find(|event| &event.id == id)
                .expect("walk: the event is content's")
                .options;
            let index = options
                .iter()
                .rposition(|option| option.hp >= 0 && option.coins >= 0 && option.tickets >= 0)
                .expect("walk: the event has a free option");
            RunAction::Choose { index }
        }
        RunPhase::Shrine { .. } => RunAction::Bless { index: None },
        RunPhase::Won | RunPhase::Dead => unreachable!("walk: the run is over"),
    }
}

/// One index into a deck of `len` cards.
fn deck_index(rng: &mut Rng, len: usize) -> usize {
    rng.int(len as i32) as usize
}

/// One random action of the phase's kinds with random indices: a column, a fight result mostly won
/// at a random HP and sometimes lost, a take, a pack, a kept subset, an extraction, a shop or
/// campfire pick, an event option, a blessing, or leaving. Refusals (what the run cannot pay) are
/// the caller's to retry.
fn random_action(state: &RunState, content: &RunContent, rng: &mut Rng) -> RunAction {
    match &state.phase {
        RunPhase::Map => {
            let row = state.row.map_or(0, |r| r + 1);
            let width = state.map.get(row).map_or(0, Vec::len) as i32;
            RunAction::Go {
                col: rng.int(width) as usize,
            }
        }
        RunPhase::Fight => {
            if rng.int(PERCENT) < WALK_LOSS_PERCENT {
                RunAction::FightOver {
                    won: false,
                    hp: 0,
                    damage_dealt: 0,
                    healing_done: 0,
                    cards_drawn: 0,
                    turns: 0,
                }
            } else {
                RunAction::FightOver {
                    won: true,
                    hp: 1 + rng.int(run_max_hp(state, content)),
                    damage_dealt: rng.int(WALK_DAMAGE_MAX),
                    healing_done: rng.int(WALK_HEALING_MAX),
                    cards_drawn: rng.int(WALK_DRAWN_MAX),
                    turns: rng.int(WALK_TURNS_MAX),
                }
            }
        }
        // Always takes: artifacts drive deck growth (KeepMore, PackSize), which is what strains
        // the deck cap; the free way on still leaves them.
        RunPhase::Reward {
            artifact: Some(_), ..
        } => RunAction::TakeArtifact { take: true },
        RunPhase::Reward { packs, .. } => RunAction::OpenPack {
            index: rng.int(packs.len() as i32) as usize,
        },
        RunPhase::Pack { cards, keep } => RunAction::Keep {
            cards: rng.shuffle(cards).into_iter().take(*keep).collect(),
        },
        RunPhase::ExtractCard => RunAction::Extract {
            index: rng.coin().then(|| deck_index(rng, state.deck.len())),
        },
        RunPhase::ExtractArtifact => RunAction::Extract {
            index: rng.coin().then(|| rng.int(state.artifacts.len() as i32) as usize),
        },
        RunPhase::Shop { stock } => match rng.int(6) {
            0 => RunAction::Leave,
            1 => RunAction::Buy {
                index: rng.int(stock.len() as i32) as usize,
            },
            2 => RunAction::RemoveCard {
                index: deck_index(rng, state.deck.len()),
            },
            3 => RunAction::MakeRadiant {
                index: deck_index(rng, state.deck.len()),
            },
            4 => RunAction::Buff {
                index: deck_index(rng, state.deck.len()),
            },
            _ => RunAction::LevelArtifact {
                index: rng.int(state.artifacts.len() as i32) as usize,
            },
        },
        RunPhase::Campfire => match rng.int(4) {
            0 => RunAction::Heal,
            1 => RunAction::MakeRadiant {
                index: deck_index(rng, state.deck.len()),
            },
            2 => RunAction::Buff {
                index: deck_index(rng, state.deck.len()),
            },
            _ => {
                let len = state.deck.len() as i32;
                let first = rng.int(len);
                let mut second = rng.int(len - 1);
                if second >= first {
                    second += 1;
                }
                RunAction::RemoveCards {
                    indices: vec![first as usize, second as usize],
                }
            }
        },
        RunPhase::Event { id } => {
            let len = content
                .events
                .iter()
                .find(|event| &event.id == id)
                .map_or(0, |event| event.options.len()) as i32;
            RunAction::Choose {
                index: rng.int(len) as usize,
            }
        }
        RunPhase::Shrine { offers } => RunAction::Bless {
            index: rng.coin().then(|| rng.int(offers.len() as i32) as usize),
        },
        RunPhase::Won | RunPhase::Dead => unreachable!("walk: the run is over"),
    }
}

/// Every count the walk holds in bounds (§15.2: nothing deals a card the deck holds; §15.4: no
/// artifact twice; §15.7, §15.8; R2701000: the deck cap). Each failure names its field, and each
/// has a hand-built failing state below.
fn check_state(state: &RunState, content: &RunContent) -> Result<(), String> {
    if matches!(state.phase, RunPhase::Dead) {
        if state.hp != 0 {
            return Err(format!("walk: hp {} while dead", state.hp));
        }
    } else if !matches!(state.phase, RunPhase::Won) {
        let max = run_max_hp(state, content);
        if state.hp < 1 || state.hp > max {
            return Err(format!("walk: hp {} while alive, max {max}", state.hp));
        }
    }
    if state.coins < 0 {
        return Err(format!("walk: coins {}", state.coins));
    }
    if state.tickets < 0 {
        return Err(format!("walk: tickets {}", state.tickets));
    }
    if state.deck.len() > RUN_DECK_MAX {
        return Err(format!(
            "walk: deck holds {} past {RUN_DECK_MAX}",
            state.deck.len()
        ));
    }
    for (i, card) in state.deck.iter().enumerate() {
        if state.deck[..i].iter().any(|other| other.id == card.id) {
            return Err(format!("walk: deck holds {} twice", card.id));
        }
    }
    for (i, artifact) in state.artifacts.iter().enumerate() {
        if state.artifacts[..i].iter().any(|other| other.id == artifact.id) {
            return Err(format!("walk: artifact {} twice", artifact.id));
        }
        if artifact.level < 1 || artifact.level > RUN_ARTIFACT_LEVEL_MAX {
            return Err(format!(
                "walk: artifact {} at level {}",
                artifact.id, artifact.level
            ));
        }
    }
    for (i, blessing) in state.blessings.iter().enumerate() {
        if state.blessings[..i].contains(blessing) {
            return Err(format!("walk: blessing {blessing} twice"));
        }
    }
    Ok(())
}

/// Every fight's setup is one the run can start, curses and Combat artifacts included (R2922001).
/// On the catalog that means the engine takes it: `validate_setup` first (`begin_game` alone is
/// infallible), then the created game's hero carries the setup's health and armor. The
/// placeholder's pseudo cards are in no engine catalog, so it checks the enemy's deck and health.
fn check_fight(state: &RunState, content: &RunContent, engine: bool, tag: &str, n: usize) {
    let setup =
        run_fight_setup(state, content).unwrap_or_else(|| panic!("walk {tag} {n}: the fight has no setup"));
    if !engine {
        assert_eq!(
            setup.enemy.deck.len(),
            RUN_ENEMY_DECK_SIZE,
            "walk {tag} {n}: the enemy's deck"
        );
        assert!(setup.enemy.hero_health > 0, "walk {tag} {n}: the enemy's health");
        return;
    }
    engine_takes(&setup).unwrap_or_else(|e| panic!("walk {tag} {n}: the engine refuses the setup: {e}"));
    let begun = begin_game(&create_recorded_game(&fight_options(&setup)));
    let hero = &begun.state.players[PlayerId::P1].hero;
    assert_eq!(
        hero.health,
        setup.human_handicap.hero_health.unwrap(),
        "walk {tag} {n}: P1's health"
    );
    assert_eq!(
        hero.armor,
        setup.human_handicap.hero_armor.unwrap_or(0),
        "walk {tag} {n}: P1's armor"
    );
}

fn phase_name(phase: &RunPhase) -> &'static str {
    match phase {
        RunPhase::Map => "Map",
        RunPhase::Fight => "Fight",
        RunPhase::Reward { .. } => "Reward",
        RunPhase::Pack { .. } => "Pack",
        RunPhase::ExtractCard => "ExtractCard",
        RunPhase::ExtractArtifact => "ExtractArtifact",
        RunPhase::Shop { .. } => "Shop",
        RunPhase::Campfire => "Campfire",
        RunPhase::Event { .. } => "Event",
        RunPhase::Shrine { .. } => "Shrine",
        RunPhase::Won => "Won",
        RunPhase::Dead => "Dead",
    }
}

/// One whole run on `content`: random actions from a fixed seed, every accepted step checked, every
/// fight's setup checked, no phase a dead end, the walk bounded, and the accepted log folded back
/// to the same end state.
fn walk(
    content: &RunContent,
    tag: &str,
    n: usize,
    engine: bool,
    seen: &mut Vec<&'static str>,
) -> (RunState, Vec<RunAction>) {
    let mut rng = Rng::new(&format!("walk-2979-{tag}-{n}"), 0);
    let (setup, unlocks) = walk_setup(tag, n, content, &mut rng);
    let mut state =
        run_create(&setup, &unlocks, content).unwrap_or_else(|e| panic!("walk {tag} {n}: no run: {e}"));
    let mut log = Vec::new();
    while !matches!(state.phase, RunPhase::Won | RunPhase::Dead) {
        if log.len() >= WALK_MAX_STEPS {
            panic!("walk {tag} {n}: past {WALK_MAX_STEPS} steps at {:?}", state.phase);
        }
        let name = phase_name(&state.phase);
        if !seen.contains(&name) {
            seen.push(name);
        }
        let free = free_action(&state, content);
        run_reduce(&state, content, &free).unwrap_or_else(|e| {
            panic!("walk {tag} {n}: dead end at {name}: {e}");
        });
        check_state(&state, content).unwrap_or_else(|e| panic!("walk {tag} {n} at {name}: {e}"));
        if matches!(state.phase, RunPhase::Fight) {
            check_fight(&state, content, engine, tag, n);
        }
        let mut action = free;
        if rng.int(PERCENT) >= WALK_WAY_ON_PERCENT {
            for _ in 0..WALK_TRIES {
                let tried = random_action(&state, content, &mut rng);
                if run_reduce(&state, content, &tried).is_ok() {
                    action = tried;
                    break;
                }
            }
        }
        state = run_reduce(&state, content, &action).unwrap_or_else(|e| {
            panic!("walk {tag} {n}: the picked action fails at {name}: {e}");
        });
        check_state(&state, content).unwrap_or_else(|e| panic!("walk {tag} {n} after {name}: {e}"));
        log.push(action);
    }
    let folded = run_fold(
        &RunFile {
            setup,
            unlocks,
            log: log.clone(),
        },
        content,
    )
    .unwrap_or_else(|e| panic!("walk {tag} {n}: no fold: {e}"));
    assert_eq!(folded, state, "walk {tag} {n}: the log folds back");
    (state, log)
}

#[test]
fn walks_play_whole_runs_in_bounds_start_every_fight_and_replay() {
    for (tag, engine) in [("placeholder", false), ("catalog", true)] {
        let content = if tag == "placeholder" {
            run_placeholder_content()
        } else {
            catalog_content()
        };
        let mut seen: Vec<&'static str> = Vec::new();
        for n in 0..WALK_SEEDS {
            let (end, _) = walk(&content, tag, n, engine, &mut seen);
            if end.phase == RunPhase::Won {
                assert!(
                    (RUN_ROUTE_FIGHTS_MIN..=RUN_ROUTE_FIGHTS_MAX).contains(&end.stats.fights_won),
                    "walk {tag} {n}: R3583001: {} fights on the route",
                    end.stats.fights_won
                );
            }
        }
        for phase in [
            "Map",
            "Fight",
            "Reward",
            "Pack",
            "ExtractCard",
            "ExtractArtifact",
            "Shop",
            "Campfire",
            "Event",
            "Shrine",
        ] {
            assert!(seen.contains(&phase), "{tag}: no walk visited {phase}");
        }
    }
}

/// A started placeholder run to break one field of.
fn started() -> (RunState, RunContent) {
    let content = run_placeholder_content();
    let setup = RunSetup {
        seed: run_daily_seed("2026-11-01", 1),
        starter: content.starter_access[..RUN_STARTER_SIZE].to_vec(),
        artifacts: vec!["temp-greedy".into()],
        curses: Vec::new(),
    };
    let state = run_create(&setup, &RunUnlocks::default(), &content).unwrap();
    (state, content)
}

#[test]
fn walk_hp_zero_while_alive_fails() {
    let (mut state, content) = started();
    state.hp = 0;
    let err = check_state(&state, &content).unwrap_err();
    assert!(err.contains("hp"), "{err}");
}

#[test]
fn walk_hp_past_the_max_fails() {
    let (mut state, content) = started();
    state.hp = run_max_hp(&state, &content) + 1;
    let err = check_state(&state, &content).unwrap_err();
    assert!(err.contains("hp"), "{err}");
}

#[test]
fn walk_negative_coins_fail() {
    let (mut state, content) = started();
    state.coins = -1;
    let err = check_state(&state, &content).unwrap_err();
    assert!(err.contains("coins"), "{err}");
}

#[test]
fn walk_negative_tickets_fail() {
    let (mut state, content) = started();
    state.tickets = -1;
    let err = check_state(&state, &content).unwrap_err();
    assert!(err.contains("tickets"), "{err}");
}

#[test]
fn walk_a_deck_past_the_max_fails() {
    let (mut state, content) = started();
    state.deck = (0..RUN_DECK_MAX + 1)
        .map(|i| RunCard {
            id: format!("walk-card-{i}"),
            radiant: false,
            buffs: 0,
        })
        .collect();
    let err = check_state(&state, &content).unwrap_err();
    assert!(err.contains("deck"), "{err}");
}

#[test]
fn walk_a_card_twice_fails() {
    let (mut state, content) = started();
    state.deck.push(state.deck[0].clone());
    let err = check_state(&state, &content).unwrap_err();
    assert!(err.contains("twice"), "{err}");
}

#[test]
fn walk_an_artifact_twice_fails() {
    let (mut state, content) = started();
    state.artifacts.push(state.artifacts[0].clone());
    let err = check_state(&state, &content).unwrap_err();
    assert!(err.contains("twice"), "{err}");
}

#[test]
fn walk_an_artifact_past_its_highest_level_fails() {
    let (mut state, content) = started();
    state.artifacts[0].level = RUN_ARTIFACT_LEVEL_MAX + 1;
    let err = check_state(&state, &content).unwrap_err();
    assert!(err.contains("level"), "{err}");
}

#[test]
fn walk_an_artifact_below_level_one_fails() {
    let (mut state, content) = started();
    state.artifacts[0].level = 0;
    let err = check_state(&state, &content).unwrap_err();
    assert!(err.contains("level"), "{err}");
}

#[test]
fn walk_a_blessing_twice_fails() {
    let (mut state, content) = started();
    state.blessings = vec!["temp-blessing-vitality".into(), "temp-blessing-vitality".into()];
    let err = check_state(&state, &content).unwrap_err();
    assert!(err.contains("blessing"), "{err}");
}

#[test]
fn walk_hp_while_dead_fails() {
    let (mut state, content) = started();
    state.phase = RunPhase::Dead;
    state.hp = 5;
    let err = check_state(&state, &content).unwrap_err();
    assert!(err.contains("dead"), "{err}");
}
