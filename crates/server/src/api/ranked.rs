//! Ranked ladder management (SPEC §9.12, R603–R612): rating persistence over `crate::ranked`'s rules,
//! for a ranked match's result (R604), a ranked series' end (R262) and the build's season at boot (R609).
//!
//! Never exposes raw ratings to clients (R612); returns `VisibleRank`, usernames, and badges (R1436).
//! The one exception is the dev dashboard's read of the whole ladder (`dev_leaderboard`, R3074002),
//! which only a dev account reaches, through `api::admin`.

use std::sync::Arc;

use indexmap::{IndexMap, IndexSet};
use serde::Serialize;
use serde_json::{Value, json};

use jackioh_engine::PerPlayer;

use crate::api::collection::caller_profile;
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, log_alert, log_info, now_ms, ok_of};
use crate::app::App;
use crate::config::{BOT_GRIND_STEP_GAMES, BOT_STRENGTHS};
use crate::cosmetics::JLORIOUS_CARD_BACK;
use crate::db::store::{
    BotRating, MatchRow, Pilot, Profile, ProfileKind, RatedGameKind, RatedGameRow, RatedReason, RatedSide,
    Season, SeasonStanding, SeriesStatus, StoreError, Tx, ladder_of_mode,
};
use crate::economy::store::{CosmeticGrant, EconomyReason};
use crate::ranked::board::standings_of;
use crate::ranked::game::{RateGameInput, RateSide, decide, position_in};
use crate::ranked::glicko2::{Glicko, START_GLICKO};
use crate::ranked::ladder::{
    GRAPE_TIERS, GrapeTier, Ladder, PeakBadge, RankTier, SeasonRank, VisibleRank, VisibleRankWire,
    jlorious_floor, jlorious_order, peak_badge, place_of, tier_index_of, visible_rank,
};
use crate::ranked::season::{ResetReport, SeasonPlan, plan_season, season_id_of, soft_reset};

// The game's version (R375) and its season (R609)

/// The newest patch's version: the game's version, which names the season (R609). Compiled in
/// from `crates/cards/patches/patches.json` (SURFACE §11.3), the same version the game records
/// file under.
pub fn load_patch_version() -> String {
    jackioh_cards::catalog_version().to_string()
}

/// Dependencies required for opening a season.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeasonDeps {
    pub patch_version: String,
}

impl SeasonDeps {
    /// Returns season dependencies for the current build.
    pub fn current() -> SeasonDeps {
        SeasonDeps {
            patch_version: load_patch_version(),
        }
    }
}

/// R609: the id of the season this build rates games in.
pub fn build_season_id(deps: &SeasonDeps) -> String {
    season_id_of(&deps.patch_version)
}

/// What opening a season did: the season, and the soft reset's report when this call opened it.
#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OpenedSeason {
    pub season: Season,
    pub opened: bool,
    /// The Constructed ladder's soft reset, when this call opened the season.
    pub reset: Option<ResetReport>,
    /// R2778000: the All Random ladder's, reset apart, its ratings pulled toward their own mean.
    pub random_reset: Option<ResetReport>,
}

