//! The ranked ladder on the server (SPEC §9.12, R603–R612): opening a season, rating a ranked game,
//! and the three reads the client has — your own rank, the leaderboard, and both players' ranks on
//! the match screen. Port of `apps/server/src/api/ranked.ts`.
//!
//! The rules are pure and live in `crate::ranked`: Glicko-2 (`glicko2.rs`), the ladder
//! (`ladder.rs`) and the season's soft reset (`season.rs`). This file only reads the store, hands
//! what it read to them, and writes back what they decided, always inside the caller's transaction.
//!
//! Who calls it:
//!  - `results.rs`, for a ranked match that is not a series game, in the transaction that writes the
//!    result (R604);
//!  - `series.rs`, for a ranked series when it ends, in the transaction of its last transition (R262);
//!  - `app.rs` at boot (`open_season`, SURFACE §11.2), and `cli/season_start.rs` by hand, to open
//!    the build's season (R609).
//!
//! Nothing here ever sends a rating to a client (R612): the reads answer with `VisibleRank`s, tags,
//! and the season's badges.

use indexmap::{IndexMap, IndexSet};
use serde::Serialize;
use serde_json::{Value, json};

use jackioh_engine::PerPlayer;

use crate::api::crypto::player_tag;
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, bad_request, log_alert, log_info, now_ms, ok_of};
use crate::app::App;
use crate::db::store::{
    BotRating, Pilot, Profile, RatedGameKind, RatedGameRow, RatedReason, RatedSide, Season, SeasonStanding,
    StoreError, Tx,
};
use crate::ranked::glicko2::{Glicko, START_GLICKO, rate_game};
use crate::ranked::ladder::{
    self, ApplyRankedGameInput, GRAPE_TIERS, GrapeTier, PeakBadge, SeasonRank, Standing, VisibleRank,
    apply_ranked_game, fresh_rank, jlorious_order, peak_badge, percentile_of, place_of, target_ladder,
    tier_index_of, visible_rank, with_jlorious_peak,
};
use crate::ranked::season::{ResetReport, season_id_of, soft_reset};

// ---------------------------------------------------------------------------
// The game's version (R375) and its season (R609)
// ---------------------------------------------------------------------------

/// The newest patch's version: the game's version, which names the season (R609). Compiled in
/// from `crates/cards/patches/patches.json` (SURFACE §11.3), the same version the game records
/// file under.
pub fn load_patch_version() -> String {
    jackioh_cards::catalog_version().to_string()
}

/// What opening a season reads: the version that names it (TS also took the clock and the log,
/// which are `now_ms` and `tracing` here).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeasonDeps {
    pub patch_version: String,
}

impl SeasonDeps {
    /// This build's: TS's `deps.patchVersion`, read at boot by `src/index.ts`.
    pub fn current() -> SeasonDeps {
        SeasonDeps { patch_version: load_patch_version() }
    }
}

/// R609: the id of the season this build rates games in.
pub fn build_season_id(deps: &SeasonDeps) -> String {
    season_id_of(&deps.patch_version)
}

/// What opening a season did: the season, and the soft reset's report when this call opened it.
#[derive(Clone, Debug)]
pub struct OpenedSeason {
    pub season: Season,
    pub opened: bool,
    pub reset: Option<ResetReport>,
}

