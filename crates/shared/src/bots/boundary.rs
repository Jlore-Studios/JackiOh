//! The ladder bots' JSON boundary: one tagged call in, one tagged answer out (the host contract,
//! `crates/shared/README.md`). `jackioh-wasm-core`'s `bots(call)` parses a [`BotCall`], runs [`apply`] and
//! writes the [`BotAnswer`]; it decides nothing. A bot at a seat is the match host's machine's
//! (`host::boundary`); these are the decisions around it, over plain records the host read from its store
//! ([`BotFacts`], [`TicketFacts`], [`GameFacts`]): the pool's size, who grinds, a new bot, its decks and
//! trio, the empty-queue pairing, a series pick and the cosmetics it wears. The decision a bot's move is
//! (`bots::decide`) is a call of its own, `jackioh-wasm-core`'s `bot_decide`, since it may be given a clock.

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

use jackioh_engine::WinRateTable;
use jackioh_engine::validator::CatalogSnapshot;
use jackioh_engine::wire::ListTotal;

use crate::bots::cosmetics::{BotDraw, CosmeticOwners, Holding, eligible, redraw};
use crate::bots::fallback::{
    BotMode, RatedGameRecord, TicketRecord, bot_match_gap_passed, bot_matches, choose_bot, choose_bot_pair,
    fallback_due_keyed, fallback_seconds_keyed, hour_of,
};
use crate::bots::pick::{bot_pick, pick_due_at, pick_is_due, pick_slot};
use crate::bots::pool::{
    BotDeck, BotFacts, BotLadder, NewBot, PoolChange, bot_target_strength, draw_decks, draw_new_bot,
    draw_trio, plan_grind, plan_pool, pool_target,
};
use crate::config::BOT_COSMETIC_FITS;
use crate::cosmetics::registry;
use crate::deck::FrozenTrio;
use crate::series::{SeriesRefusal, SeriesRow, SeriesSeat};

/// A waiting ticket as a plain record (what the fallback reads of it).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct TicketFacts {
    pub id: String,
    /// Its player's rating on the ladder of its mode.
    pub rating: f64,
    pub mode: BotMode,
    /// Epoch ms it was queued.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub enqueued_at: i64,
}

impl TicketRecord for TicketFacts {
    fn id(&self) -> &str {
        &self.id
    }
    fn rating(&self) -> f64 {
        self.rating
    }
    fn mode(&self) -> BotMode {
        self.mode
    }
    fn enqueued_at(&self) -> i64 {
        self.enqueued_at
    }
}

/// A finished rated game as a plain record (what the bots' kickoff reads of it).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct GameFacts {
    /// Whether both sides were bots.
    pub between_bots: bool,
    /// Epoch ms it ended.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub ended_at: i64,
}

impl RatedGameRecord for GameFacts {
    fn between_bots(&self) -> bool {
        self.between_bots
    }
    fn ended_at(&self) -> i64 {
        self.ended_at
    }
}