/// R609: the build's season, opened inside `t` when it does not exist yet. The first season of all
/// resets nothing; every later one soft-resets every rated player in the season row's transaction,
/// so a season is either open and reset or neither. Opens serialize on `ranked_lock_seasons`, so a
/// second opener (racing deploys too) never soft-resets off a snapshot older than the first's row.
pub async fn open_season_in_tx(t: &mut Tx<'_>, deps: &SeasonDeps) -> Result<OpenedSeason, StoreError> {
    t.ranked_lock_seasons().await?;
    let seasons = t.ranked_seasons().await?;
    // R609: which season this is, and whether it is the first of all, is `jackioh-shared`'s to say
    // (`plan_season`); the time it starts is read here.
    let (season, first) = match plan_season(&seasons, &deps.patch_version, now_ms()) {
        SeasonPlan::Exists { season } => {
            return Ok(OpenedSeason {
                season,
                opened: false,
                reset: None,
                random_reset: None,
            });
        }
        SeasonPlan::New { season, first } => (season, first),
    };
    let id = season.id.clone();
    if !t.ranked_create_season(&season).await? {
        let raced = t
            .ranked_seasons()
            .await?
            .into_iter()
            .find(|candidate| candidate.id == id);
        let Some(raced) = raced else {
            return Err(StoreError::Other(format!(
                "season {id} was neither created nor found"
            )));
        };
        return Ok(OpenedSeason {
            season: raced,
            opened: false,
            reset: None,
            random_reset: None,
        });
    }
    if first {
        log_info(
            "season.opened",
            json!({ "seasonId": id, "patchVersion": deps.patch_version, "reset": null }),
        );
        return Ok(OpenedSeason {
            season,
            opened: true,
            reset: None,
            random_reset: None,
        });
    }
    // R2778000: each ladder's ratings reset apart, each toward its own players' mean.
    let mut reports: Vec<ResetReport> = Vec::new();
    for ladder in Ladder::ALL {
        let players = t.ranked_rated_players(ladder).await?;
        let reset = soft_reset(&players);
        t.ranked_reset_ratings(ladder, &reset.changes).await?;
        reports.push(reset.report);
    }
    let random_reset = reports.pop();
    let reset = reports.pop();
    log_info(
        "season.opened",
        json!({
            "seasonId": id,
            "patchVersion": deps.patch_version,
            "reset": serde_json::to_value(&reset).unwrap_or(Value::Null),
            "randomReset": serde_json::to_value(&random_reset).unwrap_or(Value::Null),
        }),
    );
    Ok(OpenedSeason {
        season,
        opened: true,
        reset,
        random_reset,
    })
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

// Rating one ranked game (R603–R608, R610, R611)

/// One side of a ranked game: a player's profile, or one of the AI bots (R610): `jackioh-shared`'s (#3105).
pub use crate::ranked::game::RankedSideInput;

#[derive(serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
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
    #[serde(default)]
    pub stake: Option<i32>,
    /// R2778000: the ladder the game moves, its mode's (`ladder_of_mode`): a Conquest series and a
    /// Best of 1 match the Constructed one, an All Random match its own.
    #[serde(default)]
    pub ladder: Ladder,
}

/// One side's rating write.
#[derive(Clone, Debug)]
pub struct GlickoWrite {
    pub side: RankedSideInput,
    pub glicko: Glicko,
    pub games: i64,
}

/// One bystander's Jlorious peak: `jackioh-shared`'s (#3105).
pub use crate::ranked::game::PeakWrite;

