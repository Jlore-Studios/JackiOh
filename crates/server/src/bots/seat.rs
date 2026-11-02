//! The bot seat (SPEC §9.12, R1452, R639000, R747000): match actor execution when a ladder bot owes a move.
//!
//! Moves are decided via `decide_styled` at the bot's strength on a blocking thread. Each decision may roll
//! a skill miss (R747000, R2214000) using a beginner move (`skill_miss_action`). The bot waits a human-paced think
//! time (R1442, R639000) scaled by speed and bounded by acting clock (R79). Targeted actions carry aim relayed
//! before the move (R738). Match replay hashes are unaffected as bot RNG streams are independent.
//!
//! Under R188, the AI engine never concedes or offers draws. Ladder bots except this (R639000): at turn
//! start they may concede (`signals::concedes`), since R2660000 only facing a lethal nothing stops
//! (`unstoppable_lethal`), accept a draw while behind (`signals::accepts_draw`), and, R2660001, offer one
//! while well behind (`signals::offers_draw`). `never_played` guards an emote.

use std::time::Duration;

use jackioh_ai::{
    AiOptions, AiStyle, DecisionReason, decide_styled, plays_best, skill_miss_action, unanswered_draw_offer,
    unstoppable_lethal,
};
use jackioh_engine::{ActionBody, ActionType, Aim, GameState, PlayerId, Rng};

use crate::actor::engine::{self, Pinned};
use crate::bots::aim::bot_aim;
use crate::bots::signals::{accepts_draw, concedes, offers_draw, standard_normal, standing};
use crate::config::{
    AIM_RELAY_INTERVAL_MS, BOT_CLOCK_MARGIN_MS, BOT_CONCEDE_FIT, BOT_DECISION_DEADLINE_MS,
    BOT_DRAW_ANSWER_FIT, BOT_DRAW_OFFER, BOT_THINK_MAX_MS, BOT_THINK_MIN_MS, BOT_THINK_PRIOR_FIRST_MU,
    BOT_THINK_PRIOR_FIRST_SIGMA, BOT_THINK_PRIOR_LATER_MU, BOT_THINK_PRIOR_LATER_SIGMA,
    BOT_THINK_PRIOR_LN_LEGAL_MEAN, BOT_THINK_PRIORS, BotStrength, THINK_COMPLEXITY_SLOPE, THINK_LETHAL_SHIFT,
    THINK_TIME_FITS, ThinkTimeFit, ThinkTimePrior,
};

/// One move a bot owes: redacted state (R185), active seat (R677), strength, speed,
/// RNG streams, and think-time parameters.
pub struct BotJob {
    pub state: GameState,
    pub seat: PlayerId,
    /// The seat the bot began the match in, which its R639000 streams are keyed by.
    pub home: PlayerId,
    /// The match seed, which its R639000 streams are drawn from.
    pub seed: String,
    pub strength: BotStrength,
    pub speed: f64,
    /// The AI's own stream (`AiOptions.rng`).
    pub ai_rng: Rng,
    /// The think time's stream.
    pub timing_rng: Rng,
    /// Whether this is the bot's first move of the turn.
    pub first_in_turn: bool,
    /// R639000: whether this is the first move of the bot's own turn, the one a concession comes at,
    /// and a draw offer (R2660001).
    pub own_turn_start: bool,
    /// R639000: how long an aim shows before the move (`signals::aim_lead_ms`).
    pub aim_lead_ms: i64,
    /// The acting clock the bot has left (R79), or none when none is running for it.
    pub clock_left_ms: Option<i64>,
    /// R388, R767000: the catalog the decision reads.
    pub pinned: Pinned,
}

/// Result of a bot's move: action, updated RNG cursors, and optional aim (R639000).
#[derive(Clone, Debug, PartialEq)]
pub struct BotMove {
    pub body: Option<ActionBody>,
    pub ai_cursor: u32,
    pub timing_cursor: u32,
    pub aim: Option<Aim>,
    pub aim_lead_ms: i64,
}

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

/// What the blocking half of `play` came back with.
struct Decided {
    body: Option<ActionBody>,
    ai_cursor: u32,
    legal_count: usize,
    lethal: bool,
    aim: Option<Aim>,
}

