//! The server's command-line tools (SURFACE §11.1): `apps/server/src/db/{seed-catalog,mint-code,
//! seed-accounts,season-start,card-stats,import-dev-records}.ts`, part 20, and `export_records`, which
//! the TS never had (the records out, where `import_dev_records` reads them in), and the play
//! telemetry's `timing_backfill` and `timing_fit` (R1442), which it never had either. Each module's
//! `pub async fn run(args: Vec<String>) -> anyhow::Result<()>` is its TS `main()`, handed the
//! arguments after the subcommand (`main.rs`).
//! Written once by part 1 (SURFACE §1).

pub mod card_stats;
pub mod export_records;
pub mod import_dev_records;
pub mod mint_code;
pub mod season_start;
pub mod seed_accounts;
pub mod seed_catalog;
pub mod timing_backfill;
pub mod timing_fit;
