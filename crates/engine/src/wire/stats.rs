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
//! (SURFACE §4.1). The web's TS copy, which no screen read, went in #900.

use std::ops::Index;

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::config::{STATS_INTERVAL_Z, STATS_SHARE_PRECISION};
use crate::state::EngineError;
use crate::wire::catalog_types::{PLAYER_IDS, PerPlayer, PlayerId};
use crate::wire::codes::utf16_len;
use crate::wire::events::{GameOverReason, Winner};
use crate::wire::string_union;

pub mod window;
pub use window::*;

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
    GameOverReason::VotedOut,
    GameOverReason::BothVotedOut,
    GameOverReason::Fled,
    GameOverReason::FledDecisive,
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
    /// Its hand once both mulligans had resolved, as the first turn began: the cards it kept and its
    /// replacements (§2.1). Empty when the game ended before a turn began. R2089000: dealt cards
    /// alone, so The Coin and every other Created card (R943) are left out, as in `drawn` and `played`.
    pub opening: Vec<String>,
    /// Every card it drew into its hand after that, in order. A card burned or cast on its draw never reached the hand.
    pub drawn: Vec<String>,
    /// Every card it played from its hand, in order: a cast (§2.4, R70) is not a play from hand.
    pub played: Vec<String>,
    /// Turn number (1-indexed) each card in `played` was played on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub played_turns: Option<Vec<i32>>,
    /// R1147000: whether each card in `played` was Radiant as played, in the same order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub played_radiant: Option<Vec<bool>>,
    /// R1256000: draws from an empty library this seat took.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fatigue_draws: Option<i32>,
    /// R1256000: the nominal pre-Armor hits those draws dealt (their `amount`s added).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fatigue_damage: Option<i32>,
    /// R1256000: Traps this seat set face-down from its hand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub traps_set: Option<i32>,
    /// R1256000: Traps of this seat's that fired, whoever's effect fired them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub traps_fired: Option<i32>,
    /// R1256000: position switches by units this seat controlled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub position_switches: Option<i32>,
    /// R1256000: cards of this seat's that an effect made Radiant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub radiant_upgrades: Option<i32>,
    /// R2089000: the Created cards this seat made that were used, each instance once, at its first use
    /// by either seat: played from a hand, cast or put onto a field. Absent when there were none, and
    /// from every record written before the change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub created_used: Option<Vec<CreatedUse>>,
    /// R2089000: the Created cards this seat made that were never used, by catalog id, one per
    /// instance: the baseline of `created_used`. Absent when there were none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub created_unused: Option<Vec<String>>,
    /// R2089000: the Created cards this seat played from its hand, whoever made them, every play, as
    /// `played` keeps the dealt ones; the quests count them with `played` (§14.9). Absent when none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub created_played: Option<Vec<CreatedUse>>,
}

/// R2089000: one use of a Created card: its catalog id, the turn (§2.5's counter) and whether it was
/// Radiant then (R1147000).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CreatedUse {
    pub card: String,
    pub turn: i32,
    pub radiant: bool,
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

/// R1284000: the start of a forced run's seed series. `cargo jackioh stats --force <id>`
/// files under `dev-force-<id>`, for `stats compare` and `stats report`, never for
/// `stats-import`.
pub const FORCED_SERIES_PREFIX: &str = "dev-force-";

/// R1284000: the forced series a development record was filed under, or `None` when it is
/// not a forced run's. A development id is `dev:<patch>:<series>:<n>`, so the series is the
/// middle; a record whose id is not shaped like one is not forced. The patch segment is not
/// compared to the record's patch, so a mistyped patch cannot hide a forced series.
pub fn forced_series(record: &GameRecord) -> Option<&str> {
    let rest = record.id.strip_prefix(DEV_RECORD_ID_PREFIX)?;
    let (_, after_patch) = rest.split_once(':')?;
    let (series, _) = after_patch.rsplit_once(':')?;
    series.starts_with(FORCED_SERIES_PREFIX).then_some(series)
}

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
    /// R1367000: the build that played the game. Absent from a record written before it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub build: Option<RecordBuild>,
}

/// R1367000: the build a game was played on, so an AI change can be split out of a patch's figures
/// without splitting the patch.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RecordBuild {
    /// The git commit the server or tool that played the game was built from.
    pub commit: String,
    /// The AI generation (`crates/ai/generation.json`) that build holds, when the AI played a seat.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ai_generation: Option<i32>,
}

impl RecordBuild {
    /// R1367000: the build of a game played by a build of `commit` holding AI generation
    /// `generation`, which it names only when the AI played one of `pilots`. `None` without a commit.
    pub fn of(
        commit: Option<&str>,
        generation: Option<i32>,
        pilots: &PerPlayer<Pilot>,
    ) -> Option<RecordBuild> {
        let commit = commit.filter(|commit| !commit.is_empty())?;
        let ai_played = PLAYER_IDS.into_iter().any(|seat| pilots[seat] == Pilot::Ai);
        Some(RecordBuild {
            commit: commit.to_string(),
            ai_generation: generation.filter(|_| ai_played),
        })
    }
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

impl Tally {
    /// No games: a breakdown nothing counted.
    pub fn is_empty(&self) -> bool {
        self.games == 0
    }
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
        PlayedRadiant = "playedRadiant",
        PlayedNormal = "playedNormal",
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
    /// R1147000: played Radiant at least once.
    #[serde(default)]
    pub played_radiant: Tally,
    /// R1147000: played, never Radiant.
    #[serde(default)]
    pub played_normal: Tally,
    /// R2089000: a seat made a Created copy of the card and used one. The four Created tallies are
    /// kept beside the deck's, under the same key, and read apart from them (`card_stats_of`'s
    /// `created`); each is absent while empty, so a deck row reads and is stored as before.
    #[serde(default, skip_serializing_if = "Tally::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Tally>", optional))]
    pub created_played: Tally,
    /// R2089000: a seat made Created copies of the card and used none.
    #[serde(default, skip_serializing_if = "Tally::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Tally>", optional))]
    pub created_not_played: Tally,
    /// R2089000: of `created_played`, a copy was Radiant at its first use.
    #[serde(default, skip_serializing_if = "Tally::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Tally>", optional))]
    pub created_played_radiant: Tally,
    /// R2089000: of `created_played`, none was.
    #[serde(default, skip_serializing_if = "Tally::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Tally>", optional))]
    pub created_played_normal: Tally,
}

/// R2089000: a Created card's figures, by its catalog id: over the seats that made a Created copy of
/// it, each seat once per game however many copies it made. `createdPlayed` against its baseline,
/// `createdNotPlayed`, makes the Created played delta (`created_played_delta`), and the used are split
/// by the face at first use as R1147000 splits a deck's plays.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CreatedStats {
    pub card: String,
    pub created_played: Tally,
    pub created_not_played: Tally,
    pub created_played_radiant: Tally,
    pub created_played_normal: Tally,
}

impl CreatedStats {
    /// The Created figures a card's totals hold.
    pub fn of(stats: &CardStats) -> CreatedStats {
        CreatedStats {
            card: stats.card.clone(),
            created_played: stats.created_played,
            created_not_played: stats.created_not_played,
            created_played_radiant: stats.created_played_radiant,
            created_played_normal: stats.created_played_normal,
        }
    }

    /// The seats it counts: every one that made a copy, used or not.
    pub fn games(&self) -> i32 {
        self.created_played.games + self.created_not_played.games
    }
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
            Breakdown::PlayedRadiant => &self.played_radiant,
            Breakdown::PlayedNormal => &self.played_normal,
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
    /// R2089000: one entry per card some counted seat made a Created copy of, by catalog id, apart
    /// from `cards`. Absent from a report written before it.
    #[serde(default)]
    pub created: Vec<CreatedStats>,
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
        played_radiant: empty_tally(),
        played_normal: empty_tally(),
        created_played: empty_tally(),
        created_not_played: empty_tally(),
        created_played_radiant: empty_tally(),
        created_played_normal: empty_tally(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    Win,
    Draw,
    Loss,
}

/// R1843004: the seat's result as the ladder reads it (`game_over::seat_outcome`): a fled game is the
/// fleer's win and the other seat's draw.
fn outcome_for(game: &GameSummary, seat: PlayerId) -> Outcome {
    match crate::game_over::seat_outcome(game.winner, game.reason, seat) {
        crate::game_over::SeatOutcome::Win => Outcome::Win,
        crate::game_over::SeatOutcome::Draw => Outcome::Draw,
        crate::game_over::SeatOutcome::Loss => Outcome::Loss,
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
/// id for the decks that held it, and a card no counted deck held has no entry. `cards` lists the ids
/// in catalog-id order. R2089000: a Created copy of a card is no dealt card, so it counts in `created`,
/// for the seat that made it, and never as the deck's card (a record written before the change still
/// counts the copies its seat played as the deck's).
///
/// R1044000: the records are folded into totals and the report read off them, as the server reads
/// the totals it keeps, so the two cannot count differently.
///
/// TS's `filter` defaulted to `DEFAULT_CARD_STATS_FILTER`; a caller passes `&DEFAULT_CARD_STATS_FILTER`.
pub fn card_stats(records: &[GameRecord], filter: &CardStatsFilter) -> CardStatsReport {
    let mut totals = StatsTotals::default();
    for record in records.iter().filter(|record| record_matches(record, filter)) {
        totals.fold(record, false);
    }
    card_stats_of(&totals, filter)
}

// ---------------------------------------------------------------------------
// The totals: the records folded (R1044000)
// ---------------------------------------------------------------------------

/// R1044000: a turn's or a co-played card's games and wins, as the card drill-down counts them: one
/// deck in one game, a draw a game not won.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct DrillCount {
    pub games: i64,
    pub wins: i64,
}

impl DrillCount {
    fn add(&mut self, other: &DrillCount) {
        self.games += other.games;
        self.wins += other.wins;
    }
}

/// R1044000: where two cards were first met in one deck, the records taken in id order: the record,
/// its seat (0 for p1, 1 for p2) and the other card's place among that seat's distinct cards. The
/// drill-down lists the co-played cards with the most games first and a tie in the order first met,
/// so the totals keep the earliest meeting. Ordered field by field, the id by its bytes, as the store
/// reads records (`order by id collate "C"`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FirstMet {
    pub record: String,
    pub seat: u8,
    pub index: u32,
}

impl FirstMet {
    /// Keeps the earlier of this meeting and `(record, seat, index)`.
    pub fn keep_earlier(&mut self, record: &str, seat: u8, index: u32) {
        if (record, seat, index) < (self.record.as_str(), self.seat, self.index) {
            *self = FirstMet {
                record: record.to_string(),
                seat,
                index,
            };
        }
    }
}

/// R1044000: a card's games and wins with one other card in its deck, and where the two first met.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CoCount {
    pub games: i64,
    pub wins: i64,
    pub first: FirstMet,
}

/// R1044000: one card's totals under one key: R377's breakdowns, and the drill-down's games by the
/// turn the card was played on and with each card that shared its deck.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CardTotal {
    pub stats: CardStats,
    pub turns: IndexMap<i32, DrillCount>,
    pub co: IndexMap<String, CoCount>,
}

impl CardTotal {
    pub fn new(card: &str) -> CardTotal {
        CardTotal {
            stats: empty_stats(card),
            turns: IndexMap::new(),
            co: IndexMap::new(),
        }
    }

    /// Adds `other`'s counts, each pair keeping its earlier first meeting.
    pub fn add(&mut self, other: &CardTotal) {
        add_stats(&mut self.stats, &other.stats);
        for (turn, count) in &other.turns {
            self.turns.entry(*turn).or_default().add(count);
        }
        for (card, count) in &other.co {
            match self.co.get_mut(card) {
                None => {
                    self.co.insert(card.clone(), count.clone());
                }
                Some(held) => {
                    held.games += count.games;
                    held.wins += count.wins;
                    held.first
                        .keep_earlier(&count.first.record, count.first.seat, count.first.index);
                }
            }
        }
    }
}

/// R1044000: the seats a card total gathers: one patch, source and mode, and the seat's pilot.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TotalsKey {
    pub patch: String,
    pub source: GameSource,
    pub mode: GameMode,
    pub pilot: Pilot,
}

/// R1044000: the records a record count gathers: one patch, source and mode, and both seats' pilots
/// (p1's, then p2's), from which a query's games and decks are read for any pilot filter.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RecordsKey {
    pub patch: String,
    pub source: GameSource,
    pub mode: GameMode,
    pub pilots: (Pilot, Pilot),
}

/// R1044000: where a decklist was first seen, the records taken in id order: the record and its seat.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FirstSeen {
    pub record: String,
    pub seat: u8,
}

