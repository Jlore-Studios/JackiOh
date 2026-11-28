//! Public card and player statistics page API (SPEC §9.11, R654).
//!
//! R1367000 (amends R654): `GET /api/stats/cards` reads each card over its own window, through the
//! engine's `windowed_card_stats`; the drill-down's per-patch rates stay per patch.
//!
//! R978000: the card figures are read off a patch's totals once and kept for
//! `CARD_STATS_CACHE_TTL_SECONDS`, one report per patch, `?source=`, `?pilot=` and `?mode=` (`KeptReports`).
//! R1044000: the totals are the records folded as written (`db::store::Tx::stats_totals_load`). R2491000
//! (amends R978000): the drill-down's patch history is one kept history per `?pilot=` and `?mode=`
//! (`KeptHistory`), shared by every card's page.
//!
//! R3067000 (amends R978000, R1367000, R2491000): every kept window and history is cut from one
//! snapshot of every patch's totals (`kept_snapshot`), kept `CARD_STATS_CACHE_TTL_SECONDS` and loaded once
//! however many keys miss it, and a patch the server does not ship reads its own totals alone, never kept.
//!
//! R1583000: `GET /api/stats/play-rates` is each card's play rate among the human seats of every live
//! game, kept for `PLAY_RATES_REFRESH_SECONDS`. R1771000: `PUT /api/stats/player` keeps R1142000's summary
//! inputs alone, for catalog cards, and drops every other key, id and value (`account_copy`).
//!
//! R1994000: `GET /api/stats/games` is the patch's game figures (R1256000's counts) over the window every
//! card shares, read through the engine's `windowed_game_figures` off `StatsTotals.game_totals` (which
//! outlive R1044001's purge), kept with the cards' windowed report and cached as long.
//!
//! R4673000, R4673001: both card routes take `?face=all|normal|radiant`, All when missing or unrecognized:
//! on Normal or Radiant each card's figures are the seats that played it on that face, read against that
//! face's own field and band (`face_fields`, `card_figures`), and the drill-down's history and drills follow
//! the face. The drill-down also answers the card's Normal and Radiant rates and the gap between them, at
//! the page's `?patch=`.
//!
//! R4677000: anyone but a dev account reads the newest patches alone (`public_patch_count`): a patch
//! beyond them is 403 `patch_dev_only` on `GET /api/stats/cards` and `GET /api/stats/games`, and the
//! patch list, the patch before and the drill-down's history stop at them. The kept figures are the same
//! for every caller; only what an answer lists of them is the caller's (`Reader`), and a dev account's
//! answer is kept by no cache (`reader_ok`).
//!
//! R4675001: `GET /api/stats/versions` is every play version's figures (R4675000), through the engine's
//! `play_version_figures`, one table per `?pilot=` and `?mode=` cut from the same snapshot and cached as
//! long. It sends figures only: a client reads a version's definition from its catalog at `last`.
//! R5186000: the kept snapshot loads on a task of its own, so a read that gives up leaves the load to
//! finish for the next read, and `GET /api/stats/loading` answers what that load is doing (its step, how
//! long it has run, and how long the last one took), never waiting and kept by no cache, for the page's
//! progress bar.
//! R5225000: the CPU-bound work of the statistics (`Snapshot::new`, `window_over`) runs off the runtime's
//! worker threads (`blocking`), so the runtime keeps answering other requests while it runs, and a window's
//! figures are cut on a task of its own, counted as the load's last step.
//! R5245000 (amends R978000, R1583000, R3067000, R5186000): a kept snapshot lasts from when its load
//! finished, and a load under way is joined whatever its age. Once a value is held, no read waits on its
//! next: a read past its time gets the held value and starts one refresh beside it (`kept_value`). The
//! snapshot past its time is let go as its next loads, so one is held at a time, and the figures cut from it
//! answer meanwhile (`kept_snapshot`).
//! R5040000: `?scope=major` sends the live play versions alone, each cut to the newest major's line
//! (`major_line_versions`) and read against those rows' own field, one table per scope too.
//!
//! R5174000 (amends R4965000): the snapshot the cards, the game figures, the drill-down and the win-rate
//! snapshot read loads the card totals of the versions it keeps as filed alone (`Reach::Kept`), a read of
//! those patches' rows. The play versions' table and the Hall of Fame read the older play versions folded
//! too (`Reach::Folded`), the one read that goes over every version's card totals, made only when one of
//! them is asked for.

use std::cmp::Ordering;
use std::future::Future;
use std::hash::Hash;
use std::sync::atomic::{self, AtomicBool};
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;
use indexmap::{IndexMap, IndexSet};
use serde::Serialize;
use serde_json::{Map, Value, json};
use tokio::runtime::{Handle, RuntimeFlavor};
use tokio::sync::OnceCell;

use jackioh_cards::{
    change_totals_since, deck_play_versions, patch_balance_lines, patch_history, patch_ship_dates,
};
use jackioh_engine::wire::stats::PlayVersion;
use jackioh_engine::wire::stats::{
    CardDrill, CardStats, CardStatsFilter, CardStatsReport, CardTier, CardWindow, CreatedStats, GameFigures,
    GameMode, GameSource, HallPair, ModeFilter, PatchStep, PatchWindow, PilotFilter, PilotPairFigures,
    PlayVersionFigures, PlayVersionQuery, PlayVersionTable, RolledKind, SourceFilter, StatsFace, StatsField,
    StatsTotals, SwingReport, SwingRows, SwingTally, Tally, TotalsKey, VariantKind, WindowQuery,
    WindowRequest, WindowSource, WindowedCreated, WindowedGames, WindowedReport, add_created, add_stats,
    card_family, card_stats_of, card_window, collapse_gameless, created_face_adjusted, created_face_field,
    created_face_side, face_adjusted, face_drills_of, face_side, face_stats_field, hall_of_fame,
    held_adjusted, major_line_versions, not_drawn, output_field, output_figures, patch_windows, play_rate,
    play_version_figures, radiant_gap, rolled_field, rolled_figures, round_share, seen, sources_of,
    stats_field, swing_baseline, swing_report, tier_in, variant_field, variant_figures, variant_rows,
    wilson_interval, win_rate, windowed_card_stats, windowed_game_figures, windowed_swing_rows,
};
use jackioh_engine::{
    CardCost, CardDef, CardDefs, Pilot, WinRateTable, made_by_a_rule, win_rates_from_report,
};

use crate::api::catalog::load_patch_versions;
use crate::api::collection::caller_profile;
use crate::api::dev::{dev_caller, is_dev};
use crate::api::http::{
    ApiError, ApiErrorCode, ApiResult, Req, bad_request, lock, now_ms, ok_of, open_route_profile,
    query_trimmed, to_json, whole_number,
};
use crate::app::App;
use crate::config::{
    CARD_STATS_CACHE_TTL_SECONDS, CARD_STATS_CO_PLAYED_TOP, CARD_STATS_CURVE_TOP,
    CARD_STATS_KEPT_IDLE_SECONDS, CARD_STATS_KEPT_REPORTS, CARD_STATS_MAX_AGE_SECONDS, CARD_STATS_MIN_SAMPLE,
    HALL_OF_FAME_LIST_LEN, MS_PER_SECOND, PLAY_RATES_MIN_GAMES, PLAY_RATES_REFRESH_SECONDS,
    PLAYER_STATS_BYTES_MAX, PLAYER_STATS_CACHE_TTL_SECONDS, PLAYER_STATS_PAGE_LIMIT,
    PUBLIC_STATS_MIN_LIVE_GAMES, PUBLIC_STATS_PATCHES_LINE_BEFORE, PUBLIC_STATS_PATCHES_NEWEST,
};
use crate::db::store::{CardFiling, CardRows, Db, PlayerStatsListOptions, StatsTotalsQuery};
use crate::env::js_number;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicCardStat {
    pub id: String,
    pub name: String,
    pub cost: i32,
    pub rarity: String,
    pub set: String,
    pub games: i64,
    pub win_rate: Option<f64>,
    // R3185000: no drawn, played, face (R1147000), drawn-not-played or never-drawn (R2427000) breakdown and
    // no played or drawn delta: no client reads them. The adjusted rate, its range and the tier below are
    // still read off those tallies.
    /// R2777000: the drawn delta kept as far as its games back it, read from an even game
    /// (`adjusted_win_rate`): the page's headline rate and its default order.
    pub adjusted_win_rate: Option<f64>,
    /// R2895000: the 95% Wilson interval of `win_rate`, over the games it counts; null where it is.
    pub win_rate_low: Option<f64>,
    pub win_rate_high: Option<f64>,
    /// R2895000: the 95% interval of `adjusted_win_rate` (`adjusted_interval`); null where it is.
    pub adjusted_low: Option<f64>,
    pub adjusted_high: Option<f64>,
    /// R4552001, R4552003: how far a play of the card typically moves its player's chance of winning,
    /// either way, as a fraction, shrunk toward the typical card's (`swing_report`); null with no turn
    /// rows. The only figure of R4552001's a client reads: no Swing and no Board swing.
    pub decisiveness: Option<f64>,
    /// R3022000: the card's balance tier (`tier_of` its adjusted rate and range); null below R654's
    /// floor, as its rate is shown only from it.
    pub tier: Option<CardTier>,
    /// R3950000: how often the card was played when seen (`play_rate_of`), in every mode.
    pub play_rate: f64,
    pub has_enough_games: bool,
    /// R1367000: the card's window and the records its figures count.
    pub window: PublicCardWindow,
}

/// R2089000: a Created card's figures, apart from the deck table: the seats that made a Created copy
/// of it, each once a game. `games` is every one of them, used or not.
///
/// R4083000: only what `/stats` reads; the used and unused figures stay in the totals for `stats-cards`
/// and `stats report`.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicCreatedStat {
    pub id: String,
    pub name: String,
    pub cost: i32,
    pub rarity: String,
    pub set: String,
    pub games: i64,
    /// R4082000: the made games' win rate read against the Created cards (`created_adjusted`): an
    /// average Created card reads an even game. R4673000: on a face, the seats that used a copy on it,
    /// against that face's Created field.
    pub adjusted_win_rate: Option<f64>,
    /// R4082000: the 95% range of `adjusted_win_rate`, R2895000's.
    pub adjusted_low: Option<f64>,
    pub adjusted_high: Option<f64>,
    /// R4552002, R4552004: its Decisiveness, read against the Created cards' (`PublicCardStat`'s).
    pub decisiveness: Option<f64>,
    /// R4082000: the card's balance tier (`tier_of` its adjusted rate and range, R3022000); null below
    /// R654's floor, as its rate is shown only from it.
    pub tier: Option<CardTier>,
    pub has_enough_games: bool,
    pub window: PublicCardWindow,
}

impl PublicCreatedStat {
    /// `created` the Created field and `deck` the deck cards' (R4082000). R4673000: on `face`, the seats
    /// that used a copy on it (`created_face_side`), `created` that face's Created field.
    fn of(
        def: &CardDef,
        row: &WindowedCreated,
        face: StatsFace,
        created: &StatsField,
        deck: &StatsField,
        swing: &SwingReport,
    ) -> PublicCreatedStat {
        let stats = &row.stats;
        let figures = PublicFaceFigures::created(stats, face, created, deck);
        PublicCreatedStat {
            id: stats.card.clone(),
            name: def.name.clone(),
            cost: numeric_cost(def),
            rarity: def.rarity.as_str().to_string(),
            set: def.set.as_str().to_string(),
            games: figures.games,
            adjusted_win_rate: figures.adjusted_win_rate,
            adjusted_low: figures.adjusted_low,
            adjusted_high: figures.adjusted_high,
            // R4673000: All's on every face, as a deck card's is.
            decisiveness: swing
                .created
                .get(&stats.card)
                .and_then(|figures| figures.decisiveness),
            tier: figures.tier,
            has_enough_games: figures.has_enough_games,
            window: PublicCardWindow::of(&row.window),
        }
    }
}

/// R1367000: a card's statistics window at the patch asked about.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicCardWindow {
    /// The version that opened the window: its X's start, or the last micro that played the card.
    pub from: String,
    /// The live games played on the window's versions, which R654's gate counts.
    pub live_games: i64,
    pub source: PublicStatsSource,
    /// The versions whose games the figures count, oldest first.
    pub counted: Vec<String>,
}

impl PublicCardWindow {
    fn of(window: &CardWindow) -> PublicCardWindow {
        PublicCardWindow {
            from: window.versions.first().cloned().unwrap_or_default(),
            live_games: window.live_games,
            source: PublicStatsSource::of(window.source),
            counted: window.counted.clone(),
        }
    }
}

/// Which records the public figures were counted from.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum PublicStatsSource {
    Provisional,
    Live,
    /// R1367000: the live games of the previous window, the window's own being short.
    Previous,
    Dev,
}

impl PublicStatsSource {
    /// The label the cards response shows beside the source.
    pub fn label(self) -> &'static str {
        match self {
            PublicStatsSource::Provisional => "AI games + live games (provisional)",
            PublicStatsSource::Live => "Live games",
            PublicStatsSource::Previous => "Live games (previous patch)",
            PublicStatsSource::Dev => "AI development games",
        }
    }

    fn of(source: WindowSource) -> PublicStatsSource {
        match source {
            WindowSource::Live => PublicStatsSource::Live,
            WindowSource::Previous => PublicStatsSource::Previous,
            WindowSource::Provisional => PublicStatsSource::Provisional,
            WindowSource::Dev => PublicStatsSource::Dev,
        }
    }
}

/// `?source=` as `get_cards` reads it below R654's gate: both sources unless the query names one
/// (`live`, or `dev`/`ai`). At or above the gate the live games count whatever it says.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum SourceParam {
    Default,
    Live,
    Dev,
}

impl SourceParam {
    /// R1367000: what the query asks of a window's gate.
    fn request(self) -> WindowRequest {
        match self {
            SourceParam::Default => WindowRequest::Gated,
            SourceParam::Live => WindowRequest::Live,
            SourceParam::Dev => WindowRequest::Dev,
        }
    }

    fn parse(raw: Option<&str>) -> SourceParam {
        match raw.map(|value| value.trim().to_lowercase()).as_deref() {
            Some("live") => SourceParam::Live,
            Some("dev") | Some("ai") => SourceParam::Dev,
            _ => SourceParam::Default,
        }
    }
}

/// Which seats the public figures count (R807000): merged counts every counted seat, human and
/// bot alike, and is what the endpoint displays; human and ai split the two sides.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum PublicStatsPilot {
    Merged,
    Human,
    Ai,
}

impl PublicStatsPilot {
    /// `?pilot=`: merged unless the query asks for a side.
    pub fn parse(raw: Option<&str>) -> PublicStatsPilot {
        match raw.map(|value| value.trim().to_lowercase()) {
            Some(value) if value == "human" => PublicStatsPilot::Human,
            Some(value) if value == "ai" => PublicStatsPilot::Ai,
            _ => PublicStatsPilot::Merged,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PublicStatsPilot::Merged => "Human and bot seats",
            PublicStatsPilot::Human => "Human seats",
            PublicStatsPilot::Ai => "Bot seats",
        }
    }

    pub fn filter(self) -> PilotFilter {
        match self {
            PublicStatsPilot::Merged => PilotFilter::Unified,
            PublicStatsPilot::Human => PilotFilter::Human,
            PublicStatsPilot::Ai => PilotFilter::Ai,
        }
    }
}

/// SPEC §11 R5040000: which play versions `GET /api/stats/versions` sends: every one over its whole span
/// (`?scope=` missing or unrecognized), or the live ones cut to the newest major's line (`?scope=major`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PublicStatsScope {
    All,
    Major,
}

impl PublicStatsScope {
    /// `?scope=`: `major`, or every play version when missing or unrecognized.
    pub fn parse(raw: Option<&str>) -> PublicStatsScope {
        match raw.map(|value| value.trim().to_lowercase()).as_deref() {
            Some("major") => PublicStatsScope::Major,
            _ => PublicStatsScope::All,
        }
    }
}

/// Which match type the public figures count (SPEC §9.11 Queries, R377): every mode, one of R257's
/// three modes, or (R2777001) Constructed, Best of 1 and Conquest together. `All` counts what the
/// endpoint always counted.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum PublicStatsMode {
    All,
    Bo1,
    Bo3,
    Random,
    Constructed,
}

