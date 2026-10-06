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
  it("#352 starts no ambience bed or patron, and resets the quiet period until a staggered damage burst settles", () => {
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
    // #352: the ambience is gone, so a match starts with no looping bed and no patron timer.
    expect(audio.nodes.filter((node) => node.kind === "bufferSource")).toHaveLength(0);
    expect(scheduled).toHaveLength(0);

    crowd.observeDamage(1);
    crowd.observeDamage(5);
    expect(scheduled.filter((task) => task.ms === CROWD_FEEL.reactionDebounceMs)).toHaveLength(0);
    // Damage animation entries arrive 300 ms apart. Each one must move the same 400 ms quiet
    // period forward, otherwise the 15 would cheer before the 25 has landed.
    crowd.observeDamage(8);
    crowd.observeDamage(15);
    crowd.observeDamage(25);
    const reaction = scheduled.filter((task) => task.ms === CROWD_FEEL.reactionDebounceMs);
    // Each later hit replaces the earlier quiet-period deadline, so the last timer alone lives.
    expect(reaction.filter((task) => !task.cancelled)).toHaveLength(1);
    reaction.at(-1)?.run();
    expect(audio.nodes.filter((node) => node.kind === "bufferSource")).toHaveLength(1);
    const filters = audio.nodes.filter((node) => node.kind === "biquad");
    expect(filters.at(-1)?.param("frequency").settled()).toBe(270); // 25 is GIGA's roar, not 8's ooh.
    const reactionPan = audio.nodesOf("stereoPanner").at(-1);
    expect(reactionPan).toBeDefined();
    expect(reactionPan?.param("pan").settled()).not.toBe(0);

    // A GIGA roar carries its promised applause tail.
    const applause = scheduled.find((task) => task.ms === CROWD_FEEL.applauseDelayMs && !task.cancelled);
    if (applause === undefined) throw new Error("GIGA roar must schedule an applause tail");
    applause.run();
    expect(audio.nodes.filter((node) => node.kind === "bufferSource")).toHaveLength(2);
    expect(audio.nodes.some((node) => node.kind === "bufferSource" && node.loop)).toBe(false);

    const before = scheduled.length;
    crowd.end();
    // #352: nothing to fade, so ending schedules nothing, and a hit after it draws no reaction.
    expect(scheduled).toHaveLength(before);
    crowd.observeDamage(25);
    expect(scheduled).toHaveLength(before);
    crowd.dispose();
    expect(audio.violations).toEqual([]);
  });

  it("#352 the engine's crowd output carries the crowd bus and no ambience bus", () => {
    const factory = fakeContextFactory({ state: "running" });
    const engine = createAudioEngine({ createContext: factory.create });
    engines.push(engine);
    expect(engine.crowdOutput()).toBeNull();
    engine.unlock();
    const output = engine.crowdOutput();
    if (output === null) throw new Error("unlocked engine must expose its crowd output");
    expect(Object.keys(output).sort()).toEqual(["context", "crowd"]);
  });
});
