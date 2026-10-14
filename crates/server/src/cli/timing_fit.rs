//! Admin script: log-normal fits of human think times, per bucket, and the rest of the human habits
//! a ladder bot copies, to paste into config (SPEC §9.11, R1442, R639000).
//!
//! ```text
//! jackioh-server timing-fit
//! ```
//!
//! Reads every row of `public.action_timings` (DATABASE_URL), leaves out the moves an AI made
//! (`pilot = 'ai'`), groups the rest by rank bucket, action kind and whether the move was the
//! seat's first of its turn, and fits each group of at least `TIMING_FIT_MIN_SAMPLES` moves with a
//! log-normal: the maximum-likelihood `mu` and `sigma` of `ln(think_ms)`. It prints one
//! `ThinkTimeFit` per bucket, to be pasted into `THINK_TIME_FITS` in `crates/server/src/config.rs`
//! by hand, as `cargo jackioh sweep` prints its rows (CLAUDE.md rule 9): no number reaches the
//! ladder bots (#636) without a person reading it first. Every row is read into memory, which is
//! fine at today's scale.
//!
//! R639000: it then prints, from the same people's rows of `action_timings`, `emote_events` and
//! `match_signals` (each signals row beside its match's result), one row for each of the other fits
//! a ladder bot reads in `config.rs`: how much longer a busier board and a lethal turn take
//! (`THINK_COMPLEXITY_SLOPE`, `THINK_LETHAL_SHIFT`), how people concede, answer a draw offer, run
//! their clock out and rematch (`BOT_CONCEDE_FIT`, `BOT_DRAW_ANSWER_FIT`, `BOT_TIMEOUT_FIT`,
//! `BOT_REMATCH_FIT`), how long their emotes wait (`BOT_EMOTE_DELAY_FIT`,
//! `BOT_EMOTE_REPLY_DELAY_FIT`) and which persona their emoting reads as (`BOT_PERSONA_WEIGHTS`).
//! A fit with fewer than `TIMING_FIT_MIN_SAMPLES` observations behind it prints a comment instead,
//! and the row's prior stays.

use anyhow::bail;
use indexmap::IndexMap;

use jackioh_engine::wire::GameOverReason;
use jackioh_engine::{ActionType, PlayerId};

use crate::bots::persona::reply_key_of;
use crate::cli::card_stats::literal;
use crate::cli::{connect_one, database_url};
use crate::config::{
    ConcedeFit, DrawAnswerFit, EmoteDelayFit, EmoteReplyKey, PERSONA_NAMES, PersonaName, PersonaWeights,
    RematchFit, ScalarFit, TIMING_FIT_CONCEDE_TURN_QUANTILE, TIMING_FIT_DELAY_QUANTILES,
    TIMING_FIT_MIN_SAMPLES, TimeoutFit,
};
use crate::db::store::{ActionTimingRow, Db, EmoteEventRow, Pilot, SignalOutcomeRow, StoreError};
use crate::env::quoted;
use crate::ranked::ladder::RankTier;

const USAGE: &str = "Usage: jackioh-server timing-fit

  Prints log-normal fits of the human think times in action_timings (R1442), one ThinkTimeFit row
  per bucket, to paste into THINK_TIME_FITS in crates/server/src/config.rs, then the ladder bots'
  other rows fitted from people's play (R639000): the complexity slope and lethal shift, the
  concession, draw-answer, timeout and rematch rates, the emote delay windows and the persona
  weights, each to paste over the row of its name there. Takes no options; reads DATABASE_URL.";

/// The comment line the printed rows open with.
const HEADER: &str = "// jackioh-server timing-fit: paste into THINK_TIME_FITS, crates/server/src/config.rs";

/// The comment line the R639000 rows open with.
const SIGNALS_HEADER: &str =
    "// jackioh-server timing-fit: paste each over the row of its name, crates/server/src/config.rs";

