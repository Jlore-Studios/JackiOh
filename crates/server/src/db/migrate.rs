//! Migration runner (← `apps/server/src/db/migrate.ts`, SURFACE §11.3). Applies every file in
//! `crates/server/migrations` in lexical order over DATABASE_URL and records what it applied in
//! `app.migrations`, so bring-up is one command against a fresh Supabase project
//! (docs/architecture.md, step 4 of the bring-up checklist).
//!
//! Why not the Supabase CLI: `supabase db push` only reads `supabase/migrations/<timestamp>_<name>.sql`,
//! and the canonical SQL lives at `crates/server/migrations/` (copied byte for byte from
//! `apps/server/src/db/migrations/` by part 1). The SQL is plain Postgres, so either path works; this
//! runner is the one the checklist uses because it needs nothing but a connection string. See
//! docs/architecture.md for the CLI variant.
//!
//! The files are compiled into the binary with `include_str!` (SURFACE §11.3), so the Docker image
//! needs no migrations directory at run time. `tests/store/migrations_pinned.rs` proves that every
//! `.sql` file in the directory is in [`MIGRATIONS`] and that none of them changed.

use anyhow::{anyhow, bail};
use sqlx::{Connection, PgConnection, Postgres, Transaction};

/// Every migration, `(filename, contents)`, in lexical order of the filename: the directory listing
/// TS read at run time (`listMigrations`), embedded at compile time. A new migration adds its line
/// here and its checksum to `tests/store/migrations_pinned.rs`; that test fails on a file in the
/// directory that this list does not name.
pub const MIGRATIONS: &[(&str, &str)] = &[
    (
        "0001_profiles_and_invites.sql",
        include_str!("../../migrations/0001_profiles_and_invites.sql"),
    ),
    (
        "0002_collection.sql",
        include_str!("../../migrations/0002_collection.sql"),
    ),
    (
        "0003_loadouts.sql",
        include_str!("../../migrations/0003_loadouts.sql"),
    ),
    (
        "0004_matches.sql",
        include_str!("../../migrations/0004_matches.sql"),
    ),
    (
        "0005_service_role_reads_auth_users.sql",
        include_str!("../../migrations/0005_service_role_reads_auth_users.sql"),
    ),
    (
        "0006_redeem_ip_lock.sql",
        include_str!("../../migrations/0006_redeem_ip_lock.sql"),
    ),
    (
        "0007_decks_and_trios.sql",
        include_str!("../../migrations/0007_decks_and_trios.sql"),
    ),
    (
        "0008_queue_modes.sql",
        include_str!("../../migrations/0008_queue_modes.sql"),
    ),
    (
        "0009_series.sql",
        include_str!("../../migrations/0009_series.sql"),
    ),
    (
        "0010_jlockeed_tag.sql",
        include_str!("../../migrations/0010_jlockeed_tag.sql"),
    ),
    (
        "0011_tutorial_progress.sql",
        include_str!("../../migrations/0011_tutorial_progress.sql"),
    ),
    (
        "0012_account_deletion.sql",
        include_str!("../../migrations/0012_account_deletion.sql"),
    ),
    (
        "0013_retention_purge.sql",
        include_str!("../../migrations/0013_retention_purge.sql"),
    ),
    (
        "0014_game_records.sql",
        include_str!("../../migrations/0014_game_records.sql"),
    ),
    (
        "0015_classic_sets_tags.sql",
        include_str!("../../migrations/0015_classic_sets_tags.sql"),
    ),
    (
        "0016_catalog_growth_grants.sql",
        include_str!("../../migrations/0016_catalog_growth_grants.sql"),
    ),
    (
        "0017_last_boards.sql",
        include_str!("../../migrations/0017_last_boards.sql"),
    ),
    (
        "0018_player_settings.sql",
        include_str!("../../migrations/0018_player_settings.sql"),
    ),
    (
        "0019_hero_portraits.sql",
        include_str!("../../migrations/0019_hero_portraits.sql"),
    ),
    (
        "0020_plague_tag.sql",
        include_str!("../../migrations/0020_plague_tag.sql"),
    ),
    (
        "0021_player_stats.sql",
        include_str!("../../migrations/0021_player_stats.sql"),
    ),
    (
        "0022_ranked_ladder.sql",
        include_str!("../../migrations/0022_ranked_ladder.sql"),
    ),
    (
        "0023_rematch.sql",
        include_str!("../../migrations/0023_rematch.sql"),
    ),
    (
        "0024_glitch_boards.sql",
        include_str!("../../migrations/0024_glitch_boards.sql"),
    ),
    (
        "0025_patch_retcon.sql",
        include_str!("../../migrations/0025_patch_retcon.sql"),
    ),
    (
        "0026_catalyst_prime_acclaimed_tags.sql",
        include_str!("../../migrations/0026_catalyst_prime_acclaimed_tags.sql"),
    ),
    (
        "0027_lean_newest.sql",
        include_str!("../../migrations/0027_lean_newest.sql"),
    ),
    (
        "0028_usernames.sql",
        include_str!("../../migrations/0028_usernames.sql"),
    ),
];

