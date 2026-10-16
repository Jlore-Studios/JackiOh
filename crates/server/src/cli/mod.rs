//! The server's command-line tools (SURFACE §11.1): `apps/server/src/db/{seed-catalog,mint-code,
//! seed-accounts,season-start,card-stats,import-dev-records}.ts`, part 20, and `export_records`, which
//! the TS never had (the records out, where `import_dev_records` reads them in), and the play
//! telemetry's `timing_backfill` and `timing_fit` (R1442), §9.8's `invite_clusters` and the ladder bots'
//! `bot_calibrate` (R748000), which it never had either. Each module's
//! `pub async fn run(args: Vec<String>) -> anyhow::Result<()>` is its TS `main()`, handed the
//! arguments after the subcommand (`main.rs`).
//! Written once by part 1 (SURFACE §1).

pub mod balance_watch;
pub mod bot_calibrate;
pub mod card_stats;
pub mod export_records;
pub mod import_dev_records;
pub mod invite_clusters;
pub mod mint_code;
pub mod moderate;
pub mod season_start;
pub mod seed_accounts;
pub mod seed_catalog;
pub mod stats_fold;
pub mod timing_backfill;
pub mod timing_fit;

use anyhow::{anyhow, bail};

use crate::db::store::Db;

/// The store's connection string, `DATABASE_URL`, which every command that reads or writes the
/// store takes from the environment.
pub fn database_url() -> anyhow::Result<String> {
    let connection_string = std::env::var("DATABASE_URL").unwrap_or_default();
    if connection_string.is_empty() {
        bail!("DATABASE_URL is not set (see docs/architecture.md, env-var contract).");
    }
    Ok(connection_string)
}

/// One connection, wrapped as the `Db` every store call takes: a command runs its queries one
/// after another and exits.
pub async fn connect_one(connection_string: &str) -> anyhow::Result<Db> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(connection_string)
        .await
        .map_err(|error| anyhow!("{error}"))?;
    Ok(Db::Pg(pool))
}
