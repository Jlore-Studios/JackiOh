// Pre-flight for e2e/fixtures/decks/*.json: the checks the engine's `validateDeck` would make
// (SPEC §9.4 L2/L3/L6, DECK_SIZE and LIBRARY_CAP read off the engine's generated config,
// MAX_COPIES = 1, no Token cards), run without a browser or a built client so a bad fixture is
// caught before the suite ever starts. e2e's `typecheck` runs it too, so CI's web job does on every
// pull request it runs for (#2049).
//
// A fixture may carry a `handicap` (R180), the seat's resources when a scenario needs other than
// SPEC's own (spec 25: a 5-card library, a 9-card opening hand, a 60-card library). It is checked as
// the engine's `validateHandicap` checks one — five non-negative integers, 1 <= deckSize <=
// LIBRARY_CAP (R184) and an optional positive heroHealth (R290) — and L2 is then its deckSize.
//
//   pnpm --dir e2e check:fixtures

import { readFileSync, readdirSync, existsSync } from "node:fs";
import path from "node:path";
import process from "node:process";

const here = path.dirname(new URL(import.meta.url).pathname);
const decksDir = path.join(here, "..", "fixtures", "decks");
const catalogPath = path.join(here, "..", "..", "crates", "cards", "catalog.json");
const engineConfigPath = path.join(here, "..", "..", "apps", "web", "src", "wire", "engineConfig.ts");

const engineConfig = readFileSync(engineConfigPath, "utf8");

/**
 * One of the constants `crates/engine/tests/export_config.rs` generates (CLAUDE.md rule 9), read
 * rather than restated: this script is plain Node and cannot import TypeScript, so it reads the line.
 */
function engineConstant(name) {
  const value = Number(new RegExp(`^export const ${name} = (\\d+);$`, "m").exec(engineConfig)?.[1]);
  if (!Number.isInteger(value)) {
    console.error(`no "export const ${name} = <n>;" line in ${engineConfigPath}`);
    process.exit(1);
  }
  return value;
}

/** #5099: a testkit stand-in's id prefix (`crates/engine/src/testkit/stand_in.rs`), a string constant. */
const STAND_IN_PREFIX = JSON.parse(/^export const STAND_IN_PREFIX = (".*");$/m.exec(engineConfig)?.[1] ?? "null");
if (typeof STAND_IN_PREFIX !== "string") {
  console.error(`no "export const STAND_IN_PREFIX = "<prefix>";" line in ${engineConfigPath}`);
  process.exit(1);
}

/**
 * #5099: the shape of a stand-in id, `STAND_IN_GRAMMAR`'s: a cost, then a Unit's `<attack>/<health>` and its
 * keywords, or a textless Spell, Field Spell, Trap or Field Trap. The engine parses the rest and panics with
 * the grammar on a bad keyword; this catches a typo in the shape before a browser starts.
 */
const STAND_IN_SHAPE = new RegExp(
  `^${STAND_IN_PREFIX} \\(\\d+\\) (\\d+/\\d+( [^+*#@:]+)?|Spell|Field Spell|Trap|Field Trap)$`,
);

/**
 * #5099: the calls that hand a fixture's cards to anything but `cy.seedGame`: the loadout and deck installs,
 * whose server engine has no testkit and refuses a stand-in (§9.4 L6), and a spec's own `cy.fixture` read
 * (spec 10 holds its cards to the starter set and installs them). Only `cy.seedGame`'s hotseat game, whose
 * engine carries the testkit, takes a stand-in.
 */
const SERVER_LOADERS = ["installLoadout(", "legalDeck(", "cy.fixture("];
const specsDirs = [path.join(here, "..", "cypress"), path.join(here, "..", "support")];

/** Every spec and support source, read once. */
function sources() {
  const out = [];
  const walk = (dir) => {
    if (!existsSync(dir)) return;
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) walk(full);
      else if (/\.tsx?$/.test(entry.name)) out.push({ file: path.relative(path.join(here, ".."), full), text: readFileSync(full, "utf8") });
    }
  };
  for (const dir of specsDirs) walk(dir);
  return out;
}