/// R1452, R639000: decides the next action, draws think time, and sleeps remaining duration less aim
/// lead. At turn start it asks whether it concedes (temper, then only facing a lethal it cannot stop
/// inside the deadline, R2660000), else whether it offers a draw when R36 lets it (R2660001).
pub async fn play(job: BotJob) -> BotMove {
    let started = tokio::time::Instant::now();
    let BotJob {
        state,
        seat,
        home,
        seed,
        strength,
        speed,
        ai_rng,
        mut timing_rng,
        first_in_turn,
        own_turn_start,
        aim_lead_ms,
        clock_left_ms,
        pinned,
    } = job;
    let fallback_cursor = ai_rng.cursor();
    let deadline_ms = decision_deadline_ms(clock_left_ms);
    let decided = tokio::task::spawn_blocking(move || {
        // R388, R767000: runs under the match's pinned catalog.
        jackioh_cards::register_all();
        pinned.run(|| {
            let legal = engine::legal_actions(pinned, &state, seat);
            let legal_count = legal.len();
            let deadline =
                std::time::Instant::now() + Duration::from_millis(u64::try_from(deadline_ms).unwrap_or(0));
            let should_stop = move || std::time::Instant::now() >= deadline;
            // R639000, R2660000: a conceder concedes facing a lethal it cannot stop; R2660001: else it
            // may offer a draw. Neither draws on the AI's stream.
            let opening = if own_turn_start
                && concedes(
                    &seed,
                    home,
                    || unstoppable_lethal(&state, seat, Some(&should_stop)),
                    &BOT_CONCEDE_FIT,
                ) {
                Some(ActionBody::Concede)
            } else if own_turn_start
                && legal.contains(&ActionBody::OfferDraw)
                && offers_draw(
                    &seed,
                    home,
                    state.turn,
                    &standing(&state, seat, &seed, home),
                    &BOT_DRAW_OFFER,
                )
            {
                Some(ActionBody::OfferDraw)
            } else {
                None
            };
            if let Some(body) = opening {
                return Decided {
                    body: Some(body),
                    ai_cursor: ai_rng.cursor(),
                    legal_count,
                    lethal: false,
                    aim: None,
                };
            }
            // R747000: skill roll on the AI stream determines optimal play vs skill miss.
            let mut ai_rng = ai_rng;
            let (chosen, lethal, ai_cursor) = if plays_best(strength.skill, &mut ai_rng) {
                let mut options = AiOptions {
                    rng: ai_rng,
                    budget: strength.budget,
                    should_stop: Some(&should_stop),
                };
                let style = AiStyle {
                    temperature: strength.temperature,
                    missed_lethal_chance: strength.missed_lethal_chance,
                };
                let decision = decide_styled(&state, seat, &mut options, &style);
                let lethal = decision
                    .as_ref()
                    .is_some_and(|decision| decision.reason == DecisionReason::Lethal);
                (
                    decision.map(|decision| decision.action),
                    lethal,
                    options.rng.cursor(),
                )
            } else if unanswered_draw_offer(&state, seat) {
                // Declines offer initially (R188); transformed into bot answer below (R639000).
                (
                    Some(ActionBody::AnswerDraw { accept: false }),
                    false,
                    ai_rng.cursor(),
                )
            } else {
                let action = skill_miss_action(&state, seat, &mut ai_rng);
                (action, false, ai_rng.cursor())
            };
            let body = chosen
                .filter(|body| !r188_forbidden(body))
                .map(|body| match body {
                    ActionBody::AnswerDraw { .. } => ActionBody::AnswerDraw {
                        accept: accepts_draw(
                            &seed,
                            home,
                            state.turn,
                            &standing(&state, seat, &seed, home),
                            &BOT_DRAW_ANSWER_FIT,
                        ),
                    },
                    body => body,
                });
            let aim = body.as_ref().and_then(|body| bot_aim(&state, seat, body));
            Decided {
                body,
                ai_cursor,
                legal_count,
                lethal,
                aim,
            }
        })
    })
    .await;
    let Decided {
        body,
        ai_cursor,
        legal_count,
        lethal,
        aim,
    } = decided.unwrap_or(Decided {
        body: None,
        ai_cursor: fallback_cursor,
        legal_count: 0,
        lethal: false,
        aim: None,
    });

    let think = ThinkFor {
        action_kind: body.as_ref().map_or(ActionType::EndTurn, ActionBody::action_type),
        first_in_turn,
        legal_count,
        lethal,
    };
    let wait = think_ms(&mut timing_rng, &think, &THINK_MODEL, speed, clock_left_ms);
    let spent = i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX);
    let left = wait.saturating_sub(spent).max(0);
    // R738: aim lead duration capped by remaining wait and `AIM_RELAY_INTERVAL_MS`.
    let lead = aim_lead_ms.min(left);
    let (aim, lead) = match aim {
        Some(aim) if lead >= AIM_RELAY_INTERVAL_MS => (Some(aim), lead),
        _ => (None, 0),
    };
    if left > lead {
        tokio::time::sleep(Duration::from_millis(u64::try_from(left - lead).unwrap_or(0))).await;
    }
    BotMove {
        body,
        ai_cursor,
        timing_cursor: timing_rng.cursor(),
        aim,
        aim_lead_ms: lead,
    }
}

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

/// R188: actions forbidden to AI (concede, draw offer, accepted draw). MD-D29, R1127: also filters
/// `Emote` (handled by persona). A ladder bot's concession, draw offer and draw answer come from
/// `signals` (R639000, R2660000, R2660001).
pub fn r188_forbidden(body: &ActionBody) -> bool {
    matches!(
        body,
        ActionBody::Concede
            | ActionBody::OfferDraw
            | ActionBody::AnswerDraw { accept: true }
            | ActionBody::Emote { .. }
    )
}

/// R639000, R2660001: the move the actor never plays for a bot: an emote (its persona's, through the
/// relay). A draw offer it plays, by `signals::offers_draw`'s roll.
pub fn never_played(body: &ActionBody) -> bool {
    matches!(body, ActionBody::Emote { .. })
}

/// Fallback action when AI produces no valid move: `EndTurn` if legal, else first legal
/// action not forbidden by `r188_forbidden`.
pub fn fallback_action(pinned: Pinned, state: &GameState, seat: PlayerId) -> Option<ActionBody> {
    let legal: Vec<ActionBody> = engine::legal_actions(pinned, state, seat)
        .into_iter()
        .filter(|body| !r188_forbidden(body))
        .collect();
    legal
        .iter()
        .find(|body| matches!(body, ActionBody::EndTurn))
        .or_else(|| legal.first())
        .cloned()
}
