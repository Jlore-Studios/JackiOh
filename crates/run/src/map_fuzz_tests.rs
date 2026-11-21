//! Random-policy map walks with illegal-action probes (§15.5, §15.7, §15.8): whole runs from
//! fixed seeds that also fire one illegal action per step, a misclick or a future screen's bad
//! call. Fights are stubbed as synthetic `FightOver` reports written directly, so the test needs
//! no AI search and stays deterministic from the run seed and cursor. Every illegal probe must
//! return `Err`; every accepted walk step must keep the run invariants and the saved file must
//! still `run_fold` back to the live run.

use super::*;
use crate::config::{
    MAP_FUZZ_DAILY_SEEDS, MAP_FUZZ_SEEDS, RUN_STARTER_CARDS, RUN_STARTER_SIZE, WALK_MAX_STEPS, WALK_TRIES,
    WALK_WAY_ON_PERCENT,
};
use crate::fight_tests::catalog_content;
use crate::invariants::check_run_invariants;
use crate::walk_tests::{free_action, random_action};
use jackioh_engine::rng::Rng;

/// A fuzz walk's start, as `walk_tests`' `walk_setup`: the placeholder's first starting cards
/// with one Greedy, or the catalog's first starting cards with none, under a random subset of the
/// content's curses. Draws come from the walk's own `Rng`, never the run's `rng_cursor` (rule 4).
fn fuzz_setup(tag: &str, n: usize, content: &RunContent, rng: &mut Rng) -> (RunSetup, RunUnlocks) {
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
        seed: run_seed(&format!("mapfuzz-4247-{tag}-{n}")),
        starter,
        artifacts,
        curses,
    };
    (setup, RunUnlocks::default())
}

/// One action the phase must refuse: a step no path reaches, a leave for no shop, a pack or a
/// take out of turn, a kept card from outside the pack, an extraction past the end (or of the
/// last card), a buy, removal or leveling past the end, a campfire removal of the wrong count,
/// an event option past the end, or a blessing past the end of the offers.
fn illegal_action(state: &RunState, content: &RunContent, rng: &mut Rng) -> RunAction {
    match &state.phase {
        RunPhase::Map => {
            let row = state.row.map_or(0, |r| r + 1);
            let width = state.map.get(row).map_or(0, Vec::len);
            let next = run_next_cols(state);
            let col = (0..width).find(|col| !next.contains(col)).unwrap_or(width);
            RunAction::Go { col }
        }
        RunPhase::Fight => RunAction::Leave,
        RunPhase::Reward {
            artifact: Some(_), ..
        } => RunAction::OpenPack { index: 0 },
        RunPhase::Reward { .. } => RunAction::TakeArtifact { take: true },
        RunPhase::Pack { cards, .. } => {
            let outsider = content
                .pool
                .iter()
                .map(|card| card.id.clone())
                .find(|id| !cards.contains(id))
                .expect("mapfuzz: the pool holds a card outside the pack");
            RunAction::Keep {
                cards: vec![outsider],
            }
        }
        RunPhase::ExtractCard => RunAction::Extract {
            index: Some(if state.deck.len() == 1 {
                0
            } else {
                state.deck.len()
            }),
        },
        RunPhase::ExtractArtifact => RunAction::Extract {
            index: Some(state.artifacts.len()),
        },
        RunPhase::Shop { stock } => match rng.int(3) {
            0 => RunAction::Buy { index: stock.len() },
            1 => RunAction::RemoveCard {
                index: state.deck.len(),
            },
            _ => RunAction::LevelArtifact {
                index: state.artifacts.len(),
            },
        },
        RunPhase::Campfire => RunAction::RemoveCards { indices: vec![0] },
        RunPhase::Event { id } => {
            let len = content
                .events
                .iter()
                .find(|event| &event.id == id)
                .map_or(0, |event| event.options.len());
            RunAction::Choose { index: len }
        }
        RunPhase::Shrine { offers } => RunAction::Bless {
            index: Some(offers.len()),
        },
        RunPhase::Won | RunPhase::Dead => unreachable!("mapfuzz: the run is over"),
    }
}

/// One whole run on `content`: at each step fire one illegal probe (each must be refused),
/// then take the walk's way on (each acceptance keeps the invariants and the log so far folds
/// back to the live state). No phase is a dead end; the walk is bounded.
fn drive(content: &RunContent, tag: &str, n: usize) {
    let mut rng = Rng::new(&format!("mapfuzz-4247-{tag}-{n}"), 0);
    let (setup, unlocks) = fuzz_setup(tag, n, content, &mut rng);
    let mut state =
        run_create(&setup, &unlocks, content).unwrap_or_else(|e| panic!("mapfuzz {tag} {n}: no run: {e}"));
    let mut log = Vec::new();
    while !matches!(state.phase, RunPhase::Won | RunPhase::Dead) {
        if log.len() >= WALK_MAX_STEPS {
            panic!(
                "mapfuzz {tag} {n}: past {WALK_MAX_STEPS} steps at {:?}",
                state.phase
            );
        }
        let probe = illegal_action(&state, content, &mut rng);
        run_reduce(&state, content, &probe).expect_err(&format!(
            "mapfuzz {tag} {n}: illegal {probe:?} accepted at {:?}",
            state.phase
        ));
        let free = free_action(&state, content);
        run_reduce(&state, content, &free)
            .unwrap_or_else(|e| panic!("mapfuzz {tag} {n}: dead end at {:?}: {e}", state.phase));
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
            panic!(
                "mapfuzz {tag} {n}: the picked action fails at {:?}: {e}",
                state.phase
            )
        });
        check_run_invariants(&state, content)
            .unwrap_or_else(|e| panic!("mapfuzz {tag} {n} after {:?}: {e}", state.phase));
        log.push(action);
        let folded = run_fold(
            &RunFile {
                setup: setup.clone(),
                unlocks: unlocks.clone(),
                log: log.clone(),
            },
            content,
        )
        .unwrap_or_else(|e| panic!("mapfuzz {tag} {n}: no fold: {e}"));
        assert_eq!(folded, state, "mapfuzz {tag} {n}: the log folds back");
    }
}

/// The catalog seeds the fast test walks: fixed seeds whose keeps fill the deck to the cap,
/// so an uncapped keep overfills it and turns the test red. Found by scanning 0..300 with the
/// cap commented out (about one seed in fourteen overfills); all ten pass on the capped code,
/// like `map_tests`' pinned seeds for rare branches.
const MAP_FUZZ_CATALOG_SEEDS: [usize; MAP_FUZZ_SEEDS] = [19, 50, 65, 86, 100, 114, 119, 125, 149, 153];

#[test]
fn map_fuzz_random_policy_with_illegal_probes_stays_in_bounds_and_replays() {
    let placeholder = run_placeholder_content();
    for n in 0..MAP_FUZZ_SEEDS {
        drive(&placeholder, "placeholder", n);
    }
    let catalog = catalog_content();
    for n in MAP_FUZZ_CATALOG_SEEDS {
        drive(&catalog, "catalog", n);
    }
}

#[test]
#[ignore = "daily: three hundred seeds per content; the fast test holds the bounds per pull request"]
fn map_fuzz_daily_wide_sweep() {
    for (tag, content) in [
        ("placeholder", run_placeholder_content()),
        ("catalog", catalog_content()),
    ] {
        for n in 0..MAP_FUZZ_DAILY_SEEDS {
            drive(&content, tag, n);
        }
    }
}
