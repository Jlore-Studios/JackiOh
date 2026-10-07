//! Migrations are append-only (`src/db/migrate.rs`): a database refuses to start the server when a
//! file it already applied has changed, and Render runs the migration on every deploy. #325's
//! repo-wide rename of "Plague Tokens" edited a comment in 0020_plague_tag.sql, and every deploy
//! from 97a00bd6 on exited at `db:migrate` ("0020_plague_tag.sql was already applied but its
//! contents changed (1384d000 -> 621182a3)"). CI never saw it, since it migrates a fresh database.
//! So every migration's checksum is pinned here: an edit to one fails this test, and a new
//! migration adds its line.
//!
//! (← `apps/server/test/db/migrations-pinned.test.ts`.) The Rust runner embeds the files with
//! `include_str!` (SURFACE §11.3), so one more check rides along: every `.sql` file in the
//! directory is one the runner embeds, under its own name and with its bytes.

use std::path::PathBuf;

use jackioh_server::db::migrate::{MIGRATIONS, REWRITTEN, checksum};

/// The directory part 1 copied byte for byte from `apps/server/src/db/migrations/`.
fn migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations")
}

/// Each migration's checksum (`checksum` in migrate.rs) as every database applied it.
const PINNED: &[(&str, &str)] = &[
    ("0001_profiles_and_invites.sql", "f77a3628"),
    ("0002_collection.sql", "344a1522"),
    ("0003_loadouts.sql", "4dde1c62"),
    ("0004_matches.sql", "b81c43e3"),
    ("0005_service_role_reads_auth_users.sql", "23466ec9"),
    ("0006_redeem_ip_lock.sql", "8b367b28"),
    ("0007_decks_and_trios.sql", "0f080973"),
    ("0008_queue_modes.sql", "ba410c60"),
    ("0009_series.sql", "c138ed05"),
    ("0010_jlockeed_tag.sql", "d4151fa8"),
    ("0011_tutorial_progress.sql", "2539a55f"),
    ("0012_account_deletion.sql", "1c23a243"),
    ("0013_retention_purge.sql", "246be4c6"),
    ("0014_game_records.sql", "3926edb3"),
    ("0015_classic_sets_tags.sql", "edd4789e"),
    ("0016_catalog_growth_grants.sql", "9b79ecdf"),
    ("0017_last_boards.sql", "0a2aab09"),
    ("0018_player_settings.sql", "396091bc"),
    ("0019_hero_portraits.sql", "79687fa8"),
    ("0020_plague_tag.sql", "1384d000"),
    ("0021_player_stats.sql", "4790815c"),
    ("0022_ranked_ladder.sql", "78ccc5b5"),
    ("0023_rematch.sql", "6e568bc0"),
    ("0024_glitch_boards.sql", "c82b008a"),
    ("0025_patch_retcon.sql", "203f79a3"),
    ("0026_catalyst_prime_acclaimed_tags.sql", "1d0838c1"),
];

/// `PINNED[name]`.
fn pinned(name: &str) -> Option<&'static str> {
    PINNED.iter().find(|(file, _)| *file == name).map(|(_, sum)| *sum)
}

/// The `.sql` files in the directory, sorted (TS `readdirSync(…).filter(…).sort()`).
fn migrations() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(migrations_dir())
        .expect("crates/server/migrations is readable")
        .map(|entry| {
            entry
                .expect("a directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| name.ends_with(".sql"))
        .collect();
    names.sort();
    names
}

/// The text of one migration file, read the way TS read it (`readFileSync(…, "utf8")`).
fn read(name: &str) -> String {
    std::fs::read_to_string(migrations_dir().join(name)).expect("a migration file is readable UTF-8")
}

mod migrations_are_append_only {
    use super::*;

    #[test]
    fn every_migration_is_pinned_and_a_new_one_adds_its_checksum_here() {
        let unpinned: Vec<String> = migrations()
            .into_iter()
            .filter(|name| pinned(name).is_none())
            .map(|name| format!("(\"{name}\", \"{}\"),", checksum(&read(&name))))
            .collect();
        assert_eq!(
            unpinned,
            Vec::<String>::new(),
            "add each new migration's line to PINNED"
        );
    }

    #[test]
    fn no_migration_a_database_applied_has_changed() {
        let changed: Vec<String> = migrations()
            .into_iter()
            .map(|name| {
                let now = checksum(&read(&name));
                (name, now)
            })
            .filter(|(name, now)| pinned(name).is_some_and(|sum| sum != now))
            .map(|(name, now)| format!("{name}: {} -> {now}", pinned(&name).unwrap_or_default()))
            .collect();
        assert_eq!(
            changed,
            Vec::<String>::new(),
            "add a new migration instead of editing an applied one"
        );
    }

    #[test]
    fn a_rewrite_migrate_allows_is_pinned_at_its_current_version() {
        for (name, earlier) in REWRITTEN {
            let current = pinned(name);
            assert!(current.is_some(), "{name} is in REWRITTEN but not pinned");
            assert!(
                !earlier.contains(&current.unwrap_or_default()),
                "{name}: REWRITTEN lists its current checksum as an earlier one"
            );
        }
    }

    #[test]
    fn the_runner_embeds_every_migration_in_the_directory_byte_for_byte() {
        let embedded: Vec<&str> = MIGRATIONS.iter().map(|(name, _)| *name).collect();
        let on_disk = migrations();
        assert_eq!(
            embedded,
            on_disk.iter().map(String::as_str).collect::<Vec<&str>>(),
            "MIGRATIONS in src/db/migrate.rs must name every file in crates/server/migrations, in order"
        );
        for (name, text) in MIGRATIONS {
            assert_eq!(*text, read(name), "{name}: the embedded text is not the file's");
            assert_eq!(
                Some(checksum(text).as_str()),
                pinned(name),
                "{name}: the embedded text's checksum"
            );
        }
    }
}