#[derive(Clone, Debug)]
pub struct RankedWrites {
    pub glickos: Vec<GlickoWrite>,
    pub ranks: Vec<SeasonRank>,
    pub peaks: Vec<PeakWrite>,
    /// R2755000: the people this game leaves Jlorious who may not own the Jlorious card back yet:
    /// its own people in Jlorious and each bystander whose peak it writes. Empty while the economy
    /// is off (§14.12).
    pub jlorious: Vec<String>,
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

/// Whether a season row counts toward the percentiles, Jlorious and the public leaderboard, and a 1-based
/// position in Jlorious: `jackioh-shared`'s (#3105), which also decides the game.
pub use crate::ranked::board::counts;

/// A side's rating and rated-game count going into the game, as the store holds them.
#[derive(Clone, Debug)]
struct Before {
    glicko: Glicko,
    games: i64,
    /// R1560000: a ladder bot's rating row, and the AI generation it was last rated on.
    bot_row: bool,
    rated_on: Option<i64>,
}

/// R1560000: the AI generation this build plays its ladder bots on (`crates/ai/generation.json`).
fn ai_generation() -> Option<i64> {
    jackioh_ai::ai_generation().map(i64::from)
}

async fn before_of(
    t: &mut Tx<'_>,
    profiles: &[Profile],
    side: &RankedSideInput,
    game_id: &str,
    ladder: Ladder,
) -> Result<Before, StoreError> {
    let fresh = |glicko: Glicko| Before {
        glicko,
        games: 0,
        bot_row: false,
        rated_on: None,
    };
    match side {
        RankedSideInput::Bot { bot_id } => {
            let bot = t.ranked_bot(bot_id, ladder).await?;
            Ok(match bot {
                // R1560000: a bot last rated on an older AI is rated from a widened deviation
                // (`jackioh_shared::ranked::game::decide` widens it).
                Some(bot) => Before {
                    glicko: bot.glicko,
                    games: bot.games,
                    bot_row: true,
                    rated_on: bot.ai_generation,
                },
                None => fresh(START_GLICKO),
            })
        }
        RankedSideInput::Player { profile_id } => {
            match profiles.iter().find(|candidate| candidate.id == *profile_id) {
                None => {
                    // A ranked game cannot outlive its players (the foreign keys say so): rate it from the
                    // start and shout, as `results.rs` always has.
                    log_alert(
                        "ranked.profile_missing",
                        json!({ "gameId": game_id, "profileId": profile_id }),
                    );
                    Ok(fresh(START_GLICKO))
                }
                Some(profile) => Ok(fresh(profile.glicko(ladder))),
            }
        }
    }
}

/// Plans one ranked game (R603–R608): reads the season, its standings, the sides' profiles and the bots'
/// rows, has `jackioh_shared::ranked::game::decide` rate both sides against each other's rating from before
/// it and move each side's season toward the rank its new rating calls for, and keeps what it decided as the
/// writes. R640000: a ladder bot's season row moves as a person's, never in Jlorious; a bot id with no bot
/// profile (R610's bare rating) holds no row.
pub async fn plan_ranked_game(
    t: &mut Tx<'_>,
    app: &App,
    input: &RankedGameInput,
) -> Result<RankedPlan, StoreError> {
    if let Some(recorded) = t.ranked_game(&input.id).await? {
        return Ok(RankedPlan {
            row: recorded,
            writes: None,
        });
    }

    let (first, second) = (&input.sides.0, &input.sides.1);
    if let (RankedSideInput::Player { profile_id: a }, RankedSideInput::Player { profile_id: b }) =
        (first, second)
        && a == b
    {
        return Err(StoreError::Other(format!(
            "rated game {} names one profile on both sides",
            input.id
        )));
    }
    let sides = [first, second];

    let deps = SeasonDeps::current();
    let OpenedSeason { season, .. } = open_season_in_tx(t, &deps).await?;
    // R2778000: the game's own ladder, its rows and ratings alone.
    let on = input.ladder;
    let standings = t.ranked_standings(&season.id, on).await?;
    let side_ids: Vec<String> = sides
        .iter()
        .map(|side| match side {
            RankedSideInput::Player { profile_id } => profile_id.clone(),
            RankedSideInput::Bot { bot_id } => bot_id.clone(),
        })
        .collect();
    let profiles = t.profiles_get_many(&side_ids).await?;

    let before = [
        before_of(t, &profiles, first, &input.id, on).await?,
        before_of(t, &profiles, second, &input.id, on).await?,
    ];
    // R640000: a bot id names a bot profile, or it is a bare rating that holds no season row.
    let rate_side = |index: usize| RateSide {
        side: sides[index].clone(),
        glicko: before[index].glicko,
        bot_row: before[index].bot_row,
        rated_on: before[index].rated_on,
        bot_profile: match sides[index] {
            RankedSideInput::Bot { bot_id } => profiles
                .iter()
                .any(|profile| profile.id == *bot_id && profile.kind == ProfileKind::Bot),
            RankedSideInput::Player { .. } => false,
        },
    };
    let decision = decide(&RateGameInput {
        standings,
        sides: (rate_side(0), rate_side(1)),
        winner_side: input.winner_side.map(|side| side as i32),
        reason: input.reason,
        stake: input.stake,
        season: season.clone(),
        ladder: on,
        ai_generation: ai_generation(),
        // R3074001: a strict server leaves a hand-set rating out of the percentiles and Jlorious.
        strict: app.env.dev_strict,
        now_ms: input.at,
    });
    let mut jlorious = decision.jlorious;
    if !app.economy_live {
        jlorious.clear();
    }

    let outcomes = [&decision.sides.0, &decision.sides.1];
    let side_of = |side: &RankedSideInput, index: usize| -> RatedSide {
        let outcome = outcomes[index];
        let (profile_id, bot_id, pilot) = match side {
            RankedSideInput::Bot { bot_id } => (None, Some(bot_id.clone()), Pilot::Ai),
            RankedSideInput::Player { profile_id } => (Some(profile_id.clone()), None, Pilot::Human),
        };
        RatedSide {
            profile_id,
            bot_id,
            pilot,
            before: outcome.before,
            after: outcome.after,
            rank_before: outcome.rank_before,
            rank_after: outcome.rank_after,
        }
    };

    let row = RatedGameRow {
        id: input.id.clone(),
        kind: input.kind,
        format: on,
        season_id: season.id.clone(),
        patch_version: deps.patch_version.clone(),
        catalog_version: input.catalog_version.clone(),
        sides: (side_of(first, 0), side_of(second, 1)),
        winner_side: input.winner_side,
        reason: input.reason,
        ended_at: input.at,
    };
    let glickos = sides
        .iter()
        .enumerate()
        .map(|(index, side)| GlickoWrite {
            side: (*side).clone(),
            glicko: outcomes[index].after,
            games: before[index].games + 1,
        })
        .collect();
    Ok(RankedPlan {
        row,
        writes: Some(RankedWrites {
            glickos,
            ranks: decision.ranks,
            peaks: decision.peaks,
            jlorious,
        }),
    })
}

/// Writes a plan inside `t`: both ratings, both season rows, the peaks, the Jlorious card back of
/// each person it leaves Jlorious (R2755000), and the record (R611).
pub async fn commit_ranked_game(t: &mut Tx<'_>, plan: &RankedPlan, at: i64) -> Result<(), StoreError> {
    let Some(writes) = &plan.writes else {
        return Ok(());
    };
    // R2778000: every write lands on the game's own ladder.
    let on = plan.row.format;
    for (index, write) in writes.glickos.iter().enumerate() {
        match &write.side {
            RankedSideInput::Player { profile_id } => {
                t.profiles_set_glicko(profile_id, on, &write.glicko).await?
            }
            RankedSideInput::Bot { bot_id } => {
                t.ranked_put_bot(&BotRating {
                    bot_id: bot_id.clone(),
                    format: on,
                    glicko: write.glicko,
                    games: write.games as _,
                    updated_at: at,
                    ai_generation: ai_generation(),
                })
                .await?;
                // R1555000: a grinder's rated win counts toward its next strength; its rating is
                // the update above alone (R610).
                if plan.row.winner_side == Some(index) {
                    t.bots_note_grind_win(
                        bot_id,
                        BOT_GRIND_STEP_GAMES,
                        BOT_STRENGTHS.len().saturating_sub(1),
                    )
                    .await?;
                }
            }
        }
    }
    for rank in &writes.ranks {
        t.ranked_put_rank(rank).await?;
    }
    for peak in &writes.peaks {
        t.ranked_note_peak_jlorious(
            &plan.row.season_id,
            on,
            &peak.profile_id,
            i64::from(peak.position),
        )
        .await?;
    }
    // R2755000: a grant to someone who owns it already writes nothing.
    for profile_id in &writes.jlorious {
        t.economy_cosmetics_grant(&CosmeticGrant {
            profile_id: profile_id.clone(),
            card_id: None,
            cosmetic_id: JLORIOUS_CARD_BACK.to_string(),
            delta: 1,
            reason: EconomyReason::Jlorious,
            reference: Some(plan.row.id.clone()),
            dev: false,
            at,
        })
        .await?;
    }
    t.ranked_record_game(&plan.row).await?;
    Ok(())
}

/// Plans and writes one ranked game inside `t`. Rating the same id again changes nothing.
pub async fn rate_ranked_game(
    t: &mut Tx<'_>,
    app: &App,
    input: &RankedGameInput,
) -> Result<RatedGameRow, StoreError> {
    let plan = plan_ranked_game(t, app, input).await?;
    commit_ranked_game(t, &plan, input.at).await?;
    Ok(plan.row)
}

// What the client reads (R612)

/// One profile's rank in a season it has standings for.
fn rank_in(standings: &[SeasonStanding], jlorious: &[String], profile_id: &str) -> VisibleRank {
    let standing = standings
        .iter()
        .find(|candidate| candidate.rank.profile_id == profile_id);
    visible_rank(
        standing.map(|standing| &standing.rank),
        position_in(jlorious, profile_id),
    )
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

/// R1436: each listed profile's username as shown, read in the transaction that read the standings
/// it labels, so a rename shows on every read at once and nothing but `profiles` holds a name. A
/// seat a deleted account left empty names nobody and is not looked up; a profile with no row
/// shows an empty name.
async fn usernames_of<'a>(
    tx: &mut Tx<'_>,
    profile_ids: impl IntoIterator<Item = &'a str>,
) -> Result<IndexMap<String, String>, StoreError> {
    let wanted: IndexSet<String> = profile_ids
        .into_iter()
        .filter(|profile_id| !profile_id.is_empty())
        .map(str::to_string)
        .collect();
    let wanted: Vec<String> = wanted.into_iter().collect();
    Ok(tx
        .profiles_get_many(&wanted)
        .await?
        .iter()
        .map(|profile| (profile.id.clone(), profile.username()))
        .collect())
}

/// A profile's username out of `usernames_of`'s answer.
fn username_in(usernames: &IndexMap<String, String>, profile_id: &str) -> String {
    usernames.get(profile_id).cloned().unwrap_or_default()
}

/// `GET /api/ranked`: the caller's own season, profile id, username and badges, on one ladder.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OwnRankBody {
    pub season: String,
    /// R2778000: the ladder the body reads, `?ladder=`'s (Constructed when none is asked for).
    pub ladder: Ladder,
    /// R1436: for keys and requests only; a player is shown by `username`.
    pub profile_id: String,
    pub username: String,
    pub rank: VisibleRank,
    /// The current win streak, which earns bonus pips below Mythic Grape (R606).
    pub streak: i64,
    pub record: RankRecord,
    /// Each season's best, newest season first; a season whose placements were never finished has none.
    pub badges: Vec<PeakBadge>,
}

