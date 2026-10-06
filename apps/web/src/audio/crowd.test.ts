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
  it("#352 plays no looping bed but keeps the patron murmurs, and resets the quiet period until a staggered damage burst settles", () => {
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
    // #352: the ambience bed is gone, so nothing loops. The patrons still murmur between hits.
    expect(audio.nodes.filter((node) => node.kind === "bufferSource")).toHaveLength(0);
    expect(scheduled).toHaveLength(1);
    const firstPatron = scheduled.at(0);
    if (firstPatron === undefined) throw new Error("the crowd must schedule a patron");
    expect(firstPatron.ms).toBeGreaterThanOrEqual(CROWD_FEEL.patronMinMs);
    expect(firstPatron.ms).toBeLessThanOrEqual(CROWD_FEEL.patronMaxMs);
    firstPatron.run();
    // The patron is a one-shot murmur at an independent, non-centre position, and the next one is queued.
    const patronPan = audio.nodesOf("stereoPanner").at(-1);
    expect(patronPan).toBeDefined();
    expect(patronPan?.param("pan").settled()).not.toBe(0);
    expect(audio.nodes.filter((node) => node.kind === "bufferSource")).toHaveLength(1);
    const patrons = scheduled.filter((task) => task.ms >= CROWD_FEEL.patronMinMs);
    expect(patrons).toHaveLength(2);
    expect(patrons.at(-1)?.cancelled).toBe(false);

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
    expect(audio.nodes.filter((node) => node.kind === "bufferSource")).toHaveLength(2);
    const filters = audio.nodes.filter((node) => node.kind === "biquad");
    expect(filters.at(-1)?.param("frequency").settled()).toBe(270); // 25 is GIGA's roar, not 8's ooh.
    const reactionPan = audio.nodesOf("stereoPanner").at(-1);
    expect(reactionPan).toBeDefined();
    expect(reactionPan?.param("pan").settled()).not.toBe(0);

    // A GIGA roar carries its promised applause tail.
    const applause = scheduled.find((task) => task.ms === CROWD_FEEL.applauseDelayMs && !task.cancelled);
    if (applause === undefined) throw new Error("GIGA roar must schedule an applause tail");
    applause.run();
    expect(audio.nodes.filter((node) => node.kind === "bufferSource")).toHaveLength(3);
    expect(audio.nodes.some((node) => node.kind === "bufferSource" && node.loop)).toBe(false);

    const before = scheduled.length;
    crowd.end();
    // #352: nothing to fade, so ending schedules nothing. It stops the patrons, and a hit after it
    // draws no reaction.
    expect(scheduled).toHaveLength(before);
    expect(scheduled.filter((task) => task.ms >= CROWD_FEEL.patronMinMs).at(-1)?.cancelled).toBe(true);
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