/// R1044000 (R1454, R750000): one distinct decklist people took into a patch's live Best-of-1
/// games: the list as the first record held it, its sorted ids (which make two orderings one list),
/// its games and the games it won (a draw a game not won).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ListTotal {
    pub deck: Vec<String>,
    pub key: Vec<String>,
    pub games: i64,
    pub wins: i64,
    pub first: FirstSeen,
}

impl ListTotal {
    /// Adds `other`'s games and wins; the list keeps the order of whichever was seen first.
    pub fn add(&mut self, other: &ListTotal) {
        self.games += other.games;
        self.wins += other.wins;
        if other.first < self.first {
            self.deck = other.deck.clone();
            self.first = other.first.clone();
        }
    }
}

/// R1994000: one key's whole games, which the game figures are read from: who won from which seat,
/// how the games ended and how long they ran, and R1256000's counts, both seats added, over the
/// records that carry them. They are kept apart from the records' detail, so R1044001's purge
/// leaves them whole.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct GameTotal {
    pub games: i64,
    /// Games nobody won.
    pub draws: i64,
    /// Games the seat that took the first turn won.
    pub first_wins: i64,
    /// Games per ending; an ending no game had is absent.
    pub reasons: IndexMap<GameOverReason, i64>,
    /// Games per turn counter at the end (§2.5).
    pub turns: IndexMap<i32, i64>,
    /// Games whose records carry all six of R1256000's counts on both seats: the sums below are
    /// over these alone.
    pub counted: i64,
    /// Counted games with a draw from an empty library.
    pub fatigue_games: i64,
    pub fatigue_damage: i64,
    pub traps_set: i64,
    pub traps_fired: i64,
    pub position_switches: i64,
    pub radiant_upgrades: i64,
}

/// R1256000's counts of one game, both seats added.
#[derive(Clone, Copy, Default)]
struct GameCounts {
    fatigue_draws: i64,
    fatigue_damage: i64,
    traps_set: i64,
    traps_fired: i64,
    position_switches: i64,
    radiant_upgrades: i64,
}

/// R1256000: a game's counts, or `None` when either seat's record lacks one (written before them).
fn game_counts(game: &GameSummary) -> Option<GameCounts> {
    let mut sum = GameCounts::default();
    for seat in PLAYER_IDS {
        let seat = &game.seats[seat];
        sum.fatigue_draws += i64::from(seat.fatigue_draws?);
        sum.fatigue_damage += i64::from(seat.fatigue_damage?);
        sum.traps_set += i64::from(seat.traps_set?);
        sum.traps_fired += i64::from(seat.traps_fired?);
        sum.position_switches += i64::from(seat.position_switches?);
        sum.radiant_upgrades += i64::from(seat.radiant_upgrades?);
    }
    Some(sum)
}

impl GameTotal {
    /// R1994000: adds one game. A win is the seat the record names, so a fled game (R1843004) is the
    /// fleer's, as `stats report --games` always read it.
    fn count(&mut self, game: &GameSummary) {
        self.games += 1;
        self.draws += i64::from(game.winner == Winner::Draw);
        self.first_wins += i64::from(game.winner.player() == Some(game.first));
        *self.reasons.entry(game.reason).or_default() += 1;
        *self.turns.entry(game.turns).or_default() += 1;
        if let Some(counts) = game_counts(game) {
            self.counted += 1;
            self.fatigue_games += i64::from(counts.fatigue_draws > 0);
            self.fatigue_damage += counts.fatigue_damage;
            self.traps_set += counts.traps_set;
            self.traps_fired += counts.traps_fired;
            self.position_switches += counts.position_switches;
            self.radiant_upgrades += counts.radiant_upgrades;
        }
    }

    /// R1994000: adds `other`'s games, each ending's and each turn's entry by entry.
    pub fn add(&mut self, other: &GameTotal) {
        self.games += other.games;
        self.draws += other.draws;
        self.first_wins += other.first_wins;
        for (reason, games) in &other.reasons {
            *self.reasons.entry(*reason).or_default() += games;
        }
        for (turn, games) in &other.turns {
            *self.turns.entry(*turn).or_default() += games;
        }
        self.counted += other.counted;
        self.fatigue_games += other.fatigue_games;
        self.fatigue_damage += other.fatigue_damage;
        self.traps_set += other.traps_set;
        self.traps_fired += other.traps_fired;
        self.position_switches += other.position_switches;
        self.radiant_upgrades += other.radiant_upgrades;
    }
}

/// R1044000: records folded into the counts every card figure is read from. Adding is commutative
/// and associative, so records folded in any order, or in pieces and added, make the same totals.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StatsTotals {
    /// Records by patch, source, mode and both seats' pilots.
    pub records: IndexMap<RecordsKey, i64>,
    /// Each key's cards, by catalog id.
    pub cards: IndexMap<TotalsKey, IndexMap<String, CardTotal>>,
    /// Each patch's live Best-of-1 human decklists, by their sorted ids joined with commas.
    pub lists: IndexMap<String, IndexMap<String, ListTotal>>,
    /// R1994000: the whole games under each records key. A record folded before they were kept
    /// counts in `records` and not here.
    pub game_totals: IndexMap<RecordsKey, GameTotal>,
}

/// One breakdown added into another.
fn add_tally(into: &mut Tally, other: &Tally) {
    into.games += other.games;
    into.wins += other.wins;
    into.draws += other.draws;
}

/// Every breakdown of one card's figures added into another's.
pub fn add_stats(into: &mut CardStats, other: &CardStats) {
    add_tally(&mut into.in_deck, &other.in_deck);
    add_tally(&mut into.opening_hand, &other.opening_hand);
    add_tally(&mut into.going_first, &other.going_first);
    add_tally(&mut into.going_second, &other.going_second);
    add_tally(&mut into.played, &other.played);
    add_tally(&mut into.drawn_not_played, &other.drawn_not_played);
    add_tally(&mut into.played_radiant, &other.played_radiant);
    add_tally(&mut into.played_normal, &other.played_normal);
    add_tally(&mut into.created_played, &other.created_played);
    add_tally(&mut into.created_not_played, &other.created_not_played);
    add_tally(&mut into.created_played_radiant, &other.created_played_radiant);
    add_tally(&mut into.created_played_normal, &other.created_played_normal);
}

/// R2089000: one Created card's figures added into another's.
pub fn add_created(into: &mut CreatedStats, other: &CreatedStats) {
    add_tally(&mut into.created_played, &other.created_played);
    add_tally(&mut into.created_not_played, &other.created_not_played);
    add_tally(&mut into.created_played_radiant, &other.created_played_radiant);
    add_tally(&mut into.created_played_normal, &other.created_played_normal);
}

/// A seat's place in `PLAYER_IDS`, as `FirstMet` and `FirstSeen` keep it.
fn seat_index(seat: PlayerId) -> u8 {
    match seat {
        PlayerId::P1 => 0,
        PlayerId::P2 => 1,
    }
}

impl StatsTotals {
    /// R1044000: adds one record. Each seat's cards go under its own pilot's key, counted as R377
    /// counts them. With `drills`, also each card's games by the turn it was played on and with each
    /// card that shared its deck, which the card drill-down reads; without, R377's breakdowns and the
    /// decklists alone, which are all `card_stats` reads. R1994000: the whole game goes under the
    /// records key either way.
    pub fn fold(&mut self, record: &GameRecord, drills: bool) {
        let records_key = RecordsKey {
            patch: record.patch.clone(),
            source: record.source,
            mode: record.mode,
            pilots: (record.pilots.p1, record.pilots.p2),
        };
        *self.records.entry(records_key.clone()).or_default() += 1;
        self.game_totals
            .entry(records_key)
            .or_default()
            .count(&record.game);

        for seat in PLAYER_IDS {
            let pilot = record.pilots[seat];
            let summary = &record.game.seats[seat];
            let outcome = outcome_for(&record.game, seat);
            let won = outcome == Outcome::Win;
            let opening: IndexSet<&str> = summary.opening.iter().map(String::as_str).collect();
            let in_hand: IndexSet<&str> = summary
                .opening
                .iter()
                .chain(summary.drawn.iter())
                .map(String::as_str)
                .collect();
            let played: IndexSet<&str> = summary.played.iter().map(String::as_str).collect();
            let first = record.game.first == seat;
            // The turn of each play, read only when every play has one.
            let played_turns = summary
                .played_turns
                .as_ref()
                .filter(|turns| turns.len() == summary.played.len());
            // R1147000: cards played Radiant at least once, read only when every play has a face flag.
            let played_radiant: Option<IndexSet<&str>> = summary
                .played_radiant
                .as_ref()
                .filter(|flags| flags.len() == summary.played.len())
                .map(|flags| {
                    summary
                        .played
                        .iter()
                        .zip(flags.iter())
                        .filter_map(|(id, &radiant)| if radiant { Some(id.as_str()) } else { None })
                        .collect()
                });

            let key = TotalsKey {
                patch: record.patch.clone(),
                source: record.source,
                mode: record.mode,
                pilot,
            };
            let deck: IndexSet<&str> = summary.deck.iter().map(String::as_str).collect();
            // R2089000: the Created cards the seat made, each once: used (Radiant at a first use, or
            // not), and made but never used. A record written before them has neither.
            let used: IndexSet<&str> = summary
                .created_used
                .iter()
                .flatten()
                .map(|used| used.card.as_str())
                .collect();
            let used_radiant: IndexSet<&str> = summary
                .created_used
                .iter()
                .flatten()
                .filter(|used| used.radiant)
                .map(|used| used.card.as_str())
                .collect();
            let unused: IndexSet<&str> = summary
                .created_unused
                .iter()
                .flatten()
                .map(String::as_str)
                .filter(|card| !used.contains(card))
                .collect();
            // A seat with no cards adds no key, as the store keeps no row for it.
            let mut none = IndexMap::new();
            let cards = if deck.is_empty() && used.is_empty() && unused.is_empty() {
                &mut none
            } else {
                self.cards.entry(key).or_default()
            };
            for &card in &deck {
                let total = cards
                    .entry(card.to_string())
                    .or_insert_with(|| CardTotal::new(card));
                let stats = &mut total.stats;
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
                    if let Some(radiant) = &played_radiant {
                        count(
                            if radiant.contains(card) {
                                &mut stats.played_radiant
                            } else {
                                &mut stats.played_normal
                            },
                            outcome,
                        );
                    }
                } else if in_hand.contains(card) {
                    count(&mut stats.drawn_not_played, outcome);
                }
                if !drills {
                    continue;
                }

                // Each turn the card was played on counts the game once.
                let mut turns_played_in_game: IndexSet<i32> = IndexSet::new();
                if let Some(played_turns) = played_turns {
                    for (index, played) in summary.played.iter().enumerate() {
                        if played == card
                            && let Some(turn) = played_turns.get(index).copied()
                            && turn > 0
                        {
                            turns_played_in_game.insert(turn);
                        }
                    }
                }
                for turn in turns_played_in_game {
                    let tally = total.turns.entry(turn).or_default();
                    tally.games += 1;
                    tally.wins += i64::from(won);
                }

                for (index, &other) in deck.iter().enumerate() {
                    if other == card {
                        continue;
                    }
                    let index = u32::try_from(index).unwrap_or(u32::MAX);
                    match total.co.get_mut(other) {
                        None => {
                            let first = FirstMet {
                                record: record.id.clone(),
                                seat: seat_index(seat),
                                index,
                            };
                            total.co.insert(
                                other.to_string(),
                                CoCount {
                                    games: 1,
                                    wins: i64::from(won),
                                    first,
                                },
                            );
                        }
                        Some(held) => {
                            held.games += 1;
                            held.wins += i64::from(won);
                            held.first.keep_earlier(&record.id, seat_index(seat), index);
                        }
                    }
                }
            }

            for &card in &used {
                let stats = &mut cards
                    .entry(card.to_string())
                    .or_insert_with(|| CardTotal::new(card))
                    .stats;
                count(&mut stats.created_played, outcome);
                count(
                    if used_radiant.contains(card) {
                        &mut stats.created_played_radiant
                    } else {
                        &mut stats.created_played_normal
                    },
                    outcome,
                );
            }
            for &card in &unused {
                let stats = &mut cards
                    .entry(card.to_string())
                    .or_insert_with(|| CardTotal::new(card))
                    .stats;
                count(&mut stats.created_not_played, outcome);
            }

            // R1454, R750000: the lists people took into live Best-of-1 games.
            if record.source == GameSource::Live && record.mode == GameMode::Bo1 && pilot == Pilot::Human {
                let mut sorted = summary.deck.clone();
                sorted.sort();
                let joined = sorted.join(",");
                let list = ListTotal {
                    deck: summary.deck.clone(),
                    key: sorted,
                    games: 1,
                    wins: i64::from(won),
                    first: FirstSeen {
                        record: record.id.clone(),
                        seat: seat_index(seat),
                    },
                };
                let lists = self.lists.entry(record.patch.clone()).or_default();
                match lists.get_mut(&joined) {
                    None => {
                        lists.insert(joined, list);
                    }
                    Some(held) => held.add(&list),
                }
            }
        }
    }

    /// R1044000: adds `other`'s totals into these.
    pub fn add(&mut self, other: &StatsTotals) {
        for (key, records) in &other.records {
            *self.records.entry(key.clone()).or_default() += records;
        }
        for (key, cards) in &other.cards {
            let held = self.cards.entry(key.clone()).or_default();
            for (card, total) in cards {
                match held.get_mut(card) {
                    None => {
                        held.insert(card.clone(), total.clone());
                    }
                    Some(mine) => mine.add(total),
                }
            }
        }
        for (patch, lists) in &other.lists {
            let held = self.lists.entry(patch.clone()).or_default();
            for (key, list) in lists {
                match held.get_mut(key) {
                    None => {
                        held.insert(key.clone(), list.clone());
                    }
                    Some(mine) => mine.add(list),
                }
            }
        }
        for (key, total) in &other.game_totals {
            self.game_totals.entry(key.clone()).or_default().add(total);
        }
    }

    /// R1044000: the totals of the sources, mode (or every mode) and patch (or every patch) given,
    /// each card's turn and co-played counts only with `drills`, their whole games (R1994000), and
    /// no decklists (`decklists` reads those).
    pub fn select(
        &self,
        sources: &[GameSource],
        mode: Option<GameMode>,
        patch: Option<&str>,
        drills: bool,
    ) -> StatsTotals {
        let wanted = |key_patch: &str, source: GameSource, key_mode: GameMode| {
            sources.contains(&source)
                && mode.is_none_or(|mode| key_mode == mode)
                && patch.is_none_or(|patch| key_patch == patch)
        };
        let records = self
            .records
            .iter()
            .filter(|(key, _)| wanted(&key.patch, key.source, key.mode))
            .map(|(key, records)| (key.clone(), *records))
            .collect();
        let cards = self
            .cards
            .iter()
            .filter(|(key, _)| wanted(&key.patch, key.source, key.mode))
            .map(|(key, cards)| {
                let cards = cards
                    .iter()
                    .map(|(card, total)| {
                        let total = if drills {
                            total.clone()
                        } else {
                            CardTotal {
                                stats: total.stats.clone(),
                                turns: IndexMap::new(),
                                co: IndexMap::new(),
                            }
                        };
                        (card.clone(), total)
                    })
                    .collect();
                (key.clone(), cards)
            })
            .collect();
        let game_totals = self
            .game_totals
            .iter()
            .filter(|(key, _)| wanted(&key.patch, key.source, key.mode))
            .map(|(key, total)| (key.clone(), total.clone()))
            .collect();
        StatsTotals {
            records,
            cards,
            lists: IndexMap::new(),
            game_totals,
        }
    }

    /// R1044000 (R1454, R750000): a patch's live Best-of-1 human decklists, in the order the records
    /// first held them.
    pub fn decklists(&self, patch: &str) -> Vec<ListTotal> {
        let mut lists: Vec<ListTotal> = self
            .lists
            .get(patch)
            .map(|lists| lists.values().cloned().collect())
            .unwrap_or_default();
        lists.sort_by(|a, b| a.first.cmp(&b.first));
        lists
    }

    /// R654: a patch's live games, which the publication gate counts.
    pub fn live_games(&self, patch: &str) -> i64 {
        self.records
            .iter()
            .filter(|(key, _)| key.source == GameSource::Live && key.patch == patch)
            .map(|(_, records)| records)
            .sum()
    }

    /// R1973001: every record filed under `patch`, live and development alike.
    pub fn games(&self, patch: &str) -> i64 {
        self.records
            .iter()
            .filter(|(key, _)| key.patch == patch)
            .map(|(_, records)| records)
            .sum()
    }
}

