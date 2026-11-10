// R3338000's Terminus cards (fx/terminus/ids.ts): the public ids presentation may read (CLAUDE.md
// rule 7: no planner reads which of them a hidden card is). The pool's order is the engine's
// `TERMINUS_CARDS` order, so a mutant reordering or dropping one fails here.

import { describe, expect, it } from "vitest";

import {
  BIG_CRUNCH,
  CEASELESS_VOID,
  DICE_OF_DOOM,
  EVENT_HORIZON,
  FAREWELL,
  FINAL_DESTINATION,
  KYS_LAST_EXAM,
  LAST_WORD,
  OLIVIAN_EXPANSE,
  TERMINUS_IDS,
  TURN_FOLLOWS,
} from "./ids.ts";

describe("terminus ids", () => {
  it("pins every Terminus card's public id", () => {
    expect(OLIVIAN_EXPANSE).toBe("stinkyscape-004");
    expect(CEASELESS_VOID).toBe("core-100");
    expect(FAREWELL).toBe("stinkyscape-004-1");
    expect(KYS_LAST_EXAM).toBe("stinkyscape-004-2");
    expect(EVENT_HORIZON).toBe("stinkyscape-004-3");
    expect(LAST_WORD).toBe("stinkyscape-004-4");
    expect(FINAL_DESTINATION).toBe("stinkyscape-004-5");
    expect(BIG_CRUNCH).toBe("stinkyscape-004-6");
    expect(DICE_OF_DOOM).toBe("stinkyscape-154");
  });

  it("lists the pool in the engine's TERMINUS_CARDS order", () => {
    expect(TERMINUS_IDS).toEqual([
      CEASELESS_VOID,
      FAREWELL,
      KYS_LAST_EXAM,
      EVENT_HORIZON,
      LAST_WORD,
      FINAL_DESTINATION,
      BIG_CRUNCH,
    ]);
  });

  it("owes a follow-up turn to Event Horizon and Last Word alone", () => {
    expect(TURN_FOLLOWS.has(EVENT_HORIZON)).toBe(true);
    expect(TURN_FOLLOWS.has(LAST_WORD)).toBe(true);
    expect(TURN_FOLLOWS.has(CEASELESS_VOID)).toBe(false);
    expect(TURN_FOLLOWS.has(BIG_CRUNCH)).toBe(false);
    expect(TURN_FOLLOWS.size).toBe(2);
  });
});