/// One advisory lock id for the whole runner, so two deploys cannot interleave migrations. It is
/// taken with `pg_advisory_xact_lock` inside each transaction, never as a session lock: Render's
/// release runs this runner on every deploy (render.yaml), over whatever `DATABASE_URL` the
/// server uses, and behind a transaction-mode pooler a session lock stays held on whichever pooled
/// backend took it, so the next deploy's lock could wait forever and the server never boot.
pub const LOCK_ID: i64 = 0x6a61636b; // "jack"

/// The ledger, created (if missing) under the lock before any file runs. Its table comment names
/// the Rust paths; `comment on` re-runs every time, so an older database's comment is brought up to
/// date and nothing else about the ledger changes.
pub const LEDGER: &str = "
create schema if not exists app;
create table if not exists app.migrations (
  filename   text primary key,
  applied_at timestamptz not null default now(),
  checksum   text not null
);
comment on table app.migrations is
  'Which files in crates/server/migrations have been applied. Written by crates/server/src/db/migrate.rs.';
";

/// Files rewritten after some database had applied them, each with the checksums of its earlier
/// versions. A database that applied an earlier version keeps it (the ledger is not rewritten); any
/// other edit to an applied file is still refused. Add an entry only when the earlier version can
/// stay where it was applied and the new one is what every database that has not applied it needs.
///
/// 0013_retention_purge.sql: its first version gave app.purge_expired_rows a `set
/// jackioh.retention_purge` clause, which only a superuser may create since Postgres 15, so it failed
/// on Supabase, whose migrating role is not one. Where a superuser applied it, it works as written.
pub const REWRITTEN: &[(&str, &[&str])] = &[("0013_retention_purge.sql", &["16b93e4d"])];

/// `REWRITTEN[filename] ?? []`: the earlier checksums a database may hold for `filename`.
pub fn rewritten(filename: &str) -> &'static [&'static str] {
    REWRITTEN
        .iter()
        .find(|(name, _)| *name == filename)
        .map(|(_, earlier)| *earlier)
        .unwrap_or(&[])
}

/// FNV-1a, so a changed file that was already applied is reported instead of silently skipped.
/// Over the UTF-16 code units of the text (TS `charCodeAt`), as 8 lower-case hex digits: the same
/// function as the state hash's (SURFACE §5.2), so every checksum an existing ledger holds matches.
pub fn checksum(text: &str) -> String {
    jackioh_engine::replay::fnv1a32_utf16(text)
}

/// TS `listMigrations()`: the `.sql` files, sorted by name (`a < b`, UTF-16 order; every name is
/// ASCII, so `str::cmp` equals it). The list is embedded, so this only filters and sorts it.
fn list_migrations() -> Vec<(&'static str, &'static str)> {
    let mut entries: Vec<(&'static str, &'static str)> = MIGRATIONS
        .iter()
        .copied()
        .filter(|(name, _)| name.ends_with(".sql"))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    entries
}

