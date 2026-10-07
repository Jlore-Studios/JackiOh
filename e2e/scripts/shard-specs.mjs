// `node scripts/shard-specs.mjs [--smoke] <k> <K>` (from e2e/): the e2e specs shard k of K runs in
// CI, as one comma-separated `--spec` list (.github/actions/e2e-shard). The daily super run
// (.github/workflows/super.yml) splits every spec over K jobs; a pull request's `e2e smoke`
// (.github/workflows/ci.yml) runs only SMOKE, with `--smoke`. Each job boots its own client and
// server, so R144's reseed at boot still gives every shard a fresh server.
//
// The split balances measured time: longest spec first, each to the shard with the least so far
// (WEIGHTS, seconds per spec on a CI runner; a spec not listed counts DEFAULT_WEIGHT). Within a shard
// the specs keep their numeric order, as a full run would play them. Every spec lands in exactly one
// shard for any K, so a new spec is never left out; refresh WEIGHTS when the timings drift.

import { readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const SPEC_DIR = "cypress/e2e";

/**
 * The per-pull-request smoke list (docs/v0.3.0/README.md §7), one spec per user-facing flow: 01 a
 * hotseat game and its replay hash, 06 a networked match, 10 the invite gate, 13 practice against the
 * AI, 19 the queue modes and a series. Spec number prefixes, as WEIGHTS keys them.
 */
export const SMOKE = ["01", "06", "10", "13", "19"];

/**
 * Seconds per spec on ubuntu-latest, Chrome and Electron's slower: CI run 36973335249 (main at
 * e762937, the TypeScript server). Specs 29-35 are estimates (29-32 from the turns they play, spec
 * 03's length per turn); once they have run, replace them with measured times (#79).
 */
const WEIGHTS = {
  "01": 67, "02": 124, "03": 30, "04": 46, "05": 20, "06": 11, "07": 38, "08": 170, "09": 32,
  "10": 4, "11": 26, "12": 43, "13": 49, "14": 22, "15": 17, "16": 28, "17": 55, "18": 51,
  "19": 28, "20": 28, "21": 20, "22": 28, "23": 11, "24": 26, "25": 36, "26": 11, "27": 4,
  "28": 25, "29": 40, "30": 25, "31": 25, "32": 15, "33": 15, "34": 20, "35": 20, "99": 1,
};
const DEFAULT_WEIGHT = 30;

export function shardSpecs(specs, k, count) {
  const weight = (spec) => WEIGHTS[spec.slice(0, 2)] ?? DEFAULT_WEIGHT;
  const shards = Array.from({ length: count }, () => ({ load: 0, specs: [] }));
  const byWeight = [...specs].sort((a, b) => weight(b) - weight(a) || (a < b ? -1 : 1));
  for (const spec of byWeight) {
    let lightest = shards[0];
    for (const shard of shards) if (shard.load < lightest.load) lightest = shard;
    lightest.specs.push(spec);
    lightest.load += weight(spec);
  }
  return shards[k - 1].specs.sort();
}

/** The SMOKE specs among `specs`; throws unless each SMOKE prefix names exactly one of them. */
export function smokeSpecs(specs) {
  return SMOKE.map((prefix) => {
    const matches = specs.filter((spec) => spec.startsWith(`${prefix}-`));
    if (matches.length !== 1) {
      throw new Error(`smoke spec ${prefix} matches ${matches.length} files in ${SPEC_DIR}: ${matches.join(", ") || "none"}`);
    }
    return matches[0];
  });
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  const smoke = args[0] === "--smoke";
  const k = Number(args[smoke ? 1 : 0]);
  const count = Number(args[smoke ? 2 : 1]);
  if (args.length !== (smoke ? 3 : 2) || !Number.isInteger(k) || !Number.isInteger(count) || k < 1 || k > count) {
    process.stderr.write("usage: node scripts/shard-specs.mjs [--smoke] <k> <K>, 1 <= k <= K\n");
    process.exit(2);
  }
  const root = join(dirname(fileURLToPath(import.meta.url)), "..");
  const every = readdirSync(join(root, SPEC_DIR)).filter((name) => name.endsWith(".cy.ts"));
  let specs = every;
  if (smoke) {
    try {
      specs = smokeSpecs(every);
    } catch (error) {
      process.stderr.write(`${error.message}; update SMOKE in scripts/shard-specs.mjs\n`);
      process.exit(1);
    }
  }
  const picked = shardSpecs(specs, k, count);
  if (picked.length === 0) {
    process.stderr.write(`shard ${k}/${count} has no specs; lower the shard count\n`);
    process.exit(1);
  }
  process.stdout.write(picked.map((spec) => `${SPEC_DIR}/${spec}`).join(","));
}
