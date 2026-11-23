//! `cargo jackioh stats calibrate <file>… [--rows <file>]` (R4552000, SPEC §9.11): fits the win
//! probability Decisiveness is read through, `P = σ(β₀ + β₁·eval + β₂·turn + β₃·first)`, over the turn
//! rows of development records (`SeatSummary.swing_turns`), and prints the four weights as the
//! `pub const` lines to paste into `crates/engine/src/config.rs`, the AI generation the records' builds
//! hold (a change to the AI's evaluation moves the raw scale, so a refit names it in
//! `WIN_PROBABILITY_AI_GENERATION`), the Brier score beside the turn-and-first-player model's, and the
//! reliability table: per tenth of P the rows' mean P and the share of them their seat won.
//!
//! A row's outcome is its seat's result: a win 1, a draw ½, a loss 0. The fit is Newton's method on the
//! log-likelihood, solved by Gaussian elimination in a fixed order, so the same rows give the same
//! weights. `--rows` also writes the rows, one `eval turn first result` line each, as
//! `tests/fixtures/win_probability.txt` holds the fixture the calibration test reads.

use std::fmt::Write as _;
use std::path::PathBuf;

use anyhow::{Context, Result};
use jackioh_engine::config::SWING_EVAL_SCALE;
use jackioh_engine::{GameRecord, PLAYER_IDS, seat_result};

use crate::stats::read_records;

/// `cargo jackioh stats calibrate`'s arguments.
#[derive(clap::Args, Debug, Clone, PartialEq)]
pub struct Args {
    /// Record files, one GameRecord per line; only records whose seats carry turn rows count.
    #[arg(required = true, value_name = "FILE")]
    pub files: Vec<PathBuf>,
    /// Also write the rows, one `eval turn first result` line each (the calibration test's fixture).
    #[arg(long, value_name = "FILE")]
    pub rows: Option<PathBuf>,
}

/// Newton steps at most; the fit stops sooner once a step moves no weight by `FIT_TOLERANCE`.
const FIT_ITERATIONS: usize = 50;
const FIT_TOLERANCE: f64 = 1e-12;
/// Added to the Hessian's diagonal, so a feature every row holds alike leaves it solvable.
const FIT_RIDGE: f64 = 1e-9;
/// The reliability table's buckets: tenths of P.
pub const RELIABILITY_BUCKETS: usize = 10;

/// One turn row read for the fit: the evaluation difference in the AI's units, the turn counter,
/// whether the seat went first, and its result.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CalibrationRow {
    pub eval: f64,
    pub turn: f64,
    pub first: f64,
    pub result: f64,
}

impl CalibrationRow {
    /// The full model's features: an intercept, the evaluation, the turn, going first.
    pub fn features(&self) -> Vec<f64> {
        vec![1.0, self.eval, self.turn, self.first]
    }

    /// The baseline model's: an intercept, the turn, going first.
    pub fn baseline_features(&self) -> Vec<f64> {
        vec![1.0, self.turn, self.first]
    }
}

/// R4552000: every turn row of a record, each with its seat's result.
pub fn calibration_rows(record: &GameRecord) -> Vec<CalibrationRow> {
    let mut rows = Vec::new();
    for seat in PLAYER_IDS {
        let result = seat_result(&record.game, seat);
        let first = if record.game.first == seat { 1.0 } else { 0.0 };
        for row in record.game.seats[seat].swing_turns.iter().flatten() {
            rows.push(CalibrationRow {
                eval: row.eval as f64 / SWING_EVAL_SCALE,
                turn: f64::from(row.turn),
                first,
                result,
            });
        }
    }
    rows
}

