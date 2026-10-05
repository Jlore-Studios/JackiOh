// R660: the flavour sidecar (`flavour.json`, src/flavour.ts) against the catalog. Its keys are
// catalog ids and nothing else, every card and token has a line, and each entry is plain words under
// its caps. Whether a line speaks rules words is the client's test (apps/web/src/cards/flavour.test.ts),
// beside the list the voice lines answer to (issue #115).

import { describe, expect, it } from "vitest";
import { CATALOG } from "../src/catalog-data";
import { ARTIST_MAX_CHARS, FLAVOUR, FLAVOUR_MAX_CHARS } from "../src/flavour";

const FIELDS: readonly string[] = ["flavour", "artist"];

/** A string the sidecar may hold: trimmed, non-empty, on one line, and no longer than `cap`. */
function problemWith(value: unknown, cap: number): string | null {
  if (typeof value !== "string") return "is not a string";
  if (value.trim() !== value) return "has space at an end";
  if (value === "") return "is empty";
  if (/[\r\n]/.test(value)) return "spans lines";
  if (value.length > cap) return `is ${String(value.length)} characters, over ${String(cap)}`;
  return null;
}

describe("R660 the flavour sidecar", () => {
  it("R660 every key is a catalog card or token", () => {
    const strangers = Object.keys(FLAVOUR).filter((id) => !Object.hasOwn(CATALOG, id));
    expect(strangers).toEqual([]);
  });

  it("R660 every card and token has a flavour line", () => {
    const without = Object.keys(CATALOG).filter((id) => FLAVOUR[id]?.flavour === undefined);
    expect(without).toEqual([]);
  });

  it("R660 an entry carries only a flavour line and an artist, each plain words under its cap", () => {
    const problems: string[] = [];
    for (const [id, entry] of Object.entries(FLAVOUR)) {
      for (const field of Object.keys(entry)) {
        if (!FIELDS.includes(field)) problems.push(`${id}: unknown field "${field}"`);
      }
      if (entry.flavour !== undefined) {
        const problem = problemWith(entry.flavour, FLAVOUR_MAX_CHARS);
        if (problem !== null) problems.push(`${id}: flavour ${problem}`);
      }
      if (entry.artist !== undefined) {
        const problem = problemWith(entry.artist, ARTIST_MAX_CHARS);
        if (problem !== null) problems.push(`${id}: artist ${problem}`);
      }
    }
    expect(problems).toEqual([]);
  });

  it("R660 the caps hold the check: a line one character over is refused", () => {
    expect(problemWith("x".repeat(FLAVOUR_MAX_CHARS), FLAVOUR_MAX_CHARS)).toBeNull();
    expect(problemWith("x".repeat(FLAVOUR_MAX_CHARS + 1), FLAVOUR_MAX_CHARS)).toMatch(/over/);
    expect(problemWith(" padded", FLAVOUR_MAX_CHARS)).toMatch(/space/);
    expect(problemWith("two\nlines", FLAVOUR_MAX_CHARS)).toMatch(/lines/);
    expect(problemWith("", FLAVOUR_MAX_CHARS)).toMatch(/empty/);
    expect(problemWith(7, FLAVOUR_MAX_CHARS)).toMatch(/string/);
  });
});