/// One bucket's fit (`config::ThinkTimeFit`'s fields, as values).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BucketFit {
    pub rank_bucket: Option<RankTier>,
    pub action_kind: ActionType,
    pub first_in_turn: bool,
    pub samples: usize,
    pub mu: f64,
    pub sigma: f64,
    /// R639000: the mean of `ln(legal_count)` over the bucket's moves.
    pub ln_legal_mean: f64,
}

/// A bucket's key: rank, action kind, first move of the turn.
type BucketKey = (Option<RankTier>, ActionType, bool);

/// `ln(think_ms)` and `ln(legal_count)` of one human move, each floored at 1 so the logarithm is
/// defined.
fn logs_of(row: &ActionTimingRow) -> (f64, f64) {
    (
        (row.think_ms.max(1) as f64).ln(),
        (f64::from(row.legal_count.max(1))).ln(),
    )
}

/// The human moves by bucket, in key order, each `logs_of`.
fn buckets_of(rows: &[ActionTimingRow]) -> IndexMap<BucketKey, Vec<(f64, f64)>> {
    let mut buckets: IndexMap<BucketKey, Vec<(f64, f64)>> = IndexMap::new();
    for row in rows.iter().filter(|row| row.pilot != Pilot::Ai) {
        buckets
            .entry((row.rank_bucket, row.action_kind, row.first_in_turn))
            .or_default()
            .push(logs_of(row));
    }
    buckets.sort_keys();
    buckets
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

/// The population standard deviation of `values` about `mu`.
fn deviation(values: &[f64], mu: f64) -> f64 {
    (values.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / values.len() as f64).sqrt()
}

/// The `q` quantile of `sorted`, the value at `floor((len - 1) * q)`; `sorted` is not empty.
fn quantile<T: Copy>(sorted: &[T], q: f64) -> T {
    let at = ((sorted.len() - 1) as f64 * q).floor() as usize;
    sorted[at.min(sorted.len() - 1)]
}

/// R1442: every bucket of human moves with at least `min_samples` of them, in key order (no rank
/// first, then the tiers lowest first; the action kinds in `ActionType`'s order; later moves before
/// first ones). A think time of 0 counts as 1 ms, so its logarithm is defined.
pub fn fit_think_times(rows: &[ActionTimingRow], min_samples: usize) -> Vec<BucketFit> {
    buckets_of(rows)
        .into_iter()
        .filter(|(_, logs)| logs.len() >= min_samples)
        .map(|((rank_bucket, action_kind, first_in_turn), logs)| {
            let thinks: Vec<f64> = logs.iter().map(|(think, _)| *think).collect();
            let legals: Vec<f64> = logs.iter().map(|(_, legal)| *legal).collect();
            let mu = mean(&thinks);
            BucketFit {
                rank_bucket,
                action_kind,
                first_in_turn,
                samples: logs.len(),
                mu,
                sigma: deviation(&thinks, mu),
                ln_legal_mean: mean(&legals),
            }
        })
        .collect()
}

/// R639000: how much `ln(think_ms)` grows per unit of `ln(legal_count)`: the least-squares slope
/// pooled within the buckets `fit_think_times` fits (each move measured from its own bucket's
/// means), over every move in them. `None` with fewer than `min_samples` moves, or when no bucket's
/// legal counts vary.
pub fn fit_complexity_slope(rows: &[ActionTimingRow], min_samples: usize) -> Option<ScalarFit> {
    let (mut covariance, mut variance, mut samples) = (0.0, 0.0, 0);
    for logs in buckets_of(rows).values().filter(|logs| logs.len() >= min_samples) {
        let thinks: Vec<f64> = logs.iter().map(|(think, _)| *think).collect();
        let legals: Vec<f64> = logs.iter().map(|(_, legal)| *legal).collect();
        let (think_mean, legal_mean) = (mean(&thinks), mean(&legals));
        for (think, legal) in logs {
            covariance += (legal - legal_mean) * (think - think_mean);
            variance += (legal - legal_mean).powi(2);
        }
        samples += logs.len();
    }
    (samples >= min_samples && variance > 0.0).then(|| ScalarFit {
        samples,
        value: covariance / variance,
    })
}

/// The seat whose account won the outcome's match, when the outcome says this seat won.
fn winner_seat(outcome: &SignalOutcomeRow) -> Option<PlayerId> {
    (outcome.won == Some(true)).then_some(outcome.signal.seat)
}

/// R639000: how much longer the moves of a turn that wins take, in `ln(think_ms)`: the mean
/// residual, against its bucket's fit and `slope`, of each move the winner made on the last turn of
/// a game between two people won by a hero's death. A move in no fitted bucket is left out. `None`
/// with fewer than `min_samples` such moves.
pub fn fit_lethal_shift(
    rows: &[ActionTimingRow],
    outcomes: &[SignalOutcomeRow],
    fits: &[BucketFit],
    slope: f64,
    min_samples: usize,
) -> Option<ScalarFit> {
    let seats = by_seat(outcomes);
    let human_match = |match_id: &str| {
        [PlayerId::P1, PlayerId::P2].iter().all(|seat| {
            seats
                .get(&(match_id, *seat))
                .is_some_and(|outcome| outcome.signal.pilot == Pilot::Human)
        })
    };
    // The winner's seat of each such game, and its last turn.
    let lethal: IndexMap<(&str, PlayerId), i64> = outcomes
        .iter()
        .filter(|outcome| outcome.reason == GameOverReason::HeroDeath)
        .filter_map(|outcome| {
            let seat = winner_seat(outcome)?;
            human_match(&outcome.signal.match_id)
                .then_some(((outcome.signal.match_id.as_str(), seat), outcome.turns))
        })
        .collect();
    let residuals: Vec<f64> = rows
        .iter()
        .filter(|row| row.pilot == Pilot::Human)
        .filter(|row| {
            lethal
                .get(&(row.match_id.as_str(), row.seat))
                .is_some_and(|turns| i64::from(row.turn) == *turns)
        })
        .filter_map(|row| {
            let fit = fits.iter().find(|fit| {
                fit.rank_bucket == row.rank_bucket
                    && fit.action_kind == row.action_kind
                    && fit.first_in_turn == row.first_in_turn
            })?;
            let (think, legal) = logs_of(row);
            Some(think - (fit.mu + slope * (legal - fit.ln_legal_mean)))
        })
        .collect();
    (residuals.len() >= min_samples).then(|| ScalarFit {
        samples: residuals.len(),
        value: mean(&residuals),
    })
}

/// People's signals rows, the AI's left out.
fn human(outcomes: &[SignalOutcomeRow]) -> impl Iterator<Item = &SignalOutcomeRow> {
    outcomes
        .iter()
        .filter(|outcome| outcome.signal.pilot == Pilot::Human)
}

/// Every seat's row, by match and seat.
type BySeat<'a> = IndexMap<(&'a str, PlayerId), &'a SignalOutcomeRow>;