/// Whether a pilot filter counts a seat with this pilot.
fn pilot_counts(filter: PilotFilter, pilot: Pilot) -> bool {
    filter == PilotFilter::Unified || pilot.as_str() == filter.as_str()
}

/// R377, R1044000: every card's breakdowns over the totals the filter reads, as `card_stats` counts
/// the records folded into them: the games are the records with a seat the filter counts, the decks
/// those seats, `cards` the cards some counted deck held, and (R2089000) `created` the cards some
/// counted seat made a Created copy of, each in catalog-id order.
pub fn card_stats_of(totals: &StatsTotals, filter: &CardStatsFilter) -> CardStatsReport {
    let sources = sources_of(filter.source);
    let wanted = |patch: &str, source: GameSource, mode: GameMode| {
        sources.contains(&source)
            && filter.mode.is_none_or(|wanted| mode == wanted)
            && filter.patch.as_ref().is_none_or(|wanted| patch == wanted)
    };
    let (mut games, mut decks) = (0i64, 0i64);
    for (key, records) in &totals.records {
        if !wanted(&key.patch, key.source, key.mode) {
            continue;
        }
        let seats = i64::from(pilot_counts(filter.pilot, key.pilots.0))
            + i64::from(pilot_counts(filter.pilot, key.pilots.1));
        if seats > 0 {
            games += records;
            decks += records * seats;
        }
    }

    let mut by_card: IndexMap<&str, CardStats> = IndexMap::new();
    for (key, cards) in &totals.cards {
        if !wanted(&key.patch, key.source, key.mode) || !pilot_counts(filter.pilot, key.pilot) {
            continue;
        }
        for (card, total) in cards {
            match by_card.get_mut(card.as_str()) {
                None => {
                    by_card.insert(card, total.stats.clone());
                }
                Some(held) => add_stats(held, &total.stats),
            }
        }
    }
    // R2089000: the Created figures apart from the deck's, and a deck row without them. A card no
    // counted deck held has no deck row.
    let mut cards: Vec<CardStats> = Vec::new();
    let mut created: Vec<CreatedStats> = Vec::new();
    for mut stats in by_card.into_values() {
        let made = CreatedStats::of(&stats);
        if made.games() > 0 {
            created.push(made);
        }
        if stats.in_deck.games > 0 {
            stats.created_played = Tally::default();
            stats.created_not_played = Tally::default();
            stats.created_played_radiant = Tally::default();
            stats.created_played_normal = Tally::default();
            cards.push(stats);
        }
    }
    // A stable sort by id (SURFACE §4.4.1; ids are ASCII, so `str::cmp` is JS's `<`).
    cards.sort_by(|a, b| a.card.cmp(&b.card));
    created.sort_by(|a, b| a.card.cmp(&b.card));
    CardStatsReport {
        filter: filter.clone(),
        games: i32::try_from(games).unwrap_or(i32::MAX),
        decks: i32::try_from(decks).unwrap_or(i32::MAX),
        cards,
        created,
    }
}

/// R1044000: one card's drill-down over the totals a query counts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardDrill {
    /// Games by the turn the card was played on, in turn order.
    pub by_turn: Vec<(i32, DrillCount)>,
    /// The cards that shared its deck, most games first (ties in the order first met), at most the
    /// `top` asked for.
    pub co_played: Vec<(String, DrillCount)>,
}

/// R978000, R1044000: every card's drill-down over the totals whose key `counts`, each turn's and
/// each other card's counts summed across those keys and each pair keeping its earliest meeting, so
/// a tie lists in the order the records first met it, as counting the records themselves did.
pub fn card_drills_of(
    totals: &StatsTotals,
    counts: impl Fn(&TotalsKey) -> bool,
    top: usize,
) -> IndexMap<String, CardDrill> {
    type Merged<'a> = (
        IndexMap<i32, DrillCount>,
        IndexMap<&'a str, (DrillCount, FirstMet)>,
    );
    let mut merged: IndexMap<&str, Merged> = IndexMap::new();
    for (key, cards) in &totals.cards {
        if !counts(key) {
            continue;
        }
        for (card, total) in cards {
            let (turns, co) = merged.entry(card.as_str()).or_default();
            for (turn, count) in &total.turns {
                turns.entry(*turn).or_default().add(count);
            }
            for (other, count) in &total.co {
                let pair = DrillCount {
                    games: count.games,
                    wins: count.wins,
                };
                match co.get_mut(other.as_str()) {
                    None => {
                        co.insert(other, (pair, count.first.clone()));
                    }
                    Some((held, first)) => {
                        held.add(&pair);
                        first.keep_earlier(&count.first.record, count.first.seat, count.first.index);
                    }
                }
            }
        }
    }
    merged
        .into_iter()
        .map(|(card, (turns, co))| {
            let mut by_turn: Vec<(i32, DrillCount)> = turns.into_iter().collect();
            by_turn.sort_by_key(|(turn, _)| *turn);
            let mut co_played: Vec<(&str, DrillCount, FirstMet)> = co
                .into_iter()
                .map(|(other, (count, first))| (other, count, first))
                .collect();
            co_played.sort_by(|a, b| b.1.games.cmp(&a.1.games).then_with(|| a.2.cmp(&b.2)));
            co_played.truncate(top);
            let co_played = co_played
                .into_iter()
                .map(|(other, count, _)| (other.to_string(), count))
                .collect();
            (card.to_string(), CardDrill { by_turn, co_played })
        })
        .collect()
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

/// R2427000: the decks that held the card and never had it in hand: in deck less played less
/// drawn but not played, for games, wins and draws alike. Read off the totals, never stored.
pub fn not_drawn(stats: &CardStats) -> Tally {
    Tally {
        games: stats.in_deck.games - stats.played.games - stats.drawn_not_played.games,
        wins: stats.in_deck.wins - stats.played.wins - stats.drawn_not_played.wins,
        draws: stats.in_deck.draws - stats.played.draws - stats.drawn_not_played.draws,
    }
}

/// R2427000: the drawn delta, the drawn win rate (played and drawn but not played together) minus
/// the never-drawn win rate, as a fraction. `None` when either side has no games; the public
/// page's floor is the server's (R654).
pub fn drawn_delta(stats: &CardStats) -> Option<f64> {
    let mut drawn = stats.played;
    add_tally(&mut drawn, &stats.drawn_not_played);
    let drawn = win_rate(&drawn)?;
    let baseline = win_rate(&not_drawn(stats))?;
    Some(drawn - baseline)
}

/// R2089000: the Created played delta, R377's over a Created card's used and never-used seats.
pub fn created_played_delta(stats: &CreatedStats) -> Option<f64> {
    let played = win_rate(&stats.created_played)?;
    let baseline = win_rate(&stats.created_not_played)?;
    Some(played - baseline)
}

// ---------------------------------------------------------------------------
// The game figures (R1994000)
// ---------------------------------------------------------------------------

/// The turns are cut into this many parts by their quartiles.
const QUARTERS: usize = 4;

/// R850000, R1994000: the 95% Wilson score interval of `wins` in `games`: the range a true win rate
/// can sit in and still give this result. It stays inside [0, 1] and is sound at 0 or every win,
/// where the normal approximation is not. No games is no information: [0, 1].
pub fn wilson_interval(wins: i64, games: i64) -> (f64, f64) {
    if games <= 0 {
        return (0.0, 1.0);
    }
    let n = games as f64;
    let p = wins as f64 / n;
    let z2 = STATS_INTERVAL_Z * STATS_INTERVAL_Z;
    let denominator = 1.0 + z2 / n;
    let center = (p + z2 / (2.0 * n)) / denominator;
    let half = STATS_INTERVAL_Z * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt() / denominator;
    ((center - half).max(0.0), (center + half).min(1.0))
}

/// R1994000: a share to three places (`STATS_SHARE_PRECISION`).
pub fn round_share(share: f64) -> f64 {
    (share * STATS_SHARE_PRECISION).round() / STATS_SHARE_PRECISION
}

