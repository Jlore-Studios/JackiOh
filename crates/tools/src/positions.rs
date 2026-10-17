//! `cargo jackioh positions`: self-play logs folded into the learned evaluation's training rows (#466,
//! #1006; `docs/learned-eval.md` D3, D4). Each game of a `selfplay` file is folded from its seed under
//! the catalog it was played on (`replay::catalog_for`, R767000) and its final hash checked: a game
//! whose log no longer reaches its hash (an engine change since) is dropped and counted, never
//! guessed at, and so is one played on a catalog this build does not carry, or one without a result
//! (R1006003).
//!
//! At each position it writes one JSON line (`PositionRow`): the game's seed, the seat, the turn, the
//! step, `FEATURES_VERSION`, the seat's features (`features_of`, which reads only `redact`, R185), the
//! hand eval's score for the seat and the game's outcome for that seat (1 a win, ½ a draw, 0 a loss).
//! The hand eval is `evaluate` over the true state, with the reply's swing rule (`NextSwing::Seat`
//! when the seat is the one to act, `Enemy` otherwise): the score the search gives the leaves the
//! learned evaluation is to replace, which it computes on a determinized world, one the seat cannot
//! tell from the true state. The true state is such a world, so the row's score is one the search
//! could have given there (R1006004). The features never read it: they read only `redact`.
//!
//! `--every turn` (the default) writes one position per seat per turn: the seat's first decision of its
//! own turn in the main phase, with no prompt open, about 16 a game. That is the state the search's
//! finalists are scored at after the opponent's reply. `--every decision` writes every decision a seat
//! makes in a main phase, about 70 a game. The file's first line is a header naming the features
//! (`PositionsHeader`), so a reader takes the feature length from the data.

use std::io::{BufRead, BufReader, BufWriter, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use jackioh_ai::{AI_EVAL, FEATURES_VERSION, NextSwing, dense_names, evaluate, features_of};
use jackioh_engine::{
    CreateGameOptions, GameResult, GameState, Phase, PlayerId, begin_game, catalog_for, create_game,
    hash_state, reduce, seat_to_act, with_shipped,
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::selfplay::SelfplayGame;

/// Games read and folded at a time: a chunk's rows are written before the next is read.
const CHUNK_GAMES: usize = 2000;

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Every {
    /// One position per seat per turn: its first main-phase decision of its own turn.
    Turn,
    /// Every decision a seat makes in a main phase.
    Decision,
}

#[derive(clap::Args, Debug)]
pub struct Args {
    /// `selfplay` files (one game per line).
    #[arg(required = true)]
    pub logs: Vec<PathBuf>,
    /// The positions file to write (JSON lines, a header first).
    #[arg(long)]
    pub out: PathBuf,
    /// Which positions to write.
    #[arg(long, value_enum, default_value_t = Every::Turn)]
    pub every: Every,
}

/// The positions file's first line.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PositionsHeader {
    pub features_version: u32,
    pub every: Every,
    pub dense_names: Vec<String>,
    /// The catalog this build compiles in; each row names the one its game was played on.
    pub build_catalog: String,
}

/// One training row.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PositionRow {
    pub game: String,
    pub seat: PlayerId,
    pub turn: i32,
    /// The number of actions folded before this position.
    pub step: usize,
    pub features_version: u32,
    pub catalog: String,
    pub dense: Vec<i32>,
    pub sparse: Vec<(String, i32)>,
    /// The hand eval's score for the seat (R1006004).
    pub hand: f64,
    /// The game's outcome for the seat: 1, 0.5 or 0.
    pub outcome: f64,
}

/// Why a game left no rows (R1006003).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Dropped {
    NoResult,
    NotReplayed,
    EarlierPatch,
}

/// A run's counts, printed at the end.
#[derive(Serialize, Default, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Counts {
    pub games: usize,
    pub replayed: usize,
    pub no_result: usize,
    pub not_replayed: usize,
    pub earlier_patch: usize,
    pub unreadable: usize,
    pub positions: usize,
}

impl Counts {
    fn add(&mut self, folded: &Result<Vec<PositionRow>, Dropped>) {
        self.games += 1;
        match folded {
            Ok(rows) => {
                self.replayed += 1;
                self.positions += rows.len();
            }
            Err(Dropped::NoResult) => self.no_result += 1,
            Err(Dropped::NotReplayed) => self.not_replayed += 1,
            Err(Dropped::EarlierPatch) => self.earlier_patch += 1,
        }
    }
}

