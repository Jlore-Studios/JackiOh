// Checks apps/web/WRITING.md's machine-checkable rules. A player's sentence is a string
// or JSX text in a non-test source that zh-CN.json translates.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import ts from "typescript";
import { beforeAll, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { assaultContent } from "./wasm/index.ts";
import glossary from "../../../crates/shared/data/glossary.json";
import zhCN from "./i18n/zh-CN.json";

const SRC = dirname(fileURLToPath(import.meta.url));

/**
 * `text` with each `{placeholder}` (no `}` inside counts) blanked, as `/\{[^}]+\}/g` with `" "` does
 * (#4009: the class rescans a brace run from every position).
 */
function stripPlaceholders(text: string): string {
  let out = "";
  let at = 0;
  for (;;) {
    const open = text.indexOf("{", at);
    if (open === -1) return out + text.slice(at);
    const close = text.indexOf("}", open + 1);
    if (close === -1) return out + text.slice(at);
    if (close === open + 1) {
      out += text.slice(at, open + 1);
      at = open + 1;
    } else {
      out += `${text.slice(at, open)} `;
      at = close + 1;
    }
  }
}

/**
 * `word` without its edge non-alphanumerics, as `/^[^a-zA-Z0-9]+|[^a-zA-Z0-9]+$/g` cuts them (#4009:
 * the edge classes rescan from every position).
 */
function trimWord(word: string): string {
  let start = 0;
  while (start < word.length && !/[a-zA-Z0-9]/.test(word.charAt(start))) start++;
  let end = word.length;
  while (end > start && !/[a-zA-Z0-9]/.test(word.charAt(end - 1))) end--;
  return word.slice(start, end);
}

function sources(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    // ts-rs's generated wire types hold registry literals verbatim (the card's own straight
    // quotes in `"Miss" Mrow`'s full arts), not developer-written player prose: not walked,
    // as `i18n.test.ts`'s `sources()` already holds.
    if (statSync(path).isDirectory()) return name === "test" || relative(SRC, path) === join("wire", "generated") ? [] : sources(path);
    return /\.(ts|tsx)$/.test(name) && !/\.test\.tsx?$/.test(name) ? [path] : [];
  });
}

function wordsIn(path: string): { at: string; text: string }[] {
  const text = readFileSync(path, "utf8");
  const file = ts.createSourceFile(path, text, ts.ScriptTarget.Latest, true, path.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS);
  const out: { at: string; text: string }[] = [];
  const visit = (node: ts.Node): void => {
    if (ts.isImportDeclaration(node) || ts.isExportDeclaration(node)) return;
    if (
      ts.isStringLiteral(node) ||
      ts.isNoSubstitutionTemplateLiteral(node) ||
      ts.isTemplateHead(node) ||
      ts.isTemplateMiddle(node) ||
      ts.isTemplateTail(node) ||
      ts.isJsxText(node)
    ) {
      const line = file.getLineAndCharacterOfPosition(node.getStart(file)).line + 1;
      out.push({ at: `${relative(SRC, path)}:${String(line)}`, text: node.text });
    }
    ts.forEachChild(node, visit);
  };
  visit(file);
  return out;
}

/** A developer word and the pattern that flags it, as WRITING.md's Plain table names it. */
const JARGON: readonly { word: string; pattern: RegExp }[] = [
  { word: "standard error", pattern: /\bstandard errors?\b/i },
  { word: "sample", pattern: /\bsamples?\b/i },
  { word: "artifact", pattern: /\bartifacts?\b/i },
  { word: "Δ", pattern: /Δ/u },
  { word: "Elo", pattern: /\bElo\b/ },
  { word: "drill-down", pattern: /\bdrill-down\b/i },
  { word: "SFX", pattern: /\bSFX\b/ },
  { word: "a rule's or section's number", pattern: /\bRule [A-Z]\d|§/u },
  { word: "Failed to", pattern: /^Failed to\b/ },
];

