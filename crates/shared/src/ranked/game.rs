//! One rated game, decided (SPEC §9.12, R603–R608, R610, R611, R640000, R672, R1560000, R1843004, R3074001):
//! both sides' new hidden ratings, each side's season row moved toward the rank its new rating calls for, the
//! Jlorious peaks it leaves, and the rating records with their leaderboard keys. A pure function over plain
//! records, `decide(input)`: the ladder's standings, the two sides, the result, the season and the time come
//! in; the new ratings, rows and records come out as data, for the host to write (the server's store, an
//! OrderedDataStore). It reads no store, clock or environment.
//!
//! The server reads the standings, the profiles and the bots' rows, calls [`decide`] and writes what it
//! returns in one transaction (`api/ranked.rs`); a client reaches the same function through
//! [`super::RankedCall::RateGame`].

use std::collections::BTreeSet;

use indexmap::IndexMap;
use jackioh_engine::GameOverReason;
use jackioh_engine::config::LOSER_DRAWS_REASONS;
use serde::{Deserialize, Serialize};

use super::board::{RatingRecord, SeasonStanding, counts, record_of, standings_of};
use super::glicko2::{Glicko, rate_game_scored, widen_for_promotions};
use super::ladder::{
    ApplyRankedGameInput, GameResult, Ladder, SeasonRank, VisibleRank, apply_ranked_game, fresh_rank,
    jlorious_order, percentile_of, target_ladder, visible_rank, with_jlorious_peak,
};
use super::season::Season;
use crate::config::STAKE_DOUBLE;
use crate::series::SeriesEnd;

/// One side of a ranked game: a player's profile, or one of the AI bots (R610).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum RankedSideInput {
    Player { profile_id: String },
    Bot { bot_id: String },
}

/// How a rated game ended: the match's reason, or the series' (R334), whose literals are disjoint.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(untagged)]
pub enum RatedReason {
    Game(GameOverReason),
    Series(SeriesEnd),
}

/// Glicko-2's score for a side's result: a win, a draw or a loss.
fn score_of(result: GameResult) -> f64 {
    match result {
        GameResult::Win => 1.0,
        GameResult::Draw => 0.5,
        GameResult::Loss => 0.0,
    }
}

/// The game's result for `side` (0 or 1). R1843004: an ending that leaves the winner's opponent a draw (the
/// engine's `LOSER_DRAWS_REASONS`, a `fled` game) is the winner's win and the other side's draw; a series'
/// reasons never do.
pub fn result_of(winner_side: Option<i32>, reason: &RatedReason, side: i32) -> GameResult {
    let loser_draws = matches!(
        reason,
        RatedReason::Game(reason) if LOSER_DRAWS_REASONS.contains(reason)
    );
    match winner_side {
        None => GameResult::Draw,
        Some(winner) if winner == side => GameResult::Win,
        Some(_) if loser_draws => GameResult::Draw,
        Some(_) => GameResult::Loss,
    }
}

/// R1560000: the AI promotions a bot's row has not been rated under: the generations since the one it
/// names (`rated_on`), or one for a row that names none (not rated since R1560000). None when the build
/// names no generation (`ai_generation`, `crates/ai/generation.json`).
pub fn promotions_since(ai_generation: Option<i64>, rated_on: Option<i64>) -> i64 {
    match (ai_generation, rated_on) {
        (None, _) => 0,
        (Some(_), None) => 1,
        (Some(now), Some(then)) => now - then,
    }
}