impl PublicStatsMode {
    /// `?mode=`: one match type, `constructed`, or every mode when missing, `all`, or unrecognized.
    pub fn parse(raw: Option<&str>) -> PublicStatsMode {
        match raw.map(|value| value.trim().to_lowercase()).as_deref() {
            Some("bo1") => PublicStatsMode::Bo1,
            Some("bo3") => PublicStatsMode::Bo3,
            Some("random") => PublicStatsMode::Random,
            Some("constructed") => PublicStatsMode::Constructed,
            _ => PublicStatsMode::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PublicStatsMode::All => "All modes",
            PublicStatsMode::Bo1 => "Best of 1",
            PublicStatsMode::Bo3 => "Conquest",
            PublicStatsMode::Random => "All Random",
            PublicStatsMode::Constructed => "Constructed",
        }
    }

    pub fn filter(self) -> ModeFilter {
        match self {
            PublicStatsMode::All => ModeFilter::All,
            PublicStatsMode::Bo1 => ModeFilter::Only(GameMode::Bo1),
            PublicStatsMode::Bo3 => ModeFilter::Only(GameMode::Bo3),
            PublicStatsMode::Random => ModeFilter::Only(GameMode::Random),
            PublicStatsMode::Constructed => ModeFilter::Constructed,
        }
    }

    /// R2777002: the games a card's own figures count: under All Random, whose decks are dealt, only the
    /// decks that saw it (its drawn games, `seen`); a card dealt and never drawn is no game of its. In
    /// every other mode, every deck that held it.
    pub fn counted(self, stats: &CardStats) -> Tally {
        match self {
            PublicStatsMode::Random => seen(stats),
            _ => stats.in_deck,
        }
    }

    /// R4673000: the games a card's own figures count on `face`: All's `counted`, or on Normal or Radiant
    /// the seats that played it on that face (`face_side`), in every mode.
    pub fn counted_on(self, stats: &CardStats, face: StatsFace) -> Tally {
        match face {
            StatsFace::All => self.counted(stats),
            StatsFace::Normal | StatsFace::Radiant => face_side(stats, face),
        }
    }
}

/// R4673001: `?face=`: one face of each card, or All when missing or unrecognized.
fn face_of(req: &Req) -> StatsFace {
    req.query
        .get("face")
        .and_then(|value| value.trim().to_lowercase().parse().ok())
        .unwrap_or(StatsFace::All)
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicStatsGate {
    pub cleared: bool,
    pub live_games: i64,
    pub min_live_games: i64,
}

/// The best or the worst card above the sample threshold.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicStatsExtreme {
    pub id: String,
    pub name: String,
    pub win_rate: f64,
    pub games: i64,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicStatsSummary {
    pub total_games: i64,
    pub live_games: i64,
    pub active_patch: String,
    pub source: PublicStatsSource,
    pub pilot: PublicStatsPilot,
    pub mode: PublicStatsMode,
    pub best_card: Option<PublicStatsExtreme>,
    pub worst_card: Option<PublicStatsExtreme>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicStatsCardsResponse {
    pub patch: String,
    pub previous_patch: Option<String>,
    pub gate: PublicStatsGate,
    pub source: PublicStatsSource,
    pub source_label: String,
    pub pilot: PublicStatsPilot,
    pub pilot_label: String,
    pub mode: PublicStatsMode,
    pub mode_label: String,
    pub min_sample: i64,
    pub total_games: i64,
    pub cards: Vec<PublicCardStat>,
    /// R2089000: the Created cards, tokens among them, apart from `cards` and filtered alike.
    pub created: Vec<PublicCreatedStat>,
    pub summary: PublicStatsSummary,
    /// R3022000: the field the adjusted rates and tiers are read against, and the band.
    pub tiers: PublicStatsTiers,
    /// R1973002: every patch the page can show, newest first: the window the cards share, its
    /// versions oldest first (ask for its figures at the last), its games and its live games.
    /// R4677000: only those the caller may read.
    pub patches: Vec<PatchWindow>,
    /// R4677000: the caller is no dev account, so the patches older than `patches` lists are for dev
    /// accounts alone.
    pub older_patches_for_devs: bool,
}

/// R3070000: the seats that made a copy of `id` during their games, used or not (R2089000): the games of a
/// card only a rule makes. R4673000: on a face, those that used one on it. `None` when no counted seat made
/// one.
fn made_games(report: &WindowedReport, id: &str, face: StatsFace) -> Option<Tally> {
    let row = report.created.iter().find(|row| row.stats.card == id)?;
    Some(created_face_side(&row.stats, face))
}

/// R3022000, R4082000, R4673000: the fields a face's figures are read against: All's deck field (`all`,
/// whose average deck a card a rule makes is read against on every face), the face's deck field (`deck`,
/// All's on All) and the face's Created field (`created`). Each reads every card of the report the catalog
/// holds, whatever the page's filters show, so a set's page reads its cards as the whole page does.
struct FaceFields {
    all: StatsField,
    deck: StatsField,
    created: StatsField,
}

fn face_fields(app: &App, report: &WindowedReport, face: StatsFace) -> FaceFields {
    // R3022000: every deckable card of the report, its centre and spread from the cards whose drawn delta
    // the page shows (R2427000's floor on both its sides). R3070000: every card counts toward the average
    // deck's win rate, which a card with fewer never-drawn games than the floor is read against.
    let floor = i32::try_from(CARD_STATS_MIN_SAMPLE).unwrap_or(i32::MAX);
    let deck_cards = || {
        app.catalog
            .defs
            .iter()
            .filter(|(_, def)| !def.token)
            .filter_map(|(id, _)| report.card(id))
            .map(|card| &card.stats)
    };
    let all = stats_field(deck_cards(), floor);
    let deck = match face {
        StatsFace::All => all,
        StatsFace::Normal | StatsFace::Radiant => face_stats_field(deck_cards(), face, floor),
    };
    // R4082000: every Created card of the report the catalog holds.
    let created = created_face_field(
        report
            .created
            .iter()
            .filter(|row| app.catalog.defs.contains_key(&row.stats.card))
            .map(|row| &row.stats),
        face,
        &all,
    );
    FaceFields { all, deck, created }
}

/// One deck card's figures on one face, as its row shows them.
struct CardFigures {
    games: i64,
    win_rate: Option<f64>,
    /// R2895000: the 95% Wilson interval of `win_rate`.
    win_range: Option<(f64, f64)>,
    /// R2777000, R2895000: the adjusted win rate and its 95% range.
    adjusted: Option<(f64, (f64, f64))>,
    tier: Option<CardTier>,
    has_enough_games: bool,
}

/// R654, R2777000, R3022000, R4673000: a deck card's figures on `face` over the windowed report, read
/// against `fields` (`face_fields` of the same face).
fn card_figures(
    report: &WindowedReport,
    id: &str,
    def: &CardDef,
    mode: PublicStatsMode,
    face: StatsFace,
    fields: &FaceFields,
) -> CardFigures {
    let stat = report.card(id).map(|card| &card.stats);
    // R3070000: a card only a rule makes, the Dormant Starship (R1857001), is in no deck: its row reads
    // the seats that made one, its Created games (R2089000).
    let made = made_by_a_rule(def)
        .then(|| made_games(report, id, face))
        .flatten();
    // R2777002: the games the card's own figures count: under All Random, the decks that saw it.
    let counted = made.unwrap_or_else(|| stat.map(|stat| mode.counted_on(stat, face)).unwrap_or_default());
    let (games, wins) = tally_of(&counted);
    let rate = win_rate(&counted);
    // R2895000: the rate's 95% range, over the same games.
    let win_range = rate.map(|_| wilson_interval(wins, games));
    let has_enough_games = games >= CARD_STATS_MIN_SAMPLE;
    // R2777000: shrunk by its own games, so it needs no floor of its own; the page shows it from R654's.
    // R3022000: read against the field. R3070000: a card a rule makes, against the average deck, as its
    // Created row reads it (R4082000).
    let adjusted = match made {
        Some(made) => held_adjusted(&made, &fields.all),
        None => stat.and_then(|stat| face_adjusted(stat, face, &fields.deck)),
    };
    // R3022000: the tier, from R654's floor as the rate is shown.
    let tier = adjusted
        .filter(|_| has_enough_games)
        .map(|(rate, (low, high))| tier_in(rate, low, high, face.band()));
    CardFigures {
        games,
        win_rate: rate,
        win_range,
        adjusted,
        tier,
        has_enough_games,
    }
}

/// R3950000: a card's play rate, how often it was played when seen: its played games over its seen
/// games (played, or drawn and never played; the engine's `seen`), in every mode. A card only a rule
/// makes (R3070000) reads its used Created games over the games a seat made one. 0 when it was never seen.
fn play_rate_of(report: &WindowedReport, id: &str, stat: Option<&CardStats>, made: Option<&Tally>) -> f64 {
    let created = report
        .created
        .iter()
        .find(|row| row.stats.card == id)
        .map(|row| &row.stats);
    play_rate(stat, created, made).unwrap_or(0.0)
}

/// R3022000: what `GET /api/stats/cards`' tiers are read against: the field (`stats_field`) of every card
/// in the report, whatever the page's filters show, and the balance band.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicStatsTiers {
    /// The field's mean drawn delta, as a fraction: what an average card shows, read as an even game.
    pub center: f64,
    /// The spread (one standard deviation) of real card lifts, as a fraction.
    pub spread: f64,
    /// The cards the field was read from; 0 when too few were, and R2777000's centre and prior are kept.
    pub cards: usize,
    /// R3070000: the average deck's win rate, which a card with too few never-drawn games, and the Dormant
    /// Starship, is read against.
    pub baseline: f64,
    /// The band: tier S is above `band_high`, tier F below `band_low` (`TIER_BAND_HIGH`, `TIER_BAND_LOW`;
    /// R4673000: on a face, that face's own band and field).
    pub band_low: f64,
    pub band_high: f64,
    /// R4552001: the typical deck card's Decisiveness (`DecisivenessField::typical`), which the page
    /// names beside each card's.
    pub decisiveness: f64,
    /// R4082000: the field the Created cards a card's text makes are read against.
    pub created: PublicCreatedTiers,
}

/// R4082000: what a Created card's adjusted win rate and tier are read against (`created_field`).
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicCreatedTiers {
    /// How much more often the seats that make an average Created card win than the average deck, as a
    /// fraction; 0 when too few Created cards were read, and each is read against the average deck.
    pub center: f64,
    /// The spread (one standard deviation) of real Created cards' lifts; the deck cards' when too few.
    pub spread: f64,
    /// The Created cards the field was read from; 0 when too few were.
    pub cards: usize,
    /// The win rate of the seats that make an average Created card, which reads an even game: the average
    /// deck's plus `center`.
    pub baseline: f64,
    /// R4552002: the typical Created card's Decisiveness.
    pub decisiveness: f64,
}

/// R1994000: `GET /api/stats/games`'s body: counts and shares only, naming no game and no player.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicStatsGamesResponse {
    pub patch: String,
    /// The window every card shares, as `GET /api/stats/cards`' `source` reads it.
    pub source: PublicStatsSource,
    pub source_label: String,
    pub mode: PublicStatsMode,
    pub mode_label: String,
    /// The games below which the page shows none of the figures.
    pub min_sample: i64,
    /// Every record the window counts, its whole game kept or, folded before migration 0063, not.
    pub total_games: i64,
    /// The counted records' whole games, or null when none is kept.
    pub figures: Option<GameFigures>,
    /// The same split by both seats' pilots, p1's first: a ladder bot's games (R1451) apart.
    pub pairs: Vec<PilotPairFigures>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DrillDownCard {
    pub id: String,
    pub name: String,
    pub cost: i32,
    pub rarity: String,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PatchRate {
    pub patch: String,
    pub games: i64,
    pub win_rate: Option<f64>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TurnRate {
    pub turn: i32,
    pub games: i64,
    pub win_rate: Option<f64>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CoPlayedRate {
    pub id: String,
    pub name: String,
    pub games: i64,
    pub win_rate: Option<f64>,
}

/// R4673000: a card's figures on one face, as the drill-down's face line reads them.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicFaceFigures {
    pub games: i64,
    pub adjusted_win_rate: Option<f64>,
    pub adjusted_low: Option<f64>,
    pub adjusted_high: Option<f64>,
    pub tier: Option<CardTier>,
    pub has_enough_games: bool,
}

impl PublicFaceFigures {
    /// R4082000, R4673000: a Created card's figures on `face`, the seats that used a copy on it
    /// (`created_face_side`), read against `created`, that face's Created field, and `deck`, All's deck
    /// field: its Created row's on that face (`PublicCreatedStat::of`) and the drill-down's face line alike.
    fn created(
        stats: &CreatedStats,
        face: StatsFace,
        created: &StatsField,
        deck: &StatsField,
    ) -> PublicFaceFigures {
        let games = i64::from(created_face_side(stats, face).games);
        let has_enough_games = games >= CARD_STATS_MIN_SAMPLE;
        let adjusted = created_face_adjusted(stats, face, created, deck);
        PublicFaceFigures {
            games,
            adjusted_win_rate: adjusted.map(|(rate, _)| rate),
            adjusted_low: adjusted.map(|(_, (low, _))| low),
            adjusted_high: adjusted.map(|(_, (_, high))| high),
            tier: adjusted
                .filter(|_| has_enough_games)
                .map(|(rate, (low, high))| tier_in(rate, low, high, face.band())),
            has_enough_games,
        }
    }

    /// The rate and its range, `(rate, (low, high))`, from R654's floor, as the page shows it.
    fn shown(&self) -> Option<(f64, (f64, f64))> {
        let rate = self.adjusted_win_rate.filter(|_| self.has_enough_games)?;
        Some((rate, (self.adjusted_low?, self.adjusted_high?)))
    }
}

/// R4673000: the Radiant gap, the card's Radiant adjusted rate less its Normal one (`radiant_gap`), and its
/// 95% range.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicRadiantGap {
    pub gap: f64,
    pub low: f64,
    pub high: f64,
}

/// R4673001: the drill-down's face line: the card's Normal and Radiant figures, each read against its own
/// face's field, and the gap between them; null when either face is below R654's floor.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicCardFaces {
    pub normal: PublicFaceFigures,
    pub radiant: PublicFaceFigures,
    pub gap: Option<PublicRadiantGap>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CardDrillDownResponse {
    pub card: DrillDownCard,
    pub patches: Vec<PatchRate>,
    pub by_turn: Vec<TurnRate>,
    pub co_played: Vec<CoPlayedRate>,
    /// R4673001: the face `patches`, `byTurn` and `coPlayed` read.
    pub face: StatsFace,
    /// R4673001: the card's Normal and Radiant figures and their gap, at the page's patch, on every face.
    pub faces: PublicCardFaces,
    /// R4676000: the card's choices at the page's patch on the face asked, one group per kind it has; empty
    /// for a card that offers none.
    pub variants: Vec<PublicVariantGroup>,
    /// R4679000: each card it made or offered at the page's patch on the face asked, best adjusted first;
    /// empty for a card that made and offered none.
    pub outputs: Vec<PublicOutputRow>,
    /// R4707001: each outcome its plays rolled at the page's patch on the face asked, one group per kind it
    /// has (its table's effects, its coins, its dice), best adjusted first; empty for a card that rolls none.
    pub rolled: Vec<PublicRolledGroup>,
}

/// R4676000: one choice of a card, as Extended stats shows it.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicVariantRow {
    pub key: String,
    pub label: String,
    /// A `mode` row's options as recorded (`VariantRow::options`), for the page to name in the card's own
    /// words; absent for the other kinds.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    pub games: i64,
    /// The row's games over the card's played games on the face: how often players choose it.
    pub share: f64,
    pub adjusted_win_rate: Option<f64>,
    pub adjusted_low: Option<f64>,
    pub adjusted_high: Option<f64>,
    pub tier: Option<CardTier>,
    pub has_enough_games: bool,
}

/// R4676000: one kind of a card's choices, its rows by share, most chosen first.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicVariantGroup {
    pub kind: VariantKind,
    pub rows: Vec<PublicVariantRow>,
}

/// R4679000: one card a card made or offered, as Extended stats' Outputs shows it.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicOutputRow {
    /// The output's catalog id, which its drill-down opens.
    pub key: String,
    pub label: String,
    pub cost: i32,
    pub rarity: String,
    pub games: i64,
    /// Its games over every output's on the face: how often the card gives it.
    pub share: f64,
    pub adjusted_win_rate: Option<f64>,
    pub adjusted_low: Option<f64>,
    pub adjusted_high: Option<f64>,
    pub tier: Option<CardTier>,
    pub has_enough_games: bool,
    /// A Discover-like card's: the seat-games the output was offered in; absent for a card that offers none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offered: Option<i64>,
    /// Picked over offered; absent when it was never offered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pick_rate: Option<f64>,
}

