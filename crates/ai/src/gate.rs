//! The quality gates (docs/polish/3-ai.md B28–B31, SPEC §9.9): three matchups, each a run of seeded
//! games in which the subject (the AI, or the Hard AI) alternates seats, every game folded back from
//! its log to prove it replays. The gate tests turn a report into a pass or a fail; this module plays
//! and measures, and says how many wins a run of a given size needs (`gate_needed`), so a tuner can
//! rerun one losing seed with `game_config(matchup, n, …)`.

use std::ops::Index;

use jackioh_engine::config::AI_DIFFICULTY;
use jackioh_engine::{
    GameOverReason, Handicap, PerPlayer, PerPlayerOpt, PlayerId, Rng, Winner, fold, hash_state,
};
use serde::{Deserialize, Serialize};

use crate::deck::{AiDeckOptions, build_ai_deck};
use crate::match_::{MatchConfig, MatchHooks, MatchRecord, MatchSetup, SeatController, play_match};
use crate::shadow_ban::FIXTURE_BANNED_IDS;
use crate::types::SearchBudget;

/// `"ai-vs-random" | "ai-vs-greedy" | "hard-vs-easy"`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Matchup {
    #[serde(rename = "ai-vs-random")]
    AiVsRandom,
    #[serde(rename = "ai-vs-greedy")]
    AiVsGreedy,
    #[serde(rename = "hard-vs-easy")]
    HardVsEasy,
}

impl Matchup {
    pub const ALL: &'static [Matchup] = &[Matchup::AiVsRandom, Matchup::AiVsGreedy, Matchup::HardVsEasy];

    pub fn as_str(self) -> &'static str {
        match self {
            Matchup::AiVsRandom => "ai-vs-random",
            Matchup::AiVsGreedy => "ai-vs-greedy",
            Matchup::HardVsEasy => "hard-vs-easy",
        }
    }
}

impl std::fmt::Display for Matchup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One value per matchup, serialized `{ "ai-vs-random": …, … }`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct ByMatchup<T> {
    #[serde(rename = "ai-vs-random")]
    pub ai_vs_random: T,
    #[serde(rename = "ai-vs-greedy")]
    pub ai_vs_greedy: T,
    #[serde(rename = "hard-vs-easy")]
    pub hard_vs_easy: T,
}

impl<T> Index<Matchup> for ByMatchup<T> {
    type Output = T;

    fn index(&self, matchup: Matchup) -> &T {
        match matchup {
            Matchup::AiVsRandom => &self.ai_vs_random,
            Matchup::AiVsGreedy => &self.ai_vs_greedy,
            Matchup::HardVsEasy => &self.hard_vs_easy,
        }
    }
}

/// `AI_GATE`'s shape.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiGate {
    /// Frozen seed series `${seedSeries}:${matchup}:${n}`. `gate:v3` re-deals decks around R635
    /// (cast on draw shuffled after mulligan, #832).
    pub seed_series: &'static str,
    /// Seeds per matchup in test runs under `gate_needed`.
    pub smoke_seeds: i32,
    /// What `pnpm ai:gate` (JACKIOH_AI_GATE=full) runs.
    pub full_seeds: ByMatchup<i32>,
    /// Target win share floors per matchup.
    pub brief_rate: ByMatchup<f64>,
    /// Measured win rate on tuning deals; lowering requires user sign-off (SPEC §9.9).
    pub measured_rate: ByMatchup<f64>,
    /// Max false-alarm probability from deal variance alone (SPEC §9.9).
    pub false_alarm: f64,
    /// SPEC §9.9: max decision time at AI_BUDGET on development machine.
    pub max_decision_ms: f64,
    /// Games replayed by timing gate.
    pub perf_smoke_games: i32,
    pub perf_full_games: i32,
    /// Max repeats per decision; stops at the first run under `max_decision_ms`.
    pub perf_repeats: i32,
    /// Timing gate yardstick games timed alongside decisions to normalize machine speed.
    pub calibration_games: i32,
    /// The yardstick's time on the development machine, alone.
    pub calibration_ref_ms: f64,
    /// #1240: max decision time at AI_BUDGET on pinned positions (`tests/fixtures/pinned-*.json`, #1337).
    pub pinned_decision_ms: f64,
}

