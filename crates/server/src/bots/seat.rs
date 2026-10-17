//! The bot seat (SPEC §9.12, R1452, R639000, R747000): what the match actor runs when a ladder bot owes
//! the match a move. The bot decides on the AI's `decide_styled` at its strength (its budget and
//! style, on the spec's resources), off the blocking pool so the actor's runtime never waits on a
//! search, except that at each decision its strength's skill may hand the move to the random policy
//! instead (R747000: skill is the chance a decision follows the AI, rolled on the bot's own AI
//! stream), then waits out a human-paced think time before the move is sent: the delay drawn from the
//! think-time fits (R1442's `THINK_TIME_FITS`, or the hand-set priors until a fit lands), longer on a
//! board with more to do and on a turn that wins (R639000), times the bot's speed, less the time the
//! decision took, inside its acting clock (R79). A targeted play or attack carries the aim a person's
//! client would show (`bots::aim`), which the actor relays a beat before the move (R738).
//!
//! The random streams are the server's own, seeded from the match seed and the seat by the actor
//! and apart from the match rng (`state.rng`): nothing here reaches the engine's state, the log or a
//! replay hash, and a match with a bot replays from `(seed, log)` like any other.
//!
//! R188 holds for the AI's own answer: `decide` never concedes, offers no draw and declines every
//! offer, and `r188_forbidden` names what it may not answer. R639000 is the ladder bot's exception:
//! at the start of its own turn it may concede, as a person behind does (`signals::concedes`), and
//! it may accept a draw offered while it stands behind (`signals::accepts_draw`); it never offers a
//! draw, and `never_played` is the actor's guard for that and for an emote.

use std::time::Duration;

use jackioh_ai::{
    AiOptions, AiStyle, DecisionReason, decide_styled, plays_best, random_action, unanswered_draw_offer,
};
use jackioh_engine::{ActionBody, ActionType, Aim, GameState, PlayerId, Rng};

use crate::actor::engine::{self, Pinned};
use crate::bots::aim::bot_aim;
use crate::bots::signals::{accepts_draw, concedes, standard_normal, standing};
use crate::config::{
    AIM_RELAY_INTERVAL_MS, BOT_CLOCK_MARGIN_MS, BOT_CONCEDE_FIT, BOT_DECISION_DEADLINE_MS,
    BOT_DRAW_ANSWER_FIT, BOT_THINK_MAX_MS, BOT_THINK_MIN_MS, BOT_THINK_PRIOR_FIRST_MU,
    BOT_THINK_PRIOR_FIRST_SIGMA, BOT_THINK_PRIOR_LATER_MU, BOT_THINK_PRIOR_LATER_SIGMA,
    BOT_THINK_PRIOR_LN_LEGAL_MEAN, BOT_THINK_PRIORS, BotStrength, THINK_COMPLEXITY_SLOPE, THINK_LETHAL_SHIFT,
    THINK_TIME_FITS, ThinkTimeFit, ThinkTimePrior,
};

/// One move a bot owes: the true state (the AI reads it only through `ai_to_act` and `redact`,
/// R185), the engine seat it plays now (R677) and the one it began in, its strength and speed, its
/// two streams and what the think time is drawn for.
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
    /// R639000: whether this is the first move of the bot's own turn, the one a concession comes at.
    pub own_turn_start: bool,
    /// R639000: how long an aim shows before the move (`signals::aim_lead_ms`).
    pub aim_lead_ms: i64,
    /// The acting clock the bot has left (R79), or none when none is running for it.
    pub clock_left_ms: Option<i64>,
    /// R388, R767000: the catalog the decision reads.
    pub pinned: Pinned,
}

/// What a bot's move came to: the action it plays (`None` when the AI owed nothing or failed), where
/// both streams stand after it, and the aim the actor relays `aim_lead_ms` before it (R639000).
#[derive(Clone, Debug, PartialEq)]
pub struct BotMove {
    pub body: Option<ActionBody>,
    pub ai_cursor: u32,
    pub timing_cursor: u32,
    pub aim: Option<Aim>,
    pub aim_lead_ms: i64,
}

/// R639000: what one think time is drawn for: the move's kind, whether it is the bot's first of the
/// turn, how many actions were legal to it (`legal_actions`, as R1442 counts a person's) and whether
/// the decision was a lethal one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThinkFor {
    pub action_kind: ActionType,
    pub first_in_turn: bool,
    pub legal_count: usize,
    pub lethal: bool,
}

/// R1452, R639000: the rows a think time is drawn from: the fits, then the per-kind priors, then the
/// first and later moves' priors; the complexity the priors are centred on; and the two shifts.
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

