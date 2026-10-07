//! The visible ladder (SPEC §9.12, R605–R608): the Grape tiers a player climbs, read off the hidden
//! rating (`glicko2.rs`) without ever showing it (← `apps/server/src/ranked/ladder.ts`).
//!
//! A placed player's rank is one integer, `ladder`: the pips they stand above the bottom of Rotten
//! Grape III. Each tier has `RANK_DIVISIONS_PER_TIER` divisions of `RANK_PIPS_PER_DIVISION` pips, so
//! the division and the pips are that integer divided out, and a full division is the next one's
//! bottom (R605). Above the Grape tiers is Jlorious, which is not a ladder position at all but the top
//! `JLORIOUS_SIZE` Mythic Grape players by rating (R608), and below them all is Raisin, the season's
//! placements (R605).
//!
//! The hidden rating pulls the visible rank toward itself gently (R606): the rating's percentile
//! among the season's placed players, read off `RANK_TIER_PERCENTS`, is the rank it calls for (the
//! target), and a game a division or more away from its target leans one pip further toward it on a
//! move already heading that way — a win below the target or a loss above it.
//!
//! Pure: every function here takes what it reads and returns what it decides. `src/api/ranked.rs`
//! reads the season from the store, calls these, and writes the result back.

use serde::{Deserialize, Serialize};

use crate::config::{
    JLORIOUS_SIZE, RANK_CONVERGENCE_GAP_PIPS, RANK_CONVERGENCE_PIPS, RANK_DIVISIONS_PER_TIER,
    RANK_LOSS_PIPS, RANK_PIPS_PER_DIVISION, RANK_PLACEMENT_GAMES, RANK_STREAK_BONUS_PIPS,
    RANK_STREAK_LENGTH, RANK_TIER_PERCENTS, RANK_WIN_PIPS,
};

/// One Grape tier (TS `GrapeTier`, `(typeof GRAPE_TIERS)[number]`). Declared lowest first, so a
/// variant's discriminant is its index in `GRAPE_TIERS`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum GrapeTier {
    Rotten,
    Normal,
    Large,
    Golden,
    Mythic,
}

impl GrapeTier {
    /// The literal, as TS writes it.
    pub fn as_str(self) -> &'static str {
        match self {
            GrapeTier::Rotten => "rotten",
            GrapeTier::Normal => "normal",
            GrapeTier::Large => "large",
            GrapeTier::Golden => "golden",
            GrapeTier::Mythic => "mythic",
        }
    }
}

/// R605: the Grape tiers, lowest first. Raisin sits below them and Jlorious above.
pub const GRAPE_TIERS: &[GrapeTier] = &[
    GrapeTier::Rotten,
    GrapeTier::Normal,
    GrapeTier::Large,
    GrapeTier::Golden,
    GrapeTier::Mythic,
];

/// Every tier a player can be shown in, lowest first.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum RankTier {
    Raisin,
    Rotten,
    Normal,
    Large,
    Golden,
    Mythic,
    Jlorious,
}

impl From<GrapeTier> for RankTier {
    fn from(tier: GrapeTier) -> RankTier {
        match tier {
            GrapeTier::Rotten => RankTier::Rotten,
            GrapeTier::Normal => RankTier::Normal,
            GrapeTier::Large => RankTier::Large,
            GrapeTier::Golden => RankTier::Golden,
            GrapeTier::Mythic => RankTier::Mythic,
        }
    }
}

impl RankTier {
    /// The Grape tier this is, or `None` for Raisin and Jlorious.
    pub fn grape(self) -> Option<GrapeTier> {
        match self {
            RankTier::Rotten => Some(GrapeTier::Rotten),
            RankTier::Normal => Some(GrapeTier::Normal),
            RankTier::Large => Some(GrapeTier::Large),
            RankTier::Golden => Some(GrapeTier::Golden),
            RankTier::Mythic => Some(GrapeTier::Mythic),
            RankTier::Raisin | RankTier::Jlorious => None,
        }
    }
}

/// Pips in one Grape tier.
pub const PIPS_PER_TIER: i32 = RANK_DIVISIONS_PER_TIER * RANK_PIPS_PER_DIVISION;
/// The highest ladder position: Mythic Grape I with every pip but the last. Wins past it hold it.
pub const LADDER_TOP: i32 = GRAPE_TIERS.len() as i32 * PIPS_PER_TIER - 1;
/// The index of Mythic Grape, the tier Jlorious is drawn from and the one no streak bonus reaches.
const MYTHIC: i32 = GrapeTier::Mythic as i32;

