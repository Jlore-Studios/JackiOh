// Migrations are append-only (src/db/migrate.ts): a database refuses to start the server when a
// file it already applied has changed, and Render runs `db:migrate` on every deploy. #325's
// repo-wide rename of "Plague Tokens" edited a comment in 0020_plague_tag.sql, and every deploy
// from 97a00bd6 on exited at `db:migrate` ("0020_plague_tag.sql was already applied but its
// contents changed (1384d000 -> 621182a3)"). CI never saw it, since it migrates a fresh database.
// So every migration's checksum is pinned here: an edit to one fails this test, and a new
// migration adds its line.

import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

import { REWRITTEN, checksum } from "../../src/db/migrate";

const MIGRATIONS_DIR = join(import.meta.dirname, "../../src/db/migrations");

/** Each migration's checksum (`checksum` in migrate.ts) as every database applied it. */
const PINNED: Readonly<Record<string, string>> = {
  "0001_profiles_and_invites.sql": "f77a3628",
  "0002_collection.sql": "344a1522",
  "0003_loadouts.sql": "4dde1c62",
  "0004_matches.sql": "b81c43e3",
  "0005_service_role_reads_auth_users.sql": "23466ec9",
  "0006_redeem_ip_lock.sql": "8b367b28",
  "0007_decks_and_trios.sql": "0f080973",
  "0008_queue_modes.sql": "ba410c60",
  "0009_series.sql": "c138ed05",
  "0010_jlockeed_tag.sql": "d4151fa8",
  "0011_tutorial_progress.sql": "2539a55f",
  "0012_account_deletion.sql": "1c23a243",
  "0013_retention_purge.sql": "246be4c6",
  "0014_game_records.sql": "3926edb3",
  "0015_classic_sets_tags.sql": "edd4789e",
  "0016_catalog_growth_grants.sql": "9b79ecdf",
  "0017_last_boards.sql": "0a2aab09",
  "0018_player_settings.sql": "396091bc",
  "0019_hero_portraits.sql": "79687fa8",
  "0020_plague_tag.sql": "1384d000",
  "0021_player_stats.sql": "4790815c",
  "0022_ranked_ladder.sql": "78ccc5b5",
  "0023_rematch.sql": "6e568bc0",
  "0024_glitch_boards.sql": "c82b008a",
};

function migrations(): string[] {
  return readdirSync(MIGRATIONS_DIR)
    .filter((name) => name.endsWith(".sql"))
    .sort();
}

describe("migrations are append-only", () => {
  it("every migration is pinned, and a new one adds its checksum here", () => {
    const unpinned = migrations()
      .filter((name) => PINNED[name] === undefined)
      .map((name) => `"${name}": "${checksum(readFileSync(join(MIGRATIONS_DIR, name), "utf8"))}",`);
    expect(unpinned, "add each new migration's line to PINNED").toEqual([]);
  });

  it("no migration a database applied has changed", () => {
    const changed = migrations()
      .map((name) => ({ name, now: checksum(readFileSync(join(MIGRATIONS_DIR, name), "utf8")) }))
      .filter(({ name, now }) => PINNED[name] !== undefined && PINNED[name] !== now)
      .map(({ name, now }) => `${name}: ${PINNED[name]} -> ${now}`);
    expect(changed, "add a new migration instead of editing an applied one").toEqual([]);
  });

  it("a rewrite migrate.ts allows is pinned at its current version", () => {
    for (const [name, earlier] of Object.entries(REWRITTEN)) {
      expect(PINNED[name]).toBeDefined();
      expect(earlier).not.toContain(PINNED[name]);
    }
  });
});
