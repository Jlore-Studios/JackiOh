//! Card statistics (SPEC §9.11, R376–R378): the record a finished game leaves, and the card win rates
//! read off a set of those records.
//!
//! Two writers make records. The server writes one for every live match that ends, once its result
//! is in (`crates/server/src/api/game_records.rs`), and the AI's development runs write one for every
//! game they play (`crates/ai/src/dev_run.rs`). Both take the `game` half from the engine's
//! `summarize_game`, which reads it off `(seed, decks, log)`. Everything below is pure data over those
//! records, shared by the server's report and import scripts and the AI's run, so the numbers the
//! three print are one computation.
//!
//! Port of `packages/shared/src/stats.ts` and, as `mod tests`, `packages/shared/test/stats.test.ts`
//! (SURFACE §4.1, §10.4: the web keeps its TS copy, `apps/web/src/wire/stats.ts`).

use std::ops::Index;

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::state::EngineError;
use crate::wire::catalog_types::{PLAYER_IDS, PerPlayer, PlayerId};
use crate::wire::events::{GameOverReason, Winner};
use crate::wire::string_union;

// ---------------------------------------------------------------------------
// The record (R376)
// ---------------------------------------------------------------------------

string_union! {
    /// R376, R378: where a game was played. `live` is a match the server ran; `dev` is a game of an
    /// internal AI development run, which no live figure includes unless it is asked for.
    pub enum GameSource {
        Live = "live",
        Dev = "dev",
    }
}

/// R376, R378: every record source, in TS's order.
pub const GAME_SOURCES: &[GameSource] = GameSource::ALL;

string_union! {
    /// R257's three modes: the match type a record is filed under. Conquest is `bo3` on the wire.
    pub enum GameMode {
        Bo1 = "bo1",
        Bo3 = "bo3",
        Random = "random",
    }
}

/// R257's three modes, in TS's order.
pub const GAME_MODES: &[GameMode] = GameMode::ALL;

string_union! {
    /// R376: who chose a seat's moves.
    pub enum Pilot {
        Human = "human",
        Ai = "ai",
    }
}

/// R376: both pilots, in TS's order.
pub const PILOTS: &[Pilot] = Pilot::ALL;

/// §2.5's endings, as `gameOver` names them, for reading a record back. TS proved the list complete
/// with a type (`GameOverReasonsAreExhaustive`); here the test `game_over_reasons_are_exhaustive` does.
pub const GAME_OVER_REASONS: &[GameOverReason] = &[
    GameOverReason::HeroDeath,
    GameOverReason::BothHeroesDead,
    GameOverReason::Concede,
    GameOverReason::DrawAccepted,
    GameOverReason::TurnCap,
    GameOverReason::Disconnect,
    GameOverReason::MatchCeiling,
    GameOverReason::Voided,
    GameOverReason::AltWin,
    GameOverReason::WonByEffect,
];

/// R376: what one seat's cards did in one game, as catalog ids.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct SeatSummary {
    /// The decklist the seat played.
    pub deck: Vec<String>,
    /// Its hand once both mulligans had resolved, as the first turn began: the cards it kept, its
    /// replacements and, for the seat going second, The Coin (§2.1). Empty when the game ended before
    /// a turn began.
    pub opening: Vec<String>,
    /// Every card it drew into its hand after that, in order. A card burned or cast on its draw never reached the hand.
    pub drawn: Vec<String>,
    /// Every card it played from its hand, in order: a cast (§2.4, R70) is not a play from hand.
    pub played: Vec<String>,
    /// Turn number (1-indexed) each card in `played` was played on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub played_turns: Option<Vec<i32>>,
}

/// R376: a finished game, read off its replay by the engine's `summarize_game`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct GameSummary {
    /// The seat that took the first turn (§2.1: Player 1).
    pub first: PlayerId,
    pub winner: Winner,
    pub reason: GameOverReason,
    /// The player-turn counter at the end (§2.5).
    pub turns: i32,
    pub seats: PerPlayer<SeatSummary>,
}

/// R378: how a development record's id begins, and a live one's (a match id) never does.
pub const DEV_RECORD_ID_PREFIX: &str = "dev:";

/// R376: one game, filed by where, how and by whom it was played.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct GameRecord {
    /// Unique among records: a live game's match id, a development game's `dev:<patch>:<series>:<n>`.
    pub id: String,
    pub source: GameSource,
    pub mode: GameMode,
    /// R388's version of the cards the game was played with: the newest patch of the build that played it.
    pub patch: String,
    pub pilots: PerPlayer<Pilot>,
    pub game: GameSummary,
}

// ---------------------------------------------------------------------------
// Which records a query reads (R377, R378)
// ---------------------------------------------------------------------------

string_union! {
    /// R378: live data unless a development run is asked for by name; `all` is both.
    pub enum SourceFilter {
        Live = "live",
        Dev = "dev",
        All = "all",
    }
}

/// R378: every source filter, in TS's order.
pub const SOURCE_FILTERS: &[SourceFilter] = SourceFilter::ALL;

string_union! {
    /// R377: the seats a query counts. `unified` is every seat, human and AI combined.
    pub enum PilotFilter {
        Human = "human",
        Ai = "ai",
        Unified = "unified",
    }
}

/// R377: every pilot filter, in TS's order.
pub const PILOT_FILTERS: &[PilotFilter] = PilotFilter::ALL;

/// Any combination of match type, patch and pilot, over live data, development data or both.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CardStatsFilter {
    pub source: SourceFilter,
    /// One match type, or null for every mode.
    pub mode: Option<GameMode>,
    /// One patch, or null for every patch.
    pub patch: Option<String>,
    pub pilot: PilotFilter,
}