/// One side going into a rated game, as the host read it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RateSide {
    pub side: RankedSideInput,
    /// The rating the side goes in with: a person's profile's on this ladder, a bot's row's; a profile that is
    /// gone, and a bot with no row, start from `START_GLICKO`.
    pub glicko: Glicko,
    /// R1560000: the side is a ladder bot with a rating row, so its deviation is widened by the promotions
    /// since its row was rated (`rated_on`).
    #[serde(default)]
    pub bot_row: bool,
    /// R1560000: the AI generation that row was last rated on; null for a row not rated since R1560000.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional, type = "number"))]
    pub rated_on: Option<i64>,
    /// R640000, R610: a bot side whose id names a bot profile holds a season row like a person's; a bare
    /// rating (a bot id with no bot profile) holds none. Read for bot sides only: a person's side always holds one.
    #[serde(default)]
    pub bot_profile: bool,
}

/// Everything [`decide`] reads: `standings` is every season row of `ladder` in `season` with its rating (the
/// store's `ranked_standings`), `winner_side` the index of the side that won (none for a draw), `strict` the
/// server's `DEV_STRICT` (R3074001), `ai_generation` this build's (R1560000) and `now_ms` the game's end,
/// epoch milliseconds.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RateGameInput {
    pub standings: Vec<SeasonStanding>,
    pub sides: (RateSide, RateSide),
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub winner_side: Option<i32>,
    pub reason: RatedReason,
    /// R672: a double-or-nothing rematch's stake; only `STAKE_DOUBLE` changes anything.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stake: Option<i32>,
    pub season: Season,
    /// R2778000: the ladder the game moves.
    pub ladder: Ladder,
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional, type = "number"))]
    pub ai_generation: Option<i64>,
    #[serde(default)]
    pub strict: bool,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub now_ms: i64,
}

/// One side's rating and visible rank, before and after the game (R611). A bot's ranks are tier-only, never a
/// Jlorious place (R640000), and a bare rating's (R610) are null.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct SideOutcome {
    /// The rating the game rated the side from: `RateSide::glicko`, widened for a bot's promotions.
    pub before: Glicko,
    pub after: Glicko,
    pub rank_before: Option<VisibleRank>,
    pub rank_after: Option<VisibleRank>,
}

/// One bystander's Jlorious peak.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PeakWrite {
    pub profile_id: String,
    pub position: i32,
}

/// What a rated game writes, decided before any of it is written.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RatedDecision {
    pub sides: (SideOutcome, SideOutcome),
    /// The season rows the game moved, in side order: one per side that holds one, with the Jlorious peak it
    /// has just held.
    pub ranks: Vec<SeasonRank>,
    /// Bystanders whose Jlorious position this game improved: their peaks to write.
    pub peaks: Vec<PeakWrite>,
    /// R2755000: the people this game leaves Jlorious who may not own the Jlorious card back yet: its own
    /// people in Jlorious and each bystander whose peak it writes. The host leaves it out while its economy
    /// is off (§14.12).
    pub jlorious: Vec<String>,
    /// The rating records to write to a leaderboard store (`board.rs`): the sides' that hold a row, and each
    /// bystander the game brings into Jlorious or pushes out of it, whose key moves with it. Any other record
    /// keeps its key.
    pub records: Vec<RatingRecord>,
}

/// A season row's move: as it stood, and after this game. `bot` marks a ladder bot's (R640000), `hand_set` a
/// hand-set rating's (R3074001).
struct RankChange {
    before: Option<SeasonRank>,
    after: SeasonRank,
    bot: bool,
    hand_set: bool,
    rating: f64,
}

/// A 1-based position in Jlorious, or none.
pub fn position_in(order: &[String], profile_id: &str) -> Option<i32> {
    order
        .iter()
        .position(|candidate| candidate == profile_id)
        .map(|at| at as i32 + 1)
}

/// The profile id a side's season row is under, and whether it is a ladder bot's, or none for a bare rating.
fn row_holder(side: &RateSide) -> Option<(&str, bool)> {
    match &side.side {
        RankedSideInput::Player { profile_id } => Some((profile_id.as_str(), false)),
        RankedSideInput::Bot { bot_id } => side.bot_profile.then_some((bot_id.as_str(), true)),
    }
}

