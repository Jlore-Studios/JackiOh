//! The daily game's clock (SPEC §9.13, #3727), pure: it reads no time and keeps no timer. A daily game
//! has days per move, not seconds, and nothing of it stays in memory between moves: its host loads the
//! game, folds it, asks [`daily_sync`] who holds the move after each action and stores the
//! [`DailyClock`] it answers beside the match. Any read past a deadline asks [`daily_expired`], whose
//! answer the host logs as `disconnectExpired` for that player (R3727000), a loss as a live game's
//! expired grace is.
//!
//! Who holds the move ([`daily_holders`]):
//!  - R3727002: while both mulligans are open (R265), each seat still owing one, each with its own window;
//!  - R3727001: an open prompt's holder, so a prompt for the non-active player hands them the move with
//!    a fresh window, and its answer hands it back fresh;
//!  - R3727006: while the active player's draw offer stands (R269), the other player, who answers it;
//!  - otherwise the active player, for the whole of their turn.
//!
//! A holder keeps its hold, deadline and all, while it holds the same move: the same seat under the same
//! [`move_key`] (the mulligan window, or one player's turn, between two of R677's seat swaps). A
//! question sequence (R1563006, Meditative+ #20 KY's Competition) is a run of prompts for one player in
//! one turn, so it has no window of its own: the move's deadline is its window (R3727003). Every
//! duration is `crate::config`'s.

use serde::{Deserialize, Serialize};

use jackioh_engine::{ActionBody, GameResult, GameState, PLAYER_IDS, PerPlayer, PlayerId, opponent_of};

use super::snapshot::{mulligan_window, snapshot};
use crate::config::{DAILY_TIME_PER_MOVE_DAYS, MS_PER_DAY};

/// One seat's hold on the move: since when, until when, and which move it holds.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DailyHold {
    pub player: PlayerId,
    /// The move held ([`move_key`]): a hold carries on only while its seat holds the same one.
    pub key: String,
    /// Epoch ms the move passed to this seat.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub since: i64,
    /// Epoch ms the move runs out: `since` plus the whole time per move (R3727001).
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub deadline: i64,
}

/// A daily game's clock, stored beside its match: the time per move and who holds the move now, in
/// seat order. No hold once the game is over.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DailyClock {
    /// One of `DAILY_TIME_PER_MOVE_DAYS`.
    pub per_move_days: i32,
    pub holds: Vec<DailyHold>,
}

impl DailyClock {
    /// A clock with nothing held yet: [`daily_sync`] it over the begun game.
    pub fn new(per_move_days: i32) -> DailyClock {
        DailyClock {
            per_move_days,
            holds: Vec::new(),
        }
    }

    /// The whole time per move, in ms.
    pub fn window_ms(&self) -> i64 {
        i64::from(self.per_move_days) * MS_PER_DAY
    }

    /// `player`'s hold, if they hold the move.
    pub fn hold_of(&self, player: PlayerId) -> Option<&DailyHold> {
        self.holds.iter().find(|hold| hold.player == player)
    }

    /// Whether `player` holds the move.
    pub fn has_move(&self, player: PlayerId) -> bool {
        self.hold_of(player).is_some()
    }

    /// The earliest deadline held, the one the sweep looks for; `None` once nothing is held.
    pub fn next_deadline(&self) -> Option<i64> {
        self.holds.iter().map(|hold| hold.deadline).min()
    }
}

/// R3727002: the key of the mulligan window's holds, which no turn's key equals.
const MULLIGAN_KEY: &str = "mulligan";

/// The move a game waits on, as a hold's key: the mulligan window, or the turn of its active player
/// (both halves, as the live turn clock's key, R79). A prompt inside a turn is a part of that turn's
/// move; its holder is who tells it apart. R677: a Glitch's seat swap hands each seat to the other
/// account mid-move, so the swaps so far are part of the key, and the account a swap hands the move
/// to gets a whole window of its own, never what is left of the other's (R3727001).
pub fn move_key(state: &GameState) -> String {
    if !mulligan_window(&snapshot(state)).is_empty() {
        return MULLIGAN_KEY.to_string();
    }
    match state.seat_swaps.unwrap_or(0) {
        0 => format!("{}:{}", state.turn, state.active.as_str()),
        swaps => format!("{}:{}:{swaps}", state.turn, state.active.as_str()),
    }
}