/// R378: what a query reads when it names nothing: live games of every mode, patch and pilot.
pub const DEFAULT_CARD_STATS_FILTER: CardStatsFilter = CardStatsFilter {
    source: SourceFilter::Live,
    mode: None,
    patch: None,
    pilot: PilotFilter::Unified,
};

/// The record sources a source filter reads. Only `dev` and `all` read a development run.
pub fn sources_of(source: SourceFilter) -> Vec<GameSource> {
    match source {
        SourceFilter::All => GAME_SOURCES.to_vec(),
        SourceFilter::Live => vec![GameSource::Live],
        SourceFilter::Dev => vec![GameSource::Dev],
    }
}

/// Whether a record is one the filter reads: its source, its mode and its patch. Pilots are per seat.
pub fn record_matches(record: &GameRecord, filter: &CardStatsFilter) -> bool {
    sources_of(filter.source).contains(&record.source)
        && filter.mode.is_none_or(|mode| record.mode == mode)
        && filter.patch.as_ref().is_none_or(|patch| record.patch == *patch)
}

/// The seats of a record the pilot filter counts, in seat order.
pub fn seats_counted(record: &GameRecord, pilot: PilotFilter) -> Vec<PlayerId> {
    PLAYER_IDS
        .into_iter()
        .filter(|seat| pilot == PilotFilter::Unified || record.pilots[*seat].as_str() == pilot.as_str())
        .collect()
}

// ---------------------------------------------------------------------------
// The breakdowns (R377)
// ---------------------------------------------------------------------------

/// Games, wins and draws for one breakdown of one card. A game is one deck in one game, so a game in
/// which both decks held the card counts once for each, and a draw is a game that was not won.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct Tally {
    pub games: i32,
    pub wins: i32,
    pub draws: i32,
}

string_union! {
    /// R377's breakdowns, each over the decks that held the card:
    ///  - `inDeck`: every one of them;
    ///  - `openingHand`: the card was in the opening hand (`SeatSummary.opening`);
    ///  - `goingFirst` and `goingSecond`: the deck's seat took the first turn, or did not;
    ///  - `played`: the seat played the card from its hand at least once;
    ///  - `drawnNotPlayed`: the card reached the seat's hand — in the opening hand or by a later draw —
    ///    and the seat never played it. This is the played delta's baseline.
    pub enum Breakdown {
        InDeck = "inDeck",
        OpeningHand = "openingHand",
        GoingFirst = "goingFirst",
        GoingSecond = "goingSecond",
        Played = "played",
        DrawnNotPlayed = "drawnNotPlayed",
    }
}

/// R377's breakdowns, in column order.
pub const BREAKDOWNS: &[Breakdown] = Breakdown::ALL;

/// `{ card: string } & Record<Breakdown, Tally>`: one card's breakdowns.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CardStats {
    pub card: String,
    pub in_deck: Tally,
    pub opening_hand: Tally,
    pub going_first: Tally,
    pub going_second: Tally,
    pub played: Tally,
    pub drawn_not_played: Tally,
}

/// `stats[breakdown]`.
impl Index<Breakdown> for CardStats {
    type Output = Tally;