/// R609: the build's season, opened inside `t` when it does not exist yet. The first season of all
/// resets nothing — there is no season before it to come back from — and every later one runs the
/// soft reset over every rated player in the same transaction as the season row, so a season is
/// either open and reset or neither. Opens serialize on `ranked_lock_seasons`: two processes
/// opening at once — the same season or, racing deploys, two different ones — run one after the
/// other, so the second sees the first's row (or resets over a world that already holds it) and
/// never soft-resets off a snapshot that predates it.
pub async fn open_season_in_tx(t: &mut Tx<'_>, deps: &SeasonDeps) -> Result<OpenedSeason, StoreError> {
    let id = build_season_id(deps);
    t.ranked_lock_seasons().await?;
    let seasons = t.ranked_seasons().await?;
    if let Some(existing) = seasons.iter().find(|season| season.id == id) {
        return Ok(OpenedSeason { season: existing.clone(), opened: false, reset: None });
    }

    let season = Season { id: id.clone(), patch_version: deps.patch_version.clone(), started_at: now_ms() };
    if !t.ranked_create_season(&season).await? {
        let raced = t.ranked_seasons().await?.into_iter().find(|candidate| candidate.id == id);
        let Some(raced) = raced else {
            return Err(StoreError::Other(format!("season {id} was neither created nor found")));
        };
        return Ok(OpenedSeason { season: raced, opened: false, reset: None });
    }
    if seasons.is_empty() {
        log_info("season.opened", json!({ "seasonId": id, "patchVersion": deps.patch_version, "reset": null }));
        return Ok(OpenedSeason { season, opened: true, reset: None });
    }
    let players = t.ranked_rated_players().await?;
    let reset = soft_reset(&players);
    t.ranked_reset_ratings(&reset.changes).await?;
    log_info(
        "season.opened",
        json!({
            "seasonId": id,
            "patchVersion": deps.patch_version,
            "reset": serde_json::to_value(&reset.report).unwrap_or(Value::Null),
        }),
    );
    Ok(OpenedSeason { season, opened: true, reset: Some(reset.report) })
}

/// R609: opens the build's season in a transaction of its own (boot, SURFACE §11.2: `app.rs` calls
/// it once before the port opens).
pub async fn open_season(app: &App) -> Result<OpenedSeason, StoreError> {
    let deps = SeasonDeps::current();
    let mut tx = app.db.begin(None).await?;
    let opened = open_season_in_tx(&mut tx, &deps).await?;
    tx.commit().await?;
    Ok(opened)
}

// ---------------------------------------------------------------------------
// Rating one ranked game (R603–R608, R610, R611)
// ---------------------------------------------------------------------------

/// One side of a ranked game: a player's profile, or one of the AI bots (R610).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RankedSideInput {
    Player { profile_id: String },
    Bot { bot_id: String },
}

#[derive(Clone, Debug)]
pub struct RankedGameInput {
    /// The match's id, or the series' id: the rated game's id, and what makes rating it idempotent.
    pub id: String,
    pub kind: RatedGameKind,
    pub catalog_version: String,
    pub sides: (RankedSideInput, RankedSideInput),
    /// The index of the side that won, or none for a draw.
    pub winner_side: Option<usize>,
    pub reason: RatedReason,
    pub at: i64,
    /// R672: a double-or-nothing rematch's stakes. Only 2 changes anything: each side's rating moves
    /// twice the single update's delta, with deviation and volatility from that single update.
    /// Absent (a normal game, a series) rates once.
    pub stake: Option<i32>,
}

/// One side's rating write.
#[derive(Clone, Debug)]
pub struct GlickoWrite {
    pub side: RankedSideInput,
    pub glicko: Glicko,
    pub games: i64,
}

/// One bystander's Jlorious peak.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeakWrite {
    pub profile_id: String,
    pub position: i32,
}

#[derive(Clone, Debug)]
pub struct RankedWrites {
    pub glickos: Vec<GlickoWrite>,
    pub ranks: Vec<SeasonRank>,
    pub peaks: Vec<PeakWrite>,
}

/// Everything one ranked game will write, decided before any of it is written. Kept apart from the
/// write because a series has to put the ratings on its own row first, by compare-and-set, and must
/// write nothing if it loses (`series.rs`).
#[derive(Clone, Debug)]
pub struct RankedPlan {
    pub row: RatedGameRow,
    /// None when the game was rated before: nothing is written again.
    pub writes: Option<RankedWrites>,
}

/// Glicko-2's score for `side`: a win, a draw or a loss.
fn score_of(winner_side: Option<usize>, side: usize) -> f64 {
    match winner_side {
        None => 0.5,
        Some(winner) if winner == side => 1.0,
        Some(_) => 0.0,
    }
}

