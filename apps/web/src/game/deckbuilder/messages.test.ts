// SPEC §9.4 / BUILD M6-T3: Extract validator fragments from Rust so duplicate client messages fail.

import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const here = dirname(fileURLToPath(import.meta.url));
const WEB_SRC = resolve(here, "../..");
const VALIDATOR = resolve(here, "../../../../../crates/engine/src/validator.rs");

/** This file quotes the fragments in its failure messages, so it is the one file exempt. */
const SELF = resolve(here, "messages.test.ts");

/** Captures Rust string bodies without mistaking comments or character literals for strings. */
const RUST_TOKENS = /\/\/[^\n]*|\/\*[\s\S]*?\*\/|'(?:[^'\\\n]|\\.)'|"((?:[^"\\]|\\.)*)"/g;

function unescaped(text: string): string {
  return text.replace(/\\(["'\\])/g, "$1");
}

/** Extracts readable validator fragments, splitting format holes to detect client copies. */
function validatorFragments(): string[] {
  const source = readFileSync(VALIDATOR, "utf8");
  const literals = [...source.matchAll(RUST_TOKENS)].flatMap((match) => (match[1] === undefined ? [] : [unescaped(match[1])]));
  const fragments = new Set<string>();
  for (const literal of literals) {
    for (const piece of literal.split(/\{[^{}]*\}/)) {
      const trimmed = piece.trim();
      const prose = trimmed.replace(/^[\s;:,."()]+|[\s;:,."()]+$/g, "");
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
    const source = unescaped(readFileSync(VALIDATOR, "utf8"));
    expect(source).toContain(fragments[0] ?? "");
  });
});
