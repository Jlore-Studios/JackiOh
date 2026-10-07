// R191, docs/polish/5-sign-in.md B1-B4: the client's reading of a typed or pasted code, held to the
// one table the server's Rust reading is held to (`crates/engine/tests/fixtures/code-input-cases.json`,
// read by `crates/engine/src/wire/codes.rs`'s tests too). The client keeps `codes.ts` as TypeScript
// and the server runs its Rust port (docs/v0.3.0/SURFACE.md §10.4), so this table is what keeps the
// two from drifting: a row the server redeems is a row this reading accepts, character for character.
//
// The format comes from the server's config (`@jackioh/server-config`, generated from
// `crates/server/src/config.rs`, CLAUDE.md rule 9); nothing below restates its length or groups.

import { describe, expect, it } from "vitest";

import { CODE_ALPHABET, INVITE_CODE_FORMAT } from "@jackioh/server-config";

import codeInputCases from "../../../../crates/engine/tests/fixtures/code-input-cases.json";
import {
  canonicalCode,
  findCodeInText,
  formatCodeCharacters,
  readCodeInput,
  type CodeInputProblem,
} from "./codes.ts";

/** One row: every column is what the shared reading gives for `INVITE_CODE_FORMAT`. */
type CodeInputCase = {
  readonly name: string;
  readonly input: string;
  /** canonicalCode(input). The server redeems a code minted as this; null for a malformed input. */
  readonly canonical: string | null;
  /** readCodeInput(input).formatted: the characters accepted before the first problem, grouped. */
  readonly formatted: string;
  readonly problem: CodeInputProblem["kind"] | null;
  /** The character an excluded or foreign problem names. */
  readonly character?: string;
  /** findCodeInText(input) !== null. */
  readonly foundInText: boolean;
};

const CODE_INPUT_CASES = codeInputCases as readonly CodeInputCase[];

const FORMAT = INVITE_CODE_FORMAT;

/** The problem a row expects, in the exact shape `CodeInputProblem` declares. */
function expectedProblem(row: CodeInputCase): CodeInputProblem | null {
  if (row.problem === null) return null;
  if (row.problem === "tooLong") return { kind: "tooLong" };
  return { kind: row.problem, character: row.character ?? "" };
}

/** `formatted` with the separators taken out: the characters the reading accepted. */
function charactersOf(formatted: string): string {
  return formatted.split(FORMAT.separator).join("");
}

describe("R191 the client reads the shared code-input table as the server does (B1-B4)", () => {
  it("R191 B1 the table is the shared one, and every canonical code in it is a whole code of R104's alphabet", () => {
    expect(CODE_INPUT_CASES.length).toBeGreaterThan(0);
    const canonicals = CODE_INPUT_CASES.flatMap((row) => (row.canonical === null ? [] : [row.canonical]));
    expect(canonicals.length).toBeGreaterThan(0);
    for (const code of canonicals) {
      expect(code).toHaveLength(FORMAT.length);
      for (const character of code) expect(CODE_ALPHABET).toContain(character);
    }
    // The rows name their problems, so a row that reads as a code must not also claim one.
    for (const row of CODE_INPUT_CASES) {
      if (row.canonical !== null) expect(row.problem, row.name).toBeNull();
    }
  });

  for (const row of CODE_INPUT_CASES) {
    it(`R191 B1 canonicalCode reads: ${row.name}`, () => {
      expect(canonicalCode(row.input, FORMAT)).toBe(row.canonical);
    });

    it(`R191 B2 readCodeInput reports: ${row.name}`, () => {
      const reading = readCodeInput(row.input, FORMAT);

      expect(reading.formatted).toBe(row.formatted);
      expect(reading.characters).toBe(charactersOf(row.formatted));
      expect(reading.problem).toEqual(expectedProblem(row));
      // "problem === null && characters.length === length"
      expect(reading.complete).toBe(row.canonical !== null);
      expect(reading.characters.length).toBeLessThanOrEqual(FORMAT.length);
      expect(formatCodeCharacters(reading.characters, FORMAT)).toBe(reading.formatted);
    });

    it(`R191 B4 findCodeInText: ${row.name}`, () => {
      const found = findCodeInText(row.input, FORMAT);

      expect(found !== null).toBe(row.foundInText);
      if (row.canonical !== null) expect(found).toBe(row.canonical);
      // Whatever it finds is a whole code, never a fragment.
      if (found !== null) expect(canonicalCode(found, FORMAT)).toBe(found);
    });
  }
});