fn by_seat(outcomes: &[SignalOutcomeRow]) -> BySeat<'_> {
    outcomes
        .iter()
        .map(|outcome| ((outcome.signal.match_id.as_str(), outcome.signal.seat), outcome))
        .collect()
}

/// The other seat's row of the same match, if it is held.
fn opponent_of<'a>(seats: &BySeat<'a>, outcome: &SignalOutcomeRow) -> Option<&'a SignalOutcomeRow> {
    seats
        .get(&(outcome.signal.match_id.as_str(), outcome.signal.seat.opponent()))
        .copied()
}

/// R639000: how people concede. The conceders are the people's seats that conceded with a deficit
/// recorded; `share` is them over the people's seats that lost, `min_turn` the
/// `TIMING_FIT_CONCEDE_TURN_QUANTILE` quantile of the turns they conceded on, and the deficit's
/// mean and population standard deviation are theirs. `None` with fewer than `min_samples`
/// conceders.
pub fn fit_concede(outcomes: &[SignalOutcomeRow], min_samples: usize) -> Option<ConcedeFit> {
    let losers = human(outcomes)
        .filter(|outcome| outcome.won == Some(false))
        .count();
    let conceded: Vec<(i32, f64)> = human(outcomes)
        .filter_map(|outcome| {
            Some((
                outcome.signal.conceded_turn?,
                outcome.signal.concede_eval_deficit?,
            ))
        })
        .collect();
    if conceded.len() < min_samples || losers == 0 {
        return None;
    }
    let mut turns: Vec<i32> = conceded.iter().map(|(turn, _)| *turn).collect();
    turns.sort_unstable();
    let deficits: Vec<f64> = conceded.iter().map(|(_, deficit)| *deficit).collect();
    let deficit_mu = mean(&deficits);
    Some(ConcedeFit {
        samples: conceded.len(),
        share: conceded.len() as f64 / losers as f64,
        min_turn: quantile(&turns, TIMING_FIT_CONCEDE_TURN_QUANTILE),
        deficit_mu,
        deficit_sigma: deviation(&deficits, deficit_mu),
    })
}

