//! Public card and player statistics page API (SPEC §9.11, R654).
//!
//! R1367000 (amends R654): `GET /api/stats/cards` reads each card over its own window, through the
//! engine's `windowed_card_stats`, the function `cargo jackioh stats report --window` and `winrates`
//! call too; the drill-down's per-patch rates stay per patch.
//!
//! R978000: the card figures are read off a patch's totals once and kept for
//! `CARD_STATS_CACHE_TTL_SECONDS`, one report per patch, `?source=`, `?pilot=` and `?mode=`
//! (`KeptReports`), and both card routes read the kept reports. R1044000: the totals are the records
//! folded as they were written (`db::store::Tx::stats_totals_load`), and read the same figures the
//! records did.
//!
//! R1583000: `GET /api/stats/play-rates` is each card's play rate among the human seats of every
//! live game, read off every patch's totals and kept for `PLAY_RATES_REFRESH_SECONDS`.

use std::cmp::Ordering;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;
use indexmap::IndexMap;
use serde::Serialize;
use serde_json::{Map, Value, json};
use tokio::sync::OnceCell;

use jackioh_cards::patch_history;
use jackioh_engine::wire::stats::{
    CardDrill, CardStats, CardStatsFilter, CardStatsReport, CardWindow, GameMode, GameSource, PatchStep,
    PilotFilter, SourceFilter, StatsTotals, Tally, TotalsKey, WindowQuery, WindowRequest, WindowSource,
    WindowedReport, add_stats, card_drills_of, card_family, card_stats_of, card_window, played_delta,
    sources_of, win_rate, windowed_card_stats,
};
use jackioh_engine::{CardCost, CardDef, CardDefs, Pilot};

use crate::api::catalog::load_patch_versions;
use crate::api::collection::caller_profile;
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, bad_request, lock, now_ms, ok_of, to_json};
use crate::app::App;
use crate::config::{
    CARD_STATS_CACHE_TTL_SECONDS, CARD_STATS_CO_PLAYED_TOP, CARD_STATS_CURVE_TOP, CARD_STATS_KEPT_REPORTS,
    CARD_STATS_MIN_SAMPLE, PLAY_RATES_MIN_GAMES, PLAY_RATES_REFRESH_SECONDS, PLAYER_STATS_BYTES_MAX,
    PLAYER_STATS_CACHE_TTL_SECONDS, PLAYER_STATS_PAGE_LIMIT, PUBLIC_STATS_MIN_LIVE_GAMES,
};
use crate::db::store::{PlayerStatsListOptions, StatsTotalsQuery};
use crate::env::js_number;

/// Unit conversion, not configuration.
const MS_PER_SECOND: i64 = 1000;

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
    pub drawn_games: i64,
    pub drawn_win_rate: Option<f64>,
    pub played_games: i64,
    pub played_win_rate: Option<f64>,
    pub played_radiant_games: i64,
    pub played_radiant_win_rate: Option<f64>,
    pub played_normal_games: i64,
    pub played_normal_win_rate: Option<f64>,
    pub drawn_not_played_games: i64,
    pub drawn_not_played_win_rate: Option<f64>,
    pub played_delta: Option<f64>,
    pub play_rate: f64,
    pub has_enough_games: bool,
    /// R1367000: the card's window and the records its figures count.
    pub window: PublicCardWindow,
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

    /// R1367000: the source a window's figures were counted from.
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

/// Which match type the public figures count (SPEC §9.11 Queries, R377): every mode, or one of
/// R257's three modes. `All` counts what the endpoint always counted.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum PublicStatsMode {
    All,
    Bo1,
    Bo3,
    Random,
}

impl PublicStatsMode {
    /// `?mode=`: one match type, or every mode when missing, `all`, or unrecognised.
    pub fn parse(raw: Option<&str>) -> PublicStatsMode {
        match raw.map(|value| value.trim().to_lowercase()).as_deref() {
            Some("bo1") => PublicStatsMode::Bo1,
            Some("bo3") => PublicStatsMode::Bo3,
            Some("random") => PublicStatsMode::Random,
            _ => PublicStatsMode::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PublicStatsMode::All => "All modes",
            PublicStatsMode::Bo1 => "Best of 1",
            PublicStatsMode::Bo3 => "Conquest",
            PublicStatsMode::Random => "All Random",
        }
    }