const MARKS: readonly { word: string; pattern: RegExp }[] = [
  { word: "a curly apostrophe", pattern: /’/u },
  { word: "a straight double quote", pattern: /"/u },
];

/** WRITING.md, Consistent: American spelling (#2445). "optimism" and "optimistic" are American too. */
const BRITISH =
  /colour|behaviour|centre|honour|favour|armour|licence|defence|catalogue|labell(ed|ing)|cancell(ed|ing)|modell(ed|ing)|travell(ed|ing)|(serial|normal|recogn|optim|organ)is[eai]/i;

let cachedTexts: { at: string; text: string }[] | undefined;

/** Parsing every source is the slow part, so the file's tests share one pass. */
function playerTexts(): { at: string; text: string }[] {
  cachedTexts ??= sources(SRC).flatMap(wordsIn).filter(({ text }) => Object.hasOwn(zhCN, text) || Object.hasOwn(zhCN, text.trim()));
  return cachedTexts;
}

function jargonIn(text: string): string[] {
  return JARGON.filter(({ pattern }) => pattern.test(text)).map(({ word }) => word);
}

function marksIn(text: string): string[] {
  return MARKS.filter(({ pattern }) => pattern.test(text)).map(({ word }) => word);
}

describe("WRITING.md the client's words for players", () => {
  // The one pass over every source is setup, not an assertion: it runs here, once, so a loaded runner
  // does not spend the first test's own budget on it.
  beforeAll(() => {
    playerTexts();
  });

  it("no sentence the client shows a player uses a developer word", () => {
    const texts = playerTexts();
    expect(texts.length).toBeGreaterThan(1000);
    const found = texts
      .flatMap(({ at, text }) => jargonIn(text).map((word) => `${at}: ${word}: ${JSON.stringify(text)}`))
      .sort();
    expect(found).toEqual([]);
  });

  it("no sentence the client shows a player has a curly apostrophe or a straight double quote, and no dictionary key a curly apostrophe", () => {
    const texts = playerTexts();
    const found = texts
      .flatMap(({ at, text }) => marksIn(text).map((word) => `${at}: ${word}: ${JSON.stringify(text)}`))
      .sort();
    expect(found).toEqual([]);
    expect(Object.keys(zhCN).filter((key) => key.includes("’"))).toEqual([]);
  });

  it("no sentence the client shows a player and no dictionary key spells a word the British way", () => {
    const found = playerTexts()
      .filter(({ text }) => BRITISH.test(text))
      .map(({ at, text }) => `${at}: ${JSON.stringify(text)}`)
      .sort();
    expect(found).toEqual([]);
    expect(Object.keys(zhCN).filter((key) => BRITISH.test(key))).toEqual([]);
    expect(BRITISH.test("Room cancelled. Its code no longer works.")).toBe(true);
    expect(BRITISH.test("Favourite card")).toBe(true);
    expect(BRITISH.test("Room canceled. Its code no longer works.")).toBe(false);
    expect(BRITISH.test("Read that edge as optimistic.")).toBe(false);
  });

  it("the guards flag the old words and marks and pass the new ones", () => {
    expect(jargonIn("No card is {z} standard errors from 50% yet.")).toHaveLength(1);
    expect(jargonIn("Sample size threshold:")).toHaveLength(1);
    expect(jargonIn("Played Δ")).toHaveLength(1);
    expect(jargonIn("Three decks with no cards in common make a trio for Conquest (Rule L4). Each card can appear in only one deck.")).toHaveLength(
      1,
    );
    expect(jargonIn("Failed to load card statistics.")).toHaveLength(1);
    expect(jargonIn("SFX volume")).toHaveLength(1);
    expect(jargonIn("Player stats are public by default. Elo ratings and rankings are kept separate.")).toHaveLength(1);
    expect(jargonIn("Loading drill-down data…")).toHaveLength(1);
    expect(jargonIn("No card is {z} margins or more from 50% yet.")).toEqual([]);
    expect(jargonIn("Too few games:")).toEqual([]);
    expect(jargonIn("Played vs held")).toEqual([]);
    expect(jargonIn("Three decks with no card in common make a trio for Conquest: each card can be in only one of them.")).toEqual([]);
    expect(jargonIn("The card statistics didn't load.")).toEqual([]);
    expect(jargonIn("Sound effects volume")).toEqual([]);
    expect(jargonIn("Player stats are public by default. Ratings and ranks are not shown here.")).toEqual([]);
    expect(jargonIn("Loading this card's details…")).toEqual([]);
    expect(marksIn("That page doesn’t exist.")).toHaveLength(1);
    expect(marksIn('"not enough games"')).toHaveLength(1);
    expect(marksIn("That page doesn't exist.")).toEqual([]);
    expect(marksIn("“not enough games”")).toEqual([]);
  });

  it("R3230000 no label has a capital past its first word outside the allowlist", () => {
    const hits: { key: string; flagged: string[] }[] = [];
    for (const key of Object.keys(zhCN)) {
      const words = stripPlaceholders(key).trim().split(/\s+/).filter(Boolean);
      if (words.length >= 2 && words.length <= 6 && !/[.?!]/.test(key) && !key.includes("\n")) {
        const flagged = strayCapitals(key);
        if (flagged.length > 0) {
          hits.push({ key, flagged });
        }
      }
    }
    expect(hits).toEqual([]);
  });

  it("R3230000 the guard flags Title Case and passes sentence case", () => {
    expect(strayCapitals("Dungeon Run")).toEqual(["Run"]);
    expect(strayCapitals("Most Toppled")).toEqual(["Toppled"]);
    expect(strayCapitals("Dungeon run")).toEqual([]);
    expect(strayCapitals("Back to Play together")).toEqual([]);
    expect(strayCapitals("Gain Armor")).toEqual([]);
    expect(strayCapitals("Choose Tributes")).toEqual([]);
    expect(strayCapitals("Field Spells and Traps")).toEqual([]);
    expect(strayCapitals(CATALOG["c002_speciesist"]?.name ?? "Speciesist")).toEqual([]);
  });

  it("R3230000 no key and no translation starts or ends with an arrow", () => {
    expect(ARROW.test("← Back")).toBe(true);
    expect(ARROW.test("Next patch →")).toBe(true);
    expect(ARROW.test("Back")).toBe(false);
    expect(ARROW.test("Next patch")).toBe(false);

    const failures: string[] = [];
    for (const [k, v] of Object.entries(zhCN)) {
      if (ARROW.test(k.trim())) failures.push(`key: ${k}`);
      if (typeof v === "string") {
        if (ARROW.test(v.trim())) failures.push(`val: ${v}`);
      } else if (v && typeof v === "object") {
        for (const [subk, subv] of Object.entries(v as Record<string, string>)) {
          if (typeof subv === "string" && ARROW.test(subv.trim())) {
            failures.push(`plural ${k}.${subk}: ${subv}`);
          }
        }
      }
    }
    expect(failures).toEqual([]);
  });

  it("R3230000 the words the issue retired are gone", () => {
    const retired = [
      "Dungeon Run",
      "Privacy Policy",
      "All Random",
      "Most Toppled",
      "Never Tried It",
      "Field Guide",
      "Field Spells and Traps",
      "Keep playing",
      "Leave game",
      "Leave fight",
      "Create an account",
      "Card almanac",
      "Privacy",
      "Terms",
      "← Back",
      "← Play together",
      "← All decks",
      "Back to lobby",
    ];
    for (const word of retired) {
      expect(Object.hasOwn(zhCN, word), `retired word still present: ${word}`).toBe(false);
    }
    expect(Object.hasOwn(zhCN, "Back")).toBe(true);
  });
});

const cardNames = Object.values(CATALOG).map((c) => c.name);
// #4115: proper names the client shows that are no catalog card — Heroic Power's powers, Profe
// Yates's old-replay choices, and the Anomaly boards. Each is removed whole, like "Play together",
// so its ordinary words stay flagged in any other label.
const PROPER_NAMES = [
  "Expedition Map",
  "Life Tap",
  "Steady Shot",
  "Cat Cafe",
  "Witness Value",
  "Armor Up",
  "Die Insect",
  "KY Brainstorm",
  "Terminus Tricks",
  "End Times",
  "Tank Up",
  "Wolf's Maw",
  "Iron Mandate",
  "Wrecked Spice Platform",
  "Drone Hideouts",
  "Frontier Outpost",
  // The Infinite Assault's upgrades, by the names players see (docs/infinite-assault.md IA3).
  "Expanded Context",
  "Bigger Dataset",
  "Fine-Tuned",
  "Reinforced Weights",
  "Raised Ceiling",
  "Hardened Shell",
  "Parallel Inference",
];
// The Infinite Assault's enemies and Commanders, by the names its day screen and briefing show
// (`crates/run/src/assault.json`, read through the module).
const ASSAULT_NAMES = ((content) => [
  ...content.doctrines.flatMap((doctrine) => doctrine.enemies),
  ...content.commanders.map((commander) => commander.name),
])(assaultContent());
const PHRASES_TO_REMOVE = [...cardNames, "Play together", ...PROPER_NAMES, ...ASSAULT_NAMES].sort((a, b) => b.length - a.length);

const cardTypes = Object.values(CATALOG).flatMap((c) => (c.type ? c.type.split(/\s+/) : []));
const cardTags = Object.values(CATALOG).flatMap((c) => (c.tags ?? []).flatMap((t) => t.split(/\s+/)));
const glossaryWords = glossary.entries.flatMap((e) =>
  [e.id, e.label, ...(e.aliases ?? [])].flatMap((s) => s.split(/\s+/)),
);

const EXPLICIT_NAMES =
  "JackiOh Jlore Credits Tickets Coins Embers Conquest Attack Defense Position Health Deck Decks Collection Almanac Practice Trios Backrow Legendary Mythic Rare Epic Shard Chase Classic Meditative Stinkyscape Core Prime Training Grounds Vanguard Universe Rift Trade Tributed Greed Opponent Cost Roblox GitHub Chrome Google Authenticator Authy English Chinese Latin Assault".split(
    /\s+/,
  );

const ALLOWED_WORDS = new Set<string>([
  ...cardTypes,
  ...cardTags,
  ...glossaryWords,
  ...EXPLICIT_NAMES,
]);

export function strayCapitals(text: string): string[] {
  let stripped = stripPlaceholders(text);

  for (const phrase of PHRASES_TO_REMOVE) {
    if (stripped.includes(phrase)) {
      stripped = stripped.replaceAll(phrase, " ");
    }
  }

  const clauses = stripped.split(/[.?!:;—–·(),“”]\s*|\s-\s/).map((c) => c.trim()).filter(Boolean);

  const flagged: string[] = [];
  for (const clause of clauses) {
    const words = clause.split(/\s+/).filter(Boolean);
    const laterWords = words.slice(1);
    for (const raw of laterWords) {
      const subwords = raw.split("-");
      for (const sub of subwords) {
        const word = trimWord(sub);
        if (!word || !/^[A-Z]/.test(word)) continue;
        if (/^[A-Z0-9+/]+$/.test(word)) continue;
        if (word.length === 1) continue;

        const noApostropheS = word.endsWith("'s") ? word.slice(0, -2) : word;
        const noS = noApostropheS.endsWith("s") ? noApostropheS.slice(0, -1) : noApostropheS;
        const noEs = noApostropheS.endsWith("es") ? noApostropheS.slice(0, -2) : noApostropheS;

        if (
          ALLOWED_WORDS.has(word) ||
          ALLOWED_WORDS.has(noApostropheS) ||
          ALLOWED_WORDS.has(noS) ||
          ALLOWED_WORDS.has(noEs)
        ) {
          continue;
        }

        flagged.push(word);
      }
    }
  }

  return flagged;
}

const ARROW = /^[←→↑↓]|[←→↑↓]$/u;
