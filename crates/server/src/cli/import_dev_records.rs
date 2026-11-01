//! Admin script: loads an AI development run's game records into `public.game_records` (SPEC §9.11,
//! R378), so `stats-cards --source=dev --patch=<version>` reads the pre-release run, to compare with
//! the same patch's live games, which `stats-cards --patch=<version>` reads.
//!
//! ```text
//! jackioh-server stats-import <records.jsonl>
//! ```
//!
//! The file is what `cargo jackioh stats` writes: one GameRecord per line. Every line is read and
//! checked before anything is written, and a file holding a record that is not a development record,
//! one whose id does not begin `dev:`, or a forced-run record, is refused whole (R1284000 refuses the
//! forced file): a live record is the server's alone to write. A record already in the table is
//! skipped, never doubled.
//!
//! ```text
//! jackioh-server stats-import --totals-only <records.jsonl>
//! ```
//!
//! R1976000: the same checks, then the file's records are folded into the totals and no row of
//! `game_records` is written for them. The one row kept is the file's SHA-256 in `game_record_batches`,
//! so the same file imported twice is folded once. A record whose id `game_records` holds, or a repeat
//! of an id the file named before, is left out. The learn lane's import and the gather workflow use it.

use std::path::{Component, Path, PathBuf};

use anyhow::{anyhow, bail};
use indexmap::IndexMap;
use serde::Serialize;
use sha2::{Digest, Sha256};

use jackioh_engine::wire::{DEV_RECORD_ID_PREFIX, GameRecord, GameSource, forced_series};

use crate::cli::card_stats::literal;
use crate::cli::{connect_one, database_url};
use crate::config::STATS_FOLD_BATCH_RECORDS;
use crate::db::store::Db;

const USAGE: &str = "Usage: jackioh-server stats-import [--totals-only] <records.jsonl>";

/// R1976000: the flag that files a run's records as totals only.
const TOTALS_ONLY_FLAG: &str = "--totals-only";

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcome {
    pub read: usize,
    pub written: usize,
    pub skipped: usize,
}

/// R1976000: what `--totals-only` did with a file: its records read, those folded into the totals
/// and those left out, and its batch (the SHA-256 of its contents), held already when `again`, in
/// which case nothing was folded.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TotalsOutcome {
    pub read: usize,
    pub folded: usize,
    pub left_out: usize,
    pub batch: String,
    pub again: bool,
}