/// R639000: of the people's seats whose opponent offered a draw, the share that accepted one.
/// `None` with fewer than `min_samples` such seats.
pub fn fit_draw_answer(outcomes: &[SignalOutcomeRow], min_samples: usize) -> Option<DrawAnswerFit> {
    let by = by_seat(outcomes);
    let offered: Vec<&SignalOutcomeRow> = human(outcomes)
        .filter(|outcome| opponent_of(&by, outcome).is_some_and(|other| other.signal.draw_offers > 0))
        .collect();
    if offered.len() < min_samples {
        return None;
    }
    let accepted = offered
        .iter()
        .filter(|outcome| outcome.signal.draw_accepted)
        .count();
    Some(DrawAnswerFit {
        samples: offered.len(),
        accept_share: accepted as f64 / offered.len() as f64,
    })
}

/// The turns of a game of `turns` that were `seat`'s own: p1 plays the odd ones, p2 the even.
fn own_turns(seat: PlayerId, turns: i64) -> i64 {
    match seat {
        PlayerId::P1 => (turns + 1) / 2,
        PlayerId::P2 => turns / 2,
    }
}

/// R639000: the people's clock timeouts per own turn. `None` with fewer than `min_samples` own
/// turns.
pub fn fit_timeouts(outcomes: &[SignalOutcomeRow], min_samples: usize) -> Option<TimeoutFit> {
    let (timeouts, turns) = human(outcomes).fold((0i64, 0i64), |(timeouts, turns), outcome| {
        (
            timeouts + i64::from(outcome.signal.timeouts),
            turns + own_turns(outcome.signal.seat, outcome.turns).max(0),
        )
    });
    let samples = usize::try_from(turns).unwrap_or(0);
    (samples >= min_samples).then(|| TimeoutFit {
        samples,
        chance_per_turn: timeouts as f64 / turns as f64,
    })
}

/// R639000: over the people's seats of matches a rematch could follow, the share that offered one
/// the opponent did not meet first (`offer_share`), and of those whose opponent offered first, the
/// share that met it (`accept_share`). `None` with fewer than `min_samples` seats, or none offered
/// a rematch first.
pub fn fit_rematch(outcomes: &[SignalOutcomeRow], min_samples: usize) -> Option<RematchFit> {
    let first =
        |outcome: &SignalOutcomeRow| outcome.signal.rematch_offered && !outcome.signal.rematch_accepted;
    let by = by_seat(outcomes);
    let seats: Vec<&SignalOutcomeRow> = human(outcomes).filter(|outcome| outcome.rematchable).collect();
    let offered_to: Vec<&&SignalOutcomeRow> = seats
        .iter()
        .filter(|outcome| opponent_of(&by, outcome).is_some_and(first))
        .collect();
    if seats.len() < min_samples || offered_to.is_empty() {
        return None;
    }
    let offers = seats.iter().filter(|outcome| first(outcome)).count();
    let accepted = offered_to
        .iter()
        .filter(|outcome| outcome.signal.rematch_accepted)
        .count();
    Some(RematchFit {
        samples: seats.len(),
        offer_share: offers as f64 / seats.len() as f64,
        accept_share: accepted as f64 / offered_to.len() as f64,
    })
}