/// R4707001: one outcome a card rolled, as Extended stats' Outputs shows it.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicRolledRow {
    /// `<table>:<outcome>`: the card whose table, coins or dice it is, and the outcome's key there.
    pub key: String,
    /// The outcome's words; a Recursion's edition's own entry is prefixed with that edition's name.
    pub label: String,
    /// How often its table expects it per roll, and how often it came up.
    pub expected: f64,
    pub share: f64,
    pub games: i64,
    pub adjusted_win_rate: Option<f64>,
    pub adjusted_low: Option<f64>,
    pub adjusted_high: Option<f64>,
    pub tier: Option<CardTier>,
    pub has_enough_games: bool,
}

/// R4707001: one kind of a card's rolled outcomes, its rows best adjusted first.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicRolledGroup {
    pub kind: RolledKind,
    pub rows: Vec<PublicRolledRow>,
}

/// R4676000: a card's baseline, the games a variant is read against, and its played games on `face`, a
/// variant's share's denominator: a deck card's never-drawn games and plays, or a Created card no deck held
/// its copies made and never used (R2089000) and its used ones.
fn variant_sides(report: &WindowedReport, id: &str, face: StatsFace) -> Option<(Tally, Tally)> {
    if let Some(card) = report.card(id) {
        let played = match face {
            StatsFace::All => card.stats.played,
            StatsFace::Normal => card.stats.played_normal,
            StatsFace::Radiant => card.stats.played_radiant,
        };
        return Some((not_drawn(&card.stats), played));
    }
    let row = report.created.iter().find(|row| row.stats.card == id)?;
    let used = match face {
        StatsFace::All => row.stats.created_played,
        StatsFace::Normal | StatsFace::Radiant => created_face_side(&row.stats, face),
    };
    Some((row.stats.created_not_played, used))
}

/// R4676000: the card's choices on `face` over the windowed report, each read against the variant field of
/// every card of the report the catalog holds, whatever the page filters (`variant_field`). A row with no
/// games on the face is left out, and so is a kind left with none.
fn card_variants(app: &App, report: &WindowedReport, id: &str, face: StatsFace) -> Vec<PublicVariantGroup> {
    let Some(choices) = report.choices.get(id) else {
        return Vec::new();
    };
    let Some((never, played)) = variant_sides(report, id, face) else {
        return Vec::new();
    };
    let floor = i32::try_from(CARD_STATS_MIN_SAMPLE).unwrap_or(i32::MAX);
    let field = variant_field(
        report
            .choices
            .iter()
            .filter(|(card, _)| app.catalog.defs.contains_key(*card))
            .filter_map(|(card, choices)| Some((choices, variant_sides(report, card, face)?.0))),
        face,
        floor,
    );
    let mut groups: Vec<PublicVariantGroup> = Vec::new();
    for row in variant_rows(choices) {
        let seen = row.games.on(face);
        if seen.games == 0 {
            continue;
        }
        let figures = variant_figures(&seen, &never, face, &field);
        let public = PublicVariantRow {
            options: row.options(),
            key: row.key,
            label: row.label,
            games: i64::from(figures.games),
            share: if played.games > 0 {
                round_share(f64::from(seen.games) / f64::from(played.games))
            } else {
                0.0
            },
            adjusted_win_rate: figures.adjusted.map(|(rate, _)| rate),
            adjusted_low: figures.adjusted.map(|(_, (low, _))| low),
            adjusted_high: figures.adjusted.map(|(_, (_, high))| high),
            tier: figures.tier,
            has_enough_games: figures.has_enough_games,
        };
        match groups.last_mut() {
            Some(group) if group.kind == row.kind => group.rows.push(public),
            _ => groups.push(PublicVariantGroup {
                kind: row.kind,
                rows: vec![public],
            }),
        }
    }
    for group in &mut groups {
        // A stable sort: rows of one share keep `variant_rows`' order.
        group.rows.sort_by_key(|row| std::cmp::Reverse(row.games));
    }
    groups
}

/// R4679000: each card the card made or offered on `face` over the windowed report, read against its other
/// outputs and the output field of every source of the report the catalog holds, whatever the page filters
/// (`output_field`), in `output_figures`' order. A card the catalog lacks is left out, so every row's
/// drill-down opens.
fn card_outputs(app: &App, report: &WindowedReport, id: &str, face: StatsFace) -> Vec<PublicOutputRow> {
    let Some(choices) = report.choices.get(id) else {
        return Vec::new();
    };
    let floor = i32::try_from(CARD_STATS_MIN_SAMPLE).unwrap_or(i32::MAX);
    let field = output_field(
        report
            .choices
            .iter()
            .filter(|(card, _)| app.catalog.defs.contains_key(*card))
            .map(|(_, choices)| choices),
        face,
        floor,
    );
    output_figures(choices, face, &field)
        .into_iter()
        .filter_map(|row| {
            let def = app.catalog.defs.get(&row.card)?;
            Some(PublicOutputRow {
                label: def.name.clone(),
                cost: numeric_cost(def),
                rarity: def.rarity.as_str().to_string(),
                games: i64::from(row.games),
                share: row.share,
                adjusted_win_rate: row.adjusted.map(|(rate, _)| rate),
                adjusted_low: row.adjusted.map(|(_, (low, _))| low),
                adjusted_high: row.adjusted.map(|(_, (_, high))| high),
                tier: row.tier,
                has_enough_games: row.has_enough_games,
                offered: row.offered.map(i64::from),
                pick_rate: row.pick_rate,
                key: row.card,
            })
        })
        .collect()
}

/// R4707001: each outcome the card's plays rolled on `face` over the windowed report, read against its
/// group's other outcomes and the rolled field of every card of the report the catalog holds, whatever the
/// page filters (`rolled_field`), one group per kind in `RolledKind`'s order, each in `rolled_figures`'
/// order. An outcome of another edition's table (a Recursion's cast, R28) names that edition first.
fn card_rolled(app: &App, report: &WindowedReport, id: &str, face: StatsFace) -> Vec<PublicRolledGroup> {
    let Some(choices) = report.choices.get(id) else {
        return Vec::new();
    };
    let floor = i32::try_from(CARD_STATS_MIN_SAMPLE).unwrap_or(i32::MAX);
    let field = rolled_field(
        report
            .choices
            .iter()
            .filter(|(card, _)| app.catalog.defs.contains_key(*card))
            .map(|(_, choices)| choices),
        face,
        floor,
    );
    let rows = rolled_figures(choices, face, &field);
    RolledKind::ALL
        .iter()
        .map(|kind| PublicRolledGroup {
            kind: *kind,
            rows: rows
                .iter()
                .filter(|row| row.kind == *kind)
                .map(|row| PublicRolledRow {
                    key: format!("{}:{}", row.table, row.key),
                    label: match app.catalog.defs.get(&row.table) {
                        Some(def) if row.table != id => format!("{}: {}", def.name, row.label),
                        _ => row.label.clone(),
                    },
                    expected: row.expected,
                    share: row.share,
                    games: i64::from(row.games),
                    adjusted_win_rate: row.adjusted.map(|(rate, _)| rate),
                    adjusted_low: row.adjusted.map(|(_, (low, _))| low),
                    adjusted_high: row.adjusted.map(|(_, (_, high))| high),
                    tier: row.tier,
                    has_enough_games: row.has_enough_games,
                })
                .collect(),
        })
        .filter(|group| !group.rows.is_empty())
        .collect()
}

/// A `public, max-age=…` answer for a route that knows nothing about its caller.
///
/// Only an `AuthLevel::None` route whose handler never reads the caller may use this: no
/// `req.caller`, `caller_of`/`caller_profile`, `Authorization`, `Cookie`, `Accept-Language`,
/// `X-Forwarded-*`, `X-Original-URL`/`X-Rewrite-URL`, or any other caller-carried header, and
/// unknown query parameters must stay ignored. Shared caches key on the URL alone, so anything
/// caller-shaped in the body would be served to every later caller (cache poisoning). A route that
/// reads whether its caller is a dev account answers through `reader_ok` instead (R4677000).
pub(crate) fn cached_ok(body: Value, max_age_seconds: i64) -> Response {
    let mut response = Response::new(Body::from(body.to_string()));
    *response.status_mut() = StatusCode::OK;
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    if let Ok(value) = HeaderValue::from_str(&format!("public, max-age={max_age_seconds}")) {
        headers.insert(header::CACHE_CONTROL, value);
    }
    response
}

/// SPEC §11 R4677000: who asks for `/stats`' patches: a dev account (`api::dev::is_dev`) reads every patch
/// back to v0.1.0, anyone else, signed in or not, the newest (`public_patch_count`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reader {
    Anyone,
    Dev,
}

/// R4677000: the request's reader, a dev account when its token names one.
async fn reader_of(app: &App, req: &Req) -> Result<Reader, ApiError> {
    let profile = open_route_profile(app, &req.headers).await?;
    Ok(if profile.is_some_and(|profile| is_dev(app, &profile)) {
        Reader::Dev
    } else {
        Reader::Anyone
    })
}

/// R4677000: a version's line, its `major.minor` (`v0.6.12d` is `v0.6`).
fn patch_line(version: &str) -> &str {
    match version.match_indices('.').nth(1) {
        Some((at, _)) => &version[..at],
        None => version,
    }
}

/// SPEC §11 R4677000: how many of `patches`, newest first as `GET /api/stats/cards` lists them, anyone but
/// a dev account may read: the longer of two newest-first runs, every patch of the newest patch's line
/// and then the `PUBLIC_STATS_PATCHES_LINE_BEFORE` newest of the line before it, or the
/// `PUBLIC_STATS_PATCHES_NEWEST` newest. Both are prefixes of the list, so the longer is their union.
pub fn public_patch_count(patches: &[PatchWindow]) -> usize {
    fn line(window: &PatchWindow) -> Option<&str> {
        window.versions.first().map(|version| patch_line(version))
    }
    let run = |from: usize| match patches.get(from) {
        Some(first) => patches[from..]
            .iter()
            .take_while(|window| line(window) == line(first))
            .count(),
        None => 0,
    };
    let current = run(0);
    let before = run(current).min(PUBLIC_STATS_PATCHES_LINE_BEFORE);
    (current + before)
        .max(PUBLIC_STATS_PATCHES_NEWEST)
        .min(patches.len())
}

/// R4677000: how many of `patches`, newest first, `reader` may read.
fn readable_count(patches: &[PatchWindow], reader: Reader) -> usize {
    match reader {
        Reader::Dev => patches.len(),
        Reader::Anyone => public_patch_count(patches),
    }
}

/// R4677000: whether `version` is one of a listed patch's past the first `readable` of `patches`. A
/// version no listed patch holds is no shipped patch, and reads its own totals as before (R3067000).
fn past_readable(patches: &[PatchWindow], readable: usize, version: &str) -> bool {
    patches
        .iter()
        .position(|window| window.versions.iter().any(|held| held == version))
        .is_some_and(|at| at >= readable)
}

/// R4677000: what a patch past the caller's answers, 403 `patch_dev_only`, before any figure is read.
const PATCH_DEV_ONLY_MESSAGE: &str = "Older patches are for dev accounts.";

/// R4677000: `patch`, refused when it is past the first `readable` of `patches`.
fn refuse_past_readable(patches: &[PatchWindow], readable: usize, patch: &str) -> Result<(), ApiError> {
    if past_readable(patches, readable, patch) {
        return Err(ApiError::new(ApiErrorCode::PatchDevOnly, PATCH_DEV_ONLY_MESSAGE));
    }
    Ok(())
}

/// R1973002: every patch the page can show, newest first: the current patch's kept window lists them, as
/// every kept window does (`kept_window`).
async fn listed_patches(app: &App, history: &[PatchStep]) -> Result<Arc<KeptWindow>, ApiError> {
    shipped_window(
        app,
        history,
        &app.catalog.version,
        SourceParam::Default,
        PublicStatsPilot::Merged,
        PublicStatsMode::All,
    )
    .await
}

/// R4677000: a statistics answer that depends on who asks. Anyone's is `cached_ok`'s, public for
/// `CARD_STATS_MAX_AGE_SECONDS` (R5274000); a dev account's is kept by no cache (`private, no-store`); and each
/// varies by `Authorization`, so no cache hands one caller's answer to another.
fn reader_ok(body: Value, reader: Reader) -> Response {
    let mut response = cached_ok(body, CARD_STATS_MAX_AGE_SECONDS);
    let headers = response.headers_mut();
    if reader == Reader::Dev {
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("private, no-store"),
        );
    }
    headers.insert(header::VARY, HeaderValue::from_static("Authorization"));
    response
}

/// A card's cost as a number: its printed cost, the base of an embiggen cost, 0 for X.
fn numeric_cost(def: &CardDef) -> i32 {
    match def.cost {
        CardCost::Fixed(cost) => cost,
        CardCost::Embiggen { base, .. } => base,
        CardCost::X => 0,
    }
}

/// Name order without a collator: case-insensitive first, then as written.
fn locale_compare(a: &str, b: &str) -> Ordering {
    a.to_lowercase().cmp(&b.to_lowercase()).then_with(|| a.cmp(b))
}

/// A tally's games and wins as `i64`s.
fn tally_of(tally: &Tally) -> (i64, i64) {
    (tally.games as i64, tally.wins as i64)
}

/// R1044000: one patch's totals (every patch's with `None`) over both sources and every mode, each
/// card's turn and co-played counts only with `drills`, and (R4965000) the card totals `cards` keeps.
async fn totals_for(
    app: &App,
    patch: Option<&str>,
    drills: bool,
    cards: CardRows,
) -> Result<StatsTotals, ApiError> {
    totals_in(&app.db, patch, drills, cards).await
}

/// `totals_for` on the store `db`, which a load on a task of its own owns (R5186000).
async fn totals_in(
    db: &Db,
    patch: Option<&str>,
    drills: bool,
    cards: CardRows,
) -> Result<StatsTotals, ApiError> {
    let mut tx = db.begin(None).await?;
    let totals = tx
        .stats_totals_load(&StatsTotalsQuery {
            source: SourceFilter::All,
            mode: None,
            patch: patch.map(str::to_string),
            drills,
            cards,
        })
        .await?;
    tx.commit().await?;
    Ok(totals)
}

/// R3067000: every patch's totals as kept: one load when none was made in the last
/// `CARD_STATS_CACHE_TTL_SECONDS`, once however many requests miss it together. A load that fails is
/// kept for no one: the next request makes it again. R4965000: the records and game totals are read
/// first, and the card totals then as `snapshot_filing` files them by those records. R5174000: only the
/// versions kept as filed unless `reach` asks for the older play versions too, which only the play
/// versions' table does, so the page's own read costs the patches it shows and not every version filed.
/// R5186000: the load runs on a task of its own, so a read that gives up (the client's timeout, a proxy
/// closing the connection) leaves it running for the next read to join. R5245000: the snapshot is kept
/// from when its load finished. The one past its time is let go as its next begins to load, so one is held
/// at a time; meanwhile the figure `held` finds, cut from the one let go, answers at once
/// (`CutFrom::Held`), and only a read with none waits for the load.
async fn kept_snapshot<T>(
    app: &App,
    reach: Reach,
    held: impl FnOnce() -> Option<T>,
) -> Result<CutFrom<T>, ApiError> {
    let (keep, folds) = app.card_stats.totals_cell(now_ms(), reach);
    if let Some(snapshot) = keep.cell.get() {
        return Ok(CutFrom::Snapshot(KeptTotals {
            made_at: snapshot.loaded_at,
            held: Arc::clone(snapshot),
        }));
    }
    let source = SnapshotSource {
        db: app.db.clone(),
        version: app.catalog.version.clone(),
        load: Arc::clone(&app.card_stats.load),
        folds,
    };
    if let Some(figure) = held() {
        refresh(keep, async move { source.load().await });
        return Ok(CutFrom::Held(figure));
    }
    let snapshot = kept_value(keep, async move { source.load().await }).await?;
    Ok(CutFrom::Snapshot(KeptTotals {
        made_at: snapshot.loaded_at,
        held: snapshot,
    }))
}

/// R5245000: what a figure cut from the kept snapshot is read from.
enum CutFrom<T> {
    /// The snapshot held.
    Snapshot(KeptTotals),
    /// The figure cut from the snapshot let go, while its next loads.
    Held(T),
}