/// The outcome of `result` for `seat`: 1, 0.5 for a draw, 0; `None` without a result.
fn outcome_for(result: Option<GameResult>, seat: PlayerId) -> Option<f64> {
    let result = result?;
    Some(match result.winner.player() {
        Some(winner) if winner == seat => 1.0,
        Some(_) => 0.0,
        None => 0.5,
    })
}

/// Whether `state` is a position to write, and for which seat. `turned` holds the `(turn, seat)`
/// pairs `Every::Turn` has written already.
fn position_seat(state: &GameState, every: Every, turned: &mut Vec<(i32, PlayerId)>) -> Option<PlayerId> {
    let seat = seat_to_act(state)?;
    if state.result.is_some() || state.phase != Phase::Main {
        return None;
    }
    match every {
        Every::Decision => Some(seat),
        Every::Turn => {
            if seat != state.active || state.pending.is_some() || turned.contains(&(state.turn, seat)) {
                return None;
            }
            turned.push((state.turn, seat));
            Some(seat)
        }
    }
}

/// R1006004: the row for `seat` at `state`: the features over `redact(state, seat)` and the hand
/// eval over the true state, a world the seat cannot tell from its determinizations.
pub(crate) fn row_at(
    state: &GameState,
    seat: PlayerId,
    step: usize,
    game: &SelfplayGame,
    outcome: f64,
) -> PositionRow {
    let next = if state.active == seat {
        NextSwing::Seat
    } else {
        NextSwing::Enemy
    };
    let found = features_of(state, seat);
    PositionRow {
        game: game.seed.clone(),
        seat,
        turn: state.turn,
        step,
        features_version: FEATURES_VERSION,
        catalog: game.catalog.clone(),
        dense: found.dense,
        sparse: found.sparse,
        hand: evaluate(state, seat, next, &AI_EVAL),
        outcome,
    }
}

/// R1006003: `game` folded under its catalog, its rows if it reaches its hash.
pub(crate) fn fold_game(
    game: &SelfplayGame,
    every: Every,
    build_catalog: &str,
) -> Result<Vec<PositionRow>, Dropped> {
    let shipped = catalog_for(&game.catalog, build_catalog).map_err(|_| Dropped::EarlierPatch)?;
    if game.result.is_none() {
        return Err(Dropped::NoResult);
    }
    let folded = catch_unwind(AssertUnwindSafe(|| {
        with_shipped(shipped, || {
            let options = CreateGameOptions {
                seed: game.seed.clone(),
                decks: game.decks.clone(),
                ..Default::default()
            };
            let mut state = begin_game(&create_game(&options)).state;
            let mut rows: Vec<PositionRow> = Vec::new();
            let mut turned: Vec<(i32, PlayerId)> = Vec::new();
            for (step, action) in game.log.iter().enumerate() {
                if let Some(seat) = position_seat(&state, every, &mut turned) {
                    let Some(outcome) = outcome_for(game.result, seat) else {
                        return Err(Dropped::NoResult);
                    };
                    rows.push(row_at(&state, seat, step, game, outcome));
                }
                let result = reduce(&state, action);
                if result.error.is_some() {
                    return Err(Dropped::NotReplayed);
                }
                state = result.state;
            }
            if hash_state(&state) != game.hash || state.result != game.result {
                return Err(Dropped::NotReplayed);
            }
            Ok(rows)
        })
    }));
    folded.unwrap_or(Err(Dropped::NotReplayed))
}

pub(crate) fn header(every: Every, build_catalog: &str) -> PositionsHeader {
    PositionsHeader {
        features_version: FEATURES_VERSION,
        every,
        dense_names: dense_names(),
        build_catalog: build_catalog.to_string(),
    }
}

/// Folds every game of `logs` into `out`, a header first, the rows in game order.
pub(crate) fn write_positions(logs: &[PathBuf], out: &Path, every: Every) -> anyhow::Result<Counts> {
    jackioh_cards::register_all();
    let build_catalog = jackioh_cards::catalog_version();
    if let Some(dir) = out.parent()
        && !dir.as_os_str().is_empty()
    {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("positions: cannot create {}", dir.display()))?;
    }
    let file =
        std::fs::File::create(out).with_context(|| format!("positions: cannot create {}", out.display()))?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer(&mut writer, &header(every, build_catalog))?;
    writer.write_all(b"\n")?;

    let mut counts = Counts::default();
    for path in logs {
        let file = std::fs::File::open(path)
            .with_context(|| format!("positions: cannot open {}", path.display()))?;
        let mut lines = BufReader::new(file).lines();
        loop {
            let mut chunk: Vec<SelfplayGame> = Vec::with_capacity(CHUNK_GAMES);
            for line in lines.by_ref().take(CHUNK_GAMES) {
                let line = line.with_context(|| format!("positions: cannot read {}", path.display()))?;
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<SelfplayGame>(&line) {
                    Ok(game) => chunk.push(game),
                    Err(_) => {
                        counts.games += 1;
                        counts.unreadable += 1;
                    }
                }
            }
            if chunk.is_empty() {
                break;
            }
            let folded: Vec<Result<Vec<PositionRow>, Dropped>> = chunk
                .par_iter()
                .map(|game| fold_game(game, every, build_catalog))
                .collect();
            for result in &folded {
                counts.add(result);
                if let Ok(rows) = result {
                    for row in rows {
                        serde_json::to_writer(&mut writer, row)?;
                        writer.write_all(b"\n")?;
                    }
                }
            }
        }
    }
    writer.flush()?;
    Ok(counts)
}

