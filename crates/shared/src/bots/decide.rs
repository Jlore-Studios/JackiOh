//! The move a ladder bot owes (SPEC §9.12, R1452, R639000, R747000): what the host answers a `decide`
//! with. One call, one move, apart from every other: the host runs it where it can bound it (a blocking
//! thread, a single-threaded server's slice of a frame) and feeds the answer to the machine, which
//! draws the think time, shows the aim and plays the move (`host/duel.rs`).
//!
//! The move is the AI's at the bot's strength (`decide_styled`), after a skill roll that may play a
//! beginner move instead (R747000, R2214000). Under R188 the AI never concedes or offers draws; a bot
//! that passes for a person (R1452) is the exception (R639000): at the start of its own turn it may
//! concede, since R2660000 only facing a lethal nothing stops (`unstoppable_lethal`), or offer a draw
//! while well behind (R2660001), and it answers a draw offer by its habit. Its random streams are its
//! own and reach no state, log or replay hash.

use serde::{Deserialize, Serialize};

use jackioh_ai::{
    AiOptions, AiStyle, DecisionReason, decide_styled, plays_best, skill_miss_action, unanswered_draw_offer,
    unstoppable_lethal,
};
use jackioh_engine::{ActionBody, Aim, GameState, PlayerId, Rng};

use crate::bots::aim::bot_aim;
use crate::bots::signals::{accepts_draw, concedes, offers_draw, standing};
use crate::config::{BOT_CONCEDE_FIT, BOT_DRAW_ANSWER_FIT, BOT_DRAW_OFFER, BotStrength};
use crate::host::bot::r188_forbidden;

/// What a bot owes a move on: the state, who it is and where its search's stream stands.
#[derive(Clone, Copy, Debug)]
pub struct BotAsk<'a> {
    pub state: &'a GameState,
    /// The engine seat it plays now (R677).
    pub seat: PlayerId,
    /// The seat it began the match in, which its R639000 streams are keyed by.
    pub home: PlayerId,
    /// The match seed, which its R639000 streams are drawn from.
    pub seed: &'a str,
    pub strength: &'a BotStrength,
    /// R1452: whether it passes for a person, and so has a person's habits (R639000).
    pub hidden: bool,
    /// R639000: whether this is the first move of the bot's own turn, the one a concession and a draw
    /// offer come at.
    pub own_turn_start: bool,
    /// Where the AI's own stream stands (`AiOptions.rng`).
    pub ai_cursor: u32,
}

/// What a decision came back with: the move, and what the machine paces it by.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct BotChoice {
    /// The move, or none when the AI found none (the machine then falls back, R188).
    pub body: Option<ActionBody>,
    /// Where the AI's stream stands after it.
    pub ai_cursor: u32,
    /// How many actions the bot could take (R1442): the board's complexity, which a think time grows with.
    pub legal_count: u32,
    /// Whether the move wins this turn, which a think time grows with (R639000).
    pub lethal: bool,
    /// R738: the aim a person's client would show before this move.
    pub aim: Option<Aim>,
}

impl BotChoice {
    /// A decision that found nothing, for a host whose decision could not run (a panic, a lost thread):
    /// the machine plays the fallback in its place, and the stream stays where it was.
    pub fn nothing(ai_cursor: u32) -> BotChoice {
        BotChoice {
            body: None,
            ai_cursor,
            legal_count: 0,
            lethal: false,
            aim: None,
        }
    }
}

/// R1452, R639000: decides the next action. At turn start a bot that passes for a person asks whether it
/// concedes (temper, then only facing a lethal it cannot stop inside the deadline, R2660000), else
/// whether it offers a draw when R36 lets it (R2660001). `should_stop` is the host's wall-clock cap
/// (`BOT_DECISION_DEADLINE_MS`, `think::decision_deadline_ms`): past it the search plays the best line
/// found so far; a host with no clock passes none and the search ends at the strength's budget. R3498000:
/// `wrap_up` is the same host's earlier stop for the search's beam alone (`AiOptions.wrap_up`, fired at
/// `wrap_up_after_ms` of the deadline), so that the reply and the other worlds still run; the concession
/// check is held to `should_stop` only.
pub fn decide(
    ask: &BotAsk<'_>,
    should_stop: Option<&dyn Fn() -> bool>,
    wrap_up: Option<&dyn Fn() -> bool>,
) -> BotChoice {
    let BotAsk {
        state,
        seat,
        home,
        seed,
        strength,
        hidden,
        own_turn_start,
        ai_cursor,
    } = *ask;
    let legal = jackioh_engine::legal_actions(state, seat);
    let legal_count = u32::try_from(legal.len()).unwrap_or(u32::MAX);
    let ai_rng = Rng::new(&format!("{seed}:bot-ai:{home}"), ai_cursor);
    // R639000, R2660000: a conceder concedes facing a lethal it cannot stop; R2660001: else it may
    // offer a draw. Neither draws on the AI's stream.
    let opening = if !hidden || !own_turn_start {
        None
    } else if concedes(
        seed,
        home,
        || unstoppable_lethal(state, seat, should_stop),
        &BOT_CONCEDE_FIT,
    ) {
        Some(ActionBody::Concede)
    } else if legal.contains(&ActionBody::OfferDraw)
        && offers_draw(
            seed,
            home,
            state.turn,
            &standing(state, seat, seed, home),
            &BOT_DRAW_OFFER,
        )
    {
        Some(ActionBody::OfferDraw)
    } else {
        None
    };
    if let Some(body) = opening {
        return BotChoice {
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
            should_stop,
            wrap_up,
        };
        let style = AiStyle {
            temperature: strength.temperature,
            missed_lethal_chance: strength.missed_lethal_chance,
        };
        let decision = decide_styled(state, seat, &mut options, &style);
        let lethal = decision
            .as_ref()
            .is_some_and(|decision| decision.reason == DecisionReason::Lethal);
        (
            decision.map(|decision| decision.action),
            lethal,
            options.rng.cursor(),
        )
    } else if unanswered_draw_offer(state, seat) {
        // Declines offer initially (R188); transformed into bot answer below (R639000).
        (
            Some(ActionBody::AnswerDraw { accept: false }),
            false,
            ai_rng.cursor(),
        )
    } else {
        let action = skill_miss_action(state, seat, &mut ai_rng);
        (action, false, ai_rng.cursor())
    };
    let body = chosen
        .filter(|body| !r188_forbidden(body))
        .map(|body| match body {
            // R639000: a bot that passes for a person answers a draw offer as a person does.
            ActionBody::AnswerDraw { .. } if hidden => ActionBody::AnswerDraw {
                accept: accepts_draw(
                    seed,
                    home,
                    state.turn,
                    &standing(state, seat, seed, home),
                    &BOT_DRAW_ANSWER_FIT,
                ),
            },
            body => body,
        });
    let aim = body.as_ref().and_then(|body| bot_aim(state, seat, body));
    BotChoice {
        body,
        ai_cursor,
        legal_count,
        lethal,
        aim,
    }
}