/// R5245000: a kept value: its cell, filled by the first read that needs it while the others wait on it;
/// the value it replaced, served while the cell fills; and whether a refresh is filling it.
#[derive(Clone)]
struct Keep<T> {
    cell: Arc<OnceCell<T>>,
    last: Option<T>,
    refreshing: Arc<AtomicBool>,
}

impl<T> Keep<T> {
    fn new(last: Option<T>) -> Keep<T> {
        Keep {
            cell: Arc::new(OnceCell::new()),
            last,
            refreshing: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// R5245000: `keep`'s value. The one its cell holds; else, when it replaced one, that one at once, its
/// cell then filled by `fill` on a task of its own, one refresh at a time, which a failure leaves for the
/// next read to start again; else `fill`'s, on a task of its own (R5186000: a read that gives up leaves it
/// to finish for the next), joined by every read that misses it together.
async fn kept_value<T, F>(keep: Keep<T>, fill: F) -> Result<T, ApiError>
where
    T: Clone + Send + Sync + 'static,
    F: Future<Output = Result<T, ApiError>> + Send + 'static,
{
    if let Some(value) = keep.cell.get() {
        return Ok(value.clone());
    }
    if let Some(last) = keep.last.clone() {
        refresh(keep, fill);
        return Ok(last);
    }
    let cell = keep.cell;
    tokio::spawn(async move { cell.get_or_try_init(|| fill).await.cloned() })
        .await
        .map_err(|error| ApiError::internal(format!("the card statistics stopped: {error}")))?
}

/// R5245000: fills `keep`'s cell with `fill` on a task of its own, unless a refresh fills it already. One
/// that fails leaves the cell empty for the next read to fill.
fn refresh<T, F>(keep: Keep<T>, fill: F)
where
    T: Send + Sync + 'static,
    F: Future<Output = Result<T, ApiError>> + Send + 'static,
{
    if keep.refreshing.swap(true, atomic::Ordering::SeqCst) {
        return;
    }
    tokio::spawn(async move {
        if let Err(error) = keep.cell.get_or_try_init(|| fill).await {
            tracing::warn!(event = "stats.refresh.failed", message = %error.message);
        }
        keep.refreshing.store(false, atomic::Ordering::SeqCst);
    });
}

/// R5186000: the steps of a load of the kept snapshot, in order, which `GET /api/stats/loading` numbers
/// from 1.
#[derive(Clone, Copy)]
enum LoadStep {
    /// The records and game totals.
    Records = 1,
    /// The card totals, as `snapshot_filing` files them by the records.
    CardTotals = 2,
    /// The snapshot every kept window and history is cut from.
    Snapshot = 3,
}

/// R5186000: how many steps a load of the kept snapshot takes.
const LOAD_STEPS: u32 = LoadStep::Snapshot as u32;

/// R5186000: what the kept snapshot's loads are doing, which `GET /api/stats/loading` answers: how many
/// are under way, the step the newest is on and when it began, and how long the last one that finished
/// took.
#[derive(Default)]
struct SnapshotLoad {
    running: u32,
    step: u32,
    started_at: i64,
    last_ms: Option<i64>,
}

/// R5186000: one load of the kept snapshot as `SnapshotLoad` counts it: under way from `begin`, at each
/// `step`, and over when dropped, its time kept as the last load's only once it is `done`.
struct LoadTicket {
    load: Arc<Mutex<SnapshotLoad>>,
    started_at: i64,
    done: bool,
}

impl LoadTicket {
    fn begin(load: Arc<Mutex<SnapshotLoad>>) -> LoadTicket {
        LoadTicket::begin_at(load, LoadStep::Records)
    }

    /// A load under way from `step`. R5225000: a load that begins while another runs (a window's figures
    /// cut beside the snapshot's load) keeps the first one's start, so the time shown never runs backwards.
    fn begin_at(load: Arc<Mutex<SnapshotLoad>>, step: LoadStep) -> LoadTicket {
        let started_at = now_ms();
        {
            let mut state = lock(&load);
            if state.running == 0 {
                state.started_at = started_at;
            }
            state.running += 1;
            state.step = step as u32;
        }
        LoadTicket {
            load,
            started_at,
            done: false,
        }
    }

    fn step(&self, step: LoadStep) {
        lock(&self.load).step = step as u32;
    }
}

impl Drop for LoadTicket {
    fn drop(&mut self) {
        let mut state = lock(&self.load);
        state.running = state.running.saturating_sub(1);
        if self.done {
            state.last_ms = Some((now_ms() - self.started_at).max(0));
        }
        if state.running == 0 {
            state.step = 0;
        }
    }
}

/// R3067000, R5186000: what a load of the kept snapshot reads from, owned, so the load can run on a task
/// of its own.
struct SnapshotSource {
    db: Db,
    version: String,
    load: Arc<Mutex<SnapshotLoad>>,
    folds: bool,
}

impl SnapshotSource {
    /// R4965000, R5174000: the records and game totals, then the card totals as `snapshot_filing` files
    /// them by those records, then the snapshot, each a step `GET /api/stats/loading` reports (R5186000).
    async fn load(&self) -> Result<Arc<Snapshot>, ApiError> {
        let mut ticket = LoadTicket::begin(Arc::clone(&self.load));
        let history = patch_history();
        let records = totals_in(&self.db, None, false, CardRows::None).await?;
        let mut filing = snapshot_filing(&history, &records, &deck_play_versions(), &self.version);
        if !self.folds {
            filing.folded.clear();
        }
        ticket.step(LoadStep::CardTotals);
        let totals = totals_in(&self.db, None, false, CardRows::Filed(filing.clone())).await?;
        ticket.step(LoadStep::Snapshot);
        // R5245000: the snapshot is kept from when its load finished, its totals read.
        let snapshot = Arc::new(blocking(|| Snapshot::new(&history, totals, filing, now_ms())));
        ticket.done = true;
        Ok(snapshot)
    }
}

/// R5225000: CPU-bound work kept off the runtime's worker threads, so they keep answering other requests
/// (`GET /api/stats/loading` among them) while it runs. Run in async code, the statistics' work held them:
/// on staging's quarter of a processor every request waited behind it. On a multi-thread runtime, the
/// server's, `block_in_place` hands the worker's other tasks to another thread for the work's length. A
/// one-thread runtime (a test's paused clock) has no other thread, so the work runs in place there and the
/// test's turns stay its own: the blocking pool's real threads would race a paused clock's yields.
fn blocking<T>(work: impl FnOnce() -> T) -> T {
    match Handle::try_current().map(|runtime| runtime.runtime_flavor()) {
        Ok(RuntimeFlavor::MultiThread) => tokio::task::block_in_place(work),
        _ => work(),
    }
}

/// R5225000: what cutting a window's figures from the snapshot reads, owned, so the cut can run on a task
/// of its own.
struct WindowCut {
    db: Db,
    held: Arc<Snapshot>,
    history: Vec<PatchStep>,
    at: String,
    source: SourceParam,
    pilot: PublicStatsPilot,
    mode: PublicStatsMode,
    load: Arc<Mutex<SnapshotLoad>>,
}

impl WindowCut {
    /// The window's figures, cut off the runtime's worker threads (`blocking`) from the snapshot's totals
    /// or, for a patch past the snapshot, its own reach loaded now (R4965000). The cut counts as the load's
    /// last step, which `GET /api/stats/loading` reports; the last load's time stays the snapshot's.
    async fn window(&self) -> Result<Arc<KeptWindow>, ApiError> {
        let _ticket = LoadTicket::begin_at(Arc::clone(&self.load), LoadStep::Snapshot);
        let own = if self.held.holds(&self.at) {
            None
        } else {
            Some(totals_in(&self.db, None, false, self.held.reach_rows(&self.at)).await?)
        };
        let totals = own.as_ref().unwrap_or(&self.held.totals);
        let window = blocking(|| {
            window_over(
                totals,
                &self.history,
                &self.at,
                self.source,
                self.pilot,
                self.mode,
            )
        });
        Ok(Arc::new(window))
    }
}

/// R5186000: `GET /api/stats/loading`'s answer.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatsLoading {
    /// Whether a load of the kept snapshot is under way.
    pub loading: bool,
    /// The step it is on, from 1, or 0 when none is.
    pub step: u32,
    /// How many steps a load takes.
    pub steps: u32,
    /// How long it has run, 0 when none is.
    pub elapsed_ms: i64,
    /// How long the last load that finished took, which the page shows as the time to expect; none
    /// before a load has finished since the server started.
    pub expected_ms: Option<i64>,
}

/// SPEC §11 R4965000: how the kept snapshot files the card totals (R3067000), by the records' patches
/// (`patch_windows`, R1973002). It keeps as filed the totals of every version from the first of the
/// window before the oldest patch anyone may read (`public_patch_count`, R4677000) through the newest:
/// the windows every card's figures at a public patch reach (R1367000: a card's own window, or the one
/// before it, never opens before the window every card shares before its own). Before them it keeps one
/// total per deck card's play version (R4675000) and key, filed under the play version's first version,
/// which the play versions' table sums as it summed each version's (R4675001), and drops the rest. The
/// build's own version, when it is no shipped one, is kept too.
pub fn snapshot_filing(
    history: &[PatchStep],
    records: &StatsTotals,
    plays: &[PlayVersion],
    current: &str,
) -> CardFiling {
    let windows = patch_windows(
        history,
        |version| records.games(version),
        |version| records.live_games(version),
    );
    let reach = (public_patch_count(&windows) + 1).min(windows.len());
    let versions: IndexSet<String> = history.iter().map(|step| step.version.clone()).collect();
    let kept_from = windows[..reach]
        .last()
        .and_then(|window| window.versions.first())
        .and_then(|first| versions.get_index_of(first))
        .unwrap_or(0);
    let mut folded: IndexMap<String, Vec<(usize, usize)>> = IndexMap::new();
    for play in plays {
        if let (Some(first), Some(last)) = (
            versions.get_index_of(play.first()),
            versions.get_index_of(play.last()),
        ) && first < kept_from
        {
            folded.entry(play.card.clone()).or_default().push((first, last));
        }
    }
    CardFiling {
        kept_to: versions.len().saturating_sub(1),
        also_kept: if versions.contains(current) {
            Vec::new()
        } else {
            vec![current.to_string()]
        },
        versions,
        kept_from,
        folded,
    }
}

/// SPEC §11 R4965000: the versions, as places in `versions`, whose card totals the figures at `at`
/// read (R1367000): from the first of the window every card shares before `at`'s own through `at`.
/// `None` for a version no window holds.
pub fn reach_of(windows: &[PatchWindow], versions: &IndexSet<String>, at: &str) -> Option<(usize, usize)> {
    let to = versions.get_index_of(at)?;
    let own = windows
        .iter()
        .position(|window| window.versions.iter().any(|version| version == at))?;
    let oldest = windows.get(own + 1).unwrap_or(&windows[own]);
    let from = versions.get_index_of(oldest.versions.first()?)?;
    Some((from, to))
}

/// R1451, amended by R807000: the seats of a record the public figures count. Development games
/// always count both seats (R378); live games count what `?pilot=` asks for, merged (both seats)
/// unless the query names a side.
fn counted_pilot(source: GameSource, live_pilot: PilotFilter) -> PilotFilter {
    match source {
        GameSource::Live => live_pilot,
        GameSource::Dev => PilotFilter::Unified,
    }
}

/// R807000: whether one seat's side counts: a development seat always does, a live seat does when
/// the requested pilot is merged or names its pilot.
fn seat_counted(source: GameSource, seat_pilot: Pilot, live_pilot: PilotFilter) -> bool {
    match source {
        GameSource::Dev => true,
        GameSource::Live => live_pilot == PilotFilter::Unified || seat_pilot.as_str() == live_pilot.as_str(),
    }
}

/// R1451, amended by R807000: the public report over `totals` for `source`: the live records'
/// report counting the requested pilot's seats (`counted_pilot`) and the development records'
/// counting both seats, summed card by card, in catalog-id order as `card_stats` lists them.
/// R377: `mode` scopes both parts to one match type, or every mode; R2777001: Constructed's two
/// modes are read apart and summed as the two sources are.
fn public_report(
    totals: &StatsTotals,
    source: SourceFilter,
    patch: &str,
    live_pilot: PilotFilter,
    mode: ModeFilter,
) -> CardStatsReport {
    let filter = |source: SourceFilter, pilot: PilotFilter, mode: Option<GameMode>| CardStatsFilter {
        source,
        mode,
        patch: Some(patch.to_string()),
        pilot,
    };
    let mut parts: Vec<CardStatsReport> = Vec::new();
    for one in mode.modes() {
        if matches!(source, SourceFilter::Live | SourceFilter::All) {
            parts.push(card_stats_of(
                totals,
                &filter(
                    SourceFilter::Live,
                    counted_pilot(GameSource::Live, live_pilot),
                    *one,
                ),
            ));
        }
        if matches!(source, SourceFilter::Dev | SourceFilter::All) {
            parts.push(card_stats_of(
                totals,
                &filter(
                    SourceFilter::Dev,
                    counted_pilot(GameSource::Dev, live_pilot),
                    *one,
                ),
            ));
        }
    }
    let mut by_card: IndexMap<String, CardStats> = IndexMap::new();
    let mut by_created: IndexMap<String, CreatedStats> = IndexMap::new();
    let (mut games, mut decks) = (0, 0);
    for part in parts {
        games += part.games;
        decks += part.decks;
        for stat in part.cards {
            match by_card.get_mut(&stat.card) {
                None => {
                    by_card.insert(stat.card.clone(), stat);
                }
                Some(held) => add_stats(held, &stat),
            }
        }
        // R2089000: the Created figures, summed the same way.
        for stat in part.created {
            match by_created.get_mut(&stat.card) {
                None => {
                    by_created.insert(stat.card.clone(), stat);
                }
                Some(held) => add_created(held, &stat),
            }
        }
    }
    let mut cards: Vec<CardStats> = by_card.into_values().collect();
    cards.sort_by(|a, b| a.card.cmp(&b.card));
    let mut created: Vec<CreatedStats> = by_created.into_values().collect();
    created.sort_by(|a, b| a.card.cmp(&b.card));
    let one_mode = match mode.modes() {
        [one] => *one,
        _ => None,
    };
    CardStatsReport {
        filter: filter(
            source,
            if source == SourceFilter::Dev {
                PilotFilter::Unified
            } else {
                live_pilot
            },
            one_mode,
        ),
        games,
        decks,
        cards,
        created,
        choices: Vec::new(),
    }
}

/// R978000: one patch's public figures for one `?source=` and `?pilot=`, read off its totals once.
struct PatchReport {
    /// The patch's live games, tutorials aside: R654's gate counts them.
    live_games: i64,
    report: CardStatsReport,
    /// Each card's drill-down over the same records, by card id, at most `CARD_STATS_CO_PLAYED_TOP`
    /// co-played cards each. Only the current patch's report for the default source has them: it is
    /// the one `get_card` reads them from. R4673001: one set per face.
    drills: IndexMap<StatsFace, IndexMap<String, CardDrill>>,
}

/// What a kept report was computed from: the patch, `?source=`, `?pilot=` and `?mode=`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ReportKey {
    patch: String,
    source: SourceParam,
    pilot: PublicStatsPilot,
    mode: PublicStatsMode,
}

/// A kept report, filled by the first request that asks for it; the others wait on it.
type KeptCell = Keep<Arc<PatchReport>>;

/// R3067000: every patch's totals, filled by the first request that needs them; the others wait on it.
type KeptTotalsCell = Keep<Arc<Snapshot>>;

/// SPEC §11 R5174000: which card totals a read of the kept snapshot needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reach {
    /// The versions kept as filed (R4965000) alone: what the cards, the game figures, the drill-down and
    /// the win-rate snapshot read.
    Kept,
    /// Those, and every deck card's play versions before them folded (R4965000): what the play versions'
    /// table (R4675001), and the Hall of Fame cut from it (R4681001), read.
    Folded,
}

/// R3067000, R5174000: the cells of every patch's totals made at one time.
struct TotalsCells {
    /// The cell every read takes first.
    kept: KeptTotalsCell,
    /// Whether `kept` folds the older play versions in: it does when the play versions' table asked first.
    kept_folds: bool,
    /// The cell that folds them in, made when the play versions' table asks after a read of `kept` that
    /// does not.
    folded: Option<KeptTotalsCell>,
}

/// R3067000: every patch's totals, and when their load finished (R5245000), which every kept window and
/// history cut from them is kept by.
struct KeptTotals {
    made_at: i64,
    held: Arc<Snapshot>,
}

