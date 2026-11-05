//! How long a ladder bot thinks before a move (SPEC §9.12, R1452, R639000): a human-paced wait drawn from
//! R1442's fitted think times for the kind of move, scaled by the bot's speed and bounded by its acting
//! clock (R79). The match host's machine draws it (`host/duel.rs`) from the bot's own timing stream when
//! the host answers a `decide` with the move, and sets the timer that plays it; nothing here sleeps.

use jackioh_engine::{ActionType, Rng};

use crate::bots::signals::standard_normal;
use crate::config::{
    BOT_CLOCK_MARGIN_MS, BOT_DECISION_DEADLINE_MS, BOT_THINK_MAX_MS, BOT_THINK_MIN_MS,
    BOT_THINK_PRIOR_FIRST_MU, BOT_THINK_PRIOR_FIRST_SIGMA, BOT_THINK_PRIOR_LATER_MU,
    BOT_THINK_PRIOR_LATER_SIGMA, BOT_THINK_PRIOR_LN_LEGAL_MEAN, BOT_THINK_PRIORS, THINK_COMPLEXITY_SLOPE,
    THINK_LETHAL_SHIFT, THINK_TIME_FITS, ThinkTimeFit, ThinkTimePrior,
};

/// R639000: parameters for drawing think time: action kind, first-in-turn flag,
/// legal action count (R1442), and lethal status.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThinkFor {
    pub action_kind: ActionType,
    pub first_in_turn: bool,
    pub legal_count: usize,
    pub lethal: bool,
}

/// R1452, R639000: model parameters for think-time distribution (fits, priors, complexity slope, lethal shift).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThinkModel<'a> {
    pub fits: &'a [ThinkTimeFit],
    pub priors: &'a [ThinkTimePrior],
    pub prior_ln_legal_mean: f64,
    pub complexity_slope: f64,
    pub lethal_shift: f64,
}

/// The think-time model the ladder bots play by: `crate::config`'s rows.
pub const THINK_MODEL: ThinkModel<'static> = ThinkModel {
    fits: THINK_TIME_FITS,
    priors: BOT_THINK_PRIORS,
    prior_ln_legal_mean: BOT_THINK_PRIOR_LN_LEGAL_MEAN,
    complexity_slope: THINK_COMPLEXITY_SLOPE.value,
    lethal_shift: THINK_LETHAL_SHIFT.value,
};

/// R1452: search deadline in ms: `BOT_DECISION_DEADLINE_MS`, bounded so at least `BOT_CLOCK_MARGIN_MS`
/// remains on the acting clock.
pub fn decision_deadline_ms(clock_left_ms: Option<i64>) -> i64 {
    match clock_left_ms {
        Some(left) => BOT_DECISION_DEADLINE_MS.min(left - BOT_CLOCK_MARGIN_MS).max(0),
        None => BOT_DECISION_DEADLINE_MS,
    }
}

/// R1452, R639000: draws think time in ms from log-normal fits/priors adjusted for action complexity
/// and lethal status. Scaled by bot speed and clamped within `[BOT_THINK_MIN_MS, BOT_THINK_MAX_MS]`
/// and remaining clock less `BOT_CLOCK_MARGIN_MS`.
pub fn think_ms(
    rng: &mut Rng,
    think: &ThinkFor,
    model: &ThinkModel<'_>,
    speed: f64,
    clock_left_ms: Option<i64>,
) -> i64 {
    let kind = think.action_kind.as_str();
    let (mu, sigma, ln_legal_mean) = model
        .fits
        .iter()
        .find(|fit| fit.action_kind == kind && fit.first_in_turn == think.first_in_turn)
        .map(|fit| (fit.mu, fit.sigma, fit.ln_legal_mean))
        .or_else(|| {
            model
                .priors
                .iter()
                .find(|prior| prior.action_kind == kind)
                .map(|prior| (prior.mu, prior.sigma, model.prior_ln_legal_mean))
        })
        .unwrap_or(if think.first_in_turn {
            (
                BOT_THINK_PRIOR_FIRST_MU,
                BOT_THINK_PRIOR_FIRST_SIGMA,
                model.prior_ln_legal_mean,
            )
        } else {
            (
                BOT_THINK_PRIOR_LATER_MU,
                BOT_THINK_PRIOR_LATER_SIGMA,
                model.prior_ln_legal_mean,
            )
        });
    let complexity = (think.legal_count.max(1) as f64).ln() - ln_legal_mean;
    let lethal = if think.lethal { model.lethal_shift } else { 0.0 };
    let z = standard_normal(rng);
    let drawn = ((mu + model.complexity_slope * complexity + lethal + sigma * z).exp() * speed).round();
    let upper = match clock_left_ms {
        Some(left) => BOT_THINK_MAX_MS.min(left - BOT_CLOCK_MARGIN_MS).max(0),
        None => BOT_THINK_MAX_MS,
    };
    let lower = BOT_THINK_MIN_MS.min(upper);
    if drawn.is_nan() {
        return lower;
    }
    (drawn.clamp(lower as f64, upper as f64)) as i64
}
