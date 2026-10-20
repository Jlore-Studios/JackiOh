//! ME-STATS (docs/meditative-set.md M5, SPEC §8.8 row 50): `cargo jackioh winrates` writes
//! `crates/cards/data/win_rates.json`, the table M #50 CN Tech reads, from a `stats report --json`
//! export or from JSONL game record files. The table is `{ patch, source, cards }` with R377's
//! in-deck wins and games per card id, as `/api/stats/cards` ranks by; the build compiles it in and
//! never reads it at run time (CLAUDE.md rule 4). Record files are counted as that endpoint counts
//! them: every counted seat of a live game, human and bot alike (R807000's merged display), and both
//! seats of a development game (R378).
//!
//! Record files are read through the engine's `windowed_card_stats`, the function the server's
//! `/api/stats/cards` calls (R1367000): each card over its own window at `--patch`, under R654's gate
//! as amended. A report export is taken as the server counted it. The table is live when no counted
//! record is a development game, provisional otherwise.
//!
//! R1368000: `cargo jackioh winrates --refresh <input>…` refreshes the table once per patch (once
//! per X, #1360). Its input is record files or a `stats report --window <v> --json` export.
//! - It is due when the newest patch's window has `PUBLIC_STATS_MIN_LIVE_GAMES` live games and the
//!   table is filed under a version outside that window.
//! - When due, it writes the table and a pending fragment that claims it, CN Tech and NA Tech
//!   (`TABLE_READERS`) through `plays`, so `patches ship` ships it as a micro.
//! - Otherwise it says why and writes nothing. It exits 0 either way, for a workflow to run.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use jackioh_engine::config::PUBLIC_STATS_MIN_LIVE_GAMES;
use jackioh_engine::{
    GameSource, PilotFilter, SourceFilter, StatsTotals, WinRateRow, WinRateSource, WinRateTable, WindowQuery,
    WindowRequest, WindowedReport, windowed_card_stats,
};

use crate::patches::patches_io::{
    PendingFragment, PlayerNote, fragment_name, pending_dir, read_fragments, write_json,
};

/// `cargo jackioh winrates`.
#[derive(clap::Args, Debug, Clone, PartialEq)]
pub struct Args {
    /// The stats to build the table from: one JSON `stats report --json` export, or JSONL game
    /// record files — never both.
    pub inputs: Vec<PathBuf>,
    /// The patch the table is for (default the catalog's newest shipped patch).
    #[arg(long)]
    pub patch: Option<String>,
    /// Print the table instead of writing `crates/cards/data/win_rates.json`.
    #[arg(long)]
    pub dry_run: bool,
    /// R1368000: write the table only when a refresh is due, with the pending fragment that ships it:
    /// the current patch's window has `PUBLIC_STATS_MIN_LIVE_GAMES` live games and the table is from
    /// an earlier patch. Reads JSONL record files or one `stats report --window <v> --json` export.
    #[arg(long)]
    pub refresh: bool,
}

/// R1368000: the cards whose play the win-rate table changes, which a refresh's fragment claims
/// (`plays`): M #50 CN Tech and M+ #2 NA Tech.
pub const TABLE_READERS: &[&str] = &["meditative-050", "meditativeplus-002"];

/// R1368000: whether a refresh is due, and why.
#[derive(Clone, Debug, PartialEq)]
pub enum Refresh {
    /// The current patch's window has its live games and the table is from an earlier patch.
    Due(WinRateTable),
    /// The current patch's window is short of `PUBLIC_STATS_MIN_LIVE_GAMES` live games.
    Short { live_games: i64 },
    /// The table is already this patch's: it was refreshed once in it (it is a version of the window).
    Current { table: String },
}

/// R1368000: whether the table `current` should be refreshed from `report`, the windowed report at
/// the newest shipped version. Due the first time the window every card shares (the patch's, its
/// X's) clears R654's gate while `current` was filed under a version outside that window; a table
/// filed under one of the window's versions was refreshed in this patch already, so it is never due
/// twice in one. `dev_on` says whether a version holds a development record, as `table_of` reads it.
pub fn refresh_due(
    report: &WindowedReport,
    current: &WinRateTable,
    dev_on: impl Fn(&str) -> bool,
) -> Refresh {
    if report.patch.versions.contains(&current.patch) {
        return Refresh::Current {
            table: current.patch.clone(),
        };
    }
    if !report.patch.cleared() {
        return Refresh::Short {
            live_games: report.patch.live_games,
        };
    }
    Refresh::Due(table_of(report, dev_on))
}

