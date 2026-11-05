//! The ranked ladder's rules (SPEC §9.12, R603–R612), pure: the hidden rating ([`glicko2`], Glicko-2), the
//! visible ladder ([`ladder`], the Grape tiers, pips, placements, Jlorious), the seasons ([`season`], which
//! season a build plays in and the soft reset that opens one), one rated game decided as data ([`game`]) and
//! the integer a leaderboard sorts a rating record by ([`board`]).
//!
//! The server reads its store, calls these natively and writes what they return (`crates/server`'s
//! `api/ranked.rs` and `db/`; the leaderboards' queries and the timers stay there). A Roblox host (#2898)
//! keeps the same records in an OrderedDataStore and a MemoryStore and calls the same code through
//! `jackioh-wasm-core`'s `ranked` body: [`RankedCall`] in, [`RankedAnswer`] out, both JSON. Every one reads
//! the time and the season's dates as arguments (README.md, the host contract). There is no decay apart from
//! the two widenings R609 and R1560000 state: a season's soft reset and an AI promotion each add to a
//! deviation, once, at the moment they happen.

pub mod board;
pub mod game;
pub mod glicko2;
pub mod ladder;
pub mod season;

use serde::{Deserialize, Serialize};

use self::game::{RateGameInput, RatedDecision, decide};
use self::ladder::{PeakBadge, SeasonRank, Standing, VisibleRank, jlorious_order, peak_badge, visible_rank};
use self::season::{ResetPlayer, Season, SeasonPlan, SoftReset, plan_season, soft_reset};

/// One call of this module as a host makes it through `jackioh-wasm-core`'s `ranked` body: the function and
/// its arguments as one JSON object, `{ "call": "rateGame", "standings": …, "sides": …, … }`. The host holds the
/// records (the standings, the seasons) and hands them back on every call; it passes the time in
/// (`nowMs`, epoch ms). The server calls the functions themselves.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "call", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum RankedCall {
    /// [`decide`]: one rated game.
    RateGame { game: Box<RateGameInput> },
    /// [`plan_season`]: the season a build plays in, among the seasons that exist.
    PlanSeason {
        patch_version: String,
        seasons: Vec<Season>,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now_ms: i64,
    },
    /// [`soft_reset`]: the reset of one ladder's rated players.
    SoftReset { players: Vec<ResetPlayer> },
    /// [`visible_rank`]: what a player is shown as.
    VisibleRank {
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional))]
        rank: Option<SeasonRank>,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional))]
        jlorious_position: Option<i32>,
    },
    /// [`peak_badge`]: a season's best, as the profile's badge shows it.
    PeakBadge { rank: Box<SeasonRank> },
    /// [`jlorious_order`]: the ids of Jlorious, #1 first, over the standings that count.
    JloriousOrder { standings: Vec<Standing> },
}

/// What a [`RankedCall`] answers, as one JSON object with one key.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum RankedAnswer {
    /// `rateGame`: the new ratings, rows and records, to write.
    Game(Box<RatedDecision>),
    /// `planSeason`.
    Season(SeasonPlan),
    /// `softReset`.
    Reset(SoftReset),
    /// `visibleRank`.
    Rank(VisibleRank),
    /// `peakBadge`: null for a season whose placements were never finished.
    Badge(Option<PeakBadge>),
    /// `jloriousOrder`.
    Jlorious(Vec<String>),
}

/// Makes `call`: the function it names, over its arguments, answered as data. Nothing is read but the call,
/// and nothing is kept.
pub fn apply(call: &RankedCall) -> RankedAnswer {
    match call {
        RankedCall::RateGame { game } => RankedAnswer::Game(Box::new(decide(game))),
        RankedCall::PlanSeason {
            patch_version,
            seasons,
            now_ms,
        } => RankedAnswer::Season(plan_season(seasons, patch_version, *now_ms)),
        RankedCall::SoftReset { players } => RankedAnswer::Reset(soft_reset(players)),
        RankedCall::VisibleRank {
            rank,
            jlorious_position,
        } => RankedAnswer::Rank(visible_rank(rank.as_ref(), *jlorious_position)),
        RankedCall::PeakBadge { rank } => RankedAnswer::Badge(peak_badge(rank)),
        RankedCall::JloriousOrder { standings } => RankedAnswer::Jlorious(jlorious_order(standings)),
    }
}
