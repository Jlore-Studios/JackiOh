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
  it("starts its two long beds and resets the quiet period until a staggered damage burst settles", () => {
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
    // The recurring patron layer also gets an independent, non-centre position.
    const firstPatron = scheduled.at(0);
    if (firstPatron === undefined) throw new Error("crowd bed must schedule a patron");
    firstPatron.run();
    const patronPan = audio.nodesOf("stereoPanner").at(-1);
    expect(patronPan).toBeDefined();
    expect(patronPan?.param("pan").settled()).not.toBe(0);

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
    expect(audio.nodes.filter((node) => node.kind === "bufferSource" && !node.loop).length).toBe(2);
    const filters = audio.nodes.filter((node) => node.kind === "biquad");
    expect(filters.at(-1)?.param("frequency").settled()).toBe(270); // 25 is GIGA's roar, not 8's ooh.
    const reactionPan = audio.nodesOf("stereoPanner").at(-1);
    expect(reactionPan).toBeDefined();
    expect(reactionPan?.param("pan").settled()).not.toBe(0);

    // A GIGA roar carries its promised applause tail and the tail remains in the ambience duck.
    const applause = scheduled.find((task) => task.ms === CROWD_FEEL.applauseDelayMs && !task.cancelled);
    if (applause === undefined) throw new Error("GIGA roar must schedule an applause tail");
    applause.run();
    expect(audio.nodes.filter((node) => node.kind === "bufferSource" && !node.loop)).toHaveLength(3);
    expect(audio.nodesOf("gain").some((node) => node.param("gain").targets().some((target) => target.value === 1))).toBe(true);

    crowd.end();
    expect(scheduled.some((task) => task.ms === CROWD_FEEL.ambientFadeOutMs)).toBe(true);
    crowd.dispose();
    expect(audio.nodes.filter((node) => node.kind === "bufferSource" && node.loop).every((node) => node.stopTime !== null)).toBe(true);
    expect(audio.violations).toEqual([]);
  });

  it("restores the shared ambience duck before a later match starts", () => {
    const factory = fakeContextFactory({ state: "running" });
    const engine = createAudioEngine({ createContext: factory.create });
    engines.push(engine);
    engine.unlock();
    const scheduled: Scheduled[] = [];
    const later = (ms: number, run: () => void): (() => void) => {
      const task = { ms, run, cancelled: false };
      scheduled.push(task);
      return () => {
        task.cancelled = true;
      };
    };

    const first = createCrowdDirector({ engine, later });
    first.start();
    first.end();
    const fade = scheduled.find((task) => task.ms === CROWD_FEEL.ambientFadeOutMs);
    if (fade === undefined) throw new Error("match end must schedule the ambience fade");
    fade.run();
    first.dispose();

    const second = createCrowdDirector({ engine, later });
    second.start();
    const output = engine.ambienceOutput();
    if (output === null) throw new Error("unlocked engine must expose its ambience output");
    expect(factory.last().nodeOf(output.ambienceDuck).param("gain").settled()).toBe(1);
    second.dispose();
  });
});