/// R639000, R645: the window between the `TIMING_FIT_DELAY_QUANTILES` of `delays` (ms). `None` with
/// fewer than `min_samples` of them.
pub fn fit_emote_delay(delays: &[i64], min_samples: usize) -> Option<EmoteDelayFit> {
    if delays.len() < min_samples || delays.is_empty() {
        return None;
    }
    let mut sorted = delays.to_vec();
    sorted.sort_unstable();
    let (low, high) = TIMING_FIT_DELAY_QUANTILES;
    Some(EmoteDelayFit {
        samples: sorted.len(),
        min_ms: quantile(&sorted, low) as f64,
        max_ms: quantile(&sorted, high) as f64,
    })
}

/// People's emotes, the AI's left out.
fn human_emotes(emotes: &[EmoteEventRow]) -> impl Iterator<Item = &EmoteEventRow> {
    emotes.iter().filter(|emote| emote.pilot == Pilot::Human)
}

/// R639000, R645: the persona a person's seat reads as from the `sent` emotes it sent, `taunts` of
/// them taunts: none is Silent, more than half taunts is BM, emotes but no taunt Polite, and
/// anything else Balanced.
fn persona_of(sent: usize, taunts: usize) -> PersonaName {
    if sent == 0 {
        PersonaName::Silent
    } else if taunts * 2 > sent {
        PersonaName::Bm
    } else if taunts == 0 {
        PersonaName::Polite
    } else {
        PersonaName::Balanced
    }
}

/// R639000, R645: the share of people's seats (one per signals row) reading as each persona, in
/// `PERSONA_NAMES` order. `None` with fewer than `min_samples` seats.
pub fn fit_persona_weights(
    outcomes: &[SignalOutcomeRow],
    emotes: &[EmoteEventRow],
    min_samples: usize,
) -> Option<PersonaWeights> {
    // Each person's seat's emotes, and how many of them were taunts.
    let mut sent: IndexMap<(&str, PlayerId), (usize, usize)> = IndexMap::new();
    for emote in human_emotes(emotes) {
        let counted = sent.entry((emote.match_id.as_str(), emote.seat)).or_default();
        counted.0 += 1;
        if reply_key_of(emote.emote_id) == Some(EmoteReplyKey::Taunt) {
            counted.1 += 1;
        }
    }
    let mut counts = [0usize; 4];
    let mut seats = 0;
    for outcome in human(outcomes) {
        let (emoted, taunts) = sent
            .get(&(outcome.signal.match_id.as_str(), outcome.signal.seat))
            .copied()
            .unwrap_or_default();
        let persona = persona_of(emoted, taunts);
        if let Some(at) = PERSONA_NAMES.iter().position(|name| *name == persona) {
            counts[at] += 1;
        }
        seats += 1;
    }
    (seats >= min_samples && seats > 0).then(|| PersonaWeights {
        samples: seats,
        weights: counts.map(|count| count as f64 / seats as f64),
    })
}

/// R639000: every fit `config.rs` holds beside `THINK_TIME_FITS`, each `None` while too few
/// observations back it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SignalFits {
    pub complexity_slope: Option<ScalarFit>,
    pub lethal_shift: Option<ScalarFit>,
    pub concede: Option<ConcedeFit>,
    pub draw_answer: Option<DrawAnswerFit>,
    pub timeout: Option<TimeoutFit>,
    pub rematch: Option<RematchFit>,
    pub emote_delay: Option<EmoteDelayFit>,
    pub emote_reply_delay: Option<EmoteDelayFit>,
    pub persona_weights: Option<PersonaWeights>,
}