/// One call of the ladder bots' decisions.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "call", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum BotCall {
    /// R1454: the pool's size for `weekly` people who finished a live game in the window.
    PoolTarget {
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        weekly: i64,
    },
    /// R1454: what brings the pool's active bots to `target`.
    PlanPool { bots: Vec<BotFacts>, target: u32 },
    /// R1555000: the bots that grind, given the people's ratings on `ladder`.
    PlanGrind {
        bots: Vec<BotFacts>,
        people: Vec<f64>,
        ladder: BotLadder,
    },
    /// R1454: a new bot drawn from `seed`. Its username is the host's to give (the answer's name is
    /// always none): the account filter is the host's.
    NewBot {
        seed: String,
        profile_id: String,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
    },
    /// R750000: how far the strength at index `strength` leans its decks on winning cards.
    BotTargetStrength { strength: u32 },
    /// R1454: legal Best-of-1 decks for a bot, from human decklists first.
    DrawDecks {
        seed: String,
        #[cfg_attr(feature = "ts", ts(type = "unknown"))]
        lists: Vec<ListTotal>,
        catalog: Box<CatalogSnapshot>,
        strength: f64,
        /// R3049002: the win-rate table the random decks lean on, as the server hands its snapshot of
        /// its cached card stats; omitted, they lean on nothing.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        win_rates: Option<WinRateTable>,
    },
    /// R641000: a Conquest trio for a bot, from its Best-of-1 decks first.
    DrawTrio {
        seed: String,
        own: Vec<BotDeck>,
        catalog: Box<CatalogSnapshot>,
        strength: f64,
        /// R3049002: as `DrawDecks`' `winRates`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        win_rates: Option<WinRateTable>,
    },
    /// R1453: the ticket's fallback time under the host's secret `key`.
    FallbackSeconds { key: String, ticket_id: String },
    /// R1453, R641000: whether the ticket may pair with a bot at `now`. `hidden`: the bots pass for
    /// people (`BotSeat.hidden`); if not, every ticket is due.
    FallbackDue {
        key: String,
        ticket: TicketFacts,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
        hidden: bool,
    },
    /// The hour of the day (UTC) a bot's schedule is read at.
    HourOf {
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
    },
    /// R1453: the bot the ticket pairs with.
    ChooseBot {
        ticket: TicketFacts,
        bots: Vec<BotFacts>,
        recent: Vec<String>,
        busy: Vec<String>,
        hour: i32,
        catalog_version: String,
    },
    /// R1555000: the two bots a match between two bots seats; `recent` is each bot's recent opponents.
    ChooseBotPair {
        bots: Vec<BotFacts>,
        recent: Vec<(String, Vec<String>)>,
        busy: Vec<String>,
        hour: i32,
        catalog_version: String,
    },
    /// R1555000: the matches two or more bots are in, from each bot's match id.
    BotMatches { in_match: Vec<Option<String>> },
    /// R1555000: whether the gap since the last game between two bots has passed.
    BotMatchGapPassed {
        games: Vec<GameFacts>,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
    },
    /// R641000: when the bot in `seat` makes its pick.
    PickDueAt {
        series: Box<SeriesRow>,
        seat: SeriesSeat,
    },
    /// R641000: whether the bot in `seat` picks at `now`.
    PickIsDue {
        series: Box<SeriesRow>,
        seat: SeriesSeat,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
    },
    /// R641000: the deck the bot in `seat` picks for the next game, by the registered catalog.
    PickSlot {
        series: Box<SeriesRow>,
        seat: SeriesSeat,
    },
    /// R641000: the bot's pick as a series transition (R331).
    BotPick {
        series: Box<SeriesRow>,
        seat: SeriesSeat,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
    },
    /// R1834005: the non-starter cosmetics a bot may own as of `now`, from how many people own each and
    /// when it was first granted to one, judged against the shipped registry.
    EligibleCosmetics {
        owners: Vec<CosmeticOwners>,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
    },
    /// R1834005: what `bot` draws of its cosmetics at a refresh at `now` from the pool's `seed`, over the
    /// `eligible` holdings, by the compiled-in fits (`BOT_COSMETIC_FITS`) and the shipped registry; none when
    /// nothing is due.
    RedrawCosmetics {
        seed: String,
        bot: Box<BotFacts>,
        eligible: Vec<Holding>,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
    },
}

/// What a [`BotCall`] answers.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "answer", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum BotAnswer {
    Count {
        count: u32,
    },
    Change {
        change: PoolChange,
    },
    Ids {
        ids: Vec<String>,
    },
    Bot {
        bot: Box<NewBot>,
    },
    Share {
        share: f64,
    },
    Decks {
        decks: Vec<BotDeck>,
    },
    Trio {
        trio: Option<Box<FrozenTrio>>,
    },
    Seconds {
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        seconds: i64,
    },
    Due {
        due: bool,
    },
    Hour {
        hour: i32,
    },
    /// The id of the bot chosen, or none.
    Chosen {
        bot: Option<String>,
    },
    /// The ids of the pair chosen, or none.
    Pair {
        pair: Option<(String, String)>,
    },
    Passed {
        passed: bool,
    },
    At {
        #[cfg_attr(feature = "ts", ts(type = "number | null"))]
        at: Option<i64>,
    },
    Slot {
        slot: Option<u32>,
    },
    /// The series row to write back.
    Row {
        row: Box<SeriesRow>,
    },
    /// R331: the pick the series refuses, as data.
    Refused {
        refused: SeriesRefusal,
    },
    /// The holdings a bot may own, sorted: a cosmetic id, and the card for a per-card one.
    Holdings {
        holdings: Vec<Holding>,
    },
    /// What a refresh changes of a bot's cosmetics (and of its decks and trio, when a deck was given its
    /// slots), or none when nothing is due.
    Dressed {
        draw: Option<Box<BotDraw>>,
    },
}

