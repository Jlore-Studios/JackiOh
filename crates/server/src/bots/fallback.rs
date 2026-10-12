//! The empty-queue fallback (SPEC §9.5, R1453, R641000): which waiting ticket may be paired with a
//! ladder bot, and with which bot. Pure but for the fallback key: the queue's sweep (`api/queue.rs`,
//! `try_pair`) reads the tickets, the bots and each player's recent opponents, and asks these
//! functions.
//!
//! A ranked ticket of any mode that has waited past its own fallback time — drawn from its id and
//! the server's key between `BOT_FALLBACK_MIN_SECONDS` and `BOT_FALLBACK_MAX_SECONDS`, so it is the
//! same at every sweep and no client can work it out — pairs with the free bot on its schedule whose
//! rating is closest to the player's, never one the player faced in their last
//! `BOT_RECENT_OPPONENTS` games. A player rated above every bot is never given one. A Conquest
//! ticket's bot must hold a trio dealt for this catalog (R641000), and plays a series rather than a
//! match. The sweep pairs every human it can first, so a human inside the window always comes before
//! a bot.

use std::sync::LazyLock;

use indexmap::IndexSet;
use jackioh_engine::Rng;

use crate::api::queue::new_seed;
use crate::config::{BOT_FALLBACK_MAX_SECONDS, BOT_FALLBACK_MIN_SECONDS, BOT_HOURS_PER_DAY};
use crate::db::store::{BotRow, QueueMode, Ticket};

/// Unit conversion, not configuration: the fallback time speaks seconds, tickets are stamped in ms.
const MS_PER_SECOND: i64 = 1000;
/// Unit conversion, not configuration: an hour in milliseconds.
const MS_PER_HOUR: i64 = 3_600_000;

/// R1453: the key every fallback time is drawn with beside its ticket's id, drawn as the process
/// starts drawing them and sent to no one. A ticket's id reaches its player (`ticketId`) and the
/// engine's `Rng` runs in their browser, so a time drawn from the id alone is one they could work out,
/// and a pairing that came at that moment would tell them it was a bot.
static FALLBACK_KEY: LazyLock<String> = LazyLock::new(new_seed);

/// R1453: the ticket's fallback time in seconds, from its id and this server's key, so the same at
/// every sweep it runs.
pub fn fallback_seconds(ticket_id: &str) -> i64 {
    fallback_seconds_keyed(&FALLBACK_KEY, ticket_id)
}

/// R1453: the fallback time `key` draws for the ticket, uniformly from `BOT_FALLBACK_MIN_SECONDS`
/// to `BOT_FALLBACK_MAX_SECONDS`.
pub fn fallback_seconds_keyed(key: &str, ticket_id: &str) -> i64 {
    let span = BOT_FALLBACK_MAX_SECONDS - BOT_FALLBACK_MIN_SECONDS + 1;
    BOT_FALLBACK_MIN_SECONDS
        + i64::from(
            Rng::new(&format!("{key}:{ticket_id}:bot-fallback"), 0).int(i32::try_from(span).unwrap_or(1)),
        )
}

/// R1453, R641000: whether the ticket may pair with a bot at `now` (epoch ms): a ticket of any mode
/// that has waited its fallback time.
pub fn fallback_due(ticket: &Ticket, now: i64) -> bool {
    now - ticket.enqueued_at >= fallback_seconds(&ticket.id) * MS_PER_SECOND
}

/// The hour of the day (UTC) at `now` (epoch ms), the hour a bot's schedule is read at.
pub fn hour_of(now: i64) -> i32 {
    i32::try_from(
        now.div_euclid(MS_PER_HOUR)
            .rem_euclid(i64::from(BOT_HOURS_PER_DAY)),
    )
    .unwrap_or(0)
}

/// R1454: whether the bot is on at `hour` of the day (UTC): its schedule runs `active_hours` hours
/// from `active_from_hour`, around midnight when it must.
pub fn on_schedule(bot: &BotRow, hour: i32) -> bool {
    (hour - bot.active_from_hour).rem_euclid(BOT_HOURS_PER_DAY) < bot.active_hours
}

/// R1453: the bot the ticket pairs with, or none. None when the player is rated above every bot in
/// the pool; else, of the bots in the pool, on their schedule, in no match, not `busy` (taken by
/// this sweep, or a side of a series that is not over, R641000), not among the player's `recent`
/// opponents and holding what the mode needs dealt for the catalog this server plays
/// (`catalog_version`): Best-of-1 decks (R1454) or, for Conquest, a trio (R641000); the one closest
/// in rating, ties on id.
pub fn choose_bot<'a>(
    ticket: &Ticket,
    bots: &'a [BotRow],
    recent: &[String],
    busy: &IndexSet<String>,
    hour: i32,
    catalog_version: &str,
) -> Option<&'a BotRow> {
    let pool: Vec<&BotRow> = bots.iter().filter(|bot| bot.retired_at.is_none()).collect();
    if pool.iter().all(|bot| ticket.rating > bot.rating) {
        return None;
    }
    pool.into_iter()
        .filter(|bot| on_schedule(bot, hour))
        .filter(|bot| bot.in_match_id.is_none() && !busy.contains(&bot.profile_id))
        .filter(|bot| !recent.contains(&bot.profile_id))
        .filter(|bot| match ticket.mode {
            QueueMode::Bo1 => {
                !bot.decks.is_empty() && bot.decks_catalog_version.as_deref() == Some(catalog_version)
            }
            QueueMode::Bo3 => {
                bot.trio.is_some() && bot.trio_catalog_version.as_deref() == Some(catalog_version)
            }
            QueueMode::Random => true,
        })
        .min_by(|a, b| {
            let gap_a = (a.rating - ticket.rating).abs();
            let gap_b = (b.rating - ticket.rating).abs();
            gap_a
                .partial_cmp(&gap_b)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.profile_id.cmp(&b.profile_id))
        })
}
