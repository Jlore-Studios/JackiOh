//! What the machine decides of a seat the host plays for, a ladder bot's (SPEC §9.12, R1452, R639000)
//! or the AI's: the moves it never plays, the move it falls back on, when it lets its clock run out and
//! how long it aims before it moves. The decision of the move itself is the host's: the machine asks
//! for it (`DuelOutput::Decide`) and is told it (`DuelEvent::Decided`), so every AI call is one
//! separate, deadline-bounded step.

use serde::{Deserialize, Serialize};

use jackioh_engine::{ActionBody, GameState, PlayerId, Rng};

/// R188: actions forbidden to the AI (concede, draw offer, accepted draw). MD-D29, R1127: also filters
/// `Emote` (handled by its persona). A ladder bot's concession, draw offer and draw answer come from
/// its own signals (R639000, R2660000, R2660001).
pub fn r188_forbidden(body: &ActionBody) -> bool {
    matches!(
        body,
        ActionBody::Concede
            | ActionBody::OfferDraw
            | ActionBody::AnswerDraw { accept: true }
            | ActionBody::Emote { .. }
    )
}

/// R639000, R2660001: the move the host never plays for a bot: an emote (its persona's, through the
/// relay). A draw offer it plays, by its signals' roll.
pub fn never_played(body: &ActionBody) -> bool {
    matches!(body, ActionBody::Emote { .. })
}

/// Fallback action when the AI produces no valid move: `EndTurn` if legal, else the first legal action
/// not forbidden by [`r188_forbidden`]. Reads the catalog the host runs the machine under (R388, R767000).
pub fn fallback_action(state: &GameState, seat: PlayerId) -> Option<ActionBody> {
    let legal: Vec<ActionBody> = jackioh_engine::legal_actions(state, seat)
        .into_iter()
        .filter(|body| !r188_forbidden(body))
        .collect();
    legal
        .iter()
        .find(|body| matches!(body, ActionBody::EndTurn))
        .or_else(|| legal.first())
        .cloned()
}

/// A whole number of ms drawn uniformly between `min` and `max`.
pub fn uniform_ms(rng: &mut Rng, min: i64, max: i64) -> i64 {
    (min as f64 + rng.next() * (max - min) as f64).round() as i64
}

/// R639000: whether the bot that began in `home` lets its turn clock run out on `turn` (R79), at
/// `chance_per_turn`, drawn on the turn's own stream so every call agrees.
pub fn idles_turn(seed: &str, home: PlayerId, turn: i32, chance_per_turn: f64) -> bool {
    let mut rng = Rng::new(&format!("{seed}:bot-idle:{home}:{turn}"), 0);
    rng.next() < chance_per_turn
}

/// R639000, R738: how long the aim shows before the move at log position `seq`, between `min_ms` and
/// `max_ms`.
pub fn aim_lead_ms(seed: &str, home: PlayerId, seq: i64, min_ms: i64, max_ms: i64) -> i64 {
    let mut rng = Rng::new(&format!("{seed}:bot-aim:{home}:{seq}"), 0);
    uniform_ms(&mut rng, min_ms, max_ms)
}

/// R639000: post-match behavior in ranked play (R672).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RematchPlan {
    /// How long it stays on the match, present for a rematch, from the game's end.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub linger_ms: i64,
    /// Whether it offers a rematch itself.
    pub offers: bool,
    /// Whether it meets a rematch the person offers while it is there.
    pub accepts: bool,
    /// How long after the game's end it offers.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub offer_after_ms: i64,
    /// How long after a person's offer it meets it.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub reply_ms: i64,
}

/// What the machine is told of the ladder bot at a home seat when the host is made (#3103 moves the
/// bots' own fits in; until then the host draws them and states them here).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct BotSeat {
    /// R639000: the chance the bot lets its turn clock run out on a turn.
    pub idle_chance_per_turn: f64,
    /// R738: the shortest and the longest its aim shows before a move.
    #[cfg_attr(feature = "ts", ts(type = "[number, number]"))]
    pub aim_lead_ms: (i64, i64),
    /// R639000: how it behaves when a ranked game ends.
    pub rematch: RematchPlan,
}
