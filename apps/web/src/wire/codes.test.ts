// R191, docs/polish/5-sign-in.md B1-B4 and B16: the client's reading of a typed or pasted code, held to
// the one table the server's Rust reading is held to (`crates/engine/tests/fixtures/code-input-cases.json`,
// read by `crates/engine/src/wire/codes.rs`'s tests too). The client keeps `codes.ts` as TypeScript
// and the server runs its Rust port (docs/v0.3.0/SURFACE.md §10.4), so this table is what keeps the
// two from drifting: a row the server redeems is a row this reading accepts, character for character.
// Finding a code in pasted text (B4), naming the excluded characters and placing the caret (B16) are
// the client's alone, so they are proved here only (#2160).
//
// The format comes from the server's config (`@jackioh/server-config`, generated from
// `crates/server/src/config.rs`, CLAUDE.md rule 9); nothing below restates its length or groups.

import { describe, expect, it } from "vitest";

import { CODE_ALPHABET, INVITE_CODE_FORMAT, ROOM_CODE_FORMAT } from "@jackioh/server-config";

import codeInputCases from "../../../../crates/engine/tests/fixtures/code-input-cases.json";
import {
  canonicalCode,
  excludedCharacters,
  findCodeInText,
  formatCodeCharacters,
  formattedCaret,
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

/** The table's base code, inside R104's alphabet, and the same code grouped. */
const BASE = "ABCDEFGHJKMNPQRS";
const BASE_FORMATTED = "ABCD-EFGH-JKMN-PQRS";

describe("R191 B2 the excluded characters", () => {
  it("R191 B2 excludes exactly 0, 1, I and O from R104's alphabet", () => {
    expect(excludedCharacters(INVITE_CODE_FORMAT)).toEqual(["0", "1", "I", "O"]);
    expect(excludedCharacters(ROOM_CODE_FORMAT)).toEqual(["0", "1", "I", "O"]);
  });
});

describe("R191 B4 findCodeInText", () => {
  const find = (text: string): string | null => findCodeInText(text, FORMAT);

  it("R191 B4 finds the one grouped code in an invite sentence", () => {
    expect(find("Your invite: abcd-efgh-jkmn-pqrs.")).toBe(BASE);
  });

  it("R191 B4 finds a code whose groups are joined by Unicode dashes or line breaks", () => {
    expect(find("Invite \u2192 ABCD\u2013EFGH\u2013JKMN\u2013PQRS")).toBe(BASE);
    expect(find("Your code:\nABCD-EFGH\nJKMN-PQRS\n")).toBe(BASE);
  });

  it("R191 B4 finds a code written with no separators at all", () => {
    expect(find("Here: ABCDEFGHJKMNPQRS!")).toBe(BASE);
  });

  it("R191 B4 finds one code written twice in two spellings", () => {
    expect(find("(ABCD-EFGH-JKMN-PQRS) (abcd efgh jkmn pqrs)")).toBe(BASE);
  });

  it("R191 B4 returns null for a text holding two different codes", () => {
    expect(find("(ABCD-EFGH-JKMN-PQRS) (ABCD-EFGH-JKMN-PQRT)")).toBeNull();
  });

  it("R191 B4 returns null for a text holding no code", () => {
    expect(find("No code here.")).toBeNull();
    expect(find("")).toBeNull();
  });

  it("R191 B4 returns null for a code with an excluded character in it", () => {
    expect(find("Your invite: ABCD-EFGH-JKMN-PQR0.")).toBeNull();
  });

  it("R191 B4 returns null for groups glued to other letters or digits", () => {
    expect(find("XABCD-EFGH-JKMN-PQRS")).toBeNull();
    expect(find("ABCD-EFGH-JKMN-PQRS2")).toBeNull();
  });

  it("R191 B4 never completes a code by upper-casing sharp s to SS", () => {
    expect(find("Code: ABCD-EFGH-JKMN-PQ\u00DF.")).toBeNull();
  });

  it("R191 B4 reads a four-letter word beside a code as a word, not as a group", () => {
    for (const text of [
      "Here ABCD-EFGH-JKMN-PQRS",
      "Take ABCD-EFGH-JKMN-PQRS",
      "HERE ABCD-EFGH-JKMN-PQRS",
      "Your invite code:\nABCD-EFGH-JKMN-PQRS\nHave fun!",
      "ABCD-EFGH-JKMN-PQRS\n\nBest,\nJack",
      "ABCD-EFGH-JKMN-PQRS\n\u2014 Mark",
      "ABCD-EFGH-JKMN-PQRS - see you at the tavern, Jack",
      "Here aBcD-eFgH-jKmN-pQrS",
    ]) {
      expect(find(text), JSON.stringify(text)).toBe(BASE);
    }
  });

  it("R191 B4 never cuts a pasted code with an extra group down to a code it never had", () => {
    for (const text of ["ABCD-EFGH-JKMN-PQRS-T", "abcd efgh jkmn pqrs t", "X-ABCD-EFGH-JKMN-PQRS", "ABCD-EFGH-JKMN-PQRS-0"]) {
      // The server reads each of these as malformed; the paste handler must not disagree.
      expect(canonicalCode(text, FORMAT), JSON.stringify(text)).toBeNull();
      expect(find(text), JSON.stringify(text)).toBeNull();
    }
  });

  it("R191 B4 returns null for two codes on one line, and the code for one code written twice", () => {
    expect(find("ABCD-EFGH-JKMN-PQRS ABCD-EFGH-JKMN-PQRT")).toBeNull();
    expect(find("ABCD-EFGH-JKMN-PQRS ABCD-EFGH-JKMN-PQRS")).toBe(BASE);
  });

  it("R191 B4 finds a code whose first letter a phone keyboard capitalised inside a message", () => {
    for (const text of [
      "Abcd-efgh-jkmn-pqrs is your code",
      "Here it is: Abcd-efgh-jkmn-pqrs",
      "Abcd efgh jkmn pqrs",
      "Your code is Abcd-efgh-jkmn-pqrs, have fun",
    ]) {
      expect(find(text), JSON.stringify(text)).toBe(BASE);
    }
  });

  it("R191 B4 the sentence-case reading only adds a code where the strict one found none", () => {
    // The strict reading already finds exactly this code beside a capitalised word; the looser one
    // must not glue "Here" onto it and lose it.
    expect(find("Here abcd efgh jkmn pqrs")).toBe(BASE);
    // Two codes stay ambiguous under either reading.
    expect(find("Abcd-efgh-jkmn-pqrs or Abcd-efgh-jkmn-pqrt")).toBeNull();
    // An extra group joined on is still one character too long.
    expect(find("Abcd-efgh-jkmn-pqrs-t is it")).toBeNull();
  });
});

// B16: where the caret goes after n characters. The code field puts it back here after every
// accepted change, so typing never lands the caret on the wrong side of a drawn separator.
describe("R191 B16 formattedCaret", () => {
  it("R191 B16 follows the surface formula for every n", () => {
    for (let n = 0; n <= FORMAT.length; n += 1) {
      const expected = n + (n > 0 ? Math.floor((n - 1) / FORMAT.groupSize) : 0);
      expect(formattedCaret(n, FORMAT), `n = ${n}`).toBe(expected);
    }
  });

  it("R191 B16 is the end of the formatted value, so it never sits after a trailing separator", () => {
    for (let n = 0; n <= FORMAT.length; n += 1) {
      expect(formattedCaret(n, FORMAT), `n = ${n}`).toBe(formatCodeCharacters(BASE.slice(0, n), FORMAT).length);
    }
  });

  it("R191 B16 stays inside a group at its edge; after a whole group it precedes the separator", () => {
    const group = FORMAT.groupSize;
    const at = formattedCaret(group, FORMAT);
    expect(BASE_FORMATTED.slice(at, at + 1)).toBe(FORMAT.separator);
    const after = formattedCaret(group + 1, FORMAT) - 1;
    expect(BASE_FORMATTED.slice(after, after + 1)).toBe(BASE.slice(group, group + 1));
  });

  it("R191 B16 counts no separators for a room code, which is one group", () => {
    for (let n = 0; n <= ROOM_CODE_FORMAT.length; n += 1) {
      expect(formattedCaret(n, ROOM_CODE_FORMAT)).toBe(n);
    }
  });
});
