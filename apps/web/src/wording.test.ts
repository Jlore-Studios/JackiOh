// R373: the rules' "library" is shown to players as the Deck, and "sacrifice" as Tribute (v0.1.1).
// Nothing in the client a player reads may say the old words: a label, an aria-label, a tooltip, a
// log line, a prompt, a coach line or a setting.
//
// R432 (v0.2.0): a specific cost is written the way card text writes it, "(N) Cost" as the noun ("a
// (1) Cost or less card", "Face-down trap, (2) Cost") and "costs (N)" as the verb ("costs (1) less").
// So no client string says the old noun "Cost (N)", nor a bare number after "cost" ("costs 3"), nor
// "a 2-cost card".
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

/** R432: the old noun ("Cost (2)"), a bare number after the word cost ("costs 3"), and "2-cost". */
const OLD_COST_WORDS: readonly RegExp[] = [/\bCost \(/, /\bcost(s|ing)? \d/i, /\b\d+-cost\b/i];

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
