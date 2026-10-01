// `pnpm ai:sweep` (R186, R390): the run that decides the AI's shadow ban, in two passes.
//
// Pass 1: for every non-token card of every set and every tier in AI_SWEEP.tiers, `sweepCard`
// forces the card into AI decks on that tier's handicap against the greedy baseline
// (AI_SWEEP.seedsPerCard games each, at AI_GATE_BUDGET). `atRiskIds` reads the at-risk cards off
// pass 1 (plus today's SHADOW_BAN and SHADOW_WATCH). Pass 2: `sweepAtRisk` forces each at-risk card
// into AI_SWEEP.seedsPerCardAtRisk more games per tier, with every at-risk card dealt more often as
// filler. `sweepVerdict` joins a card's tiers and passes. This prints the flagged rows of both
// passes, the at-risk cards, the suspects, the unswept cards, then the ready-made SHADOW_BAN and
// SHADOW_WATCH entries and the header line for packages/ai/src/shadowBan.ts. It always exits 0: the
// tables are copied in by hand from this output, never edited to taste.
//
//   pnpm ai:sweep                              both passes over every card, then the report
//   pnpm ai:sweep core-011 classic-020         only these ids (pass 2: those of them at risk)
//   pnpm ai:sweep --json core-011 …            pass 1 only: one JSON SweepResult per line (a slice)
//   pnpm ai:sweep --pass2 a.jsonl,b.jsonl …    pass 2 from every slice's pass-1 lines: one JSON
//                                              SweepPass2 per line, for the at-risk ids listed after
//                                              the files (default every at-risk id); slice it too
//   pnpm ai:sweep --report a.jsonl p2.jsonl …  the report from pass-1 and pass-2 lines
//
// Pass 2 needs every slice's pass 1 first, because the at-risk list it boosts is the whole sweep's.
// Node tooling, so it may read the clock (`performance.now` for decision timing), read files and
// write to the console; src/ stays pure and receives the clock as `now`.

import { readFileSync } from "node:fs";
import { CATALOG, registerAll } from "@jackioh/cards";
import { query } from "@jackioh/engine";
import {
  AI_GATE_BUDGET,
  AI_SWEEP,
  atRiskIds,
  pass2KeepOut,
  pass2Stats,
  sweepAtRisk,
  sweepCard,
  sweepVerdict,
  type SweepPass2,
  type SweepResult,
  type SweepStats,
} from "../src/index";

/** The mean evaluate change per play, and each card's seconds, printed to this many decimals. */
const DECIMALS = 1;

const MS_PER_SECOND = 1000;

function nameOf(defId: string): string {
  return CATALOG[defId]?.name ?? defId;
}

function meanDelta(stats: SweepStats): string {
  if (stats.evalDeltaCount === 0) return "n/a";
  return (stats.evalDeltaSum / stats.evalDeltaCount).toFixed(DECIMALS);
}

function row(stats: SweepStats, tier: string, flags: string): string {
  const cells = [
    stats.defId,
    nameOf(stats.defId),
    tier,
    flags,
    `${stats.drawnGames}/${stats.games}`,
    String(stats.affordableTurns),
    String(stats.plays),
    String(stats.errors),
    String(stats.timeouts),
    meanDelta(stats),
  ].map((cell) => ` ${cell.replace(/\|/g, "\\|")} `);
  return `|${cells.join("|")}|`;
}

const TABLE_HEAD = [
  "| id | name | tier | flags | drawn/games | affordable turns | plays | errors | timeouts | mean eval delta |",
  "|---|---|---|---|---|---|---|---|---|---|",
];

function timed<T>(label: string, run: () => T, describe: (result: T) => string): T {
  const started = performance.now();
  const result = run();
  const seconds = ((performance.now() - started) / MS_PER_SECOND).toFixed(DECIMALS);
  process.stderr.write(`[ai:sweep] ${label}: ${describe(result)} (${seconds}s)\n`);
  return result;
}

