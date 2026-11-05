// R2777003: a practice game counts in the player's own statistics only against Easy.

import { describe, expect, it } from "vitest";

import { DIFFICULTIES } from "@jackioh/engine/config";

import { talliesPractice } from "./tally.ts";

describe("which practice games the player's statistics count", () => {
  it("R2777003 counts a game against Easy and none against Medium or Hard", () => {
    expect(DIFFICULTIES.filter((difficulty) => talliesPractice({ difficulty }))).toEqual(["easy"]);
  });

  it("R2777003 still counts no lesson or sandbox game, even against Easy", () => {
    expect(talliesPractice({ difficulty: "easy", lesson: { id: "lesson-1" } })).toBe(false);
    expect(talliesPractice({ difficulty: "easy", sandbox: true })).toBe(false);
    expect(talliesPractice({ difficulty: "easy", sandbox: false })).toBe(true);
  });
});
