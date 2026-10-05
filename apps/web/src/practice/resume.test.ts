// R662: the setup of the free game this device left in progress (resume.ts), read as untrusted input.

import { afterEach, describe, expect, it, vi } from "vitest";

import { PRACTICE_RESUME_STORAGE_KEY, clearPracticeResume, readPracticeResume, writePracticeResume } from "./resume.ts";
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

describe("R662 the practice game to resume, on the device", () => {
  it("R662 keeps the setup only (no last board), and clears", () => {
    expect(readPracticeResume()).toBeNull();
    writePracticeResume({ ...CONFIG, lastBoard: [{ defId: "core-008", radiant: false }] });
    expect(readPracticeResume()).toEqual(CONFIG);
    clearPracticeResume();
    expect(readPracticeResume()).toBeNull();
  });

  it("R662 a lesson is never remembered: writing one forgets the free game", () => {
    writePracticeResume(CONFIG);
    writePracticeResume({ ...CONFIG, lesson: "basics" });
    expect(readPracticeResume()).toBeNull();
  });

  it("R662 reads anything malformed as no game", () => {
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

  it("R662 storage that throws is no game, and a refused write throws nothing", () => {
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