fn result_of(winner_side: Option<usize>, side: usize) -> ladder::GameResult {
    match winner_side {
        None => ladder::GameResult::Draw,
        Some(winner) if winner == side => ladder::GameResult::Win,
        Some(_) => ladder::GameResult::Loss,
    }
}

/// A standing without its rating: the season row itself.
fn row_of(standing: &SeasonStanding) -> SeasonRank {
    standing.rank.clone()
}

/// The season as the ladder's percentiles and Jlorious read it.
fn standings_of(standings: &[SeasonStanding]) -> Vec<Standing> {
    standings
        .iter()
        .map(|standing| Standing {
            profile_id: standing.rank.profile_id.clone(),
            ladder: standing.rank.ladder,
            rating: standing.rating,
        })
        .collect()
}

/// A 1-based position in Jlorious, or none.
fn position_in(order: &[String], profile_id: &str) -> Option<i32> {
    order.iter().position(|candidate| candidate == profile_id).map(|at| at as i32 + 1)
}

/// A side's rating and rated-game count going into the game.
#[derive(Clone, Debug)]
struct Before {
    glicko: Glicko,
    games: i64,
}

async fn before_of(
    t: &mut Tx<'_>,
    profiles: &[Profile],
    side: &RankedSideInput,
    game_id: &str,
) -> Result<Before, StoreError> {
    match side {
        RankedSideInput::Bot { bot_id } => {
            let bot = t.ranked_bot(bot_id).await?;
            Ok(match bot {
                Some(bot) => Before { glicko: bot.glicko.clone(), games: bot.games as i64 },
                None => Before { glicko: START_GLICKO, games: 0 },
            })
        }
        RankedSideInput::Player { profile_id } => match profiles.iter().find(|candidate| candidate.id == *profile_id) {
            None => {
                // A ranked game cannot outlive its players (the foreign keys say so): rate it from the
                // start and shout, as `results.rs` always has.
                log_alert("ranked.profile_missing", json!({ "gameId": game_id, "profileId": profile_id }));
                Ok(Before { glicko: START_GLICKO, games: 0 })
            }
            Some(profile) => Ok(Before {
                glicko: Glicko {
                    rating: profile.rating,
                    deviation: profile.rating_deviation,
                    volatility: profile.rating_volatility,
                },
                games: 0,
            }),
        },
    }
}

/// A season row's move: as it stood, and after this game.
#[derive(Clone, Debug)]
struct RankChange {
    before: Option<SeasonRank>,
    after: SeasonRank,
}