function pass1(ids: readonly string[]): SweepResult[] {
  const results: SweepResult[] = [];
  ids.forEach((id, index) => {
    for (const tier of AI_SWEEP.tiers) {
      const result = timed(
        `pass 1 ${index + 1}/${ids.length} ${id} ${nameOf(id)} @${tier}`,
        () => sweepCard(id, { now: () => performance.now(), tier }),
        (r) => (r.flags.length > 0 ? r.flags.join(", ") : r.unswept ? "unswept" : "clean"),
      );
      results.push(result);
    }
  });
  return results;
}

function pass2(ids: readonly string[], atRisk: readonly string[], keepOut: readonly string[]): SweepPass2[] {
  const results: SweepPass2[] = [];
  ids.forEach((id, index) => {
    for (const tier of AI_SWEEP.tiers) {
      const result = timed(
        `pass 2 ${index + 1}/${ids.length} ${id} ${nameOf(id)} @${tier}`,
        () => sweepAtRisk(id, atRisk, keepOut, { now: () => performance.now(), tier }),
        (r) => `${r.cards.length} at-risk card(s) dealt, ${r.suspects.length} suspect line(s)`,
      );
      results.push(result);
    }
  });
  return results;
}

function report(first: readonly SweepResult[], second: readonly SweepPass2[], elapsed: number | null): string {
  const ids = [...new Set(first.map((result) => result.defId))].sort();
  const atRisk = atRiskIds(first);
  const forced = [...new Set(second.map((result) => result.forced))].sort();
  const verdicts = [...new Set([...ids, ...forced])].sort().map((id) =>
    sweepVerdict(
      first.filter((result) => result.defId === id),
      second,
    ),
  );
  const banned = verdicts.filter((verdict) => verdict.reason !== null);
  const watched = verdicts.filter((verdict) => verdict.watch !== null);
  const unswept = verdicts.filter((verdict) => verdict.unswept);
  const suspects = second.flatMap((result) => result.suspects);

  // The run's date for shadowBan.ts's header. Intl formats "today" without a Date expression, which
  // this package's lint bans everywhere, tooling included.
  const date = new Intl.DateTimeFormat("en-CA", { timeZone: "UTC" }).format();
  const budget = JSON.stringify(AI_GATE_BUDGET);
  const tiers = AI_SWEEP.tiers.join(" and ");
  const passes =
    `pass 1 over ${ids.length} card(s) at ${tiers}, ${AI_SWEEP.seedsPerCard} seeds each (\`sweep:<tier>:<id>:<n>\`), ` +
    `pass 2 over ${forced.length} at-risk card(s), ${AI_SWEEP.seedsPerCardAtRisk} seeds each (\`sweep2:<tier>:<id>:<n>\`, ` +
    `at-risk filler ×${AI_SWEEP.atRiskBoost}), budget AI_GATE_BUDGET ${budget}`;

  const out: string[] = [];
  out.push("# AI shadow-ban sweep");
  out.push("");
  out.push(
    `Run ${date} (UTC): ${passes}; ${banned.length} banned, ${watched.length} watched, ${unswept.length} unswept, ` +
      `${suspects.length} suspect line(s)${elapsed === null ? "" : `, ${elapsed}s`}.`,
  );
  out.push("");
  out.push("## Pass 1 flags (a `neverPlayed` or `selfHarm` here only puts a card at risk)");
  out.push("");
  const flaggedRows = first.filter((result) => result.flags.length > 0);
  if (flaggedRows.length === 0) out.push("No card was flagged at any tier.");
  else out.push(...TABLE_HEAD, ...flaggedRows.map((result) => row(result, result.tier, result.flags.join(", "))));
  out.push("");
  out.push(`## At risk (${atRisk.length}): pass 1 at half strength, SHADOW_BAN and SHADOW_WATCH`);
  out.push("");
  out.push(atRisk.length === 0 ? "None." : atRisk.map((id) => `${id} ${nameOf(id)}`).join(", "));
  const missing = atRisk.filter((id) => ids.includes(id) && !forced.includes(id));
  if (missing.length > 0) out.push("", `Not swept in pass 2 (no evidence, no ban for neverPlayed or selfHarm): ${missing.join(", ")}`);
  out.push("");
  out.push("## Pass 2: every at-risk card over every game it was dealt in, forced or filler");
  out.push("");
  if (second.length === 0) {
    out.push("Pass 2 did not run.");
  } else {
    out.push(...TABLE_HEAD);
    for (const id of [...new Set(second.flatMap((result) => result.cards.map((card) => card.defId)))].sort()) {
      const verdict = verdicts.find((entry) => entry.defId === id);
      for (const tier of AI_SWEEP.tiers) {
        const total = pass2Stats(second, id, tier);
        if (total.games > 0) out.push(row(total, tier, verdict?.reason === null || verdict === undefined ? "cleared" : verdict.flags.join(", ")));
      }
    }
  }
  out.push("");
  out.push("## Suspects (an error or timeout in a pass-2 game that also dealt this at-risk card as filler)");
  out.push("");
  out.push(
    suspects.length === 0
      ? "None."
      : suspects
          .map(
            (s) =>
              `- suspect: ${s.defId} ${nameOf(s.defId)}: ${s.errors} error(s), ${s.timeouts} timeout(s) in ${s.seed} ` +
              `(forced ${s.forced} ${nameOf(s.forced)}); banned only if its own games repeat it`,
          )
          .join("\n"),
  );
  out.push("");
  out.push("## Unswept (never affordable at any tier: no evidence either way)");
  out.push("");
  out.push(unswept.length === 0 ? "None." : unswept.map((verdict) => `- ${verdict.defId} ${nameOf(verdict.defId)}`).join("\n"));
  out.push("");
  out.push("## For packages/ai/src/shadowBan.ts");
  out.push("");
  out.push("Header line:");
  out.push("");
  out.push(`// Sweep of record: ${date} (UTC), \`pnpm ai:sweep\`, ${passes}.`);
  out.push("");
  out.push("SHADOW_BAN entries:");
  out.push("");
  out.push("```ts");
  for (const verdict of banned) out.push(`  ${JSON.stringify(verdict.defId)}: ${JSON.stringify(verdict.reason)},`);
  out.push("```");
  out.push("");
  out.push("SHADOW_WATCH entries:");
  out.push("");
  out.push("```ts");
  for (const verdict of watched) out.push(`  ${JSON.stringify(verdict.defId)}: ${JSON.stringify(verdict.watch)},`);
  out.push("```");
  return `${out.join("\n")}\n`;
}

