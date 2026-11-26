// `node scripts/shard-specs.mjs [--smoke | --component] <k> <K>` (from e2e/): the e2e specs shard k of K runs in
// CI, as one comma-separated `--spec` list (.github/actions/e2e-shard). The daily super run
// (.github/workflows/super.yml) splits every spec over K jobs; a pull request's `e2e smoke`
// (.github/workflows/ci.yml) runs SMOKE plus the specs its changed cards reach (#5099), with `--smoke`. Each job boots its own client and
// server, so R144's reseed at boot still gives every shard a fresh server.
//
// The split balances measured time: longest spec first, each to the shard with the least so far
// (WEIGHTS, seconds per spec on a CI runner; a spec not listed counts DEFAULT_WEIGHT). Within a shard
// the specs keep their numeric order, as a full run would play them. Every spec lands in exactly one
// shard for any K, so a new spec is never left out; refresh WEIGHTS when the timings drift. `--component` splits
// cypress/component/*.cy.tsx by COMPONENT_WEIGHTS the same way, for the super run's component jobs (#3439).

import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const SPEC_DIR = "cypress/e2e";
const COMPONENT_DIR = "cypress/component";
const DECK_DIR = "fixtures/decks";
const JACKIOH_BIN = "target/release/jackioh";
const CHANGED_CARDS_TIMEOUT_MS = 60_000;

/**
 * The per-pull-request smoke list (docs/v0.3.0/README.md §7), one spec per user-facing flow: 01 a
 * hotseat game and its replay hash, 06 a networked match, 10 the invite gate, 13 practice against the
 * AI, 19 the queue modes and a series, 51 the menus' shell (R3728000). Specs 46 (every menu on one
 * screen, R1581000) and 47 (every control's tap area under a finger, R2122000) run in the daily super
 * run only: issue #4092, an owner decision, cut them because each took a smoke shard to itself (7.1-7.7
 * and 5.6-6.0 minutes) and set the slowest chain through CI. Spec number prefixes, as WEIGHTS keys them.
 */
export const SMOKE = ["01", "06", "10", "13", "19", "51"];

/**
 * Seconds of WEIGHTS a pull request's extra specs may add per shard (#5099); today's heaviest smoke
 * shard is 82 s, and shards of about 285 s ran 8m 7s and 8m 57s (#3764).
 */
export const SMOKE_EXTRA_BUDGET_S = 120;
const NEVER_ADDED = ["46", "47"];

/**
 * Seconds per spec on ubuntu-latest, Chrome and Electron's slower: CI run 36973335249 (main at
 * e762937, the TypeScript server). Specs 01, 06 and 19 are #2181's: the smoke shards run on a
 * 4-core runner against the release jackioh-server, Chrome for Testing 154 (main at 17d45db), each
 * spec's Cypress "Duration"; 19 then waited about 50 s in its All random test for the rating window to
 * widen after the rated game before it. #4485 has 19 reseed before each test and 51 shoot only with
 * `--expose shots=1`: their 39 and 73 are the Cypress "Duration" of a run of each on a 4-core sandbox
 * against the release server (19 took 39 s there, 51 took 73 s without the shots and 133 s with them),
 * until the smoke run on that pull request measures them; replace both with its "Duration".
 * Specs 29-36 are estimates (29-32 from the turns they play, spec 03's length per turn; 36 from its play-through of the Meditative set, about six
 * minutes in Chromium on a loaded machine); 50 from
 * its sampling windows (5 s idle + 8 s combat) plus its setup turns, until it has run (#3605);
 * 46 and 47 are #3728's estimates for its seven sizes (`MENU_SIZES`), scaled from 46's 52 s at two
 * sizes (#2181) and 47's 35 s at one, until each is measured;
 * once they have run, replace them with measured times (#79).
 */