/// R1994000: the figures of some games read whole, whichever of their seats a filter counts.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameFigures {
    /// How many games.
    pub games: i64,
    /// How many of them nobody won.
    pub draws: i64,
    /// How many the seat that took the first turn won.
    pub first_wins: i64,
    /// R2567000: `first_wins` over the decided games (`games - draws`), to three places, or null
    /// when every game was drawn.
    pub first_share: Option<f64>,
    /// The low end of the 95% Wilson interval of `first_share`, over the decided games: 0 when none.
    pub first_low: f64,
    /// The high end of the 95% Wilson interval of `first_share`, over the decided games: 1 when none.
    pub first_high: f64,
    /// Games per ending, every `GameOverReason` in its declared order, the ones no game had at 0.
    pub reasons: IndexMap<GameOverReason, i64>,
    /// The turn counter at the end: the first, second and third quartiles, nearest rank.
    pub turn_quartiles: [i32; QUARTERS - 1],
    /// How many of the games have R1256000's counts: the figures below are over these only.
    pub counted: i64,
    /// How many of the counted games had a draw from an empty library, or null when none is counted.
    pub fatigue_games: Option<i64>,
    /// `fatigue_games` over `counted`, to three places.
    pub fatigue_share: Option<f64>,
    /// The mean of the nominal fatigue damage a counted game dealt, both seats added, to three places.
    pub fatigue_damage_mean: Option<f64>,
    /// Traps set from a hand over the counted games, both seats.
    pub traps_set: Option<i64>,
    /// Traps that fired over the counted games, both seats.
    pub traps_fired: Option<i64>,
    /// `traps_fired` over `traps_set` to three places, or null when no Trap was set.
    pub traps_fired_per_set: Option<f64>,
    /// Position switches by units over the counted games, both seats.
    pub position_switches: Option<i64>,
    /// The mean of `position_switches` over the counted games, to three places.
    pub position_switches_mean: Option<f64>,
    /// Cards an effect made Radiant over the counted games, both seats.
    pub radiant_upgrades: Option<i64>,
    /// The mean of `radiant_upgrades` over the counted games, to three places.
    pub radiant_upgrades_mean: Option<f64>,
}

/// `stats report --games`: one patch's figures in one mode from one source.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameGroup {
    /// The patch the games were played with.
    pub patch: String,
    /// The match type.
    pub mode: GameMode,
    /// Where the games were played: live matches or an AI development run's.
    pub source: GameSource,
    #[serde(flatten)]
    pub figures: GameFigures,
}

/// `stats report --games`: the groups a filter reads and the records it left to another source.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameReport {
    /// What the report read.
    pub filter: CardStatsFilter,
    /// How many records came from a source the filter does not read (R378).
    pub other_sources: i64,
    /// One group per patch, mode and source, in the order a record of it was first read.
    pub groups: Vec<GameGroup>,
}

/// R1994000: the figures of a total's games, `None` when it holds none. The turn quartiles are
/// read off the histogram by nearest rank, as off the games' turns sorted.
fn game_figures(total: &GameTotal) -> Option<GameFigures> {
    if total.games <= 0 {
        return None;
    }
    // R2567000: a draw is neither seat's win, so the first seat's share is over the decided games.
    let decided = total.games - total.draws;
    let (low, high) = wilson_interval(total.first_wins, decided);

    let mut reasons: IndexMap<GameOverReason, i64> =
        GAME_OVER_REASONS.iter().map(|reason| (*reason, 0)).collect();
    for (reason, games) in &total.reasons {
        *reasons.entry(*reason).or_default() += games;
    }

    let mut turns: Vec<(i32, i64)> = total.turns.iter().map(|(turn, games)| (*turn, *games)).collect();
    turns.sort_unstable();
    let n = usize::try_from(total.games).unwrap_or(usize::MAX);
    let turn_quartiles = std::array::from_fn(|cut| {
        let rank = ((cut + 1) * n).div_ceil(QUARTERS);
        let mut seen = 0usize;
        turns
            .iter()
            .find(|(_, games)| {
                seen += usize::try_from(*games).unwrap_or(0);
                seen >= rank
            })
            .map_or(0, |(turn, _)| *turn)
    });

    let counted = total.counted;
    let any = counted > 0;
    let mean = |sum: i64| round_share(sum as f64 / counted as f64);
    Some(GameFigures {
        games: total.games,
        draws: total.draws,
        first_wins: total.first_wins,
        first_share: (decided > 0).then(|| round_share(total.first_wins as f64 / decided as f64)),
        first_low: round_share(low),
        first_high: round_share(high),
        reasons,
        turn_quartiles,
        counted,
        fatigue_games: any.then_some(total.fatigue_games),
        fatigue_share: any.then(|| mean(total.fatigue_games)),
        fatigue_damage_mean: any.then(|| mean(total.fatigue_damage)),
        traps_set: any.then_some(total.traps_set),
        traps_fired: any.then_some(total.traps_fired),
        traps_fired_per_set: (any && total.traps_set > 0)
            .then(|| round_share(total.traps_fired as f64 / total.traps_set as f64)),
        position_switches: any.then_some(total.position_switches),
        position_switches_mean: any.then(|| mean(total.position_switches)),
        radiant_upgrades: any.then_some(total.radiant_upgrades),
        radiant_upgrades_mean: any.then(|| mean(total.radiant_upgrades)),
    })
}

/// R1994000: the game figures over the totals the filter reads, as `stats report --games` counted
/// the records folded into them: a key counts when the filter reads its source, mode and patch and
/// its pilot reads one of the key's seats (as `CardStatsReport.games` counts a record), and a key of
/// a source the filter does not read is only tallied (R378). One group per patch, mode and source,
/// in the order their first key was folded.
pub fn game_figures_of(totals: &StatsTotals, filter: &CardStatsFilter) -> GameReport {
    let sources = sources_of(filter.source);
    let mut other_sources = 0;
    let mut grouped: IndexMap<(String, GameMode, GameSource), GameTotal> = IndexMap::new();
    for (key, total) in &totals.game_totals {
        if !sources.contains(&key.source) {
            other_sources += total.games;
        } else if filter.mode.is_none_or(|mode| key.mode == mode)
            && filter.patch.as_ref().is_none_or(|patch| key.patch == *patch)
            && (pilot_counts(filter.pilot, key.pilots.0) || pilot_counts(filter.pilot, key.pilots.1))
        {
            grouped
                .entry((key.patch.clone(), key.mode, key.source))
                .or_default()
                .add(total);
        }
    }
    GameReport {
        filter: filter.clone(),
        other_sources,
        groups: grouped
            .into_iter()
            .filter_map(|((patch, mode, source), total)| {
                Some(GameGroup {
                    patch,
                    mode,
                    source,
                    figures: game_figures(&total)?,
                })
            })
            .collect(),
    }
}