const PERCENT: i32 = 100;

/// `RANK_TIER_PERCENTS[tier]`.
const fn percent_of(tier: GrapeTier) -> i32 {
    match tier {
        GrapeTier::Rotten => RANK_TIER_PERCENTS.rotten,
        GrapeTier::Normal => RANK_TIER_PERCENTS.normal,
        GrapeTier::Large => RANK_TIER_PERCENTS.large,
        GrapeTier::Golden => RANK_TIER_PERCENTS.golden,
        GrapeTier::Mythic => RANK_TIER_PERCENTS.mythic,
    }
}

// TS checked this when the module loaded and threw; here it is checked when the crate compiles.
// The percents are integers by type, so "whole" holds already.
const _: () = {
    let mut total = 0;
    let mut index = 0;
    while index < GRAPE_TIERS.len() {
        let share = percent_of(GRAPE_TIERS[index]);
        assert!(share > 0, "RANK_TIER_PERCENTS must be positive whole percents that sum to 100 (R606)");
        total += share;
        index += 1;
    }
    assert!(total == PERCENT, "RANK_TIER_PERCENTS must be positive whole percents that sum to 100 (R606)");
};

/// A game's result for one side.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum GameResult {
    Win,
    Loss,
    Draw,
}

/// One player's season on the ladder: the store's `season_ranks` row (R605). `ladder` is null until
/// the placements are played. `floor` is the Grape tier the player has reached this season (its index
/// in `GRAPE_TIERS`), which the ladder never drops below (R607); `peak_ladder` and `peak_jlorious` are
/// the best the season has seen, for the profile's badge.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SeasonRank {
    pub season_id: String,
    pub profile_id: String,
    /// Rated games played this season, placements included.
    pub games: i32,
    pub wins: i32,
    pub losses: i32,
    pub draws: i32,
    pub ladder: Option<i32>,
    pub floor: i32,
    /// Consecutive wins, ended by a loss. A draw neither extends nor ends it (R606).
    pub streak: i32,
    pub peak_ladder: Option<i32>,
    /// The best (lowest) Jlorious position held this season, or null.
    pub peak_jlorious: Option<i32>,
    pub updated_at: i64,
}

/// A placed or placing player's standing, as the season's percentiles and Jlorious read it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Standing {
    pub profile_id: String,
    pub ladder: Option<i32>,
    pub rating: f64,
}

/// A season row before its first game.
pub fn fresh_rank(season_id: &str, profile_id: &str, at: i64) -> SeasonRank {
    SeasonRank {
        season_id: season_id.to_string(),
        profile_id: profile_id.to_string(),
        games: 0,
        wins: 0,
        losses: 0,
        draws: 0,
        ladder: None,
        floor: 0,
        streak: 0,
        peak_ladder: None,
        peak_jlorious: None,
        updated_at: at,
    }
}

/// The Grape tier (an index into `GRAPE_TIERS`) a ladder position is in.
pub fn tier_index_of(ladder: i32) -> i32 {
    ladder.div_euclid(PIPS_PER_TIER).max(0).min(GRAPE_TIERS.len() as i32 - 1)
}

/// The lowest ladder position of a Grape tier: its Division III with no pip.
pub fn tier_bottom(tier_index: i32) -> i32 {
    tier_index * PIPS_PER_TIER
}

/// `GRAPE_TIERS[index] ?? fallback`.
fn grape_at(index: i32, fallback: GrapeTier) -> GrapeTier {
    usize::try_from(index).ok().and_then(|at| GRAPE_TIERS.get(at)).copied().unwrap_or(fallback)
}

/// A position inside a Grape tier, as the client draws it: division III to I, and pips.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LadderPlace {
    pub tier: GrapeTier,
    pub division: i32,
    pub pips: i32,
}

/// R605: where a ladder position sits. Division 3 is the bottom of a tier (III), 1 the top (I).
pub fn place_of(ladder: i32) -> LadderPlace {
    let tier_index = tier_index_of(ladder);
    let within = ladder - tier_bottom(tier_index);
    let from_bottom = within.div_euclid(RANK_PIPS_PER_DIVISION);
    LadderPlace {
        tier: grape_at(tier_index, GrapeTier::Rotten),
        division: RANK_DIVISIONS_PER_TIER - from_bottom,
        pips: within - from_bottom * RANK_PIPS_PER_DIVISION,
    }
}

