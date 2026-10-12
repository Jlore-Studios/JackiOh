//! Admin script: the balance watch (SPEC §9.11, R377, R378, R654, R1451; issue #699's intake,
//! proposed by issue #726).
//!
//! ```text
//! jackioh-server balance-watch --patch=<version> [--source=live|all]
//! ```
//!
//! Reads one patch's game records through the store (DATABASE_URL) and prints, as Markdown to paste
//! into #699, the cards whose in-deck win rate stands out against 50%: too strong, too weak, and a
//! played delta big enough to ask after, each with the games behind it. The bands are `config.rs`
//! (CLAUDE.md rule 9): a z-score against 50% with at least `BALANCE_WATCH_MIN_GAMES` games, never a
//! fixed win-rate cut-off, because the same 55% is noise at 40 games and a signal at 700.
//!
//! It is advisory and read-only. It changes no card, tunes nothing and files nothing: below the
//! R654 gate (`PUBLIC_STATS_MIN_LIVE_GAMES` live games) the figures blend AI development runs with
//! live games and the report says they are provisional, and a big played delta is the sign of a card
//! the AI pilot cannot use, so every flag asks to be confirmed against live human games, never to be
//! acted on as it stands. Seat counting matches the public endpoint (`api::stats::public_report`,
//! R1451): live records count their human seats only, development records (read only with
//! `--source=all` while the gate stands) count every seat.

use std::cmp::Ordering;

use anyhow::{anyhow, bail};
use indexmap::IndexMap;

use jackioh_engine::wire::{
    CardStats, CardStatsFilter, GameRecord, GameSource, PilotFilter, SourceFilter, Tally, card_stats,
    played_delta,
};

use super::card_stats::flag;
use crate::config::{
    BALANCE_WATCH_MIN_GAMES, BALANCE_WATCH_PLAYED_DELTA_THRESHOLD, BALANCE_WATCH_Z_THRESHOLD,
    CARD_STATS_MIN_SAMPLE, PUBLIC_STATS_MIN_LIVE_GAMES,
};
use crate::db::store::{Db, GameRecordQuery, StoreError};
use crate::env::quoted;

const USAGE: &str = "Usage: jackioh-server balance-watch --patch=<version> [--source=live|all]

  --patch=<version>    One patch, as patches.json names it. Required.
  --source=live|all    Live games only, or live games plus AI development runs. Default: all.

Advisory and read-only: it prints a Markdown report to paste into #699, and
changes nothing.";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BalanceWatchOptions {
    pub patch: String,
    pub source: SourceFilter,
}