/// Plans one ranked game (R603–R608): both sides rated against each other's rating from before it,
/// each player's season moved toward the rank their new rating calls for, and the Jlorious peaks it
/// gave anyone. Both targets are read from the season as it stood before the game, with both new
/// ratings in it, so neither side's placement depends on which of the two is computed first.
pub async fn plan_ranked_game(t: &mut Tx<'_>, _app: &App, input: &RankedGameInput) -> Result<RankedPlan, StoreError> {
    if let Some(recorded) = t.ranked_game(&input.id).await? {
        return Ok(RankedPlan { row: recorded, writes: None });
    }

    let (first, second) = (&input.sides.0, &input.sides.1);
    if let (RankedSideInput::Player { profile_id: a }, RankedSideInput::Player { profile_id: b }) = (first, second) {
        if a == b {
            return Err(StoreError::Other(format!("rated game {} names one profile on both sides", input.id)));
        }
    }
    let sides = [first, second];

    let deps = SeasonDeps::current();
    let OpenedSeason { season, .. } = open_season_in_tx(t, &deps).await?;
    let standings = t.ranked_standings(&season.id).await?;
    let player_ids: Vec<String> = sides
        .iter()
        .filter_map(|side| match side {
            RankedSideInput::Player { profile_id } => Some(profile_id.clone()),
            RankedSideInput::Bot { .. } => None,
        })
        .collect();
    let profiles = t.profiles_get_many(&player_ids).await?;

    let before = [
        before_of(t, &profiles, first, &input.id).await?,
        before_of(t, &profiles, second, &input.id).await?,
    ];
    let rated = rate_game(&before[0].glicko, &before[1].glicko, score_of(input.winner_side, 0));
    // R672: a double-or-nothing rematch doubles each side's rating movement around its own before:
    // one update, then the delta twice. Deviation and volatility are the single update's — a doubled
    // game moves the rating twice without sharpening the confidence twice.
    let after: [Glicko; 2] = if input.stake == Some(2) {
        [
            Glicko {
                rating: before[0].glicko.rating + 2.0 * (rated.a.rating - before[0].glicko.rating),
                ..rated.a.clone()
            },
            Glicko {
                rating: before[1].glicko.rating + 2.0 * (rated.b.rating - before[1].glicko.rating),
                ..rated.b.clone()
            },
        ]
    } else {
        [rated.a.clone(), rated.b.clone()]
    };

    // The season with both new ratings in it, and each player's row as it stood.
    let mut new_rating: IndexMap<String, f64> = IndexMap::new();
    for (index, side) in sides.iter().enumerate() {
        if let RankedSideInput::Player { profile_id } = side {
            new_rating.insert(profile_id.clone(), after[index].rating);
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
    let jlorious_before = jlorious_order(&standings_of(&standings));

    let mut ranks: IndexMap<String, RankChange> = IndexMap::new();
    for (index, side) in sides.iter().enumerate() {
        let RankedSideInput::Player { profile_id } = side else {
            continue;
        };
        let standing = standings.iter().find(|candidate| candidate.rank.profile_id == *profile_id);
        let row_before = standing.map(row_of);
        let others: Vec<f64> = re_rated
            .iter()
            .filter(|candidate| candidate.rank.profile_id != *profile_id && candidate.rank.ladder.is_some())
            .map(|candidate| candidate.rating)
            .collect();
        let target = target_ladder(&percentile_of(after[index].rating, &others));
        let start = row_before.clone().unwrap_or_else(|| fresh_rank(&season.id, profile_id, input.at));
        let row_after = apply_ranked_game(
            &start,
            &ApplyRankedGameInput { result: result_of(input.winner_side, index), target, at: input.at },
        );
        ranks.insert(profile_id.clone(), RankChange { before: row_before, after: row_after });
    }

    // Jlorious after the game, and the peak it gave each member who now stands higher than ever.
    let mut standings_after: Vec<SeasonStanding> =
        re_rated.iter().filter(|standing| !ranks.contains_key(&standing.rank.profile_id)).cloned().collect();
    for (profile_id, change) in &ranks {
        standings_after.push(SeasonStanding {
            rank: change.after.clone(),
            rating: new_rating.get(profile_id).copied().unwrap_or(0.0),
        });
    }
    let jlorious_after = jlorious_order(&standings_of(&standings_after));
    let mut peaks: Vec<PeakWrite> = Vec::new();
    for (index, profile_id) in jlorious_after.iter().enumerate() {
        let position = index as i32 + 1;
        if let Some(own) = ranks.get_mut(profile_id) {
            own.after = with_jlorious_peak(&own.after, position);
            continue;
        }
        let standing = standings_after.iter().find(|candidate| candidate.rank.profile_id == *profile_id);
        if let Some(standing) = standing {
            if standing.rank.peak_jlorious.is_none_or(|peak| position < peak) {
                peaks.push(PeakWrite { profile_id: profile_id.clone(), position });
            }
        }
    }

    let side_of = |side: &RankedSideInput, index: usize| -> RatedSide {
        match side {
            RankedSideInput::Bot { bot_id } => RatedSide {
                profile_id: None,
                bot_id: Some(bot_id.clone()),
                pilot: Pilot::Ai,
                before: before[index].glicko.clone(),
                after: after[index].clone(),
                rank_before: None,
                rank_after: None,
            },
            RankedSideInput::Player { profile_id } => {
                let rank = ranks.get(profile_id);
                RatedSide {
                    profile_id: Some(profile_id.clone()),
                    bot_id: None,
                    pilot: Pilot::Human,
                    before: before[index].glicko.clone(),
                    after: after[index].clone(),
                    rank_before: Some(visible_rank(
                        rank.and_then(|rank| rank.before.as_ref()),
                        position_in(&jlorious_before, profile_id),
                    )),
                    rank_after: Some(visible_rank(
                        rank.map(|rank| &rank.after),
                        position_in(&jlorious_after, profile_id),
                    )),
                }
            }
        }
    };

    let row = RatedGameRow {
        id: input.id.clone(),
        kind: input.kind.clone(),
        season_id: season.id.clone(),
        patch_version: deps.patch_version.clone(),
        catalog_version: input.catalog_version.clone(),
        sides: (side_of(first, 0), side_of(second, 1)),
        winner_side: input.winner_side,
        reason: input.reason.clone(),
        ended_at: input.at,
    };
    let glickos = sides
        .iter()
        .enumerate()
        .map(|(index, side)| GlickoWrite {
            side: (*side).clone(),
            glicko: after[index].clone(),
            games: before[index].games + 1,
        })
        .collect();
    Ok(RankedPlan {
        row,
        writes: Some(RankedWrites { glickos, ranks: ranks.values().map(|rank| rank.after.clone()).collect(), peaks }),
    })
}

/// Writes a plan inside `t`: both ratings, both season rows, the peaks, and the record (R611).
pub async fn commit_ranked_game(t: &mut Tx<'_>, plan: &RankedPlan, at: i64) -> Result<(), StoreError> {
    let Some(writes) = &plan.writes else {
        return Ok(());
    };
    for write in &writes.glickos {
        match &write.side {
            RankedSideInput::Player { profile_id } => t.profiles_set_glicko(profile_id, &write.glicko).await?,
            RankedSideInput::Bot { bot_id } => {
                t.ranked_put_bot(&BotRating {
                    bot_id: bot_id.clone(),
                    glicko: write.glicko.clone(),
                    games: write.games as _,
                    updated_at: at,
                })
                .await?
            }
        }
    }
    for rank in &writes.ranks {
        t.ranked_put_rank(rank).await?;
    }
    for peak in &writes.peaks {
        t.ranked_note_peak_jlorious(&plan.row.season_id, &peak.profile_id, i64::from(peak.position)).await?;
    }
    t.ranked_record_game(&plan.row).await?;
    Ok(())
}

/// Plans and writes one ranked game inside `t`. Rating the same id again changes nothing.
pub async fn rate_ranked_game(t: &mut Tx<'_>, app: &App, input: &RankedGameInput) -> Result<RatedGameRow, StoreError> {
    let plan = plan_ranked_game(t, app, input).await?;
    commit_ranked_game(t, &plan, input.at).await?;
    Ok(plan.row)
}

// ---------------------------------------------------------------------------
// What the client reads (R612)
// ---------------------------------------------------------------------------

/// One profile's rank in a season it has standings for.
fn rank_in(standings: &[SeasonStanding], jlorious: &[String], profile_id: &str) -> VisibleRank {
    let standing = standings.iter().find(|candidate| candidate.rank.profile_id == profile_id);
    visible_rank(standing.map(|standing| &standing.rank), position_in(jlorious, profile_id))
}

/// The caller's record this season.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RankRecord {
    pub games: i64,
    pub wins: i64,
    pub losses: i64,
    pub draws: i64,
}

/// `GET /api/ranked`: the caller's own season, tag and badges.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OwnRankBody {
    pub season: String,
    pub tag: String,
    pub rank: VisibleRank,
    /// The current win streak, which earns bonus pips below Mythic Grape (R606).
    pub streak: i64,
    pub record: RankRecord,
    /// Each season's best, newest season first; a season whose placements were never finished has none.
    pub badges: Vec<PeakBadge>,
}

