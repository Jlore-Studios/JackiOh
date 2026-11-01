import { afterEach, describe, expect, it } from "vitest";

import { CROWD_FEEL } from "../game/damageFeel.ts";
import { createCrowdDirector, type CrowdDirector } from "./crowd.ts";
import { createAudioEngine, type MatchFeelAudioEngine } from "./engine.ts";
import { fakeContextFactory, settle, type FakeAudio, type FakeContextFactory, type FakeNode } from "./test/fakeAudio.ts";
import type { AudioEngine } from "./types.ts";

type Scheduled = { ms: number; run: () => void; canceled: boolean };

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
        const task = { ms, run, canceled: false };
        scheduled.push(task);
        return () => {
          task.canceled = true;
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
    const patronPan = audio.nodesOf("stereoPanner").at(-1);
    expect(patronPan).toBeDefined();
    expect(patronPan?.param("pan").settled()).not.toBe(0);
    expect(audio.nodes.filter((node) => node.kind === "bufferSource")).toHaveLength(1);
    const patrons = scheduled.filter((task) => task.ms >= CROWD_FEEL.patronMinMs);
    expect(patrons).toHaveLength(2);
    expect(patrons.at(-1)?.canceled).toBe(false);

    crowd.observeDamage(1);
    crowd.observeDamage(5);
    expect(scheduled.filter((task) => task.ms === CROWD_FEEL.reactionDebounceMs)).toHaveLength(0);
    // Damage animation entries arrive 300 ms apart. Each one must move the same 400 ms quiet
    // period forward, otherwise the 15 would cheer before the 25 has landed.
    crowd.observeDamage(8);
    crowd.observeDamage(15);
    crowd.observeDamage(25);
    const reaction = scheduled.filter((task) => task.ms === CROWD_FEEL.reactionDebounceMs);
    expect(reaction.filter((task) => !task.canceled)).toHaveLength(1);
    reaction.at(-1)?.run();
    expect(audio.nodes.filter((node) => node.kind === "bufferSource")).toHaveLength(2);
    const filters = audio.nodes.filter((node) => node.kind === "biquad");
    expect(filters.at(-1)?.param("frequency").settled()).toBe(270); // 25 is GIGA's roar, not 8's ooh.
    const reactionPan = audio.nodesOf("stereoPanner").at(-1);
    expect(reactionPan).toBeDefined();
    expect(reactionPan?.param("pan").settled()).not.toBe(0);

    const applause = scheduled.find((task) => task.ms === CROWD_FEEL.applauseDelayMs && !task.canceled);
    if (applause === undefined) throw new Error("GIGA roar must schedule an applause tail");
    applause.run();
    expect(audio.nodes.filter((node) => node.kind === "bufferSource")).toHaveLength(3);
    expect(audio.nodes.some((node) => node.kind === "bufferSource" && node.loop)).toBe(false);

    const before = scheduled.length;
    crowd.end();
    // #352: nothing to fade, so ending schedules nothing. It stops the patrons, and a hit after it
    // draws no reaction.
    expect(scheduled).toHaveLength(before);
    expect(scheduled.filter((task) => task.ms >= CROWD_FEEL.patronMinMs).at(-1)?.canceled).toBe(true);
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

describe("background: the page hides mid-game and comes back (issue #2034)", () => {
  type Task = Scheduled & { ran: boolean };

  function rig(): { factory: FakeContextFactory; engine: MatchFeelAudioEngine; crowd: CrowdDirector; tasks: Task[] } {
    const factory = fakeContextFactory({ state: "running" });
    const engine = createAudioEngine({ createContext: factory.create, lifecycle: null, speech: null });
    engines.push(engine);
    engine.unlock();
    const tasks: Task[] = [];
    const crowd = createCrowdDirector({
      engine,
      later: (ms, run) => {
        const task = { ms, run, canceled: false, ran: false };
        tasks.push(task);
        return () => {
          task.canceled = true;
        };
      },
    });
    crowd.start();
    return { factory, engine, crowd, tasks };
  }

  /** Fires every timer now due (the ones it schedules wait for the next call). */
  function due(tasks: Task[]): void {
    for (const task of tasks.filter((t) => !t.canceled && !t.ran)) {
      task.ran = true;
      task.run();
    }
  }

  function heard(audio: FakeAudio, crowdBus: FakeNode): FakeNode[] {
    return audio.startedSources().filter((n) => audio.reaches(n, crowdBus));
  }

  function crowdBusOf(engine: MatchFeelAudioEngine, audio: FakeAudio): FakeNode {
    const output = engine.crowdOutput();
    if (output === null) throw new Error("an unlocked engine exposes its crowd output");
    return audio.nodeOf(output.crowd);
  }

  it("background: patrons and reactions that come due on a suspended or interrupted context are dropped, never saved up for the resume", async () => {
    const r = rig();
    const audio = r.factory.last();
    const bus = crowdBusOf(r.engine, audio);
    // A GIGA hit lands, then the page hides and the OS stops the context inside its quiet period.
    r.crowd.observeDamage(25);
    for (const state of ["suspended", "interrupted"] as const) {
      audio.state = state;
      // The patron and the roar come due on the stopped clock and are dropped …
      due(r.tasks);
      // … and a hit asked for now draws nothing, not even a quiet period.
      const before = r.tasks.length;
      r.crowd.observeDamage(25);
      expect(r.tasks).toHaveLength(before);
    }
    expect(heard(audio, bus)).toEqual([]);
    // The murmurs keep their rhythm for when sound comes back.
    expect(r.tasks.filter((t) => t.ms >= CROWD_FEEL.patronMinMs && !t.canceled && !t.ran)).toHaveLength(1);

    // The next tap resumes the context: nothing held back plays as a burst …
    r.engine.unlock();
    await settle();
    expect(r.engine.state()).toBe("running");
    due(r.tasks);
    // … only the one patron now due murmurs.
    expect(heard(audio, bus)).toHaveLength(1);
    expect(audio.violations).toEqual([]);
  });

  it("background: after the OS closed the context and the tap rebuilt it, the crowd plays on the new context", async () => {
    const r = rig();
    const closed = r.factory.last();
    const oldBus = crowdBusOf(r.engine, closed);
    closed.state = "closed";
    due(r.tasks);

    r.engine.unlock();
    await settle();
    const audio = r.factory.last();
    expect(audio).not.toBe(closed);
    const bus = crowdBusOf(r.engine, audio);
    due(r.tasks);
    expect(heard(audio, bus)).toHaveLength(1);
    r.crowd.observeDamage(25);
    due(r.tasks);
    expect(heard(audio, bus)).toHaveLength(3);
    expect(heard(closed, oldBus)).toEqual([]);
    expect(audio.violations).toEqual([]);
    expect(closed.violations).toEqual([]);
  });
});