/// Who holds the move, in seat order; nobody once the game is over.
pub fn daily_holders(state: &GameState) -> Vec<PlayerId> {
    let snapshot = snapshot(state);
    if snapshot.result.is_some() {
        return Vec::new();
    }
    // R3727002: R265's window, each seat still owing its mulligan.
    let window = mulligan_window(&snapshot);
    if !window.is_empty() {
        return window;
    }
    // R3727001: an open prompt is its holder's move, whoever's turn it is; a sequence's questions
    // (R1563006) are one prompt after another for one player, so they keep one hold (R3727003).
    if let Some(holder) = snapshot.pending_for {
        return vec![holder];
    }
    // R3727006: a standing draw offer waits on the player it was made to.
    if jackioh_engine::standing_draw_offer(state) == Some(state.active) {
        return vec![opponent_of(state.active)];
    }
    vec![state.active]
}

/// The clock after the game reached `state` at `now`: each holder still holding the same move keeps
/// its hold; one the move passed to gets the whole time per move from `now` (R3727001).
pub fn daily_sync(clock: &DailyClock, state: &GameState, now: i64) -> DailyClock {
    let key = move_key(state);
    let holds = daily_holders(state)
        .into_iter()
        .map(|player| {
            clock
                .hold_of(player)
                .filter(|hold| hold.key == key)
                .cloned()
                .unwrap_or_else(|| DailyHold {
                    player,
                    key: key.clone(),
                    since: now,
                    deadline: now + clock.window_ms(),
                })
        })
        .collect();
    DailyClock {
        per_move_days: clock.per_move_days,
        holds,
    }
}

/// R3727000: the player whose move ran out by `now`, who loses the game: the earliest deadline at or
/// before `now`, and between two at the same moment the first seat (R3727002).
pub fn daily_expired(clock: &DailyClock, now: i64) -> Option<PlayerId> {
    PLAYER_IDS
        .iter()
        .filter_map(|player| clock.hold_of(*player))
        .filter(|hold| hold.deadline <= now)
        .min_by_key(|hold| hold.deadline)
        .map(|hold| hold.player)
}

/// §9.13: whether a daily game lets `player` send `action` on top of the engine's own rules. Either
/// player may concede at any time; any other action is only the move's holder's ([`daily_holders`]),
/// so a player not holding it can neither append to the log nor make the holder's next move stale
/// (an `setAutoEndTurn` the engine takes from either seat included). R3727006: while their own draw
/// offer stands the move is the other player's, so the offerer may only concede until it is
/// answered. What is left is the engine's to judge.
pub fn daily_allows(state: &GameState, player: PlayerId, action: &ActionBody) -> bool {
    if matches!(action, ActionBody::Concede) {
        return true;
    }
    jackioh_engine::standing_draw_offer(state) != Some(player) && daily_holders(state).contains(&player)
}

/// The engine's legal actions for `player` that a daily game lets them send ([`daily_allows`]).
pub fn daily_legal(state: &GameState, player: PlayerId, legal: Vec<ActionBody>) -> Vec<ActionBody> {
    legal
        .into_iter()
        .filter(|action| daily_allows(state, player, action))
        .collect()
}

/// §9.13: whether `days` is one of the times per move a daily game may be played at.
pub fn is_daily_time(days: i32) -> bool {
    DAILY_TIME_PER_MOVE_DAYS.contains(&days)
}

/// What a daily game's row in a list shows of its board without folding it: the turn, each seat's
/// hero's life and whether the accounts hold each other's seat (R677), all public, and its result.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DailyBoard {
    pub turn: i32,
    pub life: PerPlayer<i32>,
    pub seats_swapped: bool,
    pub result: Option<GameResult>,
}

/// The list row's board of a game at `state`.
pub fn daily_board(state: &GameState) -> DailyBoard {
    let snapshot = snapshot(state);
    DailyBoard {
        turn: snapshot.turn,
        life: PerPlayer::new(state.players.p1.hero.health, state.players.p2.hero.health),
        seats_swapped: snapshot.seats_swapped,
        result: snapshot.result,
    }
}