/// The message of a database error as `pg` reported it (`error.message`), without sqlx's
/// "error returned from database: " prefix; any other error as it displays.
fn error_message(error: &sqlx::Error) -> String {
    match error {
        sqlx::Error::Database(db) => db.message().to_string(),
        other => other.to_string(),
    }
}

/// TS `locked`'s opening half: one transaction under the lock. The caller commits it; a `?` that
/// leaves before the commit drops the transaction, and sqlx rolls a dropped transaction back
/// (TS: `rollback().catch(() => undefined)` then rethrow).
async fn locked(client: &mut PgConnection) -> anyhow::Result<Transaction<'_, Postgres>> {
    let mut tx = client.begin().await.map_err(|e| anyhow!(error_message(&e)))?;
    sqlx::query("select pg_advisory_xact_lock($1)")
        .bind(LOCK_ID)
        .execute(&mut *tx)
        .await
        .map_err(|e| anyhow!(error_message(&e)))?;
    Ok(tx)
}

/// Apply every pending migration over `connection_string`; answers the files it applied, in order.
pub async fn migrate(connection_string: &str) -> anyhow::Result<Vec<String>> {
    let mut client = PgConnection::connect(connection_string)
        .await
        .map_err(|e| anyhow!(error_message(&e)))?;
    let mut applied: Vec<String> = Vec::new();

    let result = run_migrations(&mut client, &mut applied).await;
    // TS `finally { await client.end() }`: the connection closes whether or not a file failed.
    let _ = client.close().await;
    result?;

    Ok(applied)
}

/// The body of TS `migrate`'s `try`: the ledger, then one locked transaction per file.
async fn run_migrations(client: &mut PgConnection, applied: &mut Vec<String>) -> anyhow::Result<()> {
    {
        let mut tx = locked(client).await?;
        sqlx::raw_sql(LEDGER)
            .execute(&mut *tx)
            .await
            .map_err(|e| anyhow!(error_message(&e)))?;
        tx.commit().await.map_err(|e| anyhow!(error_message(&e)))?;
    }

    for (filename, sql) in list_migrations() {
        let sum = checksum(sql);

        // One transaction per file: a migration either lands whole or not at all. The ledger is
        // read under the lock, so a runner that waited on another sees what that one applied.
        let mut tx = locked(client).await?;
        let previous: Option<String> =
            sqlx::query_scalar("select checksum from app.migrations where filename = $1")
                .bind(filename)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| anyhow!(error_message(&e)))?;
        if let Some(previous) = previous {
            if previous != sum && !rewritten(filename).contains(&previous.as_str()) {
                bail!(
                    "{filename} was already applied but its contents changed ({previous} -> {sum}). \
                     Migrations are append-only: add a new file instead of editing this one."
                );
            }
            tx.commit().await.map_err(|e| anyhow!(error_message(&e)))?;
            continue;
        }
        if let Err(error) = sqlx::raw_sql(sql).execute(&mut *tx).await {
            bail!("{filename} failed: {}", error_message(&error));
        }
        sqlx::query("insert into app.migrations (filename, checksum) values ($1, $2)")
            .bind(filename)
            .bind(&sum)
            .execute(&mut *tx)
            .await
            .map_err(|e| anyhow!(error_message(&e)))?;
        tx.commit().await.map_err(|e| anyhow!(error_message(&e)))?;

        applied.push(filename.to_string());
    }
    Ok(())
}

/// TS `main()` (`pnpm db:migrate`; `jackioh-server migrate`, and the first step of `release`).
/// The arguments are unused, as TS read none.
pub async fn run(_args: Vec<String>) -> anyhow::Result<()> {
    let connection_string = std::env::var("DATABASE_URL").unwrap_or_default();
    if connection_string.is_empty() {
        bail!(
            "DATABASE_URL is not set. Supabase dashboard -> Project Settings -> Database -> \
             Connection string -> URI (or the local value printed by `supabase start`)."
        );
    }

    let applied = migrate(&connection_string).await?;
    if applied.is_empty() {
        println!("migrate: nothing to do, the database is up to date");
        return Ok(());
    }
    for filename in &applied {
        println!("migrate: applied {filename}");
    }
    Ok(())
}
