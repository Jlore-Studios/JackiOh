//! R1367000 (amends R654): a card's statistics window, and the one function every reader of public
//! card figures calls (`windowed_card_stats`): `GET /api/stats/cards` (and so the stats page),
//! `cargo jackioh stats report --window` and `cargo jackioh winrates`.
//!
//! A card's window at a version runs from whichever came later: the start of its X (the newest major
//! or patch), or the last micro whose plays hold the card's family. A major or a patch opens a new
//! window for every card; a micro opens one only for the families it plays (#1360). R654's gate,
//! `PUBLIC_STATS_MIN_LIVE_GAMES`, counts every live game played on the window's versions, not only
//! the games that hold the card. A window short of it shows the card's previous window (the window
//! the card had at the version before its own opened) when that one is not short too; only when both
//! are do AI development games pad the current window (provisional).
//!
//! The engine is pure (CLAUDE.md rule 4), so the patch history is an argument: each shipped version
//! in `patches.json`'s order with its tier and play set (`PatchStep`). `jackioh_cards::patch_history`
//! builds it from the compiled-in history. A version string is compared for equality and never
//! parsed or ordered (R105): order is the history's.

use std::borrow::Cow;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{
    CardStats, CardStatsFilter, GameMode, GameSource, PilotFilter, SourceFilter, StatsTotals, add_stats,
    card_stats_of,
};
use crate::config::PUBLIC_STATS_MIN_LIVE_GAMES;

/// R1365000: how far a shipped version moved the cards. Serialized as `patches.json` records it.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum PatchTier {
    Major,
    Patch,
    Micro,
}

/// R1367000: one shipped version as a card's window reads it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PatchStep {
    pub version: String,
    pub tier: PatchTier,
    /// The card families (`card_family`) that play differently in it. Read only for a micro: a
    /// major or a patch opens a new window for every card.
    pub plays: Vec<String>,
}

impl PatchStep {
    /// Whether this version opens a new window for `family`, or for every card when `None`.
    fn opens(&self, family: Option<&str>) -> bool {
        match self.tier {
            PatchTier::Major | PatchTier::Patch => true,
            PatchTier::Micro => family.is_some_and(|family| self.plays.iter().any(|plays| plays == family)),
        }
    }
}

/// R1365000: a card's family, its first two `-` fields: a token counts with its card
/// (`meditative-097-6` is `meditative-097`).
pub fn card_family(id: &str) -> &str {
    match id.match_indices('-').nth(1) {
        Some((at, _)) => &id[..at],
        None => id,
    }
}

/// R1367000: which records a window's figures count.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum WindowSource {
    /// The window's own live games: it cleared R654's gate, or the query asked for live games.
    Live,
    /// The live games of the card's previous window: its own is short and the previous one is not.
    Previous,
    /// The window's own live and development games: it and the previous window are both short.
    Provisional,
    /// The window's own development games, which the query asked for (below the gate only).
    Dev,
}

/// R1367000: what a query asks of the gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindowRequest {
    /// R654's gate as amended: live, else the previous window, else provisional.
    Gated,
    /// Below the gate, the window's live games alone; at or above it, as `Gated`.
    Live,
    /// Below the gate, the window's development games alone; at or above it, as `Gated`.
    Dev,
}

/// R1367000: a card's window at a version, and the records its figures count.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CardWindow {
    /// The window's versions, oldest first, ending at the version asked about.
    pub versions: Vec<String>,
    /// Every live game played on them, which R654's gate counts.
    pub live_games: i64,
    pub source: WindowSource,
    /// The versions whose games the figures count: `versions`, or the previous window's.
    pub counted: Vec<String>,
    /// Every live game played on `counted`.
    pub counted_live_games: i64,
}

impl CardWindow {
    /// Whether the window's own live games reach R654's gate.
    pub fn cleared(&self) -> bool {
        self.live_games >= PUBLIC_STATS_MIN_LIVE_GAMES
    }

    /// The record sources the figures count.
    pub fn sources(&self) -> &'static [GameSource] {
        match self.source {
            WindowSource::Live | WindowSource::Previous => &[GameSource::Live],
            WindowSource::Provisional => &[GameSource::Live, GameSource::Dev],
            WindowSource::Dev => &[GameSource::Dev],
        }
    }
}

