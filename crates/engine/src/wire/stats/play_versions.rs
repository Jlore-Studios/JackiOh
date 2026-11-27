//! R4675000: a card's play versions, the runs of shipped versions over which it plays the same
//! (`jackioh_cards::play_versions` builds them from the compiled-in `patches.json`), and R4675001: each
//! one's figures, R377's breakdowns summed over the versions it spans, read against a field of every
//! play version's figures so versions far apart in time read on one scale.
//!
//! A play version is no window (R1367000): a major or a patch does not end it, only a change to how its
//! card plays. Its figures count live games once the live games played on its span reach R654's gate,
//! `PUBLIC_STATS_MIN_LIVE_GAMES`, and live and development games together below it, as a window short of
//! the gate is padded; a play version never borrows another's games, as a window borrows its previous
//! one's. No figure names its source (R661). A version string is compared for equality and never parsed
//! or ordered (R105).
//!
//! Its id is `<card>~<first version>` (`PLAY_VERSION_MARK`): `@` is a past version's (R3538002), and its
//! last version moves at every ship while it is live, its first never.
//!
//! R4678001: each row also counts the patches its span reaches, the windows every card shares
//! (`patch_windows`, R1973002) as the statistics page's patch bar lists them, which the Experimental
//! tab's Running view names beside the version a card's current play version began at. R5033000: and the
//! position of its first version in the shipped history, which the Date sort reads.
//!
//! R5037000: each row also reads the Cards tab's Win rate (R3185000) and Play rate (R3950000), plain
//! tallies summed over the span as the games are, never an average of its versions' rates.
//!
//! R5040000: `major_line_versions` cuts each live play version to the newest major's line;
//! `play_version_figures` over the cut versions is the Major patch running view's table, its field read
//! over the cut rows alone.
//!
//! R5035000: each play version's iteration, `v1`, `v1b`, `v2` (`jackioh_cards::play_versions_of` counts
//! it over its card's whole shipped history), which each row sends as its label.
//!
//! R5036000: each row's Decisiveness (R4552001) over its span, against one baseline of every record the
//! table counts.

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

use super::window::{Part, PatchStep, PatchTier, part_of, patch_windows, split_by_version};
use super::{
    CardStats, CardTier, CreatedStats, DECIMALS, DecisivenessRow, GameMode, GameSource, ModeFilter,
    NO_FIGURE, PERCENT, PilotFilter, StatsField, StatsTotals, SwingRows, SwingTally, Tally, add_created,
    add_stats, add_swing_rows, adjusted_interval, adjusted_win_rate, decisiveness_rows, format_decisiveness,
    held_adjusted, percent, play_rate, seen, stats_field, swing_baseline, tier_of, to_fixed, win_rate,
};
use crate::catalog::RULE_MADE_DEF_IDS;
use crate::config::{PUBLIC_STATS_MIN_LIVE_GAMES, TIER_BAND_HIGH, TIER_BAND_LOW};

/// R4675000: what joins a card's id to the first shipped version of a play version in its id.
pub const PLAY_VERSION_MARK: char = '~';

/// R5035000: what an iteration's label begins with: `v1`, `v1b`.
pub const ITERATION_MARK: char = 'v';

/// R5035000: a card's first play version's iteration.
pub const FIRST_ITERATION: u32 = 1;

/// R4675000: the id of `card`'s play version that begins at `first`.
pub fn play_version_id(card: &str, first: &str) -> String {
    format!("{card}{PLAY_VERSION_MARK}{first}")
}

/// R4675000: one run of shipped versions over which a card plays the same.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlayVersion {
    /// `play_version_id(card, first)`.
    pub id: String,
    pub card: String,
    /// The card's name at the last version it spans.
    pub name: String,
    /// The shipped versions it spans, oldest first, in `patches.json`'s order.
    pub versions: Vec<String>,
    /// Whether it is the card's version in the newest shipped catalog: its last version is still open.
    pub live: bool,
    /// R5035000: its number among its card's play versions, `FIRST_ITERATION` for the first: a new one
    /// where its card's face changed.
    pub iteration: u32,
    /// R5035000: its letters after the number (R3631000's, b to z, then aa), `""` for a number's first:
    /// the next ones where only its card's play changed.
    pub iteration_letters: String,
}

impl PlayVersion {
    /// The shipped version it begins at.
    pub fn first(&self) -> &str {
        self.versions.first().map_or("", String::as_str)
    }

    /// The last shipped version it spans: the newest shipped one while it is live.
    pub fn last(&self) -> &str {
        self.versions.last().map_or("", String::as_str)
    }

    /// R5035000: its iteration as the Experimental tab prints it: `v1`, `v1b`, `v2`.
    pub fn iteration_label(&self) -> String {
        format!("{ITERATION_MARK}{}{}", self.iteration, self.iteration_letters)
    }
}

/// R4675001: what `play_version_figures` reads.
#[derive(Clone, Debug)]
pub struct PlayVersionQuery<'a> {
    /// The play versions to read, in the order the table lists them.
    pub versions: &'a [PlayVersion],
    /// R4678001: every shipped version, in `patches.json`'s order (`jackioh_cards::patch_history`), whose
    /// shared windows (`patch_windows`) a row's `patches` counts.
    pub history: &'a [PatchStep],
    /// The seats of a live record counted (R807000); a development record always counts both (R378).
    pub live_pilot: PilotFilter,
    /// R377: the match types counted. R654's gate counts every mode's live games whatever it names.
    pub mode: ModeFilter,
    /// R654's floor: the games a tier needs, and the never-drawn games a drawn delta needs (R3070000).
    pub min_sample: i64,
}

/// R4675001: one play version's figures.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayVersionFigures {
    pub id: String,
    pub card: String,
    pub name: String,
    pub first: String,
    pub last: String,
    pub live: bool,
    /// R5035000: its play version's `iteration_label()`, the same however the table is read or cut.
    pub iteration: String,
    /// R4678001: the patches its versions reach: the windows every card shares (`patch_windows`,
    /// R1973002, a gameless patch folded into the one before as R1973001 folds it), counted once each.
    pub patches: usize,
    /// R5033000: the position of its first version in the shipped history (`PlayVersionQuery::history`,
    /// `patches.json`'s order, which is ship order), or `None` when the history does not hold it. The
    /// client sorts by it and never orders version names (R105).
    pub first_shipped: Option<usize>,
    /// The games its figures count: the decks that held it, under All Random the decks that saw it
    /// (R2777002), and for a card only a rule makes the seats that made one (R3070000).
    pub games: i64,
    /// R2777000, R3022000: its adjusted win rate, read against the table's field, and its 95% range (R2895000).
    pub adjusted_win_rate: Option<f64>,
    pub adjusted_low: Option<f64>,
    pub adjusted_high: Option<f64>,
    /// R5036000: R4552001's Decisiveness over its span; none below the floor or with no turns.
    pub decisiveness: Option<f64>,
    /// R3022000: its tier, only once it has the floor's games.
    pub tier: Option<CardTier>,
    pub has_enough_games: bool,
    /// R5037000: its win rate over the `games` it counts, as R3185000 reads a card's; `None` with no
    /// games, so a play version nobody held never reads as 0%. The floor (R654) is `has_enough_games`'s.
    pub win_rate: Option<f64>,
    /// R5037000: how often it was played when seen, its played games over its seen games (R3950000), a
    /// card only a rule makes its used Created games over the games a seat made one (R3070000); `None`
    /// when it was never seen, as 0% would say it was seen and never played.
    pub play_rate: Option<f64>,
}