/// R1452: decides, draws the think time, and waits whatever of it the decision did not take, less
/// the aim's lead, which the actor waits out once it has relayed the aim (R639000). At the start of
/// its own turn the bot first asks whether it concedes; a draw offer it is answering it accepts or
/// declines as `signals::accepts_draw` says (R639000).
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
        // The blocking pool's thread has no card override: the registered catalog it reads is the
        // real one, as the match's own (`actor::engine`). The whole decision runs under the match's
        // pinned catalog (R388, R767000).
        jackioh_cards::register_all();
        pinned.run(|| {
            let legal_count = engine::legal_actions(pinned, &state, seat).len();
            if own_turn_start
                && concedes(
                    &seed,
                    home,
                    state.turn,
                    &standing(&state, seat, &seed, home),
                    &BOT_CONCEDE_FIT,
                )
            {
                return Decided {
                    body: Some(ActionBody::Concede),
                    ai_cursor: ai_rng.cursor(),
                    legal_count,
                    lethal: false,
                    aim: None,
                };
            }
            // R747000: the skill's roll comes first, on the AI's own stream; a full skill takes no draw,
            // so a bot at it plays as it did before the knob.
            let mut ai_rng = ai_rng;
            let (chosen, lethal, ai_cursor) = if plays_best(strength.skill, &mut ai_rng) {
                let deadline = std::time::Instant::now()
                    + Duration::from_millis(u64::try_from(deadline_ms).unwrap_or(0));
                let should_stop = move || std::time::Instant::now() >= deadline;
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
                // The random policy never answers an offer (§10.7), so the bot declines as the AI does
                // and the roll below turns that into its own answer (R639000); no draw is taken.
                (
                    Some(ActionBody::AnswerDraw { accept: false }),
                    false,
                    ai_rng.cursor(),
                )
            } else {
                let action = random_action(&state, seat, &mut ai_rng);
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
    // R738: an aim shows for its lead, never longer than the wait left, and only when the relay
    // can send it before the move (one relay per `AIM_RELAY_INTERVAL_MS`).
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

/// R1452: how long a decision may search, in ms: `BOT_DECISION_DEADLINE_MS`, and never so long that
/// less than `BOT_CLOCK_MARGIN_MS` is left on the acting clock, so a search never runs the bot's own
/// clock out. At 0 the search stops at its first node and the AI answers with what it has.
pub fn decision_deadline_ms(clock_left_ms: Option<i64>) -> i64 {
    match clock_left_ms {
        Some(left) => BOT_DECISION_DEADLINE_MS.min(left - BOT_CLOCK_MARGIN_MS).max(0),
        None => BOT_DECISION_DEADLINE_MS,
    }
}

/// R1452, R639000: one think time, in ms. The log-normal is the first of `model`'s fits for this
/// action kind and first-or-later move of the turn, else its prior for the kind, else the first or
/// later moves' prior; its mean grows by `complexity_slope` for each unit of `ln(legal_count)` above
/// the row's (`ln_legal_mean`, or the priors' `prior_ln_legal_mean`), and by `lethal_shift` on a
/// lethal decision. One draw of it (Box–Muller, two draws of `rng`) is scaled by the bot's speed and
/// kept inside `BOT_THINK_MIN_MS` and `BOT_THINK_MAX_MS`, and inside the acting clock less
/// `BOT_CLOCK_MARGIN_MS`, so a bot never runs its own clock out by waiting.
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

/// R188: the answers the AI may not give, which `play` drops: a concession, a draw offer and an
/// accepted draw. MD-D29, R1127: nor an `Emote`, as practice's AI never takes one: a bot's emotes
/// come from its persona through the relay, which mints the action while a card hears it. A ladder
/// bot's own concession and draw answer come from `signals` instead (R639000).
pub fn r188_forbidden(body: &ActionBody) -> bool {
    matches!(
        body,
        ActionBody::Concede
            | ActionBody::OfferDraw
            | ActionBody::AnswerDraw { accept: true }
            | ActionBody::Emote { .. }
    )
}

/// R639000: the moves the actor never plays for a bot, whatever `play` came back with: a draw offer
/// (a ladder bot never offers one) and an emote (its persona's, through the relay).
pub fn never_played(body: &ActionBody) -> bool {
    matches!(body, ActionBody::OfferDraw | ActionBody::Emote { .. })
}

/// What a bot plays when the AI gave nothing it may play, or the engine refused it: ending the turn
/// when that is legal, else the first legal action (a prompt's first answer, a mulligan's keep), else
/// nothing. Never one `r188_forbidden` names.
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