pub async fn own_rank(app: &App, profile_id: &str, ladder: Ladder) -> Result<OwnRankBody, ApiError> {
    let season_id = build_season_id(&SeasonDeps::current());
    let mut tx = app.db.begin(Some(profile_id)).await?;
    // #2022: the rows shown and Mythic Grape's, all that a rank and Jlorious read.
    let standings = tx
        .ranked_standings_subset(&season_id, ladder, &[profile_id.to_string()], jlorious_floor())
        .await?;
    let history = tx.ranked_ranks_of(profile_id).await?;
    let usernames = usernames_of(&mut tx, [profile_id]).await?;
    tx.commit().await?;
    let jlorious = jlorious_order(&standings_of(&standings, app.env.dev_strict));
    let mine = standings
        .iter()
        .find(|standing| standing.rank.profile_id == profile_id)
        .map(|standing| &standing.rank);
    Ok(OwnRankBody {
        season: season_id,
        ladder,
        profile_id: profile_id.to_string(),
        username: username_in(&usernames, profile_id),
        rank: rank_in(&standings, &jlorious, profile_id),
        streak: mine.map(|rank| rank.streak as i64).unwrap_or(0),
        record: RankRecord {
            games: mine.map(|rank| rank.games as i64).unwrap_or(0),
            wins: mine.map(|rank| rank.wins as i64).unwrap_or(0),
            losses: mine.map(|rank| rank.losses as i64).unwrap_or(0),
            draws: mine.map(|rank| rank.draws as i64).unwrap_or(0),
        },
        // R2778000: the ladder's own seasons' badges.
        badges: history
            .iter()
            .filter(|rank| rank.format == ladder)
            .filter_map(peak_badge)
            .rev()
            .collect(),
    })
}

