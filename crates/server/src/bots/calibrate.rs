//! The strengths' seed ratings, measured (R748000, R1454, R610, R732001; #748): the games, the fit
//! and the maths `jackioh-server bot-calibrate` and `bot-calibrate-live` run (`cli::bot_calibrate`,
//! docs/bot-calibration.md).
//!
//! The strongest strength is the reference. Every pair of strengths plays the same number of games
//! with All Random decks and alternating seats, a fit turns the tallies into each strength's Elo
//! offset from the reference, and the reference's own rating comes from its rated games against
//! people, or, until those are read, from the seed of record shifted by the AI promotions since.
//! A seed is that anchor plus the strength's offset.
//!
//! Each game is a function of its seed: its decks, its engine rng and each seat's AI stream are all
//! drawn from it, and the AI's search is held to a node budget with no clock, so the tallies are the
//! same whatever the thread count. A game is the ladder bot's own decision (`decide_styled` at its
//! strength, its skill's roll included (R747000)) and nothing of its manner: no think time, no
//! concession, no idle turn.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicUsize, Ordering};

use jackioh_ai::{AiOptions, AiStyle, decide_styled, plays_best, random_action};
use jackioh_engine::wire::PerPlayer;
use jackioh_engine::{Action, PlayerId, Rng, Winner, seat_played_by, seat_to_act};

use crate::actor::engine::{self, Pinned};
use crate::bots::seat::{fallback_action, r188_forbidden};
use crate::config::{
    BOT_CALIBRATION_BISECTIONS, BOT_CALIBRATION_FIT_ITERATIONS, BOT_CALIBRATION_MAX_ACTIONS,
    BOT_CALIBRATION_MAX_TURN_ACTIONS, BOT_CALIBRATION_PRIOR_GAMES, BOT_CALIBRATION_SEARCH_SPAN,
    BOT_STRENGTHS, BotStrength, GLICKO_SCALE,
};
use crate::db::store::{BotRow, Pilot, RatedGameRow};

/// R748000: the strength the others are measured against: the strongest, the last of `BOT_STRENGTHS`.
pub const REFERENCE_STRENGTH: usize = BOT_STRENGTHS.len() - 1;

/// What a drawn game scores each side (definitional, as Elo's).
const DRAW_SCORE: f64 = 0.5;

/// One pair of strengths' games, `a < b`: how each ended, counted by strength, and how many were
/// left unfinished (`play_game` came back with no result) and so are not in the others.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PairTally {
    pub a: usize,
    pub b: usize,
    pub a_wins: usize,
    pub b_wins: usize,
    pub draws: usize,
    pub unfinished: usize,
}

/// One strength's fitted rating relative to the reference, in Elo points, and its standard error.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrengthOffset {
    pub strength: usize,
    pub offset: f64,
    pub error: f64,
}

/// One AI promotion as `training/history/*.jsonl` records it: its generation and its `vsParent`,
/// `wins` of `games`, the new AI against the one it replaced (R732001).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Promotion {
    pub generation: i64,
    pub wins: i64,
    pub games: i64,
}

/// Every pair of the first `count` strengths, `a < b`, in ascending order.
pub fn pairings(count: usize) -> Vec<(usize, usize)> {
    (0..count)
        .flat_map(|a| (a + 1..count).map(move |b| (a, b)))
        .collect()
}

/// The seed of game `n` between strengths `a` and `b` of the calibration `base` names.
pub fn game_seed(base: &str, a: usize, b: usize, n: usize) -> String {
    format!("{base}:{a}v{b}:{n}")
}

