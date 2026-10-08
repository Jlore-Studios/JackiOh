//! Admin script: writes the game records `stats-cards` reads to a file, so they can be worked on
//! outside Postgres (SPEC §9.11, R376, R378).
//!
//! ```text
//! jackioh-server stats-export --out=<records.jsonl> [--source=live|dev|all] [--mode=bo1|bo3|random]
//!   [--patch=<version>]
//! ```
//!
//! Reads `public.game_records` through the store (DATABASE_URL) with the query `stats-cards` makes
//! for the same flags, and writes one GameRecord per line in id order: the format `cargo jackioh
//! stats --out` and the training lanes write, which `stats-import` and `cargo jackioh stats report`
//! read and `analysis/` loads. Live games only unless `--source=dev` or `--source=all` says
//! otherwise (R378). Pilots are per seat, so the file holds every seat of every game it names; the
//! pilot filter and the card are for the report that reads it. `--out` is resolved against the
//! directory the command was started in, and an existing file there is replaced.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail};

use jackioh_engine::wire::{
    DEFAULT_CARD_STATS_FILTER, GAME_MODES, GameMode, GameRecord, SOURCE_FILTERS, SourceFilter,
};

use crate::cli::card_stats::{flag, one_of};
use crate::db::store::{Db, GameRecordQuery, StoreError};
use crate::env::quoted;

const USAGE: &str = "Usage: jackioh-server stats-export --out=<records.jsonl> [options]

  --out=<file>               Where to write the records, one JSON object per line. Required.
  --source=live|dev|all      Live games (the default), an AI development run's, or both.
  --mode=bo1|bo3|random      One match type. Default: every mode.
  --patch=<version>          One patch, as patches.json names it. Default: every patch.";

#[derive(Clone, Debug, PartialEq)]
pub struct ExportOptions {
    pub query: GameRecordQuery,
    pub out: PathBuf,
}

/// Parses `--flag=value` arguments, refusing anything it does not know and a call with no `--out`.
pub fn parse_export_args(argv: &[String]) -> anyhow::Result<ExportOptions> {
    let mut query = GameRecordQuery {
        source: DEFAULT_CARD_STATS_FILTER.source,
        mode: None,
        patch: None,
    };
    let mut out: Option<PathBuf> = None;

    for arg in argv {
        let Some((name, raw)) = flag(arg) else {
            bail!("Unrecognised argument {}.\n\n{USAGE}", quoted(arg));
        };
        match name {
            "source" => query.source = one_of::<SourceFilter>(name, raw, SOURCE_FILTERS, USAGE)?,
            "mode" => query.mode = Some(one_of::<GameMode>(name, raw, GAME_MODES, USAGE)?),
            "patch" => query.patch = Some(raw.to_owned()),
            "out" => out = Some(PathBuf::from(raw)),
            _ => bail!("Unrecognised option --{name}.\n\n{USAGE}"),
        }
    }

    let Some(out) = out else {
        bail!(
            "--out=<file> is required: the records are written to a file, never to the console.\n\n{USAGE}"
        );
    };
    Ok(ExportOptions { query, out })
}

/// The records the query names, in id order, as the store lists them.
pub async fn read_records(db: &Db, query: &GameRecordQuery) -> Result<Vec<GameRecord>, StoreError> {
    let mut tx = db.begin(None).await?;
    let records = tx.game_records_list(query).await?;
    tx.commit().await?;
    Ok(records)
}

/// One JSON object per line, each ended by a newline: what `cargo jackioh stats --out` writes.
pub fn render_records(records: &[GameRecord]) -> anyhow::Result<String> {
    let mut text = String::new();
    for record in records {
        text.push_str(&serde_json::to_string(record)?);
        text.push('\n');
    }
    Ok(text)
}

/// Reads the records the options name and writes them to the file, answering how many.
pub async fn export_records(db: &Db, options: &ExportOptions) -> anyhow::Result<usize> {
    let records = read_records(db, &options.query).await?;
    let text = render_records(&records)?;
    tokio::fs::write(&options.out, text)
        .await
        .map_err(|error| anyhow!("{}: {error}", Path::new(&options.out).display()))?;
    Ok(records.len())
}

/// `stats-export`'s arguments after the subcommand (`main.rs`).
pub async fn run(args: Vec<String>) -> anyhow::Result<()> {
    let options = parse_export_args(&args)?;
    let connection_string = std::env::var("DATABASE_URL").unwrap_or_default();
    if connection_string.is_empty() {
        bail!("DATABASE_URL is not set (see docs/architecture.md, env-var contract).");
    }

    // One connection: this process runs one read and exits.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&connection_string)
        .await
        .map_err(|error| anyhow!("{error}"))?;
    let db = Db::Pg(pool);
    let outcome = export_records(&db, &options).await;
    db.close().await;
    let written = outcome?;
    println!(
        "stats:export: wrote {written} records to {}",
        options.out.display()
    );
    Ok(())
}