/// One row of a Grape tier on the leaderboard.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardRow {
    /// R1436: the client's list key; the row is shown by `username`.
    pub profile_id: String,
    pub username: String,
    pub division: i64,
    pub pips: i64,
    pub you: bool,
}

/// One Jlorious place on the leaderboard, #1 first.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JloriousRow {
    pub position: i32,
    pub profile_id: String,
    pub username: String,
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
    /// R2778000: the ladder the board shows, `?ladder=`'s (Constructed when none is asked for).
    pub ladder: Ladder,
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
    profile_id: String,
    ladder: i32,
    you: bool,
}

pub async fn leaderboard(app: &App, viewer_id: &str, ladder: Ladder) -> Result<LeaderboardBody, ApiError> {
    let season_id = build_season_id(&SeasonDeps::current());
    let mut tx = app.db.begin(Some(viewer_id)).await?;
    let standings = tx.ranked_standings(&season_id, ladder).await?;
    let usernames = usernames_of(
        &mut tx,
        standings.iter().map(|standing| standing.rank.profile_id.as_str()),
    )
    .await?;
    tx.commit().await?;
    let jlorious = jlorious_order(&standings_of(&standings, app.env.dev_strict));
    let in_jlorious: IndexSet<&str> = jlorious.iter().map(String::as_str).collect();

    let tier_count = GRAPE_TIERS.len();
    let mut groups: Vec<Vec<TierEntry>> = vec![Vec::new(); tier_count];
    let mut raisins = 0;
    for standing in &standings {
        // R3074001: a strict server leaves a hand-set rating off the public ladder, placed or not.
        if app.env.dev_strict && standing.hand_set {
            continue;
        }
        let Some(ladder) = standing.rank.ladder else {
            raisins += 1;
            continue;
        };
        if in_jlorious.contains(standing.rank.profile_id.as_str()) {
            continue;
        }
        let index = (tier_index_of(ladder) as usize).min(tier_count.saturating_sub(1));
        groups[index].push(TierEntry {
            profile_id: standing.rank.profile_id.clone(),
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
            // A tie on the ladder is broken by profile id, which no rename moves (R1436).
            rows.sort_by(|a, b| {
                b.ladder
                    .cmp(&a.ladder)
                    .then_with(|| a.profile_id.cmp(&b.profile_id))
            });
            LeaderboardTier {
                tier: *tier,
                count: rows.len(),
                players: rows
                    .iter()
                    .map(|row| {
                        let place = place_of(row.ladder);
                        LeaderboardRow {
                            profile_id: row.profile_id.clone(),
                            username: username_in(&usernames, &row.profile_id),
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
        ladder,
        jlorious: jlorious
            .iter()
            .enumerate()
            .map(|(index, profile_id)| JloriousRow {
                position: index as i32 + 1,
                profile_id: profile_id.clone(),
                username: username_in(&usernames, profile_id),
                you: profile_id == viewer_id,
            })
            .collect(),
        tiers,
        raisins,
        you: rank_in(&standings, &jlorious, viewer_id),
    })
}

/// R3074002: one row of the dev dashboard's whole ladder: every season row on it, a ladder bot's
/// as a person's, each marked which it is, with its rating, record and visible rank, and a bot's
/// strength and persona beside it.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DevLeaderboardRow {
    pub profile_id: String,
    pub username: String,
    /// `person` or `bot`: the one place a client is told (R1452's exception, R3074002).
    pub kind: &'static str,
    pub rating: f64,
    pub games: i64,
    pub wins: i64,
    pub losses: i64,
    pub draws: i64,
    pub rank: VisibleRank,
    /// R3074001: the row's player holds a hand-set rating on this ladder.
    pub hand_set: bool,
    pub you: bool,
    /// A ladder bot's `bots` row, read-only: its strength (an index of `BOT_STRENGTHS`), its persona
    /// and whether it is retired. Null for a person.
    pub bot: Option<Value>,
}

/// R3074002: `GET /api/admin/leaderboard`'s body: the season, the ladder and every row on it.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DevLeaderboardBody {
    pub season: String,
    pub ladder: Ladder,
    /// Highest rating first; a tie by profile id.
    pub rows: Vec<DevLeaderboardRow>,
}

/// R3074002: the whole ladder as the dev dashboard shows it: every season row on `ladder`, people
/// and bots alike, each row's visible rank read as the public leaderboard and the match screen read
/// it (Jlorious people's alone, and on a strict server not a hand-set rating's, R3074001). Only
/// `api::admin`, behind its dev check, calls this; `GET /api/leaderboard` is unchanged.
pub async fn dev_leaderboard(
    app: &App,
    viewer_id: &str,
    ladder: Ladder,
) -> Result<DevLeaderboardBody, ApiError> {
    let season_id = build_season_id(&SeasonDeps::current());
    let mut tx = app.db.begin(Some(viewer_id)).await?;
    let standings = tx.ranked_standings(&season_id, ladder).await?;
    let usernames = usernames_of(
        &mut tx,
        standings.iter().map(|standing| standing.rank.profile_id.as_str()),
    )
    .await?;
    let bots = tx.bots_list().await?;
    tx.commit().await?;
    let jlorious = jlorious_order(&standings_of(&standings, app.env.dev_strict));
    let mut rows: Vec<DevLeaderboardRow> = standings
        .iter()
        .map(|standing| {
            let profile_id = standing.rank.profile_id.as_str();
            let bot = standing
                .bot
                .then(|| bots.iter().find(|bot| bot.profile_id == profile_id))
                .flatten()
                .map(|bot| {
                    json!({
                        "strength": bot.strength,
                        "persona": bot.persona.as_str(),
                        "retired": bot.retired_at.is_some(),
                    })
                });
            DevLeaderboardRow {
                profile_id: profile_id.to_string(),
                username: username_in(&usernames, profile_id),
                kind: if standing.bot { "bot" } else { "person" },
                rating: standing.rating,
                games: standing.rank.games as i64,
                wins: standing.rank.wins as i64,
                losses: standing.rank.losses as i64,
                draws: standing.rank.draws as i64,
                rank: rank_in(&standings, &jlorious, profile_id),
                hand_set: standing.hand_set,
                you: profile_id == viewer_id,
                bot,
            }
        })
        .collect();
    rows.sort_by(|a, b| {
        b.rating
            .total_cmp(&a.rating)
            .then_with(|| a.profile_id.cmp(&b.profile_id))
    });
    Ok(DevLeaderboardBody {
        season: season_id,
        ladder,
        rows,
    })
}

/// One seat on the match screen.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SeatRank {
    /// R1436: empty for a seat a deleted account left, as the match row holds it.
    pub profile_id: String,
    pub username: String,
    pub rank: VisibleRank,
    /// R2101000: the seat's ranked win streak off its season row (R606), as `GET /api/ranked` sends
    /// the caller's; 0 for a seat with no row.
    pub streak: i64,
    pub you: bool,
    /// R2388000: the seat's visible rank before and after this match's rated record (R611; a ranked
    /// series' last game, the series', R604). Absent until it is written and on an unranked match or
    /// a series' earlier game: "not recorded yet", never "no change". Never the rating (R612).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank_before: Option<VisibleRank>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank_after: Option<VisibleRank>,
}

/// R2388000: the rated record a match's seats moved by: its own for a ranked match, the series'
/// for the last game of a ranked series that is over (R604), none otherwise.
async fn rated_game_id(tx: &mut Tx<'_>, row: &MatchRow) -> Result<Option<String>, StoreError> {
    if row.ranked != Some(true) {
        return Ok(None);
    }
    Ok(match tx.series_with_game(&row.id).await? {
        None => Some(row.id.clone()),
        Some(series) => {
            let last = series.games.last().is_some_and(|game| game.match_id == row.id);
            if series.status == SeriesStatus::Over && last {
                Some(series.id)
            } else {
                None
            }
        }
    })
}

/// `GET /api/matches/:matchId/ranks`: both seats' ranks and win streaks for the match screen (R612, R2101000), and once its rated record is written each seat's rank before and after it (R2388000).
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MatchRanksBody {
    /// R604: whether the match is ranked. A ranked match's end moves the rating and the ladder
    /// itself; a ranked series' games carry the flag while the series moves the rating once,
    /// when it ends.
    pub ranked: bool,
    /// R2778000: the ladder the seats' ranks are read on, the match's mode's.
    pub ladder: Ladder,
    pub seats: PerPlayer<SeatRank>,
}

pub async fn match_ranks(
    app: &App,
    match_id: &str,
    viewer_id: &str,
) -> Result<Option<MatchRanksBody>, ApiError> {
    let mut tx = app.db.begin(Some(viewer_id)).await?;
    let Some(row) = tx.matches_get(match_id).await? else {
        return Ok(None);
    };
    if row.players.0 != viewer_id && row.players.1 != viewer_id {
        return Ok(None);
    }
    // R2778000: the match's own ladder; a match no queue, room or series made reads Constructed.
    let ladder = tx
        .matches_mode_of(match_id)
        .await?
        .map_or(Ladder::Constructed, ladder_of_mode);
    let standings = tx
        .ranked_standings_subset(
            &build_season_id(&SeasonDeps::current()),
            ladder,
            &[row.players.0.clone(), row.players.1.clone()],
            jlorious_floor(),
        )
        .await?;
    let usernames = usernames_of(&mut tx, [row.players.0.as_str(), row.players.1.as_str()]).await?;
    let rated = match rated_game_id(&mut tx, &row).await? {
        Some(id) => tx.ranked_game(&id).await?,
        None => None,
    };
    tx.commit().await?;
    // R640000: a ladder bot's seat shows its own season row, read as a player's is, as the
    // leaderboard shows it: nothing on the screen tells the two apart.
    let jlorious = jlorious_order(&standings_of(&standings, app.env.dev_strict));
    let seat = |profile_id: &str| {
        // R2388000: the seat's side of the record, a ladder bot's named by its bot id (R640000).
        let side = rated.as_ref().and_then(|game| {
            [&game.sides.0, &game.sides.1]
                .into_iter()
                .find(|side| side.profile_id.as_deref().or(side.bot_id.as_deref()) == Some(profile_id))
        });
        SeatRank {
            profile_id: profile_id.to_string(),
            username: username_in(&usernames, profile_id),
            rank: rank_in(&standings, &jlorious, profile_id),
            streak: standings
                .iter()
                .find(|standing| standing.rank.profile_id == profile_id)
                .map(|standing| standing.rank.streak as i64)
                .unwrap_or(0),
            you: profile_id == viewer_id,
            rank_before: side.and_then(|side| side.rank_before),
            rank_after: side.and_then(|side| side.rank_after),
        }
    };
    // A missing flag is unranked (a pre-0022 row, or a room's), so this game moved nothing.
    Ok(Some(MatchRanksBody {
        ranked: row.ranked.unwrap_or(false),
        ladder,
        seats: PerPlayer {
            p1: seat(&row.players.0),
            p2: seat(&row.players.1),
        },
    }))
}

/// R1442: the ladder tier each of a match's players holds now (seat order, `MatchRow.players`), read
/// as `match_ranks` reads a rank, for the play telemetry's rank bucket. R2778000: on the match's own
/// ladder.
pub async fn ladder_tiers(
    app: &App,
    match_id: &str,
    players: &(String, String),
) -> Result<(RankTier, RankTier), StoreError> {
    let mut tx = app.db.begin(None).await?;
    let ladder = tx
        .matches_mode_of(match_id)
        .await?
        .map_or(Ladder::Constructed, ladder_of_mode);
    let standings = tx
        .ranked_standings_subset(
            &build_season_id(&SeasonDeps::current()),
            ladder,
            &[players.0.clone(), players.1.clone()],
            jlorious_floor(),
        )
        .await?;
    tx.commit().await?;
    let jlorious = jlorious_order(&standings_of(&standings, app.env.dev_strict));
    let tier = |profile_id: &str| VisibleRankWire::from(rank_in(&standings, &jlorious, profile_id)).tier;
    Ok((tier(&players.0), tier(&players.1)))
}

// Routes

/// `GET /api/ranked`: the caller's own rank, record, streak, username and season badges. Never the
/// rating (R612). R2778000: on `?ladder=` (`constructed`, the default, or `random`).
pub async fn get_ranked(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let ladder = Ladder::parse(req.query.get("ladder").map(String::as_str));
    ok_of(&own_rank(app, &profile.id, ladder).await?)
}

/// `GET /api/leaderboard`. R612: Jlorious #1–#100, then everyone else by Grape tier, then how many
/// are still placing. R2778000: of `?ladder=` (`constructed`, the default, or `random`).
pub async fn get_leaderboard(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let ladder = Ladder::parse(req.query.get("ladder").map(String::as_str));
    ok_of(&leaderboard(app, &profile.id, ladder).await?)
}

/// `GET /api/matches/:matchId/ranks`. Both players' ranks for the match screen. 404 when the caller
/// is not one of the match's players, exactly as for a match that does not exist (§9.1: a caller
/// learns nothing about matches it is not in).
pub async fn get_match_ranks(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let match_id = req.params.get("matchId").map(String::as_str).unwrap_or("");
    match match_ranks(app, match_id, &profile.id).await? {
        Some(body) => ok_of(&body),
        None => Err(ApiError::new(ApiErrorCode::NotFound, "no such match")),
    }
}
