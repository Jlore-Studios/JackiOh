// The shared wire's runtime shape (`./index.ts`, SURFACE §10.4): the event table the
// animation and sound tables test against, and the hand-kept zone helpers.

import { describe, expect, it } from "vitest";

import { GAME_EVENT_TYPES } from "./index.ts";

describe("GAME_EVENT_TYPES", () => {
  it("holds the engine's order, starting where the engine starts", () => {
    expect(GAME_EVENT_TYPES[0]).toBe("cardPlayed");
    expect(GAME_EVENT_TYPES).toContain("cardPlayed");
    expect(GAME_EVENT_TYPES).toContain("emoted");
    expect(GAME_EVENT_TYPES).toContain("gameOver");
    expect(GAME_EVENT_TYPES).toContain("rolled");
    expect(GAME_EVENT_TYPES).toContain("hookResolving");
  });

  it("names every type once", () => {
    expect(new Set(GAME_EVENT_TYPES).size).toBe(GAME_EVENT_TYPES.length);
  });
});
