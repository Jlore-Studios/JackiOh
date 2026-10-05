// #185 Unit Slam's sound: the thud by tier, the impact under a Huge or MASSIVE one, and the crowd
// through #57's quiet period (R702).

import type { GameEvent, UnitView } from "@jackioh/shared";
import { afterEach, describe, expect, it } from "vitest";

import { CROWD_FEEL, SLAM_PITCH_SPREAD, UNIT_SLAM } from "../game/damageFeel.ts";
import { baseView, unit } from "../test/fixtures.ts";
import { createCrowdDirector } from "./crowd.ts";
import { SOUND_CUES, type CueContext } from "./cues.ts";
import { createAudioEngine } from "./engine.ts";
import { SFX } from "./sfx.ts";
import { fakeContextFactory } from "./test/fakeAudio.ts";
import type { AudioEngine, SoundCue } from "./types.ts";

const engines: AudioEngine[] = [];
afterEach(() => {
  for (const engine of engines.splice(0)) engine.dispose();
});

const LANDING: Extract<GameEvent, { type: "summoned" }> = { type: "summoned", player: "p2", instanceId: "x", defId: "core-004", row: "units", lane: 2 };

function cuesOf(landed: UnitView | null, text = ""): readonly SoundCue[] {
  const ctx = {
    view: baseView(),
    lines: { voices: {}, effects: {}, cards: {} },
    unitNow: () => landed,
    card: () => ({ type: "Unit" as const, tags: [], text }),
  } as unknown as CueContext;
  return SOUND_CUES.summoned.cues(LANDING, ctx);
}

describe("R702 a landing Unit sounds its tier", () => {
  it("R702 the thud carries the tier and a pitch sample; Huge adds a Big impact and MASSIVE a GIGA one", () => {
    const small = cuesOf(unit("p2", { attack: 3, health: 4 }));
    expect(small.find((cue) => cue.kind === "sfx" && cue.id === "summon")).toMatchObject({ params: { slamTier: "small" } });
    expect(small.some((cue) => cue.kind === "sfx" && cue.id === "impact")).toBe(false);
    const huge = cuesOf(unit("p2", { attack: 12, health: 12 }), "Tribute 2");
    expect(huge.find((cue) => cue.kind === "sfx" && cue.id === "impact")).toMatchObject({ params: { impactTier: "big" } });
    const massive = cuesOf(unit("p2", { attack: 20, health: 20 }));
    expect(massive.find((cue) => cue.kind === "sfx" && cue.id === "impact")).toMatchObject({ params: { impactTier: "giga" } });
    const variation = small.find((cue) => cue.kind === "sfx" && cue.id === "summon");
    const sample = variation?.kind === "sfx" ? variation.params?.variation : undefined;
    expect(sample).toBeGreaterThanOrEqual(0);
    expect(sample).toBeLessThanOrEqual(1);
  });

  it("R702 a Unit not in the newest view keeps its plain summon thud", () => {
    expect(cuesOf(null)).toEqual([{ kind: "sfx", id: "summon", delayMs: 0 }]);
  });

  it("R702 the thud weighs more each tier up, and its pitch stays within ±5%", () => {
    const thuds = (["tiny", "small", "medium", "large", "huge"] as const).map((tier) => UNIT_SLAM[tier].thud);
    for (let i = 1; i < thuds.length; i += 1) expect(thuds[i]).toBeGreaterThan(thuds[i - 1] ?? 0);
    expect(SLAM_PITCH_SPREAD).toBe(0.05);
    expect(SFX.summon).toBeDefined();
  });
});

describe("R702 the crowd answers a landing through #57's quiet period, and stays neutral", () => {
  function setup() {
    const factory = fakeContextFactory({ state: "running" });
    const engine = createAudioEngine({ createContext: factory.create });
    engines.push(engine);
    engine.unlock();
    const scheduled: { ms: number; run: () => void; cancelled: boolean }[] = [];
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
    return { crowd, scheduled, audio: factory.last() };
  }

  it("R702 small Units draw nothing; several landings settle on the heaviest, with no cheer or applause after", () => {
    const { crowd, scheduled, audio } = setup();
    crowd.observeSlam("medium");
    expect(scheduled.filter((task) => task.ms === CROWD_FEEL.reactionDebounceMs)).toHaveLength(0);
    crowd.observeSlam("large");
    crowd.observeSlam("massive");
    crowd.observeSlam("huge");
    const due = scheduled.filter((task) => task.ms === CROWD_FEEL.reactionDebounceMs && !task.cancelled);
    expect(due).toHaveLength(1);
    const before = scheduled.length;
    due[0]?.run();
    // MASSIVE's excitement is the venue's roar (270 Hz), and no applause tail is scheduled after it.
    expect(audio.nodes.filter((node) => node.kind === "biquad").at(-1)?.param("frequency").settled()).toBe(270);
    expect(scheduled.slice(before).some((task) => task.ms === CROWD_FEEL.applauseDelayMs)).toBe(false);
  });

  it("R702 a hit and a landing in one resolution share the quiet period", () => {
    const { crowd, scheduled } = setup();
    crowd.observeDamage(25);
    crowd.observeSlam("large");
    expect(scheduled.filter((task) => task.ms === CROWD_FEEL.reactionDebounceMs && !task.cancelled)).toHaveLength(1);
  });
});