/// A rating's percentile among the others, as an exact fraction (R606). It is the mid-rank: the
/// others below, half of those level with it, and half of the player's own place, over everyone
/// counted, `(2·below + level + 1) / (2·(others + 1))`. Kept as two integers so a tier boundary is
/// compared exactly. With nobody else placed the percentile is one half.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Percentile {
    pub numerator: i64,
    pub denominator: i64,
}

pub fn percentile_of(rating: f64, others: &[f64]) -> Percentile {
    let mut below: i64 = 0;
    let mut level: i64 = 0;
    for &other in others {
        if other < rating {
            below += 1;
        } else if other == rating {
            level += 1;
        }
    }
    Percentile { numerator: 2 * below + level + 1, denominator: 2 * (others.len() as i64 + 1) }
}

/// R606: the ladder position a percentile calls for. The tiers take `RANK_TIER_PERCENTS` of the
/// range in order, a percentile exactly on a boundary belonging to the tier above it, and inside a
/// tier the percentile is spread evenly over its pips.
pub fn target_ladder(percentile: &Percentile) -> i32 {
    let scaled = percentile.numerator * i64::from(PERCENT);
    let mut below: i64 = 0;
    for (tier_index, &tier) in GRAPE_TIERS.iter().enumerate() {
        let share = i64::from(percent_of(tier));
        let last = tier_index == GRAPE_TIERS.len() - 1;
        if last || scaled < (below + share) * percentile.denominator {
            // How far into the tier, in pips: (p − below) / share of the tier's pips, floored, in integers.
            let into = ((scaled - below * percentile.denominator) * i64::from(PIPS_PER_TIER))
                .div_euclid(share * percentile.denominator);
            let clamped = into.max(0).min(i64::from(PIPS_PER_TIER - 1)) as i32;
            return tier_bottom(tier_index as i32) + clamped;
        }
        below += share;
    }
    LADDER_TOP
}

/// `pip_delta`'s input (TS's anonymous `{ result, ladder, target, streak }`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PipDeltaInput {
    pub result: GameResult,
    pub ladder: i32,
    pub target: i32,
    pub streak: i32,
}

/// R606: the pips one game moves a placed player, before the floor and the top clamp it. A win gives
/// `RANK_WIN_PIPS`, plus the streak bonus below Mythic Grape, plus the lean when the rating calls for
/// a rank a division or more above; a loss takes `RANK_LOSS_PIPS`, plus the lean when the rating calls
/// for a rank a division or more below; a draw moves nothing. `ladder` is the position before the
/// game and `streak` the win streak counting this game.
pub fn pip_delta(input: &PipDeltaInput) -> i32 {
    let gap = input.target - input.ladder;
    match input.result {
        GameResult::Draw => 0,
        GameResult::Win => {
            let lean = if gap >= RANK_CONVERGENCE_GAP_PIPS { RANK_CONVERGENCE_PIPS } else { 0 };
            let bonus = if input.streak >= RANK_STREAK_LENGTH && tier_index_of(input.ladder) < MYTHIC {
                RANK_STREAK_BONUS_PIPS
            } else {
                0
            };
            RANK_WIN_PIPS + lean + bonus
        }
        GameResult::Loss => {
            let lean = if gap <= -RANK_CONVERGENCE_GAP_PIPS { RANK_CONVERGENCE_PIPS } else { 0 };
            -(RANK_LOSS_PIPS + lean)
        }
    }
}

/// `apply_ranked_game`'s input (TS's anonymous `{ result, target, at }`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApplyRankedGameInput {
    pub result: GameResult,
    pub target: i32,
    pub at: i64,
}

/// R605–R607: one rated game on a player's season. `target` is the rank the player's rating calls for
/// once this game has moved it. A player still placing plays toward their placements, and the game
/// that completes them puts them straight at the target; a placed player moves by `pip_delta`, never
/// below the floor of the highest Grape tier they have reached this season, and never past the top.
pub fn apply_ranked_game(rank: &SeasonRank, input: &ApplyRankedGameInput) -> SeasonRank {
    let games = rank.games + 1;
    let streak = match input.result {
        GameResult::Win => rank.streak + 1,
        GameResult::Loss => 0,
        GameResult::Draw => rank.streak,
    };
    let tally = SeasonRank {
        games,
        wins: rank.wins + i32::from(input.result == GameResult::Win),
        losses: rank.losses + i32::from(input.result == GameResult::Loss),
        draws: rank.draws + i32::from(input.result == GameResult::Draw),
        streak,
        updated_at: input.at,
        ..rank.clone()
    };

    let Some(current) = rank.ladder else {
        if games < RANK_PLACEMENT_GAMES {
            return tally;
        }
        let placed = input.target.max(0).min(LADDER_TOP);
        return SeasonRank {
            ladder: Some(placed),
            floor: rank.floor.max(tier_index_of(placed)),
            peak_ladder: Some(rank.peak_ladder.unwrap_or(placed).max(placed)),
            ..tally
        };
    };

    let moved =
        current + pip_delta(&PipDeltaInput { result: input.result, ladder: current, target: input.target, streak });
    let ladder = LADDER_TOP.min(tier_bottom(rank.floor).max(moved));
    SeasonRank {
        ladder: Some(ladder),
        floor: rank.floor.max(tier_index_of(ladder)),
        peak_ladder: Some(rank.peak_ladder.unwrap_or(ladder).max(ladder)),
        ..tally
    }
}

