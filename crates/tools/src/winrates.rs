//! ME-STATS (docs/meditative-set.md M5, SPEC §8.8 row 50): `cargo jackioh winrates` writes
//! `crates/cards/data/win_rates.json`, the table M #50 CN Tech reads, from a `stats report --json`
//! export or from JSONL game record files. The table is `{ patch, source, cards }` with R377's
//! in-deck wins and games per card id, as `/api/stats/cards` ranks by; the build compiles it in and
//! never reads it at run time (CLAUDE.md rule 4).
//!
//! The command does not apply R654's gate: the threshold lives in the server, and the exporter
//! picks `--source` there. What reaches the table is what the input holds — live games when the
//! input is all live, provisional otherwise.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use jackioh_engine::{
    CardStatsFilter, GameSource, PilotFilter, SourceFilter, WinRateRow, WinRateSource, WinRateTable,
    card_stats, parse_game_record_lines,
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
}

/// One built table row: `table_from_report_reads_in_deck`, `table_from_records_counts_every_seat`
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

/// The table from JSONL game record files: every record counted under `{ source: All, mode: None,
/// patch, pilot: Unified }`, `live` only when every record counted is live.
fn table_from_records(texts: &[String], patch: &str) -> Result<WinRateTable> {
    let mut records = Vec::new();
    for text in texts {
        records.extend(parse_game_record_lines(text).context("parsing the game records")?);
    }
    let filter = CardStatsFilter {
        source: SourceFilter::All,
        mode: None,
        patch: Some(patch.to_string()),
        pilot: PilotFilter::Unified,
    };
    let report = card_stats(&records, &filter);
    let counted: Vec<&jackioh_engine::GameRecord> = records
        .iter()
        .filter(|record| {
            jackioh_engine::record_matches(record, &filter)
                && !jackioh_engine::seats_counted(record, filter.pilot).is_empty()
        })
        .collect();
    let live = !counted.is_empty() && counted.iter().all(|record| record.source == GameSource::Live);
    let mut cards: Vec<WinRateRow> = report
        .cards
        .iter()
        .filter_map(|stats| row_of(&stats.card, stats.in_deck.wins, stats.in_deck.games))
        .collect();
    cards.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(WinRateTable {
        patch: patch.to_string(),
        source: if live {
            WinRateSource::Live
        } else {
            WinRateSource::Provisional
        },
        cards,
    })
}

/// Whether the file is a JSON stats report rather than JSONL records: it parses as a
/// `CardStatsReport`.
fn is_report(text: &str) -> bool {
    serde_json::from_str::<jackioh_engine::CardStatsReport>(text).is_ok()
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
    let table = if kinds[0] {
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
    let path = crate::patches::repo_root()
        .join("crates")
        .join("cards")
        .join("data")
        .join("win_rates.json");
    std::fs::write(&path, out).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn table_from_records_counts_every_seat() {
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
        };
        let line = |id: &str, source: GameSource| {
            let record = GameRecord {
                id: id.to_string(),
                source,
                mode: GameMode::Bo1,
                patch: "v0.3.3".to_string(),
                pilots: PerPlayer::new(Pilot::Human, Pilot::Human),
                game: GameSummary {
                    first: PlayerId::P1,
                    winner: Winner::P1,
                    reason: GameOverReason::Concede,
                    turns: 4,
                    seats: PerPlayer::new(seat(&["core-001"]), seat(&["core-001", "core-003"])),
                },
            };
            serde_json::to_string(&record).unwrap_or_default()
        };
        let table = table_from_records(&vec![line("m1", GameSource::Live)], "v0.3.3")
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
        let table = table_from_records(&vec![line("dev:v0.3.3:s:1", GameSource::Dev)], "v0.3.3")
            .unwrap_or_else(|error| panic!("{error:?}"));
        assert_eq!(table.source, WinRateSource::Provisional);
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
