// The audio barrel (audio/index.ts): the board's hooks and the engine, the only face the game
// reads (Game.tsx, Clock.tsx). A dropped re-export fails here instead of in a board suite.

import { describe, expect, it } from "vitest";

import {
  getAudioEngine,
  useGameAudio,
  usePickupSound,
  useTurnClockAlarm,
  useVoiceSpeaking,
} from "./index.ts";

describe("audio barrel", () => {
  it("re-exports the game hook and the pickup, clock and speaking hooks", () => {
    expect(typeof useGameAudio).toBe("function");
    expect(typeof usePickupSound).toBe("function");
    expect(typeof useTurnClockAlarm).toBe("function");
    expect(typeof useVoiceSpeaking).toBe("function");
  });

  it("re-exports the engine getter", () => {
    expect(typeof getAudioEngine).toBe("function");
  });
});