/// R4965000: the kept snapshot: the totals, how their card totals were filed (`snapshot_filing`), and
/// the windows every card shares over their records (`patch_windows`).
struct Snapshot {
    totals: StatsTotals,
    filing: CardFiling,
    windows: Vec<PatchWindow>,
    /// R5245000: when its load finished, which it is kept `CARD_STATS_CACHE_TTL_SECONDS` from.
    loaded_at: i64,
}

impl Snapshot {
    fn new(history: &[PatchStep], totals: StatsTotals, filing: CardFiling, loaded_at: i64) -> Snapshot {
        let windows = patch_windows(
            history,
            |version| totals.games(version),
            |version| totals.live_games(version),
        );
        Snapshot {
            totals,
            filing,
            windows,
            loaded_at,
        }
    }

    /// R4965000: whether `version`'s card totals are held as filed.
    fn holds_patch(&self, version: &str) -> bool {
        self.filing.also_kept.iter().any(|kept| kept == version)
            || self
                .filing
                .versions
                .get_index_of(version)
                .is_some_and(|at| (self.filing.kept_from..=self.filing.kept_to).contains(&at))
    }

    /// R4965000: whether the figures at `at` can be cut from the snapshot: every version they read
    /// (`reach_of`) is held as filed. A version no window holds reads its own totals alone (R1367000).
    fn holds(&self, at: &str) -> bool {
        match reach_of(&self.windows, &self.filing.versions, at) {
            Some((from, to)) => from >= self.filing.kept_from && to <= self.filing.kept_to,
            None => self.holds_patch(at),
        }
    }

    /// R4965000: the card totals the figures at `at` read, a patch past the snapshot's (a dev account's,
    /// R4677000): every patch's records and game totals, and the card totals of `at`'s reach alone, so
    /// the load holds no more than that reach.
    fn reach_rows(&self, at: &str) -> CardRows {
        let (kept_from, kept_to) =
            reach_of(&self.windows, &self.filing.versions, at).unwrap_or((usize::MAX, usize::MAX));
        CardRows::Filed(CardFiling {
            versions: self.filing.versions.clone(),
            kept_from,
            kept_to,
            also_kept: Vec::new(),
            folded: IndexMap::new(),
        })
    }
}

/// R978000: the computed per-patch reports, each with when it was first asked for, the one kept
/// longest ago first. One per `App`, so a deploy or a test starts with none. The lock is never held
/// across a computation.
#[derive(Default)]
pub struct KeptReports {
    entries: Kept<ReportKey, Arc<PatchReport>>,
    /// R1367000: the windowed reports `GET /api/stats/cards` reads, kept as the per-patch ones are.
    windows: Kept<ReportKey, Arc<KeptWindow>>,
    /// R2491000: the card drill-downs' patch histories, one per `?pilot=` and `?mode=` (R4673001: and
    /// `?face=`).
    histories: Kept<(PublicStatsPilot, PublicStatsMode, StatsFace), Arc<KeptHistory>>,
    /// R4675001: the play versions' tables `GET /api/stats/versions` reads, one per `?pilot=` and `?mode=`
    /// (R5040000: and `?scope=`).
    versions: Kept<(PublicStatsPilot, PublicStatsMode, PublicStatsScope), Arc<PlayVersionTable>>,
    /// R4681001: the Hall of Fame's bodies `GET /api/stats/hall-of-fame` reads, one per `?mode=`.
    hall: Kept<PublicStatsMode, Arc<HallOfFameBody>>,
    /// R3067000: every patch's totals, which every kept window and history is cut from, with when they
    /// were first asked for. R5174000: beside them, the same totals with the older play versions folded in,
    /// when the play versions' table asked for those after the cards.
    totals: Mutex<Option<TotalsCells>>,
    /// R1583000: the play rates `GET /api/stats/play-rates` reads, with when they were made.
    play_rates: Kept<(), Arc<PlayRatesResponse>>,
    /// R5186000: what the kept snapshot's loads are doing, shared with each load's task.
    load: Arc<Mutex<SnapshotLoad>>,
}

/// Kept cells by what they were computed from, each with when it was made.
type Kept<K, T> = Mutex<IndexMap<K, (i64, Keep<T>)>>;

/// The cell for `key` in `entries` at `now`: the one made less than `ttl_seconds` before `now`, or one still
/// filling whatever its age (R5245000: so one fill runs at a time); else a new empty one that serves the
/// value it replaces while it fills (`kept_value`). One past its time by `CARD_STATS_KEPT_IDLE_SECONDS`
/// with no read to renew it is dropped, and the one kept longest ago to stay within `CARD_STATS_KEPT_REPORTS`.
fn kept_cell<K: Hash + Eq, T: Clone>(entries: &Kept<K, T>, key: K, now: i64, ttl_seconds: i64) -> Keep<T> {
    let mut entries = lock(entries);
    entries.retain(|_, (made_at, _)| {
        now - *made_at < (ttl_seconds + CARD_STATS_KEPT_IDLE_SECONDS) * MS_PER_SECOND
    });
    if let Some((made_at, keep)) = entries.get_mut(&key) {
        if keep.cell.get().is_none() {
            return keep.clone();
        }
        if now - *made_at < ttl_seconds * MS_PER_SECOND {
            keep.last = None;
            return keep.clone();
        }
    }
    let last = entries
        .shift_remove(&key)
        .and_then(|(_, keep)| keep.cell.get().cloned());
    while entries.len() >= CARD_STATS_KEPT_REPORTS {
        entries.shift_remove_index(0);
    }
    let keep = Keep::new(last);
    entries.insert(key, (now, keep.clone()));
    keep
}

/// R5245000: the value held for `key` in `entries`, whatever its time: its cell's, or the one that cell
/// replaced while it fills.
fn held<K: Hash + Eq, T: Clone>(entries: &Kept<K, T>, key: &K) -> Option<T> {
    lock(entries)
        .get(key)
        .and_then(|(_, keep)| keep.cell.get().cloned().or_else(|| keep.last.clone()))
}

impl KeptReports {
    fn held_window(&self, key: &ReportKey) -> Option<Arc<KeptWindow>> {
        held(&self.windows, key)
    }

    fn held_history(&self, key: &(PublicStatsPilot, PublicStatsMode, StatsFace)) -> Option<Arc<KeptHistory>> {
        held(&self.histories, key)
    }

    fn held_versions(
        &self,
        key: &(PublicStatsPilot, PublicStatsMode, PublicStatsScope),
    ) -> Option<Arc<PlayVersionTable>> {
        held(&self.versions, key)
    }

    fn held_hall(&self, key: &PublicStatsMode) -> Option<Arc<HallOfFameBody>> {
        held(&self.hall, key)
    }

    fn cell(&self, key: ReportKey, now: i64) -> KeptCell {
        kept_cell(&self.entries, key, now, CARD_STATS_CACHE_TTL_SECONDS)
    }

    fn window_cell(&self, key: ReportKey, now: i64) -> KeptWindowCell {
        kept_cell(&self.windows, key, now, CARD_STATS_CACHE_TTL_SECONDS)
    }

    fn history_cell(&self, key: (PublicStatsPilot, PublicStatsMode, StatsFace), now: i64) -> KeptHistoryCell {
        kept_cell(&self.histories, key, now, CARD_STATS_CACHE_TTL_SECONDS)
    }

    fn versions_cell(
        &self,
        key: (PublicStatsPilot, PublicStatsMode, PublicStatsScope),
        now: i64,
    ) -> KeptVersionsCell {
        kept_cell(&self.versions, key, now, CARD_STATS_CACHE_TTL_SECONDS)
    }

    fn hall_cell(&self, key: PublicStatsMode, now: i64) -> KeptHallCell {
        kept_cell(&self.hall, key, now, CARD_STATS_CACHE_TTL_SECONDS)
    }

    /// R5186000: what the kept snapshot's load is doing at `now`.
    pub fn loading(&self, now: i64) -> StatsLoading {
        let state = lock(&self.load);
        let loading = state.running > 0;
        StatsLoading {
            loading,
            step: if loading { state.step } else { 0 },
            steps: LOAD_STEPS,
            elapsed_ms: if loading {
                (now - state.started_at).max(0)
            } else {
                0
            },
            expected_ms: state.last_ms,
        }
    }

    /// R3067000: the cell of every patch's totals, and whether it folds the older play versions in
    /// (R5174000). A read that needs them takes a cell that does: the one there when it folds, or one made
    /// beside it, so everything cut from either is kept and dropped together. R5245000: the cells are made
    /// again once the held snapshot's load finished `CARD_STATS_CACHE_TTL_SECONDS` before `now`, never while
    /// it loads; a folding load under way is joined.
    fn totals_cell(&self, now: i64, reach: Reach) -> (KeptTotalsCell, bool) {
        let mut kept = lock(&self.totals);
        let renew = kept.as_ref().is_none_or(|cells| {
            cells
                .kept
                .cell
                .get()
                .is_some_and(|held| now - held.loaded_at >= CARD_STATS_CACHE_TTL_SECONDS * MS_PER_SECOND)
        });
        if renew {
            // R5245000: the snapshots past their time are let go here, so one is held at a time; a folding load
            // still under way is kept, to be joined.
            let folds = reach == Reach::Folded;
            let folded = kept
                .take()
                .and_then(|cells| cells.folded)
                .filter(|folded| !folds && folded.cell.get().is_none());
            *kept = Some(TotalsCells {
                kept: Keep::new(None),
                kept_folds: folds,
                folded,
            });
        }
        let cells = kept.as_mut().expect("the cells were just made");
        if reach == Reach::Kept || cells.kept_folds {
            return (cells.kept.clone(), cells.kept_folds);
        }
        let folded = cells.folded.get_or_insert_with(|| Keep::new(None));
        (folded.clone(), true)
    }

    /// R1583000: the play rates' cell made less than `PLAY_RATES_REFRESH_SECONDS` before `now`, or a
    /// new empty one in its place (R5245000: serving the rates it replaces while it fills).
    fn play_rates_cell(&self, now: i64) -> PlayRatesCell {
        kept_cell(&self.play_rates, (), now, PLAY_RATES_REFRESH_SECONDS)
    }

    /// R5274000: a dev account's Fetch live. Every figure kept is let go, and the snapshot with them, so the
    /// next read loads the totals again and waits for them, as the first read after a boot does. A load under
    /// way is kept, to be joined, and so is a folding load (R5174000), so no second load starts beside one.
    /// The play rates are kept: they refresh on their own (`PLAY_RATES_REFRESH_SECONDS`).
    pub fn let_go(&self) {
        lock(&self.entries).clear();
        lock(&self.windows).clear();
        lock(&self.histories).clear();
        lock(&self.versions).clear();
        lock(&self.hall).clear();
        let mut totals = lock(&self.totals);
        if totals
            .as_ref()
            .is_some_and(|cells| cells.kept.cell.get().is_some())
        {
            let folding = totals
                .take()
                .and_then(|cells| cells.folded)
                .filter(|folded| folded.cell.get().is_none());
            *totals = folding.map(|folded| TotalsCells {
                kept: Keep::new(None),
                kept_folds: false,
                folded: Some(folded),
            });
        }
    }
}

/// R1367000: windowed figures for a patch (R1973001, R1973002, R1994000).
struct KeptWindow {
    report: WindowedReport,
    games: WindowedGames,
    /// R4552001: every turn of the counted records, which an ordinary turn's gain is read off.
    swing_rows: SwingRows,
    live_games: IndexMap<String, i64>,
    history: Vec<PatchStep>,
    patches: Vec<PatchWindow>,
}

impl KeptWindow {
    /// The window of a card, held or not.
    fn window_of(&self, id: &str, request: WindowRequest) -> CardWindow {
        match self.report.card(id) {
            Some(card) => card.window.clone(),
            None => card_window(
                &self.history,
                &self.report.at,
                Some(card_family(id)),
                request,
                |version| self.live_games.get(version).copied().unwrap_or(0),
            ),
        }
    }
}

type KeptWindowCell = Keep<Arc<KeptWindow>>;

/// R1367000: the windowed figures at `at` over `totals`: the report and game figures R1367000 and
/// R1994000 read, each patch's live games, and the patch lists R1973001 and R1973002 read off the
/// totals' games. Pure, so a kept window is cut from the snapshot as its own load was.
fn window_over(
    totals: &StatsTotals,
    history: &[PatchStep],
    at: &str,
    source: SourceParam,
    pilot: PublicStatsPilot,
    mode: PublicStatsMode,
) -> KeptWindow {
    let report = windowed_card_stats(
        totals,
        &WindowQuery {
            history,
            at,
            live_pilot: pilot.filter(),
            mode: mode.filter(),
            request: source.request(),
        },
    );
    let figures = windowed_game_figures(totals, history, at, mode.filter(), source.request());
    let swing_rows = windowed_swing_rows(totals, history, at, mode.filter(), source.request());
    let mut live_games: IndexMap<String, i64> = IndexMap::new();
    for key in totals.records.keys() {
        live_games
            .entry(key.patch.clone())
            .or_insert_with(|| totals.live_games(&key.patch));
    }
    let games = |version: &str| totals.games(version);
    KeptWindow {
        report,
        games: figures,
        swing_rows,
        live_games,
        history: collapse_gameless(history, games),
        patches: patch_windows(history, games, |version| totals.live_games(version)),
    }
}

/// R3067000: the window of a patch the server ships, kept per `ReportKey` as long as the snapshot it is
/// cut from: its cell carries the snapshot's `made_at`, so no figure is older than
/// `CARD_STATS_CACHE_TTL_SECONDS`.
async fn shipped_window(
    app: &App,
    history: &[PatchStep],
    at: &str,
    source: SourceParam,
    pilot: PublicStatsPilot,
    mode: PublicStatsMode,
) -> Result<Arc<KeptWindow>, ApiError> {
    let key = ReportKey {
        patch: at.to_string(),
        source,
        pilot,
        mode,
    };
    let snapshot = match kept_snapshot(app, Reach::Kept, || app.card_stats.held_window(&key)).await? {
        CutFrom::Snapshot(snapshot) => snapshot,
        CutFrom::Held(window) => return Ok(window),
    };
    let keep = app.card_stats.window_cell(key, snapshot.made_at);
    // R5225000: the cut runs on a task of its own, as the snapshot loads (R5186000), so a read that gives up
    // leaves it to finish for the next.
    let cut = WindowCut {
        db: app.db.clone(),
        held: Arc::clone(&snapshot.held),
        history: history.to_vec(),
        at: at.to_string(),
        source,
        pilot,
        mode,
        load: Arc::clone(&app.card_stats.load),
    };
    kept_value(keep, async move { cut.window().await }).await
}

/// R1367000: the windowed report at `at`, as kept (`CARD_STATS_CACHE_TTL_SECONDS`): cut from every
/// patch's totals (`kept_snapshot`), loaded once however many keys miss. R978000, amended by R3067000:
/// a patch the server does not ship is never kept, and its window is that patch alone (the engine's
/// `resolve`), so it reads that patch's totals only; the patch lists come from the kept window of the
/// current patch.
async fn kept_window(
    app: &App,
    history: &[PatchStep],
    at: &str,
    source: SourceParam,
    pilot: PublicStatsPilot,
    mode: PublicStatsMode,
) -> Result<Arc<KeptWindow>, ApiError> {
    let known = at == app.catalog.version || history.iter().any(|step| step.version == at);
    if known {
        return shipped_window(app, history, at, source, pilot, mode).await;
    }
    let shared = shipped_window(
        app,
        history,
        &app.catalog.version,
        SourceParam::Default,
        PublicStatsPilot::Merged,
        PublicStatsMode::All,
    )
    .await?;
    let own = totals_for(app, Some(at), false, CardRows::All).await?;
    let mut window = blocking(|| window_over(&own, history, at, source, pilot, mode));
    window.history.clone_from(&shared.history);
    window.patches.clone_from(&shared.patches);
    Ok(Arc::new(window))
}

/// R3049000: the win-rate snapshot a match starts with: each card's in-deck wins and games over its
/// own window at this build's version, the figures `/api/stats/cards` shows by default (every seat,
/// every mode, R654's gate), read off the same kept report (`CARD_STATS_CACHE_TTL_SECONDS`). A store
/// that cannot answer leaves the empty table, which names no card, and says so in the log.
pub async fn win_rate_snapshot(app: &App) -> WinRateTable {
    let history = patch_history();
    match kept_window(
        app,
        &history,
        &app.catalog.version,
        SourceParam::Default,
        PublicStatsPilot::Merged,
        PublicStatsMode::All,
    )
    .await
    {
        Ok(kept) => win_rates_from_report(&kept.report),
        Err(error) => {
            tracing::warn!(event = "stats.win_rates.unavailable", message = %error.message);
            WinRateTable::default()
        }
    }
}

