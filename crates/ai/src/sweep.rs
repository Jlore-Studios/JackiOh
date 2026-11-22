//! The shadow-ban sweep (R186, R390, docs/polish/3-ai.md B25, docs/classic-sets.md B4.4, SURFACE §12):
//! forces cards into AI decks against greedy baselines across tiers (AI_SWEEP.tiers) to identify failures.
//!
//! Evaluates cards across two tiers (Easy, Hard) and two passes (R390) for flags (`neverPlayed`, `selfHarm`,
//! `selfKill`, `error`, `timeout`). Retimes slow decisions against perf gate (R977000). Each card is swept
//! on both faces (#4466): its ordinary one, and its Radiant one through a deck mark (R2702000), each
//! judged over its own games.

use std::ops::{Deref, DerefMut};
use std::panic::{AssertUnwindSafe, catch_unwind};

use indexmap::{IndexMap, IndexSet};
use jackioh_engine::prelude::json_as;
use jackioh_engine::{
    AI_DIFFICULTY, ActionBody, CostOptions, DeckMark, Difficulty, GameState, OffFieldZone, PerPlayer,
    PerPlayerOpt, Phase, PlayerId, Rng, create_rng, effective_cost, legal_actions, opponent_of, zone_cards,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::{AI_EVAL, AI_GATE_BUDGET};
use crate::deck::{AiDeckOptions, build_ai_deck};
use crate::evaluate::{NextSwing, evaluate};
use crate::match_::{MatchConfig, MatchHooks, MatchSetup, SeatController, play_match};
use crate::shadow_ban::SHADOW_BAN;
use crate::types::SearchBudget;

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
    /// The literal (as a SHADOW_BAN reason starts with it).
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

/// #4466: the face a sweep deals its forced card on: the ordinary one, or the Radiant one through a
/// deck mark (R2702000). Filler is always dealt on its base face.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SweepFace {
    #[default]
    Base,
    Radiant,
}

impl SweepFace {
    /// Every face, in the order a sweep plays them.
    pub const ALL: [SweepFace; 2] = [SweepFace::Base, SweepFace::Radiant];

    /// The literal, as a sweep line writes it.
    pub fn as_str(self) -> &'static str {
        match self {
            SweepFace::Base => "base",
            SweepFace::Radiant => "radiant",
        }
    }

    /// A base line writes no `face` key, so it reads as it did before faces existed.
    fn is_base(&self) -> bool {
        *self == SweepFace::Base
    }
}

impl std::fmt::Display for SweepFace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// `AI_SWEEP`'s shape (SURFACE §4.2: a constant object is a const of a struct named after it).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AiSweep {
    /// The tiers every card is swept at: the fewest resources and the most (see the header).
    pub tiers: &'static [Difficulty],
    /// Games per card, tier and face (#4466). Four left the neverPlayed flag at the mercy of one deal.
    pub seeds_per_card: i32,
    pub decision_ms: i32,
    pub max_actions: i32,
    /// R2425000: pass 1's neverPlayed needs this many playable turns.
    pub min_affordable_turns: i32,
    pub self_harm_delta: f64,
    /// selfHarm needs at least this many plays, so one play into a trap does not ban a card.
    pub min_harm_plays: i32,
    /// R390: pass 2's games per at-risk card, tier and face (#4466).
    pub seeds_per_card_at_risk: i32,
    /// R390: pass 2's filler draw multiplies every at-risk card's weight by this (as AI_DECK.themeBoost).
    /// A whole number, so `AiDeckOptions.boost.by` reads it whatever its number type.
    pub at_risk_boost: i32,
    /// R390, R2425000: a neverPlayed ban needs this many playable turns over pass 2's games, and no play.
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
    /// #4466: the face the card was dealt on. Absent on a base line, so an older line reads as base.
    #[serde(default, skip_serializing_if = "SweepFace::is_base")]
    pub face: SweepFace,
    pub games: i32,
    pub drawn_games: i32,
    pub affordable_turns: i32,
    /// R2425000: the AI's turn starts on which `legal_actions` listed a play of the card from its
    /// hand, the count `neverPlayed` reads. 0 in older slices.
    #[serde(default)]
    pub playable_turns: i32,
    pub plays: i32,
    pub errors: i32,
    pub timeouts: i32,
    /// R977000: the timeouts before the re-timer: every decision the clock found slow, and every
    /// game past maxActions. 0 in older slices.
    #[serde(default)]
    pub raw_timeouts: i32,
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
/// say nothing about whether the AI can play it (a report, never a ban flag). The stats are flattened
/// beside `tier`, `flags` and `unswept`, and the result reads as its stats (`Deref`).
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
    /// The SHADOW_BAN reason, `"<flags>: <tier>: <what it measured>; …"`, or null when unflagged; a
    /// Radiant face's numbers are named `<tier> (Radiant)` (#4466).
    pub reason: Option<String>,
    /// R390, R600: the SHADOW_WATCH entry (at risk by its own numbers, and not banned), or null.
    pub watch: Option<String>,
}