/// R1368000: the fragment a refresh ships with, so the table goes out as a micro: it claims the table
/// (`winRates`) and the cards that read it (`plays`), and no catalog card.
pub fn refresh_fragment(table: &WinRateTable, live_games: i64) -> PendingFragment {
    PendingFragment {
        title: format!("Patch: CN Tech and NA Tech read {}'s live win rates", table.patch),
        sources: "cargo jackioh winrates --refresh (#1368)".to_string(),
        notes: format!(
            "R1368000: the win-rate table CN Tech and NA Tech read, refreshed once per patch: each card's \
             in-deck wins and games over its window at {}, whose patch window has {live_games} live games \
             (PUBLIC_STATS_MIN_LIVE_GAMES or more). Every shipped version keeps the table it shipped with.",
            table.patch
        ),
        player: PlayerNote {
            title: "CN Tech and NA Tech read new win rates".to_string(),
            changes: vec![
                "CN Tech and NA Tech now read the card win rates of this patch's live games.".to_string(),
            ],
            ..PlayerNote::default()
        },
        cards: Vec::new(),
        plays: TABLE_READERS.iter().map(|id| id.to_string()).collect(),
        win_rates: true,
        ..PendingFragment::default()
    }
}

/// The table file's text, as `run` writes it.
fn table_text(table: &WinRateTable) -> String {
    let mut out = serde_json::to_string_pretty(table).unwrap_or_default();
    out.push('\n');
    out
}

/// R1368000: `winrates --refresh` over `report` (and `dev_on`), against the table at `table_path`,
/// writing the table and `pending_dir`'s fragment when due. A pending fragment that claims the
/// table already is refused: two claims fail `patches check` (R1329000).
pub fn refresh(
    report: &WindowedReport,
    dev_on: impl Fn(&str) -> bool,
    table_path: &Path,
    pending_dir: &Path,
    dry_run: bool,
) -> Result<Refresh> {
    let text =
        std::fs::read_to_string(table_path).with_context(|| format!("reading {}", table_path.display()))?;
    let current: WinRateTable =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", table_path.display()))?;
    let decision = refresh_due(report, &current, dev_on);
    let Refresh::Due(table) = &decision else {
        return Ok(decision);
    };
    if let Some(claim) = read_fragments(pending_dir)?
        .into_iter()
        .find(|named| named.fragment.win_rates)
    {
        bail!(
            "pending/{} claims the win-rate table already (R1329000)",
            claim.name
        );
    }
    if dry_run {
        return Ok(decision);
    }
    let fragment = refresh_fragment(table, report.patch.live_games);
    std::fs::create_dir_all(pending_dir).with_context(|| format!("creating {}", pending_dir.display()))?;
    write_json(&pending_dir.join(fragment_name(&fragment.title)), &fragment)?;
    std::fs::write(table_path, table_text(table))
        .with_context(|| format!("writing {}", table_path.display()))?;
    Ok(decision)
}

/// One built table row: `table_from_report_reads_in_deck`, `r807000_table_from_records_counts_as_the_public_stats_do`
/// and `rows_sorted_zero_games_dropped` pin it.
fn row_of(card: &str, wins: i32, games: i32) -> Option<WinRateRow> {
    if games <= 0 {
        return None;
    }
    Some(WinRateRow {
        id: card.to_string(),
        wins,
        games,
    })
}

/// The table from one JSON `stats report --json` export: R377's in-deck wins and games, `live`
/// only when the report read live games.
fn table_from_report(text: &str, patch: &str) -> Result<WinRateTable> {
    let report: jackioh_engine::CardStatsReport =
        serde_json::from_str(text).context("parsing the stats report")?;
    let mut cards: Vec<WinRateRow> = report
        .cards
        .iter()
        .filter_map(|stats| row_of(&stats.card, stats.in_deck.wins, stats.in_deck.games))
        .collect();
    cards.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(WinRateTable {
        patch: patch.to_string(),
        source: if report.filter.source == SourceFilter::Live {
            WinRateSource::Live
        } else {
            WinRateSource::Provisional
        },
        cards,
    })
}