/// R608: Jlorious, in order: the season's placed Mythic Grape players by hidden rating, highest
/// first, ties broken by profile id, the first `JLORIOUS_SIZE` of them. When fewer qualify, every one
/// of them is Jlorious. A player who falls out is in Mythic Grape again, where the floor holds them.
pub fn jlorious_order(standings: &[Standing]) -> Vec<String> {
    let mut mythic: Vec<&Standing> = standings
        .iter()
        .filter(|standing| standing.ladder.is_some_and(|ladder| tier_index_of(ladder) == MYTHIC))
        .collect();
    // TS `b.rating - a.rating || (a.profileId < b.profileId ? -1 : …)`: a difference of 0 (or NaN)
    // falls through to the id. Stable, as `Array.prototype.sort` is.
    mythic.sort_by(|a, b| {
        b.rating
            .partial_cmp(&a.rating)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.profile_id.cmp(&b.profile_id))
    });
    mythic.into_iter().take(JLORIOUS_SIZE).map(|standing| standing.profile_id.clone()).collect()
}

/// R608: the rank row with a Jlorious position it has just held, kept if it is its best.
pub fn with_jlorious_peak(rank: &SeasonRank, position: i32) -> SeasonRank {
    if rank.peak_jlorious.is_some_and(|peak| peak <= position) {
        return rank.clone();
    }
    SeasonRank { peak_jlorious: Some(position), ..rank.clone() }
}

/// What a player is shown as (R612): Raisin with their placements, a Grape tier with its division,
/// pips and the floor that holds them, or a Jlorious position. Never the rating.
///
/// TS's union is discriminated on `tier`, whose Grape arm takes any of five values, so serde reads
/// and writes it through [`VisibleRankWire`]: `{ "tier": "raisin", "placementsPlayed", "placementGames" }`,
/// `{ "tier": <grape>, "division", "pips", "pipsPerDivision", "floor" }` or
/// `{ "tier": "jlorious", "position" }`, keys in TS's order.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(into = "VisibleRankWire", try_from = "VisibleRankWire")]
pub enum VisibleRank {
    Raisin { placements_played: i32, placement_games: i32 },
    Grape { tier: GrapeTier, division: i32, pips: i32, pips_per_division: i32, floor: GrapeTier },
    Jlorious { position: i32 },
}

/// The JSON shape of [`VisibleRank`] and [`PeakBadge`]: every arm's fields, each present only on its
/// own arm.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VisibleRankWire {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub season_id: Option<String>,
    pub tier: RankTier,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placements_played: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement_games: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub division: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pips: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pips_per_division: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floor: Option<GrapeTier>,
}

impl VisibleRankWire {
    fn tier_only(season_id: Option<String>, tier: RankTier) -> VisibleRankWire {
        VisibleRankWire {
            season_id,
            tier,
            placements_played: None,
            placement_games: None,
            position: None,
            division: None,
            pips: None,
            pips_per_division: None,
            floor: None,
        }
    }
}

impl From<VisibleRank> for VisibleRankWire {
    fn from(rank: VisibleRank) -> VisibleRankWire {
        match rank {
            VisibleRank::Raisin { placements_played, placement_games } => VisibleRankWire {
                placements_played: Some(placements_played),
                placement_games: Some(placement_games),
                ..VisibleRankWire::tier_only(None, RankTier::Raisin)
            },
            VisibleRank::Grape { tier, division, pips, pips_per_division, floor } => VisibleRankWire {
                division: Some(division),
                pips: Some(pips),
                pips_per_division: Some(pips_per_division),
                floor: Some(floor),
                ..VisibleRankWire::tier_only(None, tier.into())
            },
            VisibleRank::Jlorious { position } => VisibleRankWire {
                position: Some(position),
                ..VisibleRankWire::tier_only(None, RankTier::Jlorious)
            },
        }
    }
}