/// R977000: a decision the sweep's clock found slower than AI_SWEEP.decisionMs, as the AI faced it:
/// deciding `state` for `seat` at `budget` on a clone of `rng` makes the same decision again.
#[derive(Clone, Copy)]
pub struct SlowDecision<'a> {
    pub state: &'a GameState,
    pub seat: PlayerId,
    pub budget: SearchBudget,
    /// The AI's stream before the decision, as `play_match` hands it to the timing hook.
    pub rng: &'a Rng,
}

/// `sweepCard`'s and `sweepAtRisk`'s options, with R977000's `retime`.
#[derive(Clone, Copy, Default)]
pub struct SweepOptions<'a> {
    pub seeds: Option<i32>,
    /// The clock, in milliseconds; absent, no decision ever times out.
    pub now: Option<&'a dyn Fn() -> f64>,
    /// R977000: whether a decision the clock found slow is still slower than decisionMs timed again
    /// (`cargo jackioh sweep`: against the perf gate's yardstick); absent, every one counts.
    pub retime: Option<&'a dyn Fn(&SlowDecision<'_>) -> bool>,
    pub tier: Option<Difficulty>,
    /// Issue #1113: the ids `sweep_card` keeps out of the AI's deck; absent, `SHADOW_BAN`'s. A seeded
    /// test passes `FIXTURE_BANNED_IDS`, so a shadow-ban change re-deals none of its games.
    pub banned: Option<&'a [&'a str]>,
    /// #1697: the setup inputs every game of the sweep is created with beyond its decks (§9.3), in
    /// seat order and the same in every game, whichever seat the AI takes (to give the AI a last
    /// board in every game, give both seats one). Absent, every game is created as before.
    pub setup: Option<&'a MatchSetup>,
    /// #4466: the face the forced card is dealt on; absent, its base face.
    pub face: Option<SweepFace>,
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