/// R4675001: every play version's figures, and the field and band they are read against.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayVersionTable {
    /// R3022000: read from every play version's figures, so each is read beside every other.
    pub field: StatsField,
    pub band_low: f64,
    pub band_high: f64,
    /// R5036000: the typical play version's Decisiveness (R4552001's `m` over these rows), which the
    /// tooltip names.
    pub decisiveness: f64,
    pub min_sample: i64,
    pub versions: Vec<PlayVersionFigures>,
}

/// One shipped version's figures from each source, and the live games R654's gate counts.
struct VersionParts {
    live_games: i64,
    live: Part,
    dev: Part,
}

impl VersionParts {
    fn of(&self, source: GameSource) -> &Part {
        match source {
            GameSource::Live => &self.live,
            GameSource::Dev => &self.dev,
        }
    }
}

/// A play version's breakdowns over its span, its Created figures there (R2089000), and the record
/// sources its span counts (`span_sources`).
struct Summed {
    stats: Option<CardStats>,
    created: Option<CreatedStats>,
    sources: &'static [GameSource],
}

impl Summed {
    /// R5036000: the turns `card`'s Decisiveness is read off. A card only a rule makes reads its Created
    /// copies' turns (R4552002), as its games are the seats that made one (R3070000).
    fn swing_row<'a>(&'a self, card: &str, empty: &'a SwingTally) -> DecisivenessRow<'a> {
        match self.created.as_ref() {
            Some(created) if RULE_MADE_DEF_IDS.contains(&card) => DecisivenessRow {
                tally: &created.swing,
                made: true,
            },
            _ => DecisivenessRow {
                tally: self.stats.as_ref().map_or(empty, |stats| stats.swing.or(empty)),
                made: false,
            },
        }
    }
}

/// The record sources a span counts: live alone once its live games reach R654's gate, else live and
/// development together (R1367000's provisional figures).
fn span_sources(live_games: i64) -> &'static [GameSource] {
    if live_games >= PUBLIC_STATS_MIN_LIVE_GAMES {
        &[GameSource::Live]
    } else {
        &[GameSource::Live, GameSource::Dev]
    }
}

fn summed(version: &PlayVersion, parts: &IndexMap<&str, VersionParts>) -> Summed {
    let spanned: Vec<&VersionParts> = version
        .versions
        .iter()
        .filter_map(|shipped| parts.get(shipped.as_str()))
        .collect();
    let live_games = spanned.iter().map(|parts| parts.live_games).sum();
    let sources = span_sources(live_games);
    let mut stats: Option<CardStats> = None;
    let mut created: Option<CreatedStats> = None;
    for parts in spanned {
        for source in sources {
            let part = parts.of(*source);
            if let Some(card) = part.cards.get(&version.card) {
                match stats.as_mut() {
                    None => stats = Some(card.clone()),
                    Some(held) => add_stats(held, card),
                }
            }
            if let Some(card) = part.created.get(&version.card) {
                match created.as_mut() {
                    None => created = Some(card.clone()),
                    Some(held) => add_created(held, card),
                }
            }
        }
    }
    Summed {
        stats,
        created,
        sources,
    }
}

/// R5036000: every turn of the records the table counts: each (version, source) a row's span counts,
/// once; in the modes `mode` reads; every pilot, as `windowed_swing_rows` reads a window's.
fn counted_swing_rows(
    totals: &StatsTotals,
    versions: &[PlayVersion],
    sums: &[Summed],
    mode: ModeFilter,
) -> SwingRows {
    let counted: IndexSet<(&str, GameSource)> = versions
        .iter()
        .zip(sums)
        .flat_map(|(version, sum)| {
            version
                .versions
                .iter()
                .flat_map(|shipped| sum.sources.iter().map(move |source| (shipped.as_str(), *source)))
        })
        .collect();
    let mut rows = SwingRows::new();
    for (key, total) in &totals.game_totals {
        if counted.contains(&(key.patch.as_str(), key.source)) && mode.counts(key.mode) {
            add_swing_rows(&mut rows, &total.swing);
        }
    }
    rows
}

/// R4675001: every play version's figures over `totals`: R377's breakdowns of its card summed over the
/// versions it spans and the sources its span counts (`span_sources`), its adjusted win rate, range and
/// tier read as `GET /api/stats/cards` reads a card's (R2777000, R2895000, R3022000, R3070000), against
/// a field read from every play version the query names. A version no record was filed under, every
/// one before the records began (R376), adds nothing: a play version wholly before them has no games
/// and no figure.
pub fn play_version_figures(totals: &StatsTotals, query: &PlayVersionQuery) -> PlayVersionTable {
    let parts: IndexMap<&str, VersionParts> = split_by_version(totals)
        .iter()
        .map(|(version, held)| {
            let part = |source| part_of(held, version, source, query.live_pilot, query.mode);
            (
                *version,
                VersionParts {
                    live_games: held.live_games(version),
                    live: part(GameSource::Live),
                    dev: part(GameSource::Dev),
                },
            )
        })
        .collect();
    let sums: Vec<Summed> = query
        .versions
        .iter()
        .map(|version| summed(version, &parts))
        .collect();
    let floor = i32::try_from(query.min_sample).unwrap_or(i32::MAX);
    let field = stats_field(sums.iter().filter_map(|sum| sum.stats.as_ref()), floor);
    // R5036000: one baseline for every row, as one field is.
    let empty = SwingTally::default();
    let rows: Vec<DecisivenessRow> = query
        .versions
        .iter()
        .zip(&sums)
        .map(|(version, sum)| sum.swing_row(&version.card, &empty))
        .collect();
    let baseline = swing_baseline(&counted_swing_rows(totals, query.versions, &sums, query.mode));
    let (decisive_field, decisive) = decisiveness_rows(&rows, &baseline);
    // R4678001: each shipped version's window, as the patch bar lists them (R1973002).
    let windows = patch_windows(
        query.history,
        |version| totals.games(version),
        |version| totals.live_games(version),
    );
    let window_of: IndexMap<&str, usize> = windows
        .iter()
        .enumerate()
        .flat_map(|(index, window)| {
            window
                .versions
                .iter()
                .map(move |version| (version.as_str(), index))
        })
        .collect();
    // R5033000: each shipped version's position, which is the order the versions shipped in.
    let position_of: IndexMap<&str, usize> = query
        .history
        .iter()
        .enumerate()
        .map(|(index, step)| (step.version.as_str(), index))
        .collect();
    let versions = query
        .versions
        .iter()
        .zip(&sums)
        .zip(&decisive)
        .map(|((version, sum), decisiveness)| {
            figures_of(
                version,
                sum,
                &field,
                query,
                &window_of,
                &position_of,
                *decisiveness,
            )
        })
        .collect();
    PlayVersionTable {
        field,
        band_low: TIER_BAND_LOW,
        band_high: TIER_BAND_HIGH,
        decisiveness: decisive_field.typical,
        min_sample: query.min_sample,
        versions,
    }
}

