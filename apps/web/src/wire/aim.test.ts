// The wire aim contract's validation (./aim.ts, R738): only public ends rebuild, zone lanes start
// at 1, a hand index at 0, and a hand is never an aim target. One `it` per rule, with real values.

import { describe, expect, it } from "vitest";

import { aimKey, parseAim, parseAimEnd } from "./aim.ts";

describe("parseAimEnd", () => {
  it("rebuilds a hero end", () => {
    expect(parseAimEnd({ at: "hero", player: "p2" })).toEqual({ at: "hero", player: "p2" });
  });

  it("rebuilds a zone end whose lane starts at 1", () => {
    expect(parseAimEnd({ at: "zone", player: "p1", row: "backrow", lane: 1 })).toEqual({
      at: "zone",
      player: "p1",
      row: "backrow",
      lane: 1,
    });
  });

  it("refuses a zone end in lane 0", () => {
    expect(parseAimEnd({ at: "zone", player: "p1", row: "units", lane: 0 })).toBeNull();
  });

  it("refuses a zone end with a row that is not a row", () => {
    expect(parseAimEnd({ at: "zone", player: "p1", row: "hand", lane: 1 })).toBeNull();
  });

  it("rebuilds a hand end at index 0", () => {
    expect(parseAimEnd({ at: "hand", player: "p1", index: 0 })).toEqual({ at: "hand", player: "p1", index: 0 });
  });

  it("refuses an end for a player that is not a seat", () => {
    expect(parseAimEnd({ at: "hero", player: "p3" })).toBeNull();
  });

  it("refuses what is not an object and an end it does not know", () => {
    expect(parseAimEnd(null)).toBeNull();
    expect(parseAimEnd({ at: "deck", player: "p1" })).toBeNull();
  });
});

describe("parseAim", () => {
  const source = { at: "zone", player: "p1", row: "units", lane: 3 };

  it("reads a finished aim as null", () => {
    expect(parseAim(null)).toBeNull();
  });

  it("reads invalid input as undefined", () => {
    expect(parseAim(42)).toBeUndefined();
    expect(parseAim({ source: { at: "hero", player: "p3" }, target: null })).toBeUndefined();
  });

  it("reads an aim off a valid landing spot with its source", () => {
    expect(parseAim({ source, target: null })).toEqual({
      source: { at: "zone", player: "p1", row: "units", lane: 3 },
      target: null,
    });
  });

  it("reads an aim onto a hero and onto a zone", () => {
    expect(parseAim({ source, target: { at: "hero", player: "p2" } })).toEqual({
      source: { at: "zone", player: "p1", row: "units", lane: 3 },
      target: { at: "hero", player: "p2" },
    });
  });

  it("never reads a hand as an aim target", () => {
    expect(parseAim({ source, target: { at: "hand", player: "p2", index: 0 } })).toBeUndefined();
  });

  it("reads an invalid target as undefined", () => {
    expect(parseAim({ source, target: { at: "zone", player: "p2", row: "units", lane: 0 } })).toBeUndefined();
  });
});

describe("aimKey", () => {
  it("keys no aim as none", () => {
    expect(aimKey(null)).toBe("none");
  });

  it("keys an aim off a landing spot with a none target", () => {
    expect(aimKey({ source: { at: "hero", player: "p1" }, target: null })).toBe("hero:p1>none");
  });

  it("keys both ends with their public fields", () => {
    expect(
      aimKey({
        source: { at: "zone", player: "p1", row: "units", lane: 3 },
        target: { at: "hand", player: "p2", index: 1 },
      }),
    ).toBe("zone:p1:units:3>hand:p2:1");
  });
});