pub async fn own_rank(app: &App, profile_id: &str) -> Result<OwnRankBody, ApiError> {
    let season_id = build_season_id(&SeasonDeps::current());
    let mut tx = app.db.begin(Some(profile_id)).await?;
    let standings = tx.ranked_standings(&season_id).await?;
    let history = tx.ranked_ranks_of(profile_id).await?;
    tx.commit().await?;
    let jlorious = jlorious_order(&standings_of(&standings));
    let mine = standings.iter().find(|standing| standing.rank.profile_id == profile_id).map(|standing| &standing.rank);
    Ok(OwnRankBody {
        season: season_id,
        tag: player_tag(profile_id),
        rank: rank_in(&standings, &jlorious, profile_id),
        streak: mine.map(|rank| rank.streak as i64).unwrap_or(0),
        record: RankRecord {
            games: mine.map(|rank| rank.games as i64).unwrap_or(0),
            wins: mine.map(|rank| rank.wins as i64).unwrap_or(0),
            losses: mine.map(|rank| rank.losses as i64).unwrap_or(0),
            draws: mine.map(|rank| rank.draws as i64).unwrap_or(0),
        },
        badges: history.iter().filter_map(|rank| peak_badge(rank)).rev().collect(),
    })
}

/// One row of a Grape tier on the leaderboard.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardRow {
    pub tag: String,
    pub division: i64,
    pub pips: i64,
    pub you: bool,
}

