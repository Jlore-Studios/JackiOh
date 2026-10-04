// R373: the rules' "library" is shown to players as the Deck, and "sacrifice" as Tribute (v0.1.1).
// Nothing in the client a player reads may say the old words: a label, an aria-label, a tooltip, a
// log line, a prompt, a coach line or a setting.
//
// R432 (v0.2.0): a specific cost is written the way card text writes it, "(N) Cost" as the noun ("a
// (1) Cost or less card", "Face-down trap, (2) Cost") and "costs (N)" as the verb ("costs (1) less").
// So no client string says the old noun "Cost (N)", the old participle "costing (N)", nor a bare
// number after "cost" ("costs 3"), nor "a 2-cost card".
//
// The guard reads every source file under src/ (tests aside) and parses it, so comments — which
// name the rules' library freely, as SPEC does — are not text. Of what is left, a machine word is
// never read by a player (a zone kind, "library"; a testid, `library-you`; a data value,
// "libraryFull"), and none of those has a space in it. So the guard flags a string, a template's
// words or JSX text that holds one of the old words and a space: words a player could read.
//
// One file is read another way: cards/glossary.ts keeps SPEC's rule text verbatim and puts it into
// players' words as it builds the table (`inPlayerWords`), and rules.test.ts proves the table.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import ts from "typescript";
import { describe, expect, it } from "vitest";

import { costPhrase, faceDownLabel } from "./cards/faceDown.ts";

const SRC = dirname(fileURLToPath(import.meta.url));

const OLD_WORDS = /\b(librar(y|ies)|sacrific\w*)\b/i;

/** SPEC's words, converted where they are read (see the header). */
const CONVERTED = new Set(["cards/glossary.ts"]);

function sources(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === "test" ? [] : sources(path);
    return /\.(ts|tsx)$/.test(name) && !/\.test\.tsx?$/.test(name) ? [path] : [];
  });
}