/// R748000: one game between two strengths with All Random decks (R258), as the engine plays it: the
/// game has no wall clock, no concession and no idle turn, so its result is a function of `seed`
/// alone. A seat that the AI gives no move it may play, or whose move the engine refuses, plays the
/// bot seat's fallback; a game that has neither, runs `BOT_CALIBRATION_MAX_ACTIONS` moves, or has a
/// turn of more than `BOT_CALIBRATION_MAX_TURN_ACTIONS` actions (with no clock to end it, a loop that
/// grows its own cost) has no result and is `None`.
///
/// The winner is the strength that began in that seat (`Winner::P1` is `p1`'s), whichever seat a
/// Glitch left it in (R677): each strength keeps its own stream and its own deck through a swap, as
/// an account does.
pub fn play_game(seed: &str, p1: &BotStrength, p2: &BotStrength) -> Option<Winner> {
    // Calibration plays on this build's own catalog.
    let pinned = Pinned::default();
    let decks = (
        engine::deal_random_deck(&format!("{seed}:p1-deck"), None),
        engine::deal_random_deck(&format!("{seed}:p2-deck"), None),
    );
    let created = engine::create_game(pinned, &engine::create_game_args(seed, &decks, None, None, None));
    let mut state = engine::begin_game(pinned, &created).state;
    let strengths = PerPlayer::new(p1, p2);
    let mut streams = PerPlayer::new(
        Rng::new(&format!("{seed}:bot-ai:{}", PlayerId::P1.as_str()), 0),
        Rng::new(&format!("{seed}:bot-ai:{}", PlayerId::P2.as_str()), 0),
    );
    let mut moves = 0;
    let (mut turn, mut turn_moves) = (state.turn, 0);
    while state.result.is_none() && moves < BOT_CALIBRATION_MAX_ACTIONS {
        if state.turn != turn {
            (turn, turn_moves) = (state.turn, 0);
        }
        if turn_moves >= BOT_CALIBRATION_MAX_TURN_ACTIONS {
            return None;
        }
        turn_moves += 1;
        let seat = seat_to_act(&state).unwrap_or(state.active);
        // R677: the strength that began in the other seat plays this one after a swap.
        let home = seat_played_by(&state, seat);
        let strength = strengths[home];
        // R747000: the skill's roll comes first, on the seat's own stream, as the bot seat's does;
        // a full skill takes no draw. No seat offers a draw here (R188), so none is left to answer.
        let mut rng = streams[home].clone();
        let chosen = if plays_best(strength.skill, &mut rng) {
            let mut options = AiOptions {
                rng,
                budget: strength.budget,
                should_stop: None,
            };
            let style = AiStyle {
                temperature: strength.temperature,
                missed_lethal_chance: strength.missed_lethal_chance,
            };
            let decision = decide_styled(&state, seat, &mut options, &style);
            rng = options.rng;
            decision.map(|decision| decision.action)
        } else {
            random_action(&state, seat, &mut rng)
        };
        streams[home] = rng;
        let body = chosen
            .filter(|body| !r188_forbidden(body))
            .or_else(|| fallback_action(pinned, &state, seat))?;
        let action = Action::new(body, seat, format!("calibrate-{moves}"));
        let mut result = engine::reduce(pinned, &state, &action);
        if result.error.is_some() {
            let body = fallback_action(pinned, &state, seat)?;
            result = engine::reduce(
                pinned,
                &state,
                &Action::new(body, seat, format!("calibrate-{moves}")),
            );
            if result.error.is_some() {
                return None;
            }
        }
        state = result.state;
        moves += 1;
    }
    let winner = state.result?.winner;
    Some(match winner {
        Winner::Draw => Winner::Draw,
        Winner::P1 => Winner::from(seat_played_by(&state, PlayerId::P1)),
        Winner::P2 => Winner::from(seat_played_by(&state, PlayerId::P2)),
    })
}

/// R748000: `games_per_pair` games for every pair of `BOT_STRENGTHS`, on `threads` workers, the
/// strength `a` of a pair in `p1` in the odd games and in `p2` in the even ones. Each game is a
/// function of its seed (`game_seed`) and each is counted by its own index, so the tallies do not
/// depend on `threads`. A game that panics counts as unfinished.
pub fn calibrate(base: &str, games_per_pair: usize, threads: usize) -> Vec<PairTally> {
    let pairs = pairings(BOT_STRENGTHS.len());
    let jobs: Vec<((usize, usize), usize)> = pairs
        .iter()
        .flat_map(|&pair| (1..=games_per_pair).map(move |n| (pair, n)))
        .collect();
    let next = AtomicUsize::new(0);
    let outcomes: Vec<(usize, Option<Winner>)> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..threads.max(1))
            .map(|_| {
                scope.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(&((a, b), n)) = jobs.get(index) else {
                            break;
                        };
                        let (first, second) = if a_first(n) { (a, b) } else { (b, a) };
                        let seed = game_seed(base, a, b, n);
                        let winner = catch_unwind(AssertUnwindSafe(|| {
                            play_game(&seed, &BOT_STRENGTHS[first], &BOT_STRENGTHS[second])
                        }))
                        .unwrap_or(None);
                        done.push((index, winner));
                    }
                    done
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| {
                worker
                    .join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
            })
            .collect()
    });
    let mut tallies: Vec<PairTally> = pairs
        .iter()
        .map(|&(a, b)| PairTally {
            a,
            b,
            a_wins: 0,
            b_wins: 0,
            draws: 0,
            unfinished: 0,
        })
        .collect();
    for (index, winner) in outcomes {
        let Some(&((a, b), n)) = jobs.get(index) else {
            continue;
        };
        let Some(tally) = tallies.iter_mut().find(|tally| (tally.a, tally.b) == (a, b)) else {
            continue;
        };
        match winner {
            None => tally.unfinished += 1,
            Some(Winner::Draw) => tally.draws += 1,
            // `play_game`'s winner is the seat a strength began in: `p1` is `a` in the odd games.
            Some(winner) if (winner == Winner::P1) == a_first(n) => tally.a_wins += 1,
            Some(_) => tally.b_wins += 1,
        }
    }
    tallies
}