/// One Jlorious place on the leaderboard, #1 first.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JloriousRow {
    pub position: i32,
    pub tag: String,
    pub you: bool,
}

/// One Grape tier: `count` is the tier's size; `players` is all of it (R612).
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardTier {
    pub tier: GrapeTier,
    pub count: usize,
    pub players: Vec<LeaderboardRow>,
}

/// `GET /api/leaderboard` (R612).
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardBody {
    pub season: String,
    /// Jlorious, #1 first.
    pub jlorious: Vec<JloriousRow>,
    /// Every other placed player, grouped by Grape tier, highest tier first and, inside a tier,
    /// highest rank first.
    pub tiers: Vec<LeaderboardTier>,
    /// Players still playing their placements.
    pub raisins: i64,
    pub you: VisibleRank,
}

/// A placed player as a tier's list holds them before sorting.
#[derive(Clone, Debug)]
struct TierEntry {
    tag: String,
    ladder: i32,
    you: bool,
}

pub async fn leaderboard(app: &App, viewer_id: &str) -> Result<LeaderboardBody, ApiError> {
    let season_id = build_season_id(&SeasonDeps::current());
    let mut tx = app.db.begin(Some(viewer_id)).await?;
    let standings = tx.ranked_standings(&season_id).await?;
    tx.commit().await?;
    let jlorious = jlorious_order(&standings_of(&standings));
    let in_jlorious: IndexSet<&str> = jlorious.iter().map(String::as_str).collect();

    let tier_count = GRAPE_TIERS.len();
    let mut groups: Vec<Vec<TierEntry>> = vec![Vec::new(); tier_count];
    let mut raisins = 0;
    for standing in &standings {
        let Some(ladder) = standing.rank.ladder else {
            raisins += 1;
            continue;
        };
        if in_jlorious.contains(standing.rank.profile_id.as_str()) {
            continue;
        }
        let index = (tier_index_of(ladder) as usize).min(tier_count.saturating_sub(1));
        groups[index].push(TierEntry {
            tag: player_tag(&standing.rank.profile_id),
            ladder,
            you: standing.rank.profile_id == viewer_id,
        });
    }

    let tiers = GRAPE_TIERS
        .iter()
        .enumerate()
        .rev()
        .map(|(index, tier)| {
            let mut rows = groups[index].clone();
            rows.sort_by(|a, b| b.ladder.cmp(&a.ladder).then_with(|| a.tag.cmp(&b.tag)));
            LeaderboardTier {
                tier: *tier,
                count: rows.len(),
                players: rows
                    .iter()
                    .map(|row| {
                        let place = place_of(row.ladder);
                        LeaderboardRow {
                            tag: row.tag.clone(),
                            division: place.division as i64,
                            pips: place.pips as i64,
                            you: row.you,
                        }
                    })
                    .collect(),
            }
        })
        .collect();

    Ok(LeaderboardBody {
        season: season_id,
        jlorious: jlorious
            .iter()
            .enumerate()
            .map(|(index, profile_id)| JloriousRow {
                position: index as i32 + 1,
                tag: player_tag(profile_id),
                you: profile_id == viewer_id,
            })
            .collect(),
        tiers,
        raisins,
        you: rank_in(&standings, &jlorious, viewer_id),
    })
}