function readLines(files: readonly string[]): unknown[] {
  return files
    .flatMap((file) => readFileSync(file, "utf8").split("\n"))
    .filter((line) => line.trim() !== "")
    .map((line) => JSON.parse(line) as unknown);
}

function isPass2(line: unknown): line is SweepPass2 {
  return typeof line === "object" && line !== null && "forced" in line;
}

function main(): void {
  registerAll();

  const args = process.argv.slice(2).filter((arg) => arg !== "--");
  if (args[0] === "--report") {
    const lines = readLines(args.slice(1));
    process.stdout.write(report(lines.filter((line): line is SweepResult => !isPass2(line)), lines.filter(isPass2), null));
    return;
  }

  const pool = query().map((def) => def.id);
  const pickIds = (requested: readonly string[]): string[] => {
    const unknown = requested.filter((id) => !pool.includes(id));
    if (unknown.length > 0) process.stderr.write(`[ai:sweep] not non-token card ids, skipped: ${unknown.join(", ")}\n`);
    return requested.length > 0 ? pool.filter((id) => requested.includes(id)) : pool;
  };

  if (args[0] === "--pass2") {
    const first = readLines((args[1] ?? "").split(",").filter((file) => file !== "")) as SweepResult[];
    const atRisk = atRiskIds(first);
    const ids = pickIds(args.slice(2)).filter((id) => atRisk.includes(id));
    for (const result of pass2(ids, atRisk, pass2KeepOut(first))) process.stdout.write(`${JSON.stringify(result)}\n`);
    return;
  }

  const json = args[0] === "--json";
  const ids = pickIds(json ? args.slice(1) : args);
  const started = performance.now();
  const first = pass1(ids);
  if (json) {
    for (const result of first) process.stdout.write(`${JSON.stringify(result)}\n`);
    return;
  }
  const atRisk = atRiskIds(first);
  const second = pass2(
    ids.filter((id) => atRisk.includes(id)),
    atRisk,
    pass2KeepOut(first),
  );
  process.stdout.write(report(first, second, Math.round((performance.now() - started) / MS_PER_SECOND)));
}

main();