/// Whether the lower strength of a pair plays `p1` in game `n`.
fn a_first(n: usize) -> bool {
    !n.is_multiple_of(2)
}

/// R748000: the Elo a score of `points` in `games` games stands for: `GLICKO_SCALE · ln(s / (1 - s))`
/// of the score `s` with `BOT_CALIBRATION_PRIOR_GAMES` drawn game added, so a sweep stays finite.
pub fn elo_of_score(points: f64, games: f64) -> f64 {
    let score = (points + BOT_CALIBRATION_PRIOR_GAMES * DRAW_SCORE) / (games + BOT_CALIBRATION_PRIOR_GAMES);
    GLICKO_SCALE * (score / (1.0 - score)).ln()
}

/// R748000: the Bradley-Terry fit of `tallies` over `strengths` strengths, each strength's offset
/// from `reference` in Elo points (0 for the reference) and its standard error. Every pair's tally
/// has `BOT_CALIBRATION_PRIOR_GAMES` drawn game added, a draw is half a win each, and unfinished
/// games are not counted. The fit is the minorization-maximization iteration, `BOT_CALIBRATION_FIT_ITERATIONS`
/// steps, each normalised so the reference's strength is 1.
pub fn fit_offsets(tallies: &[PairTally], strengths: usize, reference: usize) -> Vec<StrengthOffset> {
    let mut games = vec![vec![0.0_f64; strengths]; strengths];
    let mut points = vec![0.0_f64; strengths];
    for tally in tallies {
        let played = (tally.a_wins + tally.b_wins + tally.draws) as f64 + BOT_CALIBRATION_PRIOR_GAMES;
        games[tally.a][tally.b] += played;
        games[tally.b][tally.a] += played;
        let drawn = (tally.draws as f64 + BOT_CALIBRATION_PRIOR_GAMES) * DRAW_SCORE;
        points[tally.a] += tally.a_wins as f64 + drawn;
        points[tally.b] += tally.b_wins as f64 + drawn;
    }
    let mut gamma = vec![1.0_f64; strengths];
    for _ in 0..BOT_CALIBRATION_FIT_ITERATIONS {
        let next: Vec<f64> = (0..strengths)
            .map(|i| {
                let denominator: f64 = (0..strengths)
                    .filter(|&j| j != i)
                    .map(|j| games[i][j] / (gamma[i] + gamma[j]))
                    .sum();
                if denominator > 0.0 {
                    points[i] / denominator
                } else {
                    gamma[i]
                }
            })
            .collect();
        let scale = next[reference];
        gamma = next.iter().map(|value| value / scale).collect();
    }
    (0..strengths)
        .map(|i| {
            let information: f64 = (0..strengths)
                .filter(|&j| j != i)
                .map(|j| {
                    let p = gamma[i] / (gamma[i] + gamma[j]);
                    games[i][j] * p * (1.0 - p)
                })
                .sum();
            StrengthOffset {
                strength: i,
                offset: GLICKO_SCALE * gamma[i].ln(),
                error: if i == reference || information <= 0.0 {
                    0.0
                } else {
                    GLICKO_SCALE / information.sqrt()
                },
            }
        })
        .collect()
}

/// R748000: the seed ratings: each strength's offset added to `anchor` (the reference's rating),
/// rounded to a whole number, in the offsets' order.
pub fn seed_ratings(anchor: f64, offsets: &[StrengthOffset]) -> Vec<f64> {
    offsets.iter().map(|fit| (anchor + fit.offset).round()).collect()
}