/// R639000: every fit, from people's timings, emotes and signals beside their outcomes. The lethal
/// shift is measured against the bucket fits and the fitted slope (0 when there is none yet).
pub fn fit_signals(
    rows: &[ActionTimingRow],
    outcomes: &[SignalOutcomeRow],
    emotes: &[EmoteEventRow],
    min_samples: usize,
) -> SignalFits {
    let fits = fit_think_times(rows, min_samples);
    let complexity_slope = fit_complexity_slope(rows, min_samples);
    let slope = complexity_slope.map_or(0.0, |fit| fit.value);
    let after_trigger: Vec<i64> = human_emotes(emotes)
        .filter_map(|emote| emote.ms_since_trigger)
        .collect();
    let after_reply: Vec<i64> = human_emotes(emotes)
        .filter_map(|emote| emote.reply_to_opponent_ms)
        .collect();
    SignalFits {
        complexity_slope,
        lethal_shift: fit_lethal_shift(rows, outcomes, &fits, slope, min_samples),
        concede: fit_concede(outcomes, min_samples),
        draw_answer: fit_draw_answer(outcomes, min_samples),
        timeout: fit_timeouts(outcomes, min_samples),
        rematch: fit_rematch(outcomes, min_samples),
        emote_delay: fit_emote_delay(&after_trigger, min_samples),
        emote_reply_delay: fit_emote_delay(&after_reply, min_samples),
        persona_weights: fit_persona_weights(outcomes, emotes, min_samples),
    }
}

/// The fits as rows of `THINK_TIME_FITS`, after one comment line.
pub fn render_fits(fits: &[BucketFit]) -> String {
    let mut out = String::from(HEADER);
    out.push('\n');
    for fit in fits {
        let rank = match fit.rank_bucket {
            Some(tier) => format!("Some({:?})", literal(&tier)),
            None => "None".to_string(),
        };
        out.push_str(&format!(
            "    ThinkTimeFit {{ rank_bucket: {rank}, action_kind: {:?}, first_in_turn: {}, samples: {}, mu: {:.4}, sigma: {:.4}, ln_legal_mean: {:.4} }},\n",
            fit.action_kind.as_str(),
            fit.first_in_turn,
            fit.samples,
            fit.mu,
            fit.sigma,
            fit.ln_legal_mean,
        ));
    }
    out
}

/// One `pub const` line, or the comment that keeps the prior.
fn constant_line(name: &str, ty: &str, fields: Option<String>) -> String {
    match fields {
        Some(fields) => format!("pub const {name}: {ty} = {ty} {{ {fields} }};\n"),
        None => format!("// {name}: under TIMING_FIT_MIN_SAMPLES; keep the prior\n"),
    }
}

