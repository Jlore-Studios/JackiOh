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
//! makes in a main phase, about 70 a game. `--every reply` writes, at each end of a seat's turn, the
//! position the search would score that line at: the opponent's reply as the search simulates it,
//! played out from the true state (R1006006). The file's first line is a header naming the features
//! (`PositionsHeader`), so a reader takes the feature length from the data.
//!
//! `--policy` (#1171) adds to each row of a decision the policy prior's label (`policy_label`): the keys
//! of every move the AI considers there and which one was played, read from `redact` alone (R1171000),
//! and the header names `POLICY_KEYS_VERSION`.

use std::io::{BufRead, BufReader, BufWriter, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use jackioh_ai::{
    AI_EVAL, FEATURES_VERSION, NextSwing, POLICY_KEYS_VERSION, PolicyLabel, create_node_counter, dense_names,
    evaluate, features_of, hidden_card_ids, policy_label, simulate_reply,
};
use jackioh_engine::{
    ActionBody, CreateGameOptions, GameResult, GameState, Phase, PlayerId, begin_game, catalog_for,
    create_game, hash_state, reduce, seat_to_act, with_shipped,
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
    /// R1006006: at each end of a seat's turn, the position the search would score its line at: the
    /// opponent's reply as the search simulates it (`simulate_reply`), played out from the true state.
    Reply,
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
    /// #1171: label each row with the policy prior's candidates and the one played (R1171000).
    #[arg(long)]
    pub policy: bool,
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
    /// #1171: the policy prior's key layout, when the rows carry its labels (`--policy`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_keys_version: Option<u32>,
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
    /// #1171: the decision's policy label (`--policy`, R1171000).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<PolicyLabel>,
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
        Every::Reply => None,
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
        policy: None,
    }
}

/// The node budget the reply a `--every reply` row plays out may spend: more than any reply takes, so
/// every reply is played to its end as the search plays it when its budget allows.
const REPLY_NODES: usize = 2000;

/// R1006006: the row for the seat whose turn just ended at `end`: the opponent's reply as the search
/// simulates it (`simulate_reply`, the hidden cards those of the true state's opponent), then the seat's
/// features and the hand eval there with the reply's swing rule, as `reply_score` scores a line. `None`
/// when the reply ends the game or runs out of nodes.
fn reply_row(
    end: &GameState,
    seat: PlayerId,
    step: usize,
    game: &SelfplayGame,
) -> Result<Option<PositionRow>, Dropped> {
    let Some(outcome) = outcome_for(game.result, seat) else {
        return Err(Dropped::NoResult);
    };
    let counter = create_node_counter(REPLY_NODES, None);
    let Some(after) = simulate_reply(end, seat, &counter, &hidden_card_ids(end, seat)) else {
        return Ok(None);
    };
    if after.result.is_some() {
        return Ok(None);
    }
    Ok(Some(row_at(&after, seat, step, game, outcome)))
}

/// R1006003: `game` folded under its catalog, its rows if it reaches its hash. With `policy`, a row
/// whose seat plays the next action carries that decision's label (R1171000).
pub(crate) fn fold_game(
    game: &SelfplayGame,
    every: Every,
    build_catalog: &str,
    policy: bool,
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
                    let mut row = row_at(&state, seat, step, game, outcome);
                    if policy && action.player_id == seat {
                        row.policy = Some(policy_label(&state, seat, &action.body));
                    }
                    rows.push(row);
                }
                let ended_by = (every == Every::Reply
                    && action.body == ActionBody::EndTurn
                    && action.player_id == state.active
                    && state.phase == Phase::Main
                    && state.result.is_none())
                .then_some(state.active);
                let result = reduce(&state, action);
                if result.error.is_some() {
                    return Err(Dropped::NotReplayed);
                }
                state = result.state;
                if let Some(seat) = ended_by
                    && let Some(row) = reply_row(&state, seat, step + 1, game)?
                {
                    rows.push(row);
                }
            }
            if hash_state(&state) != game.hash || state.result != game.result {
                return Err(Dropped::NotReplayed);
            }
            Ok(rows)
        })
    }));
    folded.unwrap_or(Err(Dropped::NotReplayed))
}

pub(crate) fn header(every: Every, build_catalog: &str, policy: bool) -> PositionsHeader {
    PositionsHeader {
        features_version: FEATURES_VERSION,
        every,
        dense_names: dense_names(),
        build_catalog: build_catalog.to_string(),
        policy_keys_version: policy.then_some(POLICY_KEYS_VERSION),
    }
}