/// R732001: a promotion's `vsParent` field, `"65/100"`, as `(wins, games)`; `None` unless it is two
/// whole numbers with at least one game and no more wins than games.
pub fn parse_vs_parent(text: &str) -> Option<(i64, i64)> {
    let (wins, games) = text.split_once('/')?;
    let (wins, games) = (
        wins.trim().parse::<i64>().ok()?,
        games.trim().parse::<i64>().ok()?,
    );
    (games > 0 && (0..=games).contains(&wins)).then_some((wins, games))
}

/// R748000: how far the reference's rating moves over the AI promotions with a generation after
/// `after` and no later than `through`: the sum of each one's `vsParent` as Elo (`elo_of_score`, a
/// game it did not win counting as lost).
pub fn promotion_shift(promotions: &[Promotion], after: i64, through: i64) -> f64 {
    promotions
        .iter()
        .filter(|promotion| promotion.generation > after && promotion.generation <= through)
        .fold(0.0, |total, promotion| {
            total + elo_of_score(promotion.wins as f64, promotion.games as f64)
        })
}

/// R748000: the rating a player held to `(opponent rating, score)` games performed at, and its
/// standard error: the `R` at which the games' expected scores, with `BOT_CALIBRATION_PRIOR_GAMES`
/// drawn game against the mean opponent added, sum to the scores' (found by `BOT_CALIBRATION_BISECTIONS`
/// halvings between `BOT_CALIBRATION_SEARCH_SPAN` below the lowest opponent and above the highest).
/// `None` with no games.
pub fn performance_rating(games: &[(f64, f64)]) -> Option<(f64, f64)> {
    if games.is_empty() {
        return None;
    }
    let expected = |rating: f64, opponent: f64| 1.0 / (1.0 + ((opponent - rating) / GLICKO_SCALE).exp());
    let mean = games.iter().map(|(opponent, _)| opponent).sum::<f64>() / games.len() as f64;
    let target = games.iter().map(|(_, score)| score).sum::<f64>() + BOT_CALIBRATION_PRIOR_GAMES * DRAW_SCORE;
    let total = |rating: f64| {
        games
            .iter()
            .map(|(opponent, _)| expected(rating, *opponent))
            .sum::<f64>()
            + BOT_CALIBRATION_PRIOR_GAMES * expected(rating, mean)
    };
    let lowest = games
        .iter()
        .map(|(opponent, _)| *opponent)
        .fold(f64::INFINITY, f64::min);
    let highest = games
        .iter()
        .map(|(opponent, _)| *opponent)
        .fold(f64::NEG_INFINITY, f64::max);
    let (mut low, mut high) = (
        lowest - BOT_CALIBRATION_SEARCH_SPAN,
        highest + BOT_CALIBRATION_SEARCH_SPAN,
    );
    for _ in 0..BOT_CALIBRATION_BISECTIONS {
        let middle = (low + high) / 2.0;
        if total(middle) < target {
            low = middle;
        } else {
            high = middle;
        }
    }
    let rating = (low + high) / 2.0;
    let information: f64 = games
        .iter()
        .map(|(opponent, _)| *opponent)
        .chain(std::iter::once(mean))
        .map(|opponent| {
            let p = expected(rating, opponent);
            p * (1.0 - p)
        })
        .sum();
    Some((rating, GLICKO_SCALE / information.sqrt()))
}

/// R748000: the reference's games against people, as `(the person's rating before the game, the
/// reference's score)`, in the games' order: a rated game with a bot of `strength` on one side
/// (`bots` says which strength a bot id is) and a person on the other. A win scores 1, a draw
/// (`winner_side` null) 0.5 and a loss 0. A game between two bots, or with a bot of another
/// strength, is not one.
pub fn reference_observations(games: &[RatedGameRow], bots: &[BotRow], strength: usize) -> Vec<(f64, f64)> {
    let is_reference = |bot_id: &Option<String>| {
        bot_id.as_ref().is_some_and(|id| {
            bots.iter()
                .any(|bot| bot.profile_id == *id && bot.strength == strength)
        })
    };
    games
        .iter()
        .filter_map(|game| {
            let (first, second) = &game.sides;
            let side = [first, second]
                .iter()
                .position(|side| is_reference(&side.bot_id))?;
            let other = if side == 0 { second } else { first };
            if other.pilot != Pilot::Human {
                return None;
            }
            let score = match game.winner_side {
                Some(winner) if winner == side => 1.0,
                None => DRAW_SCORE,
                Some(_) => 0.0,
            };
            Some((other.before.rating, score))
        })
        .collect()
}
