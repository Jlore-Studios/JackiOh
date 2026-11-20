// Reading untrusted JSON (`./json.ts`): a record is a non-null, non-array object, and nothing else.

import { describe, expect, it } from "vitest";

import { isRecord, isSafeKey, isStringArray } from "./json.ts";

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

describe("isSafeKey", () => {
  it("isSafeKey refuses proto keys", () => {
    expect(isSafeKey("__proto__")).toBe(false);
    expect(isSafeKey("constructor")).toBe(false);
    expect(isSafeKey("prototype")).toBe(false);
    expect(isSafeKey("p1")).toBe(true);
    expect(isSafeKey("card-1")).toBe(true);
  });
});

describe("isStringArray", () => {
  it("isStringArray holds only an array of strings", () => {
    expect(isStringArray([])).toBe(true);
    expect(isStringArray(["a"])).toBe(true);
    expect(isStringArray(["a", 1])).toBe(false);
    expect(isStringArray("a")).toBe(false);
    expect(isStringArray(null)).toBe(false);
    expect(isStringArray({})).toBe(false);
  });
});
