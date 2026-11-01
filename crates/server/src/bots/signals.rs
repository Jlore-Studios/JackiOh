//! A ladder bot's end-of-game habits (SPEC §9.12, R639000; #639): concessions, draw answers,
//! timeouts, and rematch behavior fitted from telemetry (R1442), and draw offers at `BOT_DRAW_OFFER`'s
//! hand-set chance (R2660001). Since R2660000 a concession also needs a lethal the bot cannot stop,
//! which the caller proves (`jackioh_ai::unstoppable_lethal`).
//!
//! Independent RNG streams are seeded per behavior without modifying engine state or replay hashes.
//! Evaluates `standing` via `redact` and `determinize` so hidden cards remain unread (R185).

use jackioh_ai::{AI_EVAL, DeterminizeOptions, NextSwing, determinize, evaluate, redact};
use jackioh_engine::{GameState, PlayerId, Rng};

use crate::config::{
    BOT_AIM_LEAD_MAX_MS, BOT_AIM_LEAD_MIN_MS, BOT_REMATCH_DELAY_MAX_MS, BOT_REMATCH_DELAY_MIN_MS,
    BOT_REMATCH_LINGER_MAX_MS, BOT_REMATCH_LINGER_MIN_MS, ConcedeFit, DrawAnswerFit, DrawOfferRule,
    RematchFit, TimeoutFit,
};

/// One draw of the standard normal (Box–Muller, two draws of `rng`).
pub fn standard_normal(rng: &mut Rng) -> f64 {
    // `1 − next()` is in (0, 1], so its logarithm is finite.
    let radius = (-2.0 * (1.0 - rng.next()).ln()).sqrt();
    let angle = std::f64::consts::TAU * rng.next();
    radius * angle.cos()
}

/// R639000: where a seat stands, read at one moment of its game.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Standing {
    /// Negated AI evaluation matching `ConcedeFit` threshold scale (R1442).
    pub deficit: f64,
    /// Half the gap between opponent's evaluation and seat's (> 0 when behind).
    pub behind: f64,
}

/// R639000: evaluates seat standing via `redact` (R185) and determinized world evaluation.
pub fn standing(state: &GameState, seat: PlayerId, seed: &str, home: PlayerId) -> Standing {
    let mut rng = Rng::new(&format!("{seed}:bot-standing:{home}:{}", state.turn), 0);
    let world = determinize(
        &redact(state, seat),
        seat,
        &mut rng,
        DeterminizeOptions::default(),
    );
    let swing = |side: PlayerId| {
        if world.active == side {
            NextSwing::Seat
        } else {
            NextSwing::Enemy
        }
    };
    let own = evaluate(&world, seat, swing(seat), &AI_EVAL);
    let theirs = evaluate(&world, seat.opponent(), swing(seat.opponent()), &AI_EVAL);
    Standing {
        deficit: -own,
        behind: (theirs - own) / 2.0,
    }
}

/// A whole number of ms drawn uniformly between `min` and `max`.
fn uniform_ms(rng: &mut Rng, min: i64, max: i64) -> i64 {
    (min as f64 + rng.next() * (max - min) as f64).round() as i64
}

/// R639000, R2660000: whether the bot that began in `home` concedes now, at its own turn start. Its
/// temper (`share`) is drawn once a match on `{seed}:bot-concede:{home}`; a conceder concedes only when
/// `doomed` (the caller's proof of a lethal nothing this turn stops) holds. `doomed` runs a search, so
/// it is called for a conceder alone.
pub fn concedes(seed: &str, home: PlayerId, doomed: impl FnOnce() -> bool, fit: &ConcedeFit) -> bool {
    let mut rng = Rng::new(&format!("{seed}:bot-concede:{home}"), 0);
    rng.next() < fit.share && doomed()
}

/// R2660001: whether the bot that began in `home` offers a draw at the start of its own turn `turn`,
/// standing as `standing` says: only while it trails its opponent by at least `rule.min_behind`, and
/// then with the chance `rule.chance_per_turn`, on the turn's own stream
/// `{seed}:bot-offer-draw:{home}:{turn}`, so every later look at the same turn answers the same.
/// Whether an offer is legal at all is still R36's, which the caller asks the engine.
pub fn offers_draw(seed: &str, home: PlayerId, turn: i32, standing: &Standing, rule: &DrawOfferRule) -> bool {
    let mut rng = Rng::new(&format!("{seed}:bot-offer-draw:{home}:{turn}"), 0);
    standing.behind >= rule.min_behind && rng.next() < rule.chance_per_turn
}

/// R639000: whether the bot accepts a draw offer while behind; otherwise refuses (R188).
pub fn accepts_draw(seed: &str, home: PlayerId, turn: i32, standing: &Standing, fit: &DrawAnswerFit) -> bool {
    let mut rng = Rng::new(&format!("{seed}:bot-draw:{home}:{turn}"), 0);
    standing.behind > 0.0 && rng.next() < fit.accept_share
}

/// R639000: whether the bot lets its turn clock run out (R79).
pub fn idles_turn(seed: &str, home: PlayerId, turn: i32, fit: &TimeoutFit) -> bool {
    let mut rng = Rng::new(&format!("{seed}:bot-idle:{home}:{turn}"), 0);
    rng.next() < fit.chance_per_turn
}

/// R639000: post-match behavior in ranked play (R672).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RematchPlan {
    /// How long it stays on the match, present for a rematch, from the game's end.
    pub linger_ms: i64,
    /// Whether it offers a rematch itself.
    pub offers: bool,
    /// Whether it meets a rematch the person offers while it is there.
    pub accepts: bool,
    /// How long after the game's end it offers.
    pub offer_after_ms: i64,
    /// How long after a person's offer it meets it.
    pub reply_ms: i64,
}

/// R639000: draws rematch plan parameters (linger, offer, accept, delays).
pub fn rematch_plan(seed: &str, home: PlayerId, fit: &RematchFit) -> RematchPlan {
    let mut rng = Rng::new(&format!("{seed}:bot-rematch:{home}"), 0);
    let linger_ms = uniform_ms(&mut rng, BOT_REMATCH_LINGER_MIN_MS, BOT_REMATCH_LINGER_MAX_MS);
    let offers = rng.next() < fit.offer_share;
    let accepts = rng.next() < fit.accept_share;
    let offer_after_ms = uniform_ms(&mut rng, BOT_REMATCH_DELAY_MIN_MS, BOT_REMATCH_DELAY_MAX_MS);
    let reply_ms = uniform_ms(&mut rng, BOT_REMATCH_DELAY_MIN_MS, BOT_REMATCH_DELAY_MAX_MS);
    RematchPlan {
        linger_ms,
        offers,
        accepts,
        offer_after_ms,
        reply_ms,
    }
}

/// R639000, R738: draws aim display duration prior to move dispatch.
pub fn aim_lead_ms(seed: &str, home: PlayerId, seq: i64) -> i64 {
    let mut rng = Rng::new(&format!("{seed}:bot-aim:{home}:{seq}"), 0);
    uniform_ms(&mut rng, BOT_AIM_LEAD_MIN_MS, BOT_AIM_LEAD_MAX_MS)
}