impl TryFrom<VisibleRankWire> for VisibleRank {
    type Error = String;

    fn try_from(wire: VisibleRankWire) -> Result<VisibleRank, String> {
        let missing = |field: &str| format!("a {:?} rank needs `{field}`", wire.tier);
        match wire.tier {
            RankTier::Raisin => Ok(VisibleRank::Raisin {
                placements_played: wire.placements_played.ok_or_else(|| missing("placementsPlayed"))?,
                placement_games: wire.placement_games.ok_or_else(|| missing("placementGames"))?,
            }),
            RankTier::Jlorious => {
                Ok(VisibleRank::Jlorious { position: wire.position.ok_or_else(|| missing("position"))? })
            }
            grape => Ok(VisibleRank::Grape {
                tier: grape.grape().ok_or_else(|| missing("tier"))?,
                division: wire.division.ok_or_else(|| missing("division"))?,
                pips: wire.pips.ok_or_else(|| missing("pips"))?,
                pips_per_division: wire.pips_per_division.ok_or_else(|| missing("pipsPerDivision"))?,
                floor: wire.floor.ok_or_else(|| missing("floor"))?,
            }),
        }
    }
}

/// A player's rank, given their season row (`None` before their first game) and Jlorious position.
pub fn visible_rank(rank: Option<&SeasonRank>, jlorious_position: Option<i32>) -> VisibleRank {
    let Some(placed) = rank.filter(|row| row.ladder.is_some()) else {
        return VisibleRank::Raisin {
            placements_played: rank.map_or(0, |row| row.games).min(RANK_PLACEMENT_GAMES),
            placement_games: RANK_PLACEMENT_GAMES,
        };
    };
    if let Some(position) = jlorious_position {
        return VisibleRank::Jlorious { position };
    }
    let place = place_of(placed.ladder.unwrap_or(0));
    VisibleRank::Grape {
        tier: place.tier,
        division: place.division,
        pips: place.pips,
        pips_per_division: RANK_PIPS_PER_DIVISION,
        floor: grape_at(placed.floor, GrapeTier::Rotten),
    }
}

/// A season's best, as the profile's badge shows it (R607): a Jlorious position or a Grape division.
/// Serialised as `{ "seasonId", "tier": "jlorious", "position" }` or
/// `{ "seasonId", "tier": <grape>, "division" }`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(into = "VisibleRankWire", try_from = "VisibleRankWire")]
pub enum PeakBadge {
    Jlorious { season_id: String, position: i32 },
    Grape { season_id: String, tier: GrapeTier, division: i32 },
}

impl From<PeakBadge> for VisibleRankWire {
    fn from(badge: PeakBadge) -> VisibleRankWire {
        match badge {
            PeakBadge::Jlorious { season_id, position } => VisibleRankWire {
                position: Some(position),
                ..VisibleRankWire::tier_only(Some(season_id), RankTier::Jlorious)
            },
            PeakBadge::Grape { season_id, tier, division } => VisibleRankWire {
                division: Some(division),
                ..VisibleRankWire::tier_only(Some(season_id), tier.into())
            },
        }
    }
}

impl TryFrom<VisibleRankWire> for PeakBadge {
    type Error = String;

    fn try_from(wire: VisibleRankWire) -> Result<PeakBadge, String> {
        let missing = |field: &str| format!("a {:?} badge needs `{field}`", wire.tier);
        let season_id = wire.season_id.clone().ok_or_else(|| missing("seasonId"))?;
        match wire.tier {
            RankTier::Jlorious => {
                Ok(PeakBadge::Jlorious { season_id, position: wire.position.ok_or_else(|| missing("position"))? })
            }
            other => Ok(PeakBadge::Grape {
                season_id,
                tier: other.grape().ok_or_else(|| missing("tier"))?,
                division: wire.division.ok_or_else(|| missing("division"))?,
            }),
        }
    }
}

/// The season's badge, or `None` for a season whose placements were never finished.
pub fn peak_badge(rank: &SeasonRank) -> Option<PeakBadge> {
    if let Some(position) = rank.peak_jlorious {
        return Some(PeakBadge::Jlorious { season_id: rank.season_id.clone(), position });
    }
    let peak = rank.peak_ladder?;
    let place = place_of(peak);
    Some(PeakBadge::Grape { season_id: rank.season_id.clone(), tier: place.tier, division: place.division })
}
