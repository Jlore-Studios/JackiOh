//! The empty-queue fallback (SPEC §9.5, R1453, R641000): which waiting ticket may be paired with a
//! ladder bot, and with which bot.
//!
//! A ranked ticket of any mode that has waited past its fallback time pairs with the free bot on
//! schedule closest in rating, never faced in the last `BOT_RECENT_OPPONENTS` games. Above every bot,
//! only a grinder within `MAX_FAIR_GAP` pairs (R1555000). A Conquest bot must hold a trio for this
//! catalog (R641000). While top bots grind and the queue is empty, the sweep kicks off a match
//! between two bots (`choose_bot_pair`, `bot_matches`, `bot_match_gap_passed`).

use std::sync::LazyLock;

use indexmap::{IndexMap, IndexSet};
use jackioh_engine::{DECK_SIZE, Rng};

use crate::api::queue::new_seed;
use crate::config::{
    BOT_FALLBACK_MAX_SECONDS, BOT_FALLBACK_MIN_SECONDS, BOT_GRIND_ACTIVE_HOURS, BOT_HOURS_PER_DAY,
    BOT_VS_BOT_MIN_GAP_SECONDS, MAX_FAIR_GAP, MS_PER_SECOND,
};
use crate::db::store::{BotRow, FrozenTrio, QueueMode, RatedGameRow, Ticket};

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

/// R1454, R1555000: the hours a day the bot is on: its own `active_hours`, widened to
/// `BOT_GRIND_ACTIVE_HOURS` while it grinds. It stops grinding back on its own.
pub fn scheduled_hours(bot: &BotRow) -> i32 {
    if bot.grinding {
        bot.active_hours.max(BOT_GRIND_ACTIVE_HOURS)
    } else {
        bot.active_hours
    }
}

/// R1705001: whether a deck holds `DECK_SIZE` cards, the size L2 asks of every deck a player queues.
pub(crate) fn holds_deck_size(cards: &[String]) -> bool {
    usize::try_from(DECK_SIZE).is_ok_and(|size| cards.len() == size)
}

/// R1454, R1705001: whether the bot holds Best-of-1 decks of `DECK_SIZE` cards for `catalog_version`.
pub fn holds_decks(bot: &BotRow, catalog_version: &str) -> bool {
    !bot.decks.is_empty()
        && bot.decks_catalog_version.as_deref() == Some(catalog_version)
        && bot.decks.iter().all(|deck| holds_deck_size(&deck.cards))
}

/// R641000, R1705001: whether the bot holds a Conquest trio of `DECK_SIZE` cards for `catalog_version`.
pub fn holds_trio(bot: &BotRow, catalog_version: &str) -> bool {
    bot.trio_catalog_version.as_deref() == Some(catalog_version)
        && bot
            .trio
            .as_ref()
            .is_some_and(|FrozenTrio { decks: (a, b, c), .. }| {
                [a, b, c].iter().all(|deck| holds_deck_size(&deck.cards))
            })
}

/// R1454: whether the bot is on schedule at `hour` of the day (UTC).
pub fn on_schedule(bot: &BotRow, hour: i32) -> bool {
    (hour - bot.active_from_hour).rem_euclid(BOT_HOURS_PER_DAY) < scheduled_hours(bot)
}

