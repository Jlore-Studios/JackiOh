// Reading untrusted JSON (`./json.ts`): a record is a non-null, non-array object, and nothing else.

import { describe, expect, it } from "vitest";

import { isRecord } from "./json.ts";

describe("isRecord", () => {
  it("takes a plain object", () => {
    expect(isRecord({})).toBe(true);
    expect(isRecord({ hand: [] })).toBe(true);
  });

  it("refuses null, undefined and every primitive", () => {
    expect(isRecord(null)).toBe(false);
    expect(isRecord(undefined)).toBe(false);
    expect(isRecord("{\"a\":1}")).toBe(false);
    expect(isRecord(0)).toBe(false);
    expect(isRecord(false)).toBe(false);
  });

  it("refuses an array, even an empty one", () => {
    expect(isRecord([])).toBe(false);
    expect(isRecord([{ a: 1 }])).toBe(false);
  });
});
