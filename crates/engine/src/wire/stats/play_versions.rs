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

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::window::{Part, part_of, split_by_version};
use super::{
    CardStats, CardTier, CreatedStats, DECIMALS, GameMode, GameSource, ModeFilter, NO_FIGURE, PERCENT,
    PilotFilter, StatsField, StatsTotals, Tally, add_created, add_stats, adjusted_interval,
    adjusted_win_rate, held_adjusted, seen, stats_field, tier_of, to_fixed,
};
use crate::catalog::RULE_MADE_DEF_IDS;
use crate::config::{PUBLIC_STATS_MIN_LIVE_GAMES, TIER_BAND_HIGH, TIER_BAND_LOW};

/// R4675000: what joins a card's id to the first shipped version of a play version in its id.
pub const PLAY_VERSION_MARK: char = '~';

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
}

/// R4675001: what `play_version_figures` reads.
#[derive(Clone, Debug)]
pub struct PlayVersionQuery<'a> {
    /// The play versions to read, in the order the table lists them.
    pub versions: &'a [PlayVersion],
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
    /// The games its figures count: the decks that held it, under All Random the decks that saw it
    /// (R2777002), and for a card only a rule makes the seats that made one (R3070000).
    pub games: i64,
    /// R2777000, R3022000: its adjusted win rate, read against the table's field, and its 95% range (R2895000).
    pub adjusted_win_rate: Option<f64>,
    pub adjusted_low: Option<f64>,
    pub adjusted_high: Option<f64>,
    /// R3022000: its tier, only once it has the floor's games.
    pub tier: Option<CardTier>,
    pub has_enough_games: bool,
}

/// R4675001: every play version's figures, and the field and band they are read against.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayVersionTable {
    /// R3022000: read from every play version's figures, so each is read beside every other.
    pub field: StatsField,
    pub band_low: f64,
    pub band_high: f64,
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

/// A play version's breakdowns over its span, and its Created figures there (R2089000).
struct Summed {
    stats: Option<CardStats>,
    created: Option<CreatedStats>,
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
    let mut stats: Option<CardStats> = None;
    let mut created: Option<CreatedStats> = None;
    for parts in spanned {
        for source in span_sources(live_games) {
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
    Summed { stats, created }
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
    let versions = query
        .versions
        .iter()
        .zip(&sums)
        .map(|(version, sum)| figures_of(version, sum, &field, query))
        .collect();
    PlayVersionTable {
        field,
        band_low: TIER_BAND_LOW,
        band_high: TIER_BAND_HIGH,
        min_sample: query.min_sample,
        versions,
    }
}

/// One play version's row, as `get_cards` reads a card's.
fn figures_of(
    version: &PlayVersion,
    sum: &Summed,
    field: &StatsField,
    query: &PlayVersionQuery,
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
        games,
        adjusted_win_rate: adjusted,
        adjusted_low: range.map(|(low, _)| low),
        adjusted_high: range.map(|(_, high)| high),
        tier,
        has_enough_games,
    }
}

/// A fraction as "52.1%", or `NO_FIGURE`.
fn percent(rate: Option<f64>) -> String {
    rate.map_or_else(
        || NO_FIGURE.to_string(),
        |rate| format!("{}%", to_fixed(rate * PERCENT, DECIMALS)),
    )
}

/// R4675001: the table as text: the field and band, one line per play version, and how many have games
/// and the floor's games, live and closed.
pub fn format_play_versions(table: &PlayVersionTable) -> String {
    let field = &table.field;
    let mut lines = vec![format!(
        "Play versions read against a field of {} play versions (centre {} points, spread {} points); band {}–{}; a tier from {} games.",
        field.cards,
        to_fixed(field.center * PERCENT, DECIMALS),
        to_fixed(field.spread * PERCENT, DECIMALS),
        percent(Some(table.band_low)),
        percent(Some(table.band_high)),
        table.min_sample
    )];
    for row in &table.versions {
        lines.push(format!(
            "{} {} {}..{}{} games {} adjusted {} ({}–{}) tier {}",
            row.id,
            row.name,
            row.first,
            row.last,
            if row.live { " live" } else { "" },
            row.games,
            percent(row.adjusted_win_rate),
            percent(row.adjusted_low),
            percent(row.adjusted_high),
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
    use crate::config::DORMANT_STARSHIP_DEF_ID;
    use crate::wire::catalog_types::{PerPlayer, PlayerId};
    use crate::wire::events::{GameOverReason, Winner};
    use crate::wire::stats::{
        CardStatsFilter, GameRecord, GameSummary, Pilot, RecordsKey, SeatSummary, SourceFilter, card_stats_of,
    };

    const A: &str = "core-001";
    const B: &str = "core-002";
    /// The card the opponent holds, which no play version reads.
    const OTHER: &str = "core-099";

    fn version(card: &str, versions: &[&str], live: bool) -> PlayVersion {
        PlayVersion {
            id: play_version_id(card, versions[0]),
            card: card.to_string(),
            name: format!("{card} name"),
            versions: versions.iter().map(|version| version.to_string()).collect(),
            live,
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

    /// `card`'s breakdowns over `records` alone, every source, mode and patch: a sum folded by hand.
    fn by_hand(records: &[GameRecord], card: &str) -> CardStats {
        card_stats_of(
            &folded(records),
            &CardStatsFilter {
                source: SourceFilter::All,
                mode: None,
                patch: None,
                pilot: PilotFilter::Unified,
            },
        )
        .cards
        .into_iter()
        .find(|stats| stats.card == card)
        .expect("the card is held")
    }

    fn query(versions: &[PlayVersion], min_sample: i64) -> PlayVersionQuery<'_> {
        PlayVersionQuery {
            versions,
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
                "core-001~v0 core-001 name v0..v0 games 0 adjusted {NO_FIGURE} ({NO_FIGURE}–{NO_FIGURE}) tier {NO_FIGURE}"
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
}
