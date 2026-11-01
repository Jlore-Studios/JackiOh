// Checks apps/web/WRITING.md's machine-checkable rules. A player's sentence is a string
// or JSX text in a non-test source that zh-CN.json translates.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import ts from "typescript";
import { describe, expect, it } from "vitest";

import zhCN from "./i18n/zh-CN.json";

const SRC = dirname(fileURLToPath(import.meta.url));

function sources(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === "test" ? [] : sources(path);
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

function playerTexts(): { at: string; text: string }[] {
  return sources(SRC).flatMap(wordsIn).filter(({ text }) => Object.hasOwn(zhCN, text) || Object.hasOwn(zhCN, text.trim()));
}

function jargonIn(text: string): string[] {
  return JARGON.filter(({ pattern }) => pattern.test(text)).map(({ word }) => word);
}

function marksIn(text: string): string[] {
  return MARKS.filter(({ pattern }) => pattern.test(text)).map(({ word }) => word);
}

describe("WRITING.md the client's words for players", () => {
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
});