const WEIGHTS = {
  "01": 82, "02": 124, "03": 30, "04": 46, "05": 20, "06": 17, "07": 38, "08": 170, "09": 32,
  "10": 4, "11": 26, "12": 43, "13": 49, "14": 22, "15": 17, "16": 28, "17": 55, "18": 51,
  "19": 39, "20": 28, "21": 20, "22": 28, "23": 11, "24": 26, "25": 36, "26": 11, "27": 4, "50": 90,
  "28": 25, "29": 40, "30": 25, "31": 25, "32": 15, "33": 15, "34": 20, "35": 20, "36": 360, "46": 180, "47": 200, "51": 73, "99": 1,
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

export function specWeight(spec) {
  return WEIGHTS[spec.slice(0, 2)] ?? DEFAULT_WEIGHT;
}

export function shardSpecs(specs, k, count, weight = specWeight) {
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

function quotes(text, word) {
  return ['"', "'", "`"].some((quote) => text.includes(`${quote}${word}${quote}`));
}

/** Spec -> { how, cards }: "names" when its source quotes a changed card's id or catalog name, else "deals" when it quotes the id of a fixture deck holding one. */
export function reachedSpecs(ids, specs, { sources, decks, names }) {
  const reached = new Map();
  for (const spec of specs) {
    if (NEVER_ADDED.includes(spec.slice(0, 2))) continue;
    const text = sources[spec];
    const named = ids.filter((id) => quotes(text, id) || (names[id] !== undefined && quotes(text, names[id])));
    const dealt = ids.filter((id) => Object.entries(decks).some(([deck, cards]) => cards.includes(id) && quotes(text, deck)));
    if (named.length > 0) reached.set(spec, { how: "names", cards: named });
    else if (dealt.length > 0) reached.set(spec, { how: "deals", cards: dealt });
  }
  return reached;
}

/** SMOKE, then the reached specs, "names" before "deals", each lightest first (ties by file name), while their weights total at most SMOKE_EXTRA_BUDGET_S * count. The first spec past the budget and all after it are `left` to the daily run. */
export function smokeSelection(every, ids, count, world) {
  const smoke = smokeSpecs(every);
  const reached = reachedSpecs(ids, every.filter((spec) => !smoke.includes(spec)), world);
  const rank = (spec) => (reached.get(spec).how === "names" ? 0 : 1);
  const order = [...reached.keys()].sort((a, b) => rank(a) - rank(b) || specWeight(a) - specWeight(b) || (a < b ? -1 : 1));
  const budget = SMOKE_EXTRA_BUDGET_S * count;
  const added = [];
  const left = [];
  let load = 0;
  for (const spec of order) {
    if (left.length === 0 && load + specWeight(spec) <= budget) {
      added.push(spec);
      load += specWeight(spec);
    } else {
      left.push(spec);
    }
  }
  return { specs: [...smoke, ...added], added, left, load, budget, reached };
}

/** `root` is e2e/. */
export function readWorld(root, ids) {
  const sources = {};
  for (const name of readdirSync(join(root, SPEC_DIR))) {
    if (name.endsWith(".cy.ts")) sources[name] = readFileSync(join(root, SPEC_DIR, name), "utf8");
  }
  const decks = {};
  for (const name of readdirSync(join(root, DECK_DIR))) {
    if (!name.endsWith(".json")) continue;
    const deck = JSON.parse(readFileSync(join(root, DECK_DIR, name), "utf8"));
    decks[deck.id ?? name.slice(0, -".json".length)] = deck.cards;
  }
  const catalog = JSON.parse(readFileSync(join(root, "..", "crates/cards/catalog.json"), "utf8"));
  const names = Object.fromEntries(ids.filter((id) => catalog[id]?.name !== undefined).map((id) => [id, catalog[id].name]));
  return { sources, decks, names };
}

function log(line) {
  process.stderr.write(`smoke: ${line}\n`);
}

/** `jackioh changed-cards --json` from JACKIOH_BIN (default ../target/release/jackioh); undefined with no binary (a local run) or when it fails: the list then stays SMOKE. */
function changedCards(root) {
  const bin = process.env.JACKIOH_BIN ?? join(root, "..", JACKIOH_BIN);
  if (!existsSync(bin)) return undefined;
  try {
    const out = execFileSync(bin, ["changed-cards", "--json", "--root", join(root, "..")], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "inherit"],
      timeout: CHANGED_CARDS_TIMEOUT_MS,
    });
    const changed = JSON.parse(out);
    if (!Array.isArray(changed.ids) || !Array.isArray(changed.notes)) throw new Error("no ids and notes in its output");
    return changed;
  } catch (error) {
    log(`${bin} changed-cards failed (${error.message}); the list stays SMOKE`);
    return undefined;
  }
}

function logSelection(changed, { added, left, load, budget, reached }) {
  log(`${changed.ids.length} changed cards: ${changed.ids.join(", ")}`);
  for (const spec of added) {
    const { how, cards } = reached.get(spec);
    log(`+ ${spec} (${specWeight(spec)} s) ${how} ${cards.join(", ")}`);
  }
  log(`${added.length} extra specs, ${load} of ${budget} s (${SMOKE_EXTRA_BUDGET_S} s a shard)`);
  if (left.length > 0) log(`left to the daily run: ${left.join(", ")}`);
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
    const changed = changedCards(root);
    if (changed !== undefined && changed.ids.length > 0) {
      const selection = smokeSelection(every, changed.ids, count, readWorld(root, changed.ids));
      specs = selection.specs;
      logSelection(changed, selection);
    } else if (changed !== undefined) {
      const why = changed.warning ? [...changed.notes, changed.warning] : changed.notes;
      log(`no changed cards${why.map((note) => `; ${note}`).join("")}`);
    }
  }
  const picked = component ? shardSpecs(specs, k, count, componentWeight) : shardSpecs(specs, k, count);
  if (picked.length === 0) {
    process.stderr.write(`shard ${k}/${count} has no specs; lower the shard count\n`);
    process.exit(1);
  }
  process.stdout.write(picked.map((spec) => `${dir}/${spec}`).join(","));
}
