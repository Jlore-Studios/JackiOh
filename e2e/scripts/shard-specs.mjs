// `node scripts/shard-specs.mjs <k> <K>` (from e2e/): the e2e specs shard k of K runs in CI, as one
// comma-separated `--spec` list (.github/workflows/ci.yml). CI splits each browser's run over K jobs
// so none of them takes long, and each job boots its own client and server, so R144's reseed at boot
// still gives every shard a fresh server.
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
 * Seconds per spec on ubuntu-latest, the slower of Chrome and Electron: CI run 36973335249 (main
 * at e762937). Specs 29-32 are estimates from the turns they play, spec 03's length per turn; once
 * they have run, replace them with measured times (#79).
 */
const WEIGHTS = {
  "01": 67, "02": 124, "03": 30, "04": 46, "05": 20, "06": 11, "07": 38, "08": 170, "09": 32,
  "10": 4, "11": 26, "12": 43, "13": 49, "14": 22, "15": 17, "16": 28, "17": 55, "18": 51,
  "19": 28, "20": 28, "21": 20, "22": 28, "23": 11, "24": 26, "25": 36, "26": 11, "27": 4,
  "28": 25, "29": 40, "30": 25, "31": 25, "32": 15, "99": 1,
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

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const k = Number(process.argv[2]);
  const count = Number(process.argv[3]);
  if (!Number.isInteger(k) || !Number.isInteger(count) || k < 1 || k > count) {
    process.stderr.write("usage: node scripts/shard-specs.mjs <k> <K>, 1 <= k <= K\n");
    process.exit(2);
  }
  const root = join(dirname(fileURLToPath(import.meta.url)), "..");
  const specs = readdirSync(join(root, SPEC_DIR)).filter((name) => name.endsWith(".cy.ts"));
  const picked = shardSpecs(specs, k, count);
  if (picked.length === 0) {
    process.stderr.write(`shard ${k}/${count} has no specs; lower the shard count\n`);
    process.exit(1);
  }
  process.stdout.write(picked.map((spec) => `${SPEC_DIR}/${spec}`).join(","));
}