/// R978000, R654: computes patch report for requested filters and optional drill-downs.
/// Tutorial games are excluded (R654).
fn patch_report(
    totals: &StatsTotals,
    patch: &str,
    source_param: SourceParam,
    live_pilot: PilotFilter,
    mode: ModeFilter,
    with_drills: bool,
) -> PatchReport {
    let live_games = totals.live_games(patch);

    let cleared = live_games >= PUBLIC_STATS_MIN_LIVE_GAMES;

    let source = if cleared || source_param == SourceParam::Live {
        // When cleared, strictly ignore AI development games (no toggle to bring them back)
        PublicStatsSource::Live
    } else if source_param == SourceParam::Dev {
        PublicStatsSource::Dev
    } else {
        PublicStatsSource::Provisional
    };
    let counted = match source {
        PublicStatsSource::Live | PublicStatsSource::Previous => SourceFilter::Live,
        PublicStatsSource::Dev => SourceFilter::Dev,
        PublicStatsSource::Provisional => SourceFilter::All,
    };

    let report = public_report(totals, counted, patch, live_pilot, mode);
    let drills = if with_drills {
        let sources = sources_of(counted);
        let counts = |key: &TotalsKey| {
            key.patch == patch
                && sources.contains(&key.source)
                && mode.counts(key.mode)
                && seat_counted(key.source, key.pilot, live_pilot)
        };
        // R4673001: each face's drills, over the same keys.
        StatsFace::ALL
            .iter()
            .map(|&face| {
                (
                    face,
                    face_drills_of(totals, counts, CARD_STATS_CO_PLAYED_TOP, face),
                )
            })
            .collect()
    } else {
        IndexMap::new()
    };
    PatchReport {
        live_games,
        report,
        drills,
    }
}

/// R978000: cached report lookup or computation (`CARD_STATS_CACHE_TTL_SECONDS`).
/// Unknown patches are computed on-demand without caching.
async fn kept_report(
    app: &App,
    versions: &[String],
    patch: &str,
    source: SourceParam,
    pilot: PublicStatsPilot,
    mode: PublicStatsMode,
) -> Result<Arc<PatchReport>, ApiError> {
    let current = patch == app.catalog.version;
    let with_drills = current && source == SourceParam::Default;
    let compute = {
        let db = app.db.clone();
        let patch = patch.to_string();
        async move {
            let totals = totals_in(&db, Some(&patch), with_drills, CardRows::All).await?;
            Ok::<_, ApiError>(Arc::new(patch_report(
                &totals,
                &patch,
                source,
                pilot.filter(),
                mode.filter(),
                with_drills,
            )))
        }
    };
    if !current && !versions.iter().any(|version| version == patch) {
        return compute.await;
    }
    let key = ReportKey {
        patch: patch.to_string(),
        source,
        pilot,
        mode,
    };
    kept_value(app.card_stats.cell(key, now_ms()), compute).await
}

/// R2491000: one `?pilot=` and `?mode=`'s patch history, which every card's drill-down reads: each
/// shipped patch whose live games clear R654's gate, in `patches.json`'s order, with each card's
/// in-deck tally there as `patch_report` counts it (R2777002: under All Random, its seen tally;
/// R4673001: on a face, the seats that played it on that face).
struct KeptHistory {
    patches: Vec<(String, IndexMap<String, Tally>)>,
}

type KeptHistoryCell = Keep<Arc<KeptHistory>>;

/// R2491000: the patch history of `versions` for `pilot` and `mode` over `totals`: each version cut
/// from them by `patch_report` as its own load was (R978000), so the figures are the same.
fn history_over(
    totals: &StatsTotals,
    versions: &[String],
    pilot: PublicStatsPilot,
    mode: PublicStatsMode,
    face: StatsFace,
) -> KeptHistory {
    let patches = versions
        .iter()
        .filter_map(|version| {
            let kept = patch_report(
                totals,
                version,
                SourceParam::Default,
                pilot.filter(),
                mode.filter(),
                false,
            );
            (kept.live_games >= PUBLIC_STATS_MIN_LIVE_GAMES).then(|| {
                // R2777002: under All Random, the decks that saw the card.
                let cards: IndexMap<String, Tally> = kept
                    .report
                    .cards
                    .into_iter()
                    .map(|stat| {
                        let counted = mode.counted_on(&stat, face);
                        (stat.card, counted)
                    })
                    .collect();
                (version.clone(), cards)
            })
        })
        .collect();
    KeptHistory { patches }
}

/// R2491000: the patch history of `versions` for `pilot` and `mode`, as kept: read off every
/// patch's totals (R3067000: `kept_snapshot`, one load however many histories and windows miss it),
/// kept as long as that snapshot, once however many requests miss it together.
async fn kept_history(
    app: &App,
    versions: &[String],
    pilot: PublicStatsPilot,
    mode: PublicStatsMode,
    face: StatsFace,
) -> Result<Arc<KeptHistory>, ApiError> {
    let key = (pilot, mode, face);
    let snapshot = match kept_snapshot(app, Reach::Kept, || app.card_stats.held_history(&key)).await? {
        CutFrom::Snapshot(snapshot) => snapshot,
        CutFrom::Held(history) => return Ok(history),
    };
    let keep = app.card_stats.history_cell(key, snapshot.made_at);
    let versions = versions.to_vec();
    kept_value(keep, async move {
        // R4965000: the patches whose card totals the snapshot holds as filed.
        let held: Vec<String> = versions
            .into_iter()
            .filter(|version| snapshot.held.holds_patch(version))
            .collect();
        Ok::<_, ApiError>(Arc::new(history_over(
            &snapshot.held.totals,
            &held,
            pilot,
            mode,
            face,
        )))
    })
    .await
}

type KeptVersionsCell = Keep<Arc<PlayVersionTable>>;

/// R4675001: every deck card's play versions' figures for `pilot` and `mode`, as kept: read off every
/// patch's totals (R3067000: `kept_snapshot`; R5174000: with the older play versions folded in, the one
/// read that needs them, made beside the cards' with its time, or shared with them when it came first), kept
/// as long as that snapshot, once however many requests miss it together. R5040000: under
/// `PublicStatsScope::Major`, the live ones alone, each cut to the newest major's line, read against
/// their own field. R4965000's snapshot keeps the whole newest line as filed (`public_patch_count`
/// reaches every patch of it), so a cut span reads its own versions' totals.
async fn kept_versions(
    app: &App,
    pilot: PublicStatsPilot,
    mode: PublicStatsMode,
    scope: PublicStatsScope,
) -> Result<Arc<PlayVersionTable>, ApiError> {
    let key = (pilot, mode, scope);
    let snapshot = match kept_snapshot(app, Reach::Folded, || app.card_stats.held_versions(&key)).await? {
        CutFrom::Snapshot(snapshot) => snapshot,
        CutFrom::Held(table) => return Ok(table),
    };
    let keep = app.card_stats.versions_cell(key, snapshot.made_at);
    kept_value(keep, async move {
        let every = deck_play_versions();
        let history = patch_history();
        let versions = match scope {
            PublicStatsScope::All => every,
            PublicStatsScope::Major => major_line_versions(&every, &history),
        };
        Ok::<_, ApiError>(Arc::new(play_version_figures(
            &snapshot.held.totals,
            &PlayVersionQuery {
                versions: &versions,
                history: &history,
                live_pilot: pilot.filter(),
                mode: mode.filter(),
                min_sample: CARD_STATS_MIN_SAMPLE,
            },
        )))
    })
    .await
}

/// R4675001: `GET /api/stats/versions`' body: the query it answers and the table, figures only (R661: no
/// source is named).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicStatsVersionsResponse<'a> {
    pilot: PublicStatsPilot,
    mode: PublicStatsMode,
    #[serde(flatten)]
    table: &'a PlayVersionTable,
}

/// GET /api/stats/versions (R4675001): every play version of every deck card (R4675000) with its figures,
/// the live ones marked, over `?pilot=` (R807000) and `?mode=` (R377), read against one field.
/// R5040000: `?scope=major` sends the live ones cut to the newest major's line; the body never names it.
pub async fn get_versions(app: &Arc<App>, req: Req) -> ApiResult {
    let pilot = PublicStatsPilot::parse(req.query.get("pilot").map(String::as_str));
    let mode = PublicStatsMode::parse(req.query.get("mode").map(String::as_str));
    let scope = PublicStatsScope::parse(req.query.get("scope").map(String::as_str));
    let table = kept_versions(app, pilot, mode, scope).await?;
    let response = PublicStatsVersionsResponse {
        pilot,
        mode,
        table: &table,
    };
    Ok(cached_ok(to_json(&response)?, CARD_STATS_MAX_AGE_SECONDS))
}

/// R4681001: one Hall of Fame entry: its before and after play-version rows with figures, their ship
/// dates, the patch's player-facing balance line, and, on the changed list alone, how many changes
/// lie between.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicHallEntry {
    pub before: PlayVersionFigures,
    pub after: PlayVersionFigures,
    pub before_date: Option<String>,
    pub after_date: Option<String>,
    pub balance: Option<String>,
    pub changes_between: Option<usize>,
}

/// R4681001: `GET /api/stats/hall-of-fame`'s body: the query it answers and the four lists.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HallOfFameBody {
    pub pilot: PublicStatsPilot,
    pub mode: PublicStatsMode,
    pub min_sample: i64,
    pub band_low: f64,
    pub band_high: f64,
    pub broken: Vec<PublicHallEntry>,
    pub nerfed: Vec<PublicHallEntry>,
    pub buffed: Vec<PublicHallEntry>,
    pub changed: Vec<PublicHallEntry>,
}

type KeptHallCell = Keep<Arc<HallOfFameBody>>;

/// R4681001: the Hall of Fame's body over `table`: R4681000's lists, each entry joined to its rows,
/// dates from `patches.json`, and the balance line matched per R4681001.
fn hall_body_of(table: &PlayVersionTable, mode: PublicStatsMode) -> HallOfFameBody {
    let dates = patch_ship_dates();
    let mut firsts: IndexMap<String, String> = IndexMap::new();
    for row in &table.versions {
        firsts
            .entry(row.card.clone())
            .or_insert_with(|| row.first.clone());
    }
    let hall = hall_of_fame(table, HALL_OF_FAME_LIST_LEN, &change_totals_since(&firsts));
    let by_id: IndexMap<&str, &PlayVersionFigures> =
        table.versions.iter().map(|row| (row.id.as_str(), row)).collect();
    let entry = |pair: &HallPair, changes_between: Option<usize>| {
        let before = by_id[pair.before.as_str()].clone();
        let after = by_id[pair.after.as_str()].clone();
        // The first `player.balance` line of the after patch naming the card; a line that names no
        // card is no entry's line rather than a wrong one, and none where before equals after.
        let balance = if pair.before == pair.after {
            None
        } else {
            let wanted = after.name.to_lowercase();
            if wanted.is_empty() {
                None
            } else {
                patch_balance_lines(&after.first)
                    .into_iter()
                    .find(|line| line.to_lowercase().contains(&wanted))
            }
        };
        PublicHallEntry {
            before_date: dates.get(&before.first).cloned(),
            after_date: dates.get(&after.first).cloned(),
            before,
            after,
            balance,
            changes_between,
        }
    };
    HallOfFameBody {
        pilot: PublicStatsPilot::Merged,
        mode,
        min_sample: table.min_sample,
        band_low: table.band_low,
        band_high: table.band_high,
        broken: hall.broken.iter().map(|pair| entry(pair, None)).collect(),
        nerfed: hall.nerfed.iter().map(|pair| entry(pair, None)).collect(),
        buffed: hall.buffed.iter().map(|pair| entry(pair, None)).collect(),
        changed: hall
            .changed
            .iter()
            .map(|(pair, lines)| entry(pair, Some(*lines)))
            .collect(),
    }
}

/// R4681001: one `?mode=`'s Hall of Fame body, as kept: R4681000's lists over the kept versions'
/// table (`kept_versions`, over the snapshot that folds the older play versions in, R5174000), kept as
/// long as that snapshot, once however many requests miss it together.
async fn kept_hall(app: &App, mode: PublicStatsMode) -> Result<Arc<HallOfFameBody>, ApiError> {
    let snapshot = match kept_snapshot(app, Reach::Folded, || app.card_stats.held_hall(&mode)).await? {
        CutFrom::Snapshot(snapshot) => snapshot,
        CutFrom::Held(body) => return Ok(body),
    };
    let table = kept_versions(app, PublicStatsPilot::Merged, mode, PublicStatsScope::All).await?;
    let keep = app.card_stats.hall_cell(mode, snapshot.made_at);
    kept_value(keep, async move {
        Ok::<_, ApiError>(Arc::new(hall_body_of(&table, mode)))
    })
    .await
}

/// GET /api/stats/hall-of-fame (R4681001): the Hall of Fame's four lists (R4681000) over `?mode=`
/// (R377), open to everyone and cached as the versions are. The merged pilots, as the tab's versions
/// fetch: only `?mode=` is read.
pub async fn get_hall_of_fame(app: &Arc<App>, req: Req) -> ApiResult {
    let mode = PublicStatsMode::parse(req.query.get("mode").map(String::as_str));
    let body = kept_hall(app, mode).await?;
    Ok(cached_ok(to_json(&body)?, CARD_STATS_MAX_AGE_SECONDS))
}

