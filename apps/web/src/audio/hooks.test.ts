// The attack-pickup cue and the speaking flag (audio/usePickupSound.ts R655,
// audio/useVoiceSpeaking.ts §9.9): the only two audio hooks with no direct test. A mutant that
// plays on a drop, or that reads the wrong seat's units, fails here.

import { renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { setAudioEngineForTests } from "./engine.ts";
import { fakeAudioEngine } from "./test/fakeAudio.ts";
import { usePickupSound } from "./usePickupSound.ts";
import { useVoiceSpeaking } from "./useVoiceSpeaking.ts";
import { baseView, emptySide, unit } from "../test/fixtures.ts";

afterEach(() => {
  setAudioEngineForTests(null);
});

function viewWithAttacker(instanceId: string, defId: string) {
  return baseView({
    you: emptySide("p1", { units: [unit("p1", { instanceId, defId }), null, null, null, null] }),
    opponent: emptySide("p2", { units: [unit("p2", { instanceId: "e1", defId }), null, null, null, null] }),
  });
}

describe("usePickupSound", () => {
  it("R655 plays the attack hook for the viewer's own lifted unit, and nothing else", () => {
    const engine = fakeAudioEngine();
    setAudioEngineForTests(engine);
    const view = viewWithAttacker("u9", "core-004");
    const { result } = renderHook(() => usePickupSound(view));

    result.current("u9");
    expect(engine.playPickup).toHaveBeenCalledTimes(1);
    expect(engine.playPickup).toHaveBeenCalledWith("core-004");
  });

  it("drops an unknown id, and never throws when the engine does", () => {
    const engine = fakeAudioEngine();
    setAudioEngineForTests(engine);
    const view = viewWithAttacker("u9", "core-004");
    const { result } = renderHook(() => usePickupSound(view));

    result.current("nope");
    result.current("e1");
    expect(engine.playPickup).not.toHaveBeenCalled();

    engine.playPickup.mockImplementationOnce(() => {
      throw new Error("no audio");
    });
    expect(() => result.current("u9")).not.toThrow();
  });
});

describe("useVoiceSpeaking", () => {
  it("reports whether a voice line holds the channel, and subscribes to it", () => {
    const engine = fakeAudioEngine();
    setAudioEngineForTests(engine);
    const { result } = renderHook(() => useVoiceSpeaking());

    expect(result.current).toBe(false);
    expect(engine.subscribeSpeaking).toHaveBeenCalledTimes(1);
    expect(engine.speaking).toHaveBeenCalled();
  });

  it("reports speaking when the engine says a line holds the channel", () => {
    const engine = fakeAudioEngine({ speaking: vi.fn(() => true) });
    setAudioEngineForTests(engine);
    const { result } = renderHook(() => useVoiceSpeaking());

    expect(result.current).toBe(true);
  });
});