/// R1994000: the game figures over records, folded into totals and read off them as the server
/// reads the totals it keeps, so `stats report --games` and `GET /api/stats/games` cannot count
/// differently. Groups are in the order a record of each was first read.
pub fn game_report(records: &[GameRecord], filter: &CardStatsFilter) -> GameReport {
    let mut totals = StatsTotals::default();
    for record in records {
        totals.fold(record, false);
    }
    game_figures_of(&totals, filter)
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
            Breakdown::PlayedRadiant => "Played Radiant",
            Breakdown::PlayedNormal => "Played normal",
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

fn pad_end(text: &str, width: usize) -> String {
    let mut out = text.to_string();
    out.push_str(&" ".repeat(width.saturating_sub(utf16_len(text))));
    out
}

fn pad_start(text: &str, width: usize) -> String {
    let mut out = " ".repeat(width.saturating_sub(utf16_len(text)));
    out.push_str(text);
    out
}

/// The report as a plain-text table: one row per card, each breakdown's win rate with its games
/// beside it, and the never-drawn tally with the drawn delta. `name_of` turns an id into the name
/// shown after it (TS's default, `() => undefined`, is `|_| None`).
pub fn format_card_stats(report: &CardStatsReport, name_of: impl Fn(&str) -> Option<String>) -> String {
    let named = |card: &str| match name_of(card) {
        None => card.to_string(),
        Some(name) => format!("{card} {name}"),
    };
    let mut header: Vec<String> = vec!["Card".to_string()];
    header.extend(
        BREAKDOWNS
            .iter()
            .map(|breakdown| BREAKDOWN_TITLES[*breakdown].to_string()),
    );
    header.push("Not drawn".to_string());
    header.push("Drawn Δ".to_string());
    let rows: Vec<Vec<String>> = report
        .cards
        .iter()
        .map(|stats| {
            let mut row = vec![named(&stats.card)];
            row.extend(
                BREAKDOWNS
                    .iter()
                    .map(|breakdown| format_tally(&stats[*breakdown])),
            );
            row.push(format_tally(&not_drawn(stats)));
            row.push(format_delta(drawn_delta(stats)));
            row
        })
        .collect();

    let mut lines: Vec<String> = vec![
        format!("Card win rates: {}.", describe_filter(&report.filter)),
        format!("{} games, {} decks.", report.games, report.decks),
        "Each cell is the win rate of the decks that held the card, with those games in brackets. A game counts once for".to_string(),
        "each deck that held the card, and a draw is a game not won. Drawn means the card reached the hand, in the opening".to_string(),
        "hand or by a later draw; Not drawn counts the decks that never had it there. Drawn Δ is the win rate of the decks".to_string(),
        "that drew it, played or not, minus Not drawn's, in points. Played Radiant counts the decks that played the card".to_string(),
        "Radiant at least once, Played normal the other played ones.".to_string(),
        String::new(),
    ];
    lines.extend(table(&header, &rows));
    // R2089000: the Created cards' own table, after the deck's, only when some seat made one.
    if !report.created.is_empty() {
        let header: Vec<String> = [
            "Card",
            "Created, used",
            "Created, not used",
            "Used Radiant",
            "Used normal",
            "Created Δ",
        ]
        .map(str::to_string)
        .to_vec();
        let rows: Vec<Vec<String>> = report
            .created
            .iter()
            .map(|stats| {
                vec![
                    named(&stats.card),
                    format_tally(&stats.created_played),
                    format_tally(&stats.created_not_played),
                    format_tally(&stats.created_played_radiant),
                    format_tally(&stats.created_played_normal),
                    format_delta(created_played_delta(stats)),
                ]
            })
            .collect();
        lines.extend([
            String::new(),
            "Created cards: every card made after the decks were built, tokens and The Coin among them. Each cell is the win".to_string(),
            "rate of the seats that made a Created copy of the card and used one, or made copies and used none, with those".to_string(),
            "games in brackets; a seat counts once a game however many it made. Created Δ is the used rate minus the other.".to_string(),
            String::new(),
        ]);
        lines.extend(table(&header, &rows));
    }
    lines.join("\n")
}

/// A plain-text table: the first column padded on the right, the rest on the left, each as wide as
/// its widest cell, two spaces between.
fn table(header: &[String], rows: &[Vec<String>]) -> Vec<String> {
    let widths: Vec<usize> = header
        .iter()
        .enumerate()
        .map(|(column, title)| {
            rows.iter()
                .map(|row| utf16_len(row.get(column).map_or("", String::as_str)))
                .fold(utf16_len(title), usize::max)
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
    std::iter::once(line(header))
        .chain(rows.iter().map(|row| line(row)))
        .collect()
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

/// R1256000: one of the per-seat counts: absent reads as `None`, anything but a non-negative
/// whole number is refused, as R1147000's malformed fields are.
fn seat_count(value: Option<&Value>, at: &str) -> Result<Option<i32>, EngineError> {
    match value {
        None => Ok(None),
        Some(value) => match whole_number(value)
            .filter(|n| *n >= 0)
            .and_then(|n| i32::try_from(n).ok())
        {
            Some(n) => Ok(Some(n)),
            None => Err(EngineError::new(format!(
                "{at} is not a non-negative whole number"
            ))),
        },
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

/// R2089000: a list of a Created card's uses: absent reads as `None`; each entry must name a card, a
/// turn of 1 or more and its face.
fn created_uses(value: Option<&Value>, at: &str) -> Result<Option<Vec<CreatedUse>>, EngineError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let Some(items) = value.as_array() else {
        return Err(EngineError::new(format!(
            "{at} is not a list of Created cards' uses"
        )));
    };
    let mut uses: Vec<CreatedUse> = Vec::with_capacity(items.len());
    for (idx, item) in items.iter().enumerate() {
        let entry = is_record(Some(item));
        let card = entry
            .and_then(|entry| entry.get("card"))
            .and_then(Value::as_str)
            .filter(|card| !card.is_empty());
        let turn = entry
            .and_then(|entry| entry.get("turn"))
            .and_then(whole_number)
            .filter(|turn| *turn >= 1)
            .and_then(|turn| i32::try_from(turn).ok());
        let radiant = entry
            .and_then(|entry| entry.get("radiant"))
            .and_then(Value::as_bool);
        match (card, turn, radiant) {
            (Some(card), Some(turn), Some(radiant)) => uses.push(CreatedUse {
                card: card.to_string(),
                turn,
                radiant,
            }),
            _ => {
                return Err(EngineError::new(format!(
                    "{at}[{idx}] is not a Created card's use"
                )));
            }
        }
    }
    Ok(Some(uses))
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
        played_radiant: None,
        fatigue_draws: seat_count(record.get("fatigueDraws"), &format!("{at}.fatigueDraws"))?,
        fatigue_damage: seat_count(record.get("fatigueDamage"), &format!("{at}.fatigueDamage"))?,
        traps_set: seat_count(record.get("trapsSet"), &format!("{at}.trapsSet"))?,
        traps_fired: seat_count(record.get("trapsFired"), &format!("{at}.trapsFired"))?,
        position_switches: seat_count(record.get("positionSwitches"), &format!("{at}.positionSwitches"))?,
        radiant_upgrades: seat_count(record.get("radiantUpgrades"), &format!("{at}.radiantUpgrades"))?,
        created_used: created_uses(record.get("createdUsed"), &format!("{at}.createdUsed"))?,
        created_unused: match record.get("createdUnused") {
            None => None,
            Some(value) => Some(ids(Some(value), &format!("{at}.createdUnused"))?),
        },
        created_played: created_uses(record.get("createdPlayed"), &format!("{at}.createdPlayed"))?,
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
    if let Some(Value::Array(flags)) = record.get("playedRadiant") {
        let mut out: Vec<bool> = Vec::with_capacity(flags.len());
        for (idx, flag) in flags.iter().enumerate() {
            match flag.as_bool() {
                Some(b) => out.push(b),
                None => {
                    return Err(EngineError::new(format!(
                        "{at}.playedRadiant[{idx}] is not a boolean"
                    )));
                }
            }
        }
        summary.played_radiant = Some(out);
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
        build: record_build(record.get("build"))?,
    })
}

/// R1367000: a record's build, absent (or null) on a record written before builds were recorded.
fn record_build(value: Option<&Value>) -> Result<Option<RecordBuild>, EngineError> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let Some(build) = value.as_object() else {
        return Err(EngineError::new("build is not an object"));
    };
    Ok(Some(RecordBuild {
        commit: text(build.get("commit"), "build.commit")?,
        ai_generation: seat_count(build.get("aiGeneration"), "build.aiGeneration")?,
    }))
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
            played_radiant: None,
            fatigue_draws: None,
            fatigue_damage: None,
            traps_set: None,
            traps_fired: None,
            position_switches: None,
            radiant_upgrades: None,
            created_used: None,
            created_unused: None,
            created_played: None,
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
            build: None,
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
                played_radiant: tally(0, 0, 0),
                played_normal: tally(0, 0, 0),
                created_played: tally(0, 0, 0),
                created_not_played: tally(0, 0, 0),
                created_played_radiant: tally(0, 0, 0),
                created_played_normal: tally(0, 0, 0),
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
        fn r2427000_never_drawn_is_in_deck_less_both_drawn_sides_and_the_drawn_delta_needs_both() {
            let stats = CardStats {
                card: "a".to_string(),
                in_deck: tally(20, 9, 3),
                opening_hand: tally(0, 0, 0),
                going_first: tally(0, 0, 0),
                going_second: tally(0, 0, 0),
                played: tally(8, 6, 1),
                drawn_not_played: tally(4, 1, 1),
                played_radiant: tally(0, 0, 0),
                played_normal: tally(0, 0, 0),
                created_played: tally(0, 0, 0),
                created_not_played: tally(0, 0, 0),
                created_played_radiant: tally(0, 0, 0),
                created_played_normal: tally(0, 0, 0),
            };
            assert_eq!(not_drawn(&stats), tally(8, 2, 1));
            let delta = drawn_delta(&stats).unwrap();
            assert!((delta - (7.0 / 12.0 - 0.25)).abs() < 1e-12);
            assert_eq!(format_delta(drawn_delta(&stats)), "+33.3");
            // Nothing ever reached the hand: no baseline and no delta.
            let never_drawn = CardStats {
                in_deck: tally(12, 7, 2),
                ..stats.clone()
            };
            assert_eq!(not_drawn(&never_drawn), tally(0, 0, 0));
            assert_eq!(drawn_delta(&never_drawn), None);
            // Nothing ever left it: never drawn is the whole deck, and still no delta.
            let never_played = CardStats {
                played: tally(0, 0, 0),
                drawn_not_played: tally(0, 0, 0),
                ..stats.clone()
            };
            assert_eq!(not_drawn(&never_played), never_played.in_deck);
            assert_eq!(drawn_delta(&never_played), None);
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
                    patch: "v0.2.3".to_string(),
                    ..record()
                },
                // A practice-shaped record: one human seat, one AI seat. The AI's deck holds only z.
                GameRecord {
                    id: "4".to_string(),
                    mode: GameMode::Random,
                    patch: "v0.2.3".to_string(),
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
            assert_eq!(games(None, Some("v0.2.3")), 2);
            assert_eq!(games(Some(GameMode::Random), Some("v0.2.3")), 1);
            assert_eq!(games(Some(GameMode::Bo1), Some("v0.2.3")), 0);

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
            assert!(heading.contains("Not drawn"));
            assert!(heading.contains("Drawn Δ"));
            assert!(!heading.contains("Played Δ"));

            let alpha = lines.iter().find(|line| line.starts_with("a Alpha")).unwrap();
            // In deck, opening hand, going first, going second, played, drawn-not-played, played-radiant, played-normal, not drawn; then the delta.
            assert_eq!(
                split_wide(alpha)[1..],
                [
                    "100.0% (1)",
                    "100.0% (1)",
                    "100.0% (1)",
                    "— (0)",
                    "100.0% (1)",
                    "— (0)",
                    "— (0)",
                    "— (0)",
                    "— (0)",
                    "—"
                ]
            );
            let beta = lines.iter().find(|line| line.starts_with("b ")).unwrap();
            assert_eq!(
                split_wide(beta)[1..],
                [
                    "0.0% (1)", "— (0)", "— (0)", "0.0% (1)", "— (0)", "0.0% (1)", "— (0)", "— (0)", "— (0)",
                    "—"
                ]
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
                    patch: Some("v0.2.3".to_string()),
                    pilot: PilotFilter::Ai
                }),
                "AI development runs, All Random, patch v0.2.3, AI pilots"
            );
        }

        #[test]
        fn r1147000_splits_the_played_decks_by_the_face_the_card_was_played_on() {
            let mut s1 = seat(&["a", "b"], &[], &[], &["a", "b", "a"]);
            s1.played_radiant = Some(vec![false, false, true]);
            let mut s2 = seat(&["a"], &[], &[], &["a"]);
            s2.played_radiant = Some(vec![false]);

            let m1 = GameRecord {
                id: "m1".to_string(),
                game: game(Winner::P1, GameOverReason::HeroDeath, 5, s1, s2),
                ..record()
            };

            let s1_m2 = seat(&["a"], &[], &[], &["a"]);
            let mut s2_m2 = seat(&["a"], &[], &[], &["a"]);
            s2_m2.played_radiant = Some(vec![true, true]); // wrong length

            let m2 = GameRecord {
                id: "m2".to_string(),
                game: game(Winner::P2, GameOverReason::HeroDeath, 5, s1_m2, s2_m2),
                ..record()
            };

            let report = card_stats(&[m1, m2], &live());
            let stats_a = stats_of(&report, "a");
            assert_eq!(stats_a.played, tally(4, 2, 0));
            assert_eq!(stats_a.played_radiant, tally(1, 1, 0));
            assert_eq!(stats_a.played_normal, tally(1, 0, 0));

            let stats_b = stats_of(&report, "b");
            assert_eq!(stats_b.played_normal, tally(1, 1, 0));

            let mut json_a = serde_json::to_value(stats_a).unwrap();
            json_a.as_object_mut().unwrap().remove("playedRadiant");
            json_a.as_object_mut().unwrap().remove("playedNormal");
            let deserialized: CardStats = serde_json::from_value(json_a).unwrap();
            assert_eq!(deserialized.played_radiant, tally(0, 0, 0));
            assert_eq!(deserialized.played_normal, tally(0, 0, 0));
        }
    }

    /// R1044000: the totals read every figure the records gave. Each reference below is the counting
    /// that read the records themselves before the totals (`card_stats` here, `card_drills` in the
    /// server's `api/stats.rs` and `ranked_human_bo1_decklists`' loop in `bots/pool.rs`), kept as it
    /// was so the totals are measured against it.
    mod totals {
        use super::*;

        /// The seats of a record the pilot filter counts, in seat order: the record-by-record
        /// reference the totals are held to.
        fn seats_counted(record: &GameRecord, pilot: PilotFilter) -> Vec<PlayerId> {
            PLAYER_IDS
                .into_iter()
                .filter(|seat| pilot_counts(pilot, record.pilots[*seat]))
                .collect()
        }

        /// `card_stats` as it counted the records themselves.
        fn reference_card_stats(records: &[GameRecord], filter: &CardStatsFilter) -> CardStatsReport {
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
                    let played_radiant: Option<IndexSet<&str>> = summary
                        .played_radiant
                        .as_ref()
                        .filter(|flags| flags.len() == summary.played.len())
                        .map(|flags| {
                            summary
                                .played
                                .iter()
                                .zip(flags.iter())
                                .filter_map(|(id, &radiant)| if radiant { Some(id.as_str()) } else { None })
                                .collect()
                        });
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
                            if let Some(radiant) = &played_radiant {
                                count(
                                    if radiant.contains(card) {
                                        &mut stats.played_radiant
                                    } else {
                                        &mut stats.played_normal
                                    },
                                    outcome,
                                );
                            }
                        } else if in_hand.contains(card) {
                            count(&mut stats.drawn_not_played, outcome);
                        }
                    }
                }
            }
            let mut cards: Vec<CardStats> = by_card.into_values().collect();
            cards.sort_by(|a, b| a.card.cmp(&b.card));
            CardStatsReport {
                filter: filter.clone(),
                games,
                decks,
                cards,
                created: Vec::new(),
            }
        }

        /// The server's `seat_counted`: a development seat always counts, a live one when the pilot
        /// is merged or names it.
        fn seat_counted(source: GameSource, pilot: Pilot, live_pilot: PilotFilter) -> bool {
            source == GameSource::Dev || pilot_counts(live_pilot, pilot)
        }

        type ReferenceDrill = (Vec<(i32, DrillCount)>, Vec<(String, DrillCount)>);

        /// The server's `card_drills`, over the records in id order.
        fn reference_card_drills(
            records: &[GameRecord],
            live_pilot: PilotFilter,
            top: usize,
        ) -> IndexMap<String, ReferenceDrill> {
            type Tallies<'a> = (IndexMap<i32, DrillCount>, IndexMap<&'a str, DrillCount>);
            let mut tallies: IndexMap<&str, Tallies> = IndexMap::new();
            for record in records {
                for seat in PLAYER_IDS {
                    if !seat_counted(record.source, record.pilots[seat], live_pilot) {
                        continue;
                    }
                    let summary = &record.game.seats[seat];
                    let won = record.game.winner == Winner::from(seat);
                    let distinct: IndexSet<&str> = summary.deck.iter().map(String::as_str).collect();
                    for &card_id in &distinct {
                        let (turn_tallies, co_played_tallies) = tallies.entry(card_id).or_default();
                        let mut turns_played_in_game: IndexSet<i32> = IndexSet::new();
                        if let Some(played_turns) = summary
                            .played_turns
                            .as_ref()
                            .filter(|turns| turns.len() == summary.played.len())
                        {
                            for (index, played) in summary.played.iter().enumerate() {
                                if played == card_id
                                    && let Some(turn) = played_turns.get(index).copied()
                                    && turn > 0
                                {
                                    turns_played_in_game.insert(turn);
                                }
                            }
                        }
                        for turn in turns_played_in_game {
                            let tally = turn_tallies.entry(turn).or_default();
                            tally.games += 1;
                            if won {
                                tally.wins += 1;
                            }
                        }
                        for &other_id in &distinct {
                            if other_id == card_id {
                                continue;
                            }
                            let tally = co_played_tallies.entry(other_id).or_default();
                            tally.games += 1;
                            if won {
                                tally.wins += 1;
                            }
                        }
                    }
                }
            }
            tallies
                .into_iter()
                .map(|(card_id, (turn_tallies, co_played_tallies))| {
                    let mut by_turn: Vec<(i32, DrillCount)> = turn_tallies.into_iter().collect();
                    by_turn.sort_by_key(|(turn, _)| *turn);
                    let mut co_played: Vec<(&str, DrillCount)> = co_played_tallies.into_iter().collect();
                    co_played.sort_by_key(|(_, tally)| std::cmp::Reverse(tally.games));
                    co_played.truncate(top);
                    let co_played = co_played
                        .into_iter()
                        .map(|(id, tally)| (id.to_string(), tally))
                        .collect();
                    (card_id.to_string(), (by_turn, co_played))
                })
                .collect()
        }

        /// The ladder bots' loop over a patch's live Best-of-1 records in id order: each distinct
        /// list's first deck, sorted ids, games and wins, in the order first met.
        fn reference_lists(records: &[GameRecord]) -> Vec<(Vec<String>, Vec<String>, i64, i64)> {
            let mut lists: IndexMap<Vec<String>, (Vec<String>, i64, i64)> = IndexMap::new();
            for record in records {
                if record.source != GameSource::Live || record.mode != GameMode::Bo1 {
                    continue;
                }
                for seat in PLAYER_IDS {
                    if record.pilots[seat] != Pilot::Human {
                        continue;
                    }
                    let deck = &record.game.seats[seat].deck;
                    let mut key = deck.clone();
                    key.sort();
                    let list = lists.entry(key).or_insert_with(|| (deck.clone(), 0, 0));
                    list.1 += 1;
                    if record.game.winner.player() == Some(seat) {
                        list.2 += 1;
                    }
                }
            }
            lists
                .into_iter()
                .map(|(key, (deck, games, wins))| (deck, key, games, wins))
                .collect()
        }

        /// A small deterministic generator (xorshift64), so the records are many and varied and the
        /// same on every run.
        struct Draws(u64);

        impl Draws {
            fn next(&mut self) -> u64 {
                self.0 ^= self.0 << 13;
                self.0 ^= self.0 >> 7;
                self.0 ^= self.0 << 17;
                self.0
            }

            fn below(&mut self, n: usize) -> usize {
                usize::try_from(self.next() % n as u64).unwrap_or(0)
            }

            fn pick<'a>(&mut self, from: &[&'a str]) -> &'a str {
                from[self.below(from.len())]
            }
        }

        /// Few cards, so pairs tie often and every tie-break is exercised.
        const CARDS: &[&str] = &["a", "b", "c", "d", "e", "f", "g"];

        fn random_seat(draws: &mut Draws) -> SeatSummary {
            let size = 3 + draws.below(4);
            let deck: Vec<String> = (0..size).map(|_| draws.pick(CARDS).to_string()).collect();
            let some = |draws: &mut Draws, most: usize| -> Vec<String> {
                (0..draws.below(most + 1))
                    .map(|_| draws.pick(CARDS).to_string())
                    .collect()
            };
            let opening = some(draws, 3);
            let drawn = some(draws, 3);
            let played = some(draws, 4);
            // Every kind of turn list: none, one per play (some of them 0, which counts no turn),
            // and one of the wrong length, which the drill-down reads as none.
            let played_turns = match draws.below(4) {
                0 => None,
                1 => Some(vec![1; played.len() + 1]),
                _ => Some(
                    played
                        .iter()
                        .map(|_| i32::try_from(draws.below(5)).unwrap_or(0))
                        .collect(),
                ),
            };
            let played_radiant = played_turns
                .as_ref()
                .map(|t| t.iter().map(|turn| turn % 2 == 1).collect());
            SeatSummary {
                deck,
                opening,
                drawn,
                played,
                played_turns,
                played_radiant,
                fatigue_draws: None,
                fatigue_damage: None,
                traps_set: None,
                traps_fired: None,
                position_switches: None,
                radiant_upgrades: None,
                created_used: None,
                created_unused: None,
                created_played: None,
            }
        }

        fn random_record(draws: &mut Draws, n: usize) -> GameRecord {
            let source = if draws.below(3) == 0 {
                GameSource::Dev
            } else {
                GameSource::Live
            };
            let pilot = |draws: &mut Draws| {
                if draws.below(3) == 0 {
                    Pilot::Ai
                } else {
                    Pilot::Human
                }
            };
            let pilots = PerPlayer::new(pilot(draws), pilot(draws));
            let winner = match draws.below(5) {
                0 => Winner::Draw,
                1 | 2 => Winner::P1,
                _ => Winner::P2,
            };
            GameRecord {
                // Ids sort apart from the order they are made in, and live ones as match ids do.
                id: match source {
                    GameSource::Dev => format!("dev:x:{:03}", (n * 37) % 1000),
                    GameSource::Live => format!("m{:03}", (n * 53) % 1000),
                },
                source,
                mode: [GameMode::Bo1, GameMode::Bo3, GameMode::Random][draws.below(3)],
                patch: ["v1", "v2"][draws.below(2)].to_string(),
                pilots,
                game: GameSummary {
                    first: if draws.below(2) == 0 {
                        PlayerId::P1
                    } else {
                        PlayerId::P2
                    },
                    winner,
                    reason: GAME_OVER_REASONS[n % GAME_OVER_REASONS.len()],
                    turns: 3 + i32::try_from(n % 23).unwrap_or(0),
                    seats: with_counts(PerPlayer::new(random_seat(draws), random_seat(draws)), n),
                },
                build: None,
            }
        }

        /// R1256000's six counts, made from `n` so the draws stay as they were: on both seats in
        /// three records of four, on p1's alone in one of eight (which no game figure counts), on
        /// neither in the rest, as records written before them.
        fn with_counts(mut seats: PerPlayer<SeatSummary>, n: usize) -> PerPlayer<SeatSummary> {
            let both = !n.is_multiple_of(4);
            let p1_only = n.is_multiple_of(8);
            for (index, seat) in [PlayerId::P1, PlayerId::P2].into_iter().enumerate() {
                if !(both || (p1_only && seat == PlayerId::P1)) {
                    continue;
                }
                let at = |offset: usize, most: usize| i32::try_from((n + index + offset) % most).unwrap_or(0);
                let summary = &mut seats[seat];
                summary.fatigue_draws = Some(at(0, 3) * at(1, 2));
                summary.fatigue_damage = Some(at(2, 7));
                summary.traps_set = Some(at(3, 4));
                summary.traps_fired = Some(at(4, 3));
                summary.position_switches = Some(at(5, 5));
                summary.radiant_upgrades = Some(at(6, 3));
            }
            seats
        }

        /// Records as the store reads them: one per id, in id order.
        fn records() -> Vec<GameRecord> {
            let mut draws = Draws(0x9e37_79b9_7f4a_7c15);
            let mut records: Vec<GameRecord> = (0..400).map(|n| random_record(&mut draws, n)).collect();
            records.sort_by(|a, b| a.id.cmp(&b.id));
            records.dedup_by(|later, earlier| later.id == earlier.id);
            records
        }

        /// The records folded out of order, in two pieces added together.
        fn folded(records: &[GameRecord]) -> StatsTotals {
            let mut shuffled: Vec<&GameRecord> = records.iter().collect();
            let mut draws = Draws(7);
            for index in (1..shuffled.len()).rev() {
                shuffled.swap(index, draws.below(index + 1));
            }
            let (first, second) = shuffled.split_at(shuffled.len() / 3);
            let mut totals = StatsTotals::default();
            for record in first {
                totals.fold(record, true);
            }
            let mut rest = StatsTotals::default();
            for record in second {
                rest.fold(record, true);
            }
            totals.add(&rest);
            totals
        }

        #[test]
        fn r1044000_totals_give_every_card_report_the_records_gave() {
            let records = records();
            let totals = folded(&records);
            let mut checked = 0;
            for source in SOURCE_FILTERS {
                for mode in [
                    None,
                    Some(GameMode::Bo1),
                    Some(GameMode::Bo3),
                    Some(GameMode::Random),
                ] {
                    for patch in [None, Some("v1"), Some("v2"), Some("v3")] {
                        for pilot in PILOT_FILTERS {
                            let filter = CardStatsFilter {
                                source: *source,
                                mode,
                                patch: patch.map(str::to_string),
                                pilot: *pilot,
                            };
                            let expected = reference_card_stats(&records, &filter);
                            assert_eq!(card_stats_of(&totals, &filter), expected, "{filter:?}");
                            assert_eq!(card_stats(&records, &filter), expected, "{filter:?}");
                            checked += expected.games;
                        }
                    }
                }
            }
            assert!(checked > 0, "the filters read some games");
        }

        #[test]
        fn r1044000_totals_give_every_drill_down_the_records_gave_ties_in_the_order_first_met() {
            let records = records();
            let totals = folded(&records);
            for patch in ["v1", "v2"] {
                for source in SOURCE_FILTERS {
                    let sources = sources_of(*source);
                    let counted: Vec<GameRecord> = records
                        .iter()
                        .filter(|record| record.patch == patch && sources.contains(&record.source))
                        .cloned()
                        .collect();
                    for live_pilot in PILOT_FILTERS {
                        for top in [1, 2, 3, 10] {
                            let expected = reference_card_drills(&counted, *live_pilot, top);
                            let drills = card_drills_of(
                                &totals,
                                |key| {
                                    key.patch == patch
                                        && sources.contains(&key.source)
                                        && seat_counted(key.source, key.pilot, *live_pilot)
                                },
                                top,
                            );
                            assert_eq!(drills.len(), expected.len(), "{patch} {source} {live_pilot}");
                            for (card, (by_turn, co_played)) in &expected {
                                let drill = &drills[card.as_str()];
                                assert_eq!(&drill.by_turn, by_turn, "{patch} {source} {live_pilot} {card}");
                                assert_eq!(
                                    &drill.co_played, co_played,
                                    "{patch} {source} {live_pilot} {card} top {top}"
                                );
                            }
                        }
                    }
                }
            }
        }

        #[test]
        fn r1044000_totals_hold_each_patchs_live_best_of_one_lists_in_the_order_first_held() {
            let records = records();
            let totals = folded(&records);
            for patch in ["v1", "v2"] {
                let of_patch: Vec<GameRecord> = records
                    .iter()
                    .filter(|record| record.patch == patch)
                    .cloned()
                    .collect();
                let lists: Vec<(Vec<String>, Vec<String>, i64, i64)> = totals
                    .decklists(patch)
                    .into_iter()
                    .map(|list| (list.deck, list.key, list.games, list.wins))
                    .collect();
                assert_eq!(lists, reference_lists(&of_patch), "{patch}");
                assert!(!lists.is_empty());
            }
        }

        #[test]
        fn r1044000_folding_in_any_order_or_in_pieces_makes_the_same_totals() {
            let records = records();
            let mut in_order = StatsTotals::default();
            for record in &records {
                in_order.fold(record, true);
            }
            assert_eq!(folded(&records), in_order);
            // Selecting without drills keeps the breakdowns and drops the turn and pair counts.
            let selected = in_order.select(&[GameSource::Live], Some(GameMode::Bo1), Some("v1"), false);
            assert!(
                selected.cards.keys().all(|key| key.source == GameSource::Live
                    && key.mode == GameMode::Bo1
                    && key.patch == "v1")
            );
            assert!(
                selected
                    .cards
                    .values()
                    .flat_map(|cards| cards.values())
                    .all(|total| total.turns.is_empty() && total.co.is_empty())
            );
            let filter = CardStatsFilter {
                source: SourceFilter::Live,
                mode: Some(GameMode::Bo1),
                patch: Some("v1".to_string()),
                pilot: PilotFilter::Unified,
            };
            assert_eq!(
                card_stats_of(&selected, &filter),
                card_stats_of(&in_order, &filter)
            );
            // R1994000: and the whole games of the keys it keeps.
            assert!(!selected.game_totals.is_empty());
            assert!(
                selected
                    .game_totals
                    .keys()
                    .all(|key| key.source == GameSource::Live
                        && key.mode == GameMode::Bo1
                        && key.patch == "v1")
            );
            assert_eq!(
                game_figures_of(&selected, &filter).groups,
                game_figures_of(&in_order, &filter).groups
            );
            assert_eq!(
                in_order.live_games("v1"),
                records
                    .iter()
                    .filter(|record| record.patch == "v1" && record.source == GameSource::Live)
                    .count() as i64
            );
        }

        /// R2089000: the Created table as the records themselves give it: each counted seat once for
        /// each card it made, in `createdPlayed` when it used one, split by whether any was Radiant at
        /// its first use, else in `createdNotPlayed`.
        fn reference_created(records: &[GameRecord], filter: &CardStatsFilter) -> Vec<CreatedStats> {
            let mut by_card: IndexMap<String, CreatedStats> = IndexMap::new();
            for record in records {
                if !record_matches(record, filter) {
                    continue;
                }
                for seat in seats_counted(record, filter.pilot) {
                    let summary = &record.game.seats[seat];
                    let outcome = outcome_for(&record.game, seat);
                    let uses = summary.created_used.clone().unwrap_or_default();
                    let unused = summary.created_unused.clone().unwrap_or_default();
                    let mut made: Vec<&str> = uses
                        .iter()
                        .map(|used| used.card.as_str())
                        .chain(unused.iter().map(String::as_str))
                        .collect();
                    made.sort_unstable();
                    made.dedup();
                    for card in made {
                        let stats = by_card.entry(card.to_string()).or_insert_with(|| CreatedStats {
                            card: card.to_string(),
                            created_played: Tally::default(),
                            created_not_played: Tally::default(),
                            created_played_radiant: Tally::default(),
                            created_played_normal: Tally::default(),
                        });
                        let of_card: Vec<&CreatedUse> =
                            uses.iter().filter(|used| used.card == card).collect();
                        if of_card.is_empty() {
                            count(&mut stats.created_not_played, outcome);
                            continue;
                        }
                        count(&mut stats.created_played, outcome);
                        count(
                            if of_card.iter().any(|used| used.radiant) {
                                &mut stats.created_played_radiant
                            } else {
                                &mut stats.created_played_normal
                            },
                            outcome,
                        );
                    }
                }
            }
            let mut created: Vec<CreatedStats> = by_card.into_values().collect();
            created.sort_by(|a, b| a.card.cmp(&b.card));
            created
        }

        /// The records with Created lists drawn for two seats in three, the rest left as records
        /// written before them. `a` is a deck card too, and `t`, `u` and `v` are no deck's.
        fn with_created(records: &[GameRecord]) -> Vec<GameRecord> {
            const MADE: &[&str] = &["a", "t", "u", "v"];
            let mut draws = Draws(0x2089_0000_2089);
            let mut out = records.to_vec();
            for record in &mut out {
                for seat in PLAYER_IDS {
                    if draws.below(3) == 0 {
                        continue;
                    }
                    let mut used: Vec<CreatedUse> = Vec::new();
                    for _ in 0..draws.below(4) {
                        used.push(CreatedUse {
                            card: draws.pick(MADE).to_string(),
                            turn: 1 + i32::try_from(draws.below(9)).unwrap_or(0),
                            radiant: draws.below(3) == 0,
                        });
                    }
                    let unused: Vec<String> = (0..draws.below(3))
                        .map(|_| draws.pick(MADE).to_string())
                        .collect();
                    let summary = &mut record.game.seats[seat];
                    summary.created_used = (!used.is_empty()).then_some(used);
                    summary.created_unused = (!unused.is_empty()).then_some(unused);
                }
            }
            out
        }

        #[test]
        fn r2089000_totals_give_the_created_table_the_records_gave_and_the_deck_table_they_gave_before() {
            let plain = records();
            let records = with_created(&plain);
            let totals = folded(&records);
            let mut in_order = StatsTotals::default();
            for record in &records {
                in_order.fold(record, true);
            }
            assert_eq!(totals, in_order, "folded in any order or in pieces");
            let mut rows = 0;
            for source in SOURCE_FILTERS {
                for mode in [None, Some(GameMode::Bo1), Some(GameMode::Random)] {
                    for patch in [None, Some("v1"), Some("v2")] {
                        for pilot in PILOT_FILTERS {
                            let filter = CardStatsFilter {
                                source: *source,
                                mode,
                                patch: patch.map(str::to_string),
                                pilot: *pilot,
                            };
                            let report = card_stats_of(&totals, &filter);
                            assert_eq!(report.created, reference_created(&records, &filter), "{filter:?}");
                            // The deck table is the one the same records gave without their lists.
                            let dealt = reference_card_stats(&plain, &filter);
                            assert_eq!(
                                (report.games, report.decks, &report.cards),
                                (dealt.games, dealt.decks, &dealt.cards),
                                "{filter:?}"
                            );
                            assert_eq!(card_stats(&records, &filter), report, "{filter:?}");
                            rows += report.created.len();
                        }
                    }
                }
            }
            assert!(rows > 0, "the filters read some Created cards");
            // A selection keeps them as it keeps the deck's.
            let filter = CardStatsFilter {
                source: SourceFilter::Live,
                mode: Some(GameMode::Bo1),
                patch: Some("v1".to_string()),
                pilot: PilotFilter::Unified,
            };
            let selected = in_order.select(&[GameSource::Live], Some(GameMode::Bo1), Some("v1"), false);
            assert!(!card_stats_of(&selected, &filter).created.is_empty());
            assert_eq!(
                card_stats_of(&selected, &filter),
                card_stats_of(&in_order, &filter)
            );
        }

        /// One group's figures over its records, which are never none: `stats report --games` as
        /// it counted the records themselves (`crates/tools/src/stats.rs`, before R1994000).
        /// The first seat's share is over the decided games (R2567000).
        fn reference_game_group(key: (String, GameMode, GameSource), records: &[&GameRecord]) -> GameGroup {
            let (patch, mode, source) = key;
            let n = records.len();
            let draws = records
                .iter()
                .filter(|record| record.game.winner == Winner::Draw)
                .count();
            let first_wins = records
                .iter()
                .filter(|record| record.game.winner.player() == Some(record.game.first))
                .count();
            let decided = n - draws;
            let (low, high) = wilson_interval(first_wins as i64, decided as i64);

            let mut reasons: IndexMap<GameOverReason, i64> =
                GAME_OVER_REASONS.iter().map(|reason| (*reason, 0)).collect();
            for record in records {
                *reasons.entry(record.game.reason).or_default() += 1;
            }

            let mut turns: Vec<i32> = records.iter().map(|record| record.game.turns).collect();
            turns.sort_unstable();
            let turn_quartiles = std::array::from_fn(|cut| turns[((cut + 1) * n).div_ceil(QUARTERS) - 1]);

            let known: Vec<GameCounts> = records
                .iter()
                .filter_map(|record| game_counts(&record.game))
                .collect();
            let counted = known.len();
            let any = counted > 0;
            let fatigue_games = known.iter().filter(|counts| counts.fatigue_draws > 0).count();
            let damage: i64 = known.iter().map(|counts| counts.fatigue_damage).sum();
            let traps_set: i64 = known.iter().map(|counts| counts.traps_set).sum();
            let traps_fired: i64 = known.iter().map(|counts| counts.traps_fired).sum();
            let switches: i64 = known.iter().map(|counts| counts.position_switches).sum();
            let upgrades: i64 = known.iter().map(|counts| counts.radiant_upgrades).sum();
            GameGroup {
                patch,
                mode,
                source,
                figures: GameFigures {
                    games: n as i64,
                    draws: draws as i64,
                    first_wins: first_wins as i64,
                    first_share: (decided > 0).then(|| round_share(first_wins as f64 / decided as f64)),
                    first_low: round_share(low),
                    first_high: round_share(high),
                    reasons,
                    turn_quartiles,
                    counted: counted as i64,
                    fatigue_games: any.then_some(fatigue_games as i64),
                    fatigue_share: any.then(|| round_share(fatigue_games as f64 / counted as f64)),
                    fatigue_damage_mean: any.then(|| round_share(damage as f64 / counted as f64)),
                    traps_set: any.then_some(traps_set),
                    traps_fired: any.then_some(traps_fired),
                    traps_fired_per_set: (any && traps_set > 0)
                        .then(|| round_share(traps_fired as f64 / traps_set as f64)),
                    position_switches: any.then_some(switches),
                    position_switches_mean: any.then(|| round_share(switches as f64 / counted as f64)),
                    radiant_upgrades: any.then_some(upgrades),
                    radiant_upgrades_mean: any.then(|| round_share(upgrades as f64 / counted as f64)),
                },
            }
        }

        /// `stats report --games` over the records themselves, before R1994000.
        fn reference_game_report(records: &[GameRecord], filter: &CardStatsFilter) -> GameReport {
            let sources = sources_of(filter.source);
            let mut other_sources = 0;
            let mut grouped: IndexMap<(String, GameMode, GameSource), Vec<&GameRecord>> = IndexMap::new();
            for record in records {
                if !sources.contains(&record.source) {
                    other_sources += 1;
                } else if record_matches(record, filter) && !seats_counted(record, filter.pilot).is_empty() {
                    grouped
                        .entry((record.patch.clone(), record.mode, record.source))
                        .or_default()
                        .push(record);
                }
            }
            GameReport {
                filter: filter.clone(),
                other_sources,
                groups: grouped
                    .into_iter()
                    .map(|(key, records)| reference_game_group(key, &records))
                    .collect(),
            }
        }

        /// A report's groups by patch, mode and source, as totals folded out of order hold them.
        fn sorted(mut report: GameReport) -> GameReport {
            report
                .groups
                .sort_by(|a, b| (&a.patch, a.mode, a.source).cmp(&(&b.patch, b.mode, b.source)));
            report
        }

        #[test]
        fn r1994000_game_figures_off_totals_folded_in_any_order_are_what_the_records_give() {
            let records = records();
            let totals = folded(&records);
            let mut counted = 0;
            for source in SOURCE_FILTERS {
                for mode in [
                    None,
                    Some(GameMode::Bo1),
                    Some(GameMode::Bo3),
                    Some(GameMode::Random),
                ] {
                    for patch in [None, Some("v1"), Some("v2"), Some("v3")] {
                        for pilot in PILOT_FILTERS {
                            let filter = CardStatsFilter {
                                source: *source,
                                mode,
                                patch: patch.map(str::to_string),
                                pilot: *pilot,
                            };
                            let expected = reference_game_report(&records, &filter);
                            assert_eq!(game_report(&records, &filter), expected, "{filter:?}");
                            assert_eq!(
                                sorted(game_figures_of(&totals, &filter)),
                                sorted(expected.clone()),
                                "{filter:?}"
                            );
                            counted += expected
                                .groups
                                .iter()
                                .filter(|group| group.figures.counted > 0)
                                .count();
                        }
                    }
                }
            }
            assert!(counted > 0, "some group's records carry R1256000's counts");
            // A total is stored as its JSON and read back whole.
            for total in totals.game_totals.values() {
                let json = serde_json::to_string(total).unwrap();
                assert_eq!(&serde_json::from_str::<GameTotal>(&json).unwrap(), total);
            }
        }

        #[test]
        fn r2567000_the_first_share_counts_only_the_decided_games() {
            // 10 games: 4 first-seat wins, 4 second-seat wins, 2 draws.
            let total = GameTotal {
                games: 10,
                draws: 2,
                first_wins: 4,
                turns: [(12, 10)].into_iter().collect(),
                ..GameTotal::default()
            };
            let figures = game_figures(&total).unwrap();
            assert_eq!((figures.games, figures.draws, figures.first_wins), (10, 2, 4));
            assert_eq!(figures.first_share, Some(0.5));
            let (low, high) = wilson_interval(4, 8);
            assert_eq!(
                (figures.first_low, figures.first_high),
                (round_share(low), round_share(high))
            );
            assert_eq!((figures.first_low, figures.first_high), (0.215, 0.785));
        }

        #[test]
        fn r2567000_every_game_drawn_leaves_the_first_share_unknown() {
            let total = GameTotal {
                games: 3,
                draws: 3,
                turns: [(40, 3)].into_iter().collect(),
                ..GameTotal::default()
            };
            let figures = game_figures(&total).unwrap();
            assert_eq!(figures.first_share, None);
            assert_eq!((figures.first_low, figures.first_high), (0.0, 1.0));
            assert_eq!(
                serde_json::to_value(&figures).unwrap()["firstShare"],
                serde_json::Value::Null
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
        fn r1367000_a_record_without_build_still_loads_and_one_with_it_reads_back() {
            let old = record();
            assert_eq!(old.build, None);
            let json = serde_json::to_value(&old).unwrap();
            assert!(json.get("build").is_none(), "an absent build is not written");
            assert_eq!(parse_game_record(&json).unwrap(), old);
            assert_eq!(serde_json::from_value::<GameRecord>(json.clone()).unwrap(), old);
            assert_eq!(
                parse_game_record(&with(&json, "build", Value::Null)).unwrap(),
                old
            );

            let built = GameRecord {
                build: Some(RecordBuild {
                    commit: "0123abc".to_string(),
                    ai_generation: Some(12),
                }),
                ..record()
            };
            let json = serde_json::to_value(&built).unwrap();
            assert_eq!(json["build"], json!({ "commit": "0123abc", "aiGeneration": 12 }));
            assert_eq!(parse_game_record(&json).unwrap(), built);
            let human = with(&json, "build", json!({ "commit": "0123abc" }));
            assert_eq!(
                parse_game_record(&human).unwrap().build.unwrap().ai_generation,
                None
            );

            assert!(refusal(&with(&json, "build", json!("0123abc"))).contains("build is not an object"));
            assert!(refusal(&with(&json, "build", json!({ "commit": "" }))).contains("build.commit"));
            assert!(
                refusal(&with(
                    &json,
                    "build",
                    json!({ "commit": "a", "aiGeneration": -1 })
                ))
                .contains("build.aiGeneration")
            );
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

        #[test]
        fn r1147000_reads_back_each_plays_face_and_refuses_a_flag_that_is_not_a_boolean() {
            let mut s = seat(&["a", "b"], &[], &[], &["a", "b"]);
            s.played_radiant = Some(vec![true, false]);
            let written = GameRecord {
                game: game(Winner::P1, GameOverReason::HeroDeath, 5, s, seat_default()),
                ..record()
            };
            let json = serde_json::to_value(&written).unwrap();
            assert_eq!(json["game"]["seats"]["p1"]["playedRadiant"], json!([true, false]));
            assert_eq!(parse_game_record(&json).unwrap(), written);

            let bad_seat = with(
                &serde_json::to_value(seat(&["a", "b"], &[], &[], &["a", "b"])).unwrap(),
                "playedRadiant",
                json!([1]),
            );
            let bad_record = with(
                &json,
                "game",
                with(
                    &json["game"],
                    "seats",
                    with(&json["game"]["seats"], "p1", bad_seat),
                ),
            );
            assert!(refusal(&bad_record).contains("game.seats.p1.playedRadiant[0]"));
        }

        #[test]
        fn r1256000_reads_back_the_per_seat_counts_absent_on_older_records() {
            // A record written before the change holds none of the six keys, and still reads.
            let json = serde_json::to_value(record()).unwrap();
            assert!(
                !json["game"]["seats"]["p1"]
                    .as_object()
                    .unwrap()
                    .contains_key("fatigueDraws")
            );
            let read = parse_game_record(&json).unwrap();
            for seat in [read.game.seats.p1, read.game.seats.p2] {
                assert_eq!(seat.fatigue_draws, None);
                assert_eq!(seat.fatigue_damage, None);
                assert_eq!(seat.traps_set, None);
                assert_eq!(seat.traps_fired, None);
                assert_eq!(seat.position_switches, None);
                assert_eq!(seat.radiant_upgrades, None);
            }

            // A full round-trip keeps every count.
            let mut s = seat(&["a"], &[], &[], &[]);
            s.fatigue_draws = Some(8);
            s.fatigue_damage = Some(36);
            s.traps_set = Some(1);
            s.traps_fired = Some(1);
            s.position_switches = Some(2);
            s.radiant_upgrades = Some(3);
            let written = GameRecord {
                game: game(Winner::P2, GameOverReason::HeroDeath, 48, s, seat_default()),
                ..record()
            };
            let json = serde_json::to_value(&written).unwrap();
            assert_eq!(json["game"]["seats"]["p1"]["fatigueDraws"], json!(8));
            assert_eq!(json["game"]["seats"]["p1"]["radiantUpgrades"], json!(3));
            assert_eq!(parse_game_record(&json).unwrap(), written);
        }

        #[test]
        fn r1256000_refuses_a_count_that_is_not_a_non_negative_whole_number() {
            let json = serde_json::to_value(record()).unwrap();
            let bad = |key: &str, value: Value| {
                refusal(&with(
                    &json,
                    "game",
                    with(
                        &json["game"],
                        "seats",
                        with(
                            &json["game"]["seats"],
                            "p2",
                            with(&serde_json::to_value(seat_default()).unwrap(), key, value),
                        ),
                    ),
                ))
            };
            for key in [
                "fatigueDraws",
                "fatigueDamage",
                "trapsSet",
                "trapsFired",
                "positionSwitches",
                "radiantUpgrades",
            ] {
                assert!(
                    bad(key, json!(-1)).contains(&format!("game.seats.p2.{key}")),
                    "{key}"
                );
                assert!(
                    bad(key, json!(1.5)).contains(&format!("game.seats.p2.{key}")),
                    "{key}"
                );
                assert!(
                    bad(key, json!(true)).contains(&format!("game.seats.p2.{key}")),
                    "{key}"
                );
                assert!(
                    bad(key, json!("3")).contains(&format!("game.seats.p2.{key}")),
                    "{key}"
                );
            }
        }
    }

    /// R2089000: the Created cards' figures, kept apart from the deck table.
    mod r2089000_created_cards {
        use super::*;

        fn used(card: &str, turn: i32, radiant: bool) -> CreatedUse {
            CreatedUse {
                card: card.to_string(),
                turn,
                radiant,
            }
        }

        /// p1 (who wins) used two Created `t`s, one Radiant at its first use, and a Created `a`, a card
        /// its deck holds too, and made a `t` and a `u` it never used; p2 made a `u` it never used.
        fn made() -> GameRecord {
            let mut p1 = seat(&["a", "b"], &["a"], &[], &["a"]);
            p1.played_radiant = Some(vec![false]);
            p1.created_used = Some(vec![used("t", 2, false), used("t", 3, true), used("a", 4, false)]);
            p1.created_unused = Some(list(&["t", "u"]));
            p1.created_played = Some(vec![used("a", 4, false)]);
            let mut p2 = seat(&["b"], &[], &["b"], &[]);
            p2.created_unused = Some(list(&["u"]));
            GameRecord {
                game: game(Winner::P1, GameOverReason::HeroDeath, 9, p1, p2),
                ..record()
            }
        }

        /// The record as one written before the lists.
        fn without_lists(mut record: GameRecord) -> GameRecord {
            for seat in PLAYER_IDS {
                let summary = &mut record.game.seats[seat];
                summary.created_used = None;
                summary.created_unused = None;
                summary.created_played = None;
            }
            record
        }

        fn created_of<'a>(report: &'a CardStatsReport, card: &str) -> &'a CreatedStats {
            report
                .created
                .iter()
                .find(|stats| stats.card == card)
                .unwrap_or_else(|| panic!("no Created entry for {card}"))
        }

        #[test]
        fn r2089000_counts_each_created_card_once_per_seat_game_apart_from_the_deck_table() {
            let report = card_stats(&[made()], &live());
            let ids: Vec<&str> = report.created.iter().map(|stats| stats.card.as_str()).collect();
            assert_eq!(ids, ["a", "t", "u"]);

            // p1 used a `t`, so it counts once among the used however many it made, and as Radiant
            // since one was at its first use; the `t` it never used puts it in no baseline.
            let t = created_of(&report, "t");
            assert_eq!(t.created_played, tally(1, 1, 0));
            assert_eq!(t.created_played_radiant, tally(1, 1, 0));
            assert_eq!(t.created_played_normal, tally(0, 0, 0));
            assert_eq!(t.created_not_played, tally(0, 0, 0));
            assert_eq!(created_played_delta(t), None);
            let a = created_of(&report, "a");
            assert_eq!(a.created_played, tally(1, 1, 0));
            assert_eq!(a.created_played_normal, tally(1, 1, 0));
            // Both seats made a `u` and used none: two games, p1's won.
            let u = created_of(&report, "u");
            assert_eq!(u.created_not_played, tally(2, 1, 0));
            assert_eq!(u.created_played, tally(0, 0, 0));
            assert_eq!(u.games(), 2);

            // No deck held `t` or `u`, so neither has a deck row, and `a`'s is its dealt copies' alone,
            // written as a deck row was before the Created figures.
            let ids: Vec<&str> = report.cards.iter().map(|stats| stats.card.as_str()).collect();
            assert_eq!(ids, ["a", "b"]);
            let deck_a = stats_of(&report, "a");
            assert_eq!(deck_a.played, tally(1, 1, 0));
            assert_eq!(deck_a.played_normal, tally(1, 1, 0));
            assert_eq!(deck_a.created_played, tally(0, 0, 0));
            let json = serde_json::to_value(deck_a).unwrap();
            assert!(
                json.as_object()
                    .unwrap()
                    .keys()
                    .all(|key| !key.starts_with("created"))
            );
            assert_eq!((report.games, report.decks), (1, 2));

            // The delta is R377's over the used and the never-used seats.
            let mut both = u.clone();
            both.created_played = tally(4, 3, 0);
            assert_eq!(created_played_delta(&both), Some(0.75 - 0.5));
        }

        #[test]
        fn r2089000_a_record_without_the_lists_folds_as_before() {
            let plain = without_lists(made());
            let report = card_stats(std::slice::from_ref(&plain), &live());
            assert!(report.created.is_empty());
            assert_eq!(report.cards, card_stats(&[made()], &live()).cards);
            let mut totals = StatsTotals::default();
            totals.fold(&plain, true);
            assert!(
                totals
                    .cards
                    .values()
                    .flat_map(|cards| cards.keys())
                    .all(|card| card == "a" || card == "b"),
                "no row for a card no deck held"
            );
            // A seat with no deck and no Created card still adds no key.
            let mut empty = StatsTotals::default();
            empty.fold(
                &GameRecord {
                    game: game(
                        Winner::P1,
                        GameOverReason::Concede,
                        1,
                        SeatSummary::default(),
                        SeatSummary::default(),
                    ),
                    ..record()
                },
                true,
            );
            assert!(empty.cards.is_empty());
        }

        #[test]
        fn r2089000_parse_reads_the_lists_and_refuses_a_malformed_use() {
            let written = made();
            let json = serde_json::to_value(&written).unwrap();
            let p1 = &json["game"]["seats"]["p1"];
            assert_eq!(
                p1["createdUsed"][1],
                json!({ "card": "t", "turn": 3, "radiant": true })
            );
            assert_eq!(p1["createdUnused"], json!(["t", "u"]));
            assert_eq!(
                p1["createdPlayed"],
                json!([{ "card": "a", "turn": 4, "radiant": false }])
            );
            assert!(
                json["game"]["seats"]["p2"].get("createdUsed").is_none(),
                "an absent list is not written"
            );
            assert_eq!(parse_game_record(&json).unwrap(), written);
            assert_eq!(
                parse_game_record(&serde_json::to_value(without_lists(made())).unwrap()).unwrap(),
                without_lists(made())
            );

            let refusal = |key: &str, list: Value| {
                let mut bad = json.clone();
                bad["game"]["seats"]["p1"][key] = list;
                parse_game_record(&bad).unwrap_err().message
            };
            for bad in [
                json!([{ "card": "t", "turn": 0, "radiant": false }]),
                json!([{ "card": "t", "turn": 2 }]),
                json!([{ "card": "", "turn": 2, "radiant": false }]),
                json!(["t"]),
            ] {
                assert!(
                    refusal("createdUsed", bad.clone())
                        .contains("game.seats.p1.createdUsed[0] is not a Created card's use"),
                    "{bad}"
                );
            }
            assert!(
                refusal("createdPlayed", json!("a"))
                    .contains("game.seats.p1.createdPlayed is not a list of Created cards' uses")
            );
            assert!(refusal("createdUnused", json!("t")).contains("game.seats.p1.createdUnused"));
        }

        #[test]
        fn r2089000_format_card_stats_prints_the_created_table_after_the_deck_table() {
            let names: IndexMap<&str, &str> = [("t", "Token")].into_iter().collect();
            let name_of = |card: &str| names.get(card).map(|name| name.to_string());
            let report = card_stats(&[made()], &live());
            let text = format_card_stats(&report, name_of);
            // The deck's table is what it was without them, and the Created one follows it.
            let plain = format_card_stats(&card_stats(&[without_lists(made())], &live()), name_of);
            assert!(text.starts_with(&format!("{plain}\n\nCreated cards: ")));
            let lines: Vec<&str> = text[plain.len()..].split('\n').collect();
            let heading = lines.iter().position(|line| line.starts_with("Card ")).unwrap();
            assert_eq!(
                split_wide(lines[heading]),
                [
                    "Card",
                    "Created, used",
                    "Created, not used",
                    "Used Radiant",
                    "Used normal",
                    "Created Δ"
                ]
            );
            let token = lines.iter().find(|line| line.starts_with("t Token")).unwrap();
            assert_eq!(
                split_wide(token)[1..],
                ["100.0% (1)", "— (0)", "100.0% (1)", "— (0)", "—"]
            );
            let u = lines.iter().find(|line| line.starts_with("u ")).unwrap();
            assert_eq!(split_wide(u)[1..], ["— (0)", "50.0% (2)", "— (0)", "— (0)", "—"]);
            assert_eq!(lines.len(), heading + 4, "a row for each of a, t and u");
        }
    }

    #[test]
    fn r1284000_forced_series_is_some_only_for_a_record_filed_under_a_forced_series() {
        let forced = GameRecord {
            id: "dev:v0.2.3:dev-force-core-090:1".to_string(),
            patch: "v0.2.3".to_string(),
            ..record()
        };
        assert_eq!(forced_series(&forced), Some("dev-force-core-090"));
        let plain = GameRecord {
            id: "dev:v0.2.3:dev:1".to_string(),
            patch: "v0.2.3".to_string(),
            ..record()
        };
        assert_eq!(forced_series(&plain), None);
        assert_eq!(forced_series(&record()), None);
        let mismatched = GameRecord {
            id: "dev:run:1".to_string(),
            patch: "v0.2.3".to_string(),
            ..record()
        };
        assert_eq!(forced_series(&mismatched), None);
        // A mistyped patch cannot hide a forced series: the id alone decides.
        let spoofed = GameRecord {
            id: "dev:v0.2.3:dev-force-core-090:1".to_string(),
            patch: "v9.9.9".to_string(),
            ..record()
        };
        assert_eq!(forced_series(&spoofed), Some("dev-force-core-090"));
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