/// R5040000: the Major patch running view's play versions: the live ones of `versions`, each cut to its
/// versions in the newest major's line, from the last `PatchTier::Major` step of `history` through its
/// newest, in `history`'s order (R105: a version is matched, never parsed). A closed play version has no
/// row; a cut one keeps its id, card, name and `live`, so it begins at the later of its own first version
/// and the major. With no major in `history` the line is all of it.
pub fn major_line_versions(versions: &[PlayVersion], history: &[PatchStep]) -> Vec<PlayVersion> {
    let from = history
        .iter()
        .rposition(|step| step.tier == PatchTier::Major)
        .unwrap_or(0);
    let line: IndexSet<&str> = history[from..].iter().map(|step| step.version.as_str()).collect();
    versions
        .iter()
        .filter(|version| version.live)
        .filter_map(|version| {
            let cut: Vec<String> = version
                .versions
                .iter()
                .filter(|shipped| line.contains(shipped.as_str()))
                .cloned()
                .collect();
            (!cut.is_empty()).then(|| PlayVersion {
                versions: cut,
                ..version.clone()
            })
        })
        .collect()
}

/// R4678001: how many of the shared windows `window_of` names `version`'s versions reach. A play
/// version's versions run in order and a window's are consecutive, so each window is a run of them.
fn patches_of(version: &PlayVersion, window_of: &IndexMap<&str, usize>) -> usize {
    let mut reached: Vec<usize> = version
        .versions
        .iter()
        .filter_map(|shipped| window_of.get(shipped.as_str()).copied())
        .collect();
    reached.dedup();
    reached.len()
}

/// One play version's row, as `get_cards` reads a card's.
fn figures_of(
    version: &PlayVersion,
    sum: &Summed,
    field: &StatsField,
    query: &PlayVersionQuery,
    window_of: &IndexMap<&str, usize>,
    position_of: &IndexMap<&str, usize>,
    decisiveness: Option<f64>,
) -> PlayVersionFigures {
    let stats = sum.stats.as_ref();
    // R3070000: a card only a rule makes is in no deck: it reads the seats that made one (R2089000).
    let made = RULE_MADE_DEF_IDS
        .contains(&version.card.as_str())
        .then(|| sum.created.as_ref().map(CreatedStats::made))
        .flatten();
    // R2777002: under All Random, the decks that saw the card.
    let counted: Tally = made.unwrap_or_else(|| {
        stats
            .map(|stats| match query.mode {
                ModeFilter::Only(GameMode::Random) => seen(stats),
                _ => stats.in_deck,
            })
            .unwrap_or_default()
    });
    let games = i64::from(counted.games);
    let has_enough_games = games >= query.min_sample;
    let rate = win_rate(&counted);
    let played = play_rate(stats, sum.created.as_ref(), made.as_ref());
    let (adjusted, range) = match made {
        Some(made) => {
            held_adjusted(&made, field).map_or((None, None), |(rate, range)| (Some(rate), Some(range)))
        }
        None => (
            stats.and_then(|stats| adjusted_win_rate(stats, field)),
            stats.and_then(|stats| adjusted_interval(stats, field)),
        ),
    };
    let tier = adjusted
        .zip(range)
        .filter(|_| has_enough_games)
        .map(|(rate, (low, high))| tier_of(rate, low, high));
    PlayVersionFigures {
        id: version.id.clone(),
        card: version.card.clone(),
        name: version.name.clone(),
        first: version.first().to_string(),
        last: version.last().to_string(),
        live: version.live,
        iteration: version.iteration_label(),
        patches: patches_of(version, window_of),
        first_shipped: position_of.get(version.first()).copied(),
        games,
        adjusted_win_rate: adjusted,
        adjusted_low: range.map(|(low, _)| low),
        adjusted_high: range.map(|(_, high)| high),
        decisiveness: decisiveness.filter(|_| has_enough_games),
        tier,
        has_enough_games,
        win_rate: rate,
        play_rate: played,
    }
}

/// R4675001: the table as text: the field and band, one line per play version, and how many have games
/// and the floor's games, live and closed.
pub fn format_play_versions(table: &PlayVersionTable) -> String {
    let field = &table.field;
    let mut lines = vec![format!(
        "Play versions read against a field of {} play versions (centre {} points, spread {} points); band {}–{}; a typical play version's Decisiveness {}; a tier from {} games.",
        field.cards,
        to_fixed(field.center * PERCENT, DECIMALS),
        to_fixed(field.spread * PERCENT, DECIMALS),
        percent(Some(table.band_low)),
        percent(Some(table.band_high)),
        format_decisiveness(Some(table.decisiveness)),
        table.min_sample
    )];
    for row in &table.versions {
        lines.push(format!(
            "{} {} {}..{}{} games {} adjusted {} ({}–{}) decisiveness {} tier {}",
            row.id,
            row.name,
            row.first,
            row.last,
            if row.live { " live" } else { "" },
            row.games,
            percent(row.adjusted_win_rate),
            percent(row.adjusted_low),
            percent(row.adjusted_high),
            format_decisiveness(row.decisiveness),
            row.tier.map_or(NO_FIGURE, CardTier::as_str)
        ));
    }
    let mut cards: Vec<&str> = table.versions.iter().map(|row| row.card.as_str()).collect();
    cards.sort_unstable();
    cards.dedup();
    let count =
        |keep: &dyn Fn(&PlayVersionFigures) -> bool| table.versions.iter().filter(|row| keep(row)).count();
    lines.push(format!(
        "{} play versions of {} cards ({} live); {} with games; {} with {} games or more ({} live, {} closed).",
        table.versions.len(),
        cards.len(),
        count(&|row| row.live),
        count(&|row| row.games > 0),
        count(&|row| row.has_enough_games),
        table.min_sample,
        count(&|row| row.has_enough_games && row.live),
        count(&|row| row.has_enough_games && !row.live)
    ));
    lines.join("\n")
}

