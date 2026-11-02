// R2321001: Training Grounds #15 In a hurry's Rush Tokens wear the engine's `endOfTurn` mark while
// their end-of-turn destroy waits (R437's reusable mark). The client knows its words and color, so the
// badge, tooltip and inspect note never fall back to the generic "an effect is waiting".

import { describe, expect, it } from "vitest";

import { END_OF_TURN_MARK, END_OF_TURN_MARK_COLOR, MARK_WORDS, UNKNOWN_MARK_WORDS, markColorOf, markWords } from "./marks.ts";

describe("R2321001 the end-of-turn mark", () => {
  it("R2321001 names the engine's mark and color, and says the card goes at the end of this turn", () => {
    expect(END_OF_TURN_MARK).toBe("endOfTurn");
    expect(markColorOf("orange")).toBe(END_OF_TURN_MARK_COLOR);
    expect(markWords(END_OF_TURN_MARK)).not.toBe(UNKNOWN_MARK_WORDS);
    expect(markWords(END_OF_TURN_MARK)).toEqual({ name: "End of turn", text: "Marked: destroyed at the end of this turn" });
    expect(MARK_WORDS[END_OF_TURN_MARK]?.text).not.toContain("next turn");
  });
});
