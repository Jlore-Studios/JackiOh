//! The shadow-ban sweep (R186, R390, docs/polish/3-ai.md B25, docs/classic-sets.md B4.4): force one
//! card into AI decks, play them against the greedy baseline, and flag what went wrong with that card.
//! `cargo jackioh sweep` (SURFACE §12; TS `scripts/sweep.ts`) runs it over every non-token card of
//! every set at every tier in AI_SWEEP.tiers and prints the rows that `shadow_ban.rs` is filled from.
//!
//! Why more than one tier. The ban keeps a card out of the AI's decks at every difficulty, and a
//! tier's resources decide what the AI can do with a card: at Easy's four crystals a 6-cost card is
//! never affordable at all, so an Easy-only sweep has no evidence either way, and a card the AI
//! never played on four crystals may well be played on seven. So each card is swept at the
//! cheapest tier (Easy) and the richest (Hard): a flag at either bans it (the reason names the
//! tier), and a card that was never once affordable at any tier is reported as unswept rather than
//! passed as clean.
//!
//! Two passes (R390). Pass 1 (`sweep_card`) is the sweep above. A card is at risk (`at_risk_ids`) when
//! pass 1's numbers meet a flag's condition at half strength (`half_flags`), or it is banned already
//! or on `SHADOW_WATCH`. Pass 2 (`sweep_at_risk`) forces each at-risk card into more games, on seeds of
//! its own, with every at-risk card's filler weight boosted, and counts every at-risk card the AI was
//! dealt, forced or not. A `neverPlayed`, `selfHarm` or `selfKill` ban needs pass 2's numbers; `error`
//! and `timeout` ban from any game whose forced card the card was (`sweep_verdict`).
//!
//! Pure like the rest of src/: the clock arrives as `now`, which the CLI passes and a test leaves
//! out, so a test's sweep never times out on a slow machine.
//!
//! Port of `packages/ai/src/sweep.ts`. TS's `try`/`catch` around dealing and playing a game is
//! `catch_unwind`: a deck that cannot be dealt, or a game that panics where TS threw, is an error.

use std::ops::{Deref, DerefMut};
use std::panic::{AssertUnwindSafe, catch_unwind};

use indexmap::{IndexMap, IndexSet};
use jackioh_engine::prelude::json_as;
use jackioh_engine::{
    AI_DIFFICULTY, ActionBody, CostOptions, Difficulty, GameState, OffFieldZone, PerPlayer, PerPlayerOpt,
    Phase, PlayerId, create_rng, effective_cost, opponent_of, zone_cards,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::{AI_EVAL, AI_GATE_BUDGET};
use crate::deck::{AiDeckOptions, build_ai_deck};
use crate::evaluate::{NextSwing, evaluate};
use crate::match_::{MatchConfig, MatchHooks, SeatController, play_match};
use crate::shadow_ban::SHADOW_BAN;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum SweepFlag {
    Error,
    Timeout,
    NeverPlayed,
    SelfHarm,
    SelfKill,
}

impl SweepFlag {
    /// The literal, as TS writes it (and as a SHADOW_BAN reason starts with it).
    pub fn as_str(self) -> &'static str {
        match self {
            SweepFlag::Error => "error",
            SweepFlag::Timeout => "timeout",
            SweepFlag::NeverPlayed => "neverPlayed",
            SweepFlag::SelfHarm => "selfHarm",
            SweepFlag::SelfKill => "selfKill",
        }
    }
}

impl std::fmt::Display for SweepFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// `AI_SWEEP`'s shape (SURFACE §4.2: a constant object is a const of a struct named after it).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AiSweep {
    /// The tiers every card is swept at: the fewest resources and the most (see the header).
    pub tiers: &'static [Difficulty],
    /// Games per card and tier. Four left the neverPlayed flag at the mercy of one deal.
    pub seeds_per_card: i32,
    pub decision_ms: i32,
    pub max_actions: i32,
    pub min_affordable_turns: i32,
    pub self_harm_delta: f64,
    /// selfHarm needs at least this many plays, so one play into a trap does not ban a card.
    pub min_harm_plays: i32,
    /// R390: pass 2's games per at-risk card and tier.
    pub seeds_per_card_at_risk: i32,
    /// R390: pass 2's filler draw multiplies every at-risk card's weight by this (as AI_DECK.themeBoost).
    /// A whole number, so `AiDeckOptions.boost.by` reads it whatever its number type.
    pub at_risk_boost: i32,
    /// R390: a neverPlayed ban needs this many affordable turns over pass 2's games, and no play.
    pub ban_affordable_turns: i32,
    /// R390: a selfHarm ban needs this many plays over pass 2's games.
    pub ban_harm_plays: i32,
    /// R390: selfKill needs at least this many plays on which the AI lost the game; half of it puts a
    /// card at risk.
    pub min_losing_plays: i32,
    /// R390, R601: a selfKill ban needs this many lost games over pass 2's games.
    pub ban_losing_plays: i32,
}