/// Decides one ranked game (R603–R608): both sides rated against each other's rating from before it, each
/// side's season moved toward the rank its new rating calls for, and any Jlorious peaks. Both targets read
/// the season as before the game with both new ratings in it, so neither placement depends on which is
/// computed first. R640000: a ladder bot's season row moves as a person's, its target read against people's
/// ratings alone, never in Jlorious; a bot id with no bot profile (R610's bare rating) holds no row.
pub fn decide(input: &RateGameInput) -> RatedDecision {
    let sides = [&input.sides.0, &input.sides.1];
    let (standings, strict, on) = (&input.standings, input.strict, input.ladder);

    // R1560000: a bot last rated on an older AI is rated from a widened deviation.
    let before: [Glicko; 2] = [0, 1].map(|index| {
        let side = sides[index];
        if side.bot_row {
            widen_for_promotions(&side.glicko, promotions_since(input.ai_generation, side.rated_on))
        } else {
            side.glicko
        }
    });
    let result_for = |index: usize| result_of(input.winner_side, &input.reason, index as i32);
    // R1843004: each side's own score, so a fled game rates the fleer's win and the other's draw.
    let rated = rate_game_scored(
        &before[0],
        &before[1],
        score_of(result_for(0)),
        score_of(result_for(1)),
    );
    // R672: a double-or-nothing rematch doubles each side's rating movement around its own before:
    // one update, then the delta twice. Deviation and volatility are the single update's — a doubled
    // game moves the rating twice without sharpening the confidence twice.
    let after: [Glicko; 2] = if input.stake == Some(STAKE_DOUBLE) {
        [
            Glicko {
                rating: before[0].rating + 2.0 * (rated.a.rating - before[0].rating),
                ..rated.a
            },
            Glicko {
                rating: before[1].rating + 2.0 * (rated.b.rating - before[1].rating),
                ..rated.b
            },
        ]
    } else {
        [rated.a, rated.b]
    };

    let mut new_rating: IndexMap<String, f64> = IndexMap::new();
    for (index, side) in sides.iter().enumerate() {
        if let Some((profile_id, _)) = row_holder(side) {
            new_rating.insert(profile_id.to_string(), after[index].rating);
        }
    }
    let re_rated: Vec<SeasonStanding> = standings
        .iter()
        .map(|standing| {
            let mut standing = standing.clone();
            if let Some(rating) = new_rating.get(&standing.rank.profile_id) {
                standing.rating = *rating;
            }
            standing
        })
        .collect();
    // R3074001: a strict server leaves a hand-set rating out of the percentiles and Jlorious.
    let jlorious_before = jlorious_order(&standings_of(standings, strict));

    let mut ranks: IndexMap<String, RankChange> = IndexMap::new();
    for (index, side) in sides.iter().enumerate() {
        let Some((profile_id, bot)) = row_holder(side) else {
            continue;
        };
        let standing = standings
            .iter()
            .find(|candidate| candidate.rank.profile_id == profile_id);
        let row_before = standing.map(|standing| standing.rank.clone());
        // R640000: percentiles are people's, so a bot's row moves no person's target; R3074001: on a
        // strict server, nor does a hand-set rating's.
        let others: Vec<f64> = re_rated
            .iter()
            .filter(|candidate| {
                candidate.rank.profile_id != profile_id
                    && candidate.rank.ladder.is_some()
                    && counts(candidate, strict)
            })
            .map(|candidate| candidate.rating)
            .collect();
        let target = target_ladder(&percentile_of(after[index].rating, &others));
        let start = row_before.clone().unwrap_or_else(|| SeasonRank {
            format: on,
            ..fresh_rank(&input.season.id, profile_id, input.now_ms)
        });
        let row_after = apply_ranked_game(
            &start,
            &ApplyRankedGameInput {
                result: result_for(index),
                target,
                at: input.now_ms,
            },
        );
        ranks.insert(
            profile_id.to_string(),
            RankChange {
                before: row_before,
                after: row_after,
                bot,
                hand_set: standing.is_some_and(|standing| standing.hand_set),
                rating: after[index].rating,
            },
        );
    }

    let mut standings_after: Vec<SeasonStanding> = re_rated
        .iter()
        .filter(|standing| !ranks.contains_key(&standing.rank.profile_id))
        .cloned()
        .collect();
    for change in ranks.values() {
        standings_after.push(SeasonStanding {
            rank: change.after.clone(),
            rating: change.rating,
            bot: change.bot,
            hand_set: change.hand_set,
        });
    }
    let jlorious_after = jlorious_order(&standings_of(&standings_after, strict));
    let mut peaks: Vec<PeakWrite> = Vec::new();
    let mut jlorious: Vec<String> = Vec::new();
    for (index, profile_id) in jlorious_after.iter().enumerate() {
        let position = index as i32 + 1;
        if let Some(own) = ranks.get_mut(profile_id) {
            own.after = with_jlorious_peak(&own.after, position);
            jlorious.push(profile_id.clone());
            continue;
        }
        let standing = standings_after
            .iter()
            .find(|candidate| candidate.rank.profile_id == *profile_id);
        if let Some(standing) = standing
            && standing.rank.peak_jlorious.is_none_or(|peak| position < peak)
        {
            peaks.push(PeakWrite {
                profile_id: profile_id.clone(),
                position,
            });
            jlorious.push(profile_id.clone());
        }
    }

    let outcome = |index: usize| -> SideOutcome {
        let side = sides[index];
        let (rank_before, rank_after) = match &side.side {
            // R640000: a ladder bot's side carries its season row's ranks, never a Jlorious place;
            // a bare rating's (R610) carries none.
            RankedSideInput::Bot { bot_id } => (
                ranks
                    .get(bot_id)
                    .map(|rank| visible_rank(rank.before.as_ref(), None)),
                ranks
                    .get(bot_id)
                    .map(|rank| visible_rank(Some(&rank.after), None)),
            ),
            RankedSideInput::Player { profile_id } => {
                let rank = ranks.get(profile_id);
                (
                    Some(visible_rank(
                        rank.and_then(|rank| rank.before.as_ref()),
                        position_in(&jlorious_before, profile_id),
                    )),
                    Some(visible_rank(
                        rank.map(|rank| &rank.after),
                        position_in(&jlorious_after, profile_id),
                    )),
                )
            }
        };
        SideOutcome {
            before: before[index],
            after: after[index],
            rank_before,
            rank_after,
        }
    };

    // The leaderboard's records: the sides', and the bystanders the game moved into or out of Jlorious.
    let now_in: BTreeSet<&str> = jlorious_after.iter().map(String::as_str).collect();
    let was_in: BTreeSet<&str> = jlorious_before.iter().map(String::as_str).collect();
    let mut records: Vec<RatingRecord> = ranks
        .iter()
        .map(|(profile_id, change)| {
            let standing = SeasonStanding {
                rank: change.after.clone(),
                rating: change.rating,
                bot: change.bot,
                hand_set: change.hand_set,
            };
            record_of(&standing, now_in.contains(profile_id.as_str()), strict)
        })
        .collect();
    for profile_id in jlorious_before.iter().chain(&jlorious_after) {
        let moved = was_in.contains(profile_id.as_str()) != now_in.contains(profile_id.as_str());
        if !moved || ranks.contains_key(profile_id) {
            continue;
        }
        if let Some(standing) = standings_after
            .iter()
            .find(|candidate| candidate.rank.profile_id == *profile_id)
        {
            records.push(record_of(standing, now_in.contains(profile_id.as_str()), strict));
        }
    }

    RatedDecision {
        sides: (outcome(0), outcome(1)),
        ranks: ranks.values().map(|rank| rank.after.clone()).collect(),
        peaks,
        jlorious,
        records,
    }
}
