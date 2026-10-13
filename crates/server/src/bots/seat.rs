//! The bot seat (SPEC §9.12, R1452): what the match actor runs when a ladder bot owes the match a
//! move. The bot decides on the AI's `decide_styled` at its strength (its budget and style, on the
//! spec's resources), off the blocking pool so the actor's runtime never waits on a search, then
//! waits out a human-paced think time before the move is sent: the delay drawn from the think-time
//! fits (R1442's `THINK_TIME_FITS`, or the hand-set priors until a fit lands) times the bot's speed,
//! less the time the decision took, inside its acting clock (R79).
//!
//! Both random streams are the server's own, seeded from the match seed and the seat by the actor
//! and apart from the match rng (`state.rng`): nothing here reaches the engine's state, the log or a
//! replay hash, and a match with a bot replays from `(seed, log)` like any other.
//!
//! R188 holds for a bot as for the practice AI: it never concedes, never offers a draw and never
//! accepts one. `decide` already declines an offer and offers none; `r188_forbidden` is the actor's
//! guard on top of it.

use std::time::Duration;

use jackioh_ai::{AiOptions, AiStyle, decide_styled, follows_best, random_action};
use jackioh_engine::{ActionBody, ActionType, GameState, PlayerId, Rng};

use crate::actor::engine;
use crate::config::{
    BOT_CLOCK_MARGIN_MS, BOT_DECISION_DEADLINE_MS, BOT_THINK_MAX_MS, BOT_THINK_MIN_MS,
    BOT_THINK_PRIOR_FIRST_MU, BOT_THINK_PRIOR_FIRST_SIGMA, BOT_THINK_PRIOR_LATER_MU,
    BOT_THINK_PRIOR_LATER_SIGMA, BotStrength, THINK_TIME_FITS,
};

/// One move a bot owes: the true state (the AI reads it only through `ai_to_act` and `redact`,
/// R185), the engine seat it plays now (R677), its strength and speed, its two streams and what the
/// think time is drawn for.
pub struct BotJob {
    pub state: GameState,
    pub seat: PlayerId,
    pub strength: BotStrength,
    pub speed: f64,
    /// The AI's own stream (`AiOptions.rng`).
    pub ai_rng: Rng,
    /// The think time's stream.
    pub timing_rng: Rng,
    /// Whether this is the bot's first move of the turn.
    pub first_in_turn: bool,
    /// The acting clock the bot has left (R79), or none when none is running for it.
    pub clock_left_ms: Option<i64>,
}

/// What a bot's move came to: the action it plays (`None` when the AI owed nothing or failed) and
/// where both streams stand after it.
#[derive(Clone, Debug, PartialEq)]
pub struct BotMove {
    pub body: Option<ActionBody>,
    pub ai_cursor: u32,
    pub timing_cursor: u32,
}

/// R747000: the bot's move under its strength's skill when the skill roll fails it: SPEC §10.7's
/// random policy, kept to what R188 lets a bot play, else the seat's fallback move. `None` only
/// when the seat has no legal move at all.
pub fn skill_random_move(state: &GameState, seat: PlayerId, rng: &mut Rng) -> Option<ActionBody> {
    match random_action(state, seat, rng) {
        Some(body) if !r188_forbidden(&body) => Some(body),
        _ => fallback_action(state, seat),
    }
}

/// R1452: decides, draws the think time, and waits whatever of it the decision did not take.
pub async fn play(job: BotJob) -> BotMove {
    let started = tokio::time::Instant::now();
    let BotJob {
        state,
        seat,
        strength,
        speed,
        ai_rng,
        mut timing_rng,
        first_in_turn,
        clock_left_ms,
    } = job;
    let mut ai_rng = ai_rng;
    // R747000's roll comes first, so a degraded move skips the search outright.
    let degraded = !follows_best(strength.skill, &mut ai_rng);
    let fallback_cursor = ai_rng.cursor();
    let (body, ai_cursor) = if degraded {
        (skill_random_move(&state, seat, &mut ai_rng), ai_rng.cursor())
    } else {
        let deadline_ms = decision_deadline_ms(clock_left_ms);
        let decided = tokio::task::spawn_blocking(move || {
            // The blocking pool's thread has no card override: the registered catalog it reads is the
            // real one, as the match's own (`actor::engine`).
            jackioh_cards::register_all();
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
            (decision.map(|decision| decision.action), options.rng.cursor())
        })
        .await;
        decided.unwrap_or((None, fallback_cursor))
    };

    let kind = body.as_ref().map_or(ActionType::EndTurn, ActionBody::action_type);
    let wait = think_ms(&mut timing_rng, kind, first_in_turn, speed, clock_left_ms);
    let spent = i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX);
    if wait > spent {
        tokio::time::sleep(Duration::from_millis(u64::try_from(wait - spent).unwrap_or(0))).await;
    }
    BotMove {
        body,
        ai_cursor,
        timing_cursor: timing_rng.cursor(),
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

/// R1452: one think time, in ms. The log-normal is the first `THINK_TIME_FITS` row for this action
/// kind and first-or-later move of the turn, else the hand-set prior; one draw of it (Box–Muller,
/// two draws of `rng`) is scaled by the bot's speed and kept inside `BOT_THINK_MIN_MS` and
/// `BOT_THINK_MAX_MS`, and inside the acting clock less `BOT_CLOCK_MARGIN_MS`, so a bot never runs its
/// own clock out by waiting.
pub fn think_ms(
    rng: &mut Rng,
    action_kind: ActionType,
    first_in_turn: bool,
    speed: f64,
    clock_left_ms: Option<i64>,
) -> i64 {
    let (mu, sigma) = THINK_TIME_FITS
        .iter()
        .find(|fit| fit.action_kind == action_kind.as_str() && fit.first_in_turn == first_in_turn)
        .map(|fit| (fit.mu, fit.sigma))
        .unwrap_or(if first_in_turn {
            (BOT_THINK_PRIOR_FIRST_MU, BOT_THINK_PRIOR_FIRST_SIGMA)
        } else {
            (BOT_THINK_PRIOR_LATER_MU, BOT_THINK_PRIOR_LATER_SIGMA)
        });
    // `1 − next()` is in (0, 1], so its logarithm is finite.
    let radius = (-2.0 * (1.0 - rng.next()).ln()).sqrt();
    let angle = std::f64::consts::TAU * rng.next();
    let z = radius * angle.cos();
    let drawn = ((mu + sigma * z).exp() * speed).round();
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

/// R188: the actions a bot never plays, whatever the AI answered: a concession, a draw offer and an
/// accepted draw. MD-D29, R1127: nor an `Emote`, as practice's AI never takes one: a bot's emotes
/// come from its persona through the relay, which mints the action while a card hears it.
pub fn r188_forbidden(body: &ActionBody) -> bool {
    matches!(
        body,
        ActionBody::Concede
            | ActionBody::OfferDraw
            | ActionBody::AnswerDraw { accept: true }
            | ActionBody::Emote { .. }
    )
}

/// What a bot plays when the AI gave nothing it may play, or the engine refused it: ending the turn
/// when that is legal, else the first legal action (a prompt's first answer, a mulligan's keep), else
/// nothing. Never one `r188_forbidden` names.
pub fn fallback_action(state: &GameState, seat: PlayerId) -> Option<ActionBody> {
    let legal: Vec<ActionBody> = engine::legal_actions(state, seat)
        .into_iter()
        .filter(|body| !r188_forbidden(body))
        .collect();
    legal
        .iter()
        .find(|body| matches!(body, ActionBody::EndTurn))
        .or_else(|| legal.first())
        .cloned()
}