pub const AI_GATE: AiGate = AiGate {
    seed_series: "gate:v3",
    smoke_seeds: 20,
    full_seeds: ByMatchup {
        ai_vs_random: 100,
        ai_vs_greedy: 50,
        hard_vs_easy: 50,
    },
    brief_rate: ByMatchup {
        ai_vs_random: 0.95,
        ai_vs_greedy: 0.7,
        hard_vs_easy: 0.8,
    },
    measured_rate: ByMatchup {
        ai_vs_random: 0.945,
        ai_vs_greedy: 0.68,
        hard_vs_easy: 0.913,
    },
    false_alarm: 0.05,
    max_decision_ms: 1500.0,
    perf_smoke_games: 1,
    perf_full_games: 6,
    perf_repeats: 3,
    calibration_games: 2,
    calibration_ref_ms: 72.0,
    pinned_decision_ms: 1000.0,
};

/// The series tuning runs play, so no tuned seed is a gate seed.
pub const AI_TUNING_SERIES: &str = "tune";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GateGame {
    pub seed: String,
    pub subject_seat: PlayerId,
    pub record: MatchRecord,
    pub won: bool,
    pub replay_hash: String,
    pub replay_errors: i32,
}

/// Matchup gate results evaluated against `gate_needed`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GateReport {
    pub matchup: Matchup,
    pub games: Vec<GateGame>,
    pub wins: i32,
    pub turn_cap_draws: i32,
    pub rate: f64,
}

/// P(X ≥ k) for X ~ Binomial(n, p), summed exactly over the distribution of n trials.
pub fn binomial_tail(n: i32, p: f64, k: i32) -> f64 {
    let mut dist: Vec<f64> = vec![1.0];
    for _trial in 0..n {
        let mut next: Vec<f64> = vec![0.0; dist.len() + 1];
        for (wins, q) in dist.iter().enumerate() {
            next[wins] += q * (1.0 - p);
            next[wins + 1] += q * p;
        }
        dist = next;
    }
    dist.iter().skip(k.max(0) as usize).fold(0.0, |sum, q| sum + q)
}

/// Wins needed for `matchup`: max k where P(Binomial(games, measured_rate) >= k) >= 1 - false_alarm,
/// capped at ceil(brief_rate * games) (SPEC §9.9).
pub fn gate_needed(matchup: Matchup, games: i32) -> i32 {
    let measured = AI_GATE.measured_rate[matchup];
    let mut k = 0;
    while k < games && binomial_tail(games, measured, k + 1) >= 1.0 - AI_GATE.false_alarm {
        k += 1;
    }
    k.min((AI_GATE.brief_rate[matchup] * f64::from(games)).ceil() as i32)
}

/// Game n (1-based): the subject sits p1 when n is odd and p2 when n is even.
pub fn subject_seat_of(n: i32) -> PlayerId {
    if n % 2 == 1 { PlayerId::P1 } else { PlayerId::P2 }
}

/// The seat the subject plays against, per matchup.
fn opponent_controller(matchup: Matchup, budget: SearchBudget) -> SeatController {
    match matchup {
        Matchup::AiVsRandom => SeatController::Random,
        Matchup::AiVsGreedy => SeatController::Greedy,
        Matchup::HardVsEasy => SeatController::Ai {
            budget: Some(budget),
            skill: None,
        },
    }
}

/// Configures game n of a matchup with alternating seats and handicap decks (shadow ban R186).
pub fn game_config(matchup: Matchup, n: i32, budget: SearchBudget, series: &str) -> MatchConfig {
    game_config_with_banned(matchup, n, budget, series, None)
}

/// #1113: deals game n using `FIXTURE_BANNED_IDS` for stable smoke test decks.
pub fn fixture_game_config(matchup: Matchup, n: i32, budget: SearchBudget, series: &str) -> MatchConfig {
    game_config_with_banned(
        matchup,
        n,
        budget,
        series,
        Some(FIXTURE_BANNED_IDS.iter().map(|id| id.to_string()).collect()),
    )
}

