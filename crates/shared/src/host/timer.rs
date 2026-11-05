//! The timers the machine asks its host to keep (the host contract's "timers to set", README.md).
//!
//! The machine reads no clock and starts no timer: it states, as data, a [`TimerCommand::Set`] naming the
//! moment (epoch ms) a [`TimerKey`] is due and an `id`, and the host feeds [`crate::host::DuelEvent::TimerFired`]
//! with that key and id when the moment comes. A timer the machine has since replaced or cancelled is told
//! apart by its id, so a host that cannot take a timer back in time (a callback already running, a Roblox
//! `task.delay`) is harmless: the stale firing finds no timer armed by that id and does nothing.

use serde::{Deserialize, Serialize};

use jackioh_engine::PlayerId;

/// Which timer. One of each is armed at most, so a host keeps them by key and replaces on `Set`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TimerKey {
    /// R79: the active player's turn clock.
    Turn,
    /// R79: a prompt held by the non-active player.
    Prompt,
    /// R268: the one mulligan clock.
    Mulligan,
    /// R1563006: a question sequence's clock.
    Sequence,
    /// R79: the hard ceiling.
    Ceiling,
    /// §9.5, R744: the seat's disconnect grace.
    Grace { player: PlayerId },
    /// R738: the wait that relays the seat's newest aim.
    Aim { player: PlayerId },
    /// R2739001: the wait that relays the seat's newest choice hover.
    Hover { player: PlayerId },
    /// R639000: the wait before the bot at this home seat offers a rematch itself.
    BotOffer { player: PlayerId },
    /// R639000: the wait before the bot at this home seat answers a person's rematch offer.
    BotReply { player: PlayerId },
    /// R1452: the wait before a bot whose move could not be written decides again.
    BotRetry,
    /// R1452, R639000: the end of the think time of the bot at this home seat: its move is played.
    BotMove { player: PlayerId },
    /// R738, R639000: the moment the bot at this home seat starts to show its aim.
    BotAim { player: PlayerId },
    /// R645: the moment the next emote of the bot at this home seat is due.
    BotEmote { player: PlayerId },
    /// R645: the wait that makes a person's turn long to the bot at this home seat.
    BotLongTurn { player: PlayerId },
}

/// One instruction about a timer, in the order the machine gave it.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "command", rename_all = "camelCase")]
pub enum TimerCommand {
    /// Keep a timer for `key` that fires at `at` (epoch ms; due at once if already past), replacing any
    /// the host holds under that key.
    Set {
        key: TimerKey,
        id: u32,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        at: i64,
    },
    /// Drop the timer `id` of `key`, if the host still holds it.
    Cancel { key: TimerKey, id: u32 },
}

/// The machine's timer ledger: the next id and the commands given since the host last took them.
/// Ids are drawn from a counter in the state, so they are the same on every host that replays the same
/// events.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct Fx {
    next_id: u32,
    #[serde(skip)]
    #[cfg_attr(feature = "ts", ts(skip))]
    commands: Vec<TimerCommand>,
}

impl Fx {
    /// Arms `key` to fire at `at` and answers its id.
    pub fn set(&mut self, key: TimerKey, at: i64) -> u32 {
        self.next_id = self.next_id.wrapping_add(1);
        let id = self.next_id;
        self.commands.push(TimerCommand::Set { key, id, at });
        id
    }

    /// Takes timer `id` of `key` back.
    pub fn cancel(&mut self, key: TimerKey, id: u32) {
        self.commands.push(TimerCommand::Cancel { key, id });
    }

    /// The commands given since the last call, in order.
    pub fn take(&mut self) -> Vec<TimerCommand> {
        std::mem::take(&mut self.commands)
    }
}

/// A countdown the machine keeps: the deadline it states to players and the id of the timer that
/// reports it.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct Countdown {
    pub timer: Option<u32>,
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub deadline: Option<i64>,
}

impl Countdown {
    /// Takes the timer back and forgets the deadline.
    pub fn cancel(&mut self, key: TimerKey, fx: &mut Fx) {
        if let Some(id) = self.timer.take() {
            fx.cancel(key, id);
        }
        self.deadline = None;
    }

    /// Whether `id` is the timer this countdown armed.
    pub fn armed_by(&self, id: u32) -> bool {
        self.timer == Some(id)
    }

    /// The timer reported: let go of it without taking it back.
    pub fn take_fired(&mut self) {
        self.timer = None;
    }
}