/// The history to read and `at`'s place in it. A version the history does not hold (a made-up
/// `?patch=`, or a run filed under an unshipped one) is a history of its own, opening its window.
fn resolve<'a>(history: &'a [PatchStep], at: &str) -> (Cow<'a, [PatchStep]>, usize) {
    match history.iter().position(|step| step.version == at) {
        Some(index) => (Cow::Borrowed(history), index),
        None => (
            Cow::Owned(vec![PatchStep {
                version: at.to_string(),
                tier: PatchTier::Major,
                plays: Vec::new(),
            }]),
            0,
        ),
    }
}

/// The index of the version that opened `family`'s window at `at`. The first version opens every window.
fn window_start(history: &[PatchStep], at: usize, family: Option<&str>) -> usize {
    (0..=at)
        .rev()
        .find(|&index| history[index].opens(family))
        .unwrap_or(0)
}

fn versions_of(history: &[PatchStep], from: usize, to: usize) -> Vec<String> {
    history[from..=to]
        .iter()
        .map(|step| step.version.clone())
        .collect()
}

fn window_in(
    history: &[PatchStep],
    at: usize,
    family: Option<&str>,
    request: WindowRequest,
    live_games: &impl Fn(&str) -> i64,
) -> CardWindow {
    let count = |versions: &[String]| versions.iter().map(|version| live_games(version)).sum::<i64>();
    let start = window_start(history, at, family);
    let versions = versions_of(history, start, at);
    let own = count(&versions);
    let (source, counted) = if own >= PUBLIC_STATS_MIN_LIVE_GAMES {
        (WindowSource::Live, versions.clone())
    } else {
        match request {
            WindowRequest::Live => (WindowSource::Live, versions.clone()),
            WindowRequest::Dev => (WindowSource::Dev, versions.clone()),
            WindowRequest::Gated => {
                let previous = start
                    .checked_sub(1)
                    .map(|end| versions_of(history, window_start(history, end, family), end));
                match previous {
                    Some(previous) if count(&previous) >= PUBLIC_STATS_MIN_LIVE_GAMES => {
                        (WindowSource::Previous, previous)
                    }
                    _ => (WindowSource::Provisional, versions.clone()),
                }
            }
        }
    };
    let counted_live_games = count(&counted);
    CardWindow {
        versions,
        live_games: own,
        source,
        counted,
        counted_live_games,
    }
}

/// R1367000: `family`'s window at `at` (`None`: the window every card shares, its X's), and the
/// records its figures count under `request`. `live_games` answers a version's live games.
pub fn card_window(
    history: &[PatchStep],
    at: &str,
    family: Option<&str>,
    request: WindowRequest,
    live_games: impl Fn(&str) -> i64,
) -> CardWindow {
    let (history, at) = resolve(history, at);
    window_in(&history, at, family, request, &live_games)
}

/// R1367000: what `windowed_card_stats` reads.
#[derive(Clone, Debug)]
pub struct WindowQuery<'a> {
    /// Every shipped version, in `patches.json`'s order (`jackioh_cards::patch_history`).
    pub history: &'a [PatchStep],
    /// The version the figures are for: the newest shipped one, or the one a query names.
    pub at: &'a str,
    /// The seats of a live record counted (R807000: `unified` is every seat, human and bot). A
    /// development record always counts both seats (R378, R1451).
    pub live_pilot: PilotFilter,
    /// R377: one match type the figures count, or every mode. R654's gate counts every mode's live
    /// games whatever it names.
    pub mode: Option<GameMode>,
    pub request: WindowRequest,
}

/// R1367000: one card's figures over its window.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WindowedCard {
    pub window: CardWindow,
    pub stats: CardStats,
    /// The counted records with a counted seat, and those seats: a play rate's denominator.
    pub games: i64,
    pub decks: i64,
}

/// R1367000: every card's figures, each over its own window.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WindowedReport {
    pub at: String,
    /// The window every card shares (its X's, opened by the newest major or patch): the gate a page
    /// shows above the cards.
    pub patch: CardWindow,
    /// The games and decks `patch`'s counted records hold.
    pub games: i64,
    pub decks: i64,
    /// Every card a counted deck held, in catalog-id order.
    pub cards: Vec<WindowedCard>,
}

impl WindowedReport {
    /// A card's row, if a counted deck held it.
    pub fn card(&self, id: &str) -> Option<&WindowedCard> {
        self.cards.iter().find(|card| card.stats.card == id)
    }
}

/// One version's figures from one source, as `card_stats_of` counts them.
struct Part {
    games: i64,
    decks: i64,
    cards: IndexMap<String, CardStats>,
}