/** Where a source hands fixture `id` to a server loader: by its literal, or by a `const` bound to it. */
function serverLoads(id, all) {
  const literal = new RegExp(`["'\`]${id}["'\`]`);
  const out = [];
  for (const { file, text } of all) {
    if (!literal.test(text)) continue;
    const names = [...text.matchAll(new RegExp(`const (\\w+) = ["'\`]${id}["'\`]`, "g"))].map((match) => match[1]);
    text.split("\n").forEach((line, at) => {
      if (!SERVER_LOADERS.some((loader) => line.includes(loader))) return;
      if (literal.test(line) || names.some((name) => new RegExp(`\\b${name}\\b`).test(line))) out.push(`${file}:${at + 1}`);
    });
  }
  return out;
}

/**
 * #5099: the hotseat fixtures that deal catalog cards a stand-in could spell, each for a reason a stand-in
 * cannot meet. Every other fixture `cy.seedGame` loads deals a stand-in for such a card (`pureStats`).
 */
const REAL_CARDS = new Map([
  ["01-aggro-a", "spec 15 hears each played Unit's own voice line, and spec 17 shows its art"],
  ["01-aggro-b", "spec 15 hears each played Unit's own voice line, and spec 17 shows its art"],
  ["50-busy-a", "spec 50 frames a busy board of real cards, their art and keyword marks"],
  ["50-busy-b", "spec 50 frames a busy board of real cards, their art and keyword marks"],
]);

/** A face's keyword line as the catalog writes it: `keyword_key`'s words joined by ", ". */
function keywordLine(face) {
  return (face?.keywords ?? []).map((keyword) => (keyword.n === undefined ? keyword.kind : `${keyword.kind} ${keyword.n}`)).join(", ");
}

/**
 * #5099: a Unit of one plain cost whose whole text, on both faces, is its keyword line: a card a spec can
 * only want for its numbers, which a stand-in spells and no balance patch moves.
 */
function pureStats(def) {
  if (def.type !== "Unit" || typeof def.cost !== "number") return false;
  return [def.base, def.radiant].every((face) => face === undefined || (face.text ?? "") === keywordLine(face));
}

/** §2.6 L2. */
const DECK_SIZE = engineConstant("DECK_SIZE");
/** R80: a library holds at most this many cards, so no handicap deck is larger (R184). */
const LIBRARY_CAP = engineConstant("LIBRARY_CAP");
const HANDICAP_FIELDS = ["deckSize", "manaBonus", "manaCap", "extraOpeningCards", "extraDrawsPerTurn"];

const isCount = (value) => Number.isInteger(value) && value >= 0;

/** The problems with a fixture's `handicap`, as `validateHandicap` would name them. */
function handicapProblems(handicap, where) {
  if (typeof handicap !== "object" || handicap === null || Array.isArray(handicap)) {
    return [`${where}: "handicap" must be an object (R180)`];
  }
  const out = [];
  for (const field of HANDICAP_FIELDS) {
    if (!isCount(handicap[field])) out.push(`${where}: handicap.${field} must be a non-negative integer (R180)`);
  }
  if (isCount(handicap.deckSize) && (handicap.deckSize < 1 || handicap.deckSize > LIBRARY_CAP)) {
    out.push(`${where}: handicap.deckSize must be between 1 and ${LIBRARY_CAP} (R184), got ${handicap.deckSize}`);
  }
  if (handicap.heroHealth !== undefined && !(isCount(handicap.heroHealth) && handicap.heroHealth >= 1)) {
    out.push(`${where}: handicap.heroHealth must be a positive integer (R290)`);
  }
  const known = new Set([...HANDICAP_FIELDS, "heroHealth"]);
  for (const key of Object.keys(handicap)) {
    if (!known.has(key)) out.push(`${where}: handicap.${key} is not a handicap field (R180)`);
  }
  return out;
}

