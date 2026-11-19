//! `cargo jackioh positions`: self-play logs folded into the learn lane's training rows (#466,
//! #1006; `docs/learned-eval.md` D3, D4). Each game is folded from its seed under the catalog it was
//! played on (`replay::catalog_for`, R767000) and its final hash checked: one that no longer reaches its
//! hash, one on a catalog this build does not carry, one without a result (R1006003) or one its run
//! marked with faults (R1489000) is dropped and counted, never guessed at.
//!
//! Each position is one JSON line (`PositionRow`): seed, seat, turn, step, `FEATURES_VERSION`, the
//! seat's features (read only from `redact`, R185), the hand eval's score over the true state, a world
//! the seat cannot tell from its determinizations (R1006004), priced as the decision at its root prices
//! it (R4070000), and the seat's outcome (1, ½ or 0).
//! `--every turn` (the default) writes the seat's first main-phase decision of its turn, `decision`
//! every main-phase decision, `reply` the position after the opponent's reply as the search simulates
//! it (R1006006). A header line (`PositionsHeader`) names the features. The learned evaluation these rows
//! fed is retired (R4159000); the command stays while the learn lane's workflow runs it (CLAUDE.md rule 13).

use std::io::{BufRead, BufReader, BufWriter, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use jackioh_ai::{
    AI_EVAL, FEATURES_VERSION, NextSwing, Prices, create_node_counter, decision_prices_at, dense_names,
    evaluate_priced, features_of, hidden_card_ids, simulate_reply,
};
use jackioh_engine::{
    ActionBody, CreateGameOptions, GameResult, GameState, Phase, PlayerId, begin_game, catalog_for,
    create_recorded_game, hash_state, reduce, seat_to_act, with_shipped,
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
    /// The hand eval's score for the seat (R1006004), priced as the decision at the row's root prices it
    /// (R4070000).
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
    /// R1489000: a seat had an action refused, fell back or caught a panic in its search
    /// (`SelfplayGame::faults`).
    Unclean,
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
    pub unclean: usize,
    pub unreadable: usize,
    pub positions: usize,
}

impl Counts {
    /// `folded`'s game counted.
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
            Err(Dropped::Unclean) => self.unclean += 1,
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
/// eval over the true state, a world the seat cannot tell from its determinizations. R4070000: priced
/// with `prices`, the prices of the decision at the row's root.
pub(crate) fn row_at(
    state: &GameState,
    seat: PlayerId,
    step: usize,
    game: &SelfplayGame,
    outcome: f64,
    prices: &Prices,
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
        hand: evaluate_priced(state, seat, next, &AI_EVAL, prices),
        outcome,
    }
}

/// The node budget the reply a `--every reply` row plays out may spend: more than any reply takes, so
/// every reply is played to its end.
const REPLY_NODES: usize = 2000;

/// R1006006: the row for the seat whose turn just ended at `end`, after the opponent's reply as the
/// search simulates it, scored as `reply_score` scores a line. `None` when the reply ends the game or
/// runs out of nodes. R4070000: `prices` are those of the decision that ended the turn, so the reply is
/// played and the row scored as that decision's search plays and scores its End Turn line.
fn reply_row(
    end: &GameState,
    seat: PlayerId,
    step: usize,
    game: &SelfplayGame,
    prices: &Prices,
) -> Result<Option<PositionRow>, Dropped> {
    let Some(outcome) = outcome_for(game.result, seat) else {
        return Err(Dropped::NoResult);
    };
    let counter = create_node_counter(REPLY_NODES, None);
    let Some(after) = simulate_reply(end, seat, &counter, &hidden_card_ids(end, seat), prices) else {
        return Ok(None);
    };
    if after.result.is_some() {
        return Ok(None);
    }
    Ok(Some(row_at(&after, seat, step, game, outcome, prices)))
}

/// R1006003: `game` folded under its catalog, its rows if it reaches its hash.
pub(crate) fn fold_game(
    game: &SelfplayGame,
    every: Every,
    build_catalog: &str,
) -> Result<Vec<PositionRow>, Dropped> {
    let shipped = catalog_for(game.catalog_now(), build_catalog).map_err(|_| Dropped::EarlierPatch)?;
    if game.result.is_none() {
        return Err(Dropped::NoResult);
    }
    if !game.faults.is_empty() {
        return Err(Dropped::Unclean);
    }
    let folded = catch_unwind(AssertUnwindSafe(|| {
        with_shipped(shipped, || {
            let options = CreateGameOptions {
                seed: game.seed.clone(),
                decks: game.decks.clone(),
                ..Default::default()
            };
            let mut state = begin_game(&create_recorded_game(&options)).state;
            let mut rows: Vec<PositionRow> = Vec::new();
            let mut turned: Vec<(i32, PlayerId)> = Vec::new();
            for (step, action) in game.log.iter().enumerate() {
                if let Some(seat) = position_seat(&state, every, &mut turned) {
                    let Some(outcome) = outcome_for(game.result, seat) else {
                        return Err(Dropped::NoResult);
                    };
                    // R4070000: priced as a decision of the seat's here prices its search.
                    let prices = decision_prices_at(&state, seat);
                    rows.push(row_at(&state, seat, step, game, outcome, &prices));
                }
                // R4070000: the prices of the decision that ended the turn, on the state it chose End
                // Turn at, priced only at a turn's end (`then`, not `then_some`).
                let ended_by = (every == Every::Reply
                    && action.body == ActionBody::EndTurn
                    && action.player_id == state.active
                    && state.phase == Phase::Main
                    && state.result.is_none())
                .then(|| (state.active, decision_prices_at(&state, state.active)));
                let result = reduce(&state, action);
                if result.error.is_some() {
                    return Err(Dropped::NotReplayed);
                }
                state = result.state;
                if let Some((seat, prices)) = &ended_by
                    && let Some(row) = reply_row(&state, *seat, step + 1, game, prices)?
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
        serde_json::json!({
            "out": args.out.display().to_string(),
            "every": args.every,
            "counts": counts,
        })
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_ai::{
        AI_GATE_BUDGET, MatchHooks, decision_prices, det_ground, dev_game_config, evaluate, play_match,
        probe_world, redact,
    };

    use crate::arena::TEST_BUDGET;
    use std::sync::OnceLock;

    /// A game as it was played and the true states before each of its actions.
    type Played = (SelfplayGame, Vec<(GameState, usize)>);

    fn play(n: i32) -> Played {
        jackioh_cards::register_all();
        let config = dev_game_config(n, "positions-test", Some(TEST_BUDGET));
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
            versions: Some(jackioh_engine::config::VERSIONS_SCHEME),
            ai_generation: None,
            faults: Vec::new(),
        };
        (game, live)
    }

    /// Game `n` (1 to 4), played once at `TEST_BUDGET` for every test that asks for it: they prove how
    /// a log folds, not how well the AI plays it.
    fn played(n: i32) -> &'static Played {
        static GAMES: [OnceLock<Played>; 4] = [const { OnceLock::new() }; 4];
        GAMES[usize::try_from(n - 1).expect("game numbers start at 1")].get_or_init(|| play(n))
    }

    /// R1006003, R1006004: the rows of a game's log are exactly the features computed on the true
    /// game, at the same steps, as it was played.
    #[test]
    fn r1006004_positions_over_a_log_are_the_features_of_the_true_game_as_it_was_played() {
        let (game, live) = played(1);
        assert!(game.result.is_some(), "the test game finishes");
        let rows = fold_game(game, Every::Decision, jackioh_cards::catalog_version()).expect("it replays");
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
        let turns = fold_game(game, Every::Turn, jackioh_cards::catalog_version()).expect("it replays");
        assert!(!turns.is_empty() && turns.len() < rows.len());
        let mut keys: Vec<(i32, PlayerId)> = turns.iter().map(|row| (row.turn, row.seat)).collect();
        keys.dedup();
        assert_eq!(keys.len(), turns.len(), "one row per seat per turn");
    }

    /// R1006003: a log that no longer reaches its hash is dropped, as is a game without a result or on
    /// a catalog this build does not carry.
    #[test]
    #[ignore = "daily: off per-PR CI by owner decision (#4093), no per-PR stand-in"]
    fn r1006003_a_game_that_does_not_replay_has_no_result_or_an_unknown_catalog_is_dropped() {
        let (game, _) = played(2);
        let build = jackioh_cards::catalog_version();
        let mut wrong = game.clone();
        wrong.hash = "00000000".to_string();
        assert_eq!(fold_game(&wrong, Every::Turn, build), Err(Dropped::NotReplayed));
        let mut unfinished = game.clone();
        unfinished.result = None;
        assert_eq!(fold_game(&unfinished, Every::Turn, build), Err(Dropped::NoResult));
        let mut old = game.clone();
        old.catalog = "0.0.0-never".to_string();
        assert_eq!(fold_game(&old, Every::Turn, build), Err(Dropped::EarlierPatch));
    }

    /// R1006006: `--every reply` writes one row per end of a seat's turn whose reply leaves the game
    /// going: the features and hand eval after the reply as the search simulates it, which is the
    /// seat's next turn, the same rows on every fold.
    #[test]
    #[ignore = "daily: off per-PR CI by owner decision (#4093), no per-PR stand-in"]
    fn r1006006_reply_rows_are_the_search_s_simulated_replies_one_per_turn_end() {
        let (game, _) = played(3);
        let build = jackioh_cards::catalog_version();
        let rows = fold_game(game, Every::Reply, build).expect("it replays");
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
        assert_eq!(rows, fold_game(game, Every::Reply, build).expect("again"));
        let turns = fold_game(game, Every::Turn, build).expect("it replays");
        // A reply row is the seat's position after the reply, a turn after the one it ended.
        for row in &rows {
            assert!(turns.iter().any(|turn| turn.turn > 0) && row.turn > 0);
        }
    }

    /// Whether `prices` price nothing: no Death, turn hook, Activate, attack hook or awakening.
    fn prices_nothing(prices: &Prices) -> bool {
        *prices == Prices::default()
    }

    /// R4070000: a reply row is played and scored with the prices of the decision that ended the turn,
    /// on the state it chose End Turn at, as that decision's search plays and scores its End Turn line;
    /// those prices are a full-budget decision's, and where they price a Death or a turn hook the row's
    /// hand eval is not the unpriced one it held before.
    #[test]
    fn r4070000_reply_rows_are_scored_with_the_prices_of_the_decision_that_ended_the_turn() {
        let (game, live) = played(1);
        let rows = fold_game(game, Every::Reply, jackioh_cards::catalog_version()).expect("it replays");
        assert!(!rows.is_empty());
        let mut moved = 0usize;
        for row in &rows {
            let (before, _) = live
                .iter()
                .find(|(_, step)| *step + 1 == row.step)
                .expect("a live state before every turn's end");
            let ended = &game.log[row.step - 1];
            assert_eq!(ended.body, ActionBody::EndTurn, "step {}", row.step);
            let end = reduce(before, ended).state;
            let prices = decision_prices_at(before, row.seat);
            let public = redact(before, row.seat);
            let probe = probe_world(&public, row.seat, &det_ground(&public, row.seat));
            assert_eq!(
                prices,
                decision_prices(&probe, row.seat, &create_node_counter(AI_GATE_BUDGET.nodes, None)),
                "step {}: every probe's cap prices as a full budget does",
                row.step
            );
            let after = simulate_reply(
                &end,
                row.seat,
                &create_node_counter(REPLY_NODES, None),
                &hidden_card_ids(&end, row.seat),
                &prices,
            )
            .expect("the row's reply runs to its end");
            let next = if after.active == row.seat {
                NextSwing::Seat
            } else {
                NextSwing::Enemy
            };
            assert_eq!(
                row.hand,
                evaluate_priced(&after, row.seat, next, &AI_EVAL, &prices),
                "step {}",
                row.step
            );
            assert_eq!(
                row.dense,
                features_of(&after, row.seat).dense,
                "step {}",
                row.step
            );
            if (!prices.deaths.is_empty() || !prices.turn_hooks.is_empty())
                && row.hand != evaluate(&after, row.seat, next, &AI_EVAL)
            {
                moved += 1;
            }
        }
        assert!(
            moved > 0,
            "no row of {} priced a Death or a turn hook",
            rows.len()
        );
    }

    /// R4070000: a decision row is priced as a decision there prices it, and one whose root prices
    /// nothing keeps the unpriced hand eval it held before.
    #[test]
    #[ignore = "daily: off per-PR CI by owner decision (#4093), no per-PR stand-in"]
    fn r4070000_a_row_whose_root_prices_nothing_keeps_the_hand_it_had() {
        let (game, live) = played(3);
        let rows = fold_game(game, Every::Decision, jackioh_cards::catalog_version()).expect("it replays");
        let mut unpriced = 0usize;
        for row in &rows {
            let (state, _) = live
                .iter()
                .find(|(_, step)| *step == row.step)
                .expect("a live state at every row's step");
            let next = if state.active == row.seat {
                NextSwing::Seat
            } else {
                NextSwing::Enemy
            };
            let prices = decision_prices_at(state, row.seat);
            assert_eq!(
                row.hand,
                evaluate_priced(state, row.seat, next, &AI_EVAL, &prices),
                "step {}",
                row.step
            );
            if prices_nothing(&prices) {
                assert_eq!(
                    row.hand,
                    evaluate(state, row.seat, next, &AI_EVAL),
                    "step {}",
                    row.step
                );
                unpriced += 1;
            }
        }
        assert!(unpriced > 0, "every one of {} rows priced something", rows.len());
    }

    /// R4159000, R1171000, R3112002: with the policy prior and the learned network retired, a positions
    /// file's header names no policy key layout, no row carries a policy label, and the counts it prints
    /// name no card a network has no row for.
    #[test]
    fn r4159000_r1171000_r3112002_positions_writes_no_policy_label_and_counts_no_unknown_card() {
        let header = serde_json::to_value(header(Every::Turn, "v0")).unwrap();
        assert!(header.get("policyKeysVersion").is_none(), "{header}");
        let (game, _) = played(1);
        let folded = fold_game(game, Every::Decision, jackioh_cards::catalog_version());
        let rows = folded.as_ref().expect("it replays");
        assert!(!rows.is_empty());
        for row in rows {
            let row = serde_json::to_value(row).unwrap();
            assert!(row.get("policy").is_none(), "{row}");
        }
        let mut counts = Counts::default();
        counts.add(&folded);
        let counts = serde_json::to_value(&counts).unwrap();
        assert_eq!(counts["positions"], rows.len());
        for gone in ["cards", "unknownCards", "unknownShare"] {
            assert!(counts.get(gone).is_none(), "{gone}: {counts}");
        }
    }

    #[test]
    fn the_outcome_is_one_a_half_or_nothing() {
        assert_eq!(outcome_for(None, PlayerId::P1), None);
    }
}