pub fn run(args: Args) -> anyhow::Result<()> {
    if args.logs.is_empty() {
        bail!("positions: name at least one selfplay file");
    }
    let counts = write_positions(&args.logs, &args.out, args.every)?;
    println!(
        "{}",
        serde_json::json!({ "out": args.out.display().to_string(), "every": args.every, "counts": counts })
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_ai::{AI_GATE_BUDGET, MatchHooks, dev_game_config, play_match};

    fn played(n: i32) -> (SelfplayGame, Vec<(GameState, usize)>) {
        jackioh_cards::register_all();
        let config = dev_game_config(n, "positions-test", Some(AI_GATE_BUDGET));
        // The true states as the game was played, each with the number of actions before it.
        let mut live: Vec<(GameState, usize)> = Vec::new();
        let mut seen = 0usize;
        let record = {
            let mut hooks = MatchHooks {
                after_action: Some(Box::new(
                    |before: &GameState, _after: &GameState, _seat, _body| {
                        live.push((before.clone(), seen));
                        seen += 1;
                    },
                )),
                ..MatchHooks::default()
            };
            play_match(&config, &mut hooks)
        };
        let game = SelfplayGame {
            seed: config.seed,
            decks: config.decks,
            log: record.log,
            hash: record.hash,
            result: record.result,
            turns: record.turns,
            catalog: jackioh_cards::catalog_version().to_string(),
            commit: "test".to_string(),
        };
        (game, live)
    }

    /// R1006003, R1006004: the rows of a game's log are exactly the features computed on the true
    /// game, at the same steps, as it was played.
    #[test]
    fn r1006004_positions_over_a_log_are_the_features_of_the_true_game_as_it_was_played() {
        let (game, live) = played(1);
        assert!(game.result.is_some(), "the test game finishes");
        let rows = fold_game(&game, Every::Decision, jackioh_cards::catalog_version()).expect("it replays");
        assert!(rows.len() > 10, "{} rows", rows.len());
        for row in &rows {
            let (state, _) = live
                .iter()
                .find(|(_, step)| *step == row.step)
                .expect("a live state at every row's step");
            let truth = features_of(state, row.seat);
            assert_eq!(row.dense, truth.dense, "step {}", row.step);
            assert_eq!(row.sparse, truth.sparse, "step {}", row.step);
        }
        let turns = fold_game(&game, Every::Turn, jackioh_cards::catalog_version()).expect("it replays");
        assert!(!turns.is_empty() && turns.len() < rows.len());
        let mut keys: Vec<(i32, PlayerId)> = turns.iter().map(|row| (row.turn, row.seat)).collect();
        keys.dedup();
        assert_eq!(keys.len(), turns.len(), "one row per seat per turn");
    }

    /// R1006003: a log that no longer reaches its hash is dropped, as is a game without a result or on
    /// a catalog this build does not carry.
    #[test]
    fn r1006003_a_game_that_does_not_replay_has_no_result_or_an_unknown_catalog_is_dropped() {
        let (game, _) = played(2);
        let build = jackioh_cards::catalog_version();
        let mut wrong = game.clone();
        wrong.hash = "00000000".to_string();
        assert_eq!(fold_game(&wrong, Every::Turn, build), Err(Dropped::NotReplayed));
        let mut unfinished = game.clone();
        unfinished.result = None;
        assert_eq!(fold_game(&unfinished, Every::Turn, build), Err(Dropped::NoResult));
        let mut old = game;
        old.catalog = "0.0.0-never".to_string();
        assert_eq!(fold_game(&old, Every::Turn, build), Err(Dropped::EarlierPatch));
    }

    #[test]
    fn the_outcome_is_one_a_half_or_nothing() {
        assert_eq!(outcome_for(None, PlayerId::P1), None);
    }
}
