//! What the machine decides of a seat the host plays for, a ladder bot's (SPEC §9.12, R1452, R639000)
//! or the AI's: the moves it never plays, the move it falls back on, and who the bot is. The move itself
//! is the AI's, so the machine asks for it (`DuelOutput::Decide`) and is told it (`DuelEvent::Decided`),
//! one separate, deadline-bounded step each (`bots::decide`). Everything else a bot does, when it moves,
//! how long it thinks, whether it aims, emotes, concedes, lets a clock run out or stays for a rematch,
//! the machine decides and sets timers for (`host/duel.rs`), so a host runs a bot the way it runs a
//! person, with no bot code of its own (#3103).

use serde::{Deserialize, Serialize};

use jackioh_engine::{ActionBody, GameState, PlayerId};

use crate::bots::decide::{BotAsk, BotChoice, decide};
use crate::config::{BotStrength, PersonaName};

pub use crate::bots::signals::RematchPlan;

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

/// Who the ladder bot at a home seat is, as the host makes the machine (R1452, R639000): the machine
/// draws the rest, its habits and its plan for the end of a ranked game, from the match seed.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct BotSeat {
    /// R1452: its index in `BOT_STRENGTHS`; a bot keeps its index, and one past the table plays the last.
    pub strength: u32,
    /// R1452: its think times' multiplier.
    pub speed: f64,
    /// R645: the persona it emotes by.
    pub persona: PersonaName,
    /// R1452: whether the bot passes for a person. A hidden bot moves at a person's pace, shows its aim
    /// before a targeted move, emotes by its persona, concedes, offers and answers draws, lets a turn run
    /// out now and then and stays on a finished ranked game for a rematch (R639000), and the queue waits
    /// out a fallback time before it pairs one with a ticket (R1453, `bots::fallback`). One that is not
    /// plays as the practice AI does (R188): it moves as soon as it has decided and does none of those. On
    /// the web every opponent is a person to the player, so the server sets it; a host whose players
    /// see a bot labelled leaves it off.
    #[serde(default)]
    pub hidden: bool,
}

/// What a decision needs, owned, so a host can run it where it can bound it: on a blocking thread, or in a
/// slice of a single-threaded server's frame. Made by `DuelHost::job` from a `DecideRequest`; the host
/// answers the request with `DuelEvent::Decided` and the [`BotChoice`] [`run`](DecideJob::run) gives.
#[derive(Clone, Debug)]
pub struct DecideJob {
    pub game: GameState,
    /// The engine seat the bot plays now (R677).
    pub seat: PlayerId,
    /// The seat it began in, which its streams are keyed by.
    pub home: PlayerId,
    pub seed: String,
    pub strength: BotStrength,
    pub hidden: bool,
    pub own_turn_start: bool,
    /// Where the bot's search stream stands.
    pub ai_cursor: u32,
    /// The most wall-clock time the decision may take, in ms (`DecideRequest.deadline_ms`).
    pub deadline_ms: i64,
}

impl DecideJob {
    /// The decision (`bots::decide`), under the catalog the caller runs it in (R388, R767000).
    /// `should_stop` is the host's wall-clock cap, built from `deadline_ms` and its clock; with none the
    /// search ends at the strength's budget.
    pub fn run(&self, should_stop: Option<&dyn Fn() -> bool>) -> BotChoice {
        decide(
            &BotAsk {
                state: &self.game,
                seat: self.seat,
                home: self.home,
                seed: &self.seed,
                strength: &self.strength,
                hidden: self.hidden,
                own_turn_start: self.own_turn_start,
                ai_cursor: self.ai_cursor,
            },
            should_stop,
        )
    }
}
