//! Admin script: loads an AI development run's game records into `public.game_records` (SPEC §9.11,
//! R378; ← `apps/server/src/db/import-dev-records.ts`), so `stats-cards --source=dev
//! --patch=<version>` reads the pre-release run, to compare with the same patch's live games, which
//! `stats-cards --patch=<version>` reads.
//!
//! ```text
//! jackioh-server stats-import <records.jsonl>
//! ```
//!
//! The file is what `cargo jackioh stats` writes: one GameRecord per line, at a path read from the
//! directory the command was started in, as `--out`'s is. Every line is read and checked before
//! anything is written, and a file holding a record that is not a development record, or one whose
//! id does not begin `dev:`, is refused whole: a live record is the server's alone to write, at the
//! end of a match, so nothing typed into a file can ever count as live play or stand in a live
//! game's place. A record already in the table (the same run imported twice) is skipped, never
//! doubled.

use std::path::{Component, Path, PathBuf};

use anyhow::{anyhow, bail};
use indexmap::IndexMap;
use serde::Serialize;

use jackioh_engine::wire::{parse_game_record_lines, GameSource, DEV_RECORD_ID_PREFIX};

use crate::db::store::Db;

const USAGE: &str = "Usage: jackioh-server stats-import <records.jsonl>";

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcome {
    pub read: usize,
    pub written: usize,
    pub skipped: usize,
}

/// Node's `path.resolve(base, path)`: `path` itself when it is absolute, else `base` joined with
/// it (and `base` itself made absolute against `cwd` first), with `.` and `..` folded away.
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
/// stats --out` writes it. Under pnpm the TS script ran in apps/server and was passed the caller's
/// directory as INIT_CWD; that variable is still honoured when set, and otherwise the binary's own
/// working directory is the caller's.
pub fn run_file_path(path: &str, env: &IndexMap<String, String>, cwd: &str) -> PathBuf {
    let base = env.get("INIT_CWD").map(String::as_str).unwrap_or(cwd);
    resolve(cwd, base, path)
}

/// The literal a string-union value serialises as (`"live"`, `"dev"`), for the refusal's text.
fn literal<T: Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(text)) => text,
        _ => String::new(),
    }
}

/// R378: every record a development record, or nothing is written.
pub async fn import_dev_records(db: &Db, contents: &str) -> anyhow::Result<ImportOutcome> {
    let records = parse_game_record_lines(contents).map_err(|error| anyhow!("{error}"))?;
    if let Some(live) = records.iter().find(|record| record.source != GameSource::Dev) {
        bail!(
            "record {} is a {} record; only an AI development run's records (source \"dev\") can be imported (R378)",
            live.id,
            literal(&live.source)
        );
    }
    // A development id cannot take a live game's place: a match id never begins this way.
    if let Some(misnamed) = records.iter().find(|record| !record.id.starts_with(DEV_RECORD_ID_PREFIX)) {
        bail!(
            "record {} is a development record whose id does not begin \"{DEV_RECORD_ID_PREFIX}\" (R378)",
            misnamed.id
        );
    }

    // One transaction per record, as TS's one store call per record was: a record already in the
    // table answers false and is counted as skipped.
    let mut written = 0;
    for record in &records {
        let mut tx = db.begin(None).await?;
        let inserted = tx.game_records_insert(record).await?;
        tx.commit().await?;
        if inserted {
            written += 1;
        }
    }
    Ok(ImportOutcome { read: records.len(), written, skipped: records.len() - written })
}

/// TS `main()`: `stats-import`'s arguments after the subcommand (`main.rs`).
pub async fn run(args: Vec<String>) -> anyhow::Result<()> {
    let mut args = args.into_iter();
    let (Some(path), None) = (args.next(), args.next()) else {
        bail!(USAGE);
    };
    let connection_string = std::env::var("DATABASE_URL").unwrap_or_default();
    if connection_string.is_empty() {
        bail!("DATABASE_URL is not set (see docs/architecture.md, env-var contract).");
    }

    let env: IndexMap<String, String> = std::env::vars().collect();
    let cwd = std::env::current_dir()?;
    let file = run_file_path(&path, &env, &cwd.to_string_lossy());
    let contents = tokio::fs::read_to_string(&file)
        .await
        .map_err(|error| anyhow!("{}: {error}", Path::new(&file).display()))?;
    // One connection: this process runs its inserts one after another and exits.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&connection_string)
        .await
        .map_err(|error| anyhow!("{error}"))?;
    let db = Db::Pg(pool);
    let outcome = import_dev_records(&db, &contents).await;
    db.close().await;
    let outcome = outcome?;
    println!(
        "stats:import: read {} records, wrote {}, skipped {} already imported",
        outcome.read, outcome.written, outcome.skipped
    );
    Ok(())
}