/// R639000: the fits as `config.rs` rows, after one comment line, four decimals to a number.
pub fn render_constants(fits: &SignalFits) -> String {
    let scalar = |fit: &ScalarFit| format!("samples: {}, value: {:.4}", fit.samples, fit.value);
    let window = |fit: &EmoteDelayFit| {
        format!(
            "samples: {}, min_ms: {:.4}, max_ms: {:.4}",
            fit.samples, fit.min_ms, fit.max_ms
        )
    };
    let lines = [
        constant_line(
            "THINK_COMPLEXITY_SLOPE",
            "ScalarFit",
            fits.complexity_slope.as_ref().map(scalar),
        ),
        constant_line(
            "THINK_LETHAL_SHIFT",
            "ScalarFit",
            fits.lethal_shift.as_ref().map(scalar),
        ),
        constant_line(
            "BOT_CONCEDE_FIT",
            "ConcedeFit",
            fits.concede.map(|fit| {
                format!(
                    "samples: {}, share: {:.4}, min_turn: {}, deficit_mu: {:.4}, deficit_sigma: {:.4}",
                    fit.samples, fit.share, fit.min_turn, fit.deficit_mu, fit.deficit_sigma
                )
            }),
        ),
        constant_line(
            "BOT_DRAW_ANSWER_FIT",
            "DrawAnswerFit",
            fits.draw_answer
                .map(|fit| format!("samples: {}, accept_share: {:.4}", fit.samples, fit.accept_share)),
        ),
        constant_line(
            "BOT_TIMEOUT_FIT",
            "TimeoutFit",
            fits.timeout.map(|fit| {
                format!(
                    "samples: {}, chance_per_turn: {:.4}",
                    fit.samples, fit.chance_per_turn
                )
            }),
        ),
        constant_line(
            "BOT_REMATCH_FIT",
            "RematchFit",
            fits.rematch.map(|fit| {
                format!(
                    "samples: {}, offer_share: {:.4}, accept_share: {:.4}",
                    fit.samples, fit.offer_share, fit.accept_share
                )
            }),
        ),
        constant_line(
            "BOT_EMOTE_DELAY_FIT",
            "EmoteDelayFit",
            fits.emote_delay.as_ref().map(window),
        ),
        constant_line(
            "BOT_EMOTE_REPLY_DELAY_FIT",
            "EmoteDelayFit",
            fits.emote_reply_delay.as_ref().map(window),
        ),
        constant_line(
            "BOT_PERSONA_WEIGHTS",
            "PersonaWeights",
            fits.persona_weights.map(|fit| {
                let weights: Vec<String> = fit.weights.iter().map(|weight| format!("{weight:.4}")).collect();
                format!("samples: {}, weights: [{}]", fit.samples, weights.join(", "))
            }),
        ),
    ];
    let mut out = String::from(SIGNALS_HEADER);
    out.push('\n');
    for line in lines {
        out.push_str(&line);
    }
    out
}

/// What `timing-fit` reads: every action timing, every emote event and every signals row beside its
/// match's result, in one transaction.
pub struct Telemetry {
    pub timings: Vec<ActionTimingRow>,
    pub emotes: Vec<EmoteEventRow>,
    pub outcomes: Vec<SignalOutcomeRow>,
}

/// Every row `timing-fit` fits, as the store holds them.
pub async fn telemetry(db: &Db) -> Result<Telemetry, StoreError> {
    let mut tx = db.begin(None).await?;
    let timings = tx.play_telemetry_timings().await?;
    let emotes = tx.play_telemetry_emotes().await?;
    let outcomes = tx.play_telemetry_outcomes().await?;
    tx.commit().await?;
    Ok(Telemetry {
        timings,
        emotes,
        outcomes,
    })
}

/// `timing-fit`'s arguments after the subcommand (`main.rs`): none.
pub async fn run(args: Vec<String>) -> anyhow::Result<()> {
    if let Some(arg) = args.first() {
        bail!("Unrecognised argument {}.\n\n{USAGE}", quoted(arg));
    }
    let connection_string = database_url()?;
    // One connection: this process runs one read and exits.
    let db = connect_one(&connection_string).await?;
    let read = telemetry(&db).await;
    db.close().await;
    let read = read?;
    let fits = fit_think_times(&read.timings, TIMING_FIT_MIN_SAMPLES);
    print!("{}", render_fits(&fits));
    if fits.is_empty() {
        eprintln!("timing-fit: no bucket holds {TIMING_FIT_MIN_SAMPLES} human moves yet");
    }
    let signals = fit_signals(
        &read.timings,
        &read.outcomes,
        &read.emotes,
        TIMING_FIT_MIN_SAMPLES,
    );
    print!("{}", render_constants(&signals));
    Ok(())
}