/// R1453: the bot the ticket pairs with, or none: on schedule, free, not recent, holding legal
/// decks for `catalog_version` at `DECK_SIZE` cards (R1454, R641000, R1705001), closest in rating.
/// Above every bot, only a grinder within `MAX_FAIR_GAP` pairs (R1555000).
pub fn choose_bot<'a>(
    ticket: &Ticket,
    bots: &'a [BotRow],
    recent: &[String],
    busy: &IndexSet<String>,
    hour: i32,
    catalog_version: &str,
) -> Option<&'a BotRow> {
    let pool: Vec<&BotRow> = bots.iter().filter(|bot| bot.retired_at.is_none()).collect();
    let above_every_bot = pool.iter().all(|bot| ticket.rating > bot.rating);
    pool.into_iter()
        // R1555000: above every bot, only a grinder within the fair gap may play them.
        .filter(|bot| !above_every_bot || (bot.grinding && ticket.rating - bot.rating <= MAX_FAIR_GAP as f64))
        .filter(|bot| on_schedule(bot, hour))
        .filter(|bot| bot.in_match_id.is_none() && !busy.contains(&bot.profile_id))
        .filter(|bot| !recent.contains(&bot.profile_id))
        .filter(|bot| match ticket.mode {
            QueueMode::Bo1 => holds_decks(bot, catalog_version),
            QueueMode::Bo3 => holds_trio(bot, catalog_version),
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

/// R1555000: the matches two or more of the bots are in, each once, in the list's order: the
/// matches between two bots live now.
pub fn bot_matches(bots: &[BotRow]) -> Vec<String> {
    let mut held: IndexMap<&str, usize> = IndexMap::new();
    for match_id in bots.iter().filter_map(|bot| bot.in_match_id.as_deref()) {
        *held.entry(match_id).or_insert(0) += 1;
    }
    held.into_iter()
        .filter(|(_, bots)| *bots >= 2)
        .map(|(match_id, _)| match_id.to_string())
        .collect()
}

/// R1555000: the earliest end (epoch ms) a game between two bots may have had for another to be
/// kicked off at `now`: `BOT_VS_BOT_MIN_GAP_SECONDS` before it.
pub fn bot_match_gap_since(now: i64) -> i64 {
    now - BOT_VS_BOT_MIN_GAP_SECONDS * MS_PER_SECOND
}

/// R1555000: whether `BOT_VS_BOT_MIN_GAP_SECONDS` have passed at `now` since every rated game
/// between two bots in `games` (`Tx::ranked_bot_games_since`) ended.
pub fn bot_match_gap_passed(games: &[RatedGameRow], now: i64) -> bool {
    let since = bot_match_gap_since(now);
    !games
        .iter()
        .any(|game| game.sides.0.bot_id.is_some() && game.sides.1.bot_id.is_some() && game.ended_at > since)
}

/// R1555000: the two bots a match between two bots seats, or none: ready on schedule, free, with
/// legal Bo1 decks (R1454, R641000, R1705001). Lowest-rated ready grinder paired with closest ready
/// opponent not in its recent opponents.
pub fn choose_bot_pair<'a>(
    bots: &'a [BotRow],
    recent: &IndexMap<String, Vec<String>>,
    busy: &IndexSet<String>,
    hour: i32,
    catalog_version: &str,
) -> Option<(&'a BotRow, &'a BotRow)> {
    let ready = |bot: &&BotRow| {
        bot.retired_at.is_none()
            && on_schedule(bot, hour)
            && bot.in_match_id.is_none()
            && !busy.contains(&bot.profile_id)
            && holds_decks(bot, catalog_version)
    };
    let mut grinders: Vec<&BotRow> = bots.iter().filter(ready).filter(|bot| bot.grinding).collect();
    grinders.sort_by(|a, b| {
        a.rating
            .total_cmp(&b.rating)
            .then_with(|| a.profile_id.cmp(&b.profile_id))
    });
    grinders.into_iter().find_map(|grinder| {
        let faced = recent.get(&grinder.profile_id).map_or(&[][..], Vec::as_slice);
        bots.iter()
            .filter(ready)
            .filter(|bot| bot.profile_id != grinder.profile_id && !faced.contains(&bot.profile_id))
            .min_by(|a, b| {
                (a.rating - grinder.rating)
                    .abs()
                    .total_cmp(&(b.rating - grinder.rating).abs())
                    .then_with(|| a.profile_id.cmp(&b.profile_id))
            })
            .map(|opponent| (grinder, opponent))
    })
}