pub const AI_SWEEP: AiSweep = AiSweep {
    tiers: &[Difficulty::Easy, Difficulty::Hard],
    seeds_per_card: 8,
    decision_ms: 2000,
    max_actions: 600,
    min_affordable_turns: 3,
    self_harm_delta: -40.0,
    min_harm_plays: 4,
    seeds_per_card_at_risk: 24,
    at_risk_boost: 4,
    ban_affordable_turns: 6,
    ban_harm_plays: 8,
    min_losing_plays: 2,
    ban_losing_plays: 4,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SweepStats {
    pub def_id: String,
    pub games: i32,
    pub drawn_games: i32,
    pub affordable_turns: i32,
    pub plays: i32,
    pub errors: i32,
    pub timeouts: i32,
    /// R390: over the plays after which the game went on.
    pub eval_delta_sum: f64,
    pub eval_delta_count: i32,
    /// R390: plays on which the AI lost the game, out of the mean (selfKill's count). 0 in older slices.
    #[serde(default)]
    pub losing_plays: i32,
    /// R390: plays on which the AI won the game, out of the mean. 0 in older slices.
    #[serde(default)]
    pub winning_plays: i32,
}

/// One card at one tier. `unswept` is a card that was never once affordable in hand, so its games
/// say nothing about whether the AI can play it (a report, never a ban flag).
///
/// TS `SweepStats & { tier; flags; unswept }`: the stats are flattened beside the three, as TS writes
/// the object, and the result reads as its stats (`Deref`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SweepResult {
    #[serde(flatten)]
    pub stats: SweepStats,
    pub tier: Difficulty,
    pub flags: Vec<SweepFlag>,
    pub unswept: bool,
}

impl Deref for SweepResult {
    type Target = SweepStats;

    fn deref(&self) -> &SweepStats {
        &self.stats
    }
}

impl DerefMut for SweepResult {
    fn deref_mut(&mut self) -> &mut SweepStats {
        &mut self.stats
    }
}

/// R390: an error or a timeout in a pass-2 game, listed against an at-risk card the AI was dealt as
/// filler in it. It bans the forced card only; the suspect is banned only if its own games repeat it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SweepSuspect {
    pub def_id: String,
    pub seed: String,
    pub forced: String,
    pub errors: i32,
    pub timeouts: i32,
}

/// R390: one at-risk card's pass 2 at one tier. `cards` holds every at-risk card the AI was dealt in
/// these games, the forced one first, each counted over the games it was dealt in; only the forced
/// card's entry counts errors and timeouts, which `suspects` lists against the filler.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SweepPass2 {
    pub forced: String,
    pub tier: Difficulty,
    pub games: i32,
    pub cards: Vec<SweepStats>,
    pub suspects: Vec<SweepSuspect>,
}

/// A card over every tier and pass it was swept at: the union of the flags, and the ban reason if any.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SweepVerdict {
    pub def_id: String,
    pub flags: Vec<SweepFlag>,
    /// Never affordable at any tier: no evidence for or against it.
    pub unswept: bool,
    /// The SHADOW_BAN reason, `"<flags>: <tier>: <what it measured>; …"`, or null when unflagged.
    pub reason: Option<String>,
    /// R390, R600: the SHADOW_WATCH entry (at risk by its own numbers, and not banned), or null.
    pub watch: Option<String>,
}

/// `sweepCard`'s and `sweepAtRisk`'s options (TS's anonymous `{ seeds?, now?, tier? }`).
#[derive(Clone, Copy, Default)]
pub struct SweepOptions<'a> {
    pub seeds: Option<i32>,
    /// The clock, in milliseconds (TS `performance.now`); absent, no decision ever times out.
    pub now: Option<&'a dyn Fn() -> f64>,
    pub tier: Option<Difficulty>,
}

