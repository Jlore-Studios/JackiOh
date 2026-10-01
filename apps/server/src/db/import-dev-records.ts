// Admin script: loads an AI development run's game records into `public.game_records` (SPEC §9.11,
// R378), so `stats:cards --source=dev --patch=<version>` reads the pre-release run, to compare with
// the same patch's live games, which `stats:cards --patch=<version>` reads.
//
//   pnpm --filter @jackioh/server stats:import <records.jsonl>
//
// The file is what `pnpm ai:stats` writes: one GameRecord per line, at a path read from the directory
// the command was started in, as `--out`'s is. Every line is read and checked
// before anything is written, and a file holding a record that is not a development record, or one
// whose id does not begin `dev:`, is refused whole: a live record is the server's alone to write, at
// the end of a match, so nothing typed into a file can ever count as live play or stand in a live
// game's place. A record already in the table (the same run imported twice) is skipped, never
// doubled.

import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

import { DEV_RECORD_ID_PREFIX, parseGameRecordLines } from "@jackioh/shared";

import type { Store } from "../api/ports";
import { createPostgresStore } from "./store";

const USAGE = "Usage: pnpm --filter @jackioh/server stats:import <records.jsonl>";

export type ImportOutcome = { read: number; written: number; skipped: number };

/**
 * The file a path names, read from the directory the command was started in, as `pnpm ai:stats
 * --out` writes it: pnpm runs this script in apps/server and passes the caller's directory as
 * INIT_CWD.
 */
export function runFilePath(path: string, env: Readonly<Record<string, string | undefined>>, cwd: string): string {
  return resolve(env["INIT_CWD"] ?? cwd, path);
}

/** R378: every record a development record, or nothing is written. */
export async function importDevRecords(store: Store, contents: string): Promise<ImportOutcome> {
  const records = parseGameRecordLines(contents);
  const live = records.find((record) => record.source !== "dev");
  if (live !== undefined) {
    throw new Error(
      `record ${live.id} is a ${live.source} record; only an AI development run's records (source "dev") can be imported (R378)`,
    );
  }
  // A development id cannot take a live game's place: a match id never begins this way.
  const misnamed = records.find((record) => !record.id.startsWith(DEV_RECORD_ID_PREFIX));
  if (misnamed !== undefined) {
    throw new Error(`record ${misnamed.id} is a development record whose id does not begin "${DEV_RECORD_ID_PREFIX}" (R378)`);
  }

  let written = 0;
  for (const record of records) {
    if (await store.gameRecords.insert(record)) written += 1;
  }
  return { read: records.length, written, skipped: records.length - written };
}

async function main(): Promise<void> {
  const [path, ...rest] = process.argv.slice(2);
  if (path === undefined || rest.length > 0) throw new Error(USAGE);
  const connectionString = process.env["DATABASE_URL"];
  if (connectionString === undefined || connectionString === "") {
    throw new Error("DATABASE_URL is not set (see docs/architecture.md, env-var contract).");
  }

  const contents = await readFile(runFilePath(path, process.env, process.cwd()), "utf8");
  // One connection: this process runs its inserts one after another and exits.
  const store = createPostgresStore({ connectionString, max: 1 });
  try {
    const outcome = await importDevRecords(store, contents);
    process.stdout.write(
      `stats:import: read ${String(outcome.read)} records, wrote ${String(outcome.written)}, ` +
        `skipped ${String(outcome.skipped)} already imported\n`,
    );
  } finally {
    await store.close();
  }
}

// Only run when invoked directly, so a test can import `importDevRecords` without side effects.
if (process.argv[1] !== undefined && import.meta.url.endsWith(process.argv[1].replace(/\\/g, "/"))) {
  main().catch((error: unknown) => {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 1;
  });
}