/// `game_config` dealing `banned`: `None` is the live shadow ban (R186), `Some` the list named.
fn game_config_with_banned(
    matchup: Matchup,
    n: i32,
    budget: SearchBudget,
    series: &str,
    banned: Option<Vec<String>>,
) -> MatchConfig {
    let seed = format!("{series}:{}:{n}", matchup.as_str());
    let subject_seat = subject_seat_of(n);
    let other_seat = subject_seat.opponent();

    let subject_handicap = if matchup == Matchup::HardVsEasy {
        AI_DIFFICULTY.hard
    } else {
        AI_DIFFICULTY.easy
    };
    let other_handicap = AI_DIFFICULTY.easy;

    let deck_for = |seat: PlayerId, handicap: &Handicap| -> Vec<String> {
        build_ai_deck(
            &mut Rng::new(&format!("{seed}:deck:{}", seat.as_str()), 0),
            handicap.deck_size,
            &AiDeckOptions {
                mana_cap: Some(handicap.mana_cap),
                banned: banned.clone(),
                ..AiDeckOptions::default()
            },
        )
    };
    let subject_deck = deck_for(subject_seat, &subject_handicap);
    let other_deck = deck_for(other_seat, &other_handicap);

    let subject_controller = SeatController::Ai {
        budget: Some(budget),
        skill: None,
    };
    let subject_first = subject_seat == PlayerId::P1;
    let controllers = if subject_first {
        PerPlayer {
            p1: subject_controller,
            p2: opponent_controller(matchup, budget),
        }
    } else {
        PerPlayer {
            p1: opponent_controller(matchup, budget),
            p2: subject_controller,
        }
    };

    MatchConfig {
        decks: if subject_first {
            (subject_deck, other_deck)
        } else {
            (other_deck, subject_deck)
        },
        handicaps: Some(if subject_first {
            PerPlayerOpt {
                p1: Some(subject_handicap),
                p2: Some(other_handicap),
            }
        } else {
            PerPlayerOpt {
                p1: Some(other_handicap),
                p2: Some(subject_handicap),
            }
        }),
        controllers,
        max_actions: None,
        seed,
        setup: MatchSetup::default(),
    }
}

/// Plays the listed matchup games, folded to record replay hash and errors against `gate_needed`.
pub fn run_gate_games(matchup: Matchup, numbers: &[i32], budget: SearchBudget) -> GateReport {
    run_games(matchup, numbers, budget, game_config)
}

/// #1113: `run_gate_games` using the frozen fixture pool.
pub fn run_fixture_gate_games(matchup: Matchup, numbers: &[i32], budget: SearchBudget) -> GateReport {
    run_games(matchup, numbers, budget, fixture_game_config)
}

/// The games `config_for` deals, each folded with its handicaps to fill replay_hash/replay_errors.
fn run_games(
    matchup: Matchup,
    numbers: &[i32],
    budget: SearchBudget,
    config_for: fn(Matchup, i32, SearchBudget, &str) -> MatchConfig,
) -> GateReport {
    let mut games: Vec<GateGame> = Vec::new();
    for &n in numbers {
        let config = config_for(matchup, n, budget, AI_GATE.seed_series);
        let subject_seat = subject_seat_of(n);
        let record = play_match(&config, &mut MatchHooks::default());

        let replayed = fold(&config.fold_args(record.log.clone()));

        let won = record
            .result
            .is_some_and(|result| result.winner.player() == Some(subject_seat));
        games.push(GateGame {
            seed: config.seed,
            subject_seat,
            won,
            replay_hash: hash_state(&replayed.state),
            replay_errors: replayed.errors.len() as i32,
            record,
        });
    }

    let wins = games.iter().filter(|game| game.won).count() as i32;
    let turn_cap_draws = games
        .iter()
        .filter(|game| {
            game.record.result.is_some_and(|result| {
                result.winner == Winner::Draw && result.reason == GameOverReason::TurnCap
            })
        })
        .count() as i32;
    let rate = if !games.is_empty() {
        f64::from(wins) / games.len() as f64
    } else {
        0.0
    };
    GateReport {
        matchup,
        games,
        wins,
        turn_cap_draws,
        rate,
    }
}

/// Partitions `total` games into interleaved shards of stride `count`.
pub fn gate_shard_games(total: i32, index: i32, count: i32) -> Vec<i32> {
    let mut numbers: Vec<i32> = Vec::new();
    let mut n = index;
    while n <= total {
        numbers.push(n);
        n += count;
    }
    numbers
}