fn sigmoid(z: f64) -> f64 {
    1.0 / (1.0 + (-z).exp())
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// `a·x = b` by Gaussian elimination with partial pivoting; a singular column gives 0.
fn solve(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Vec<f64> {
    let n = b.len();
    for column in 0..n {
        let pivot = (column..n)
            .max_by(|i, j| a[*i][column].abs().total_cmp(&a[*j][column].abs()))
            .unwrap_or(column);
        a.swap(column, pivot);
        b.swap(column, pivot);
        let head = a[column][column];
        if head.abs() < f64::MIN_POSITIVE {
            continue;
        }
        let (pivot_row, pivot_value) = (a[column].clone(), b[column]);
        for (cells, value) in a.iter_mut().zip(b.iter_mut()).skip(column + 1) {
            let factor = cells[column] / head;
            for (cell, pivot) in cells[column..].iter_mut().zip(&pivot_row[column..]) {
                *cell -= factor * pivot;
            }
            *value -= factor * pivot_value;
        }
    }
    let mut x = vec![0.0; n];
    for row in (0..n).rev() {
        let head = a[row][row];
        if head.abs() < f64::MIN_POSITIVE {
            continue;
        }
        let rest: f64 = (row + 1..n).map(|k| a[row][k] * x[k]).sum();
        x[row] = (b[row] - rest) / head;
    }
    x
}

/// R4552000: the logistic regression of the results on the features, by Newton's method from 0.
pub fn fit_logistic(rows: &[(Vec<f64>, f64)]) -> Vec<f64> {
    let k = rows.first().map_or(0, |(features, _)| features.len());
    let mut weights = vec![0.0; k];
    for _ in 0..FIT_ITERATIONS {
        let mut gradient = vec![0.0; k];
        let mut hessian = vec![vec![0.0; k]; k];
        for (features, result) in rows {
            let p = sigmoid(dot(&weights, features));
            let spread = p * (1.0 - p);
            for i in 0..k {
                gradient[i] += (result - p) * features[i];
                for j in 0..k {
                    hessian[i][j] += spread * features[i] * features[j];
                }
            }
        }
        for (i, row) in hessian.iter_mut().enumerate() {
            row[i] += FIT_RIDGE;
        }
        let step = solve(hessian, gradient);
        for (weight, moved) in weights.iter_mut().zip(&step) {
            *weight += moved;
        }
        if step.iter().all(|moved| moved.abs() < FIT_TOLERANCE) {
            break;
        }
    }
    weights
}

/// The full model's weights over some rows.
pub fn fit_rows(rows: &[CalibrationRow]) -> Vec<f64> {
    fit_logistic(
        &rows
            .iter()
            .map(|row| (row.features(), row.result))
            .collect::<Vec<_>>(),
    )
}

/// The baseline model's weights over some rows.
pub fn fit_baseline(rows: &[CalibrationRow]) -> Vec<f64> {
    fit_logistic(
        &rows
            .iter()
            .map(|row| (row.baseline_features(), row.result))
            .collect::<Vec<_>>(),
    )
}

/// The mean squared distance of each row's P from its result.
pub fn brier(rows: &[CalibrationRow], p: impl Fn(&CalibrationRow) -> f64) -> f64 {
    if rows.is_empty() {
        return 0.0;
    }
    rows.iter().map(|row| (p(row) - row.result).powi(2)).sum::<f64>() / rows.len() as f64
}

/// One tenth of P: its rows' mean P, the mean of their results, and how many there are.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReliabilityBucket {
    pub mean_p: f64,
    pub observed: f64,
    pub rows: usize,
}

/// R4552000: the reliability table, a bucket per `1 / buckets` of P, empty buckets left out.
pub fn reliability(
    rows: &[CalibrationRow],
    p: impl Fn(&CalibrationRow) -> f64,
    buckets: usize,
) -> Vec<ReliabilityBucket> {
    let mut sums = vec![(0.0, 0.0, 0usize); buckets];
    for row in rows {
        let probability = p(row);
        let at = ((probability * buckets as f64) as usize).min(buckets - 1);
        sums[at].0 += probability;
        sums[at].1 += row.result;
        sums[at].2 += 1;
    }
    sums.into_iter()
        .filter(|(_, _, n)| *n > 0)
        .map(|(p, observed, n)| ReliabilityBucket {
            mean_p: p / n as f64,
            observed: observed / n as f64,
            rows: n,
        })
        .collect()
}

/// The full model's P of a row under `weights`.
pub fn model_p(weights: &[f64], row: &CalibrationRow) -> f64 {
    sigmoid(dot(weights, &row.features()))
}

/// Reads the rows `--rows` writes (the calibration test's fixture).
#[cfg(test)]
fn parse_rows(text: &str) -> Result<Vec<CalibrationRow>> {
    let mut rows = Vec::new();
    for (at, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<f64> = line
            .split_whitespace()
            .map(str::parse::<f64>)
            .collect::<Result<_, _>>()
            .with_context(|| format!("line {}: not four numbers", at + 1))?;
        let [eval, turn, first, result] = fields[..] else {
            anyhow::bail!("line {}: not four numbers", at + 1);
        };
        rows.push(CalibrationRow {
            eval,
            turn,
            first,
            result,
        });
    }
    Ok(rows)
}

/// The rows as `--rows` writes them.
pub fn format_rows(rows: &[CalibrationRow]) -> String {
    let mut out =
        String::from("# eval turn first result (R4552000; `cargo jackioh stats calibrate --rows`)\n");
    for row in rows {
        let _ = writeln!(out, "{} {} {} {}", row.eval, row.turn, row.first, row.result);
    }
    out
}

/// The fit as `calibrate` prints it.
pub fn format_fit(rows: &[CalibrationRow], games: usize, generations: &[Option<i32>]) -> String {
    let weights = fit_rows(rows);
    let baseline = fit_baseline(rows);
    let named = ["INTERCEPT", "EVAL", "TURN", "FIRST"];
    let mut out = format!(
        "Win probability over {} turn rows of {games} games (R4552000).\n",
        rows.len()
    );
    for (name, weight) in named.iter().zip(&weights) {
        let _ = writeln!(out, "pub const WIN_PROBABILITY_{name}: f64 = {weight:?};");
    }
    let generations: Vec<String> = generations
        .iter()
        .map(|generation| generation.map_or_else(|| "none".to_string(), |n| n.to_string()))
        .collect();
    let _ = writeln!(
        out,
        "AI generation of the records' builds: {}",
        generations.join(", ")
    );
    let _ = writeln!(
        out,
        "Brier score: {:.5}; the turn and first player alone: {:.5}.",
        brier(rows, |row| model_p(&weights, row)),
        brier(rows, |row| sigmoid(dot(&baseline, &row.baseline_features())))
    );
    let _ = writeln!(out, "Reliability (mean P, observed, rows):");
    for bucket in reliability(rows, |row| model_p(&weights, row), RELIABILITY_BUCKETS) {
        let _ = writeln!(
            out,
            "  {:.3}  {:.3}  {}",
            bucket.mean_p, bucket.observed, bucket.rows
        );
    }
    out
}

/// `cargo jackioh stats calibrate`.
pub fn run(args: &Args) -> Result<()> {
    let records = read_records(&args.files)?;
    let carrying: Vec<&GameRecord> = records
        .iter()
        .filter(|record| {
            record
                .game
                .seats
                .iter()
                .any(|(_, seat)| seat.swing_turns.is_some())
        })
        .collect();
    let rows: Vec<CalibrationRow> = carrying
        .iter()
        .flat_map(|record| calibration_rows(record))
        .collect();
    if rows.is_empty() {
        anyhow::bail!("no record of the files named carries turn rows (R4552000)");
    }
    let mut generations: Vec<Option<i32>> = carrying
        .iter()
        .map(|record| record.build.as_ref().and_then(|build| build.ai_generation))
        .collect();
    generations.sort_unstable();
    generations.dedup();
    if let Some(path) = &args.rows {
        std::fs::write(path, format_rows(&rows)).with_context(|| format!("writing {}", path.display()))?;
    }
    print!("{}", format_fit(&rows, carrying.len(), &generations));
    Ok(())
}

// R4552000: the weights in `config.rs` are the fixture's fit, and the fixture's rows read through them
// are calibrated: the fixture is 120 development games' rows (`stats --games 120 --series calibration`,
// then `stats calibrate --rows`).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::config::{
        WIN_PROBABILITY_EVAL, WIN_PROBABILITY_FIRST, WIN_PROBABILITY_INTERCEPT, WIN_PROBABILITY_TURN,
    };
    use jackioh_engine::win_probability;

    const FIXTURE: &str = include_str!("../tests/fixtures/win_probability.txt");

    /// A bucket of P counts toward the check once it holds this many rows.
    const RELIABILITY_MIN_ROWS: usize = 100;
    /// How far a bucket's observed win rate may stand from its mean P.
    const RELIABILITY_TOLERANCE: f64 = 0.10;

    fn fixture() -> Vec<CalibrationRow> {
        parse_rows(FIXTURE).expect("the fixture parses")
    }

    /// The engine's P of a fixture row, the row's evaluation back in a record's thousandths.
    fn engine_p(row: &CalibrationRow) -> f64 {
        win_probability(
            (row.eval * SWING_EVAL_SCALE).round() as i64,
            row.turn as i32,
            row.first > 0.5,
        )
    }

    #[test]
    fn r4552000_the_win_probability_is_calibrated_on_the_development_fixture() {
        let rows = fixture();
        assert!(rows.len() > 1000, "{} rows", rows.len());
        let buckets = reliability(&rows, engine_p, RELIABILITY_BUCKETS);
        let checked: Vec<&ReliabilityBucket> = buckets
            .iter()
            .filter(|bucket| bucket.rows >= RELIABILITY_MIN_ROWS)
            .collect();
        assert!(checked.len() >= 4, "{buckets:?}");
        for bucket in checked {
            assert!(
                (bucket.observed - bucket.mean_p).abs() <= RELIABILITY_TOLERANCE,
                "{bucket:?}"
            );
        }
        let baseline = fit_baseline(&rows);
        let model = brier(&rows, engine_p);
        let alone = brier(&rows, |row| sigmoid(dot(&baseline, &row.baseline_features())));
        assert!(
            model < alone,
            "Brier {model} against the turn and first player's {alone}"
        );
    }

    #[test]
    fn r4552000_the_constants_are_the_fixtures_fit() {
        let fit = fit_rows(&fixture());
        let constants = [
            WIN_PROBABILITY_INTERCEPT,
            WIN_PROBABILITY_EVAL,
            WIN_PROBABILITY_TURN,
            WIN_PROBABILITY_FIRST,
        ];
        for (fitted, constant) in fit.iter().zip(constants) {
            assert!(
                (fitted - constant).abs() <= 1e-6 * constant.abs().max(1.0),
                "{fit:?} against {constants:?}"
            );
        }
    }

    #[test]
    fn r4552000_a_fit_recovers_known_weights_and_rows_round_trip() {
        // Rows whose results are their P's expected share, so the fit's optimum is the weights.
        let truth = [-0.4, 0.05, 0.02, 0.3];
        let mut rows = Vec::new();
        for eval in -20..=20 {
            for turn in 1..=12 {
                for first in [0.0, 1.0] {
                    let row = CalibrationRow {
                        eval: f64::from(eval) * 2.5,
                        turn: f64::from(turn),
                        first,
                        result: 0.0,
                    };
                    rows.push(CalibrationRow {
                        result: model_p(&truth, &row),
                        ..row
                    });
                }
            }
        }
        let fit = fit_rows(&rows);
        for (fitted, known) in fit.iter().zip(truth) {
            assert!((fitted - known).abs() < 1e-6, "{fit:?}");
        }
        assert_eq!(parse_rows(&format_rows(&rows)).unwrap(), rows);
    }
}