fn set(ids: Vec<String>) -> IndexSet<String> {
    ids.into_iter().collect()
}

/// The one function behind the boundary. The deals and the pick's rule read the registered catalog.
pub fn apply(call: BotCall) -> BotAnswer {
    match call {
        BotCall::PoolTarget { weekly } => BotAnswer::Count {
            count: u32::try_from(pool_target(weekly)).unwrap_or(u32::MAX),
        },
        BotCall::PlanPool { bots, target } => BotAnswer::Change {
            change: plan_pool(&bots, usize::try_from(target).unwrap_or(usize::MAX)),
        },
        BotCall::PlanGrind { bots, people, ladder } => BotAnswer::Ids {
            ids: plan_grind(&bots, &people, ladder),
        },
        BotCall::NewBot {
            seed,
            profile_id,
            now,
        } => {
            let (bot, _) = draw_new_bot(&seed, &profile_id, now, &mut |_| None);
            BotAnswer::Bot { bot: Box::new(bot) }
        }
        BotCall::BotTargetStrength { strength } => BotAnswer::Share {
            share: bot_target_strength(usize::try_from(strength).unwrap_or(usize::MAX)),
        },
        BotCall::DrawDecks {
            seed,
            lists,
            catalog,
            strength,
            win_rates,
        } => BotAnswer::Decks {
            decks: draw_decks(&seed, &lists, &catalog, strength, win_rates.as_ref()),
        },
        BotCall::DrawTrio {
            seed,
            own,
            catalog,
            strength,
            win_rates,
        } => BotAnswer::Trio {
            trio: draw_trio(&seed, &own, &catalog, strength, win_rates.as_ref()).map(Box::new),
        },
        BotCall::FallbackSeconds { key, ticket_id } => BotAnswer::Seconds {
            seconds: fallback_seconds_keyed(&key, &ticket_id),
        },
        BotCall::FallbackDue {
            key,
            ticket,
            now,
            hidden,
        } => BotAnswer::Due {
            due: fallback_due_keyed(&key, &ticket, now, hidden),
        },
        BotCall::HourOf { now } => BotAnswer::Hour { hour: hour_of(now) },
        BotCall::ChooseBot {
            ticket,
            bots,
            recent,
            busy,
            hour,
            catalog_version,
        } => BotAnswer::Chosen {
            bot: choose_bot(&ticket, &bots, &recent, &set(busy), hour, &catalog_version)
                .map(|bot| bot.profile_id.clone()),
        },
        BotCall::ChooseBotPair {
            bots,
            recent,
            busy,
            hour,
            catalog_version,
        } => {
            let recent: IndexMap<String, Vec<String>> = recent.into_iter().collect();
            BotAnswer::Pair {
                pair: choose_bot_pair(&bots, &recent, &set(busy), hour, &catalog_version)
                    .map(|(first, second)| (first.profile_id.clone(), second.profile_id.clone())),
            }
        }
        BotCall::BotMatches { in_match } => BotAnswer::Ids {
            ids: bot_matches(in_match.iter().map(Option::as_deref)),
        },
        BotCall::BotMatchGapPassed { games, now } => BotAnswer::Passed {
            passed: bot_match_gap_passed(&games, now),
        },
        BotCall::PickDueAt { series, seat } => BotAnswer::At {
            at: pick_due_at(&series, seat),
        },
        BotCall::PickIsDue { series, seat, now } => BotAnswer::Due {
            due: pick_is_due(&series, seat, now),
        },
        BotCall::PickSlot { series, seat } => BotAnswer::Slot {
            slot: pick_slot(&series, seat, jackioh_engine::registered_catalog())
                .and_then(|slot| u32::try_from(slot).ok()),
        },
        BotCall::BotPick { series, seat, now } => {
            match bot_pick(&series, seat, jackioh_engine::registered_catalog(), now) {
                Ok(row) => BotAnswer::Row { row: Box::new(row) },
                Err(refused) => BotAnswer::Refused { refused },
            }
        }
        BotCall::EligibleCosmetics { owners, now } => BotAnswer::Holdings {
            holdings: eligible(registry(), &owners, now),
        },
        BotCall::RedrawCosmetics {
            seed,
            bot,
            eligible,
            now,
        } => BotAnswer::Dressed {
            draw: redraw(&seed, &*bot, BOT_COSMETIC_FITS, registry(), &eligible, now).map(Box::new),
        },
    }
}
