// The sounds and buzz of a drag (`./cues.ts`, #3347): each cue fires once through the game's sound
// engine, pick-up alone buzzes, and a failure is dropped, never thrown. The engine and haptics are
// fakes, as `notfound/NotFoundPlay.test.tsx` fakes the engine.

import { beforeEach, describe, expect, it, vi } from "vitest";

const playSfx = vi.hoisted(() => vi.fn());
const moment = vi.hoisted(() => vi.fn());

vi.mock("../../audio/engine.ts", async (importOriginal) => ({
  ...(await importOriginal<Record<string, unknown>>()),
  getAudioEngine: () => ({ playSfx }),
}));

vi.mock("../../haptics/haptics.ts", async (importOriginal) => ({
  ...(await importOriginal<Record<string, unknown>>()),
  sharedHaptics: () => ({ moment }),
}));

import { dragCue } from "./cues.ts";

beforeEach(() => {
  playSfx.mockReset();
  moment.mockReset();
});

describe("dragCue", () => {
  it("plays each cue once through the sound engine", () => {
    dragCue("dragOver");
    expect(playSfx).toHaveBeenCalledTimes(1);
    expect(playSfx).toHaveBeenCalledWith("dragOver");
    dragCue("drop");
    dragCue("returnToHand");
    expect(playSfx).toHaveBeenCalledTimes(3);
  });

  it("buzzes the pick-up and nothing else", () => {
    dragCue("pickUp");
    expect(playSfx).toHaveBeenCalledWith("pickUp");
    expect(moment).toHaveBeenCalledTimes(1);
    expect(moment).toHaveBeenCalledWith("pickup");
    dragCue("drop");
    expect(moment).toHaveBeenCalledTimes(1);
  });

  it("drops a refused sound and a refused buzz without throwing", () => {
    playSfx.mockImplementation(() => {
      throw new Error("locked");
    });
    moment.mockImplementation(() => {
      throw new Error("no haptics");
    });
    expect(() => dragCue("pickUp")).not.toThrow();
    expect(() => dragCue("drop")).not.toThrow();
  });
});
