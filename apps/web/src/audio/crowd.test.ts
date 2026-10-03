import { afterEach, describe, expect, it } from "vitest";

import { CROWD_FEEL } from "../game/damageFeel.ts";
import { createCrowdDirector } from "./crowd.ts";
import { createAudioEngine } from "./engine.ts";
import { fakeContextFactory } from "./test/fakeAudio.ts";
import type { AudioEngine } from "./types.ts";

type Scheduled = { ms: number; run: () => void; cancelled: boolean };

const engines: AudioEngine[] = [];

afterEach(() => {
  for (const engine of engines.splice(0)) engine.dispose();
});

describe("issue #57 crowd director", () => {
  it("starts its two long beds and debounces simultaneous public damage to one largest reaction", () => {
    const factory = fakeContextFactory({ state: "running" });
    const engine = createAudioEngine({ createContext: factory.create });
    engines.push(engine);
    engine.unlock();
    const scheduled: Scheduled[] = [];
    const crowd = createCrowdDirector({
      engine,
      later: (ms, run) => {
        const task = { ms, run, cancelled: false };
        scheduled.push(task);
        return () => {
          task.cancelled = true;
        };
      },
    });

    crowd.start();
    const audio = factory.last();
    expect(audio.nodes.filter((node) => node.kind === "bufferSource" && node.loop).length).toBe(2);

    crowd.observeDamage(1);
    crowd.observeDamage(5);
    expect(scheduled.filter((task) => task.ms === CROWD_FEEL.reactionDebounceMs)).toHaveLength(0);
    crowd.observeDamage(8);
    crowd.observeDamage(15);
    crowd.observeDamage(25);
    const reaction = scheduled.filter((task) => task.ms === CROWD_FEEL.reactionDebounceMs);
    expect(reaction).toHaveLength(1);
    reaction[0]?.run();
    expect(audio.nodes.filter((node) => node.kind === "bufferSource" && !node.loop).length).toBe(1);
    const filters = audio.nodes.filter((node) => node.kind === "biquad");
    expect(filters.at(-1)?.param("frequency").settled()).toBe(270); // 25 is GIGA's roar, not 8's ooh.

    crowd.end();
    expect(scheduled.some((task) => task.ms === CROWD_FEEL.ambientFadeOutMs)).toBe(true);
    crowd.dispose();
    expect(audio.violations).toEqual([]);
  });
});