/// GET /api/stats/cards: public card win rates (R654, R1367000).
/// Supports pilot filtering (R1451, R807000, R1452) and match modes (R377).
/// R2777001: `?mode=constructed` reads Best of 1 and Conquest together. R2777002: under `random` a card's
/// games, rate and floor read the decks that saw it. R3950000: its play rate is its played games over its
/// seen ones in every mode. R2777000: each card carries its adjusted win rate, and the cards are listed by it.
/// R4673000: `?face=normal|radiant` reads every figure but the play rate on that face, against its own field
/// and band.
pub async fn get_cards(app: &Arc<App>, req: Req) -> ApiResult {
    let versions = load_patch_versions().await;
    let current_patch = app.catalog.version.clone();
    let requested_patch = query_trimmed(&req, "patch").unwrap_or(current_patch);
    let source_param = SourceParam::parse(req.query.get("source").map(String::as_str));
    let pilot = PublicStatsPilot::parse(req.query.get("pilot").map(String::as_str));
    let mode = PublicStatsMode::parse(req.query.get("mode").map(String::as_str));
    let face = face_of(&req);
    let patch_index = versions.iter().position(|version| *version == requested_patch);
    let previous_patch = match patch_index {
        Some(index) if index > 0 => versions.get(index - 1).cloned(),
        _ => None,
    };

    // R4677000: the caller's patches; one past them is refused before its figures are read, and the
    // patch before stops at them.
    let history = patch_history();
    let reader = reader_of(app, &req).await?;
    let listed = listed_patches(app, &history).await?;
    let readable = readable_count(&listed.patches, reader);
    refuse_past_readable(&listed.patches, readable, &requested_patch)?;
    let previous_patch = previous_patch.filter(|version| !past_readable(&listed.patches, readable, version));

    // R978000, R1367000: every card's figures over its own window at the patch, as kept.
    let request = source_param.request();
    let kept = kept_window(app, &history, &requested_patch, source_param, pilot, mode).await?;
    let report = &kept.report;
    let live_games_count = report.patch.live_games;
    let cleared = report.patch.cleared();
    let source = PublicStatsSource::of(report.patch.source);

    let set_param = req
        .query
        .get("set")
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty());
    let rarity_param = req
        .query
        .get("rarity")
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty());
    let cost_param = req.query.get("cost").cloned();
    let card_param = query_trimmed(&req, "card");
    let shown = |id: &str, def: &CardDef| {
        if card_param.as_ref().is_some_and(|card| id != card) {
            return false;
        }
        // A set is its cards' `set` alone: `classic` begins every Classic+ id (`classicplus-…`).
        if let Some(set) = &set_param
            && def.set.as_str().to_lowercase() != *set
        {
            return false;
        }
        if let Some(rarity) = &rarity_param
            && def.rarity.as_str().to_lowercase() != *rarity
        {
            return false;
        }
        let cost = numeric_cost(def);
        if let Some(cost_text) = cost_param.as_deref().filter(|text| !text.is_empty())
            && let Some(parsed_cost) =
                Some(js_number(&cost_text.replacen('+', "", 1))).filter(|number| !number.is_nan())
        {
            if parsed_cost >= CARD_STATS_CURVE_TOP as f64 {
                if (cost as f64) < CARD_STATS_CURVE_TOP as f64 {
                    return false;
                }
            } else if cost as f64 != parsed_cost {
                return false;
            }
        }
        true
    };

    // R3022000, R4082000: the fields every card's adjusted rate is read against, whatever the filters show;
    // R4673000: the face's own.
    let fields = face_fields(app, report, face);
    // R4552001, R4552002: every card's Decisiveness, read off the same cards as All's two fields, beside
    // the turns of the records the window counts. R4673000: All's on every face, as the play rate is: a
    // turn row names no face.
    let empty = SwingTally::default();
    let swing = swing_report(
        app.catalog
            .defs
            .iter()
            .filter(|(_, def)| !def.token)
            .filter_map(|(id, _)| report.card(id))
            .map(|card| (card.stats.card.as_str(), card.stats.swing.or(&empty))),
        report
            .created
            .iter()
            .filter(|row| app.catalog.defs.contains_key(&row.stats.card))
            .map(|row| (row.stats.card.as_str(), &row.stats.swing)),
        &swing_baseline(&kept.swing_rows),
    );

    let mut cards: Vec<PublicCardStat> = Vec::new();
    for (id, def) in &app.catalog.defs {
        if def.token {
            continue; // Only non-token deckable cards
        }
        if !shown(id, def) {
            continue;
        }
        let cost = numeric_cost(def);

        let stat = report.card(id).map(|card| &card.stats);
        let window = kept.window_of(id, request);
        let figures = card_figures(report, id, def, mode, face, &fields);
        // R3950000: the play rate, All's on every face (R4673000).
        let made = made_by_a_rule(def)
            .then(|| made_games(report, id, StatsFace::All))
            .flatten();
        let play_rate = play_rate_of(report, id, stat, made.as_ref());
        // R4552002: a card a rule makes reads its Created copies' turns, as its adjusted rate does.
        let swung = if made.is_some() {
            swing.created.get(id.as_str())
        } else {
            swing.cards.get(id.as_str())
        };
        let decisiveness = swung.and_then(|figures| figures.decisiveness);

        cards.push(PublicCardStat {
            id: id.clone(),
            name: def.name.clone(),
            cost,
            rarity: def.rarity.as_str().to_string(),
            set: def.set.as_str().to_string(),
            games: figures.games,
            win_rate: figures.win_rate,
            adjusted_win_rate: figures.adjusted.map(|(rate, _)| rate),
            win_rate_low: figures.win_range.map(|(low, _)| low),
            win_rate_high: figures.win_range.map(|(_, high)| high),
            adjusted_low: figures.adjusted.map(|(_, (low, _))| low),
            adjusted_high: figures.adjusted.map(|(_, (_, high))| high),
            decisiveness,
            tier: figures.tier,
            play_rate,
            has_enough_games: figures.has_enough_games,
            window: PublicCardWindow::of(&window),
        });
    }

    // Enough games first, then the higher rate, the more games and the name. R2777000: the cards are
    // listed by their adjusted win rate; the summary's best and worst card stay R654's raw rate's.
    let by = |rate: fn(&PublicCardStat) -> Option<f64>| {
        move |a: &&PublicCardStat, b: &&PublicCardStat| {
            if a.has_enough_games && !b.has_enough_games {
                return Ordering::Less;
            }
            if !a.has_enough_games && b.has_enough_games {
                return Ordering::Greater;
            }
            match (rate(a), rate(b)) {
                (Some(a_rate), Some(b_rate)) if a_rate != b_rate => {
                    return b_rate.partial_cmp(&a_rate).unwrap_or(Ordering::Equal);
                }
                (Some(_), None) => return Ordering::Less,
                (None, Some(_)) => return Ordering::Greater,
                _ => {}
            }
            b.games
                .cmp(&a.games)
                .then_with(|| locale_compare(&a.name, &b.name))
        }
    };
    let mut qualifying: Vec<&PublicCardStat> = cards
        .iter()
        .filter(|card| card.has_enough_games && card.win_rate.is_some())
        .collect();
    qualifying.sort_by(by(|card| card.win_rate));
    let extreme = |card: &PublicCardStat| PublicStatsExtreme {
        id: card.id.clone(),
        name: card.name.clone(),
        win_rate: card.win_rate.unwrap_or(0.0),
        games: card.games,
    };
    let best_card = qualifying.first().map(|card| extreme(card));
    let worst_card = qualifying.last().map(|card| extreme(card));
    let by_adjusted = by(|card| card.adjusted_win_rate);
    cards.sort_by(|a, b| by_adjusted(&a, &b));

    // R2089000: every Created card the catalog holds, tokens among them, with enough games first.
    let mut created: Vec<PublicCreatedStat> = report
        .created
        .iter()
        .filter_map(|row| {
            let def = app.catalog.defs.get(&row.stats.card)?;
            shown(&row.stats.card, def)
                .then(|| PublicCreatedStat::of(def, row, face, &fields.created, &fields.all, &swing))
        })
        .collect();
    created.sort_by(|a, b| {
        b.has_enough_games
            .cmp(&a.has_enough_games)
            .then_with(|| b.games.cmp(&a.games))
            .then_with(|| locale_compare(&a.name, &b.name))
    });

    let response = PublicStatsCardsResponse {
        patch: requested_patch.clone(),
        previous_patch,
        gate: PublicStatsGate {
            cleared,
            live_games: live_games_count,
            min_live_games: PUBLIC_STATS_MIN_LIVE_GAMES,
        },
        source,
        source_label: source.label().to_string(),
        pilot,
        pilot_label: pilot.label().to_string(),
        mode,
        mode_label: mode.label().to_string(),
        min_sample: CARD_STATS_MIN_SAMPLE,
        total_games: report.games,
        cards,
        created,
        tiers: PublicStatsTiers {
            center: fields.deck.center,
            spread: fields.deck.spread,
            cards: fields.deck.cards,
            baseline: fields.deck.baseline,
            band_low: face.band().low,
            band_high: face.band().high,
            decisiveness: swing.deck_field.typical,
            created: PublicCreatedTiers {
                center: fields.created.center,
                spread: fields.created.spread,
                cards: fields.created.cards,
                baseline: fields.created.baseline,
                decisiveness: swing.created_field.typical,
            },
        },
        patches: listed.patches[..readable].to_vec(),
        older_patches_for_devs: reader == Reader::Anyone,
        summary: PublicStatsSummary {
            total_games: report.games,
            live_games: live_games_count,
            active_patch: requested_patch,
            source,
            pilot,
            mode,
            best_card,
            worst_card,
        },
    };

    Ok(reader_ok(to_json(&response)?, reader))
}

/// GET /api/stats/games (R1994000): game figures across windowed reports (R654, `PUBLIC_STATS_MIN_LIVE_GAMES`).
/// Pairs breakdown separates ladder bot games (R1451).
pub async fn get_games(app: &Arc<App>, req: Req) -> ApiResult {
    let patch = query_trimmed(&req, "patch").unwrap_or_else(|| app.catalog.version.clone());
    let source_param = SourceParam::parse(req.query.get("source").map(String::as_str));
    let mode = PublicStatsMode::parse(req.query.get("mode").map(String::as_str));
    let history = patch_history();
    // R4677000: a patch past the caller's is refused before its figures are read.
    let reader = reader_of(app, &req).await?;
    let listed = listed_patches(app, &history).await?;
    refuse_past_readable(&listed.patches, readable_count(&listed.patches, reader), &patch)?;
    let kept = kept_window(
        app,
        &history,
        &patch,
        source_param,
        PublicStatsPilot::Merged,
        mode,
    )
    .await?;
    let games = &kept.games;
    let source = PublicStatsSource::of(games.window.source);
    let response = PublicStatsGamesResponse {
        patch,
        source,
        source_label: source.label().to_string(),
        mode,
        mode_label: mode.label().to_string(),
        min_sample: CARD_STATS_MIN_SAMPLE,
        total_games: games.records,
        figures: games.figures.clone(),
        pairs: games.pairs.clone(),
    };
    Ok(reader_ok(to_json(&response)?, reader))
}

/// GET /api/stats/cards/:id: win rate by cleared patch and by turn played, and co-played cards. R4673001: on
/// `?face=`, of the seats that played the card on that face; and on every face the card's Normal and Radiant
/// figures and their gap at the page's `?patch=` (the current patch when missing).
pub async fn get_card(app: &Arc<App>, req: Req) -> ApiResult {
    let card_id = req.params.get("id").cloned().unwrap_or_default();
    let def = if card_id.is_empty() {
        None
    } else {
        app.catalog.defs.get(&card_id)
    };
    let Some(def) = def else {
        return Err(ApiError::new(
            ApiErrorCode::NotFound,
            format!("Card {card_id} not found"),
        ));
    };

    let pilot = PublicStatsPilot::parse(req.query.get("pilot").map(String::as_str));
    let mode = PublicStatsMode::parse(req.query.get("mode").map(String::as_str));
    let face = face_of(&req);
    let steps = patch_history();
    let patch = query_trimmed(&req, "patch").unwrap_or_else(|| app.catalog.version.clone());

    // R4677000: the history lists only the caller's patches, so the limit cannot be read around here, and
    // R4673001: the face line's patch past them is refused before any figure is read, as the table's is.
    let reader = reader_of(app, &req).await?;
    let listed = listed_patches(app, &steps).await?;
    let readable = readable_count(&listed.patches, reader);
    refuse_past_readable(&listed.patches, readable, &patch)?;

    // R2491000: every shipped patch's history reads one kept history, and R978000: the current
    // patch's turn and co-played figures read its kept report.
    let versions = load_patch_versions().await;
    let history = kept_history(app, &versions, pilot, mode, face).await?;
    let patch_history: Vec<PatchRate> = history
        .patches
        .iter()
        .filter(|(patch, _)| !past_readable(&listed.patches, readable, patch))
        .map(|(patch, cards)| {
            let tally = cards.get(&card_id);
            PatchRate {
                patch: patch.clone(),
                games: tally.map(|tally| tally_of(tally).0).unwrap_or(0),
                win_rate: tally.and_then(win_rate),
            }
        })
        .collect();

    let current = kept_report(
        app,
        &versions,
        &app.catalog.version,
        SourceParam::Default,
        pilot,
        mode,
    )
    .await?;
    let drill = current.drills.get(&face).and_then(|drills| drills.get(&card_id));

    let by_turn: Vec<TurnRate> = drill
        .map(|drill| drill.by_turn.as_slice())
        .unwrap_or_default()
        .iter()
        .map(|(turn, tally)| TurnRate {
            turn: *turn,
            games: tally.games,
            win_rate: if tally.games > 0 {
                Some(tally.wins as f64 / tally.games as f64)
            } else {
                None
            },
        })
        .collect();

    let co_played: Vec<CoPlayedRate> = drill
        .map(|drill| drill.co_played.as_slice())
        .unwrap_or_default()
        .iter()
        .map(|(id, tally)| CoPlayedRate {
            id: id.clone(),
            name: app
                .catalog
                .defs
                .get(id)
                .map(|def| def.name.clone())
                .unwrap_or_else(|| id.clone()),
            games: tally.games,
            win_rate: if tally.games > 0 {
                Some(tally.wins as f64 / tally.games as f64)
            } else {
                None
            },
        })
        .collect();

    // R4673001: the card's Normal and Radiant figures at the page's patch, read off the window its row is
    // (R3067000: cut from the snapshot the history read, so it loads nothing more).
    let window = kept_window(app, &steps, &patch, SourceParam::Default, pilot, mode).await?;
    let faces = card_faces(app, &window.report, &card_id, def, mode);
    // R4676000: its choices off the same window, on the face asked.
    let variants = card_variants(app, &window.report, &card_id, face);
    // R4679000: what it made and offered, off the same window, on the face asked.
    let outputs = card_outputs(app, &window.report, &card_id, face);
    // R4707001: what its plays rolled, off the same window, on the face asked.
    let rolled = card_rolled(app, &window.report, &card_id, face);

    let body = CardDrillDownResponse {
        card: DrillDownCard {
            id: def.id.clone(),
            name: def.name.clone(),
            cost: numeric_cost(def),
            rarity: def.rarity.as_str().to_string(),
        },
        patches: patch_history,
        by_turn,
        co_played,
        face,
        faces,
        variants,
        outputs,
        rolled,
    };
    Ok(reader_ok(to_json(&body)?, reader))
}

/// R4673000: a card's figures on a face as its row on that face shows them: a Created card no deck held
/// (a token, say) by its Created row, any other card by its deck row (`card_figures`).
fn face_figures(
    app: &App,
    report: &WindowedReport,
    id: &str,
    def: &CardDef,
    mode: PublicStatsMode,
    face: StatsFace,
) -> PublicFaceFigures {
    let fields = face_fields(app, report, face);
    let created = report.created.iter().find(|row| row.stats.card == id);
    if let Some(row) = created.filter(|_| report.card(id).is_none() && !made_by_a_rule(def)) {
        return PublicFaceFigures::created(&row.stats, face, &fields.created, &fields.all);
    }
    let figures = card_figures(report, id, def, mode, face, &fields);
    PublicFaceFigures {
        games: figures.games,
        adjusted_win_rate: figures.adjusted.map(|(rate, _)| rate),
        adjusted_low: figures.adjusted.map(|(_, (low, _))| low),
        adjusted_high: figures.adjusted.map(|(_, (_, high))| high),
        tier: figures.tier,
        has_enough_games: figures.has_enough_games,
    }
}

/// R4673001: the drill-down's face line: the card's Normal and Radiant figures and, when both are shown
/// (R654's floor), the Radiant gap between them.
fn card_faces(
    app: &App,
    report: &WindowedReport,
    id: &str,
    def: &CardDef,
    mode: PublicStatsMode,
) -> PublicCardFaces {
    let normal = face_figures(app, report, id, def, mode, StatsFace::Normal);
    let radiant = face_figures(app, report, id, def, mode, StatsFace::Radiant);
    let gap = normal.shown().zip(radiant.shown()).map(|(normal, radiant)| {
        let (gap, (low, high)) = radiant_gap(normal, radiant);
        PublicRadiantGap { gap, low, high }
    });
    PublicCardFaces { normal, radiant, gap }
}

/// R1583000: one card's play rate: the share of the counted seats that played it from hand.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayRate {
    pub id: String,
    pub rate: f64,
}

/// R1583000: `GET /api/stats/play-rates`'s body: counts and rates only, naming no game and no player.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayRatesResponse {
    /// The human seats of every live game, on every patch and in every mode.
    pub decks: i64,
    /// Each non-token card at least `PLAY_RATES_MIN_GAMES` of those seats held, in catalog-id order.
    pub cards: Vec<PlayRate>,
}

/// The kept play rates, filled by the first request that asks; the others wait on it.
type PlayRatesCell = Keep<Arc<PlayRatesResponse>>;

/// R1583000: the play rates over `totals`. A ladder bot's seat (R1451) and a development game (R378)
/// never count; a card's rate is R377's `played` over every seat counted. `defs` holds only the
/// shipped sets, so a card the server does not serve is left out with the tokens.
fn play_rates_of(totals: &StatsTotals, defs: &CardDefs) -> PlayRatesResponse {
    let report = card_stats_of(
        totals,
        &CardStatsFilter {
            source: SourceFilter::Live,
            mode: None,
            patch: None,
            pilot: PilotFilter::Human,
        },
    );
    let decks = i64::from(report.decks);
    // A listed card's seats are among `decks`, so `decks` is at least `PLAY_RATES_MIN_GAMES` here.
    let cards = report
        .cards
        .iter()
        .filter(|stat| defs.get(&stat.card).is_some_and(|def| !def.is_token()))
        .filter(|stat| tally_of(&stat.in_deck).0 >= PLAY_RATES_MIN_GAMES)
        .map(|stat| PlayRate {
            id: stat.card.clone(),
            rate: tally_of(&stat.played).0 as f64 / decks as f64,
        })
        .collect();
    PlayRatesResponse { decks, cards }
}

/// GET /api/stats/loading (R5186000): what the kept snapshot's load is doing, for the page's progress
/// bar. It never waits on a load, and no cache keeps it.
pub async fn get_loading(app: &Arc<App>, _req: Req) -> ApiResult {
    let mut response = ok_of(&app.card_stats.loading(now_ms()))?;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok(response)
}