/// JSONL game record files as `/api/stats/cards` counts them, through the same function (R1367000,
/// `windowed_card_stats`): each card over its own window at `patch`, every counted seat of a live
/// game (R807000) and both seats of a development game. With the totals, whose development records
/// say whether a window's provisional figures hold any.
fn windowed_records(texts: &[String], patch: &str) -> Result<(WindowedReport, StatsTotals)> {
    let mut totals = StatsTotals::default();
    for text in texts {
        for record in
            jackioh_cards::versions::read_game_record_lines(text).context("parsing the game records")?
        {
            totals.fold(&record, false);
        }
    }
    let history = jackioh_cards::patch_history();
    let report = windowed_card_stats(
        &totals,
        &WindowQuery {
            history: &history,
            at: patch,
            live_pilot: PilotFilter::Unified,
            mode: None,
            request: WindowRequest::Gated,
        },
    );
    Ok((report, totals))
}

/// The table a windowed report holds: each card's in-deck wins and games over its window, filed
/// under `report.at`; `live` only when no card's window counts a development game, which
/// `dev_on(version)` says a version holds.
fn table_of(report: &WindowedReport, dev_on: impl Fn(&str) -> bool) -> WinRateTable {
    let live = !report.cards.is_empty()
        && report.cards.iter().all(|card| {
            !card.window.sources().contains(&GameSource::Dev)
                || !card.window.counted.iter().any(|version| dev_on(version))
        });
    let mut cards: Vec<WinRateRow> = report
        .cards
        .iter()
        .filter_map(|card| {
            row_of(
                &card.stats.card,
                card.stats.in_deck.wins,
                card.stats.in_deck.games,
            )
        })
        .collect();
    cards.sort_by(|a, b| a.id.cmp(&b.id));
    WinRateTable {
        patch: report.at.clone(),
        source: if live {
            WinRateSource::Live
        } else {
            WinRateSource::Provisional
        },
        cards,
    }
}

/// Whether the totals hold a development record of `version`.
fn dev_on(totals: &StatsTotals, version: &str) -> bool {
    totals
        .records
        .keys()
        .any(|key| key.source == GameSource::Dev && key.patch == version)
}

/// The table from JSONL game record files, each card over its own window at `patch` (R1367000);
/// `live` only when no counted record is a development game.
fn table_from_records(texts: &[String], patch: &str) -> Result<WinRateTable> {
    let (report, totals) = windowed_records(texts, patch)?;
    Ok(table_of(&report, |version| dev_on(&totals, version)))
}

/// Whether the file is a JSON stats report rather than JSONL records: it parses as a
/// `CardStatsReport`, or as a `stats report --window <v> --json` export.
fn is_report(text: &str) -> bool {
    serde_json::from_str::<jackioh_engine::CardStatsReport>(text).is_ok() || is_windowed(text)
}

/// Whether the file is a `stats report --window <v> --json` export (R1367000).
fn is_windowed(text: &str) -> bool {
    serde_json::from_str::<WindowedReport>(text).is_ok()
}