// R4675001: figures over totals folded from records written by hand, so each sum can be read off the test.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DECISIVENESS_PRIOR_CENTER, DORMANT_STARSHIP_DEF_ID};
    use crate::wire::catalog_types::{PerPlayer, PlayerId};
    use crate::wire::events::{GameOverReason, Winner};
    use crate::wire::stats::{
        CardStatsFilter, CreatedUse, GameRecord, GameSummary, PatchTier, Pilot, RecordsKey, SeatSummary,
        SourceFilter, SwingReport, SwingTurn, WindowQuery, WindowRequest, card_stats_of, swing_report,
        swing_rows_of, windowed_card_stats, windowed_swing_rows,
    };

    const A: &str = "core-001";
    const B: &str = "core-002";
    const C: &str = "core-003";
    const D: &str = "core-004";
    /// The card the opponent holds, which no play version reads.
    const OTHER: &str = "core-099";

    fn version(card: &str, versions: &[&str], live: bool) -> PlayVersion {
        PlayVersion {
            id: play_version_id(card, versions[0]),
            card: card.to_string(),
            name: format!("{card} name"),
            versions: versions.iter().map(|version| version.to_string()).collect(),
            live,
            iteration: FIRST_ITERATION,
            iteration_letters: String::new(),
        }
    }

    /// A game on `patch` holding `deck`, which p1 wins when `won`; the opponent holds `OTHER`.
    fn game(id: &str, source: GameSource, patch: &str, deck: &[&str], won: bool) -> GameRecord {
        let seat = |deck: &[&str]| SeatSummary {
            deck: deck.iter().map(|id| id.to_string()).collect(),
            opening: deck.iter().map(|id| id.to_string()).collect(),
            ..SeatSummary::default()
        };
        GameRecord {
            id: id.to_string(),
            source,
            mode: GameMode::Bo1,
            patch: patch.to_string(),
            pilots: PerPlayer::new(Pilot::Human, Pilot::Human),
            game: GameSummary {
                first: PlayerId::P1,
                winner: if won { Winner::P1 } else { Winner::P2 },
                reason: GameOverReason::HeroDeath,
                turns: 5,
                seats: PerPlayer::new(seat(deck), seat(&[OTHER])),
            },
            build: None,
        }
    }

    /// Live games on `patch` that hold no card, so only R654's gate reads them.
    fn add_live(totals: &mut StatsTotals, patch: &str, games: i64) {
        *totals
            .records
            .entry(RecordsKey {
                patch: patch.to_string(),
                source: GameSource::Live,
                mode: GameMode::Bo1,
                pilots: (Pilot::Human, Pilot::Human),
            })
            .or_default() += games;
    }

    fn folded(records: &[GameRecord]) -> StatsTotals {
        let mut totals = StatsTotals::default();
        for record in records {
            totals.fold(record, false);
        }
        totals
    }

    /// Every source, mode, patch and seat.
    fn all() -> CardStatsFilter {
        CardStatsFilter {
            source: SourceFilter::All,
            mode: None,
            patch: None,
            pilot: PilotFilter::Unified,
        }
    }

    /// `card`'s breakdowns over `records` alone, every source, mode and patch: a sum folded by hand.
    fn by_hand(records: &[GameRecord], card: &str) -> CardStats {
        card_stats_of(&folded(records), &all())
            .cards
            .into_iter()
            .find(|stats| stats.card == card)
            .expect("the card is held")
    }

    fn query(versions: &[PlayVersion], min_sample: i64) -> PlayVersionQuery<'_> {
        PlayVersionQuery {
            versions,
            history: &[],
            live_pilot: PilotFilter::Unified,
            mode: ModeFilter::All,
            min_sample,
        }
    }

    fn row<'a>(table: &'a PlayVersionTable, id: &str) -> &'a PlayVersionFigures {
        table
            .versions
            .iter()
            .find(|row| row.id == id)
            .expect("the play version has a row")
    }

    #[test]
    fn r4675001_a_versions_figures_sum_its_span_and_read_against_every_versions_field() {
        let a1 = version(A, &["v1", "v2"], false);
        let a2 = version(A, &["v3"], true);
        let b1 = version(B, &["v1", "v2", "v3"], true);
        let versions = [a1.clone(), a2.clone(), b1.clone()];
        let on_v1 = [
            game("m1", GameSource::Live, "v1", &[A, B], true),
            game("m2", GameSource::Live, "v1", &[A], false),
        ];
        let on_v2 = [game("m3", GameSource::Live, "v2", &[A, B], true)];
        let on_v3 = [game("m4", GameSource::Live, "v3", &[A, B], false)];
        let dev_v1 = [game("dev:v1:s:1", GameSource::Dev, "v1", &[A, B], true)];
        let dev_v3 = [
            game("dev:v3:s:1", GameSource::Dev, "v3", &[A, B], true),
            game("dev:v3:s:2", GameSource::Dev, "v3", &[A], false),
        ];
        let every: Vec<GameRecord> = [&on_v1[..], &on_v2, &on_v3, &dev_v1, &dev_v3].concat();
        let mut totals = folded(&every);
        // v1's games reach R654's gate: A1 and B1 count live games alone; A2's span, v3, is short.
        add_live(&mut totals, "v1", PUBLIC_STATS_MIN_LIVE_GAMES);

        let table = play_version_figures(&totals, &query(&versions, 2));
        assert_eq!(
            table
                .versions
                .iter()
                .map(|row| row.id.as_str())
                .collect::<Vec<_>>(),
            ["core-001~v1", "core-001~v3", "core-002~v1"]
        );
        // A1: v1's and v2's live games, v1's development game left out.
        let a1_sum = by_hand(&[&on_v1[..], &on_v2].concat(), A);
        // A2: v3 is short, so its development games count beside its live one.
        let a2_sum = by_hand(&[&on_v3[..], &dev_v3].concat(), A);
        // B1 spans every version and clears the gate: live games alone, v3's development game left out.
        let b1_sum = by_hand(&[&on_v1[..], &on_v2, &on_v3].concat(), B);
        assert_eq!(
            (a1_sum.in_deck.games, a2_sum.in_deck.games, b1_sum.in_deck.games),
            (3, 3, 3)
        );
        let field = stats_field([&a1_sum, &a2_sum, &b1_sum], 2);
        assert_eq!(table.field, field);
        // The field is read from every play version's sum: A2's losses move its baseline.
        assert_ne!(table.field, stats_field([&a1_sum, &b1_sum], 2));
        for (play, sum) in [(&a1, &a1_sum), (&a2, &a2_sum), (&b1, &b1_sum)] {
            let row = row(&table, &play.id);
            assert_eq!(row.games, i64::from(sum.in_deck.games), "{}", play.id);
            assert_eq!(
                row.adjusted_win_rate,
                adjusted_win_rate(sum, &field),
                "{}",
                play.id
            );
            let range = adjusted_interval(sum, &field);
            assert_eq!(
                (row.adjusted_low, row.adjusted_high),
                (range.map(|(low, _)| low), range.map(|(_, high)| high))
            );
            assert!(row.has_enough_games);
            assert!(row.tier.is_some(), "{}", play.id);
            assert_eq!(
                (row.first.as_str(), row.last.as_str()),
                (play.first(), play.last())
            );
            assert_eq!(row.live, play.live);
        }
        assert_eq!((table.band_low, table.band_high), (TIER_BAND_LOW, TIER_BAND_HIGH));

        // Below the floor a play version keeps its rate and has no tier.
        let table = play_version_figures(&totals, &query(&versions, 4));
        let a1_row = row(&table, &a1.id);
        assert!(!a1_row.has_enough_games);
        assert!(a1_row.adjusted_win_rate.is_some());
        assert_eq!(a1_row.tier, None);
    }

    #[test]
    fn r4675001_a_version_before_the_records_has_no_figures() {
        let old = version(A, &["v0", "v0b"], false);
        let now = version(A, &["v1"], true);
        let versions = [old.clone(), now.clone()];
        let totals = folded(&[game("m1", GameSource::Live, "v1", &[A], true)]);
        let table = play_version_figures(&totals, &query(&versions, 1));
        let row = row(&table, &old.id);
        assert_eq!(row.games, 0);
        assert_eq!(
            (
                row.adjusted_win_rate,
                row.adjusted_low,
                row.adjusted_high,
                row.tier
            ),
            (None, None, None, None)
        );
        assert!(!row.has_enough_games);
        assert_eq!(table.versions[1].games, 1);
    }

    // R3070000: the Dormant Starship is in no deck, so its play version reads the seats that made one.
    #[test]
    fn r4675001_a_card_only_a_rule_makes_reads_the_seats_that_made_one() {
        let starship = version(DORMANT_STARSHIP_DEF_ID, &["v1"], true);
        let mut made = game("m1", GameSource::Live, "v1", &[A], true);
        made.game.seats.p1.created_unused = Some(vec![DORMANT_STARSHIP_DEF_ID.to_string()]);
        let totals = folded(&[made]);
        let table = play_version_figures(&totals, &query(std::slice::from_ref(&starship), 1));
        let row = &table.versions[0];
        assert_eq!(row.games, 1);
        let held = held_adjusted(
            &Tally {
                games: 1,
                wins: 1,
                draws: 0,
            },
            &table.field,
        )
        .expect("a seat made one");
        assert_eq!(row.adjusted_win_rate, Some(held.0));
        assert!(row.tier.is_some());
    }

    #[test]
    fn r4675001_format_play_versions_lists_each_version_and_counts_those_with_enough_games() {
        let versions = [
            version(A, &["v0"], false),
            version(A, &["v1", "v2"], true),
            version(B, &["v0", "v1", "v2"], true),
        ];
        let totals = folded(&[
            game("m1", GameSource::Live, "v1", &[A, B], true),
            game("m2", GameSource::Live, "v2", &[A], true),
        ]);
        let text = format_play_versions(&play_version_figures(&totals, &query(&versions, 2)));
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 1 + versions.len() + 1, "{text}");
        assert!(
            lines[0].starts_with("Play versions read against a field of 0 play versions"),
            "{text}"
        );
        assert!(lines[0].ends_with("a tier from 2 games."), "{text}");
        assert_eq!(
            lines[1],
            format!(
                "core-001~v0 core-001 name v0..v0 games 0 adjusted {NO_FIGURE} ({NO_FIGURE}–{NO_FIGURE}) decisiveness — tier {NO_FIGURE}"
            )
        );
        assert!(
            lines[2].starts_with("core-001~v1 core-001 name v1..v2 live games 2 adjusted "),
            "{text}"
        );
        assert!(!lines[2].ends_with(&format!("tier {NO_FIGURE}")), "{text}");
        assert!(
            lines[3].starts_with("core-002~v0 core-002 name v0..v2 live games 1 "),
            "{text}"
        );
        assert!(lines[3].ends_with(&format!("tier {NO_FIGURE}")), "{text}");
        assert_eq!(
            lines[4],
            "3 play versions of 2 cards (2 live); 2 with games; 1 with 2 games or more (1 live, 0 closed)."
        );
    }

    // R4678001: a row counts the shared windows its versions reach, as the patch bar lists them: a micro is
    // no patch of its own, and a patch no game was played on before the next replaced it folds into the
    // one before (R1973001).
    #[test]
    fn r4678001_a_row_counts_the_patches_its_versions_reach() {
        let step = |version: &str, tier: PatchTier| PatchStep {
            version: version.to_string(),
            tier,
            plays: Vec::new(),
        };
        let history = [
            step("v1", PatchTier::Major),
            step("v1b", PatchTier::Micro),
            step("v2", PatchTier::Patch),
            step("v3", PatchTier::Patch),
            step("v4", PatchTier::Patch),
            step("v4b", PatchTier::Micro),
        ];
        // No game on v3 before v4 replaced it: the windows are v1–v1b, v2–v3 and v4–v4b.
        let totals = folded(&[
            game("m1", GameSource::Live, "v1", &[A, B], true),
            game("m2", GameSource::Live, "v2", &[A, B], false),
            game("m3", GameSource::Live, "v4", &[A, B], true),
        ]);
        let versions = [
            version(A, &["v1"], false),
            version(A, &["v1b", "v2", "v3", "v4", "v4b"], true),
            version(B, &["v1", "v1b", "v2", "v3", "v4", "v4b"], true),
            version("core-003", &["v3", "v4", "v4b"], true),
            version("core-004", &["v4b"], true),
            version("core-005", &["v0"], false),
        ];
        let read = |history: &[PatchStep]| -> Vec<usize> {
            let query = PlayVersionQuery {
                history,
                ..query(&versions, 1)
            };
            play_version_figures(&totals, &query)
                .versions
                .iter()
                .map(|row| row.patches)
                .collect()
        };
        assert_eq!(read(&history), [1, 3, 3, 2, 1, 0]);
        // With a game on v3 it is a patch of its own, and every span across it reaches one more.
        let mut on_v3 = totals.clone();
        add_live(&mut on_v3, "v3", 1);
        let query = PlayVersionQuery {
            history: &history,
            ..query(&versions, 1)
        };
        let reached: Vec<usize> = play_version_figures(&on_v3, &query)
            .versions
            .iter()
            .map(|row| row.patches)
            .collect();
        assert_eq!(reached, [1, 4, 4, 2, 1, 0]);
        // No history names no window.
        assert_eq!(read(&[]), [0; 6]);
    }

    /// A live game on `patch` holding `deck`, which p1 wins when `won`, where p1 played `played` and, with
    /// `seen` false, never had any card in hand.
    fn played_game(
        id: &str,
        patch: &str,
        deck: &[&str],
        won: bool,
        played: &[&str],
        seen: bool,
    ) -> GameRecord {
        let mut record = game(id, GameSource::Live, patch, deck, won);
        record.game.seats.p1.played = played.iter().map(|card| card.to_string()).collect();
        if !seen {
            record.game.seats.p1.opening = vec![];
        }
        record
    }

    // R5037000: a one-version play version reads the Cards tab's figures for the same records: R3185000's
    // win rate over the decks that held the card, R3950000's played over seen.
    #[test]
    fn r5037000_a_one_version_play_versions_win_rate_and_play_rate_are_the_cards_tabs() {
        let a = version(A, &["v1"], true);
        let records = [
            played_game("m1", "v1", &[A], true, &[A], true),
            played_game("m2", "v1", &[A], false, &[], true),
            played_game("m3", "v1", &[A], true, &[], false),
            played_game("m4", "v1", &[A], false, &[A], true),
        ];
        let table = play_version_figures(&folded(&records), &query(std::slice::from_ref(&a), 1));
        let row = row(&table, &a.id);
        assert_eq!(row.games, 4);
        assert_eq!(row.win_rate, Some(0.5));
        // Played in two of the three games that saw it; the fourth never had it in hand.
        assert_eq!(row.play_rate, Some(2.0 / 3.0));
        let card = by_hand(&records, A);
        assert_eq!(row.win_rate, win_rate(&card.in_deck));
        assert_eq!(row.play_rate, play_rate(Some(&card), None, None));
    }

    // R5037000: a span sums its versions' tallies, so its rates differ from the mean of theirs.
    #[test]
    fn r5037000_a_two_version_span_reads_the_summed_tallies() {
        let span = version(A, &["v1", "v2"], true);
        let first = version(B, &["v1"], false);
        let versions = [span.clone(), first.clone()];
        let records = [
            played_game("m1", "v1", &[A, B], true, &[A, B], true),
            played_game("m2", "v2", &[A], false, &[A], true),
            played_game("m3", "v2", &[A], false, &[], true),
            played_game("m4", "v2", &[A], false, &[], false),
        ];
        let table = play_version_figures(&folded(&records), &query(&versions, 1));
        let row_a = row(&table, &span.id);
        // One win in four games, and two plays in the three games that saw it.
        assert_eq!(
            (row_a.games, row_a.win_rate, row_a.play_rate),
            (4, Some(0.25), Some(2.0 / 3.0))
        );
        // v1 alone reads 100% and 100%, v2 alone 0% and 50%: the mean of the two would be 50% and 75%.
        let v1 = row(&table, &first.id);
        assert_eq!((v1.win_rate, v1.play_rate), (Some(1.0), Some(1.0)));
        let card = by_hand(&records, A);
        assert_eq!(row_a.win_rate, win_rate(&card.in_deck));
        assert_eq!(row_a.play_rate, play_rate(Some(&card), None, None));
    }

    // R5037000, R3070000: the Dormant Starship is in no deck: it reads its used games over the games a
    // seat made one.
    #[test]
    fn r5037000_a_card_only_a_rule_makes_reads_its_used_games_over_its_made_games() {
        let starship = version(DORMANT_STARSHIP_DEF_ID, &["v1"], true);
        let card = DORMANT_STARSHIP_DEF_ID.to_string();
        let mut used = game("m1", GameSource::Live, "v1", &[A], true);
        used.game.seats.p1.created_used = Some(vec![CreatedUse {
            card: card.clone(),
            turn: 5,
            radiant: false,
            variant: None,
            source: None,
        }]);
        let mut unused_won = game("m2", GameSource::Live, "v1", &[A], true);
        unused_won.game.seats.p1.created_unused = Some(vec![card.clone()]);
        let mut unused_lost = game("m3", GameSource::Live, "v1", &[A], false);
        unused_lost.game.seats.p1.created_unused = Some(vec![card]);
        let table = play_version_figures(
            &folded(&[used, unused_won, unused_lost]),
            &query(std::slice::from_ref(&starship), 1),
        );
        let row = &table.versions[0];
        assert_eq!(row.games, 3);
        assert_eq!(row.win_rate, Some(2.0 / 3.0));
        assert_eq!(row.play_rate, Some(1.0 / 3.0));
    }

    // R5037000: a play version nobody saw has a win rate and no play rate, which 0% would misread as seen
    // and never played; one wholly before the records has neither.
    #[test]
    fn r5037000_a_version_never_seen_reads_no_play_rate_and_one_before_the_records_reads_none() {
        let old = version(A, &["v0"], false);
        let now = version(A, &["v1"], true);
        let versions = [old.clone(), now.clone()];
        let records = [
            played_game("m1", "v1", &[A], true, &[], false),
            played_game("m2", "v1", &[A], false, &[], false),
        ];
        let table = play_version_figures(&folded(&records), &query(&versions, 1));
        let seen_never = row(&table, &now.id);
        assert_eq!((seen_never.games, seen_never.win_rate), (2, Some(0.5)));
        assert_eq!(seen_never.play_rate, None);
        let before = row(&table, &old.id);
        assert_eq!((before.games, before.win_rate, before.play_rate), (0, None, None));
    }

    // R5033000: a row's `first_shipped` is its first version's position in the history, ship order and never
    // name order: v0.6.12b shipped before v0.6.4d here.
    #[test]
    fn r5033000_a_rows_first_shipped_is_its_first_versions_position_in_the_history() {
        let step = |version: &str, tier: PatchTier| PatchStep {
            version: version.to_string(),
            tier,
            plays: Vec::new(),
        };
        let history = [
            step("v0.6.0", PatchTier::Major),
            step("v0.6.12b", PatchTier::Micro),
            step("v0.6.4d", PatchTier::Micro),
        ];
        let versions = [
            version(A, &["v0.6.0"], false),
            version(A, &["v0.6.12b", "v0.6.4d"], true),
            version(B, &["v0.6.4d"], true),
            version(B, &["v9"], true),
        ];
        let query = PlayVersionQuery {
            history: &history,
            ..query(&versions, 1)
        };
        let positions: Vec<Option<usize>> = play_version_figures(&folded(&[]), &query)
            .versions
            .iter()
            .map(|row| row.first_shipped)
            .collect();
        assert_eq!(positions, [Some(0), Some(1), Some(2), None]);
    }

    #[test]
    fn r5033000_play_versions_shipped_on_one_day_keep_their_ship_order() {
        let step = |version: &str, tier: PatchTier| PatchStep {
            version: version.to_string(),
            tier,
            plays: Vec::new(),
        };
        let history = [
            step("v1", PatchTier::Major),
            step("v1b", PatchTier::Micro),
            step("v1c", PatchTier::Micro),
        ];
        let versions = [
            version(A, &["v1c"], true),
            version(B, &["v1b"], true),
            version(A, &["v1"], false),
        ];
        let read = |history: &[PatchStep]| -> Vec<(String, Option<usize>)> {
            let query = PlayVersionQuery {
                history,
                ..query(&versions, 1)
            };
            play_version_figures(&folded(&[]), &query)
                .versions
                .into_iter()
                .map(|row| (row.first, row.first_shipped))
                .collect()
        };
        let owned = |rows: &[(&str, Option<usize>)]| -> Vec<(String, Option<usize>)> {
            rows.iter()
                .map(|(first, position)| ((*first).to_string(), *position))
                .collect()
        };
        // Micros of one day are told apart by position, and the table keeps the query's order.
        assert_eq!(
            read(&history),
            owned(&[("v1c", Some(2)), ("v1b", Some(1)), ("v1", Some(0))])
        );
        // No history holds none.
        assert_eq!(read(&[]), owned(&[("v1c", None), ("v1b", None), ("v1", None)]));
    }

    fn shipped(version: &str, tier: PatchTier) -> PatchStep {
        PatchStep {
            version: version.to_string(),
            tier,
            plays: Vec::new(),
        }
    }

    /// R5040000: a history written by hand with two majors, v1 and v3.
    fn two_majors() -> Vec<PatchStep> {
        vec![
            shipped("v1", PatchTier::Major),
            shipped("v1b", PatchTier::Micro),
            shipped("v2", PatchTier::Patch),
            shipped("v3", PatchTier::Major),
            shipped("v3b", PatchTier::Micro),
            shipped("v4", PatchTier::Patch),
        ]
    }

    /// R5040000: A spans both lines; B changed at v2 and its live version crosses v3; C began after v3;
    /// D's version closed inside the newer line.
    fn line_versions() -> [PlayVersion; 5] {
        [
            version(A, &["v1", "v1b", "v2", "v3", "v3b", "v4"], true),
            version(B, &["v1", "v1b"], false),
            version(B, &["v2", "v3", "v3b", "v4"], true),
            version(C, &["v3b", "v4"], true),
            version(D, &["v2", "v3"], false),
        ]
    }

    /// R5040000: the games before the newer major, and those from it on.
    fn line_games() -> (Vec<GameRecord>, Vec<GameRecord>) {
        let before = vec![
            game("m1", GameSource::Live, "v1", &[A, B], true),
            game("m2", GameSource::Live, "v2", &[A, B, D], false),
        ];
        let after = vec![
            game("m3", GameSource::Live, "v3", &[A, B, D], true),
            game("m4", GameSource::Live, "v3b", &[A, B, C], true),
            game("m5", GameSource::Live, "v4", &[A, B, C], false),
        ];
        (before, after)
    }

    fn ids(versions: &[PlayVersion]) -> Vec<&str> {
        versions.iter().map(|version| version.id.as_str()).collect()
    }

    // R5040000: Major patch running cuts each live play version to the newest major's line: one begun
    // before the major counts from it, its games, first version and patches read off the cut span alone,
    // and the field is read over the cut rows.
    #[test]
    fn r5040000_a_version_begun_before_the_newest_major_is_cut_to_start_at_it_and_read_off_the_cut_span_alone()
     {
        let history = two_majors();
        let (before, after) = line_games();
        let totals = folded(&[&before[..], &after].concat());
        let every = line_versions();
        let cut = major_line_versions(&every, &history);
        assert_eq!(ids(&cut), ["core-001~v1", "core-002~v2", "core-003~v3b"]);
        assert!(cut.iter().all(|version| version.live));
        assert_eq!(cut[0].versions, ["v3", "v3b", "v4"]);
        assert_eq!(cut[1].versions, ["v3", "v3b", "v4"]);
        assert_eq!((cut[0].card.as_str(), cut[0].name.as_str()), (A, "core-001 name"));

        let read = |versions: &[PlayVersion]| {
            play_version_figures(
                &totals,
                &PlayVersionQuery {
                    history: &history,
                    ..query(versions, 1)
                },
            )
        };
        let whole = read(&every);
        let major = read(&cut);
        let span = |table: &PlayVersionTable, id: &str| {
            let row = row(table, id);
            (row.first.clone(), row.last.clone(), row.games, row.patches)
        };
        // The windows are v1–v1b, v2, v3–v3b and v4: A reaches two of them from the major, four in all.
        assert_eq!(span(&major, "core-001~v1"), ("v3".into(), "v4".into(), 3, 2));
        assert_eq!(span(&whole, "core-001~v1"), ("v1".into(), "v4".into(), 5, 4));
        assert_eq!(span(&major, "core-002~v2"), ("v3".into(), "v4".into(), 3, 2));
        assert_eq!(span(&whole, "core-002~v2"), ("v2".into(), "v4".into(), 4, 3));
        // D's version closed: it has no row, nor has B's first.
        assert_eq!(major.versions.len(), 3);
        assert_eq!(whole.versions.len(), 5);

        let field = stats_field([&by_hand(&after, A), &by_hand(&after, B), &by_hand(&after, C)], 1);
        assert_eq!(major.field, field);
        assert_ne!(major.field, whole.field);
        let a = by_hand(&after, A);
        assert_eq!(
            row(&major, "core-001~v1").adjusted_win_rate,
            adjusted_win_rate(&a, &field)
        );
    }

    // R5040000: a play version begun after the newest major spans its line already, so it is the same
    // in both scopes, as it reads against each scope's field.
    #[test]
    fn r5040000_a_version_begun_after_the_newest_major_is_the_same_in_both_scopes() {
        let history = two_majors();
        let (before, after) = line_games();
        let totals = folded(&[&before[..], &after].concat());
        let every = line_versions();
        let cut = major_line_versions(&every, &history);
        assert_eq!(cut[2], every[3]);
        for versions in [&every[..], &cut] {
            let table = play_version_figures(
                &totals,
                &PlayVersionQuery {
                    history: &history,
                    ..query(versions, 1)
                },
            );
            let c = row(&table, "core-003~v3b");
            assert_eq!(
                (
                    c.first.as_str(),
                    c.last.as_str(),
                    c.games,
                    c.patches,
                    c.has_enough_games
                ),
                ("v3b", "v4", 2, 2, true)
            );
        }
    }

    // R5040000: a new major starts every row again from it: no games on it yet, so no tier.
    #[test]
    fn r5040000_a_third_major_restarts_every_row() {
        let mut history = two_majors();
        history.push(shipped("v5", PatchTier::Major));
        let every = line_versions().map(|mut version| {
            if version.live {
                version.versions.push("v5".to_string());
            }
            version
        });
        let (before, after) = line_games();
        let totals = folded(&[&before[..], &after].concat());
        let cut = major_line_versions(&every, &history);
        assert_eq!(ids(&cut), ["core-001~v1", "core-002~v2", "core-003~v3b"]);
        assert!(cut.iter().all(|version| version.versions == ["v5"]));
        let table = play_version_figures(
            &totals,
            &PlayVersionQuery {
                history: &history,
                ..query(&cut, 1)
            },
        );
        for row in &table.versions {
            assert_eq!(
                (
                    row.first.as_str(),
                    row.last.as_str(),
                    row.games,
                    row.patches,
                    row.has_enough_games,
                    row.tier
                ),
                ("v5", "v5", 0, 1, false, None),
                "{}",
                row.id
            );
        }
        // With no major in the history the line is all of it: only the closed versions go.
        let unmajored: Vec<PatchStep> = history
            .iter()
            .map(|step| shipped(&step.version, PatchTier::Patch))
            .collect();
        let live: Vec<PlayVersion> = every.iter().filter(|version| version.live).cloned().collect();
        assert_eq!(major_line_versions(&every, &unmajored), live);
        // A live version none of whose versions the line holds has no row: C, against the history up to v2.
        assert_eq!(
            ids(&major_line_versions(&line_versions(), &two_majors()[..3])),
            ["core-001~v1", "core-002~v2"]
        );
    }

    /// R5036000: `game` on live `patch` whose p1 plays `played` on its first turn, its two turns read at
    /// `evals`.
    fn swung(
        id: &str,
        patch: &str,
        deck: &[&str],
        won: bool,
        played: &[&str],
        evals: [i64; 2],
    ) -> GameRecord {
        let mut record = game(id, GameSource::Live, patch, deck, won);
        let p1 = &mut record.game.seats.p1;
        p1.played = played.iter().map(|card| card.to_string()).collect();
        p1.played_turns = Some(vec![1; played.len()]);
        p1.swing_turns = Some(
            [(1, evals[0]), (3, evals[1])]
                .map(|(turn, eval)| SwingTurn {
                    turn,
                    eval,
                    acting: Vec::new(),
                    facing: Vec::new(),
                })
                .to_vec(),
        );
        record
    }

    /// R5036000: the Cards tab's Decisiveness at `at` (R4552001), as `GET /api/stats/cards` reads it: every
    /// card a counted deck held and every Created card, beside the turns of the window every card shares,
    /// over a history of `at` alone.
    fn cards_tab(totals: &StatsTotals, at: &str) -> SwingReport {
        let history = [PatchStep {
            version: at.to_string(),
            tier: PatchTier::Major,
            plays: Vec::new(),
        }];
        let report = windowed_card_stats(
            totals,
            &WindowQuery {
                history: &history,
                at,
                live_pilot: PilotFilter::Unified,
                mode: ModeFilter::All,
                request: WindowRequest::Gated,
            },
        );
        let empty = SwingTally::default();
        swing_report(
            report
                .cards
                .iter()
                .map(|card| (card.stats.card.as_str(), card.stats.swing.or(&empty))),
            report
                .created
                .iter()
                .map(|row| (row.stats.card.as_str(), &row.stats.swing)),
            &swing_baseline(&windowed_swing_rows(
                totals,
                &history,
                at,
                ModeFilter::All,
                WindowRequest::Gated,
            )),
        )
    }

    // R5036000: over one version a play version's Decisiveness is its card's on the Cards tab over the same
    // records; below the floor it has none, as it has no tier. Fewer rows than `ADJUSTED_FIELD_MIN_CARDS`,
    // so both read the named priors and agree to the bit.
    #[test]
    fn r5036000_a_play_versions_decisiveness_over_one_version_is_the_cards_tabs() {
        let versions = [version(A, &["v1"], true), version(B, &["v1"], true)];
        let totals = folded(&[
            swung("m1", "v1", &[A, B], true, &[A], [0, 30_000]),
            swung("m2", "v1", &[A, B], false, &[B], [0, -20_000]),
            swung("m3", "v1", &[A, B], true, &[A, B], [5_000, 10_000]),
            swung("m4", "v1", &[A, B], false, &[A], [-10_000, 0]),
        ]);
        let swing = cards_tab(&totals, "v1");
        let table = play_version_figures(&totals, &query(&versions, 1));
        for play in &versions {
            let read = row(&table, &play.id).decisiveness;
            assert!(read.is_some(), "{}", play.id);
            assert_eq!(read, swing.cards[play.card.as_str()].decisiveness, "{}", play.id);
        }
        assert_eq!(table.decisiveness, swing.deck_field.typical);

        let table = play_version_figures(&totals, &query(&versions, 5));
        for row in &table.versions {
            assert_eq!(
                (row.has_enough_games, row.decisiveness),
                (false, None),
                "{}",
                row.id
            );
        }
    }

    // R5036000: over two versions a play version reads its tallies summed over both (R1044000), against the
    // baseline of every record the table counts, another card's span among them.
    #[test]
    fn r5036000_over_two_versions_a_play_version_reads_its_summed_tallies_against_one_baseline() {
        let a = version(A, &["v1", "v2"], true);
        let b = version(B, &["v3"], true);
        let on_v1 = [
            swung("m1", "v1", &[A], true, &[A], [0, 30_000]),
            swung("m2", "v1", &[A], false, &[A], [5_000, -10_000]),
        ];
        let on_v2 = [swung("m3", "v2", &[A], true, &[A], [-5_000, 20_000])];
        let on_v3 = [swung("m4", "v3", &[B], false, &[B], [-30_000, 30_000])];
        let spanned = [&on_v1[..], &on_v2].concat();
        let every = [&spanned[..], &on_v3].concat();
        let table = play_version_figures(&folded(&every), &query(&[a.clone(), b], 1));
        let empty = SwingTally::default();
        // A's Decisiveness over `records`, beside the turns of `beside`.
        let read = |records: &[GameRecord], beside: &[GameRecord]| {
            let held = by_hand(records, A);
            swing_report(
                [(A, held.swing.or(&empty))],
                std::iter::empty(),
                &swing_baseline(&swing_rows_of(&folded(beside), &all())),
            )
            .cards[A]
                .decisiveness
        };
        let decisiveness = row(&table, &a.id).decisiveness;
        assert!(decisiveness.is_some());
        assert_eq!(decisiveness, read(&spanned, &every));
        assert_ne!(decisiveness, read(&on_v1, &every));
        assert_ne!(decisiveness, read(&spanned, &spanned));
    }

    // R5036000: records written before R4552000 carry no turns, so a span of them has its games and tier and
    // no Decisiveness, and the table's typical value is the named prior's.
    #[test]
    fn r5036000_a_span_with_no_swing_rows_has_no_decisiveness() {
        let versions = [version(A, &["v1"], true)];
        let totals = folded(&[
            game("m1", GameSource::Live, "v1", &[A], true),
            game("m2", GameSource::Live, "v1", &[A], false),
        ]);
        let table = play_version_figures(&totals, &query(&versions, 1));
        let row = &table.versions[0];
        assert!(row.has_enough_games);
        assert!(row.tier.is_some());
        assert_eq!(row.decisiveness, None);
        assert_eq!(table.decisiveness, DECISIVENESS_PRIOR_CENTER);
    }

    // R5036000, R4552002: the Dormant Starship is in no deck, so its play version reads its Created copies'
    // turns, as the Cards tab reads them.
    #[test]
    fn r5036000_a_card_only_a_rule_makes_reads_its_created_copies_turns() {
        let starship = version(DORMANT_STARSHIP_DEF_ID, &["v1"], true);
        let mut made = swung("m1", "v1", &[A], true, &[], [0, 30_000]);
        // p1 made one and played it on its first turn.
        let used = vec![CreatedUse {
            card: DORMANT_STARSHIP_DEF_ID.to_string(),
            turn: 1,
            radiant: false,
            variant: None,
            source: None,
        }];
        made.game.seats.p1.created_used = Some(used.clone());
        made.game.seats.p1.created_played = Some(used);
        let totals = folded(&[made]);
        let table = play_version_figures(&totals, &query(std::slice::from_ref(&starship), 1));
        let created = card_stats_of(&totals, &all())
            .created
            .into_iter()
            .find(|row| row.card == DORMANT_STARSHIP_DEF_ID)
            .expect("a seat made one");
        let swing = swing_report(
            std::iter::empty(),
            [(DORMANT_STARSHIP_DEF_ID, &created.swing)],
            &swing_baseline(&swing_rows_of(&totals, &all())),
        );
        let read = table.versions[0].decisiveness;
        assert!(read.is_some());
        assert_eq!(read, swing.created[DORMANT_STARSHIP_DEF_ID].decisiveness);
    }

    #[test]
    fn r4675000_a_play_versions_id_is_its_card_and_first_version() {
        let play = version("meditative-097", &["v0.3.0", "v0.3.0b"], false);
        assert_eq!(play.id, "meditative-097~v0.3.0");
        assert_eq!((play.first(), play.last()), ("v0.3.0", "v0.3.0b"));
        assert_eq!(
            play_version_id("core-001", "v1"),
            format!("core-001{PLAY_VERSION_MARK}v1")
        );
    }

    // R5035000: each row sends its play version's iteration as its label, and a cut row keeps it: the
    // number is its card's own, whatever the table reads.
    #[test]
    fn r5035000_a_row_sends_its_play_versions_iteration_label() {
        let numbered =
            |card: &str, versions: &[&str], live: bool, iteration: u32, letters: &str| PlayVersion {
                iteration,
                iteration_letters: letters.to_string(),
                ..version(card, versions, live)
            };
        let versions = [
            numbered(A, &["v1"], false, FIRST_ITERATION, ""),
            numbered(A, &["v1b", "v2"], false, FIRST_ITERATION, "b"),
            numbered(A, &["v3", "v3b", "v4"], true, FIRST_ITERATION + 1, ""),
        ];
        assert_eq!(versions[1].iteration_label(), format!("{ITERATION_MARK}1b"));
        let labels = |table: &PlayVersionTable| -> Vec<String> {
            table.versions.iter().map(|row| row.iteration.clone()).collect()
        };
        let whole = play_version_figures(&StatsTotals::default(), &query(&versions, 1));
        assert_eq!(labels(&whole), ["v1", "v1b", "v2"]);
        let cut = major_line_versions(&versions, &two_majors());
        let major = play_version_figures(&StatsTotals::default(), &query(&cut, 1));
        assert_eq!(labels(&major), ["v2"]);
    }
}