    fn index(&self, breakdown: Breakdown) -> &Tally {
        match breakdown {
            Breakdown::InDeck => &self.in_deck,
            Breakdown::OpeningHand => &self.opening_hand,
            Breakdown::GoingFirst => &self.going_first,
            Breakdown::GoingSecond => &self.going_second,
            Breakdown::Played => &self.played,
            Breakdown::DrawnNotPlayed => &self.drawn_not_played,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CardStatsReport {
    pub filter: CardStatsFilter,
    /// Records the filter read with at least one seat it counts.
    pub games: i32,
    /// Seats counted across them: the decks every card's figures are drawn from.
    pub decks: i32,
    /// One entry per card some counted deck held, by catalog id.
    pub cards: Vec<CardStats>,
}

fn empty_tally() -> Tally {
    Tally {
        games: 0,
        wins: 0,
        draws: 0,
    }
}

fn empty_stats(card: &str) -> CardStats {
    CardStats {
        card: card.to_string(),
        in_deck: empty_tally(),
        opening_hand: empty_tally(),
        going_first: empty_tally(),
        going_second: empty_tally(),
        played: empty_tally(),
        drawn_not_played: empty_tally(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    Win,
    Draw,
    Loss,
}

fn outcome_for(game: &GameSummary, seat: PlayerId) -> Outcome {
    match game.winner.player() {
        None => Outcome::Draw,
        Some(winner) if winner == seat => Outcome::Win,
        Some(_) => Outcome::Loss,
    }
}

fn count(tally: &mut Tally, outcome: Outcome) {
    tally.games += 1;
    if outcome == Outcome::Win {
        tally.wins += 1;
    }
    if outcome == Outcome::Draw {
        tally.draws += 1;
    }
}

/// R377: every card's breakdowns over the records the filter reads. A card is counted by its catalog
/// id for the decks that held it, so a copy of it made in play counts as the card for those decks and
/// a card no counted deck held has no entry. `cards` lists the ids in catalog-id order.
///
/// TS's `filter` defaulted to `DEFAULT_CARD_STATS_FILTER`; a caller passes `&DEFAULT_CARD_STATS_FILTER`.
pub fn card_stats(records: &[GameRecord], filter: &CardStatsFilter) -> CardStatsReport {
    let mut by_card: IndexMap<String, CardStats> = IndexMap::new();
    let mut games = 0;
    let mut decks = 0;

    for record in records {
        if !record_matches(record, filter) {
            continue;
        }
        let seats = seats_counted(record, filter.pilot);
        if seats.is_empty() {
            continue;
        }
        games += 1;

        for seat in seats {
            decks += 1;
            let summary = &record.game.seats[seat];
            let outcome = outcome_for(&record.game, seat);
            let opening: IndexSet<&str> = summary.opening.iter().map(String::as_str).collect();
            let in_hand: IndexSet<&str> = summary
                .opening
                .iter()
                .chain(summary.drawn.iter())
                .map(String::as_str)
                .collect();
            let played: IndexSet<&str> = summary.played.iter().map(String::as_str).collect();
            let first = record.game.first == seat;

            let deck: IndexSet<&str> = summary.deck.iter().map(String::as_str).collect();
            for card in deck {
                let stats = by_card
                    .entry(card.to_string())
                    .or_insert_with(|| empty_stats(card));
                count(&mut stats.in_deck, outcome);
                if opening.contains(card) {
                    count(&mut stats.opening_hand, outcome);
                }
                count(
                    if first {
                        &mut stats.going_first
                    } else {
                        &mut stats.going_second
                    },
                    outcome,
                );
                if played.contains(card) {
                    count(&mut stats.played, outcome);
                } else if in_hand.contains(card) {
                    count(&mut stats.drawn_not_played, outcome);
                }
            }
        }
    }

    let mut cards: Vec<CardStats> = by_card.into_values().collect();
    // A stable sort by id (SURFACE §4.4.1; ids are ASCII, so `str::cmp` is JS's `<`).
    cards.sort_by(|a, b| a.card.cmp(&b.card));
    CardStatsReport {
        filter: filter.clone(),
        games,
        decks,
        cards,
    }
}

/// Wins over games; `None` when there are no games, so an empty breakdown never reads as 0%.
pub fn win_rate(tally: &Tally) -> Option<f64> {
    if tally.games == 0 {
        None
    } else {
        Some(f64::from(tally.wins) / f64::from(tally.games))
    }
}

/// R377: the played delta, the played win rate minus the drawn-but-not-played win rate, as a fraction
/// (0.082 is 8.2 points). `None` when either side has no games.
pub fn played_delta(stats: &CardStats) -> Option<f64> {
    let played = win_rate(&stats.played)?;
    let baseline = win_rate(&stats.drawn_not_played)?;
    Some(played - baseline)
}

// ---------------------------------------------------------------------------
// The report as text
// ---------------------------------------------------------------------------

/// A rate or a delta prints with this many decimal places: "52.1%", "+8.2". Presentation only.
const DECIMALS: usize = 1;
/// A fraction as a percentage.
const PERCENT: f64 = 100.0;
/// What a breakdown with no games, or a delta it cannot compute, prints.
const NO_FIGURE: &str = "—";
/// How many places past the rounding place `to_fixed` reads a double's exact expansion to: far past
/// the last place where a double near a tie can differ from it.
const EXACT_EXTRA_DIGITS: usize = 40;

/// TS `MODE_NAMES`.
fn mode_name(mode: GameMode) -> &'static str {
    match mode {
        GameMode::Bo1 => "Best of 1",
        GameMode::Bo3 => "Conquest",
        GameMode::Random => "All Random",
    }
}

/// TS `SOURCE_NAMES`.
fn source_name(source: SourceFilter) -> &'static str {
    match source {
        SourceFilter::Live => "live games",
        SourceFilter::Dev => "AI development runs",
        SourceFilter::All => "live games and AI development runs",
    }
}

/// TS `PILOT_NAMES`.
fn pilot_name(pilot: PilotFilter) -> &'static str {
    match pilot {
        PilotFilter::Human => "human pilots",
        PilotFilter::Ai => "AI pilots",
        PilotFilter::Unified => "human and AI pilots (unified)",
    }
}

/// `Readonly<Record<Breakdown, string>>`: `BREAKDOWN_TITLES[breakdown]` is the column's heading.
#[derive(Clone, Copy, Debug)]
pub struct BreakdownTitles;

impl Index<Breakdown> for BreakdownTitles {
    type Output = str;

    fn index(&self, breakdown: Breakdown) -> &str {
        match breakdown {
            Breakdown::InDeck => "In deck",
            Breakdown::OpeningHand => "Opening hand",
            Breakdown::GoingFirst => "Going first",
            Breakdown::GoingSecond => "Going second",
            Breakdown::Played => "Played",
            Breakdown::DrawnNotPlayed => "Drawn, not played",
        }
    }
}

/// The column headings, in `BREAKDOWNS` order.
pub const BREAKDOWN_TITLES: BreakdownTitles = BreakdownTitles;

/// One line naming what a filter read.
pub fn describe_filter(filter: &CardStatsFilter) -> String {
    let mode = match filter.mode {
        None => "every mode".to_string(),
        Some(mode) => mode_name(mode).to_string(),
    };
    let patch = match &filter.patch {
        None => "every patch".to_string(),
        Some(patch) => format!("patch {patch}"),
    };
    format!(
        "{}, {}, {}, {}",
        source_name(filter.source),
        mode,
        patch,
        pilot_name(filter.pilot)
    )
}

/// JS `x.toFixed(digits)` for a finite `x` below 1e21: the decimal with `digits` places nearest `x`'s
/// exact value, a tie going to the larger magnitude, and a "-" before any negative `x` — JS writes
/// `(-0.01).toFixed(1)` as "-0.0". Rust's `{:.1}` breaks ties to even, so it is not used directly.
fn to_fixed(x: f64, digits: usize) -> String {
    let negative = x < 0.0;
    let exact = format!("{:.*}", digits + EXACT_EXTRA_DIGITS, x.abs());
    let (whole, fraction) = exact.split_once('.').unwrap_or((exact.as_str(), ""));
    let mut kept: Vec<u8> = whole
        .bytes()
        .chain(fraction.bytes().take(digits))
        .map(|digit| digit - b'0')
        .collect();
    if fraction
        .as_bytes()
        .get(digits)
        .is_some_and(|digit| *digit >= b'5')
    {
        let mut at = kept.len();
        loop {
            if at == 0 {
                kept.insert(0, 1);
                break;
            }
            at -= 1;
            if kept[at] == 9 {
                kept[at] = 0;
            } else {
                kept[at] += 1;
                break;
            }
        }
    }
    let whole_len = kept.len() - digits;
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    for (at, digit) in kept.iter().enumerate() {
        if at == whole_len {
            out.push('.');
        }
        out.push(char::from(b'0' + digit));
    }
    out
}

/// "52.1% (310)": the win rate with its sample size, the games behind it, always beside it.
pub fn format_tally(tally: &Tally) -> String {
    let shown = match win_rate(tally) {
        None => NO_FIGURE.to_string(),
        Some(rate) => format!("{}%", to_fixed(rate * PERCENT, DECIMALS)),
    };
    format!("{shown} ({})", tally.games)
}

/// "+8.2": the played delta in percentage points.
pub fn format_delta(delta: Option<f64>) -> String {
    let Some(delta) = delta else {
        return NO_FIGURE.to_string();
    };
    let points = to_fixed(delta * PERCENT, DECIMALS);
    if delta > 0.0 { format!("+{points}") } else { points }
}

/// JS `string.length`: UTF-16 code units, which `padEnd`/`padStart` count too ("Δ", "—").
fn js_length(text: &str) -> usize {
    text.encode_utf16().count()
}

fn pad_end(text: &str, width: usize) -> String {
    let mut out = text.to_string();
    out.push_str(&" ".repeat(width.saturating_sub(js_length(text))));
    out
}

fn pad_start(text: &str, width: usize) -> String {
    let mut out = " ".repeat(width.saturating_sub(js_length(text)));
    out.push_str(text);
    out
}

/// The report as a plain-text table: one row per card, each breakdown's win rate with its games
/// beside it, and the played delta. `name_of` turns an id into the name shown after it (TS's
/// default, `() => undefined`, is `|_| None`).
pub fn format_card_stats(report: &CardStatsReport, name_of: impl Fn(&str) -> Option<String>) -> String {
    let mut header: Vec<String> = vec!["Card".to_string()];
    header.extend(
        BREAKDOWNS
            .iter()
            .map(|breakdown| BREAKDOWN_TITLES[*breakdown].to_string()),
    );
    header.push("Played Δ".to_string());
    let rows: Vec<Vec<String>> = report
        .cards
        .iter()
        .map(|stats| {
            let name = name_of(&stats.card);
            let mut row = vec![match name {
                None => stats.card.clone(),
                Some(name) => format!("{} {}", stats.card, name),
            }];
            row.extend(
                BREAKDOWNS
                    .iter()
                    .map(|breakdown| format_tally(&stats[*breakdown])),
            );
            row.push(format_delta(played_delta(stats)));
            row
        })
        .collect();
    let widths: Vec<usize> = header
        .iter()
        .enumerate()
        .map(|(column, title)| {
            rows.iter()
                .map(|row| js_length(row.get(column).map_or("", String::as_str)))
                .fold(js_length(title), usize::max)
        })
        .collect();
    let line = |cells: &[String]| -> String {
        cells
            .iter()
            .enumerate()
            .map(|(column, cell)| {
                let width = widths.get(column).copied().unwrap_or(0);
                if column == 0 {
                    pad_end(cell, width)
                } else {
                    pad_start(cell, width)
                }
            })
            .collect::<Vec<String>>()
            .join("  ")
            .trim_end()
            .to_string()
    };

    let mut lines: Vec<String> = vec![
        format!("Card win rates: {}.", describe_filter(&report.filter)),
        format!("{} games, {} decks.", report.games, report.decks),
        "Each cell is the win rate of the decks that held the card, with those games in brackets. A game counts once for".to_string(),
        "each deck that held the card, and a draw is a game not won. Played Δ is the played win rate minus the drawn-but-".to_string(),
        "not-played win rate, in points: drawn means the card reached the hand, in the opening hand or by a later draw.".to_string(),
        String::new(),
        line(&header),
    ];
    lines.extend(rows.iter().map(|row| line(row)));
    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Reading a record back (the import script, the AI run's own file)
// ---------------------------------------------------------------------------

fn is_record(value: Option<&Value>) -> Option<&Map<String, Value>> {
    // `typeof value === "object" && value !== null && !Array.isArray(value)`.
    value.and_then(Value::as_object)
}

/// `typeof value === "number" && Number.isInteger(value)`: any JSON number holding a whole number.
fn whole_number(value: &Value) -> Option<i64> {
    let number = value.as_f64()?;
    if number.is_finite() && number.fract() == 0.0 {
        Some(number as i64)
    } else {
        None
    }
}

fn ids(value: Option<&Value>, at: &str) -> Result<Vec<String>, EngineError> {
    let list = value.and_then(Value::as_array).filter(|items| {
        items
            .iter()
            .all(|item| item.as_str().is_some_and(|id| !id.is_empty()))
    });
    match list {
        Some(items) => Ok(items
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect()),
        None => Err(EngineError::new(format!("{at} is not a list of card ids"))),
    }
}

fn one_of<T: Copy + std::fmt::Display>(
    value: Option<&Value>,
    allowed: &[T],
    at: &str,
) -> Result<T, EngineError> {
    if let Some(text) = value.and_then(Value::as_str)
        && let Some(found) = allowed.iter().find(|candidate| candidate.to_string() == text)
    {
        return Ok(*found);
    }
    let names: Vec<String> = allowed.iter().map(T::to_string).collect();
    Err(EngineError::new(format!(
        "{at} is not one of {}",
        names.join(", ")
    )))
}

fn text(value: Option<&Value>, at: &str) -> Result<String, EngineError> {
    match value.and_then(Value::as_str) {
        Some(text) if !text.is_empty() => Ok(text.to_string()),
        _ => Err(EngineError::new(format!("{at} is not a non-empty string"))),
    }
}

fn seat_summary(value: Option<&Value>, at: &str) -> Result<SeatSummary, EngineError> {
    let Some(record) = is_record(value) else {
        return Err(EngineError::new(format!("{at} is not a seat summary")));
    };
    let mut summary = SeatSummary {
        deck: ids(record.get("deck"), &format!("{at}.deck"))?,
        opening: ids(record.get("opening"), &format!("{at}.opening"))?,
        drawn: ids(record.get("drawn"), &format!("{at}.drawn"))?,
        played: ids(record.get("played"), &format!("{at}.played"))?,
        played_turns: None,
    };
    if let Some(Value::Array(turns)) = record.get("playedTurns") {
        let mut out: Vec<i32> = Vec::with_capacity(turns.len());
        for (idx, turn) in turns.iter().enumerate() {
            match whole_number(turn)
                .filter(|t| *t >= 1)
                .and_then(|t| i32::try_from(t).ok())
            {
                Some(turn) => out.push(turn),
                None => {
                    return Err(EngineError::new(format!(
                        "{at}.playedTurns[{idx}] is not a positive whole number"
                    )));
                }
            }
        }
        summary.played_turns = Some(out);
    }
    Ok(summary)
}

/// A record read back from JSON, checked field by field. Refuses with an error naming the first bad
/// field (TS threw it), in TS's order.
pub fn parse_game_record(value: &Value) -> Result<GameRecord, EngineError> {
    let Some(record) = value.as_object() else {
        return Err(EngineError::new("a game record is an object"));
    };
    let Some(pilots) = is_record(record.get("pilots")) else {
        return Err(EngineError::new("pilots is not an object"));
    };
    let Some(game) = is_record(record.get("game")) else {
        return Err(EngineError::new("game is not an object"));
    };
    let Some(seats) = is_record(game.get("seats")) else {
        return Err(EngineError::new("game.seats is not an object"));
    };
    let Some(turns) = game
        .get("turns")
        .and_then(whole_number)
        .filter(|turns| *turns >= 0)
        .and_then(|turns| i32::try_from(turns).ok())
    else {
        return Err(EngineError::new("game.turns is not a whole number"));
    };
    // Struct fields are evaluated in the order written, which is TS's object literal's.
    Ok(GameRecord {
        id: text(record.get("id"), "id")?,
        source: one_of(record.get("source"), GAME_SOURCES, "source")?,
        mode: one_of(record.get("mode"), GAME_MODES, "mode")?,
        patch: text(record.get("patch"), "patch")?,
        pilots: PerPlayer {
            p1: one_of(pilots.get("p1"), PILOTS, "pilots.p1")?,
            p2: one_of(pilots.get("p2"), PILOTS, "pilots.p2")?,
        },
        game: GameSummary {
            first: one_of(game.get("first"), &PLAYER_IDS, "game.first")?,
            winner: one_of(game.get("winner"), Winner::ALL, "game.winner")?,
            reason: one_of(game.get("reason"), GAME_OVER_REASONS, "game.reason")?,
            turns,
            seats: PerPlayer {
                p1: seat_summary(seats.get("p1"), "game.seats.p1")?,
                p2: seat_summary(seats.get("p2"), "game.seats.p2")?,
            },
        },
    })
}

/// Records written one JSON object per line, as a development run writes them. Blank lines are skipped.
pub fn parse_game_record_lines(contents: &str) -> Result<Vec<GameRecord>, EngineError> {
    let mut records: Vec<GameRecord> = Vec::new();
    for (index, line) in contents.split('\n').enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let parsed = serde_json::from_str::<Value>(line)
            .map_err(|error| error.to_string())
            .and_then(|value| parse_game_record(&value).map_err(|error| error.message));
        match parsed {
            Ok(record) => records.push(record),
            Err(message) => {
                return Err(EngineError::new(format!("line {}: {}", index + 1, message)));
            }
        }
    }
    Ok(records)
}

// SPEC §9.11, R376–R378: card win rates read off game records. The records here are written by hand,
// so each figure can be counted on the fingers; the engine's `summarize_game` (game_summary's tests)
// and the two writers (the server's game-records tests, the AI's dev-run tests) prove the records
// themselves.
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn list(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    /// TS `seat()`: deck a, b, c and nothing else.
    fn seat_default() -> SeatSummary {
        seat(&["a", "b", "c"], &[], &[], &[])
    }

    fn seat(deck: &[&str], opening: &[&str], drawn: &[&str], played: &[&str]) -> SeatSummary {
        SeatSummary {
            deck: list(deck),
            opening: list(opening),
            drawn: list(drawn),
            played: list(played),
            played_turns: None,
        }
    }

    /// TS `record({ winner })` with nothing else given.
    fn record_won_by(winner: Winner) -> GameRecord {
        GameRecord {
            id: "m1".to_string(),
            source: GameSource::Live,
            mode: GameMode::Bo1,
            patch: "v0.1.1".to_string(),
            pilots: PerPlayer::new(Pilot::Human, Pilot::Human),
            game: GameSummary {
                first: PlayerId::P1,
                winner,
                reason: if winner == Winner::Draw {
                    GameOverReason::TurnCap
                } else {
                    GameOverReason::HeroDeath
                },
                turns: 12,
                seats: PerPlayer::new(seat_default(), seat_default()),
            },
        }
    }

    fn record() -> GameRecord {
        record_won_by(Winner::P1)
    }

    fn game(
        winner: Winner,
        reason: GameOverReason,
        turns: i32,
        p1: SeatSummary,
        p2: SeatSummary,
    ) -> GameSummary {
        GameSummary {
            first: PlayerId::P1,
            winner,
            reason,
            turns,
            seats: PerPlayer::new(p1, p2),
        }
    }

    fn stats_of<'a>(report: &'a CardStatsReport, card: &str) -> &'a CardStats {
        report
            .cards
            .iter()
            .find(|stats| stats.card == card)
            .unwrap_or_else(|| panic!("no entry for {card}"))
    }

    fn tally(games: i32, wins: i32, draws: i32) -> Tally {
        Tally { games, wins, draws }
    }

    fn live() -> CardStatsFilter {
        DEFAULT_CARD_STATS_FILTER.clone()
    }

    /// `line.split(/\s{2,}/)`: the cells of a table line, split on runs of two or more spaces.
    fn split_wide(line: &str) -> Vec<String> {
        let mut cells: Vec<String> = Vec::new();
        let mut cell = String::new();
        let mut run = String::new();
        for character in line.chars() {
            if character.is_whitespace() {
                run.push(character);
                continue;
            }
            if run.chars().count() >= 2 {
                cells.push(std::mem::take(&mut cell));
            } else {
                cell.push_str(&run);
            }
            run.clear();
            cell.push(character);
        }
        if run.chars().count() >= 2 {
            cells.push(std::mem::take(&mut cell));
        } else {
            cell.push_str(&run);
        }
        cells.push(cell);
        cells
    }

    #[test]
    fn game_over_reasons_are_exhaustive() {
        assert_eq!(GAME_OVER_REASONS, GameOverReason::ALL);
    }

    mod card_win_rates {
        use super::*;

        #[test]
        fn r377_counts_every_breakdown_over_the_decks_that_held_the_card_a_mirror_once_per_deck() {
            let games = vec![
                // p1 wins going first. p1 opens with a, draws b and plays a; c stays in its library.
                GameRecord {
                    id: "g1".to_string(),
                    game: game(
                        Winner::P1,
                        GameOverReason::HeroDeath,
                        9,
                        seat(&["a", "b", "c"], &["a"], &["b"], &["a"]),
                        // p2 holds a too: the mirror counts it once more, as a loss going second.
                        seat(&["a", "d"], &["core-t-coin", "d"], &["a"], &["d"]),
                    ),
                    ..record()
                },
                // p2 wins going second with a in its opening hand, played.
                GameRecord {
                    id: "g2".to_string(),
                    game: game(
                        Winner::P2,
                        GameOverReason::Concede,
                        4,
                        seat(&["b", "c"], &["b", "c"], &[], &["c"]),
                        seat(&["a", "b"], &["a"], &[], &["a", "a"]),
                    ),
                    ..record_won_by(Winner::P2)
                },
                // A draw: both decks hold b, drawn and never played.
                GameRecord {
                    id: "g3".to_string(),
                    game: game(
                        Winner::Draw,
                        GameOverReason::TurnCap,
                        30,
                        seat(&["b"], &[], &["b"], &[]),
                        seat(&["b"], &["b"], &[], &[]),
                    ),
                    ..record()
                },
            ];

            let report = card_stats(&games, &live());
            assert_eq!(report.games, 3);
            assert_eq!(report.decks, 6);
            let cards: Vec<&str> = report.cards.iter().map(|stats| stats.card.as_str()).collect();
            assert_eq!(cards, ["a", "b", "c", "d"]);

            let a = stats_of(&report, "a");
            assert_eq!(a.in_deck, tally(3, 2, 0));
            assert_eq!(a.opening_hand, tally(2, 2, 0));
            assert_eq!(a.going_first, tally(1, 1, 0));
            assert_eq!(a.going_second, tally(2, 1, 0));
            // Played twice in g2 is still one game.
            assert_eq!(a.played, tally(2, 2, 0));
            // g1's p2 drew a and never played it.
            assert_eq!(a.drawn_not_played, tally(1, 0, 0));

            let b = stats_of(&report, "b");
            assert_eq!(b.in_deck, tally(5, 2, 2));
            assert_eq!(b.opening_hand, tally(2, 0, 1));
            assert_eq!(b.played, tally(0, 0, 0));
            // g1 p1 (drawn), g2 p1 (opening), g3 both: never played. g2 p2 held b and never drew it.
            assert_eq!(b.drawn_not_played, tally(4, 1, 2));

            // c never reached g1's hand, so it is in neither side of the delta there.
            let c = stats_of(&report, "c");
            assert_eq!(c.in_deck.games, 2);
            assert_eq!(c.played, tally(1, 0, 0));
            assert_eq!(c.drawn_not_played, tally(0, 0, 0));

            // The Coin was in an opening hand and is in no deck, so it has no entry.
            assert!(!report.cards.iter().any(|stats| stats.card == "core-t-coin"));
        }

        #[test]
        fn r377_puts_a_card_played_without_being_drawn_on_the_played_side_never_the_baseline() {
            let made = GameRecord {
                game: game(
                    Winner::P1,
                    GameOverReason::HeroDeath,
                    8,
                    seat(&["a"], &[], &[], &["a"]),
                    seat(&["z"], &[], &[], &[]),
                ),
                ..record()
            };
            let report = card_stats(&[made], &live());
            let a = stats_of(&report, "a");
            assert_eq!(a.played.games, 1);
            assert_eq!(a.drawn_not_played.games, 0);
        }

        #[test]
        fn r377_defines_the_played_delta_against_the_drawn_but_not_played_baseline_in_points() {
            let stats = CardStats {
                card: "a".to_string(),
                in_deck: tally(10, 5, 0),
                opening_hand: tally(0, 0, 0),
                going_first: tally(5, 3, 0),
                going_second: tally(5, 2, 0),
                played: tally(4, 3, 0),
                drawn_not_played: tally(5, 2, 1),
            };
            assert_eq!(win_rate(&stats.played), Some(0.75));
            assert_eq!(win_rate(&stats.drawn_not_played), Some(0.4));
            let delta = played_delta(&stats).unwrap();
            assert!((delta - 0.35).abs() < 0.005);
            assert_eq!(format_delta(played_delta(&stats)), "+35.0");
            assert_eq!(format_delta(Some(-0.082)), "-8.2");
            assert_eq!(win_rate(&stats.opening_hand), None);
            assert_eq!(
                played_delta(&CardStats {
                    drawn_not_played: tally(0, 0, 0),
                    ..stats.clone()
                }),
                None
            );
            assert_eq!(format_delta(None), "—");
        }

        #[test]
        fn r378_reads_live_games_only_unless_a_development_run_is_asked_for_by_name() {
            let records = vec![
                GameRecord {
                    id: "live-1".to_string(),
                    source: GameSource::Live,
                    ..record_won_by(Winner::P1)
                },
                GameRecord {
                    id: "dev-1".to_string(),
                    source: GameSource::Dev,
                    pilots: PerPlayer::new(Pilot::Ai, Pilot::Ai),
                    ..record_won_by(Winner::P2)
                },
                GameRecord {
                    id: "dev-2".to_string(),
                    source: GameSource::Dev,
                    pilots: PerPlayer::new(Pilot::Ai, Pilot::Ai),
                    ..record_won_by(Winner::P2)
                },
            ];
            assert_eq!(DEFAULT_CARD_STATS_FILTER.source, SourceFilter::Live);
            assert_eq!(card_stats(&records, &DEFAULT_CARD_STATS_FILTER).games, 1);
            assert_eq!(
                stats_of(&card_stats(&records, &DEFAULT_CARD_STATS_FILTER), "a")
                    .in_deck
                    .wins,
                1
            );
            let dev = CardStatsFilter {
                source: SourceFilter::Dev,
                ..live()
            };
            assert_eq!(card_stats(&records, &dev).games, 2);
            let all = CardStatsFilter {
                source: SourceFilter::All,
                ..live()
            };
            assert_eq!(card_stats(&records, &all).games, 3);
        }

        #[test]
        fn r377_filters_by_any_combination_of_match_type_patch_and_pilot() {
            let records = vec![
                GameRecord {
                    id: "1".to_string(),
                    mode: GameMode::Bo1,
                    patch: "v0.1.1".to_string(),
                    ..record()
                },
                GameRecord {
                    id: "2".to_string(),
                    mode: GameMode::Random,
                    patch: "v0.1.1".to_string(),
                    ..record()
                },
                GameRecord {
                    id: "3".to_string(),
                    mode: GameMode::Bo3,
                    patch: "v0.2.5".to_string(),
                    ..record()
                },
                // A practice-shaped record: one human seat, one AI seat. The AI's deck holds only z.
                GameRecord {
                    id: "4".to_string(),
                    mode: GameMode::Random,
                    patch: "v0.2.5".to_string(),
                    pilots: PerPlayer::new(Pilot::Human, Pilot::Ai),
                    game: game(
                        Winner::P2,
                        GameOverReason::HeroDeath,
                        10,
                        seat_default(),
                        seat(&["z"], &[], &[], &[]),
                    ),
                    ..record()
                },
            ];
            let games = |mode: Option<GameMode>, patch: Option<&str>| -> i32 {
                let filter = CardStatsFilter {
                    mode,
                    patch: patch.map(str::to_string),
                    ..live()
                };
                card_stats(&records, &filter).games
            };

            assert_eq!(games(None, None), 4);
            assert_eq!(games(Some(GameMode::Random), None), 2);
            assert_eq!(games(None, Some("v0.2.5")), 2);
            assert_eq!(games(Some(GameMode::Random), Some("v0.2.5")), 1);
            assert_eq!(games(Some(GameMode::Bo1), Some("v0.2.5")), 0);

            // Pilots count seats: record 4's human seat holds a, its AI seat z.
            let by_pilot = |pilot: PilotFilter| card_stats(&records, &CardStatsFilter { pilot, ..live() });
            let human = by_pilot(PilotFilter::Human);
            assert_eq!(human.decks, 7);
            let human_cards: Vec<&str> = human.cards.iter().map(|stats| stats.card.as_str()).collect();
            assert_eq!(human_cards, ["a", "b", "c"]);
            let ai = by_pilot(PilotFilter::Ai);
            assert_eq!(ai.games, 1);
            assert_eq!(ai.decks, 1);
            let ai_cards: Vec<&str> = ai.cards.iter().map(|stats| stats.card.as_str()).collect();
            assert_eq!(ai_cards, ["z"]);
            assert_eq!(stats_of(&ai, "z").in_deck, tally(1, 1, 0));
            let unified = by_pilot(PilotFilter::Unified);
            assert_eq!(unified.decks, 8);
            assert_eq!(stats_of(&unified, "a").in_deck.games, 7);
        }

        #[test]
        fn r377_shows_every_breakdown_for_each_card_with_its_games_beside_every_win_rate() {
            let report = card_stats(
                &[GameRecord {
                    game: game(
                        Winner::P1,
                        GameOverReason::HeroDeath,
                        9,
                        seat(&["a"], &["a"], &[], &["a"]),
                        seat(&["b"], &[], &["b"], &[]),
                    ),
                    ..record()
                }],
                &CardStatsFilter {
                    mode: Some(GameMode::Bo1),
                    patch: Some("v0.1.1".to_string()),
                    pilot: PilotFilter::Human,
                    ..live()
                },
            );
            let names: IndexMap<&str, &str> = [("a", "Alpha")].into_iter().collect();
            let text = format_card_stats(&report, |card| names.get(card).map(|name| name.to_string()));
            let lines: Vec<&str> = text.split('\n').collect();

            assert_eq!(
                lines[0],
                "Card win rates: live games, Best of 1, patch v0.1.1, human pilots."
            );
            assert_eq!(lines[1], "1 games, 2 decks.");
            // The table starts after the blank line under the legend.
            let blank = lines.iter().position(|line| line.is_empty()).unwrap();
            let heading = lines[blank + 1];
            assert!(heading.starts_with("Card "));
            for breakdown in BREAKDOWNS {
                assert!(heading.contains(&BREAKDOWN_TITLES[*breakdown]));
            }
            assert!(heading.contains("Played Δ"));

            let alpha = lines.iter().find(|line| line.starts_with("a Alpha")).unwrap();
            // In deck, opening hand, going first, going second, played, drawn-not-played; then the delta.
            assert_eq!(
                split_wide(alpha)[1..],
                [
                    "100.0% (1)",
                    "100.0% (1)",
                    "100.0% (1)",
                    "— (0)",
                    "100.0% (1)",
                    "— (0)",
                    "—"
                ]
            );
            let beta = lines.iter().find(|line| line.starts_with("b ")).unwrap();
            assert_eq!(
                split_wide(beta)[1..],
                ["0.0% (1)", "— (0)", "— (0)", "0.0% (1)", "— (0)", "0.0% (1)", "—"]
            );

            assert_eq!(format_tally(&tally(3, 1, 1)), "33.3% (3)");
            assert_eq!(
                describe_filter(&CardStatsFilter {
                    source: SourceFilter::All,
                    mode: None,
                    patch: None,
                    pilot: PilotFilter::Unified
                }),
                "live games and AI development runs, every mode, every patch, human and AI pilots (unified)"
            );
            assert_eq!(
                describe_filter(&CardStatsFilter {
                    source: SourceFilter::Dev,
                    mode: Some(GameMode::Random),
                    patch: Some("v0.2.5".to_string()),
                    pilot: PilotFilter::Ai
                }),
                "AI development runs, All Random, patch v0.2.5, AI pilots"
            );
        }
    }

    mod reading_a_record_back {
        use super::*;

        /// `{ ...value, [key]: replacement }` on a JSON object.
        fn with(value: &Value, key: &str, replacement: Value) -> Value {
            let mut out = value.clone();
            out.as_object_mut().unwrap().insert(key.to_string(), replacement);
            out
        }

        fn refusal(value: &Value) -> String {
            parse_game_record(value).unwrap_err().message
        }

        #[test]
        fn r376_reads_back_what_was_written_and_refuses_a_record_with_a_bad_field() {
            let written = GameRecord {
                source: GameSource::Dev,
                pilots: PerPlayer::new(Pilot::Ai, Pilot::Ai),
                ..record()
            };
            let json = serde_json::to_value(&written).unwrap();
            assert_eq!(parse_game_record(&json).unwrap(), written);

            assert!(
                refusal(&with(&json, "source", json!("practice"))).contains("source is not one of live, dev")
            );
            assert!(refusal(&with(&json, "mode", json!("bo5"))).contains("mode"));
            assert!(refusal(&with(&json, "patch", json!(""))).contains("patch"));
            assert!(
                refusal(&with(&json, "pilots", json!({ "p1": "ai", "p2": "robot" }))).contains("pilots.p2")
            );
            let game = &json["game"];
            assert!(
                refusal(&with(&json, "game", with(game, "reason", json!("rage-quit"))))
                    .contains("game.reason")
            );
            assert!(refusal(&with(&json, "game", with(game, "turns", json!(-1)))).contains("game.turns"));
            let bad_seat = with(
                &serde_json::to_value(seat_default()).unwrap(),
                "played",
                json!([7]),
            );
            let seats = with(&game["seats"], "p2", bad_seat);
            assert!(
                refusal(&with(&json, "game", with(game, "seats", seats))).contains("game.seats.p2.played")
            );
            assert!(refusal(&json!([json])).contains("an object"));
        }

        #[test]
        fn reads_one_record_per_line_skipping_blank_lines_and_naming_the_line_it_cannot_read() {
            let one = GameRecord {
                id: "x".to_string(),
                ..record()
            };
            let two = GameRecord {
                id: "y".to_string(),
                ..record()
            };
            let one_line = serde_json::to_string(&one).unwrap();
            let two_line = serde_json::to_string(&two).unwrap();
            assert_eq!(
                parse_game_record_lines(&format!("{one_line}\n\n{two_line}\n")).unwrap(),
                vec![one, two]
            );
            assert!(
                parse_game_record_lines(&format!("{one_line}\n{{\"id\":\"z\"}}\n"))
                    .unwrap_err()
                    .message
                    .starts_with("line 2: ")
            );
            assert!(
                parse_game_record_lines("not json")
                    .unwrap_err()
                    .message
                    .starts_with("line 1: ")
            );
        }
    }

    #[test]
    fn to_fixed_rounds_as_js_does() {
        // JS breaks a tie towards the larger magnitude, where Rust's formatter goes to even.
        assert_eq!(to_fixed(0.25, 1), "0.3");
        assert_eq!(to_fixed(12.5, 0), "13");
        assert_eq!(to_fixed(99.95, 1), "100.0");
        assert_eq!(to_fixed(-8.2, 1), "-8.2");
        assert_eq!(to_fixed(-0.01, 1), "-0.0");
        assert_eq!(to_fixed(0.0, 1), "0.0");
        assert_eq!(to_fixed(100.0 / 3.0, 1), "33.3");
    }
}
