// `node scripts/shard-specs.mjs [--smoke | --component] <k> <K>` (from e2e/): the e2e specs shard k of K runs in
// CI, as one comma-separated `--spec` list (.github/actions/e2e-shard). The daily super run
// (.github/workflows/super.yml) splits every spec over K jobs; a pull request's `e2e smoke`
// (.github/workflows/ci.yml) runs only SMOKE, with `--smoke`. Each job boots its own client and
// server, so R144's reseed at boot still gives every shard a fresh server.
//
// The split balances measured time: longest spec first, each to the shard with the least so far
// (WEIGHTS, seconds per spec on a CI runner; a spec not listed counts DEFAULT_WEIGHT). Within a shard
// the specs keep their numeric order, as a full run would play them. Every spec lands in exactly one
// shard for any K, so a new spec is never left out; refresh WEIGHTS when the timings drift. `--component` splits
// cypress/component/*.cy.tsx by COMPONENT_WEIGHTS the same way, for the super run's component jobs (#3439).

import { readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const SPEC_DIR = "cypress/e2e";
const COMPONENT_DIR = "cypress/component";

/**
 * The per-pull-request smoke list (docs/v0.3.0/README.md §7), one spec per user-facing flow: 01 a
 * hotseat game and its replay hash, 06 a networked match, 10 the invite gate, 13 practice against the
 * AI, 19 the queue modes and a series, 46 every menu on one screen (R1581000), 47 every
 * control's tap area under a finger (R2122000), 51 the menus' shell (R3728000). Spec number prefixes,
 * as WEIGHTS keys them.
 */
export const SMOKE = ["01", "06", "10", "13", "19", "46", "47", "51"];

/**
 * Seconds per spec on ubuntu-latest, Chrome and Electron's slower: CI run 36973335249 (main at
 * e762937, the TypeScript server). Specs 01, 06 and 19 are #2181's: the smoke shards run on a
 * 4-core runner against the release jackioh-server, Chrome for Testing 154 (main at 17d45db), each
 * spec's Cypress "Duration"; 19's All random test alone waits about 50 s for the rating window to
 * widen after the rated game before it. Specs 29-36 are estimates (29-32 from the turns
 * they play, spec 03's length per turn; 36 from its play-through of the Meditative set, about six
 * minutes in Chromium on a loaded machine); 50 from
 * its sampling windows (5 s idle + 8 s combat) plus its setup turns, until it has run (#3605);
 * 46 and 47 are #3728's estimates for its seven sizes (`MENU_SIZES`), scaled from 46's 52 s at two
 * sizes (#2181) and 47's 35 s at one, and 51 an estimate, until each is measured;
 * once they have run, replace them with measured times (#79).
 */
const WEIGHTS = {
  "01": 82, "02": 124, "03": 30, "04": 46, "05": 20, "06": 17, "07": 38, "08": 170, "09": 32,
  "10": 4, "11": 26, "12": 43, "13": 49, "14": 22, "15": 17, "16": 28, "17": 55, "18": 51,
  "19": 99, "20": 28, "21": 20, "22": 28, "23": 11, "24": 26, "25": 36, "26": 11, "27": 4, "50": 90,
  "28": 25, "29": 40, "30": 25, "31": 25, "32": 15, "33": 15, "34": 20, "35": 20, "36": 360, "46": 180, "47": 200, "51": 90, "99": 1,
};
const DEFAULT_WEIGHT = 30;

/**
 * Seconds per component spec over 30 s, by file name: each spec's Cypress "Duration" in the daily super run
 * 43123557600 (all 78 on CI's 4-core runner, Chrome 156, `_staging` at 6cbaca4, #3744). fx-phone-living-game's
 * 120 is an estimate of its every moment played once at three sizes (R3744000), which that run never reached.
 * A spec not listed counts DEFAULT_COMPONENT_WEIGHT.
 */
const COMPONENT_WEIGHTS = {
  "anomaly-moments.cy.tsx": 67, "arena-corners.cy.tsx": 59, "board-layout.cy.tsx": 33, "card-diorama.cy.tsx": 251,
  "card-faces.cy.tsx": 420, "deck-cosmetics.cy.tsx": 65, "deckbuilder-layout.cy.tsx": 124, "emotes-layout.cy.tsx": 31,
  "fx-gallery-captures.cy.tsx": 69, "fx-phone-frame.cy.tsx": 78, "fx-phone-living-game.cy.tsx": 120,
  "fx-signature-classic.cy.tsx": 43, "fx-signature-stinkyscape-1.cy.tsx": 120, "fx-signature-stinkyscape.cy.tsx": 48,
  "fx-signature.cy.tsx": 299, "fx-terminus.cy.tsx": 99, "meditative-faces.cy.tsx": 55, "mobile-ux.cy.tsx": 34,
  "practice-table.cy.tsx": 47, "starship-moments.cy.tsx": 179, "stinkyscape-scenes.cy.tsx": 58,
};
const DEFAULT_COMPONENT_WEIGHT = 10;

/** A component spec's weight, for `shardSpecs`. */
export function componentWeight(spec) {
  return COMPONENT_WEIGHTS[spec] ?? DEFAULT_COMPONENT_WEIGHT;
}

export function shardSpecs(specs, k, count, weight = (spec) => WEIGHTS[spec.slice(0, 2)] ?? DEFAULT_WEIGHT) {
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
  const component = args[0] === "--component";
  const flagged = smoke || component;
  const k = Number(args[flagged ? 1 : 0]);
  const count = Number(args[flagged ? 2 : 1]);
  if (args.length !== (flagged ? 3 : 2) || !Number.isInteger(k) || !Number.isInteger(count) || k < 1 || k > count) {
    process.stderr.write("usage: node scripts/shard-specs.mjs [--smoke | --component] <k> <K>, 1 <= k <= K\n");
    process.exit(2);
  }
  const root = join(dirname(fileURLToPath(import.meta.url)), "..");
  const dir = component ? COMPONENT_DIR : SPEC_DIR;
  const every = readdirSync(join(root, dir)).filter((name) => name.endsWith(component ? ".cy.tsx" : ".cy.ts"));
  let specs = every;
  if (smoke) {
    try {
      specs = smokeSpecs(every);
    } catch (error) {
      process.stderr.write(`${error.message}; update SMOKE in scripts/shard-specs.mjs\n`);
      process.exit(1);
    }
  }
  const picked = component ? shardSpecs(specs, k, count, componentWeight) : shardSpecs(specs, k, count);
  if (picked.length === 0) {
    process.stderr.write(`shard ${k}/${count} has no specs; lower the shard count\n`);
    process.exit(1);
  }
  process.stdout.write(picked.map((spec) => `${dir}/${spec}`).join(","));
}