/// error: errors > 0; timeout: timeouts > 0; neverPlayed: playableTurns >= minAffordableTurns && plays === 0 (R2425000);
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
    if stats.playable_turns >= AI_SWEEP.min_affordable_turns && stats.plays == 0 {
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

/// R390: the judgement flags at half strength, which put a card at risk: `neverPlayed` when it was playable on minAffordableTurns turns (R2425000) and was played at most once, `selfHarm` when its plays'
/// mean evaluation change is below half of selfHarmDelta, `selfKill` on half of minLosingPlays lost
/// games (one).
pub fn half_flags(stats: &SweepStats) -> Vec<SweepFlag> {
    let mut flags = Vec::new();
    if stats.playable_turns >= AI_SWEEP.min_affordable_turns && stats.plays <= 1 {
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

/// Game n of a card's sweep at a tier: the AI on that tier's handicap, greedy on Easy's (a human's),
/// the forced card dealt on `face`.
fn sweep_config(
    seed: &str,
    def_id: &str,
    n: i32,
    tier: Difficulty,
    filler: &Filler,
    setup: &MatchSetup,
    face: SweepFace,
) -> SweepSetup {
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

    // #4466: the Radiant face deals the forced card Radiant through its seat's deck mark (R2702000). An
    // AI deck holds distinct ids, so the forced id has one index.
    let mut setup = setup.clone();
    if face == SweepFace::Radiant
        && let Some(index) = ai_deck.iter().position(|id| id.as_str() == def_id)
    {
        let marks = setup
            .deck_marks
            .get_or_insert_with(PerPlayerOpt::default)
            .slot(ai_seat)
            .get_or_insert_with(Vec::new);
        if let Some(mark) = marks.iter_mut().find(|mark| mark.index == index) {
            // A mark the setup carries keeps its Buffs.
            mark.radiant = true;
        } else {
            marks.push(DeckMark {
                index,
                radiant: true,
                upgrades: 0,
            });
        }
    }

    let ai = SeatController::Ai {
        budget: Some(AI_GATE_BUDGET),
        skill: None,
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
            setup,
        },
    }
}

fn empty_stats(def_id: &str) -> SweepStats {
    SweepStats {
        def_id: def_id.to_string(),
        face: SweepFace::Base,
        games: 0,
        drawn_games: 0,
        affordable_turns: 0,
        playable_turns: 0,
        plays: 0,
        errors: 0,
        timeouts: 0,
        raw_timeouts: 0,
        eval_delta_sum: 0.0,
        eval_delta_count: 0,
        losing_plays: 0,
        winning_plays: 0,
    }
}

fn empty_face_stats(def_id: &str, face: SweepFace) -> SweepStats {
    SweepStats {
        face,
        ..empty_stats(def_id)
    }
}

/// Game n's seed of a card's sweep at a tier on a face (`prefix` "sweep" in pass 1, "sweep2" in pass
/// 2): the base face's as before faces existed, the Radiant face's seeds of its own (#4466).
fn sweep_seed(prefix: &str, tier: Difficulty, def_id: &str, face: SweepFace, n: i32) -> String {
    match face {
        SweepFace::Base => format!("{prefix}:{tier}:{def_id}:{n}"),
        SweepFace::Radiant => format!("{prefix}:{tier}:{def_id}:radiant:{n}"),
    }
}

/// A tier and face as a reason or a watch entry names them: the base face by its tier alone.
fn face_label(tier: Difficulty, face: SweepFace) -> String {
    match face {
        SweepFace::Base => format!("{tier}"),
        SweepFace::Radiant => format!("{tier} (Radiant)"),
    }
}

/// The tiers a card's results were swept at, in the order they first appear.
fn tiers_of(pass1: &[SweepResult], pass2: &[SweepPass2]) -> IndexSet<Difficulty> {
    pass1
        .iter()
        .map(|result| result.tier)
        .chain(pass2.iter().map(|result| result.tier))
        .collect()
}

/// `into` += `from`, field by field (defId and face stay).
fn add_stats(into: &mut SweepStats, from: &SweepStats) {
    into.games += from.games;
    into.drawn_games += from.drawn_games;
    into.affordable_turns += from.affordable_turns;
    into.playable_turns += from.playable_turns;
    into.plays += from.plays;
    into.errors += from.errors;
    into.timeouts += from.timeouts;
    into.raw_timeouts += from.raw_timeouts;
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
    raw_timeouts: i32,
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

/// R2425000: one AI turn start, counted into each of `cards`. A card `seat` holds an instance of at
/// an `effective_cost` within its mana counts an affordable turn; a card one of whose instances in
/// hand `legal_actions` lists a `Play` of (the action `play_sweep_game` counts as a play) counts a
/// playable turn. `legal_actions` runs only when one of `cards` is in hand.
pub fn record_sweep_turn(cards: &mut [SweepStats], state: &GameState, seat: PlayerId) {
    let mana = state.players[seat].mana.current;
    let hand = zone_cards(state, seat, OffFieldZone::Hand);
    if !cards
        .iter()
        .any(|stats| hand.iter().any(|card| card.def_id == stats.def_id))
    {
        return;
    }
    let playable: Vec<&str> = legal_actions(state, seat)
        .iter()
        .filter_map(|action| match action {
            ActionBody::Play { instance_id, .. } => hand
                .iter()
                .find(|card| card.id == *instance_id)
                .map(|card| card.def_id.as_str()),
            _ => None,
        })
        .collect();
    for stats in cards.iter_mut() {
        if hand.iter().any(|card| {
            card.def_id == stats.def_id && effective_cost(state, card, CostOptions::default()) <= mana
        }) {
            stats.affordable_turns += 1;
        }
        if playable.contains(&stats.def_id.as_str()) {
            stats.playable_turns += 1;
        }
    }
}

/// Simulates one sweep game, recording affordable/playable turns (R2425000), plays, errors (#1429),
/// and timeouts (retimed via R977000).
fn play_sweep_game(
    seed: &str,
    def_id: &str,
    n: i32,
    tier: Difficulty,
    filler: &Filler,
    tracked: &[String],
    options: &SweepOptions,
) -> SweepGame {
    let no_setup = MatchSetup::default();
    let inputs = options.setup.unwrap_or(&no_setup);
    let face = options.face.unwrap_or_default();
    let setup = match catch_unwind(AssertUnwindSafe(|| {
        sweep_config(seed, def_id, n, tier, filler, inputs, face)
    })) {
        Ok(setup) => setup,
        // The card cannot even be dealt (not a deck-legal card, or the pool is too small): an error.
        Err(_) => {
            return SweepGame {
                cards: vec![SweepStats {
                    games: 1,
                    ..empty_face_stats(def_id, face)
                }],
                errors: 1,
                timeouts: 0,
                raw_timeouts: 0,
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
            // Only the forced card is dealt on the sweep's face; filler is dealt on its base one.
            ..empty_face_stats(
                id,
                if id.as_str() == def_id {
                    face
                } else {
                    SweepFace::Base
                },
            )
        })
        .collect();
    let mut errors = 0;
    let mut timeouts = 0;
    let mut raw_timeouts = 0;
    // The last AI turn whose start was already counted, so a turn is counted once however many
    // actions it holds.
    let mut counted_turn: Option<i32> = None;
    let mut slow_decisions = 0;
    let mut raw_slow_decisions = 0;

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
                        record_sweep_turn(&mut cards, after, ai_seat);
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
        if let Some(now) = options.now {
            let retime = options.retime;
            let slow = &mut slow_decisions;
            let raw = &mut raw_slow_decisions;
            hooks.time_decision = Some(Box::new(
                move |state: &GameState, seat: PlayerId, rng: &Rng, run: &mut dyn FnMut()| {
                    let started = now();
                    run();
                    if seat == ai_seat && now() - started > f64::from(AI_SWEEP.decision_ms) {
                        *raw += 1;
                        // R977000: the re-timer has the last word; without one the clock's stands.
                        // AI_GATE_BUDGET is the budget `sweep_config` seats the AI at.
                        let position = SlowDecision {
                            state,
                            seat,
                            budget: AI_GATE_BUDGET,
                            rng,
                        };
                        if retime.is_none_or(|retime| retime(&position)) {
                            *slow += 1;
                        }
                    }
                },
            ));
        }
        catch_unwind(AssertUnwindSafe(|| play_match(&config, &mut hooks)))
    };
    timeouts += slow_decisions;
    raw_timeouts += raw_slow_decisions;

    match played {
        Ok(record) => {
            errors += record.thrown.len() as i32
                + record.rejected.len() as i32
                + record.fallbacks
                + record.sim_panics as i32;
            // A game still running at maxActions is a timeout; one a throw ended is already an error.
            if record.result.is_none() && record.thrown.is_empty() {
                timeouts += 1;
                raw_timeouts += 1;
            }
        }
        Err(_) => errors += 1,
    }
    SweepGame {
        cards,
        errors,
        timeouts,
        raw_timeouts,
    }
}

/// Pass 1: `seeds` games (default seedsPerCard) of an AI on `tier`'s handicap (default Easy) whose
/// deck includes `defId` on `face` (default base) against greedy on Easy's, at AI_GATE_BUDGET, on seeds
/// `sweep:<tier>:<id>:<n>` (the Radiant face's `sweep:<tier>:<id>:radiant:<n>`). Every other banned
/// card (`banned`, default `SHADOW_BAN`) stays out of the AI's deck, so its errors are not charged to
/// this one.
pub fn sweep_card(def_id: &str, options: &SweepOptions) -> SweepResult {
    let seeds = options.seeds.unwrap_or(AI_SWEEP.seeds_per_card);
    let tier = options.tier.unwrap_or(Difficulty::Easy);
    let face = options.face.unwrap_or_default();
    let mut stats = empty_face_stats(def_id, face);
    let banned: Vec<&str> = options.banned.map_or_else(
        || SHADOW_BAN.iter().map(|(id, _)| *id).collect(),
        <[&str]>::to_vec,
    );
    let filler = Filler {
        banned: banned.into_iter().map(str::to_string).collect(),
        boost: None,
    };
    let tracked = [def_id.to_string()];

    for n in 1..=seeds {
        let game = play_sweep_game(
            &sweep_seed("sweep", tier, def_id, face, n),
            def_id,
            n,
            tier,
            &filler,
            &tracked,
            options,
        );
        let card = game.cards.into_iter().next().unwrap_or_else(|| SweepStats {
            games: 1,
            ..empty_face_stats(def_id, face)
        });
        add_stats(
            &mut stats,
            &SweepStats {
                errors: game.errors,
                timeouts: game.timeouts,
                raw_timeouts: game.raw_timeouts,
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
/// `face` (default base), on seeds `sweep2:<tier>:<id>:<n>` (the Radiant face's
/// `sweep2:<tier>:<id>:radiant:<n>`). Every at-risk card's filler weight is multiplied by atRiskBoost,
/// `keepOut` (`pass2_keep_out`) is never filler, and every at-risk card the AI was dealt is counted,
/// filler on its base face.
pub fn sweep_at_risk(
    def_id: &str,
    at_risk: &[String],
    keep_out: &[String],
    options: &SweepOptions,
) -> SweepPass2 {
    let seeds = options.seeds.unwrap_or(AI_SWEEP.seeds_per_card_at_risk);
    let tier = options.tier.unwrap_or(Difficulty::Easy);
    let face = options.face.unwrap_or_default();
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
    totals.insert(def_id.to_string(), empty_face_stats(def_id, face));
    let mut suspects: Vec<SweepSuspect> = Vec::new();

    for n in 1..=seeds {
        let seed = sweep_seed("sweep2", tier, def_id, face, n);
        let game = play_sweep_game(&seed, def_id, n, tier, &filler, &tracked, options);
        for card in &game.cards {
            let forced = card.def_id == def_id;
            let total = totals
                .entry(card.def_id.clone())
                .or_insert_with(|| empty_face_stats(&card.def_id, card.face));
            add_stats(
                total,
                &SweepStats {
                    errors: if forced { game.errors } else { 0 },
                    timeouts: if forced { game.timeouts } else { 0 },
                    raw_timeouts: if forced { game.raw_timeouts } else { 0 },
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

/// R390: a card's pass-2 numbers at one tier on one face (#4466), summed over every game it was dealt
/// in on that face, forced or filler (filler is always dealt on its base face).
pub fn pass2_stats(pass2: &[SweepPass2], def_id: &str, tier: Difficulty, face: SweepFace) -> SweepStats {
    let mut total = empty_face_stats(def_id, face);
    for result in pass2 {
        if result.tier != tier {
            continue;
        }
        for card in &result.cards {
            if card.def_id == def_id && card.face == face {
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
            "affordable in hand on {} turns and playable on {}{over}, never played",
            stats.affordable_turns, stats.playable_turns
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
        "affordable on {} turns and playable on {} over {} games, {played}{harm}{lost}",
        stats.affordable_turns, stats.playable_turns, stats.games
    )
}

/// #4466: a card's numbers at one tier on one face, and the flags they raise, each with what it
/// measured.
struct FaceJudgement {
    tier: Difficulty,
    face: SweepFace,
    first: SweepStats,
    second: SweepStats,
    raised: Vec<(SweepFlag, String)>,
}

/// A card judged at every tier it was swept at, on each face (#4466), every flag over that face's own
/// games: error and timeout over both passes, neverPlayed, selfHarm and selfKill over pass 2 alone
/// (R390, R2425000, R601). A face with no games raises nothing.
fn judge_card(def_id: &str, pass1: &[SweepResult], pass2: &[SweepPass2]) -> Vec<FaceJudgement> {
    let mut judged = Vec::new();
    for tier in tiers_of(pass1, pass2) {
        for face in SweepFace::ALL {
            let mut first = empty_face_stats(def_id, face);
            for result in pass1 {
                if result.tier == tier && result.face == face {
                    add_stats(&mut first, result);
                }
            }
            let second = pass2_stats(pass2, def_id, tier, face);

            let mut both = first.clone();
            add_stats(&mut both, &second);
            let mut raised: Vec<(SweepFlag, String)> = Vec::new();
            for flag in sweep_flags(&SweepStats {
                games: both.games,
                errors: both.errors,
                timeouts: both.timeouts,
                ..empty_face_stats(def_id, face)
            }) {
                raised.push((flag, flag_detail(&both, flag, false)));
            }
            if second.playable_turns >= AI_SWEEP.ban_affordable_turns && second.plays == 0 {
                raised.push((
                    SweepFlag::NeverPlayed,
                    flag_detail(&second, SweepFlag::NeverPlayed, true),
                ));
            }
            if second.eval_delta_count >= AI_SWEEP.ban_harm_plays && mean(&second) < AI_SWEEP.self_harm_delta
            {
                raised.push((
                    SweepFlag::SelfHarm,
                    flag_detail(&second, SweepFlag::SelfHarm, true),
                ));
            }
            if second.losing_plays >= AI_SWEEP.ban_losing_plays {
                raised.push((
                    SweepFlag::SelfKill,
                    flag_detail(&second, SweepFlag::SelfKill, true),
                ));
            }
            judged.push(FaceJudgement {
                tier,
                face,
                first,
                second,
                raised,
            });
        }
    }
    judged
}

/// #4466: the flags a card's numbers on one face raise at any tier, in a reason's order.
pub fn face_flags(
    def_id: &str,
    pass1: &[SweepResult],
    pass2: &[SweepPass2],
    face: SweepFace,
) -> Vec<SweepFlag> {
    let raised: IndexSet<SweepFlag> = judge_card(def_id, pass1, pass2)
        .into_iter()
        .filter(|judged| judged.face == face)
        .flat_map(|judged| judged.raised.into_iter().map(|(flag, _)| flag))
        .collect();
    FLAG_ORDER
        .iter()
        .copied()
        .filter(|flag| raised.contains(flag))
        .collect()
}

/// Card's shadow-ban sweep verdict across tiers, faces and passes (R186, R390, R2425000, R600, #4466).
pub fn sweep_verdict(pass1: &[SweepResult], pass2: &[SweepPass2]) -> SweepVerdict {
    let def_id = pass1
        .first()
        .map(|result| result.def_id.clone())
        .or_else(|| pass2.first().map(|result| result.forced.clone()))
        .unwrap_or_default();
    let tiers = tiers_of(pass1, pass2);
    let mut flagged: IndexSet<SweepFlag> = IndexSet::new();
    let mut details: Vec<String> = Vec::new();
    let mut watched: Vec<String> = Vec::new();
    let mut affordable = 0;

    for judged in judge_card(&def_id, pass1, pass2) {
        let FaceJudgement {
            tier,
            face,
            first,
            second,
            raised,
        } = judged;
        let label = face_label(tier, face);
        affordable += first.affordable_turns + second.affordable_turns;
        let mut parts: Vec<String> = Vec::new();
        for (flag, detail) in raised {
            flagged.insert(flag);
            parts.push(detail);
        }
        if !parts.is_empty() {
            details.push(format!("{label}: {}", parts.join(", ")));
        }

        let mut on_track: Vec<String> = Vec::new();
        if !half_flags(&first).is_empty() {
            on_track.push(format!("pass 1 {}", numbers_of(&first)));
        }
        if !half_flags(&second).is_empty() {
            on_track.push(format!("pass 2 {}", numbers_of(&second)));
        }
        if !on_track.is_empty() {
            watched.push(format!("{label}: {}", on_track.join("; ")));
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

#[cfg(test)]
mod tests {
    use jackioh_engine::{CreateGameOptions, LastBoardEntry, begin_game, create_game};

    use super::*;

    const PORTAL: &str = "classicplus-029";

    #[test]
    fn a_sweep_game_carries_the_sweeps_setup_inputs_and_deals_as_without_them() {
        jackioh_cards::register_all();
        let board = vec![LastBoardEntry {
            def_id: "core-012".to_string(),
            radiant: false,
        }];
        let setup = MatchSetup {
            last_boards: Some((board.clone(), board)),
            ..MatchSetup::default()
        };
        let filler = Filler {
            banned: vec![],
            boost: None,
        };
        for n in [1, 2] {
            let seed = format!("sweep:easy:{PORTAL}:{n}");
            let plain = sweep_config(
                &seed,
                PORTAL,
                n,
                Difficulty::Easy,
                &filler,
                &MatchSetup::default(),
                SweepFace::Base,
            );
            let carried = sweep_config(
                &seed,
                PORTAL,
                n,
                Difficulty::Easy,
                &filler,
                &setup,
                SweepFace::Base,
            );
            assert_eq!(plain.config.setup, MatchSetup::default(), "game {n}");
            assert_eq!(carried.config.setup, setup, "game {n}");
            assert_eq!(carried.ai_seat, plain.ai_seat, "game {n}");
            assert_eq!(
                MatchConfig {
                    setup: MatchSetup::default(),
                    ..carried.config
                },
                plain.config,
                "game {n}: only the setup differs"
            );
        }
    }

    fn no_filler() -> Filler {
        Filler {
            banned: vec![],
            boost: None,
        }
    }

    /// Game n of PORTAL's Easy sweep on `face`, on the Radiant face's seed, from `setup`.
    fn portal_config(n: i32, setup: &MatchSetup, face: SweepFace) -> SweepSetup {
        let seed = sweep_seed("sweep", Difficulty::Easy, PORTAL, SweepFace::Radiant, n);
        sweep_config(&seed, PORTAL, n, Difficulty::Easy, &no_filler(), setup, face)
    }

    fn ai_deck_of(setup: &SweepSetup) -> &Vec<String> {
        if setup.ai_seat == PlayerId::P1 {
            &setup.config.decks.0
        } else {
            &setup.config.decks.1
        }
    }

    /// The game a config begins, as `play_match` creates it.
    fn begun(config: &MatchConfig) -> GameState {
        let created = create_game(&CreateGameOptions {
            seed: config.seed.clone(),
            decks: config.decks.clone(),
            handicaps: config.handicaps.clone(),
            deck_marks: config.setup.deck_marks.clone(),
            ..CreateGameOptions::default()
        });
        begin_game(&created).state
    }

    #[test]
    fn the_radiant_face_deals_the_forced_card_radiant_and_the_base_face_does_not() {
        jackioh_cards::register_all();
        for n in [1, 2] {
            let base = portal_config(n, &MatchSetup::default(), SweepFace::Base);
            let radiant = portal_config(n, &MatchSetup::default(), SweepFace::Radiant);
            assert_eq!(base.config.setup.deck_marks, None, "game {n}");
            assert_eq!(radiant.ai_seat, base.ai_seat, "game {n}");
            assert_eq!(radiant.config.decks, base.config.decks, "game {n}");
            let index = ai_deck_of(&radiant)
                .iter()
                .position(|id| id == PORTAL)
                .unwrap_or_else(|| panic!("game {n}: the forced card is dealt"));
            let marks = radiant.config.setup.deck_marks.clone().unwrap_or_default();
            assert_eq!(
                marks.get(radiant.ai_seat),
                Some(&vec![DeckMark {
                    index,
                    radiant: true,
                    upgrades: 0
                }]),
                "game {n}"
            );
            assert_eq!(marks.get(opponent_of(radiant.ai_seat)), None, "game {n}");

            for (setup, radiant_face) in [(&base, false), (&radiant, true)] {
                let state = begun(&setup.config);
                for seat in [PlayerId::P1, PlayerId::P2] {
                    let dealt: Vec<_> = [OffFieldZone::Hand, OffFieldZone::Library]
                        .into_iter()
                        .flat_map(|zone| zone_cards(&state, seat, zone))
                        .collect();
                    let forced = dealt.iter().filter(|card| card.def_id == PORTAL).count();
                    let radiant_cards: Vec<&str> = dealt
                        .iter()
                        .filter(|card| card.radiant)
                        .map(|card| card.def_id.as_str())
                        .collect();
                    if seat == setup.ai_seat {
                        assert_eq!(forced, 1, "game {n}: the AI holds the forced card once");
                        let want: Vec<&str> = if radiant_face { vec![PORTAL] } else { vec![] };
                        assert_eq!(radiant_cards, want, "game {n}, Radiant face {radiant_face}");
                    } else {
                        assert!(
                            radiant_cards.is_empty(),
                            "game {n}: the greedy seat holds no Radiant card"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_radiant_face_keeps_a_mark_the_setup_carries() {
        jackioh_cards::register_all();
        let plain = portal_config(1, &MatchSetup::default(), SweepFace::Radiant);
        let ai_seat = plain.ai_seat;
        let index = ai_deck_of(&plain)
            .iter()
            .position(|id| id == PORTAL)
            .expect("the forced card is dealt");
        let other = DeckMark {
            index: 0,
            radiant: false,
            upgrades: 1,
        };
        let mut marks = PerPlayerOpt::default();
        *marks.slot(ai_seat) = Some(vec![DeckMark {
            index,
            radiant: false,
            upgrades: 2,
        }]);
        *marks.slot(opponent_of(ai_seat)) = Some(vec![other]);
        let setup = MatchSetup {
            deck_marks: Some(marks),
            ..MatchSetup::default()
        };

        let radiant = portal_config(1, &setup, SweepFace::Radiant);
        let got = radiant.config.setup.deck_marks.clone().unwrap_or_default();
        assert_eq!(
            got.get(ai_seat),
            Some(&vec![DeckMark {
                index,
                radiant: true,
                upgrades: 2
            }])
        );
        assert_eq!(got.get(opponent_of(ai_seat)), Some(&vec![other]));
        assert_eq!(portal_config(1, &setup, SweepFace::Base).config.setup, setup);
    }

    #[test]
    fn the_radiant_face_plays_on_seeds_of_its_own() {
        let id = "core-011";
        assert_eq!(
            sweep_seed("sweep", Difficulty::Easy, id, SweepFace::Base, 3),
            "sweep:easy:core-011:3"
        );
        assert_eq!(
            sweep_seed("sweep", Difficulty::Easy, id, SweepFace::Radiant, 3),
            "sweep:easy:core-011:radiant:3"
        );
        assert_eq!(
            sweep_seed("sweep2", Difficulty::Hard, id, SweepFace::Radiant, 3),
            "sweep2:hard:core-011:radiant:3"
        );
    }
}