if (!existsSync(decksDir)) {
  console.error(`no fixture decks at ${decksDir}`);
  process.exit(1);
}

let catalog = null;
if (existsSync(catalogPath)) {
  catalog = JSON.parse(readFileSync(catalogPath, "utf8"));
} else {
  console.warn("crates/cards/catalog.json is missing: skipping the id checks.");
}

const problems = [];
const files = readdirSync(decksDir).filter((name) => name.endsWith(".json")).sort();
const allSources = sources();

for (const file of files) {
  const id = file.replace(/\.json$/, "");
  const where = `decks/${file}`;
  let deck;
  try {
    deck = JSON.parse(readFileSync(path.join(decksDir, file), "utf8"));
  } catch (error) {
    problems.push(`${where}: not valid JSON (${error.message})`);
    continue;
  }

  if (deck.id !== id) problems.push(`${where}: "id" is "${deck.id}", expected "${id}"`);
  if (typeof deck.spec !== "string" || deck.spec.length === 0) problems.push(`${where}: missing "spec"`);
  if (typeof deck.description !== "string" || deck.description.length === 0) {
    problems.push(`${where}: missing "description" (say why these cards)`);
  }
  if (!Array.isArray(deck.cards)) {
    problems.push(`${where}: "cards" must be an array`);
    continue;
  }

  let size = DECK_SIZE;
  let sized = true;
  if (deck.handicap !== undefined) {
    const wrong = handicapProblems(deck.handicap, where);
    problems.push(...wrong);
    // A handicap that is itself wrong names no deck size to hold the cards to.
    if (wrong.length === 0) size = deck.handicap.deckSize;
    else sized = false;
  }
  if (sized && deck.cards.length !== size) {
    problems.push(
      deck.handicap === undefined
        ? `${where}: ${deck.cards.length} cards, DECK_SIZE is ${DECK_SIZE} (L2)`
        : `${where}: ${deck.cards.length} cards, its handicap's deckSize is ${size} (R184)`,
    );
  }
  const seen = new Set();
  for (const card of deck.cards) {
    if (seen.has(card)) problems.push(`${where}: "${card}" appears twice (L3, MAX_COPIES = 1)`);
    seen.add(card);
    if (card.startsWith(STAND_IN_PREFIX)) {
      if (!STAND_IN_SHAPE.test(card)) {
        problems.push(`${where}: "${card}" is no stand-in id; see STAND_IN_GRAMMAR in crates/engine/src/testkit/stand_in.rs`);
      }
      continue;
    }
    if (catalog === null) continue;
    const def = catalog[card];
    if (def === undefined) {
      problems.push(`${where}: "${card}" is not in the catalog (L6)`);
      continue;
    }
    if (def.token === true || (def.tags ?? []).includes("Token")) {
      problems.push(`${where}: "${card}" (${def.name}) is a Token card (L3)`);
    }
  }
  const loads = serverLoads(id, allSources);
  if (catalog !== null && loads.length === 0 && !REAL_CARDS.has(id)) {
    for (const card of deck.cards) {
      const def = catalog[card];
      if (def !== undefined && pureStats(def)) {
        problems.push(
          `${where}: "${card}" (${def.name}) is only its numbers and keywords; deal a stand-in that spells them (e2e/support/standIns.ts, #5099)`,
        );
      }
    }
  }
  if (deck.cards.some((card) => card.startsWith(STAND_IN_PREFIX))) {
    for (const at of loads) {
      problems.push(`${where}: holds a stand-in, which the server's engine refuses, yet ${at} installs it through the server (#5099)`);
    }
  }
}

if (files.length === 0) problems.push("no deck fixtures found");

if (problems.length > 0) {
  console.error(`${problems.length} problem(s) in e2e/fixtures/decks:`);
  for (const problem of problems) console.error(`  ${problem}`);
  process.exit(1);
}

console.log(`${files.length} deck fixture(s) OK`);
