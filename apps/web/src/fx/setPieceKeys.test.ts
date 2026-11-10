// R2800000's set-piece keys (fx/setPieceKeys.ts): the names a card's effects table may give a
// set piece, apart from their drawing. A mutant accepting a wrong key, or dropping one of the
// eleven, fails here.

import { describe, expect, it } from "vitest";

import { isSetPieceKey, SET_PIECE_KEYS } from "./setPieceKeys.ts";

describe("set piece keys", () => {
  it("names the eleven set pieces: the clears, Olivian Expanse's rift, the six tokens, Dice of Doom", () => {
    expect(SET_PIECE_KEYS).toEqual([
      "nether",
      "voidFold",
      "vortex",
      "terminusRift",
      "farewell",
      "lastExam",
      "eventHorizon",
      "lastWord",
      "finalDestination",
      "bigCrunch",
      "diceOfDoom",
    ]);
  });

  it("accepts every key, and nothing else", () => {
    for (const key of SET_PIECE_KEYS) expect(isSetPieceKey(key)).toBe(true);
    expect(isSetPieceKey("vortex")).toBe(true);
    expect(isSetPieceKey("terminus")).toBe(false);
    expect(isSetPieceKey("")).toBe(false);
  });
});