/** The words of every string, template part and JSX text in one file, with where each stands. */
function wordsIn(path: string): { at: string; text: string }[] {
  const text = readFileSync(path, "utf8");
  const file = ts.createSourceFile(path, text, ts.ScriptTarget.Latest, true, path.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS);
  const out: { at: string; text: string }[] = [];
  const visit = (node: ts.Node): void => {
    // A module path is no text.
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

describe("R373 players read Deck and Tribute", () => {
  it("R373 no text a player can read in the client says library or sacrifice", () => {
    const files = sources(SRC).filter((path) => !CONVERTED.has(relative(SRC, path)));
    expect(files.length).toBeGreaterThan(100);
    const found = files
      .flatMap(wordsIn)
      .filter(({ text }) => OLD_WORDS.test(text) && /\s/.test(text.trim()))
      .map(({ at, text }) => `${at}: ${JSON.stringify(text)}`);
    expect(found).toEqual([]);
  });

  it("R373 the guard does flag the old words where a player would read them", () => {
    const flagged = (text: string): boolean => OLD_WORDS.test(text) && /\s/.test(text.trim());
    expect(flagged("Your library")).toBe(true);
    expect(flagged("Library full")).toBe(true);
    expect(flagged("That unit is sacrificed to pay for it.")).toBe(true);
    // Machine words stay: a zone kind, a testid, a data value.
    expect(flagged("library")).toBe(false);
    expect(flagged("library-you")).toBe(false);
    expect(flagged("libraryFull")).toBe(false);
    expect(flagged("Your deck")).toBe(false);
  });
});

/** R432: the old noun ("Cost (2)"), the old participle ("costing (2)"), a bare number after the word cost ("costs 3"), and "2-cost". */
const OLD_COST_WORDS: readonly RegExp[] = [/\bCost \(/, /\bcosting \(/i, /\bcost(s|ing)? \d/i, /\b\d+-cost\b/i, /\bcosts? $/i];

function oldCostWords(text: string): boolean {
  return OLD_COST_WORDS.some((pattern) => pattern.test(text));
}

describe("R432 a cost is \"(N) Cost\" as a noun and \"costs (N)\" as a verb", () => {
  it("R432 no text a player can read in the client writes a cost the old way", () => {
    const files = sources(SRC);
    expect(files.length).toBeGreaterThan(100);
    const found = files
      .flatMap(wordsIn)
      .filter(({ text }) => oldCostWords(text))
      .map(({ at, text }) => `${at}: ${JSON.stringify(text)}`);
    expect(found).toEqual([]);
  });

  it("R432 the guard flags the old ways and passes the new ones", () => {
    expect(oldCostWords("Face-down trap, Cost (2)")).toBe(true);
    expect(oldCostWords("Discover 2 Cost (2) or less Units")).toBe(true);
    expect(oldCostWords("Jlockeed Shredder-10 costs 3.")).toBe(true);
    expect(oldCostWords("a 2-cost unit")).toBe(true);
    expect(oldCostWords("a card costing 1 or less")).toBe(true);
    expect(oldCostWords("Add 3 random cards costing (0)")).toBe(true);
    // A template that writes the number bare after "costs" ("costs ${n}") ends its text there.
    expect(oldCostWords(", costs ")).toBe(true);
    expect(oldCostWords("Face-down trap, (2) Cost")).toBe(false);
    expect(oldCostWords("Discover 2 (2) Cost or less Units")).toBe(false);
    expect(oldCostWords("cards cost (1) less")).toBe(false);
    expect(oldCostWords("It costs (0).")).toBe(false);
  });

  it("R432 the face-down trap's label reads \"Face-down trap, (2) Cost\"", () => {
    expect(faceDownLabel(2)).toBe("Face-down trap, (2) Cost");
    expect(costPhrase(0)).toBe("(0) Cost");
  });
});

/** Vocabulary table (patch v0.2.4, issue #45): retired words and variants. */
const RETIRED_VOCABULARY: readonly { name: string; pattern: RegExp }[] = [
  { name: "bounce", pattern: /\bbounce(s|d)?\b/i },
  { name: "backrow zone", pattern: /\bbackrow zone\b/i },
  { name: "cost 1 less", pattern: /\bcosts? 1 less\b/i },
  { name: "turn trigger", pattern: /\b(at the (start|end)( and end)? of your turn|(start|end) of your turn)\b/i },
  { name: "End your turn", pattern: /\bEnd your turn\b/ },
  { name: "Start of Game", pattern: /\bStart of Game\b/ },
  { name: "Once per Turn", pattern: /\bOnce per Turn\b/ },
  { name: "that costs (N)", pattern: /\bthat costs \(/i },
  { name: "Cost (N)", pattern: /\bCost \(/ },
  { name: "costing (N)", pattern: /\bcosting \(/i },
  { name: "Trigger the Cry", pattern: /\bTrigger the Cry\b/i },
  { name: "Set a hero's health", pattern: /\bSet a hero's health\b/i },
  { name: "Cannot be in Defense Position", pattern: /\bCannot be in Defense Position\b/i },
];

describe("patch v0.2.4 vocabulary table (SPEC §11 R366)", () => {
  it("no tutorial script line uses a word the vocabulary table retired", () => {
    const tutorialFiles = sources(join(SRC, "tutorial/scripts"));
    expect(tutorialFiles.length).toBeGreaterThan(0);
    const words = tutorialFiles.flatMap(wordsIn);
    for (const { name, pattern } of RETIRED_VOCABULARY) {
      const found = words.filter(({ text }) => pattern.test(text)).map(({ at, text }) => `${at}: ${JSON.stringify(text)}`);
      expect(found, `tutorial scripts should not say ${name}`).toEqual([]);
    }
  });

  it("the retired vocabulary guard catches retired terms", () => {
    expect(RETIRED_VOCABULARY.some(({ pattern }) => pattern.test("Bounce that unit"))).toBe(true);
    expect(RETIRED_VOCABULARY.some(({ pattern }) => pattern.test("in the backrow zone"))).toBe(true);
    expect(RETIRED_VOCABULARY.some(({ pattern }) => pattern.test("cost 1 less"))).toBe(true);
    expect(RETIRED_VOCABULARY.some(({ pattern }) => pattern.test("Start of your turn: Draw 1."))).toBe(true);
    expect(RETIRED_VOCABULARY.some(({ pattern }) => pattern.test("At the end of your turn, draw 1."))).toBe(true);
    expect(RETIRED_VOCABULARY.some(({ pattern }) => pattern.test("End your turn."))).toBe(true);
    expect(RETIRED_VOCABULARY.some(({ pattern }) => pattern.test("Start of Game: Draw 1."))).toBe(true);
    expect(RETIRED_VOCABULARY.some(({ pattern }) => pattern.test("Once per Turn: Draw 1."))).toBe(true);
    expect(RETIRED_VOCABULARY.some(({ pattern }) => pattern.test("a Spell that costs (1)"))).toBe(true);
    expect(RETIRED_VOCABULARY.some(({ pattern }) => pattern.test("Face-down trap, Cost (2)"))).toBe(true);
    expect(RETIRED_VOCABULARY.some(({ pattern }) => pattern.test("a card costing (1) or less"))).toBe(true);
    expect(RETIRED_VOCABULARY.some(({ pattern }) => pattern.test("Trigger the Cry of a Unit"))).toBe(true);
    expect(RETIRED_VOCABULARY.some(({ pattern }) => pattern.test("Set a hero's health to 13."))).toBe(true);
    expect(RETIRED_VOCABULARY.some(({ pattern }) => pattern.test("Cannot be in Defense Position."))).toBe(true);
    expect(RETIRED_VOCABULARY.some(({ pattern }) => pattern.test("costs (1) less"))).toBe(false);
    expect(RETIRED_VOCABULARY.some(({ pattern }) => pattern.test("in the backrow"))).toBe(false);
    expect(RETIRED_VOCABULARY.some(({ pattern }) => pattern.test("return that unit to hand"))).toBe(false);
    expect(RETIRED_VOCABULARY.some(({ pattern }) => pattern.test("Start of turn: Draw 1."))).toBe(false);
    expect(RETIRED_VOCABULARY.some(({ pattern }) => pattern.test("End the turn."))).toBe(false);
    expect(RETIRED_VOCABULARY.some(({ pattern }) => pattern.test("Mr. Vanilla costs (1)"))).toBe(false);
    expect(RETIRED_VOCABULARY.some(({ pattern }) => pattern.test("at the end of each of your turns"))).toBe(false);
  });
});