/// R1367000 (amends R654): every card's figures, each over its own window at `query.at`: R377's
/// breakdowns summed over the versions and sources its window counts (`card_window`), with the
/// games and decks of those records. Every caller of public card figures reads them here:
/// `GET /api/stats/cards`, `cargo jackioh stats report --window` and `cargo jackioh winrates`.
pub fn windowed_card_stats(totals: &StatsTotals, query: &WindowQuery) -> WindowedReport {
    let (history, at) = resolve(query.history, query.at);
    // Each version's totals apart, so a version's figures read only its own keys.
    let mut by_version: IndexMap<&str, StatsTotals> = IndexMap::new();
    for (key, records) in &totals.records {
        by_version
            .entry(key.patch.as_str())
            .or_default()
            .records
            .insert(key.clone(), *records);
    }
    for (key, cards) in &totals.cards {
        by_version
            .entry(key.patch.as_str())
            .or_default()
            .cards
            .insert(key.clone(), cards.clone());
    }
    let live: IndexMap<&str, i64> = by_version
        .iter()
        .map(|(version, held)| (*version, held.live_games(version)))
        .collect();
    let live_games = |version: &str| live.get(version).copied().unwrap_or(0);
    let empty = StatsTotals::default();
    let part = |version: &str, source: GameSource| {
        let (filter_source, pilot) = match source {
            GameSource::Live => (SourceFilter::Live, query.live_pilot),
            GameSource::Dev => (SourceFilter::Dev, PilotFilter::Unified),
        };
        let report = card_stats_of(
            by_version.get(version).unwrap_or(&empty),
            &CardStatsFilter {
                source: filter_source,
                mode: query.mode,
                patch: Some(version.to_string()),
                pilot,
            },
        );
        Part {
            games: i64::from(report.games),
            decks: i64::from(report.decks),
            cards: report
                .cards
                .into_iter()
                .map(|stats| (stats.card.clone(), stats))
                .collect(),
        }
    };
    let mut parts: IndexMap<(String, GameSource), Part> = IndexMap::new();
    for step in &history[..=at] {
        for source in [GameSource::Live, GameSource::Dev] {
            parts.insert((step.version.clone(), source), part(&step.version, source));
        }
    }
    let counted_parts = |window: &CardWindow| {
        let mut chosen: Vec<&Part> = Vec::new();
        for version in &window.counted {
            for source in window.sources() {
                if let Some(part) = parts.get(&(version.clone(), *source)) {
                    chosen.push(part);
                }
            }
        }
        chosen
    };

    let patch = window_in(&history, at, None, query.request, &live_games);
    let (games, decks) = counted_parts(&patch).iter().fold((0, 0), |(games, decks), part| {
        (games + part.games, decks + part.decks)
    });

    let mut ids: Vec<&str> = parts
        .values()
        .flat_map(|part| part.cards.keys().map(String::as_str))
        .collect();
    ids.sort_unstable();
    ids.dedup();
    let mut windows: IndexMap<&str, CardWindow> = IndexMap::new();
    let mut cards: Vec<WindowedCard> = Vec::new();
    for id in ids {
        let family = card_family(id);
        let window = windows
            .entry(family)
            .or_insert_with(|| window_in(&history, at, Some(family), query.request, &live_games))
            .clone();
        let mut stats: Option<CardStats> = None;
        let (mut games, mut decks) = (0, 0);
        for part in counted_parts(&window) {
            games += part.games;
            decks += part.decks;
            if let Some(card) = part.cards.get(id) {
                match stats.as_mut() {
                    None => stats = Some(card.clone()),
                    Some(held) => add_stats(held, card),
                }
            }
        }
        if let Some(stats) = stats {
            cards.push(WindowedCard {
                window,
                stats,
                games,
                decks,
            });
        }
    }
    WindowedReport {
        at: query.at.to_string(),
        patch,
        games,
        decks,
        cards,
    }
}

