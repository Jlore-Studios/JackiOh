//! What a daily game's "your move" notifications send (SPEC §9.13, #3727), pure: decided from the games
//! waiting on one account, what was last sent and the account's settings; the server only sends it.
//!
//! A channel last sent at `L` is due when a game started waiting on the account after `L`, or a
//! waiting game's reminder (`DAILY_REMINDER_HOURS` before its deadline) came due after `L`. An email
//! goes out only when it is due, the account wants email and has an address, and
//! `DAILY_EMAIL_MIN_GAP_HOURS` have passed since the last one (§9.8). A push needs a device and
//! nothing else. Each lists every game waiting, the nearest deadline first, so a held email loses none.

use serde::{Deserialize, Serialize};

use crate::config::{DAILY_EMAIL_MIN_GAP_HOURS, DAILY_REMINDER_HOURS, MS_PER_HOUR};

/// One daily game waiting on the account: since when, and until when.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WaitingGame {
    pub match_id: String,
    /// Epoch ms the move passed to the account.
    pub since: i64,
    /// Epoch ms the move runs out.
    pub deadline: i64,
}

/// Everything the decision reads of one account.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NotifyInput {
    /// The account's setting: email on or off.
    pub email_on: bool,
    /// Whether the account has an email address (an account signed in with Roblox has none, R3147000).
    pub has_email: bool,
    /// The devices that turned push on.
    pub push_devices: u32,
    pub last_email_at: Option<i64>,
    pub last_push_at: Option<i64>,
    pub waiting: Vec<WaitingGame>,
}

/// What to send: an email, a push, and the games each lists, the nearest deadline first.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NotifyPlan {
    pub email: bool,
    pub push: bool,
    pub games: Vec<WaitingGame>,
}

/// When a waiting game's reminder comes due.
fn reminder_at(game: &WaitingGame) -> i64 {
    game.deadline - DAILY_REMINDER_HOURS * MS_PER_HOUR
}

/// Whether a channel last sent at `last` has something new to say at `now`.
fn due(waiting: &[WaitingGame], last: Option<i64>, now: i64) -> bool {
    let Some(last) = last else {
        return !waiting.is_empty();
    };
    waiting
        .iter()
        .any(|game| game.since > last || (now >= reminder_at(game) && reminder_at(game) > last))
}

/// The plan for one account at `now`.
pub fn decide_notify(input: &NotifyInput, now: i64) -> NotifyPlan {
    let mut games = input.waiting.clone();
    games.sort_by(|a, b| {
        a.deadline
            .cmp(&b.deadline)
            .then_with(|| a.match_id.cmp(&b.match_id))
    });
    let gap_kept = input
        .last_email_at
        .is_none_or(|last| now - last >= DAILY_EMAIL_MIN_GAP_HOURS * MS_PER_HOUR);
    let email = input.email_on && input.has_email && gap_kept && due(&games, input.last_email_at, now);
    let push = input.push_devices > 0 && due(&games, input.last_push_at, now);
    NotifyPlan { email, push, games }
}
