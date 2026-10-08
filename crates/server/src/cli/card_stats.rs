//! Admin script: card win rates off the game records (SPEC §9.11, R377, R378;
//! ← `apps/server/src/db/card-stats.ts`).
//!
//! ```text
//! jackioh-server stats-cards [--source=live|dev|all] [--mode=bo1|bo3|random]
//!   [--patch=<version>] [--pilot=human|ai|unified] [--card=<id>] [--json]
//! ```
//!
//! Reads `public.game_records` through the store (DATABASE_URL), keeps the records the filter names,
//! and prints every card's four breakdowns — in deck, in the opening hand, going first and going
//! second, played against drawn but not played — each with the games behind it. With nothing named
//! it reads live games of every mode, patch and pilot: a development run's records are read only
//! with `--source=dev` or `--source=all` (R378). `--json` prints the report as data instead.
//!
//! The arithmetic is `card_stats` in `jackioh_engine::wire::stats` (TS `packages/shared/src/stats.ts`),
//! the same the AI's development run prints its own games with, so a pre-release run and the live
//! games after it compare figure for figure: run both with the same `--patch`.

use anyhow::{anyhow, bail};
use serde::Serialize;

use jackioh_engine::wire::{
    CardStatsFilter, CardStatsReport, DEFAULT_CARD_STATS_FILTER, GAME_MODES, GameMode, PILOT_FILTERS,
    PilotFilter, SOURCE_FILTERS, SourceFilter, card_stats, format_card_stats,
};

use crate::db::store::{Db, GameRecordQuery, StoreError};
use crate::env::quoted;

const USAGE: &str = "Usage: jackioh-server stats-cards [options]

  --source=live|dev|all      Live games (the default), an AI development run's, or both.
  --mode=bo1|bo3|random      One match type. Default: every mode.
  --patch=<version>          One patch, as patches.json names it. Default: every patch.
  --pilot=human|ai|unified   The seats counted. Default: unified, every seat.
  --card=<id>                One card's row only.
  --json                     The report as JSON.";

#[derive(Clone, Debug, PartialEq)]
pub struct CardStatsOptions {
    pub filter: CardStatsFilter,
    /// One card's row, or `None` for every card.
    pub card: Option<String>,
    pub json: bool,
}

/// The literal a string-union value serialises as (`"live"`, `"bo1"`, …): what the flag's value is
/// compared with and what the refusal lists.
pub(crate) fn literal<T: Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(text)) => text,
        _ => String::new(),
    }
}

/// The value of a `--name=value` flag among `allowed`, or a refusal listing them and ending with the
/// command's own `usage`.
pub(crate) fn one_of<T: Copy + Serialize>(
    name: &str,
    raw: &str,
    allowed: &[T],
    usage: &str,
) -> anyhow::Result<T> {
    if let Some(found) = allowed.iter().find(|value| literal(*value) == raw) {
        return Ok(*found);
    }
    let names: Vec<String> = allowed.iter().map(literal).collect();
    bail!(
        "--{name} must be one of {} (got {}).\n\n{usage}",
        names.join(", "),
        quoted(raw)
    )
}

/// TS's `/^--(?<name>[a-z]+)=(?<value>.+)$/u`: the name, then the value, or `None` when the argument
/// is not of that form. `.` matches anything but a line terminator, so a value holding one does not
/// match either.
pub(crate) fn flag(arg: &str) -> Option<(&str, &str)> {
    let rest = arg.strip_prefix("--")?;
    let (name, value) = rest.split_once('=')?;
    if name.is_empty() || !name.bytes().all(|byte| byte.is_ascii_lowercase()) {
        return None;
    }
    if value.is_empty()
        || value
            .chars()
            .any(|ch| matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
    {
        return None;
    }
    Some((name, value))
}

/// Parses `--flag=value` arguments (and the bare `--json`), refusing anything it does not know.
pub fn parse_card_stats_args(argv: &[String]) -> anyhow::Result<CardStatsOptions> {
    let mut filter: CardStatsFilter = DEFAULT_CARD_STATS_FILTER.clone();
    let mut card: Option<String> = None;
    let mut json = false;

    for arg in argv {
        if arg == "--json" {
            json = true;
            continue;
        }
        let Some((name, raw)) = flag(arg) else {
            bail!("Unrecognised argument {}.\n\n{USAGE}", quoted(arg));
        };
        match name {
            "source" => filter.source = one_of::<SourceFilter>(name, raw, SOURCE_FILTERS, USAGE)?,
            "mode" => filter.mode = Some(one_of::<GameMode>(name, raw, GAME_MODES, USAGE)?),
            "patch" => filter.patch = Some(raw.to_owned()),
            "pilot" => filter.pilot = one_of::<PilotFilter>(name, raw, PILOT_FILTERS, USAGE)?,
            "card" => card = Some(raw.to_owned()),
            _ => bail!("Unrecognised option --{name}.\n\n{USAGE}"),
        }
    }

    Ok(CardStatsOptions { filter, card, json })
}

/// R377: the report the options ask for, off the records the store holds.
pub async fn card_stats_report(db: &Db, options: &CardStatsOptions) -> Result<CardStatsReport, StoreError> {
    let query = GameRecordQuery {
        source: options.filter.source,
        mode: options.filter.mode,
        patch: options.filter.patch.clone(),
    };
    let mut tx = db.begin(None).await?;
    let records = tx.game_records_list(&query).await?;
    tx.commit().await?;
    let mut report = card_stats(&records, &options.filter);
    if let Some(only) = &options.card {
        report.cards.retain(|stats| &stats.card == only);
    }
    Ok(report)
}

/// The report as the script prints it: the table, with the catalog's names, or JSON.
pub fn render_card_stats(
    report: &CardStatsReport,
    options: &CardStatsOptions,
    name_of: &dyn Fn(&str) -> Option<String>,
) -> String {
    if options.json {
        return serde_json::to_string_pretty(report).unwrap_or_default();
    }
    let table = format_card_stats(report, name_of);
    if let Some(card) = &options.card
        && report.cards.is_empty()
    {
        return format!("{table}\nNo deck the filter counts held {card}.");
    }
    table
}

/// TS `main()`: `stats-cards`'s arguments after the subcommand (`main.rs`).
pub async fn run(args: Vec<String>) -> anyhow::Result<()> {
    let options = parse_card_stats_args(&args)?;
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
    let outcome = card_stats_report(&db, &options).await;
    db.close().await;
    let report = outcome?;
    let name_of = |card: &str| catalog.get(card).map(|def| def.name.clone());
    println!("{}", render_card_stats(&report, &options, &name_of));
    Ok(())
}
