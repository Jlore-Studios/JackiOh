// R668: the setup of the free game this device left in progress (resume.ts), read as untrusted input,
// and R765: whether the player left it with Save and leave.

import { afterEach, describe, expect, it, vi } from "vitest";

import {
  PRACTICE_RESUME_STORAGE_KEY,
  clearPracticeResume,
  readPracticeResume,
  readPracticeResumeState,
  writePracticeResume,
} from "./resume.ts";
import type { PracticeStartConfig } from "./protocol.ts";

afterEach(() => {
  vi.restoreAllMocks();
  window.localStorage.clear();
});

const CONFIG: PracticeStartConfig = {
  seed: "abc",
  difficulty: "medium",
  humanSeat: "p2",
  deck: { kind: "saved", index: 2, cards: ["core-001"] },
};

describe("R668 the practice game to resume, on the device", () => {
  it("R668 keeps the setup only (no last board), and clears", () => {
    expect(readPracticeResume()).toBeNull();
    writePracticeResume({ ...CONFIG, lastBoard: [{ defId: "core-008", radiant: false }] });
    expect(readPracticeResume()).toEqual(CONFIG);
    clearPracticeResume();
    expect(readPracticeResume()).toBeNull();
  });

  it("R668 a lesson is never remembered: writing one forgets the free game", () => {
    writePracticeResume(CONFIG);
    writePracticeResume({ ...CONFIG, lesson: "basics" });
    expect(readPracticeResume()).toBeNull();
  });

  it("R668 reads anything malformed as no game", () => {
    for (const raw of [
      "{not json",
      "null",
      JSON.stringify({ ...CONFIG, difficulty: "impossible" }),
      JSON.stringify({ ...CONFIG, humanSeat: "p3" }),
      JSON.stringify({ ...CONFIG, deck: { kind: "saved", index: 1 } }),
      JSON.stringify({ ...CONFIG, lesson: "basics" }),
    ]) {
      window.localStorage.setItem(PRACTICE_RESUME_STORAGE_KEY, raw);
      expect(readPracticeResume(), raw).toBeNull();
    }
  });

  it("R765 keeps where the player left the game: in it, or with Save and leave", () => {
    writePracticeResume(CONFIG);
    expect(readPracticeResumeState()).toEqual({ config: CONFIG, saved: false });
    writePracticeResume(CONFIG, { saved: true });
    expect(readPracticeResumeState()).toEqual({ config: CONFIG, saved: true });
    // The setup alone, whichever: a resume names the game, never where it was left.
    expect(readPracticeResume()).toEqual(CONFIG);
    // Picked up again, the player is in it once more.
    writePracticeResume(CONFIG);
    expect(readPracticeResumeState()).toEqual({ config: CONFIG, saved: false });
    // A lesson is never kept, saved or not.
    writePracticeResume({ ...CONFIG, lesson: "basics" }, { saved: true });
    expect(readPracticeResumeState()).toBeNull();
  });

  it("R765 a game kept before R765 carries no flag and reads as one the player was in (R668)", () => {
    window.localStorage.setItem(PRACTICE_RESUME_STORAGE_KEY, JSON.stringify(CONFIG));
    expect(readPracticeResumeState()).toEqual({ config: CONFIG, saved: false });
  });

  it("R765 a flag that is not true or false is malformed: no game", () => {
    for (const saved of ["yes", 1, null]) {
      window.localStorage.setItem(PRACTICE_RESUME_STORAGE_KEY, JSON.stringify({ ...CONFIG, saved }));
      expect(readPracticeResumeState(), String(saved)).toBeNull();
    }
  });

  it("R668 storage that throws is no game, and a refused write throws nothing", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("full");
    });
    expect(() => {
      writePracticeResume(CONFIG);
    }).not.toThrow();
    expect(readPracticeResume()).toBeNull();
  });
});