/// One card's figures as the report prints them: the in-deck win rate (draws count as not won,
/// R377) over `games` in-deck games, its z-score against 50%, and its played delta when both sides
/// have games.
#[derive(Clone, Debug, PartialEq)]
pub struct FlaggedCard {
    pub card: String,
    pub win_rate: f64,
    pub games: i64,
    pub z: f64,
    pub played_delta: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BalanceWatchReport {
    pub patch: String,
    pub source: SourceFilter,
    /// Whether the R654 gate still stands: under `PUBLIC_STATS_MIN_LIVE_GAMES` live games the
    /// figures are provisional.
    pub provisional: bool,
    /// Live games on the patch, which the gate counts (a game against a ladder bot included, R1451).
    pub live_games: i64,
    /// Records the report counted, and the seats (decks) across them.
    pub games: i64,
    pub decks: i64,
    /// At least `BALANCE_WATCH_MIN_GAMES` games and `z >= +BALANCE_WATCH_Z_THRESHOLD`, best first.
    pub high: Vec<FlaggedCard>,
    /// At least `BALANCE_WATCH_MIN_GAMES` games and `z <= -BALANCE_WATCH_Z_THRESHOLD`, worst first.
    pub low: Vec<FlaggedCard>,
    /// At least `BALANCE_WATCH_MIN_GAMES` games and `|played delta| >=
    /// BALANCE_WATCH_PLAYED_DELTA_THRESHOLD`, biggest gap first.
    pub delta: Vec<FlaggedCard>,
    /// `CARD_STATS_MIN_SAMPLE` to `BALANCE_WATCH_MIN_GAMES` games: figures shown, never flagged.
    pub watched: Vec<FlaggedCard>,
    /// Under `CARD_STATS_MIN_SAMPLE` games: never flagged.
    pub not_enough: Vec<String>,
}

/// Parses `--flag=value` arguments, refusing anything it does not know. `--patch` is required;
/// `--source` reads live games plus AI development runs unless `live` is named, and `dev` is
/// refused: a development run is never read on its own, only as the provisional padding.
pub fn parse_balance_watch_args(argv: &[String]) -> anyhow::Result<BalanceWatchOptions> {
    let mut patch: Option<String> = None;
    let mut source = SourceFilter::All;
    for arg in argv {
        let Some((name, raw)) = flag(arg) else {
            bail!("Unrecognised argument {}.\n\n{USAGE}", quoted(arg));
        };
        match name {
            "patch" => patch = Some(raw.to_owned()),
            "source" => {
                source = match raw {
                    "live" => SourceFilter::Live,
                    "all" => SourceFilter::All,
                    _ => bail!(
                        "--source must be one of live, all (got {}).\n\n{USAGE}",
                        quoted(raw)
                    ),
                };
            }
            _ => bail!("Unrecognised option --{name}.\n\n{USAGE}"),
        }
    }
    let Some(patch) = patch else {
        bail!("--patch=<version> is required.\n\n{USAGE}");
    };
    Ok(BalanceWatchOptions { patch, source })
}

/// R377's flag: the normal approximation of an in-deck win rate against 50%, in units of standard
/// deviations. Draws count as not won, because `wins / games` counts them so. `None` with no games.
pub fn z_score(wins: i64, games: i64) -> Option<f64> {
    if games == 0 {
        return None;
    }
    Some((wins as f64 - games as f64 * 0.5) / (games as f64 * 0.25).sqrt())
}

/// R654's gate: under `PUBLIC_STATS_MIN_LIVE_GAMES` live games the figures are provisional.
pub fn is_provisional(live_games: i64) -> bool {
    live_games < PUBLIC_STATS_MIN_LIVE_GAMES
}

fn add_tally(into: &mut Tally, other: &Tally) {
    into.games += other.games;
    into.wins += other.wins;
    into.draws += other.draws;
}

fn merge_into(by_card: &mut IndexMap<String, CardStats>, stat: CardStats) {
    match by_card.get_mut(&stat.card) {
        None => {
            by_card.insert(stat.card.clone(), stat);
        }
        Some(held) => {
            add_tally(&mut held.in_deck, &stat.in_deck);
            add_tally(&mut held.opening_hand, &stat.opening_hand);
            add_tally(&mut held.going_first, &stat.going_first);
            add_tally(&mut held.going_second, &stat.going_second);
            add_tally(&mut held.played, &stat.played);
            add_tally(&mut held.drawn_not_played, &stat.drawn_not_played);
        }
    }
}

/// R377: the report the options ask for, off the records the store holds. One
/// `GameRecordQuery` through `game_records_list`, then the counting replicates
/// `api::stats::public_report` locally (R1451): live records count their human seats only, and
/// development records count every seat, but only with `--source=all` while the gate stands —
/// `--source=live` always counts live-only, and once the gate clears development runs are ignored
/// with no toggle to bring them back.
pub async fn balance_watch_report(
    db: &Db,
    options: &BalanceWatchOptions,
) -> Result<BalanceWatchReport, StoreError> {
    let query = GameRecordQuery {
        source: options.source,
        mode: None,
        patch: Some(options.patch.clone()),
    };
    let mut tx = db.begin(None).await?;
    let records = tx.game_records_list(&query).await?;
    tx.commit().await?;

    Ok(balance_watch_report_for_records(&records, options))
}

fn balance_watch_report_for_records(
    records: &[GameRecord],
    options: &BalanceWatchOptions,
) -> BalanceWatchReport {
    let live_games = records
        .iter()
        .filter(|record| record.source == GameSource::Live)
        .count() as i64;
    let provisional = is_provisional(live_games);

    let filter = |source: SourceFilter, pilot: PilotFilter| CardStatsFilter {
        source,
        mode: None,
        patch: Some(options.patch.clone()),
        pilot,
    };
    let mut by_card: IndexMap<String, CardStats> = IndexMap::new();
    let (mut games, mut decks) = (0i64, 0i64);
    let live = card_stats(records, &filter(SourceFilter::Live, PilotFilter::Human));
    games += live.games as i64;
    decks += live.decks as i64;
    for stat in live.cards {
        merge_into(&mut by_card, stat);
    }
    if options.source == SourceFilter::All && provisional {
        let dev = card_stats(records, &filter(SourceFilter::Dev, PilotFilter::Unified));
        games += dev.games as i64;
        decks += dev.decks as i64;
        for stat in dev.cards {
            merge_into(&mut by_card, stat);
        }
    }

    let mut high: Vec<FlaggedCard> = Vec::new();
    let mut low: Vec<FlaggedCard> = Vec::new();
    let mut delta: Vec<FlaggedCard> = Vec::new();
    let mut watched: Vec<FlaggedCard> = Vec::new();
    let mut not_enough: Vec<String> = Vec::new();
    for stat in by_card.into_values() {
        let card_games = stat.in_deck.games as i64;
        if card_games < CARD_STATS_MIN_SAMPLE {
            not_enough.push(stat.card);
            continue;
        }
        let row = FlaggedCard {
            card: stat.card.clone(),
            win_rate: stat.in_deck.wins as f64 / card_games as f64,
            games: card_games,
            z: z_score(stat.in_deck.wins as i64, card_games).unwrap_or(0.0),
            played_delta: played_delta(&stat),
        };
        if card_games < BALANCE_WATCH_MIN_GAMES {
            watched.push(row);
            continue;
        }
        if row.z >= BALANCE_WATCH_Z_THRESHOLD {
            high.push(row.clone());
        }
        if row.z <= -BALANCE_WATCH_Z_THRESHOLD {
            low.push(row.clone());
        }
        if row
            .played_delta
            .is_some_and(|gap| gap.abs() >= BALANCE_WATCH_PLAYED_DELTA_THRESHOLD)
        {
            delta.push(row);
        }
    }
    high.sort_by(|a, b| b.z.partial_cmp(&a.z).unwrap_or(Ordering::Equal));
    low.sort_by(|a, b| a.z.partial_cmp(&b.z).unwrap_or(Ordering::Equal));
    delta.sort_by(|a, b| {
        b.played_delta
            .unwrap_or(0.0)
            .abs()
            .partial_cmp(&a.played_delta.unwrap_or(0.0).abs())
            .unwrap_or(Ordering::Equal)
    });
    watched.sort_by(|a, b| b.games.cmp(&a.games).then_with(|| a.card.cmp(&b.card)));
    not_enough.sort();

    BalanceWatchReport {
        patch: options.patch.clone(),
        source: options.source,
        provisional,
        live_games,
        games,
        decks,
        high,
        low,
        delta,
        watched,
        not_enough,
    }
}

fn card_cell(card: &FlaggedCard, name_of: &dyn Fn(&str) -> Option<String>) -> String {
    match name_of(&card.card) {
        Some(name) => format!("{name} (`{}`)", card.card),
        None => format!("`{}`", card.card),
    }
}

fn flag_table(out: &mut String, rows: &[FlaggedCard], name_of: &dyn Fn(&str) -> Option<String>) {
    if rows.is_empty() {
        out.push_str("No cards flagged.\n");
        return;
    }
    out.push_str("| Card | In-deck | Games | z | Played Δ |\n| --- | ---: | ---: | ---: | ---: |\n");
    for row in rows {
        let rate = format!("{:.1}%", row.win_rate * 100.0);
        let z = format!("{:+.2}", row.z);
        let gap = row
            .played_delta
            .map(|gap| format!("{:+.1} pts", gap * 100.0))
            .unwrap_or_else(|| "—".to_owned());
        out.push_str(&format!(
            "| {} | {rate} | {} | {z} | {gap} |\n",
            card_cell(row, name_of),
            row.games
        ));
    }
}

/// The report as the script prints it: Markdown to paste into #699.
pub fn render_balance_watch(report: &BalanceWatchReport, name_of: &dyn Fn(&str) -> Option<String>) -> String {
    let mut out = String::from("# Balance watch");
    out.push_str(&format!(" — {}\n\n", report.patch));
    let source = match report.source {
        SourceFilter::Live => "live games only (--source=live)",
        SourceFilter::All => "live games and AI development runs (--source=all)",
        // Unreachable: the parser refuses `--source=dev`.
        SourceFilter::Dev => "AI development runs",
    };
    out.push_str(&format!("Source: {source}, patch {}.\n", report.patch));
    out.push_str(&format!(
        "Counted {} games ({} decks).\n",
        report.games, report.decks
    ));
    if report.provisional {
        out.push_str(&format!(
            "Gate: provisional — {} live games of {PUBLIC_STATS_MIN_LIVE_GAMES} \
             (PUBLIC_STATS_MIN_LIVE_GAMES); treat every figure as provisional.\n",
            report.live_games
        ));
    } else {
        out.push_str(&format!(
            "Gate: live-only — the gate cleared with {} live games \
             (PUBLIC_STATS_MIN_LIVE_GAMES is {PUBLIC_STATS_MIN_LIVE_GAMES}); AI development runs are ignored.\n",
            report.live_games
        ));
    }
    out.push_str("\n## Too strong\n\n");
    out.push_str(
        "In-deck win rate above 50% with at least 40 games (BALANCE_WATCH_MIN_GAMES) and \
         z >= +2.5 (BALANCE_WATCH_Z_THRESHOLD).\n\n",
    );
    flag_table(&mut out, &report.high, name_of);
    out.push_str("\n## Too weak\n\n");
    out.push_str("In-deck win rate below 50% with at least 40 games and z <= -2.5.\n\n");
    flag_table(&mut out, &report.low, name_of);
    out.push_str("\n## Played delta\n\n");
    out.push_str(
        "Played win rate minus drawn-but-not-played win rate of at least 15 points \
         (BALANCE_WATCH_PLAYED_DELTA_THRESHOLD) with at least 40 games: the sign of a card the AI \
         pilot cannot use, whatever its in-deck win rate.\n\n",
    );
    flag_table(&mut out, &report.delta, name_of);
    out.push_str("\n## Watched\n\n");
    out.push_str(
        "20–39 games (CARD_STATS_MIN_SAMPLE to BALANCE_WATCH_MIN_GAMES): figures shown, never \
         flagged.\n\n",
    );
    flag_table(&mut out, &report.watched, name_of);
    out.push_str("\n## Not enough games\n\n");
    if report.not_enough.is_empty() {
        out.push_str("Every counted card has enough games.\n");
    } else {
        out.push_str(&format!(
            "{} card{} under {CARD_STATS_MIN_SAMPLE} games, never flagged: {}.\n",
            report.not_enough.len(),
            if report.not_enough.len() == 1 { "" } else { "s" },
            report
                .not_enough
                .iter()
                .map(|card| format!("`{card}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    out.push_str(
        "\nAdvisory only: this changes nothing and files nothing. Confirm every flag against live \
         human games before acting in #699.\n",
    );
    out
}

/// TS `main()`: `balance-watch`'s arguments after the subcommand (`main.rs`).
pub async fn run(args: Vec<String>) -> anyhow::Result<()> {
    let options = parse_balance_watch_args(&args)?;
    let connection_string = std::env::var("DATABASE_URL").unwrap_or_default();
    if connection_string.is_empty() {
        bail!("DATABASE_URL is not set (see docs/architecture.md, env-var contract).");
    }

    // The catalog's names come from the compiled-in catalog (TS read it with `loadCatalog()`).
    let catalog = &*jackioh_cards::CATALOG;
    // One connection: this process runs one read and exits.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&connection_string)
        .await
        .map_err(|error| anyhow!("{error}"))?;
    let db = Db::Pg(pool);
    let outcome = balance_watch_report(&db, &options).await;
    db.close().await;
    let report = outcome?;
    let name_of = |card: &str| catalog.get(card).map(|def| def.name.clone());
    print!("{}", render_balance_watch(&report, &name_of));
    Ok(())
}
