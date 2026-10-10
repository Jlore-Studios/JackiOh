//! The quality gates (docs/polish/3-ai.md B28–B31, SPEC §9.9): three matchups, each a run of seeded
//! games in which the subject (the AI, or the Hard AI) alternates seats, every game folded back from
//! its log to prove it replays. The gate tests turn a report into a pass or a fail; this module plays
//! and measures, and says how many wins a run of a given size needs (`gate_needed`), so a tuner can
//! rerun one losing seed with `game_config(matchup, n, …)`.

use std::ops::Index;

use jackioh_engine::config::AI_DIFFICULTY;
use jackioh_engine::{
    FoldArgs, GameOverReason, Handicap, PerPlayer, PerPlayerOpt, PlayerId, Rng, Winner, fold, hash_state,
};
use serde::{Deserialize, Serialize};

use crate::deck::{AiDeckOptions, build_ai_deck};
use crate::match_::{MatchConfig, MatchHooks, MatchRecord, SeatController, play_match};
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

/// One value per matchup, serialised `{ "ai-vs-random": …, … }`.
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
    /// The frozen seed series: game n of a matchup is `${seedSeries}:${matchup}:${n}`. Only the gate
    /// plays it; tuning plays `AI_TUNING_SERIES`, so no tuning run touches a gate seed.
    ///
    /// `gate:v3` is a re-roll of the deals, not tuning and not a lower floor: R635 (a card that casts
    /// on draw is set aside and shuffled in after the mulligan) re-dealt every game whose deck holds
    /// one, and a gate on its floor goes red on any neutral reshuffle (reviews/2026-09-23-polish-part-b.md
    /// B-3). If it fails with the AI untouched, look at the deals first.
    pub seed_series: &'static str,
    /// What `pnpm test` runs per matchup (seeds 1..20), under the same rule as the full run
    /// (`gate_needed`: 17, 10 and 16 wins). At four games no count could tell a working AI from a broken
    /// one.
    pub smoke_seeds: i32,
    /// What `pnpm ai:gate` (JACKIOH_AI_GATE=full) runs.
    pub full_seeds: ByMatchup<i32>,
    /// The brief's floors (docs/polish/reference.md, "Quality gates"): the share of its games the
    /// subject is to win. Only wins count, in every gate; a draw at the turn cap is reported beside them.
    pub brief_rate: ByMatchup<f64>,
    /// The subject's win rate as it ships, measured on tuning deals that no run had tuned on and no
    /// gate plays (`tune` 3001–4000 for the Easy matchups, 3001–3300 for Hard against Easy): 945, 680
    /// and 274 wins. Against greedy it is short of the brief. A new measurement of the same kind may
    /// raise it; lowering it lowers every count below and needs the user's sign-off (SPEC §9.9).
    pub measured_rate: ByMatchup<f64>,
    /// The most often one gate run may fail an AI exactly as strong as `measuredRate` from the luck of
    /// its deals alone. `gate_needed` sets each count from it. Proposed in SPEC §9.9, pending the user's
    /// acceptance.
    pub false_alarm: f64,
    /// SPEC §9.9: the most one decision at AI_BUDGET may take on the development machine.
    pub max_decision_ms: f64,
    /// ai-vs-greedy games whose AI decisions the timing gate replays: `pnpm test`, then `pnpm ai:gate`.
    pub perf_smoke_games: i32,
    pub perf_full_games: i32,
    /// The most runs per decision; the fastest counts, so a context switch on a shared machine is not a
    /// failure. The runs stop at the first under `maxDecisionMs`, since later ones cannot fail it.
    pub perf_repeats: i32,
    /// The timing gate's yardstick: random-policy games 1..calibrationGames of ai-vs-random, played
    /// through `reduce`, a fixed piece of engine work timed beside every decision. A decision is judged
    /// by its time over the yardstick's, so a slower or busier machine slows both and fails nothing.
    pub calibration_games: i32,
    /// The yardstick's time on the development machine, alone (the median of 25 runs, 2026-09-23).
    pub calibration_ref_ms: f64,
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

/// `wins` is what a gate holds against `gate_needed`, and `rate` is wins / games. `turn_cap_draws` is
/// reported beside them and counts for nothing.
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