    pub fn filter(self) -> Option<GameMode> {
        match self {
            PublicStatsMode::All => None,
            PublicStatsMode::Bo1 => Some(GameMode::Bo1),
            PublicStatsMode::Bo3 => Some(GameMode::Bo3),
            PublicStatsMode::Random => Some(GameMode::Random),
        }
    }
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
    pub summary: PublicStatsSummary,
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

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CardDrillDownResponse {
    pub card: DrillDownCard,
    pub patches: Vec<PatchRate>,
    pub by_turn: Vec<TurnRate>,
    pub co_played: Vec<CoPlayedRate>,
}

fn cached_ok(body: Value, max_age_seconds: i64) -> Response {
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

/// A query parameter, trimmed, when it is present and not empty after trimming.
fn query_trimmed(req: &Req, name: &str) -> Option<String> {
    req.query
        .get(name)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// R1044000: one patch's totals over both sources and every mode, each card's turn and co-played
/// counts only with `drills`.
async fn totals_for(app: &App, patch: &str, drills: bool) -> Result<StatsTotals, ApiError> {
    let mut tx = app.db.begin(None).await?;
    let totals = tx
        .stats_totals_load(&StatsTotalsQuery {
            source: SourceFilter::All,
            mode: None,
            patch: Some(patch.to_string()),
            drills,
        })
        .await?;
    tx.commit().await?;
    Ok(totals)
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
/// R377: `mode` scopes both parts to one match type, or every mode when `None`.
fn public_report(
    totals: &StatsTotals,
    source: SourceFilter,
    patch: &str,
    live_pilot: PilotFilter,
    mode: Option<GameMode>,
) -> CardStatsReport {
    let filter = |source: SourceFilter, pilot: PilotFilter| CardStatsFilter {
        source,
        mode,
        patch: Some(patch.to_string()),
        pilot,
    };
    let mut parts: Vec<CardStatsReport> = Vec::new();
    if matches!(source, SourceFilter::Live | SourceFilter::All) {
        parts.push(card_stats_of(
            totals,
            &filter(SourceFilter::Live, counted_pilot(GameSource::Live, live_pilot)),
        ));
    }
    if matches!(source, SourceFilter::Dev | SourceFilter::All) {
        parts.push(card_stats_of(
            totals,
            &filter(SourceFilter::Dev, counted_pilot(GameSource::Dev, live_pilot)),
        ));
    }
    let mut by_card: IndexMap<String, CardStats> = IndexMap::new();
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
    }
    let mut cards: Vec<CardStats> = by_card.into_values().collect();
    cards.sort_by(|a, b| a.card.cmp(&b.card));
    CardStatsReport {
        filter: filter(
            source,
            if source == SourceFilter::Dev {
                PilotFilter::Unified
            } else {
                live_pilot
            },
        ),
        games,
        decks,
        cards,
    }
}

/// R978000: one patch's public figures for one `?source=` and `?pilot=`, read off its totals once.
struct PatchReport {
    /// The patch's live games, tutorials aside: R654's gate counts them.
    live_games: i64,
    report: CardStatsReport,
    /// Each card's drill-down over the same records, by card id, at most `CARD_STATS_CO_PLAYED_TOP`
    /// co-played cards each. Only the current patch's report for the default source has them: it is
    /// the one `get_card` reads them from.
    drills: IndexMap<String, CardDrill>,
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
type KeptCell = Arc<OnceCell<Arc<PatchReport>>>;

/// R978000: the computed per-patch reports, each with when it was first asked for, the one kept
/// longest ago first. One per `App`, so a deploy or a test starts with none. The lock is never held
/// across a computation.
#[derive(Default)]
pub struct KeptReports {
    entries: Kept<Arc<PatchReport>>,
    /// R1367000: the windowed reports `GET /api/stats/cards` reads, kept as the per-patch ones are.
    windows: Kept<Arc<KeptWindow>>,
    /// R1583000: the play rates `GET /api/stats/play-rates` reads, with when they were first asked for.
    play_rates: Mutex<Option<(i64, PlayRatesCell)>>,
}

/// Kept cells by what they were computed from, each with when it was first asked for.
type Kept<T> = Mutex<IndexMap<ReportKey, (i64, Arc<OnceCell<T>>)>>;

/// The cell for `key` in `entries`: the one made less than `CARD_STATS_CACHE_TTL_SECONDS` before
/// `now`, or a new empty one, dropping the oldest to stay within `CARD_STATS_KEPT_REPORTS`.
fn kept_cell<T>(entries: &Kept<T>, key: ReportKey, now: i64) -> Arc<OnceCell<T>> {
    let mut entries = lock(entries);
    entries.retain(|_, (made_at, _)| now - *made_at < CARD_STATS_CACHE_TTL_SECONDS * MS_PER_SECOND);
    if let Some((_, cell)) = entries.get(&key) {
        return Arc::clone(cell);
    }
    while entries.len() >= CARD_STATS_KEPT_REPORTS {
        entries.shift_remove_index(0);
    }
    let cell = Arc::new(OnceCell::new());
    entries.insert(key, (now, Arc::clone(&cell)));
    cell
}

impl KeptReports {
    fn cell(&self, key: ReportKey, now: i64) -> KeptCell {
        kept_cell(&self.entries, key, now)
    }

    fn window_cell(&self, key: ReportKey, now: i64) -> KeptWindowCell {
        kept_cell(&self.windows, key, now)
    }

    /// R1583000: the play rates' cell made less than `PLAY_RATES_REFRESH_SECONDS` before `now`, or a
    /// new empty one in its place.
    fn play_rates_cell(&self, now: i64) -> PlayRatesCell {
        let mut kept = lock(&self.play_rates);
        if let Some((made_at, cell)) = kept.as_ref()
            && now - *made_at < PLAY_RATES_REFRESH_SECONDS * MS_PER_SECOND
        {
            return Arc::clone(cell);
        }
        let cell: PlayRatesCell = Arc::new(OnceCell::new());
        *kept = Some((now, Arc::clone(&cell)));
        cell
    }
}

/// R1367000: one patch's windowed figures for one `?source=` and `?pilot=`, and each version's live
/// games, which the window of a card no counted deck held is read from.
struct KeptWindow {
    report: WindowedReport,
    live_games: IndexMap<String, i64>,
}

impl KeptWindow {
    /// The window of a card, held or not.
    fn window_of(&self, history: &[PatchStep], id: &str, request: WindowRequest) -> CardWindow {
        match self.report.card(id) {
            Some(card) => card.window.clone(),
            None => card_window(
                history,
                &self.report.at,
                Some(card_family(id)),
                request,
                |version| self.live_games.get(version).copied().unwrap_or(0),
            ),
        }
    }
}

type KeptWindowCell = Arc<OnceCell<Arc<KeptWindow>>>;

/// R1367000: `at`'s windowed report for `source` and `pilot`, kept as `kept_report` keeps a patch's:
/// read off every patch's totals once per `CARD_STATS_CACHE_TTL_SECONDS`, and never kept for a patch
/// the server does not know.
async fn kept_window(
    app: &App,
    history: &[PatchStep],
    at: &str,
    source: SourceParam,
    pilot: PublicStatsPilot,
    mode: PublicStatsMode,
) -> Result<Arc<KeptWindow>, ApiError> {
    let compute = || async move {
        let mut tx = app.db.begin(None).await?;
        let totals = tx
            .stats_totals_load(&StatsTotalsQuery {
                source: SourceFilter::All,
                mode: None,
                patch: None,
                drills: false,
            })
            .await?;
        tx.commit().await?;
        let report = windowed_card_stats(
            &totals,
            &WindowQuery {
                history,
                at,
                live_pilot: pilot.filter(),
                mode: mode.filter(),
                request: source.request(),
            },
        );
        let mut live_games: IndexMap<String, i64> = IndexMap::new();
        for key in totals.records.keys() {
            live_games
                .entry(key.patch.clone())
                .or_insert_with(|| totals.live_games(&key.patch));
        }
        Ok::<_, ApiError>(Arc::new(KeptWindow { report, live_games }))
    };
    let known = at == app.catalog.version || history.iter().any(|step| step.version == at);
    if !known {
        return compute().await;
    }
    let key = ReportKey {
        patch: at.to_string(),
        source,
        pilot,
        mode,
    };
    let cell = app.card_stats.window_cell(key, now_ms());
    let kept = cell.get_or_try_init(compute).await?;
    Ok(Arc::clone(kept))
}

/// R978000: `patch`'s figures for `source_param`, `live_pilot` and `mode`, read off the patch's
/// totals (R654's gate and `?source=` choose which source's count). With `with_drills`, each card's
/// drill-down too, over the seats `public_report` counts (`seat_counted`). R654: a tutorial game is
/// never counted; no game mode is `tutorial` and the server records no tutorial game, so the totals
/// hold none.
fn patch_report(
    totals: &StatsTotals,
    patch: &str,
    source_param: SourceParam,
    live_pilot: PilotFilter,
    mode: Option<GameMode>,
    with_drills: bool,
) -> PatchReport {
    let live_games = totals.live_games(patch);

    // R654 publication gate.
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
        card_drills_of(
            totals,
            |key: &TotalsKey| {
                key.patch == patch
                    && sources.contains(&key.source)
                    && mode.is_none_or(|mode| key.mode == mode)
                    && seat_counted(key.source, key.pilot, live_pilot)
            },
            CARD_STATS_CO_PLAYED_TOP,
        )
    } else {
        IndexMap::new()
    };
    PatchReport {
        live_games,
        report,
        drills,
    }
}

/// R978000: `patch`'s report for `source`, `pilot` and `mode`, as kept: computed from the
/// patch's totals when none was made in the last `CARD_STATS_CACHE_TTL_SECONDS`, once however many
/// requests miss it together. A patch the server does not know (neither in `versions` nor the
/// current one) is computed on every request and never kept, so made-up `?patch=` values push no
/// kept report out.
async fn kept_report(
    app: &App,
    versions: &[String],
    patch: &str,
    source: SourceParam,
    pilot: PublicStatsPilot,
    mode: PublicStatsMode,
) -> Result<Arc<PatchReport>, ApiError> {
    let current = patch == app.catalog.version;
    let compute = || async move {
        let with_drills = current && source == SourceParam::Default;
        let totals = totals_for(app, patch, with_drills).await?;
        Ok::<_, ApiError>(Arc::new(patch_report(
            &totals,
            patch,
            source,
            pilot.filter(),
            mode.filter(),
            with_drills,
        )))
    };
    if !current && !versions.iter().any(|version| version == patch) {
        return compute().await;
    }
    let key = ReportKey {
        patch: patch.to_string(),
        source,
        pilot,
        mode,
    };
    let cell = app.card_stats.cell(key, now_ms());
    let kept = cell.get_or_try_init(compute).await?;
    Ok(Arc::clone(kept))
}

/// GET /api/stats/cards: public card win rates. R654's gate, amended by R1367000: each card's figures
/// are read over its own window (`windowed_card_stats`), live games only once the window has logged
/// 1000 live games, else its previous window's live games when that one has, else AI games pad the
/// window. `gate` and `source` are the window every card shares (its X's). Below 20 games
/// `hasEnoughGames` is false.
/// R1451, amended by R807000: a ladder bot's game counts toward the gate like any live game, and
/// the figures count every counted seat, human and bot alike (`?pilot=human` reads the people's
/// sides only, `?pilot=ai` the bots'). R377: `?mode=` scopes the figures to one match type
/// (`bo1`, `bo3`, `random`), or every mode when missing, `all`, or unrecognised. A bot game adds
/// two decks like any game, so the per-game deck tell R1452 recorded is gone.
pub async fn get_cards(app: &Arc<App>, req: Req) -> ApiResult {
    let versions = load_patch_versions().await;
    let current_patch = app.catalog.version.clone();
    let requested_patch = query_trimmed(&req, "patch").unwrap_or(current_patch);
    let source_param = SourceParam::parse(req.query.get("source").map(String::as_str));
    let pilot = PublicStatsPilot::parse(req.query.get("pilot").map(String::as_str));
    let mode = PublicStatsMode::parse(req.query.get("mode").map(String::as_str));
    let patch_index = versions.iter().position(|version| *version == requested_patch);
    let previous_patch = match patch_index {
        Some(index) if index > 0 => versions.get(index - 1).cloned(),
        _ => None,
    };

    // R978000, R1367000: every card's figures over its own window at the patch, as kept.
    let history = patch_history();
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

    let mut cards: Vec<PublicCardStat> = Vec::new();
    for (id, def) in &app.catalog.defs {
        if def.token {
            continue; // Only non-token deckable cards
        }
        if card_param.as_ref().is_some_and(|card| id != card) {
            continue;
        }
        // A set is its cards' `set` alone: `classic` begins every Classic+ id (`classicplus-…`).
        if let Some(set) = &set_param
            && def.set.as_str().to_lowercase() != *set
        {
            continue;
        }
        if let Some(rarity) = &rarity_param
            && def.rarity.as_str().to_lowercase() != *rarity
        {
            continue;
        }
        let cost = numeric_cost(def);
        if let Some(cost_text) = cost_param.as_deref().filter(|text| !text.is_empty())
            && let Some(parsed_cost) =
                Some(js_number(&cost_text.replacen('+', "", 1))).filter(|number| !number.is_nan())
        {
            if parsed_cost >= CARD_STATS_CURVE_TOP as f64 {
                if (cost as f64) < CARD_STATS_CURVE_TOP as f64 {
                    continue;
                }
            } else if cost as f64 != parsed_cost {
                continue;
            }
        }

        let held = report.card(id);
        let stat = held.map(|card| &card.stats);
        let total_decks = held.map(|card| card.decks).unwrap_or(0);
        let window = kept.window_of(&history, id, request);
        let (in_deck_games, _) = stat.map(|stat| tally_of(&stat.in_deck)).unwrap_or((0, 0));
        let rate = stat.and_then(|stat| win_rate(&stat.in_deck));
        let (played_games, played_wins) = stat.map(|stat| tally_of(&stat.played)).unwrap_or((0, 0));
        let (played_radiant_games, _) = stat.map(|stat| tally_of(&stat.played_radiant)).unwrap_or((0, 0));
        let played_radiant_rate = stat.and_then(|stat| win_rate(&stat.played_radiant));
        let (played_normal_games, _) = stat.map(|stat| tally_of(&stat.played_normal)).unwrap_or((0, 0));
        let played_normal_rate = stat.and_then(|stat| win_rate(&stat.played_normal));
        let (unplayed_games, unplayed_wins) = stat
            .map(|stat| tally_of(&stat.drawn_not_played))
            .unwrap_or((0, 0));
        let drawn_games = played_games + unplayed_games;
        let drawn_wins = played_wins + unplayed_wins;
        let drawn_rate = if drawn_games > 0 {
            Some(drawn_wins as f64 / drawn_games as f64)
        } else {
            None
        };
        let played_rate = stat.and_then(|stat| win_rate(&stat.played));
        let drawn_not_played_rate = stat.and_then(|stat| win_rate(&stat.drawn_not_played));
        // R654: the delta waits for the sample floor on both its sides, as a small baseline swings it ±50.
        let delta = stat
            .filter(|_| played_games >= CARD_STATS_MIN_SAMPLE && unplayed_games >= CARD_STATS_MIN_SAMPLE)
            .and_then(played_delta);
        let play_rate = if total_decks > 0 {
            in_deck_games as f64 / total_decks as f64
        } else {
            0.0
        };
        let has_enough_games = in_deck_games >= CARD_STATS_MIN_SAMPLE;

        cards.push(PublicCardStat {
            id: id.clone(),
            name: def.name.clone(),
            cost,
            rarity: def.rarity.as_str().to_string(),
            set: def.set.as_str().to_string(),
            games: in_deck_games,
            win_rate: rate,
            drawn_games,
            drawn_win_rate: drawn_rate,
            played_games,
            played_win_rate: played_rate,
            played_radiant_games,
            played_radiant_win_rate: played_radiant_rate,
            played_normal_games,
            played_normal_win_rate: played_normal_rate,
            drawn_not_played_games: unplayed_games,
            drawn_not_played_win_rate: drawn_not_played_rate,
            played_delta: delta,
            play_rate,
            has_enough_games,
            window: PublicCardWindow::of(&window),
        });
    }

    cards.sort_by(|a, b| {
        if a.has_enough_games && !b.has_enough_games {
            return Ordering::Less;
        }
        if !a.has_enough_games && b.has_enough_games {
            return Ordering::Greater;
        }
        if let (Some(a_rate), Some(b_rate)) = (a.win_rate, b.win_rate)
            && a_rate != b_rate
        {
            return b_rate.partial_cmp(&a_rate).unwrap_or(Ordering::Equal);
        }
        b.games
            .cmp(&a.games)
            .then_with(|| locale_compare(&a.name, &b.name))
    });

    let qualifying: Vec<&PublicCardStat> = cards
        .iter()
        .filter(|card| card.has_enough_games && card.win_rate.is_some())
        .collect();
    let extreme = |card: &PublicCardStat| PublicStatsExtreme {
        id: card.id.clone(),
        name: card.name.clone(),
        win_rate: card.win_rate.unwrap_or(0.0),
        games: card.games,
    };
    let best_card = qualifying.first().map(|card| extreme(card));
    let worst_card = qualifying.last().map(|card| extreme(card));

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

    Ok(cached_ok(to_json(&response)?, CARD_STATS_CACHE_TTL_SECONDS))
}

/// GET /api/stats/cards/:id: win rate by cleared patch and by turn played, and co-played cards.
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

    // R978000: each patch's history and the current patch's drill-down read the kept reports.
    let versions = load_patch_versions().await;
    let mut patch_history: Vec<PatchRate> = Vec::new();

    for version in &versions {
        let kept = kept_report(app, &versions, version, SourceParam::Default, pilot, mode).await?;
        if kept.live_games >= PUBLIC_STATS_MIN_LIVE_GAMES {
            let stat = kept.report.cards.iter().find(|stat| stat.card == card_id);
            patch_history.push(PatchRate {
                patch: version.clone(),
                games: stat.map(|stat| tally_of(&stat.in_deck).0).unwrap_or(0),
                win_rate: stat.and_then(|stat| win_rate(&stat.in_deck)),
            });
        }
    }

    let current = kept_report(
        app,
        &versions,
        &app.catalog.version,
        SourceParam::Default,
        pilot,
        mode,
    )
    .await?;
    let drill = current.drills.get(&card_id);

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
    };
    Ok(cached_ok(to_json(&body)?, CARD_STATS_CACHE_TTL_SECONDS))
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
type PlayRatesCell = Arc<OnceCell<Arc<PlayRatesResponse>>>;

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

/// GET /api/stats/play-rates (R1583000): read off every patch's live totals once per
/// `PLAY_RATES_REFRESH_SECONDS` however many ask (a failed load is kept for no one), cacheable as long.
pub async fn get_play_rates(app: &Arc<App>, _req: Req) -> ApiResult {
    let cell = app.card_stats.play_rates_cell(now_ms());
    let rates = cell
        .get_or_try_init(|| async {
            let mut tx = app.db.begin(None).await?;
            let totals = tx
                .stats_totals_load(&StatsTotalsQuery {
                    source: SourceFilter::Live,
                    mode: None,
                    patch: None,
                    drills: false,
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

/// The body's length as `JSON.stringify(body).length`: UTF-16 code units.
fn stringified_length(body: &Value) -> usize {
    body.to_string().encode_utf16().count()
}

/// PUT /api/stats/player: the caller updates their statistics and privacy setting.
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
        Some(Value::Object(map)) => map
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
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
