// Admin script: card win rates off the game records (SPEC §9.11, R377, R378).
//
//   pnpm --filter @jackioh/server stats:cards [--source=live|dev|all] [--mode=bo1|bo3|random]
//     [--patch=<version>] [--pilot=human|ai|unified] [--card=<id>] [--json]
//
// Reads `public.game_records` through the store (DATABASE_URL), keeps the records the filter names,
// and prints every card's four breakdowns — in deck, in the opening hand, going first and going
// second, played against drawn but not played — each with the games behind it. With nothing named
// it reads live games of every mode, patch and pilot: a development run's records are read only
// with `--source=dev` or `--source=all` (R378). `--json` prints the report as data instead.
//
// The arithmetic is `cardStats` in packages/shared/src/stats.ts, the same the AI's development run
// prints its own games with, so a pre-release run and the live games after it compare figure for
// figure: run both with the same `--patch`.

import {
  cardStats,
  DEFAULT_CARD_STATS_FILTER,
  formatCardStats,
  GAME_MODES,
  PILOT_FILTERS,
  SOURCE_FILTERS,
  type CardStatsFilter,
  type CardStatsReport,
  type GameMode,
  type PilotFilter,
  type SourceFilter,
} from "@jackioh/shared";

import { loadCatalog } from "../api/catalog";
import type { Store } from "../api/ports";
import { createPostgresStore } from "./store";

const USAGE = `Usage: pnpm --filter @jackioh/server stats:cards [options]

  --source=live|dev|all      Live games (the default), an AI development run's, or both.
  --mode=bo1|bo3|random      One match type. Default: every mode.
  --patch=<version>          One patch, as patches.json names it. Default: every patch.
  --pilot=human|ai|unified   The seats counted. Default: unified, every seat.
  --card=<id>                One card's row only.
  --json                     The report as JSON.`;

export type CardStatsOptions = {
  filter: CardStatsFilter;
  /** One card's row, or null for every card. */
  card: string | null;
  json: boolean;
};

function oneOf<T extends string>(name: string, raw: string, allowed: readonly T[]): T {
  if ((allowed as readonly string[]).includes(raw)) return raw as T;
  throw new Error(`--${name} must be one of ${allowed.join(", ")} (got ${JSON.stringify(raw)}).\n\n${USAGE}`);
}

/** Parses `--flag=value` arguments (and the bare `--json`), refusing anything it does not know. */
export function parseCardStatsArgs(argv: readonly string[]): CardStatsOptions {
  const filter: CardStatsFilter = { ...DEFAULT_CARD_STATS_FILTER };
  let card: string | null = null;
  let json = false;

  for (const arg of argv) {
    if (arg === "--json") {
      json = true;
      continue;
    }
    const match = /^--(?<name>[a-z]+)=(?<value>.+)$/u.exec(arg);
    const name = match?.groups?.["name"];
    const raw = match?.groups?.["value"];
    if (name === undefined || raw === undefined) {
      throw new Error(`Unrecognised argument ${JSON.stringify(arg)}.\n\n${USAGE}`);
    }
    switch (name) {
      case "source":
        filter.source = oneOf<SourceFilter>(name, raw, SOURCE_FILTERS);
        break;
      case "mode":
        filter.mode = oneOf<GameMode>(name, raw, GAME_MODES);
        break;
      case "patch":
        filter.patch = raw;
        break;
      case "pilot":
        filter.pilot = oneOf<PilotFilter>(name, raw, PILOT_FILTERS);
        break;
      case "card":
        card = raw;
        break;
      default:
        throw new Error(`Unrecognised option --${name}.\n\n${USAGE}`);
    }
  }

  return { filter, card, json };
}

/** R377: the report the options ask for, off the records the store holds. */
export async function cardStatsReport(store: Store, options: CardStatsOptions): Promise<CardStatsReport> {
  const { source, mode, patch } = options.filter;
  const report = cardStats(await store.gameRecords.list({ source, mode, patch }), options.filter);
  if (options.card === null) return report;
  return { ...report, cards: report.cards.filter((stats) => stats.card === options.card) };
}

/** The report as the script prints it: the table, with the catalog's names, or JSON. */
export function renderCardStats(report: CardStatsReport, options: CardStatsOptions, nameOf: (card: string) => string | undefined): string {
  if (options.json) return JSON.stringify(report, null, 2);
  const table = formatCardStats(report, nameOf);
  if (options.card !== null && report.cards.length === 0) {
    return `${table}\nNo deck the filter counts held ${options.card}.`;
  }
  return table;
}

async function main(): Promise<void> {
  const options = parseCardStatsArgs(process.argv.slice(2));
  const connectionString = process.env["DATABASE_URL"];
  if (connectionString === undefined || connectionString === "") {
    throw new Error("DATABASE_URL is not set (see docs/architecture.md, env-var contract).");
  }

  const catalog = await loadCatalog();
  // One connection: this process runs one read and exits.
  const store = createPostgresStore({ connectionString, max: 1 });
  try {
    const report = await cardStatsReport(store, options);
    process.stdout.write(`${renderCardStats(report, options, (card) => catalog.defs[card]?.name)}\n`);
  } finally {
    await store.close();
  }
}

// Only run when invoked directly, so a test can import the functions without side effects.
if (process.argv[1] !== undefined && import.meta.url.endsWith(process.argv[1].replace(/\\/g, "/"))) {
  main().catch((error: unknown) => {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 1;
  });
}