/// `cargo jackioh winrates`.
pub fn run(args: Args) -> Result<()> {
    if args.inputs.is_empty() {
        bail!("name a stats report (--json) or JSONL record files");
    }
    jackioh_cards::register_all();
    let patch = args
        .patch
        .clone()
        .unwrap_or_else(|| jackioh_cards::CATALOG_VERSION.to_string());
    let mut texts: Vec<String> = Vec::with_capacity(args.inputs.len());
    for input in &args.inputs {
        texts.push(std::fs::read_to_string(input).with_context(|| format!("reading {}", input.display()))?);
    }
    let kinds: Vec<bool> = texts.iter().map(|text| is_report(text)).collect();
    if kinds.iter().all(|kind| *kind) {
        if texts.len() != 1 {
            bail!("name one stats report, not several");
        }
    } else if kinds.iter().any(|kind| *kind) {
        bail!("refusing a mix of a stats report and record files");
    }
    let path = crate::patches::repo_root()
        .join("crates")
        .join("cards")
        .join("data")
        .join("win_rates.json");
    if args.refresh {
        let decision = if kinds[0] {
            let report: WindowedReport = serde_json::from_str(&texts[0])
                .context("--refresh reads a `stats report --window <v> --json` export, or record files")?;
            // A report says nothing of the records behind it: a window that counts development games is
            // taken to hold some.
            refresh(&report, |_| true, &path, &pending_dir(), args.dry_run)?
        } else {
            let (report, totals) = windowed_records(&texts, &patch)?;
            refresh(
                &report,
                |version| dev_on(&totals, version),
                &path,
                &pending_dir(),
                args.dry_run,
            )?
        };
        match decision {
            Refresh::Due(table) => println!(
                "winrates: refresh due: {} (the patch's window has its live games){}",
                table.patch,
                if args.dry_run {
                    "; --dry-run wrote nothing"
                } else {
                    "; wrote the table and its pending fragment"
                }
            ),
            Refresh::Short { live_games } => println!(
                "winrates: no refresh: the patch's window has {live_games} live games of {PUBLIC_STATS_MIN_LIVE_GAMES}"
            ),
            Refresh::Current { table } => {
                println!("winrates: no refresh: the table ({table}) is this patch's already")
            }
        }
        return Ok(());
    }
    let table = if kinds[0] && is_windowed(&texts[0]) {
        table_of(&serde_json::from_str(&texts[0])?, |_| true)
    } else if kinds[0] {
        table_from_report(&texts[0], &patch)?
    } else {
        table_from_records(&texts, &patch)?
    };
    let mut out = serde_json::to_string_pretty(&table).unwrap_or_default();
    out.push('\n');
    if args.dry_run {
        print!("{out}");
        return Ok(());
    }
    std::fs::write(&path, out).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::CardStatsFilter;

    fn stats(card: &str, wins: i32, games: i32) -> jackioh_engine::CardStats {
        jackioh_engine::CardStats {
            card: card.to_string(),
            in_deck: jackioh_engine::Tally {
                games,
                wins,
                draws: 0,
            },
            opening_hand: jackioh_engine::Tally {
                games: 0,
                wins: 0,
                draws: 0,
            },
            going_first: jackioh_engine::Tally {
                games: 0,
                wins: 0,
                draws: 0,
            },
            going_second: jackioh_engine::Tally {
                games: 0,
                wins: 0,
                draws: 0,
            },
            played: jackioh_engine::Tally {
                games: 0,
                wins: 0,
                draws: 0,
            },
            played_radiant: jackioh_engine::Tally {
                games: 0,
                wins: 0,
                draws: 0,
            },
            played_normal: jackioh_engine::Tally {
                games: 0,
                wins: 0,
                draws: 0,
            },
            drawn_not_played: jackioh_engine::Tally {
                games: 0,
                wins: 0,
                draws: 0,
            },
        }
    }

    fn report(source: SourceFilter, cards: Vec<jackioh_engine::CardStats>) -> String {
        let report = jackioh_engine::CardStatsReport {
            filter: CardStatsFilter {
                source,
                mode: None,
                patch: None,
                pilot: PilotFilter::Unified,
            },
            games: 1,
            decks: 2,
            cards,
        };
        serde_json::to_string(&report).unwrap_or_default()
    }

    #[test]
    fn table_from_report_reads_in_deck() {
        let text = report(
            SourceFilter::Live,
            vec![stats("core-001", 3, 5), stats("core-002", 0, 0)],
        );
        let table = table_from_report(&text, "v0.3.3").unwrap_or_else(|error| panic!("{error:?}"));
        assert_eq!(table.patch, "v0.3.3");
        assert_eq!(table.source, WinRateSource::Live);
        assert_eq!(
            table.cards,
            vec![WinRateRow {
                id: "core-001".to_string(),
                wins: 3,
                games: 5,
            }]
        );
        let text = report(SourceFilter::Dev, vec![stats("core-001", 3, 5)]);
        let table = table_from_report(&text, "v0.3.3").unwrap_or_else(|error| panic!("{error:?}"));
        assert_eq!(table.source, WinRateSource::Provisional);
    }

    #[test]
    fn r807000_table_from_records_counts_as_the_public_stats_do() {
        use jackioh_engine::{
            GameMode, GameOverReason, GameRecord, GameSummary, PerPlayer, Pilot, PlayerId, SeatSummary,
            Winner,
        };
        jackioh_cards::register_all();
        let seat = |deck: &[&str]| SeatSummary {
            deck: deck.iter().map(|card| card.to_string()).collect(),
            opening: Vec::new(),
            drawn: Vec::new(),
            played: Vec::new(),
            played_turns: None,
            played_radiant: None,
            fatigue_draws: None,
            fatigue_damage: None,
            traps_set: None,
            traps_fired: None,
            position_switches: None,
            radiant_upgrades: None,
        };
        let line = |id: &str, source: GameSource, pilots: PerPlayer<Pilot>| {
            let record = GameRecord {
                id: id.to_string(),
                source,
                mode: GameMode::Bo1,
                patch: "v0.3.3".to_string(),
                pilots,
                game: GameSummary {
                    first: PlayerId::P1,
                    winner: Winner::P1,
                    reason: GameOverReason::Concede,
                    turns: 4,
                    seats: PerPlayer::new(seat(&["core-001"]), seat(&["core-001", "core-003"])),
                },
                build: None,
            };
            jackioh_cards::versions::game_record_line(&record)
        };
        let people = PerPlayer::new(Pilot::Human, Pilot::Human);
        let table = table_from_records(&[line("m1", GameSource::Live, people)], "v0.3.3")
            .unwrap_or_else(|error| panic!("{error:?}"));
        assert_eq!(table.source, WinRateSource::Live);
        let hit: Vec<(&str, i32, i32)> = table
            .cards
            .iter()
            .map(|row| (row.id.as_str(), row.wins, row.games))
            .collect();
        // Both seats held core-001: p1 won it, p2 lost it.
        assert!(hit.contains(&("core-001", 1, 2)), "{hit:?}");
        assert!(hit.contains(&("core-003", 0, 1)), "{hit:?}");
        let ais = PerPlayer::new(Pilot::Ai, Pilot::Ai);
        let table = table_from_records(&[line("dev:v0.3.3:s:1", GameSource::Dev, ais.clone())], "v0.3.3")
            .unwrap_or_else(|error| panic!("{error:?}"));
        assert_eq!(table.source, WinRateSource::Provisional);
        let hit: Vec<(&str, i32, i32)> = table
            .cards
            .iter()
            .map(|row| (row.id.as_str(), row.wins, row.games))
            .collect();
        assert!(hit.contains(&("core-001", 1, 2)), "{hit:?}");

        // R807000: p2 a ladder bot (`ai` on a live record): both sides count, as on /stats.
        // p1's core-001 won, p2's core-001 and core-003 lost.
        let bot_game = PerPlayer::new(Pilot::Human, Pilot::Ai);
        let table = table_from_records(&[line("m2", GameSource::Live, bot_game)], "v0.3.3")
            .unwrap_or_else(|error| panic!("{error:?}"));
        assert_eq!(table.source, WinRateSource::Live);
        let hit: Vec<(&str, i32, i32)> = table
            .cards
            .iter()
            .map(|row| (row.id.as_str(), row.wins, row.games))
            .collect();
        assert_eq!(hit, [("core-001", 1, 2), ("core-003", 0, 1)]);
        // Summed with a development game: the dev game's two seats and the live game's two.
        let both = [
            line("m2", GameSource::Live, PerPlayer::new(Pilot::Human, Pilot::Ai)),
            line("dev:v0.3.3:s:1", GameSource::Dev, ais),
        ];
        let table = table_from_records(&both, "v0.3.3").unwrap_or_else(|error| panic!("{error:?}"));
        assert_eq!(table.source, WinRateSource::Provisional);
        let hit: Vec<(&str, i32, i32)> = table
            .cards
            .iter()
            .map(|row| (row.id.as_str(), row.wins, row.games))
            .collect();
        assert_eq!(hit, [("core-001", 2, 4), ("core-003", 0, 2)]);
    }

    #[test]
    fn r1367000_table_from_records_reads_each_card_over_its_own_window() {
        use jackioh_engine::{
            GameMode, GameOverReason, GameRecord, GameSummary, PerPlayer, Pilot, PlayerId, SeatSummary,
            Winner,
        };
        jackioh_cards::register_all();
        let seat = |deck: &[&str]| SeatSummary {
            deck: deck.iter().map(|card| card.to_string()).collect(),
            ..SeatSummary::default()
        };
        // The second version, a micro after the patch before it, plays Classic #55 alone. Each line
        // is marked, as the build writes a record file (R1371000).
        let line = |id: &str, patch: &str| {
            jackioh_cards::versions::game_record_line(&GameRecord {
                id: id.to_string(),
                source: GameSource::Live,
                mode: GameMode::Bo1,
                patch: patch.to_string(),
                pilots: PerPlayer::new(Pilot::Human, Pilot::Human),
                game: GameSummary {
                    first: PlayerId::P1,
                    winner: Winner::P1,
                    reason: GameOverReason::Concede,
                    turns: 4,
                    seats: PerPlayer::new(seat(&["core-001", "classic-055"]), seat(&["core-003"])),
                },
                build: None,
            })
        };
        let table = table_from_records(&[line("m1", "v0.2.5"), line("m2", "v0.2.6")], "v0.2.6")
            .unwrap_or_else(|error| panic!("{error:?}"));
        let hit: Vec<(&str, i32, i32)> = table
            .cards
            .iter()
            .map(|row| (row.id.as_str(), row.wins, row.games))
            .collect();
        assert_eq!(
            hit,
            [("classic-055", 1, 1), ("core-001", 2, 2), ("core-003", 0, 2)]
        );
        assert_eq!(table.source, WinRateSource::Live);
    }

    /// R1368000: a history of one patch and a micro after it, and a newer patch.
    fn history() -> Vec<jackioh_engine::PatchStep> {
        use jackioh_engine::{PatchStep, PatchTier};
        let step = |version: &str, tier| PatchStep {
            version: version.to_string(),
            tier,
            plays: Vec::new(),
        };
        vec![
            step("v1", PatchTier::Major),
            step("v2", PatchTier::Patch),
            step("v2b", PatchTier::Micro),
            step("v3", PatchTier::Patch),
        ]
    }

    /// `live` live games on each version named, one of them holding CN Tech and won.
    fn totals(live: &[(&str, i64)]) -> StatsTotals {
        use jackioh_engine::{
            GameMode, GameOverReason, GameRecord, GameSummary, PerPlayer, Pilot, PlayerId, RecordsKey,
            SeatSummary, Winner,
        };
        let mut totals = StatsTotals::default();
        for (version, games) in live {
            let seat = |deck: &[&str]| SeatSummary {
                deck: deck.iter().map(|card| card.to_string()).collect(),
                ..SeatSummary::default()
            };
            totals.fold(
                &GameRecord {
                    id: format!("m-{version}"),
                    source: GameSource::Live,
                    mode: GameMode::Bo1,
                    patch: version.to_string(),
                    pilots: PerPlayer::new(Pilot::Human, Pilot::Human),
                    game: GameSummary {
                        first: PlayerId::P1,
                        winner: Winner::P1,
                        reason: GameOverReason::Concede,
                        turns: 4,
                        seats: PerPlayer::new(seat(&[TABLE_READERS[0]]), seat(&["core-001"])),
                    },
                    build: None,
                },
                false,
            );
            *totals
                .records
                .entry(RecordsKey {
                    patch: version.to_string(),
                    source: GameSource::Live,
                    mode: GameMode::Bo1,
                    pilots: (Pilot::Human, Pilot::Human),
                })
                .or_default() += games - 1;
        }
        totals
    }

    fn report_at(history: &[jackioh_engine::PatchStep], at: &str, totals: &StatsTotals) -> WindowedReport {
        windowed_card_stats(
            totals,
            &WindowQuery {
                history,
                at,
                live_pilot: PilotFilter::Unified,
                mode: None,
                request: WindowRequest::Gated,
            },
        )
    }

    fn table(patch: &str) -> WinRateTable {
        WinRateTable {
            patch: patch.to_string(),
            source: WinRateSource::Provisional,
            cards: Vec::new(),
        }
    }

    /// A scratch directory of its own with `data/win_rates.json` holding `table` and no fragment.
    fn scratch(table: &WinRateTable) -> (PathBuf, PathBuf, PathBuf) {
        static SCRATCH: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = SCRATCH.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("jackioh-winrates-refresh-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("win_rates.json");
        std::fs::write(&path, table_text(table)).unwrap();
        (dir.clone(), path, dir.join("pending"))
    }

    #[test]
    fn r1368000_a_refresh_is_due_when_the_patch_window_clears_and_the_table_is_from_an_earlier_patch() {
        let history = history();
        let short = PUBLIC_STATS_MIN_LIVE_GAMES - 1;
        // v2's window is v2 and v2b: 999 live games is short, 1,000 clears it.
        let under = report_at(
            &history,
            "v2b",
            &totals(&[("v1", 5000), ("v2", short - 10), ("v2b", 10)]),
        );
        assert_eq!(
            refresh_due(&under, &table("v1"), |_| false),
            Refresh::Short { live_games: short }
        );
        let over = report_at(
            &history,
            "v2b",
            &totals(&[("v1", 5000), ("v2", short - 9), ("v2b", 10)]),
        );
        let Refresh::Due(due) = refresh_due(&over, &table("v1"), |_| false) else {
            panic!("a refresh is due");
        };
        assert_eq!(due.patch, "v2b");
        assert_eq!(due.source, WinRateSource::Live);
        // CN Tech's row is its window's: one game on each of v2 and v2b, both won.
        let row = due
            .cards
            .iter()
            .find(|row| row.id == TABLE_READERS[0])
            .expect("CN Tech's row");
        assert_eq!((row.wins, row.games), (2, 2));
        // Until then the table holds the earlier patch's numbers, however many it carries.
        let (dir, path, pending) = scratch(&table("v1"));
        assert_eq!(
            refresh(&under, |_| false, &path, &pending, false).unwrap(),
            Refresh::Short { live_games: short }
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), table_text(&table("v1")));
        assert!(!pending.exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn r1368000_a_refresh_is_not_due_a_second_time_in_the_same_patch() {
        let history = history();
        let live = totals(&[("v2", PUBLIC_STATS_MIN_LIVE_GAMES), ("v2b", 1)]);
        let report = report_at(&history, "v2b", &live);
        let (dir, path, pending) = scratch(&table("v1"));
        assert!(matches!(
            refresh(&report, |_| false, &path, &pending, false).unwrap(),
            Refresh::Due(_)
        ));
        let written: WinRateTable = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(written.patch, "v2b");
        // Run again, before or after the fragment ships: the table is this patch's.
        assert_eq!(
            refresh(&report, |_| false, &path, &pending, false).unwrap(),
            Refresh::Current {
                table: "v2b".to_string()
            }
        );
        assert_eq!(read_fragments(&pending).unwrap().len(), 1);
        // The micro it ships as (v2c, say) is still the patch's; the next patch makes one due again.
        let history_on: Vec<_> = history
            .iter()
            .filter(|step| step.version != "v3")
            .cloned()
            .chain(std::iter::once(jackioh_engine::PatchStep {
                version: "v2c".to_string(),
                tier: jackioh_engine::PatchTier::Micro,
                plays: TABLE_READERS.iter().map(|id| id.to_string()).collect(),
            }))
            .collect();
        let micro = report_at(&history_on, "v2c", &live);
        assert_eq!(
            refresh_due(&micro, &written, |_| false),
            Refresh::Current {
                table: "v2b".to_string()
            }
        );
        let next = report_at(&history, "v3", &totals(&[("v3", PUBLIC_STATS_MIN_LIVE_GAMES)]));
        assert!(matches!(refresh_due(&next, &written, |_| false), Refresh::Due(_)));
        // A pending claim on the table already is refused rather than doubled.
        let (other, path, _) = scratch(&table("v1"));
        assert!(refresh(&next, |_| false, &path, &pending, false).is_err());
        std::fs::remove_dir_all(dir).unwrap();
        std::fs::remove_dir_all(other).unwrap();
    }

    #[test]
    fn r1368000_the_fragment_claims_cn_tech_and_na_tech_and_ships_as_a_micro() {
        use crate::classify::{self, Checked, Side, Tier};
        use crate::patches::patches_io::{Catalog, player_note_problems};
        jackioh_cards::register_all();
        for id in TABLE_READERS {
            assert!(jackioh_cards::CATALOG.contains_key(*id), "{id} is a shipped card");
        }
        assert_eq!(jackioh_cards::CATALOG[TABLE_READERS[0]].name, "CN Tech");
        assert_eq!(jackioh_cards::CATALOG[TABLE_READERS[1]].name, "NA Tech");

        let report = report_at(&history(), "v2b", &totals(&[("v2", PUBLIC_STATS_MIN_LIVE_GAMES)]));
        let (dir, path, pending) = scratch(&table("v1"));
        refresh(&report, |_| false, &path, &pending, false).unwrap();
        let fragments = read_fragments(&pending).unwrap();
        assert_eq!(fragments.len(), 1);
        let fragment = &fragments[0].fragment;
        assert_eq!(fragments[0].name, fragment_name(&fragment.title));
        assert_eq!(fragment.plays, TABLE_READERS);
        assert!(fragment.win_rates && fragment.cards.is_empty() && !fragment.rule);
        assert!(
            player_note_problems(&fragment.player).is_empty(),
            "{:?}",
            fragment.player
        );
        assert!(!fragment.title.is_empty() && !fragment.sources.is_empty() && !fragment.notes.is_empty());

        // `patches ship` reads its tier as `tier_of` does: no catalog change, its plays as claims.
        let side = Side {
            catalog: Catalog::new(),
            sets: Vec::new(),
        };
        let checked = Checked {
            scripts: fragment.plays.iter().map(|id| classify::family(id)).collect(),
            ..Checked::default()
        };
        let classified = classify::classify(&classify::read_diff(&side, &side, &[]), Some(&checked));
        assert_eq!(classified.tier, Some(Tier::Micro));
        assert_eq!(classified.cards, TABLE_READERS);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn r1368000_an_older_shipped_catalog_still_folds_with_its_own_table() {
        jackioh_cards::register_all();
        let dir = crate::patches::repo_root().join("crates/cards/patches/win_rates");
        let carried = jackioh_cards::carried_versions();
        let mut read = 0;
        for version in &carried {
            let Ok(text) = std::fs::read_to_string(dir.join(format!("{version}.json"))) else {
                continue;
            };
            let own: WinRateTable = serde_json::from_str(&text).unwrap();
            let folded = jackioh_cards::catalog_at(version).expect("carried").win_rates();
            assert_eq!(folded.patch, own.patch, "{version}");
            assert_eq!(folded.cards, own.cards, "{version}");
            read += 1;
        }
        assert!(read > 0, "a carried version ships a table");
        // A refresh writes the build's own table and no shipped version's.
        let report = report_at(&history(), "v2b", &totals(&[("v2", PUBLIC_STATS_MIN_LIVE_GAMES)]));
        let (scratch_dir, path, pending) = scratch(&table("v1"));
        refresh(&report, |_| false, &path, &pending, false).unwrap();
        for version in &carried {
            if let Ok(text) = std::fs::read_to_string(dir.join(format!("{version}.json"))) {
                let own: WinRateTable = serde_json::from_str(&text).unwrap();
                assert_eq!(
                    jackioh_cards::catalog_at(version)
                        .expect("carried")
                        .win_rates()
                        .cards,
                    own.cards
                );
            }
        }
        std::fs::remove_dir_all(scratch_dir).unwrap();
    }

    #[test]
    fn rows_sorted_zero_games_dropped() {
        let text = report(
            SourceFilter::Live,
            vec![
                stats("core-003", 1, 2),
                stats("core-001", 0, 0),
                stats("core-002", 2, 4),
            ],
        );
        let table = table_from_report(&text, "v0.3.3").unwrap_or_else(|error| panic!("{error:?}"));
        let ids: Vec<&str> = table.cards.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(ids, vec!["core-002", "core-003"]);
    }

    #[test]
    fn refuses_a_mix() {
        let report_text = report(SourceFilter::Live, vec![stats("core-001", 1, 1)]);
        assert!(is_report(&report_text));
        assert!(!is_report("{\"id\": \"m1\"}\n"));
    }
}
