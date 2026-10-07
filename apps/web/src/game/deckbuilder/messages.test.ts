// SPEC §9.4: "one validator module shared by client and server". This file is the grep that keeps
// it one.
//
// The fragments are not typed out here either — they are extracted from the validator itself,
// `crates/engine/src/validator.rs` (the Rust port the client calls through the WebAssembly module,
// `src/wire/validator.ts`), at test time, by pulling the literal chunks out of its string literals.
// So the test tracks the validator: reword an L-rule message there and this test looks for the new
// wording, without anyone editing this file. If a client source ever contains one of those chunks,
// there are two sources of truth for that sentence and the test fails naming both.
//
// BUILD M6-T3's acceptance already asks for a grep test ("`apps/web` and `apps/server` both import
// from `@jackioh/validator`"); this is its mirror image, and both are needed: importing the module
// is worth nothing if the screen also carries its own copy of the words.

import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const here = dirname(fileURLToPath(import.meta.url));
const WEB_SRC = resolve(here, "../..");
const VALIDATOR = resolve(here, "../../../../../crates/engine/src/validator.rs");

/** This file quotes the fragments in its failure messages, so it is the one file exempt. */
const SELF = resolve(here, "messages.test.ts");

/**
 * Rust source in one pass: a comment (whose quotes are prose, not messages), a char literal (whose
 * `'"'` would open a string that is not there), or a string literal, whose body is group 1.
 */
const RUST_TOKENS = /\/\/[^\n]*|\/\*[\s\S]*?\*\/|'(?:[^'\\\n]|\\.)'|"((?:[^"\\]|\\.)*)"/g;

/** A literal's text as the program sees it: `\"` is a quote, `\\` a backslash. */
function unescaped(text: string): string {
  return text.replace(/\\(["'\\])/g, "$1");
}

/**
 * The literal chunks of every message the validator builds: each string literal (a `format!`
 * string or a plain one), split on its `{…}` holes, keeping the pieces long enough to be prose
 * rather than punctuation. A Rust message is one literal where TypeScript's was several template
 * literals joined, so a piece can begin or end on the punctuation that joins it to a hole
 * (`; at most`): that punctuation is not counted towards the length, though it stays in the piece.
 */
function validatorFragments(): string[] {
  const source = readFileSync(VALIDATOR, "utf8");
  const literals = [...source.matchAll(RUST_TOKENS)].flatMap((match) => (match[1] === undefined ? [] : [unescaped(match[1])]));
  const fragments = new Set<string>();
  for (const literal of literals) {
    for (const piece of literal.split(/\{[^{}]*\}/)) {
      const trimmed = piece.trim();
      const prose = trimmed.replace(/^[\s;:,."()]+|[\s;:,."()]+$/g, "");
      // Long enough to be a sentence fragment (`but you own`, `appears in`), and two words.
      if (prose.length < 9) continue;
      if (!/[A-Za-z]\s[A-Za-z]/.test(prose)) continue;
      fragments.add(trimmed);
    }
  }
  return [...fragments].sort();
}

function sourceFiles(root: string): string[] {
  const found: string[] = [];
  const walk = (directory: string): void => {
    for (const entry of readdirSync(directory)) {
      const path = join(directory, entry);
      if (statSync(path).isDirectory()) {
        walk(path);
        continue;
      }
      if (/\.tsx?$/.test(entry)) found.push(path);
    }
  };
  walk(root);
  return found;
}

describe("no L1–L6 sentence is written anywhere in the client", () => {
  const fragments = validatorFragments();

  it("finds the validator's own message fragments to look for", () => {
    // One per rule at least, or the scan below would be looking for nothing.
    expect(fragments.length, `fragments found in ${VALIDATOR}`).toBeGreaterThanOrEqual(12);
  });

  it("finds none of them in any file under apps/web/src", () => {
    const offences: string[] = [];
    for (const file of sourceFiles(WEB_SRC)) {
      if (file === SELF) continue;
      const text = readFileSync(file, "utf8");
      for (const fragment of fragments) {
        if (text.includes(fragment)) {
          offences.push(`${relative(WEB_SRC, file)} contains ${JSON.stringify(fragment)}`);
        }
      }
    }
    expect(
      offences,
      `a validator sentence has a second copy in the client:\n  ${offences.join("\n  ")}`,
    ).toEqual([]);
  });

  it("would catch a copy, so the scan is not vacuous", () => {
    // The scan's own control: the first fragment really is findable in the validator's source.
    const source = unescaped(readFileSync(VALIDATOR, "utf8"));
    expect(source).toContain(fragments[0] ?? "");
  });
});
