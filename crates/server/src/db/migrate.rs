//! Migration runner (SURFACE §11.3). Applies every file in
//! `crates/server/migrations` in lexical order over DATABASE_URL and records what it applied in
//! `app.migrations`. The files are compiled in with `include_str!`.

use anyhow::{anyhow, bail};
use sqlx::{Connection, PgConnection, Postgres, Transaction};

/// Every migration, `(filename, contents)`, in lexical order of the filename.
/// Embedded at compile time.
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
    (
        "0029_play_telemetry.sql",
        include_str!("../../migrations/0029_play_telemetry.sql"),
    ),
    (
        "0030_meditative_set.sql",
        include_str!("../../migrations/0030_meditative_set.sql"),
    ),
    (
        "0031_meditative_endings.sql",
        include_str!("../../migrations/0031_meditative_endings.sql"),
    ),
    (
        "0032_ladder_bots.sql",
        include_str!("../../migrations/0032_ladder_bots.sql"),
    ),
    (
        "0033_bot_trios.sql",
        include_str!("../../migrations/0033_bot_trios.sql"),
    ),
    (
        "0034_account_consent.sql",
        include_str!("../../migrations/0034_account_consent.sql"),
    ),
    (
        "0035_replay_final_hash.sql",
        include_str!("../../migrations/0035_replay_final_hash.sql"),
    ),
    (
        "0036_lean_set.sql",
        include_str!("../../migrations/0036_lean_set.sql"),
    ),
    (
        "0037_custom_cards.sql",
        include_str!("../../migrations/0037_custom_cards.sql"),
    ),
    (
        "0038_social.sql",
        include_str!("../../migrations/0038_social.sql"),
    ),
    ("0039_duos.sql", include_str!("../../migrations/0039_duos.sql")),
    (
        "0040_moderation_switch_grant.sql",
        include_str!("../../migrations/0040_moderation_switch_grant.sql"),
    ),
    (
        "0041_card_totals.sql",
        include_str!("../../migrations/0041_card_totals.sql"),
    ),
    (
        "0042_match_checkpoints.sql",
        include_str!("../../migrations/0042_match_checkpoints.sql"),
    ),
    (
        "0043_meditative_plus_set.sql",
        include_str!("../../migrations/0043_meditative_plus_set.sql"),
    ),
    (
        "0044_deck_cosmetics.sql",
        include_str!("../../migrations/0044_deck_cosmetics.sql"),
    ),
    (
        "0045_economy.sql",
        include_str!("../../migrations/0045_economy.sql"),
    ),
    (
        "0046_dev_accounts.sql",
        include_str!("../../migrations/0046_dev_accounts.sql"),
    ),
    (
        "0047_practice_seeds.sql",
        include_str!("../../migrations/0047_practice_seeds.sql"),
    ),
    (
        "0048_game_record_builds.sql",
        include_str!("../../migrations/0048_game_record_builds.sql"),
    ),
    (
        "0049_version_retcon.sql",
        include_str!("../../migrations/0049_version_retcon.sql"),
    ),
    (
        "0050_going_live.sql",
        include_str!("../../migrations/0050_going_live.sql"),
    ),
    (
        "0051_duos_seats_set_null.sql",
        include_str!("../../migrations/0051_duos_seats_set_null.sql"),
    ),
    (
        "0052_bot_rating_generation.sql",
        include_str!("../../migrations/0052_bot_rating_generation.sql"),
    ),
    (
        "0053_bot_grind.sql",
        include_str!("../../migrations/0053_bot_grind.sql"),
    ),
    (
        "0054_seat_epoch.sql",
        include_str!("../../migrations/0054_seat_epoch.sql"),
    ),
    (
        "0055_deck_size_25.sql",
        include_str!("../../migrations/0055_deck_size_25.sql"),
    ),
    (
        "0056_foreign_key_indexes.sql",
        include_str!("../../migrations/0056_foreign_key_indexes.sql"),
    ),
    (
        "0057_portrait_cosmetics.sql",
        include_str!("../../migrations/0057_portrait_cosmetics.sql"),
    ),
    (
        "0058_terminus_quest_otherworldly_other_origins_jelly_bean_tags.sql",
        include_str!("../../migrations/0058_terminus_quest_otherworldly_other_origins_jelly_bean_tags.sql"),
    ),
    (
        "0059_voted_out_and_fled.sql",
        include_str!("../../migrations/0059_voted_out_and_fled.sql"),
    ),
    (
        "0060_game_record_batches.sql",
        include_str!("../../migrations/0060_game_record_batches.sql"),
    ),
    (
        "0061_cosmetic_loadouts.sql",
        include_str!("../../migrations/0061_cosmetic_loadouts.sql"),
    ),
    (
        "0062_queue_readout_indexes.sql",
        include_str!("../../migrations/0062_queue_readout_indexes.sql"),
    ),
    (
        "0063_game_totals.sql",
        include_str!("../../migrations/0063_game_totals.sql"),
    ),
    (
        "0064_season_ranks_ladder_index.sql",
        include_str!("../../migrations/0064_season_ranks_ladder_index.sql"),
    ),
    (
        "0065_stinkyscape_set.sql",
        include_str!("../../migrations/0065_stinkyscape_set.sql"),
    ),
    (
        "0066_deck_versions.sql",
        include_str!("../../migrations/0066_deck_versions.sql"),
    ),
    (
        "0067_idempotency_keys.sql",
        include_str!("../../migrations/0067_idempotency_keys.sql"),
    ),
];

/// Advisory lock id taken with `pg_advisory_xact_lock` inside each transaction.
/// Transaction-scoped so behind a connection pooler the lock cannot leak across deploys.
pub const LOCK_ID: i64 = 0x6a61636b; // "jack"

/// The ledger, created under the lock before any file runs.
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

/// Files rewritten after some database had applied them, each with checksums of earlier versions.
/// 0013_retention_purge.sql: first version used a superuser-only clause that failed on Supabase.
pub const REWRITTEN: &[(&str, &[&str])] = &[("0013_retention_purge.sql", &["16b93e4d"])];

/// Earlier checksums a database may hold for `filename`.
pub fn rewritten(filename: &str) -> &'static [&'static str] {
    REWRITTEN
        .iter()
        .find(|(name, _)| *name == filename)
        .map(|(_, earlier)| *earlier)
        .unwrap_or(&[])
}

/// FNV-1a checksum over UTF-16 code units (SURFACE §5.2).
pub fn checksum(text: &str) -> String {
    jackioh_engine::replay::fnv1a32_utf16(text)
}

/// The `.sql` files, sorted by name.
fn list_migrations() -> Vec<(&'static str, &'static str)> {
    let mut entries: Vec<(&'static str, &'static str)> = MIGRATIONS
        .iter()
        .copied()
        .filter(|(name, _)| name.ends_with(".sql"))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    entries
}

/// Strips sqlx's database error prefix; any other error as it displays.
fn error_message(error: &sqlx::Error) -> String {
    match error {
        sqlx::Error::Database(db) => db.message().to_string(),
        other => other.to_string(),
    }
}

/// One transaction under the advisory lock.
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
    let _ = client.close().await;
    result?;

    Ok(applied)
}

/// Applies the ledger, then one locked transaction per file.
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

        // One transaction per file: a migration either lands whole or not at all.
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

/// CLI entry point for `jackioh-server migrate`.
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