/// `path` itself when it is absolute, else `base` joined with it (and `base` itself made absolute
/// against `cwd` first), with `.` and `..` folded away.
fn resolve(cwd: &str, base: &str, path: &str) -> PathBuf {
    let mut joined = PathBuf::from(cwd);
    joined.push(base);
    joined.push(path);
    let mut out = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// The file a path names, read from the directory the command was started in, as `cargo jackioh
/// stats --out` writes it. INIT_CWD, the caller's directory under pnpm, is honored when set, and
/// otherwise the binary's own working directory is the caller's.
pub fn run_file_path(path: &str, env: &IndexMap<String, String>, cwd: &str) -> PathBuf {
    let base = env.get("INIT_CWD").map(String::as_str).unwrap_or(cwd);
    resolve(cwd, base, path)
}

fn dev_records(contents: &str) -> anyhow::Result<Vec<GameRecord>> {
    // R1371000: a file written before the version retcon names its versions the old way.
    let records =
        jackioh_cards::versions::read_game_record_lines(contents).map_err(|error| anyhow!("{error}"))?;
    if let Some(live) = records.iter().find(|record| record.source != GameSource::Dev) {
        bail!(
            "record {} is a {} record; only an AI development run's records (source \"dev\") can be imported (R378)",
            live.id,
            literal(&live.source)
        );
    }
    // A development id cannot take a live game's place: a match id never begins this way.
    if let Some(misnamed) = records
        .iter()
        .find(|record| !record.id.starts_with(DEV_RECORD_ID_PREFIX))
    {
        bail!(
            "record {} is a development record whose id does not begin \"{DEV_RECORD_ID_PREFIX}\" (R378)",
            misnamed.id
        );
    }

    // R1284000: a forced run's records are for `stats compare` and `stats report`, never for
    // `stats-import`: a file holding one is refused whole, so rigged games can never count as a
    // sample of All Random.
    if let Some(forced) = records.iter().find(|record| forced_series(record).is_some()) {
        // `forced_series` is `Some` here, so the series exists.
        let series = forced_series(forced).unwrap_or_default();
        bail!(
            "record {} is a forced-run record (series \"{series}\"); a forced run is for stats compare and stats report, never for stats-import (R1284000)",
            forced.id
        );
    }
    Ok(records)
}

/// The SHA-256 of a file's contents, in lowercase hex: its batch (R1976000).
pub fn batch_of(contents: &str) -> String {
    Sha256::digest(contents.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub async fn import_dev_records(db: &Db, contents: &str) -> anyhow::Result<ImportOutcome> {
    let records = dev_records(contents)?;

    // R1044000: `STATS_FOLD_BATCH_RECORDS` records to a transaction, the ones written folded into the
    // totals together, so a day's file rewrites each card's totals a few times rather than once per
    // game. A record already in the table answers false and is counted as skipped.
    let mut written = 0;
    for batch in records.chunks(STATS_FOLD_BATCH_RECORDS) {
        let mut tx = db.begin(None).await?;
        let inserted = tx.game_records_insert_batch(batch).await?;
        tx.commit().await?;
        written += inserted.into_iter().filter(|&wrote| wrote).count();
    }
    Ok(ImportOutcome {
        read: records.len(),
        written,
        skipped: records.len() - written,
    })
}

pub async fn import_dev_records_totals_only(db: &Db, contents: &str) -> anyhow::Result<TotalsOutcome> {
    let records = dev_records(contents)?;
    let batch = batch_of(contents);
    let mut tx = db.begin(None).await?;
    let done = tx.game_record_batch_fold(&batch, &records).await?;
    tx.commit().await?;
    Ok(match done {
        Some(done) => TotalsOutcome {
            read: records.len(),
            folded: done.folded,
            left_out: done.left_out,
            batch,
            again: false,
        },
        None => TotalsOutcome {
            read: records.len(),
            folded: 0,
            left_out: records.len(),
            batch,
            again: true,
        },
    })
}

pub async fn run(args: Vec<String>) -> anyhow::Result<()> {
    let totals_only = args.iter().any(|arg| arg == TOTALS_ONLY_FLAG);
    let mut paths = args.into_iter().filter(|arg| arg != TOTALS_ONLY_FLAG);
    let (Some(path), None) = (paths.next(), paths.next()) else {
        bail!(USAGE);
    };
    if path.starts_with("--") {
        bail!(USAGE);
    }
    let connection_string = database_url()?;

    let env: IndexMap<String, String> = std::env::vars().collect();
    let cwd = std::env::current_dir()?;
    let file = run_file_path(&path, &env, &cwd.to_string_lossy());
    let contents = tokio::fs::read_to_string(&file)
        .await
        .map_err(|error| anyhow!("{}: {error}", Path::new(&file).display()))?;
    let db = connect_one(&connection_string).await?;
    if totals_only {
        let outcome = import_dev_records_totals_only(&db, &contents).await;
        db.close().await;
        let outcome = outcome?;
        if outcome.again {
            println!(
                "stats:import: read {} records; this file (batch {}) was imported before, so nothing was folded",
                outcome.read, outcome.batch
            );
        } else {
            println!(
                "stats:import: read {} records, folded {} into the totals, left out {} already imported (batch {})",
                outcome.read, outcome.folded, outcome.left_out, outcome.batch
            );
        }
        return Ok(());
    }
    let outcome = import_dev_records(&db, &contents).await;
    db.close().await;
    let outcome = outcome?;
    println!(
        "stats:import: read {} records, wrote {}, skipped {} already imported",
        outcome.read, outcome.written, outcome.skipped
    );
    Ok(())
}
