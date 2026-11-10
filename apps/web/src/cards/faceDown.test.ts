// The face-down words (cards/faceDown.ts, SPEC §10.10 R370, R371, R1046): the one module the
// board, the inspect overlays, the showcase and the log read, so every surface says the same thing.
// A mutant rewording, mistiming or un-costing a back fails here.

import { describe, expect, it } from "vitest";

import {
  costPhrase,
  FACE_DOWN_HINT,
  FACE_DOWN_TAG,
  FACE_DOWN_TITLE,
  faceDownLabel,
  PLAGUE_COUNT_WORDS,
  REVEAL_AT_LABEL,
  REVEAL_AT_NOTE,
  UNREVEALED_NOTE,
} from "./faceDown.ts";

describe("face-down words", () => {
  it("R371 tags your own face-down trap, and notes what the opponent cannot see", () => {
    expect(FACE_DOWN_TAG).toBe("Face down");
    expect(UNREVEALED_NOTE).toBe("Face down — your opponent can't see this card");
    expect(FACE_DOWN_TITLE).toBe("Face-down trap");
  });

  it("R370 hints the spring, and R471 states the counters with the count in place", () => {
    expect(FACE_DOWN_HINT).toBe(
      "Only the player who set it can see what it is. It springs by itself when its condition is met.",
    );
    expect(PLAGUE_COUNT_WORDS).toBe("Plague Counters: {count}");
  });

  it("R1046 names each reveal timing in the picker's D3 words, Cloaked by its own reveal", () => {
    expect(REVEAL_AT_LABEL).toEqual({
      endOfThisTurn: "End of your turn",
      startOfNextTurn: "Start of your next turn",
      endOfNextTurn: "End of your next turn",
      cloaked: "When you reveal it",
    });
    expect(REVEAL_AT_NOTE).toBe("Reveals — your opponent can't see when");
  });

  it("R432 writes a cost as card text does, and hides it when the view gives none", () => {
    expect(costPhrase(2)).toBe("(2) Cost");
    expect(faceDownLabel(undefined)).toBe("Face-down trap");
    expect(faceDownLabel(2)).toBe("Face-down trap, (2) Cost");
  });
});