/// Folds every game of `logs` into `out`, a header first, the rows in game order.
pub(crate) fn write_positions(
    logs: &[PathBuf],
    out: &Path,
    every: Every,
    policy: bool,
) -> anyhow::Result<Counts> {
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
    serde_json::to_writer(&mut writer, &header(every, build_catalog, policy))?;
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
                .map(|game| fold_game(game, every, build_catalog, policy))
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
    if args.policy && args.every == Every::Reply {
        bail!("positions: --policy labels decisions: use --every decision or turn");
    }
    let counts = write_positions(&args.logs, &args.out, args.every, args.policy)?;
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
    use jackioh_engine::find_instance;

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
            ai_generation: None,
        };
        (game, live)
    }

    /// R1006003, R1006004: the rows of a game's log are exactly the features computed on the true
    /// game, at the same steps, as it was played.
    #[test]
    fn r1006004_positions_over_a_log_are_the_features_of_the_true_game_as_it_was_played() {
        let (game, live) = played(1);
        assert!(game.result.is_some(), "the test game finishes");
        let rows =
            fold_game(&game, Every::Decision, jackioh_cards::catalog_version(), false).expect("it replays");
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
        let turns =
            fold_game(&game, Every::Turn, jackioh_cards::catalog_version(), false).expect("it replays");
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
        assert_eq!(
            fold_game(&wrong, Every::Turn, build, false),
            Err(Dropped::NotReplayed)
        );
        let mut unfinished = game.clone();
        unfinished.result = None;
        assert_eq!(
            fold_game(&unfinished, Every::Turn, build, false),
            Err(Dropped::NoResult)
        );
        let mut old = game;
        old.catalog = "0.0.0-never".to_string();
        assert_eq!(
            fold_game(&old, Every::Turn, build, false),
            Err(Dropped::EarlierPatch)
        );
    }

    /// R1006006: `--every reply` writes one row per end of a seat's turn whose reply leaves the game
    /// going: the features and hand eval after the reply as the search simulates it, which is the
    /// seat's next turn, the same rows on every fold.
    #[test]
    fn r1006006_reply_rows_are_the_search_s_simulated_replies_one_per_turn_end() {
        let (game, _) = played(3);
        let build = jackioh_cards::catalog_version();
        let rows = fold_game(&game, Every::Reply, build, false).expect("it replays");
        let ends = game
            .log
            .iter()
            .filter(|action| action.body == ActionBody::EndTurn)
            .count();
        assert!(
            !rows.is_empty() && rows.len() <= ends,
            "{} rows for {ends} turn ends",
            rows.len()
        );
        assert_eq!(rows, fold_game(&game, Every::Reply, build, false).expect("again"));
        let turns = fold_game(&game, Every::Turn, build, false).expect("it replays");
        // A reply row is the seat's position after the reply, a turn after the one it ended.
        for row in &rows {
            assert!(turns.iter().any(|turn| turn.turn > 0) && row.turn > 0);
        }
    }

    /// R1171000: `--policy` labels every decision row with the label the live game gives there, the
    /// same on every fold, its keys naming no instance and no hero; without it the rows are the same
    /// less their labels, and only a labelled file's header names the key layout.
    #[test]
    fn r1171000_policy_labels_over_a_log_are_deterministic_and_name_no_instance() {
        let (game, live) = played(4);
        let build = jackioh_cards::catalog_version();
        let rows = fold_game(&game, Every::Decision, build, true).expect("it replays");
        assert!(rows.len() > 10, "{} rows", rows.len());
        for row in &rows {
            let label = row
                .policy
                .as_ref()
                .unwrap_or_else(|| panic!("step {} has no label", row.step));
            assert!(
                label.chosen.is_some(),
                "step {}: the AI played one of its candidates",
                row.step
            );
            let (state, _) = live
                .iter()
                .find(|(_, step)| *step == row.step)
                .expect("a live state at every row's step");
            assert_eq!(
                *label,
                policy_label(state, row.seat, &game.log[row.step].body),
                "step {}",
                row.step
            );
            for key in label.candidates.iter().flatten() {
                assert!(
                    !key.split([':', '.'])
                        .any(|part| part.starts_with("hero-") || find_instance(state, part).is_some()),
                    "step {}: {key}",
                    row.step
                );
            }
        }
        assert_eq!(
            rows,
            fold_game(&game, Every::Decision, build, true).expect("again")
        );
        let plain = fold_game(&game, Every::Decision, build, false).expect("it replays");
        let cleared: Vec<PositionRow> = rows
            .iter()
            .cloned()
            .map(|row| PositionRow { policy: None, ..row })
            .collect();
        assert_eq!(plain, cleared);

        let labelled = serde_json::to_value(header(Every::Decision, build, true)).unwrap();
        assert_eq!(labelled["policyKeysVersion"], POLICY_KEYS_VERSION);
        let unlabelled = serde_json::to_value(header(Every::Decision, build, false)).unwrap();
        assert!(unlabelled.get("policyKeysVersion").is_none(), "{unlabelled}");
    }

    #[test]
    fn the_outcome_is_one_a_half_or_nothing() {
        assert_eq!(outcome_for(None, PlayerId::P1), None);
    }
}