/// The wins a run of `games` games of `matchup` needs: the brief's share of them, or fewer where an
/// AI exactly as strong as `AI_GATE.measuredRate` would fall short of that more often than
/// `AI_GATE.falseAlarm` allows. That is the largest k with P(Binomial(games, measuredRate) ≥ k) ≥
/// 1 − falseAlarm, capped at ceil(briefRate × games) (SPEC §9.9).
pub fn gate_needed(matchup: Matchup, games: i32) -> i32 {
    let measured = AI_GATE.measured_rate[matchup];
    let mut k = 0;
    while k < games && binomial_tail(games, measured, k + 1) >= 1.0 - AI_GATE.false_alarm {
        k += 1;
    }
    k.min((AI_GATE.brief_rate[matchup] * f64::from(games)).ceil() as i32)
}

/// Game n (1-based): the subject sits p1 when n is odd and p2 when n is even.
fn subject_seat_of(n: i32) -> PlayerId {
    if n % 2 == 1 { PlayerId::P1 } else { PlayerId::P2 }
}

/// The seat the subject plays against, per matchup.
fn opponent_controller(matchup: Matchup, budget: SearchBudget) -> SeatController {
    match matchup {
        Matchup::AiVsRandom => SeatController::Random,
        Matchup::AiVsGreedy => SeatController::Greedy,
        Matchup::HardVsEasy => SeatController::Ai { budget: Some(budget) },
    }
}

/// Game n (1-based) of a matchup: seed `${series}:${matchup}:${n}`, the subject (the AI, or the Hard
/// AI) on p1 when n is odd. Every seat's deck is built by one rule, `build_ai_deck` with its
/// handicap's deck size and mana cap, the shadow ban (R186) included: the gates measure play, so
/// neither side is dealt cards the other side's rule keeps out. Handicaps: ai-vs-* use Easy for both
/// seats; hard-vs-easy gives the subject AI_DIFFICULTY.hard and the other AI_DIFFICULTY.easy.
pub fn game_config(matchup: Matchup, n: i32, budget: SearchBudget, series: &str) -> MatchConfig {
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
                ..AiDeckOptions::default()
            },
        )
    };
    let subject_deck = deck_for(subject_seat, &subject_handicap);
    let other_deck = deck_for(other_seat, &other_handicap);

    let subject_controller = SeatController::Ai { budget: Some(budget) };
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
    }
}

/// Plays games 1..seeds; each is folded with its handicaps to fill replay_hash/replay_errors.
pub fn run_gate(matchup: Matchup, seeds: i32, budget: SearchBudget) -> GateReport {
    let numbers: Vec<i32> = (1..=seeds.max(0)).collect();
    run_gate_games(matchup, &numbers, budget)
}

/// Plays the listed games of a matchup (1-based, as `game_config` numbers them), each folded with its
/// handicaps to fill replay_hash/replay_errors. CI plays a gate in shards, each one every k-th game
/// (`gate_shard_games`), and holds the wins of all of them together against `gate_needed`.
pub fn run_gate_games(matchup: Matchup, numbers: &[i32], budget: SearchBudget) -> GateReport {
    let mut games: Vec<GateGame> = Vec::new();
    for &n in numbers {
        let config = game_config(matchup, n, budget, AI_GATE.seed_series);
        let subject_seat = subject_seat_of(n);
        let record = play_match(&config, &mut MatchHooks::default());

        let replayed = fold(&FoldArgs {
            seed: config.seed.clone(),
            decks: config.decks.clone(),
            log: record.log.clone(),
            handicaps: config.handicaps.clone(),
            ..FoldArgs::default()
        });

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

/// Shard `index` of `count` (1-based) of a gate run of `total` games: games index, index + count,
/// index + 2 × count, …, so every shard gets a share of early and late seeds and the shards together
/// play each game exactly once.
pub fn gate_shard_games(total: i32, index: i32, count: i32) -> Vec<i32> {
    let mut numbers: Vec<i32> = Vec::new();
    let mut n = index;
    while n <= total {
        numbers.push(n);
        n += count;
    }
    numbers
}