const FLAG_ORDER: &[SweepFlag] = &[
    SweepFlag::Error,
    SweepFlag::Timeout,
    SweepFlag::NeverPlayed,
    SweepFlag::SelfHarm,
    SweepFlag::SelfKill,
];

fn mean(stats: &SweepStats) -> f64 {
    stats.eval_delta_sum / f64::from(stats.eval_delta_count.max(1))
}

/// JS `x.toFixed(1)`. Rust's `{:.1}` writes the decimal closest to `x`, exactly as JS does, except at
/// an exact tie (only an odd multiple of 0.25 is one), which JS rounds away from zero and Rust to
/// even; and JS writes a negative zero without its sign.
fn to_fixed_1(x: f64) -> String {
    if x.is_nan() {
        return "NaN".to_string();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    if x == 0.0 {
        return "0.0".to_string();
    }
    let quarters = x.abs() * 4.0;
    if quarters.fract() == 0.0 && quarters % 2.0 == 1.0 {
        let tenths = (x.abs() * 10.0).round() as i64;
        let sign = if x < 0.0 { "-" } else { "" };
        return format!("{sign}{}.{}", tenths / 10, tenths % 10);
    }
    format!("{x:.1}")
}

/// error: errors > 0; timeout: timeouts > 0; neverPlayed: affordableTurns >= minAffordableTurns && plays === 0;
/// selfHarm: evalDeltaCount >= minHarmPlays && evalDeltaSum / evalDeltaCount < selfHarmDelta;
/// selfKill: losingPlays >= minLosingPlays. In that order.
pub fn sweep_flags(stats: &SweepStats) -> Vec<SweepFlag> {
    let mut flags = Vec::new();
    if stats.errors > 0 {
        flags.push(SweepFlag::Error);
    }
    if stats.timeouts > 0 {
        flags.push(SweepFlag::Timeout);
    }
    if stats.affordable_turns >= AI_SWEEP.min_affordable_turns && stats.plays == 0 {
        flags.push(SweepFlag::NeverPlayed);
    }
    if stats.eval_delta_count >= AI_SWEEP.min_harm_plays && mean(stats) < AI_SWEEP.self_harm_delta {
        flags.push(SweepFlag::SelfHarm);
    }
    if stats.losing_plays >= AI_SWEEP.min_losing_plays {
        flags.push(SweepFlag::SelfKill);
    }
    flags
}

/// R390: the judgement flags at half strength, which put a card at risk: `neverPlayed` when it sat
/// affordable on minAffordableTurns turns and was played at most once, `selfHarm` when its plays'
/// mean evaluation change is below half of selfHarmDelta, `selfKill` on half of minLosingPlays lost
/// games (one).
pub fn half_flags(stats: &SweepStats) -> Vec<SweepFlag> {
    let mut flags = Vec::new();
    if stats.affordable_turns >= AI_SWEEP.min_affordable_turns && stats.plays <= 1 {
        flags.push(SweepFlag::NeverPlayed);
    }
    if stats.eval_delta_count > 0 && mean(stats) < AI_SWEEP.self_harm_delta / 2.0 {
        flags.push(SweepFlag::SelfHarm);
    }
    if stats.losing_plays >= (AI_SWEEP.min_losing_plays / 2).max(1) {
        flags.push(SweepFlag::SelfKill);
    }
    flags
}

/// The flags a SHADOW_BAN reason starts with ("error, neverPlayed: easy: …" → error, neverPlayed).
pub fn ban_flags(reason: &str) -> Vec<SweepFlag> {
    let head: Vec<&str> = reason.split(':').next().unwrap_or("").split(", ").collect();
    FLAG_ORDER
        .iter()
        .copied()
        .filter(|flag| head.contains(&flag.as_str()))
        .collect()
}

/// R390: the at-risk cards — every card some tier's pass-1 numbers meet at half strength
/// (`half_flags`), and every card on `ban` or `watch` (today's SHADOW_BAN and SHADOW_WATCH by
/// default). A pure function of pass 1's results and the two tables, sorted.
///
/// TS's defaults: pass `SHADOW_BAN` and `SHADOW_WATCH`.
pub fn at_risk_ids(pass1: &[SweepResult], ban: &[(&str, &str)], watch: &[(&str, &str)]) -> Vec<String> {
    let mut ids: IndexSet<String> = ban
        .iter()
        .chain(watch.iter())
        .map(|(id, _)| (*id).to_string())
        .collect();
    for result in pass1 {
        if !half_flags(result).is_empty() {
            ids.insert(result.def_id.clone());
        }
    }
    let mut sorted: Vec<String> = ids.into_iter().collect();
    sorted.sort();
    sorted
}

/// R390: the cards pass 2's filler never deals — banned for `error` or `timeout` today, or flagged so
/// by pass 1 — so a known bug is never filler. A card banned for `neverPlayed`, `selfHarm` or
/// `selfKill` is at risk, and pass 2's filler lifts its ban.
///
/// TS's default: pass `SHADOW_BAN`.
pub fn pass2_keep_out(pass1: &[SweepResult], ban: &[(&str, &str)]) -> Vec<String> {
    let bug = |flags: &[SweepFlag]| -> bool {
        flags.contains(&SweepFlag::Error) || flags.contains(&SweepFlag::Timeout)
    };
    let mut ids: IndexSet<String> = ban
        .iter()
        .filter(|(_, reason)| bug(&ban_flags(reason)))
        .map(|(id, _)| (*id).to_string())
        .collect();
    for result in pass1 {
        if bug(&result.flags) {
            ids.insert(result.def_id.clone());
        }
    }
    let mut sorted: Vec<String> = ids.into_iter().collect();
    sorted.sort();
    sorted
}

/// R390's boost in pass 2's filler: these ids' weights multiplied by `by`.
#[derive(Clone, Debug, PartialEq)]
struct FillerBoost {
    ids: Vec<String>,
    by: i32,
}

/// What the AI's deck is dealt from in one sweep game, beside its forced card.
#[derive(Clone, Debug, PartialEq)]
struct Filler {
    banned: Vec<String>,
    boost: Option<FillerBoost>,
}

/// Game n (1-based) of a card's sweep: the AI sits p1 when n is odd and p2 when even.
fn swept_seat_of(n: i32) -> PlayerId {
    if n % 2 == 1 { PlayerId::P1 } else { PlayerId::P2 }
}

/// `sweepConfig`'s answer.
struct SweepSetup {
    config: MatchConfig,
    ai_seat: PlayerId,
}

/// Game n of a card's sweep at a tier: the AI on that tier's handicap, greedy on Easy's (a human's).
fn sweep_config(seed: &str, def_id: &str, n: i32, tier: Difficulty, filler: &Filler) -> SweepSetup {
    let ai_seat = swept_seat_of(n);
    let greedy_seat = opponent_of(ai_seat);
    let handicap = AI_DIFFICULTY[tier];
    let easy = AI_DIFFICULTY.easy;

    // The swept card is forced in even when it is banned today, so a rerun can clear it.
    let banned: Vec<&String> = filler.banned.iter().filter(|id| id.as_str() != def_id).collect();
    let mut ai_options = json!({
        "include": [def_id],
        "banned": banned,
        "manaCap": handicap.mana_cap,
    });
    if let Some(boost) = &filler.boost {
        ai_options["boost"] = json!({ "ids": boost.ids, "by": boost.by });
    }
    let ai_deck = build_ai_deck(
        &mut create_rng(&format!("{seed}:deck:{ai_seat}"), 0),
        handicap.deck_size,
        &json_as::<AiDeckOptions>(ai_options),
    );
    // The greedy seat stands in for a human, whose random deck ignores the AI's ban and its soft gate
    // (R1390).
    let greedy_deck = build_ai_deck(
        &mut create_rng(&format!("{seed}:deck:{greedy_seat}"), 0),
        easy.deck_size,
        &json_as::<AiDeckOptions>(json!({ "banned": [], "gatedSets": [], "manaCap": easy.mana_cap })),
    );

    let ai = SeatController::Ai {
        budget: Some(AI_GATE_BUDGET),
    };
    let greedy = SeatController::Greedy;
    let p1_is_ai = ai_seat == PlayerId::P1;
    SweepSetup {
        ai_seat,
        config: MatchConfig {
            seed: seed.to_string(),
            decks: if p1_is_ai {
                (ai_deck, greedy_deck)
            } else {
                (greedy_deck, ai_deck)
            },
            handicaps: Some(if p1_is_ai {
                PerPlayerOpt {
                    p1: Some(handicap),
                    p2: Some(easy),
                }
            } else {
                PerPlayerOpt {
                    p1: Some(easy),
                    p2: Some(handicap),
                }
            }),
            controllers: if p1_is_ai {
                PerPlayer::new(ai, greedy)
            } else {
                PerPlayer::new(greedy, ai)
            },
            max_actions: Some(AI_SWEEP.max_actions as _),
        },
    }
}

fn empty_stats(def_id: &str) -> SweepStats {
    SweepStats {
        def_id: def_id.to_string(),
        games: 0,
        drawn_games: 0,
        affordable_turns: 0,
        plays: 0,
        errors: 0,
        timeouts: 0,
        eval_delta_sum: 0.0,
        eval_delta_count: 0,
        losing_plays: 0,
        winning_plays: 0,
    }
}

/// `into` += `from`, field by field (defId stays).
fn add_stats(into: &mut SweepStats, from: &SweepStats) {
    into.games += from.games;
    into.drawn_games += from.drawn_games;
    into.affordable_turns += from.affordable_turns;
    into.plays += from.plays;
    into.errors += from.errors;
    into.timeouts += from.timeouts;
    into.eval_delta_sum += from.eval_delta_sum;
    into.eval_delta_count += from.eval_delta_count;
    into.losing_plays += from.losing_plays;
    into.winning_plays += from.winning_plays;
}

fn holds_card(state: &GameState, seat: PlayerId, def_id: &str) -> bool {
    zone_cards(state, seat, OffFieldZone::Hand)
        .iter()
        .any(|card| card.def_id == def_id)
}

/// `playSweepGame`'s answer.
struct SweepGame {
    cards: Vec<SweepStats>,
    errors: i32,
    timeouts: i32,
}

/// R390: one play of a card by `seat`, counted into its stats. Every play counts in `plays`. A play
/// from or into a finished game stays out of the mean evaluation change, since `evaluate` scores a
/// finished game ±AI_EVAL.win and one such play would decide the mean alone: `seat` winning on it
/// counts in `winning_plays`, losing on it in `losing_plays`, and a draw in neither.
pub fn record_sweep_play(stats: &mut SweepStats, before: &GameState, after: &GameState, seat: PlayerId) {
    stats.plays += 1;
    if before.result.is_some() {
        return;
    }
    match after.result.as_ref().map(|result| result.winner.player()) {
        None => {
            stats.eval_delta_sum += evaluate(after, seat, NextSwing::Enemy, &AI_EVAL)
                - evaluate(before, seat, NextSwing::Enemy, &AI_EVAL);
            stats.eval_delta_count += 1;
        }
        Some(Some(winner)) if winner == seat => stats.winning_plays += 1,
        Some(Some(_)) => stats.losing_plays += 1,
        Some(None) => {}
    }
}

/// One sweep game, counted for each `tracked` card the AI's deck holds (games 1): drawn (drawnGames
/// 1), the turns it sat in hand affordable (effectiveCost <= mana at the AI's turn start), and its
/// plays (`record_sweep_play`). The game's errors (throws, rejected or "fallback" AI actions; a game
/// that cannot even be dealt is one) and timeouts (a decision whose `now()` duration > decisionMs, or
/// maxActions hit) come back beside them.
fn play_sweep_game(
    seed: &str,
    def_id: &str,
    n: i32,
    tier: Difficulty,
    filler: &Filler,
    tracked: &[String],
    now: Option<&dyn Fn() -> f64>,
) -> SweepGame {
    let setup = match catch_unwind(AssertUnwindSafe(|| sweep_config(seed, def_id, n, tier, filler))) {
        Ok(setup) => setup,
        // The card cannot even be dealt (not a deck-legal card, or the pool is too small): an error.
        Err(_) => {
            return SweepGame {
                cards: vec![SweepStats {
                    games: 1,
                    ..empty_stats(def_id)
                }],
                errors: 1,
                timeouts: 0,
            };
        }
    };
    let SweepSetup { config, ai_seat } = setup;
    let dealt: &Vec<String> = if ai_seat == PlayerId::P1 {
        &config.decks.0
    } else {
        &config.decks.1
    };
    let mut cards: Vec<SweepStats> = tracked
        .iter()
        .filter(|id| dealt.contains(id))
        .map(|id| SweepStats {
            games: 1,
            ..empty_stats(id)
        })
        .collect();
    let mut errors = 0;
    let mut timeouts = 0;
    // The last AI turn whose start was already counted, so a turn is counted once however many
    // actions it holds.
    let mut counted_turn: Option<i32> = None;
    let mut slow_decisions = 0;

    let played = {
        let mut hooks = MatchHooks {
            after_action: Some(Box::new(
                |before: &GameState, after: &GameState, seat: PlayerId, action: &ActionBody| {
                    for stats in cards.iter_mut() {
                        if stats.drawn_games == 0
                            && (holds_card(before, ai_seat, &stats.def_id)
                                || holds_card(after, ai_seat, &stats.def_id))
                        {
                            stats.drawn_games = 1;
                        }
                    }

                    // The AI's turn start: the first main-phase state of a new AI turn.
                    if after.result.is_none()
                        && after.active == ai_seat
                        && after.phase == Phase::Main
                        && counted_turn != Some(after.turn)
                    {
                        counted_turn = Some(after.turn);
                        let mana = after.players[ai_seat].mana.current;
                        let hand = zone_cards(after, ai_seat, OffFieldZone::Hand);
                        for stats in cards.iter_mut() {
                            if hand.iter().any(|card| {
                                card.def_id == stats.def_id
                                    && effective_cost(after, card, CostOptions::default()) <= mana
                            }) {
                                stats.affordable_turns += 1;
                            }
                        }
                    }

                    if seat == ai_seat
                        && let ActionBody::Play { instance_id, .. } = action
                    {
                        let played_def = zone_cards(before, ai_seat, OffFieldZone::Hand)
                            .iter()
                            .find(|instance| instance.id == *instance_id)
                            .map(|card| card.def_id.clone());
                        if let Some(played_def) = played_def
                            && let Some(stats) = cards.iter_mut().find(|entry| entry.def_id == played_def)
                        {
                            record_sweep_play(stats, before, after, ai_seat);
                        }
                    }
                },
            )),
            ..MatchHooks::default()
        };
        if let Some(now) = now {
            let slow = &mut slow_decisions;
            hooks.time_decision = Some(Box::new(move |seat: PlayerId, run: &mut dyn FnMut()| {
                let started = now();
                run();
                if seat == ai_seat && now() - started > f64::from(AI_SWEEP.decision_ms) {
                    *slow += 1;
                }
            }));
        }
        catch_unwind(AssertUnwindSafe(|| play_match(&config, &mut hooks)))
    };
    timeouts += slow_decisions;

    match played {
        Ok(record) => {
            errors += record.thrown.len() as i32 + record.rejected.len() as i32 + record.fallbacks;
            // A game still running at maxActions is a timeout; one a throw ended is already an error.
            if record.result.is_none() && record.thrown.is_empty() {
                timeouts += 1;
            }
        }
        Err(_) => errors += 1,
    }
    SweepGame {
        cards,
        errors,
        timeouts,
    }
}

/// Pass 1: `seeds` games (default seedsPerCard) of an AI on `tier`'s handicap (default Easy) whose
/// deck includes `defId` against greedy on Easy's, at AI_GATE_BUDGET, on seeds
/// `sweep:<tier>:<id>:<n>`. Every other banned card stays out of the AI's deck, so its errors are not
/// charged to this one.
pub fn sweep_card(def_id: &str, options: &SweepOptions) -> SweepResult {
    let seeds = options.seeds.unwrap_or(AI_SWEEP.seeds_per_card);
    let tier = options.tier.unwrap_or(Difficulty::Easy);
    let mut stats = empty_stats(def_id);
    let filler = Filler {
        banned: SHADOW_BAN.iter().map(|(id, _)| (*id).to_string()).collect(),
        boost: None,
    };
    let tracked = [def_id.to_string()];

    for n in 1..=seeds {
        let game = play_sweep_game(
            &format!("sweep:{tier}:{def_id}:{n}"),
            def_id,
            n,
            tier,
            &filler,
            &tracked,
            options.now,
        );
        let card = game.cards.into_iter().next().unwrap_or_else(|| SweepStats {
            games: 1,
            ..empty_stats(def_id)
        });
        add_stats(
            &mut stats,
            &SweepStats {
                errors: game.errors,
                timeouts: game.timeouts,
                ..card
            },
        );
    }

    let flags = sweep_flags(&stats);
    let unswept = stats.affordable_turns == 0;
    SweepResult {
        stats,
        tier,
        flags,
        unswept,
    }
}

/// Pass 2 (R390): `seeds` games (default seedsPerCardAtRisk) of `defId`, one of the `atRisk` cards, on
/// seeds `sweep2:<tier>:<id>:<n>`. Every at-risk card's filler weight is multiplied by atRiskBoost,
/// `keepOut` (`pass2_keep_out`) is never filler, and every at-risk card the AI was dealt is counted.
pub fn sweep_at_risk(
    def_id: &str,
    at_risk: &[String],
    keep_out: &[String],
    options: &SweepOptions,
) -> SweepPass2 {
    let seeds = options.seeds.unwrap_or(AI_SWEEP.seeds_per_card_at_risk);
    let tier = options.tier.unwrap_or(Difficulty::Easy);
    let filler = Filler {
        banned: keep_out.to_vec(),
        boost: Some(FillerBoost {
            ids: at_risk.to_vec(),
            by: AI_SWEEP.at_risk_boost,
        }),
    };
    let tracked: Vec<String> = std::iter::once(def_id.to_string())
        .chain(at_risk.iter().filter(|id| id.as_str() != def_id).cloned())
        .collect();
    let mut totals: IndexMap<String, SweepStats> = IndexMap::new();
    totals.insert(def_id.to_string(), empty_stats(def_id));
    let mut suspects: Vec<SweepSuspect> = Vec::new();

    for n in 1..=seeds {
        let seed = format!("sweep2:{tier}:{def_id}:{n}");
        let game = play_sweep_game(&seed, def_id, n, tier, &filler, &tracked, options.now);
        for card in &game.cards {
            let forced = card.def_id == def_id;
            let total = totals
                .entry(card.def_id.clone())
                .or_insert_with(|| empty_stats(&card.def_id));
            add_stats(
                total,
                &SweepStats {
                    errors: if forced { game.errors } else { 0 },
                    timeouts: if forced { game.timeouts } else { 0 },
                    ..card.clone()
                },
            );
            if !forced && game.errors + game.timeouts > 0 {
                suspects.push(SweepSuspect {
                    def_id: card.def_id.clone(),
                    seed: seed.clone(),
                    forced: def_id.to_string(),
                    errors: game.errors,
                    timeouts: game.timeouts,
                });
            }
        }
    }

    SweepPass2 {
        forced: def_id.to_string(),
        tier,
        games: seeds,
        cards: totals.into_values().collect(),
        suspects,
    }
}

/// R390: a card's pass-2 numbers at one tier, summed over every game it was dealt in, forced or filler.
pub fn pass2_stats(pass2: &[SweepPass2], def_id: &str, tier: Difficulty) -> SweepStats {
    let mut total = empty_stats(def_id);
    for result in pass2 {
        if result.tier != tier {
            continue;
        }
        for card in &result.cards {
            if card.def_id == def_id {
                add_stats(&mut total, card);
            }
        }
    }
    total
}

/// What one flag measured, for the ban reason.
fn flag_detail(stats: &SweepStats, flag: SweepFlag, pass2: bool) -> String {
    let over = if pass2 {
        format!(" over {} pass-2 games", stats.games)
    } else {
        String::new()
    };
    match flag {
        SweepFlag::Error => format!(
            "{} engine or search error(s) over {} games",
            stats.errors, stats.games
        ),
        SweepFlag::Timeout => format!(
            "{} decision(s) over {} ms or game(s) past {} actions",
            stats.timeouts, AI_SWEEP.decision_ms, AI_SWEEP.max_actions
        ),
        SweepFlag::NeverPlayed => format!(
            "affordable in hand on {} turns{over}, never played",
            stats.affordable_turns
        ),
        SweepFlag::SelfHarm => format!(
            "mean evaluate change {} over {} play(s) that did not end the game{over}",
            to_fixed_1(mean(stats)),
            stats.eval_delta_count
        ),
        SweepFlag::SelfKill => format!(
            "lost the game on {} of {} play(s){over}",
            stats.losing_plays, stats.plays
        ),
    }
}

/// A card's numbers in a few words, for a SHADOW_WATCH entry.
fn numbers_of(stats: &SweepStats) -> String {
    let played = if stats.plays == 0 {
        "never played".to_string()
    } else {
        format!("played {} time(s)", stats.plays)
    };
    let harm = if stats.eval_delta_count == 0 {
        String::new()
    } else {
        format!(", mean evaluate change {}", to_fixed_1(mean(stats)))
    };
    let lost = if stats.losing_plays == 0 {
        String::new()
    } else {
        format!(", lost the game on {} play(s)", stats.losing_plays)
    };
    format!(
        "affordable on {} turns over {} games, {played}{harm}{lost}",
        stats.affordable_turns, stats.games
    )
}

/// One card's verdict over its tiers and passes (R186, R390). `pass1` is the card's own pass-1
/// results; `pass2` may be every pass-2 result of the sweep, since the card's numbers are read out of
/// each, forced or filler. Per tier: `error` and `timeout` from the card's own games of either pass;
/// `neverPlayed` from pass 2's numbers alone, banAffordableTurns affordable turns and no play;
/// `selfHarm` from pass 2's, at least banHarmPlays plays averaging below selfHarmDelta; `selfKill` from
/// pass 2's, at least banLosingPlays lost games. A flag at any tier bans the card, every flagging
/// tier named. Unswept when no tier of either pass ever saw it affordable. Watched (R600) when it is
/// not banned and its own numbers this sweep, of either pass, meet a flag at half strength.
///
/// TS's default `pass2 = []`: pass `&[]`.
pub fn sweep_verdict(pass1: &[SweepResult], pass2: &[SweepPass2]) -> SweepVerdict {
    let def_id = pass1
        .first()
        .map(|result| result.def_id.clone())
        .or_else(|| pass2.first().map(|result| result.forced.clone()))
        .unwrap_or_default();
    let tiers: IndexSet<Difficulty> = pass1
        .iter()
        .map(|result| result.tier)
        .chain(pass2.iter().map(|result| result.tier))
        .collect();
    let mut flagged: IndexSet<SweepFlag> = IndexSet::new();
    let mut details: Vec<String> = Vec::new();
    let mut watched: Vec<String> = Vec::new();
    let mut affordable = 0;

    for &tier in &tiers {
        let mut first = empty_stats(&def_id);
        for result in pass1 {
            if result.tier == tier {
                add_stats(&mut first, result);
            }
        }
        let second = pass2_stats(pass2, &def_id, tier);
        affordable += first.affordable_turns + second.affordable_turns;

        let mut both = first.clone();
        add_stats(&mut both, &second);
        let mut parts: Vec<String> = Vec::new();
        for flag in sweep_flags(&SweepStats {
            games: both.games,
            errors: both.errors,
            timeouts: both.timeouts,
            ..empty_stats(&def_id)
        }) {
            flagged.insert(flag);
            parts.push(flag_detail(&both, flag, false));
        }
        if second.affordable_turns >= AI_SWEEP.ban_affordable_turns && second.plays == 0 {
            flagged.insert(SweepFlag::NeverPlayed);
            parts.push(flag_detail(&second, SweepFlag::NeverPlayed, true));
        }
        if second.eval_delta_count >= AI_SWEEP.ban_harm_plays && mean(&second) < AI_SWEEP.self_harm_delta {
            flagged.insert(SweepFlag::SelfHarm);
            parts.push(flag_detail(&second, SweepFlag::SelfHarm, true));
        }
        if second.losing_plays >= AI_SWEEP.ban_losing_plays {
            flagged.insert(SweepFlag::SelfKill);
            parts.push(flag_detail(&second, SweepFlag::SelfKill, true));
        }
        if !parts.is_empty() {
            details.push(format!("{tier}: {}", parts.join(", ")));
        }

        let mut on_track: Vec<String> = Vec::new();
        if !half_flags(&first).is_empty() {
            on_track.push(format!("pass 1 {}", numbers_of(&first)));
        }
        if !half_flags(&second).is_empty() {
            on_track.push(format!("pass 2 {}", numbers_of(&second)));
        }
        if !on_track.is_empty() {
            watched.push(format!("{tier}: {}", on_track.join("; ")));
        }
    }

    let flags: Vec<SweepFlag> = FLAG_ORDER
        .iter()
        .copied()
        .filter(|flag| flagged.contains(flag))
        .collect();
    let reason = if flags.is_empty() {
        None
    } else {
        Some(format!(
            "{}: {}",
            flags
                .iter()
                .map(|flag| flag.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            details.join("; ")
        ))
    };
    let watch = if reason.is_none() && !watched.is_empty() {
        Some(format!("at risk: {}", watched.join("; ")))
    } else {
        None
    };
    SweepVerdict {
        def_id,
        flags,
        unswept: !tiers.is_empty() && affordable == 0,
        reason,
        watch,
    }
}
