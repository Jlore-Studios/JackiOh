//! A ladder bot's end-of-game habits (SPEC §9.12, R639000; #639): whether it concedes, answers a
//! draw offer, lets its turn clock run out, offers or meets a rematch, and how long its aim arrow
//! shows before a targeted move, each drawn from `crate::config`'s fitted rows (R1442's telemetry,
//! printed by `jackioh-server timing-fit`) as a person's would be.
//!
//! Every roll draws on its own stream, `Rng::new("{seed}:bot-…:{home}…", 0)`, from the match seed and
//! the seat the bot began in, apart from the match rng (`state.rng`), the search's and the think
//! time's. A stream always starts at cursor 0, so the actor keeps no cursor for it and an actor
//! rebuilt after a restart draws the same. Nothing here reaches the engine's state, the log or a
//! replay hash. The one read of the state is `deficit`, through `redact`, so a hidden card never
//! changes what a bot does (R185).

use jackioh_ai::{AI_EVAL, NextSwing, evaluate, redact};
use jackioh_engine::{GameState, PlayerId, Rng};

use crate::config::{
    BOT_AIM_LEAD_MAX_MS, BOT_AIM_LEAD_MIN_MS, BOT_REMATCH_DELAY_MAX_MS, BOT_REMATCH_DELAY_MIN_MS,
    BOT_REMATCH_LINGER_MAX_MS, BOT_REMATCH_LINGER_MIN_MS, ConcedeFit, DrawAnswerFit, RematchFit, TimeoutFit,
};

/// One draw of the standard normal (Box–Muller, two draws of `rng`).
pub fn standard_normal(rng: &mut Rng) -> f64 {
    // `1 − next()` is in (0, 1], so its logarithm is finite.
    let radius = (-2.0 * (1.0 - rng.next()).ln()).sqrt();
    let angle = std::f64::consts::TAU * rng.next();
    radius * angle.cos()
}

/// R639000: how far behind `seat` stands, as R1442 measures a concession: the AI's evaluation of
/// the state from the seat's side, negated, the next swing the active seat's. It reads the state
/// only through `redact` (R185).
pub fn deficit(state: &GameState, seat: PlayerId) -> f64 {
    let next = if state.active == seat {
        NextSwing::Seat
    } else {
        NextSwing::Enemy
    };
    -evaluate(&redact(state, seat), seat, next, &AI_EVAL)
}

/// A whole number of ms drawn uniformly between `min` and `max`.
fn uniform_ms(rng: &mut Rng, min: i64, max: i64) -> i64 {
    (min as f64 + rng.next() * (max - min) as f64).round() as i64
}

/// R639000: whether the bot that began in `home` concedes now, at the start of its own turn `turn`,
/// standing `deficit` behind. Its temper is drawn once a match: whether it is one who concedes at
/// all (`share`), and the deficit it gives up at (normal, `deficit_mu` and `deficit_sigma`). It
/// never concedes before `min_turn`, nor while it is not behind at all, whatever its threshold drew.
pub fn concedes(seed: &str, home: PlayerId, turn: i32, deficit: f64, fit: &ConcedeFit) -> bool {
    let mut rng = Rng::new(&format!("{seed}:bot-concede:{home}"), 0);
    let conceder = rng.next() < fit.share;
    let threshold = fit.deficit_mu + fit.deficit_sigma * standard_normal(&mut rng);
    conceder && turn >= fit.min_turn && deficit > 0.0 && deficit >= threshold
}

/// R639000: whether the bot that began in `home` accepts the draw offered in turn `turn`. Only a
/// bot that stands behind accepts, and then at the fitted share; the AI's own answer, a refusal
/// (R188), stands otherwise.
pub fn accepts_draw(seed: &str, home: PlayerId, turn: i32, deficit: f64, fit: &DrawAnswerFit) -> bool {
    let mut rng = Rng::new(&format!("{seed}:bot-draw:{home}:{turn}"), 0);
    deficit > 0.0 && rng.next() < fit.accept_share
}

/// R639000: whether the bot that began in `home` lets its own turn `turn` run out (R79) rather than
/// play it.
pub fn idles_turn(seed: &str, home: PlayerId, turn: i32, fit: &TimeoutFit) -> bool {
    let mut rng = Rng::new(&format!("{seed}:bot-idle:{home}:{turn}"), 0);
    rng.next() < fit.chance_per_turn
}

/// R639000: what the bot does once a ranked match it played is over (R672).
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

/// R639000: the bot that began in `home`'s plan for the end of its match, drawn in a fixed order:
/// the linger, whether it offers, whether it accepts, then the two delays.
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

/// R639000, R738: how long the bot that began in `home` shows its aim before the move it decided
/// when the log's next seq was `seq`.
pub fn aim_lead_ms(seed: &str, home: PlayerId, seq: i64) -> i64 {
    let mut rng = Rng::new(&format!("{seed}:bot-aim:{home}:{seq}"), 0);
    uniform_ms(&mut rng, BOT_AIM_LEAD_MIN_MS, BOT_AIM_LEAD_MAX_MS)
}
