//! Admin script: writes the game records whose detail is still held to a file, so they can be
//! worked on outside Postgres (SPEC §9.11, R376, R378).
//!
//! ```text
//! jackioh-server stats-export --out=<records.jsonl> [--source=live|dev|all] [--mode=bo1|bo3|random]
//!   [--patch=<version>]
//! jackioh-server stats-export --claims --out=<claims.jsonl>
//! ```
//!
//! Reads `public.game_records` through the store (DATABASE_URL) for the sources, mode and patch the
//! flags name. R1044001: only the records written in the last `GAME_RECORD_RAW_RETENTION_DAYS` hold
//! their detail, so only those are written. One GameRecord per line in id order, every seat of every
//! game: the format `cargo jackioh stats --out` and the training lanes write, which `stats-import` and
//! `cargo jackioh stats report` read and `analysis/` loads. `--out` is resolved against the directory
//! the command was started in, and an existing file there is replaced.
//!
//! `--claims` writes migration 0078's practice claim counts instead (#2884, §14.8): one
//! `{ day, outcome, count }` per line, oldest day first, how many claims ended that UTC day paid or
//! refused for each reason. Aggregates alone: no account, seed or log leaves with them.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail};

use jackioh_engine::wire::{
    DEFAULT_CARD_STATS_FILTER, GAME_MODES, GameMode, GameRecord, SOURCE_FILTERS, SourceFilter,
};

use crate::cli::card_stats::{flag, one_of};
use crate::cli::{connect_one, database_url};
use crate::db::store::{Db, GameRecordQuery, StoreError};
use crate::economy::store::PracticeClaimCount;
use crate::env::quoted;

const USAGE: &str = "Usage: jackioh-server stats-export --out=<records.jsonl> [options]

  --out=<file>               Where to write the records, one JSON object per line. Required.
  --source=live|dev|all      Live games (the default), an AI development run's, or both.
  --mode=bo1|bo3|random      One match type. Default: every mode.
  --patch=<version>          One patch, as patches.json names it. Default: every patch.
  --claims                   Every day's practice claim counts instead, one { day, outcome, count }
                             per line. Not with --source, --mode or --patch.";

#[derive(Clone, Debug, PartialEq)]
pub struct ExportOptions {
    pub query: GameRecordQuery,
    pub out: PathBuf,
    /// #2884: the practice claim counts instead of the records.
    pub claims: bool,
}

pub fn parse_export_args(argv: &[String]) -> anyhow::Result<ExportOptions> {
    let mut query = GameRecordQuery {
        source: DEFAULT_CARD_STATS_FILTER.source,
        mode: None,
        patch: None,
    };
    let mut out: Option<PathBuf> = None;
    let mut claims = false;
    let mut filtered = false;

    for arg in argv {
        if arg == "--claims" {
            claims = true;
            continue;
        }
        let Some((name, raw)) = flag(arg) else {
            bail!("Unrecognized argument {}.\n\n{USAGE}", quoted(arg));
        };
        match name {
            "source" => {
                query.source = one_of::<SourceFilter>(name, raw, SOURCE_FILTERS, USAGE)?;
                filtered = true;
            }
            "mode" => {
                query.mode = Some(one_of::<GameMode>(name, raw, GAME_MODES, USAGE)?);
                filtered = true;
            }
            "patch" => {
                query.patch = Some(raw.to_owned());
                filtered = true;
            }
            "out" => out = Some(PathBuf::from(raw)),
            _ => bail!("Unrecognized option --{name}.\n\n{USAGE}"),
        }
    }
    if claims && filtered {
        bail!(
            "--claims writes every day's practice claim counts; drop --source, --mode and --patch.\n\n{USAGE}"
        );
    }

    let Some(out) = out else {
        bail!(
            "--out=<file> is required: the records are written to a file, never to the console.\n\n{USAGE}"
        );
    };
    Ok(ExportOptions { query, out, claims })
}

pub async fn read_records(db: &Db, query: &GameRecordQuery) -> Result<Vec<GameRecord>, StoreError> {
    let mut tx = db.begin(None).await?;
    let records = tx.game_records_list(query).await?;
    tx.commit().await?;
    Ok(records)
}

pub fn render_records(records: &[GameRecord]) -> anyhow::Result<String> {
    let mut text = String::new();
    for record in records {
        text.push_str(&jackioh_cards::versions::game_record_line(record));
        text.push('\n');
    }
    Ok(text)
}

/// #2884: every day's practice claim counts, oldest day first.
pub async fn read_claim_counts(db: &Db) -> Result<Vec<PracticeClaimCount>, StoreError> {
    let mut tx = db.begin(None).await?;
    let counts = tx.economy_practice_claim_counts().await?;
    tx.commit().await?;
    Ok(counts)
}

/// #2884: one `{ day, outcome, count }` per line.
pub fn render_claim_counts(counts: &[PracticeClaimCount]) -> anyhow::Result<String> {
    let mut text = String::new();
    for count in counts {
        text.push_str(&serde_json::to_string(count)?);
        text.push('\n');
    }
    Ok(text)
}

/// Writes the records, or with `claims` the claim counts, to `out`; answers how many lines.
pub async fn export_records(db: &Db, options: &ExportOptions) -> anyhow::Result<usize> {
    let (text, written) = if options.claims {
        let counts = read_claim_counts(db).await?;
        (render_claim_counts(&counts)?, counts.len())
    } else {
        let records = read_records(db, &options.query).await?;
        (render_records(&records)?, records.len())
    };
    tokio::fs::write(&options.out, text)
        .await
        .map_err(|error| anyhow!("{}: {error}", Path::new(&options.out).display()))?;
    Ok(written)
}

pub async fn run(args: Vec<String>) -> anyhow::Result<()> {
    let options = parse_export_args(&args)?;
    let connection_string = database_url()?;

    let db = connect_one(&connection_string).await?;
    let outcome = export_records(&db, &options).await;
    db.close().await;
    let written = outcome?;
    let what = if options.claims {
        "claim-count rows"
    } else {
        "records"
    };
    println!(
        "stats:export: wrote {written} {what} to {}",
        options.out.display()
    );
    Ok(())
}
