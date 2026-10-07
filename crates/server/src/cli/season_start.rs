//! Admin script: opens the build's season (R609) — the same path the server takes at boot
//! (← `apps/server/src/db/season-start.ts`).
//!
//! A season is named by the minor version of the newest patch (R375), so the first deploy of a
//! new minor version starts one. Opening it is what runs R609's soft reset: every rated player's
//! rating pulled partway to the mean and the deviation widened, in the same transaction as the
//! season row — a season is either open and reset or neither.
//!
//! Run it ahead of such a deploy, against a copy of the live database first (R609):
//!
//! ```text
//! jackioh-server season-start --dry-run
//! ```
//!
//! `--dry-run` runs the identical path — open the season, reset every rated player — and rolls
//! the transaction back, printing the report the real run would write. Without it the writes
//! commit: the database now holds the season the next build's boot will find already open.
//!
//! The script never touches a season that is already open: `open_season_in_tx` answers
//! `opened: false` and writes nothing, so the script is safe to re-run.

use std::time::Duration;

use anyhow::{anyhow, bail};
use indexmap::IndexMap;
use serde_json::json;

use crate::api::ranked::{OpenedSeason, SeasonDeps, open_season_in_tx};
use crate::db::pg::assert_postgres_url;
use crate::db::store::Db;

/// The one flag (TS `SeasonStartOptions`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct SeasonStartOptions {
    pub dry_run: bool,
}

/// The one flag. Anything else is a usage error, named in the refusal.
pub fn parse_season_start_args(argv: &[String]) -> anyhow::Result<SeasonStartOptions> {
    let mut dry_run = false;
    for arg in argv {
        if arg == "--dry-run" {
            dry_run = true;
        } else {
            bail!(
                "usage: db:season-start [--dry-run] — unknown argument {}",
                serde_json::to_string(arg).unwrap_or_else(|_| format!("\"{arg}\""))
            );
        }
    }
    Ok(SeasonStartOptions { dry_run })
}

/// R609's season open, through the same `open_season_in_tx` the boot path's `open_season` calls. On
/// a dry run the report is read out of the transaction and then the transaction is thrown away, so
/// the answer describes exactly what a real run would have done and nothing it did survives.
///
/// TS `openSeason({ ...deps, store })` is `store.tx((t) => openSeasonInTx(t, deps))`; that is what
/// the real run does here, on the `Db` it was handed. TS's dry run threw a private symbol out of the
/// transaction to roll it back; in Rust the transaction is dropped without `commit`, which rolls a
/// Postgres transaction back and restores a fake's snapshot (`db::fake::FakeTx`).
pub async fn start_season(
    store: &Db,
    deps: &SeasonDeps,
    options: SeasonStartOptions,
) -> anyhow::Result<OpenedSeason> {
    let mut t = store.begin(None).await.map_err(|e| anyhow!("{e}"))?;
    let opened = open_season_in_tx(&mut t, deps)
        .await
        .map_err(|e| anyhow!("{e}"))?;
    if options.dry_run {
        // ROLL_BACK: nothing this transaction wrote survives it.
        drop(t);
    } else {
        t.commit().await.map_err(|e| anyhow!("{e}"))?;
    }
    Ok(opened)
}

/// TS `createPostgresStore({ connectionString })` for a one-shot script: a small pool with the same
/// checkout and idle timeouts (`store.ts`), wrapped as the `Db` every store call takes.
async fn connect(connection_string: &str) -> anyhow::Result<Db> {
    assert_postgres_url(connection_string)?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .idle_timeout(Duration::from_secs(10))
        .acquire_timeout(Duration::from_secs(10))
        .connect(connection_string)
        .await?;
    Ok(Db::Pg(pool))
}

/// TS `main()` (`pnpm db:season-start`; `jackioh-server season-start [--dry-run]`).
pub async fn run(args: Vec<String>) -> anyhow::Result<()> {
    let options = parse_season_start_args(&args)?;
    let source: IndexMap<String, String> = std::env::vars().collect();
    let env = crate::env::load_env(&source).map_err(|e| anyhow!("{e}"))?;
    let store = connect(&env.database_url).await?;

    // TS `loadPatchVersion()`: the newest patch's version, which the binary carries compiled in
    // (SURFACE §11.3); the clock and the log are the server's own (`tokio::time`, `tracing`).
    let deps = SeasonDeps {
        patch_version: jackioh_cards::catalog_version().to_string(),
    };
    let result = start_season(&store, &deps, options).await;

    // TS `finally { await store.close() }`.
    if let Db::Pg(pool) = &store {
        pool.close().await;
    }
    let opened = result?;

    println!(
        "{}",
        json!({
            "season": opened.season.id,
            "patchVersion": opened.season.patch_version,
            "opened": opened.opened,
            "dryRun": options.dry_run,
            "reset": serde_json::to_value(&opened.reset)?,
        })
    );
    let outcome = if options.dry_run {
        "dry run — every write above was rolled back".to_string()
    } else if opened.opened {
        format!("season {} opened", opened.season.id)
    } else {
        format!("season {} was already open; nothing written", opened.season.id)
    };
    eprintln!("season-start: {outcome}");
    Ok(())
}