/// One seat on the match screen.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SeatRank {
    pub tag: String,
    pub rank: VisibleRank,
    pub you: bool,
}

/// `GET /api/matches/:matchId/ranks`: both seats' ranks for the match screen (R612).
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MatchRanksBody {
    /// R604: whether the match is ranked. A ranked match's end moves the rating and the ladder
    /// itself; a ranked series' games carry the flag while the series moves the rating once,
    /// when it ends.
    pub ranked: bool,
    pub seats: PerPlayer<SeatRank>,
}

pub async fn match_ranks(app: &App, match_id: &str, viewer_id: &str) -> Result<Option<MatchRanksBody>, ApiError> {
    let mut tx = app.db.begin(Some(viewer_id)).await?;
    let Some(row) = tx.matches_get(match_id).await? else {
        return Ok(None);
    };
    if row.players.0 != viewer_id && row.players.1 != viewer_id {
        return Ok(None);
    }
    let standings = tx.ranked_standings(&build_season_id(&SeasonDeps::current())).await?;
    tx.commit().await?;
    let jlorious = jlorious_order(&standings_of(&standings));
    let seat = |profile_id: &str| SeatRank {
        tag: player_tag(profile_id),
        rank: rank_in(&standings, &jlorious, profile_id),
        you: profile_id == viewer_id,
    };
    // A missing flag is unranked (a pre-0022 row, or a room's), so this game moved nothing.
    Ok(Some(MatchRanksBody {
        ranked: row.ranked.unwrap_or(false),
        seats: PerPlayer { p1: seat(&row.players.0), p2: seat(&row.players.1) },
    }))
}

/// The caller behind a route that declares `AuthLevel::Active` (a private copy of
/// `collection.rs`'s `callerProfile`).
fn caller_profile(req: &Req) -> Result<&Profile, ApiError> {
    req.caller.as_ref().map(|caller| &caller.profile).ok_or_else(|| bad_request("this endpoint needs a signed-in profile"))
}

// TS's `createRankedRoutes()`, in its order, is three `AuthLevel::Active` rows of `app.rs`'s
// `ROUTES`: GET /api/ranked → get_ranked; GET /api/leaderboard → get_leaderboard;
// GET /api/matches/:matchId/ranks → get_match_ranks.

/// `GET /api/ranked`: the caller's own rank, record, streak, tag and season badges. Never the
/// rating (R612).
pub async fn get_ranked(app: &App, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    ok_of(&own_rank(app, &profile.id).await?)
}

/// `GET /api/leaderboard`. R612: Jlorious #1–#100, then everyone else by Grape tier, then how many
/// are still placing.
pub async fn get_leaderboard(app: &App, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    ok_of(&leaderboard(app, &profile.id).await?)
}

/// `GET /api/matches/:matchId/ranks`. Both players' ranks for the match screen. 404 when the caller
/// is not one of the match's players, exactly as for a match that does not exist (§9.1: a caller
/// learns nothing about matches it is not in).
pub async fn get_match_ranks(app: &App, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let match_id = req.params.get("matchId").map(String::as_str).unwrap_or("");
    match match_ranks(app, match_id, &profile.id).await? {
        Some(body) => ok_of(&body),
        None => Err(ApiError::new(ApiErrorCode::NotFound, "no such match")),
    }
}