// R1367000: windows over a history written by hand, and figures over totals folded from records
// written by hand, so each count can be read off the test.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::catalog_types::{PerPlayer, PlayerId};
    use crate::wire::events::{GameOverReason, Winner};
    use crate::wire::stats::{GameRecord, GameSummary, Pilot, RecordsKey, SeatSummary};

    const A: &str = "core-001";
    const B: &str = "core-002";

    fn step(version: &str, tier: PatchTier, plays: &[&str]) -> PatchStep {
        PatchStep {
            version: version.to_string(),
            tier,
            plays: plays.iter().map(|family| family.to_string()).collect(),
        }
    }

    fn versions(window: &CardWindow) -> Vec<&str> {
        window.versions.iter().map(String::as_str).collect()
    }

    fn counted(window: &CardWindow) -> Vec<&str> {
        window.counted.iter().map(String::as_str).collect()
    }

    fn none(_: &str) -> i64 {
        0
    }

    fn seat(deck: &[&str]) -> SeatSummary {
        SeatSummary {
            deck: deck.iter().map(|id| id.to_string()).collect(),
            ..SeatSummary::default()
        }
    }

    /// A game p1 won holding `deck`, its opponent holding a card neither test reads.
    fn game(id: &str, source: GameSource, patch: &str, deck: &[&str]) -> GameRecord {
        GameRecord {
            id: id.to_string(),
            source,
            mode: GameMode::Bo1,
            patch: patch.to_string(),
            pilots: PerPlayer::new(Pilot::Human, Pilot::Human),
            game: GameSummary {
                first: PlayerId::P1,
                winner: Winner::P1,
                reason: GameOverReason::HeroDeath,
                turns: 5,
                seats: PerPlayer::new(seat(deck), seat(&["core-099"])),
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

    fn query<'a>(history: &'a [PatchStep], at: &'a str) -> WindowQuery<'a> {
        WindowQuery {
            history,
            at,
            live_pilot: PilotFilter::Unified,
            mode: None,
            request: WindowRequest::Gated,
        }
    }

    #[test]
    fn r1367000_a_micro_touching_a_card_reopens_its_window_and_leaves_the_others_alone() {
        let history = [
            step("v1", PatchTier::Major, &[]),
            step("v2", PatchTier::Micro, &[A]),
            step("v3", PatchTier::Micro, &[]),
        ];
        let window = |family| card_window(&history, "v3", family, WindowRequest::Gated, none);
        assert_eq!(versions(&window(Some(A))), ["v2", "v3"]);
        assert_eq!(versions(&window(Some(B))), ["v1", "v2", "v3"]);
        assert_eq!(versions(&window(None)), ["v1", "v2", "v3"]);
        // At the version before the micro, A's window is still its X's.
        assert_eq!(
            versions(&card_window(&history, "v1", Some(A), WindowRequest::Gated, none)),
            ["v1"]
        );
        // A token is its card's family.
        assert_eq!(card_family("core-001-2"), A);
        assert_eq!(card_family(A), A);
    }

    #[test]
    fn r1367000_a_patch_or_a_major_reopens_every_cards_window() {
        for tier in [PatchTier::Patch, PatchTier::Major] {
            let history = [
                step("v1", PatchTier::Major, &[]),
                step("v2", PatchTier::Micro, &[A]),
                step("v3", tier, &[]),
                step("v4", PatchTier::Micro, &[B]),
            ];
            let window = |family| card_window(&history, "v4", family, WindowRequest::Gated, none);
            assert_eq!(versions(&window(Some(A))), ["v3", "v4"]);
            assert_eq!(versions(&window(Some(B))), ["v4"]);
            assert_eq!(versions(&window(None)), ["v3", "v4"]);
        }
    }

    #[test]
    fn r1367000_a_window_at_999_live_games_falls_back_and_at_1000_it_does_not() {
        let history = [
            step("v1", PatchTier::Major, &[]),
            step("v2", PatchTier::Micro, &[A]),
        ];
        let short = PUBLIC_STATS_MIN_LIVE_GAMES - 1;
        let live = |v2: i64| {
            move |version: &str| {
                if version == "v1" {
                    PUBLIC_STATS_MIN_LIVE_GAMES
                } else {
                    v2
                }
            }
        };

        let fallen = card_window(&history, "v2", Some(A), WindowRequest::Gated, live(short));
        assert_eq!(versions(&fallen), ["v2"]);
        assert_eq!(fallen.live_games, short);
        assert!(!fallen.cleared());
        assert_eq!(fallen.source, WindowSource::Previous);
        assert_eq!(counted(&fallen), ["v1"]);
        assert_eq!(fallen.counted_live_games, PUBLIC_STATS_MIN_LIVE_GAMES);
        // B's window holds both versions' games, and clears.
        let b = card_window(&history, "v2", Some(B), WindowRequest::Gated, live(short));
        assert_eq!((b.source, counted(&b)), (WindowSource::Live, vec!["v1", "v2"]));

        let cleared = card_window(
            &history,
            "v2",
            Some(A),
            WindowRequest::Gated,
            live(PUBLIC_STATS_MIN_LIVE_GAMES),
        );
        assert!(cleared.cleared());
        assert_eq!(
            (cleared.source, counted(&cleared)),
            (WindowSource::Live, vec!["v2"])
        );
    }

    #[test]
    fn r1367000_ai_games_pad_a_window_only_when_its_previous_window_is_short_too() {
        let history = [
            step("v1", PatchTier::Major, &[]),
            step("v2", PatchTier::Micro, &[A]),
        ];
        let short = |_: &str| PUBLIC_STATS_MIN_LIVE_GAMES - 1;
        let padded = card_window(&history, "v2", Some(A), WindowRequest::Gated, short);
        assert_eq!(
            (padded.source, counted(&padded)),
            (WindowSource::Provisional, vec!["v2"])
        );
        assert_eq!(padded.sources(), [GameSource::Live, GameSource::Dev]);
        // The first window has no previous one.
        let first = card_window(&history, "v1", Some(A), WindowRequest::Gated, short);
        assert_eq!(first.source, WindowSource::Provisional);
        // A query that names a source reads that source's games of the window itself below the gate.
        let live = card_window(&history, "v2", Some(A), WindowRequest::Live, short);
        assert_eq!((live.source, counted(&live)), (WindowSource::Live, vec!["v2"]));
        let dev = card_window(&history, "v2", Some(A), WindowRequest::Dev, short);
        assert_eq!((dev.source, counted(&dev)), (WindowSource::Dev, vec!["v2"]));
        // A version the history does not hold is a window of its own.
        let unknown = card_window(&history, "v9", Some(A), WindowRequest::Gated, short);
        assert_eq!(
            (versions(&unknown), unknown.source),
            (vec!["v9"], WindowSource::Provisional)
        );
    }

    #[test]
    fn r1367000_windowed_card_stats_counts_each_card_over_its_own_window() {
        let history = [
            step("v1", PatchTier::Major, &[]),
            step("v2", PatchTier::Micro, &[A]),
        ];
        let mut totals = StatsTotals::default();
        totals.fold(&game("m1", GameSource::Live, "v1", &[A, B]), false);
        totals.fold(&game("m2", GameSource::Live, "v2", &[A, B]), false);
        totals.fold(&game("dev:v2:s:1", GameSource::Dev, "v2", &[A]), false);
        add_live(&mut totals, "v1", PUBLIC_STATS_MIN_LIVE_GAMES);
        add_live(&mut totals, "v2", PUBLIC_STATS_MIN_LIVE_GAMES);

        let report = windowed_card_stats(&totals, &query(&history, "v2"));
        assert_eq!(report.at, "v2");
        assert_eq!(versions(&report.patch), ["v1", "v2"]);
        assert_eq!(report.patch.source, WindowSource::Live);
        // The patch window's live records: the two folded and the games added on each version.
        assert_eq!(report.games, 2 * PUBLIC_STATS_MIN_LIVE_GAMES + 2);
        let a = report.card(A).expect("A is held");
        assert_eq!(versions(&a.window), ["v2"]);
        assert_eq!((a.stats.in_deck.games, a.stats.in_deck.wins), (1, 1));
        assert_eq!(a.games, PUBLIC_STATS_MIN_LIVE_GAMES + 1);
        assert_eq!(a.decks, 2 * a.games);
        let b = report.card(B).expect("B is held");
        assert_eq!(b.stats.in_deck.games, 2);
        assert_eq!(
            report
                .cards
                .iter()
                .map(|card| card.stats.card.as_str())
                .collect::<Vec<_>>(),
            [A, B, "core-099"]
        );

        // v2 short: A falls back to v1's live games; v1 short too, and A's window is padded with v2's
        // development game.
        let mut short = StatsTotals::default();
        short.fold(&game("m1", GameSource::Live, "v1", &[A, B]), false);
        short.fold(&game("m2", GameSource::Live, "v2", &[A, B]), false);
        short.fold(&game("dev:v2:s:1", GameSource::Dev, "v2", &[A]), false);
        add_live(&mut short, "v1", PUBLIC_STATS_MIN_LIVE_GAMES - 1);
        let report = windowed_card_stats(&short, &query(&history, "v2"));
        let a = report.card(A).expect("A is held");
        assert_eq!(
            (a.window.source, counted(&a.window)),
            (WindowSource::Previous, vec!["v1"])
        );
        assert_eq!(a.stats.in_deck.games, 1);
        add_live(&mut short, "v1", -1);
        let report = windowed_card_stats(&short, &query(&history, "v2"));
        let a = report.card(A).expect("A is held");
        assert_eq!(a.window.source, WindowSource::Provisional);
        assert_eq!(a.stats.in_deck.games, 2);
        // The window every card shares holds v1's 999 and v2's one, and clears.
        assert_eq!(report.patch.source, WindowSource::Live);
    }
}