/// POST /api/dev/stats-refresh (R5274000): `/stats`' Fetch live, for a dev account (anyone else is answered
/// 403). Lets go of the card statistics the server keeps (`KeptReports::let_go`), so the page's next reads
/// wait for figures loaded now, and answers what the load is doing, as `GET /api/stats/loading` does.
pub async fn post_refresh(app: &Arc<App>, req: Req) -> ApiResult {
    dev_caller(app, &req)?;
    app.card_stats.let_go();
    tracing::info!(event = "stats.let_go");
    ok_of(&app.card_stats.loading(now_ms()))
}

/// GET /api/stats/play-rates (R1583000, `PLAY_RATES_REFRESH_SECONDS`).
pub async fn get_play_rates(app: &Arc<App>, _req: Req) -> ApiResult {
    let keep = app.card_stats.play_rates_cell(now_ms());
    let app = Arc::clone(app);
    let rates = kept_value(keep, async move {
        let mut tx = app.db.begin(None).await?;
        let totals = tx
            .stats_totals_load(&StatsTotalsQuery {
                source: SourceFilter::Live,
                mode: None,
                patch: None,
                drills: false,
                // R4965000: every patch's live totals summed alike, so one total per card and key.
                cards: CardRows::Merged,
            })
            .await?;
        tx.commit().await?;
        Ok::<_, ApiError>(Arc::new(play_rates_of(&totals, &app.catalog.defs)))
    })
    .await?;
    Ok(cached_ok(to_json(rates.as_ref())?, PLAY_RATES_REFRESH_SECONDS))
}

/// GET /api/stats/player: the caller's tracked statistics and privacy setting.
pub async fn get_player(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let row = tx.player_stats_get(&profile.id).await?;
    tx.commit().await?;
    let body = match row {
        Some(row) => {
            json!({ "stats": to_json(&row.stats)?, "isPrivate": row.is_private, "updatedAt": row.updated_at })
        }
        None => json!({ "stats": {}, "isPrivate": false, "updatedAt": null }),
    };
    ok_of(&body)
}

/// R1771000: the game totals the account copy keeps (R1142000), in `syncPayload`'s order.
const ACCOUNT_STATS_TOTALS: [&str; 4] = ["games", "wins", "losses", "draws"];
/// R1771000: the counters each card of the account copy keeps (R1142000), in `syncPayload`'s order.
const ACCOUNT_STATS_CARD_COUNTERS: [&str; 4] = ["played", "playedAgainst", "destroyed", "defeated"];

/// R1771000, R1142000: keeps valid catalog card counters for account copy (R654).
fn account_copy(stats: &Map<String, Value>, defs: &CardDefs) -> IndexMap<String, Value> {
    let mut copy = IndexMap::new();
    for key in ACCOUNT_STATS_TOTALS {
        if let Some(count) = stats.get(key).and_then(whole_number) {
            copy.insert(key.to_string(), Value::from(count));
        }
    }
    if let Some(Value::Object(entries)) = stats.get("cards") {
        let mut cards = Map::new();
        for (id, counters) in entries {
            if !defs.contains_key(id) {
                continue;
            }
            let mut kept = Map::new();
            for key in ACCOUNT_STATS_CARD_COUNTERS {
                if let Some(count) = counters
                    .get(key)
                    .and_then(whole_number)
                    .filter(|count| *count > 0)
                {
                    kept.insert(key.to_string(), Value::from(count));
                }
            }
            if !kept.is_empty() {
                cards.insert(id.clone(), Value::Object(kept));
            }
        }
        copy.insert("cards".to_string(), Value::Object(cards));
    }
    copy
}

/// Returns the serialized length in UTF-16 code units.
fn stringified_length(body: &Value) -> usize {
    body.to_string().encode_utf16().count()
}

/// PUT /api/stats/player (R1771000, R1142000).
pub async fn put_player(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    if stringified_length(&req.body) > PLAYER_STATS_BYTES_MAX {
        return Err(bad_request(format!(
            "player stats exceeds maximum payload size of {PLAYER_STATS_BYTES_MAX} bytes"
        )));
    }

    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let existing = tx.player_stats_get(&profile.id).await?;
    let is_private = match req.body.get("isPrivate") {
        Some(Value::Bool(value)) => *value,
        _ => existing.as_ref().map(|row| row.is_private).unwrap_or(false),
    };

    let stats: IndexMap<String, Value> = match req.body.get("stats") {
        Some(Value::Object(map)) => account_copy(map, &app.catalog.defs),
        _ => existing.as_ref().map(|row| row.stats.clone()).unwrap_or_default(),
    };

    let now = now_ms();
    tx.player_stats_put(&profile.id, &stats, is_private, now).await?;
    tx.commit().await?;
    let stats_json: Map<String, Value> = stats.into_iter().collect();
    ok_of(&json!({ "stats": Value::Object(stats_json), "isPrivate": is_private, "updatedAt": now }))
}

/// GET /api/stats/players: public player summaries, private players excluded.
pub async fn get_players(app: &Arc<App>, req: Req) -> ApiResult {
    let search = query_trimmed(&req, "search");
    let page: u64 = match req.query.get("page") {
        Some(text) if !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit()) => {
            text.parse::<u64>().unwrap_or(u64::MAX).max(1)
        }
        _ => 1,
    };
    let limit = PLAYER_STATS_PAGE_LIMIT as u64;
    let offset = (page - 1).saturating_mul(limit);

    let mut tx = app.db.begin(None).await?;
    let players = tx
        .player_stats_list_public(&PlayerStatsListOptions {
            search,
            limit: limit as i64,
            offset: i64::try_from(offset).unwrap_or(i64::MAX),
        })
        .await?;
    tx.commit().await?;
    Ok(cached_ok(
        json!({ "players": to_json(&players)?, "page": page, "limit": limit }),
        PLAYER_STATS_CACHE_TTL_SECONDS,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    /// How long the test's work waits on the timer before it calls the runtime stuck.
    const STUCK: Duration = Duration::from_secs(10);

    /// How long the timer sleeps before it lets the work go.
    const TIMER: Duration = Duration::from_millis(10);

    /// R5225000: heavy work leaves the runtime free. On a multi-thread runtime of one worker, the work and
    /// the timer it waits on are both that worker's tasks: `block_in_place` hands the timer to another
    /// thread, so it fires; work run in place would hold the one worker, and the timer would never fire.
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn r5225000_heavy_work_leaves_the_runtime_free() {
        let (tell, heard) = mpsc::channel::<()>();
        let work = tokio::spawn(async move { blocking(move || heard.recv_timeout(STUCK).is_ok()) });
        let timer = tokio::spawn(async move {
            tokio::time::sleep(TIMER).await;
            tell.send(()).ok();
        });
        assert!(
            work.await.expect("the work ran"),
            "the work waited out its time: the runtime never ran the timer"
        );
        timer.await.expect("the timer ran");
    }

    /// R5225000: on a one-thread runtime, a test's paused clock among them, the work runs in place, on the
    /// thread that asked for it, so a test that yields a few turns finds it done. The blocking pool's real
    /// threads raced those turns: a series game the sweeper started wrote its match row too late for
    /// `r333_the_sweeper_runs_every_series_sweep_interval_seconds_on_its_own_and_a_failed_sweep_does_not_stop_it`.
    #[tokio::test(flavor = "current_thread")]
    async fn r5225000_a_one_thread_runtime_runs_the_work_in_place() {
        let asked = std::thread::current().id();
        assert_eq!(blocking(|| std::thread::current().id()), asked);
    }

    /// R5245000: when the tests' snapshot finished loading, past the TTL from zero.
    const LOADED: i64 = (CARD_STATS_CACHE_TTL_SECONDS + 100) * MS_PER_SECOND;

    /// R5245000: the value a kept cell holds before its refresh, and the one the refresh fills it with.
    const HELD: u32 = 1;
    const FRESH: u32 = 2;

    /// R5245000: how many turns of the runtime a refresh may take before the test calls it stuck.
    const SETTLE_TURNS: usize = 100;

    fn ttl_ms() -> i64 {
        CARD_STATS_CACHE_TTL_SECONDS * MS_PER_SECOND
    }

    /// An empty snapshot whose load finished at `loaded_at`.
    fn snapshot_at(loaded_at: i64) -> Arc<Snapshot> {
        let history = patch_history();
        let totals = StatsTotals::default();
        let filing = snapshot_filing(&history, &totals, &deck_play_versions(), "x");
        Arc::new(Snapshot::new(&history, totals, filing, loaded_at))
    }

    /// Turns the runtime until no refresh fills `keep`.
    async fn settle<T>(keep: &Keep<T>) {
        for _ in 0..SETTLE_TURNS {
            tokio::task::yield_now().await;
            if !keep.refreshing.load(atomic::Ordering::SeqCst) {
                return;
            }
        }
        panic!("the refresh never finished");
    }

    /// R5245000 (the gap #5207 found): a snapshot still loading is joined however long it takes, never
    /// replaced by a second load beside it once the TTL has passed since it was asked for.
    #[test]
    fn r5245000_a_snapshot_still_loading_is_joined_past_its_ttl() {
        let reports = KeptReports::default();
        let (first, _) = reports.totals_cell(0, Reach::Kept);
        let (again, _) = reports.totals_cell(ttl_ms() + 1, Reach::Kept);
        assert!(
            Arc::ptr_eq(&first.cell, &again.cell),
            "a second load began beside the first"
        );
    }

    /// R5245000: a snapshot is kept the TTL from when its load finished; past it the next cell is made empty
    /// and the snapshot is let go, so the server never holds two (#5139).
    #[test]
    fn r5245000_a_snapshot_is_kept_from_when_its_load_finished_and_let_go_as_the_next_loads() {
        let reports = KeptReports::default();
        let (first, _) = reports.totals_cell(0, Reach::Kept);
        let held = snapshot_at(LOADED);
        assert!(first.cell.set(Arc::clone(&held)).is_ok());
        drop(first);

        let (kept, _) = reports.totals_cell(LOADED + ttl_ms() - 1, Reach::Kept);
        assert!(
            kept.cell
                .get()
                .is_some_and(|snapshot| Arc::ptr_eq(snapshot, &held))
        );
        drop(kept);

        let (next, folds) = reports.totals_cell(LOADED + ttl_ms(), Reach::Kept);
        assert!(!folds);
        assert!(next.cell.get().is_none(), "no next snapshot was asked for");
        assert!(next.last.is_none());
        assert_eq!(
            Arc::strong_count(&held),
            1,
            "the snapshot past its time is still held beside its next"
        );
    }

    /// R5274000: the snapshot is kept six hours from when its load finished, so the reads a match start
    /// makes every few minutes (`win_rate_snapshot`, R3049000) load nothing; the first read at six hours
    /// starts the next load. At five minutes, each of those reads past the fifth reloaded `card_totals`.
    #[test]
    fn r5274000_a_snapshot_is_kept_six_hours_and_the_reads_within_it_load_nothing() {
        const HOUR_MS: i64 = 3600 * MS_PER_SECOND;
        let reports = KeptReports::default();
        let (first, _) = reports.totals_cell(0, Reach::Kept);
        let held = snapshot_at(LOADED);
        assert!(first.cell.set(Arc::clone(&held)).is_ok());
        drop(first);

        for minutes in (5..360).step_by(5) {
            let (kept, _) = reports.totals_cell(LOADED + minutes * HOUR_MS / 60, Reach::Kept);
            assert!(
                kept.cell
                    .get()
                    .is_some_and(|snapshot| Arc::ptr_eq(snapshot, &held)),
                "a read {minutes} minutes after the load started another"
            );
        }
        let (next, _) = reports.totals_cell(LOADED + 6 * HOUR_MS, Reach::Kept);
        assert!(next.cell.get().is_none(), "the snapshot was kept past six hours");
    }

    /// R5274000: Fetch live lets go of the snapshot and every figure cut from it, so the next read waits for a
    /// load rather than answering what was held; a load under way is kept, to be joined.
    #[test]
    fn r5274000_fetch_live_lets_go_of_the_snapshot_and_its_figures_but_joins_a_load_under_way() {
        let reports = KeptReports::default();
        let (loading, _) = reports.totals_cell(0, Reach::Kept);
        reports.let_go();
        let (joined, _) = reports.totals_cell(1, Reach::Kept);
        assert!(
            Arc::ptr_eq(&loading.cell, &joined.cell),
            "a second load began beside the one under way"
        );

        let held = snapshot_at(LOADED);
        assert!(joined.cell.set(Arc::clone(&held)).is_ok());
        drop((loading, joined));
        let key = ReportKey {
            patch: "x".to_string(),
            source: SourceParam::Default,
            pilot: PublicStatsPilot::Merged,
            mode: PublicStatsMode::All,
        };
        let window = reports.window_cell(key.clone(), LOADED);
        let cut = window_over(
            &held.totals,
            &patch_history(),
            &key.patch,
            key.source,
            key.pilot,
            key.mode,
        );
        assert!(window.cell.set(Arc::new(cut)).is_ok());
        assert!(reports.held_window(&key).is_some());

        reports.let_go();
        assert!(
            reports.held_window(&key).is_none(),
            "a figure cut from the snapshot is still held"
        );
        let (next, _) = reports.totals_cell(LOADED + 1, Reach::Kept);
        assert!(next.cell.get().is_none(), "the snapshot was still held");
        assert_eq!(Arc::strong_count(&held), 1, "the snapshot let go is still held");
    }

    /// R5245000: a kept figure still being cut is joined past its TTL; one cut and past it is replaced by a
    /// cell that serves it while its next is cut; one past its time by `CARD_STATS_KEPT_IDLE_SECONDS` with no
    /// read to renew it is let go.
    #[test]
    fn r5245000_a_kept_figure_past_its_ttl_serves_itself_while_its_next_is_cut() {
        let entries: Kept<(), u32> = Mutex::default();
        let first = kept_cell(&entries, (), 0, CARD_STATS_CACHE_TTL_SECONDS);
        let joined = kept_cell(&entries, (), ttl_ms(), CARD_STATS_CACHE_TTL_SECONDS);
        assert!(Arc::ptr_eq(&first.cell, &joined.cell));

        assert!(first.cell.set(HELD).is_ok());
        let next = kept_cell(&entries, (), ttl_ms(), CARD_STATS_CACHE_TTL_SECONDS);
        assert!(!Arc::ptr_eq(&first.cell, &next.cell));
        assert_eq!(next.last, Some(HELD));

        let held_on = kept_cell(&entries, (), 2 * ttl_ms(), CARD_STATS_CACHE_TTL_SECONDS);
        assert_eq!(held_on.last, Some(HELD), "a figure just past its time was let go");
        let idle = ttl_ms() + (CARD_STATS_CACHE_TTL_SECONDS + CARD_STATS_KEPT_IDLE_SECONDS) * MS_PER_SECOND;
        let gone = kept_cell(&entries, (), idle, CARD_STATS_CACHE_TTL_SECONDS);
        assert_eq!(gone.last, None);
    }

    /// R5245000: a held value answers at once while one refresh fills its cell beside it. A refresh that
    /// fails leaves the held value, and the next read starts another; a read while one runs starts none.
    #[tokio::test(flavor = "current_thread")]
    async fn r5245000_a_held_value_answers_at_once_and_one_refresh_fills_it() {
        let keep = Keep::new(Some(HELD));
        let failed = kept_value(keep.clone(), async {
            Err::<u32, _>(ApiError::internal("the store is down"))
        })
        .await;
        assert_eq!(failed.ok(), Some(HELD));
        settle(&keep).await;
        assert!(keep.cell.get().is_none(), "a failed refresh filled the cell");

        let gate = Arc::new(tokio::sync::Notify::new());
        let opened = Arc::clone(&gate);
        let first = kept_value(keep.clone(), async move {
            opened.notified().await;
            Ok::<_, ApiError>(FRESH)
        })
        .await;
        assert_eq!(first.ok(), Some(HELD));
        tokio::task::yield_now().await;
        assert!(keep.refreshing.load(atomic::Ordering::SeqCst));

        let second_ran = Arc::new(AtomicBool::new(false));
        let ran = Arc::clone(&second_ran);
        let second = kept_value(keep.clone(), async move {
            ran.store(true, atomic::Ordering::SeqCst);
            Ok::<_, ApiError>(FRESH)
        })
        .await;
        assert_eq!(second.ok(), Some(HELD));

        gate.notify_one();
        settle(&keep).await;
        assert_eq!(keep.cell.get(), Some(&FRESH));
        assert!(
            !second_ran.load(atomic::Ordering::SeqCst),
            "a second refresh began beside the first"
        );
        let read = kept_value(keep.clone(), async { Ok::<_, ApiError>(HELD) }).await;
        assert_eq!(read.ok(), Some(FRESH));
    }
}
