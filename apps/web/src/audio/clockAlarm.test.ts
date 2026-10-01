// The turn clock's last 30 seconds (R439, R506): the pure schedule, then the same readings into a
// real engine over the fake Web Audio context, and the hook `game/Clock.tsx` calls.

import { renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  CLOCK_ALARM_IDLE,
  alarmAudible,
  alarmBeatFor,
  planClockAlarm,
  stepClockAlarm,
  type ClockAlarmBeat,
  type ClockAlarmMemory,
} from "./clockAlarm.ts";
import {
  CLOCK_ALARM_BEAT_MS,
  CLOCK_ALARM_FROM_MS,
  CLOCK_ALARM_LATE_MS,
  CLOCK_ALARM_LOOKAHEAD_MS,
  CLOCK_ALARM_SHARP_FROM_MS,
} from "./constants.ts";
import { createAudioEngine, setAudioEngineForTests } from "./engine.ts";
import { DEFAULT_AUDIO_SETTINGS, resetAudioSettingsForTests, writeAudioSettings } from "./settings.ts";
import { FakeClock, fakeContextFactory, fakeSpeech } from "./test/fakeAudio.ts";
import type { AudioEngine, AudioSettings, PlayedCue } from "./types.ts";
import { useTurnClockAlarm } from "./useTurnClockAlarm.ts";

/** Clock's repaint cadence (game/Clock.tsx TICK_MS). */
const REPAINT_MS = 200;
const FROM_S = CLOCK_ALARM_FROM_MS / CLOCK_ALARM_BEAT_MS;
const SHARP_S = CLOCK_ALARM_SHARP_FROM_MS / CLOCK_ALARM_BEAT_MS;

type Heard = ClockAlarmBeat & { atMs: number };

/**
 * Reads a clock that starts at `startMs` left, every `stepMs`, down to 0, and returns each beat with
 * the moment it lands: the reading's time plus its delay.
 */
function countdown(startMs: number, stepMs: number, audible = true, own = true): Heard[] {
  let memory: ClockAlarmMemory = CLOCK_ALARM_IDLE;
  const heard: Heard[] = [];
  for (let elapsed = 0; elapsed <= startMs; elapsed += stepMs) {
    const plan = planClockAlarm(memory, { remainingMs: startMs - elapsed, own, audible });
    memory = plan.memory;
    for (const beat of plan.beats) heard.push({ ...beat, atMs: elapsed + beat.delayMs });
  }
  return heard;
}

describe("R506 the turn clock's alarm: the schedule (R439)", () => {
  it("R506 one beat for each of the last 30 seconds, in order, each landing on its second", () => {
    const start = 45_000;
    const heard = countdown(start, REPAINT_MS);
    expect(heard.map((b) => b.second)).toEqual(Array.from({ length: FROM_S }, (_, i) => FROM_S - i));
    for (const beat of heard) expect(beat.atMs, `second ${String(beat.second)}`).toBe(start - beat.second * CLOCK_ALARM_BEAT_MS);
  });

  it("R506 a heartbeat until the last ten seconds, then a tick sharper each second, amount 1 to 10", () => {
    expect(alarmBeatFor(FROM_S)).toEqual({ id: "heartbeat" });
    expect(alarmBeatFor(SHARP_S + 1)).toEqual({ id: "heartbeat" });
    expect(alarmBeatFor(SHARP_S)).toEqual({ id: "clockTick", params: { amount: 1 } });
    expect(alarmBeatFor(1)).toEqual({ id: "clockTick", params: { amount: SHARP_S } });
    const heard = countdown(35_000, REPAINT_MS);
    expect(heard.filter((b) => b.id === "heartbeat")).toHaveLength(FROM_S - SHARP_S);
    expect(heard.filter((b) => b.id === "clockTick").map((b) => b.params?.amount)).toEqual(Array.from({ length: SHARP_S }, (_, i) => i + 1));
  });

  it("R506 nothing before the last 30 seconds, and nothing at 0", () => {
    let memory: ClockAlarmMemory = CLOCK_ALARM_IDLE;
    for (const remainingMs of [90_000, CLOCK_ALARM_FROM_MS + CLOCK_ALARM_LOOKAHEAD_MS + 1]) {
      const plan = planClockAlarm(memory, { remainingMs, own: true, audible: true });
      expect(plan.beats, String(remainingMs)).toEqual([]);
      memory = plan.memory;
    }
    expect(planClockAlarm(CLOCK_ALARM_IDLE, { remainingMs: 0, own: true, audible: true }).beats).toEqual([]);
  });

  it("R506 irregular readings still sound every second exactly once, on time", () => {
    const start = 31_000;
    const heard = countdown(start, CLOCK_ALARM_LOOKAHEAD_MS - 50);
    expect(heard.map((b) => b.second)).toEqual(Array.from({ length: FROM_S }, (_, i) => FROM_S - i));
    for (const beat of heard) expect(beat.atMs).toBe(start - beat.second * CLOCK_ALARM_BEAT_MS);
  });

  it("R506 a beat whose moment has just passed plays at once; an older one is skipped, never played late", () => {
    const justPast = planClockAlarm({ lastSecond: 21 }, { remainingMs: 20_000 - (CLOCK_ALARM_LATE_MS - 50), own: true, audible: true });
    expect(justPast.beats).toEqual([{ second: 20, id: "heartbeat", delayMs: 0 }]);
    const long = planClockAlarm({ lastSecond: 21 }, { remainingMs: 20_000 - (CLOCK_ALARM_LATE_MS + 300), own: true, audible: true });
    expect(long.beats).toEqual([]);
  });

  it("R506 a clock seen first mid-countdown beats from the next second on", () => {
    const plan = planClockAlarm(CLOCK_ALARM_IDLE, { remainingMs: 17_300, own: true, audible: true });
    expect(plan.beats).toEqual([{ second: 17, id: "heartbeat", delayMs: 300 }]);
  });

  it("R506 only the viewer's own running turn clock beats: the other seat's, a paused one, or none is silent", () => {
    expect(countdown(31_000, REPAINT_MS, true, false)).toEqual([]);
    const mid = planClockAlarm({ lastSecond: 12 }, { remainingMs: 11_100, own: false, audible: true });
    expect(mid).toEqual({ memory: CLOCK_ALARM_IDLE, beats: [] });
    expect(planClockAlarm({ lastSecond: 12 }, { remainingMs: null, own: true, audible: true })).toEqual({ memory: CLOCK_ALARM_IDLE, beats: [] });
  });

  it("R506 a clock that goes back up (a new turn) counts down afresh", () => {
    let memory: ClockAlarmMemory = { lastSecond: 3 };
    memory = planClockAlarm(memory, { remainingMs: 90_000, own: true, audible: true }).memory;
    expect(memory.lastSecond).toBeNull();
    const plan = planClockAlarm(memory, { remainingMs: 30_100, own: true, audible: true });
    expect(plan.beats.map((b) => b.second)).toEqual([FROM_S]);
  });

  it("R506 a beat that cannot be heard is passed over, not saved up for when sound returns", () => {
    const silent = planClockAlarm(CLOCK_ALARM_IDLE, { remainingMs: 25_200, own: true, audible: false });
    expect(silent.beats).toEqual([]);
    expect(silent.memory.lastSecond).toBe(25);
    const back = planClockAlarm(silent.memory, { remainingMs: 25_050, own: true, audible: true });
    expect(back.beats).toEqual([]);
  });
});

/* --------------------------------------------------------------------------------------------- *
 * Into a real engine over the fake context
 * --------------------------------------------------------------------------------------------- */

describe("R506 the turn clock's alarm through the engine", () => {
  let engine: AudioEngine | null = null;

  beforeEach(() => {
    vi.useFakeTimers();
    localStorage.clear();
    resetAudioSettingsForTests();
  });

  afterEach(() => {
    engine?.dispose();
    engine = null;
    setAudioEngineForTests(null);
    resetAudioSettingsForTests();
    localStorage.clear();
    vi.useRealTimers();
  });

  function rig(state: "running" | "suspended" = "running") {
    const factory = fakeContextFactory({ state, resumeMode: "run" });
    const clock = new FakeClock(10_000);
    engine = createAudioEngine({ createContext: factory.create, speech: fakeSpeech().port, now: clock.now });
    return { engine, factory, clock };
  }

  const sfxLog = (e: AudioEngine): Extract<PlayedCue, { kind: "sfx" }>[] => e.log().filter((c): c is Extract<PlayedCue, { kind: "sfx" }> => c.kind === "sfx");

  it("R506 alarmAudible needs a running context and neither mute nor a 0 master or effects volume", () => {
    const running = { state: () => "running" as const };
    expect(alarmAudible(running, DEFAULT_AUDIO_SETTINGS)).toBe(true);
    expect(alarmAudible({ state: () => "locked" as const }, DEFAULT_AUDIO_SETTINGS)).toBe(false);
    expect(alarmAudible({ state: () => "suspended" as const }, DEFAULT_AUDIO_SETTINGS)).toBe(false);
    const quiet = (patch: Partial<AudioSettings>): AudioSettings => ({ ...DEFAULT_AUDIO_SETTINGS, ...patch });
    expect(alarmAudible(running, quiet({ muted: true }))).toBe(false);
    expect(alarmAudible(running, quiet({ sfx: 0 }))).toBe(false);
    expect(alarmAudible(running, quiet({ master: 0 }))).toBe(false);
    expect(alarmAudible(running, quiet({ voice: 0, voiceOn: false }))).toBe(true);
  });

  it("R506 each beat is scheduled on the effects bus at its second, ahead of the reading", () => {
    const { engine: e, factory } = rig();
    e.unlock();
    const audio = factory.last();
    const settings = { ...DEFAULT_AUDIO_SETTINGS };
    const before = audio.nodes.length;
    stepClockAlarm(e, settings, CLOCK_ALARM_IDLE, 12_300, true);
    expect(sfxLog(e).map((c) => [c.id, c.delayMs])).toEqual([["heartbeat", 300]]);
    const started = audio.nodes.slice(before).filter((n) => n.started);
    expect(Math.min(...started.map((n) => n.startTime ?? Infinity))).toBeCloseTo(audio.context.currentTime + 0.3, 9);
  });

  it("R506 nothing before the first gesture: no context is made and nothing is logged", () => {
    const { engine: e, factory } = rig();
    let memory: ClockAlarmMemory = CLOCK_ALARM_IDLE;
    for (let ms = 12_000; ms >= 0; ms -= REPAINT_MS) memory = stepClockAlarm(e, DEFAULT_AUDIO_SETTINGS, memory, ms, true);
    expect(e.contextsCreated()).toBe(0);
    expect(factory.made).toHaveLength(0);
    expect(e.log()).toEqual([]);
  });

  it("R506 nothing under mute or with the effects volume at 0", () => {
    const { engine: e } = rig();
    e.unlock();
    for (const patch of [{ muted: true }, { muted: false, sfx: 0 }]) {
      const settings = writeAudioSettings(patch);
      let memory: ClockAlarmMemory = CLOCK_ALARM_IDLE;
      for (let ms = 12_000; ms >= 0; ms -= REPAINT_MS) memory = stepClockAlarm(e, settings, memory, ms, true);
    }
    expect(sfxLog(e)).toEqual([]);
  });

  it("R506 a whole countdown logs twenty heartbeats and ten ticks", () => {
    const { engine: e, clock, factory } = rig();
    e.unlock();
    const audio = factory.last();
    let memory: ClockAlarmMemory = CLOCK_ALARM_IDLE;
    for (let ms = 31_000; ms >= 0; ms -= REPAINT_MS) {
      memory = stepClockAlarm(e, DEFAULT_AUDIO_SETTINGS, memory, ms, true);
      clock.advance(REPAINT_MS);
      audio.advance(REPAINT_MS / 1000);
    }
    const log = sfxLog(e);
    expect(log.filter((c) => c.id === "heartbeat")).toHaveLength(FROM_S - SHARP_S);
    expect(log.filter((c) => c.id === "clockTick").map((c) => c.params?.amount)).toEqual(Array.from({ length: SHARP_S }, (_, i) => i + 1));
  });
});

/* --------------------------------------------------------------------------------------------- *
 * The hook
 * --------------------------------------------------------------------------------------------- */

describe("R506 useTurnClockAlarm", () => {
  afterEach(() => {
    setAudioEngineForTests(null);
    resetAudioSettingsForTests();
    localStorage.clear();
  });

  function recordingEngine(state: "running" | "locked"): AudioEngine & { beats: [string, number | undefined][] } {
    const beats: [string, number | undefined][] = [];
    const fake = {
      beats,
      state: () => state,
      playSfx: (id: string, _params?: unknown, delayMs?: number) => {
        beats.push([id, delayMs]);
        return true;
      },
    };
    return fake as unknown as AudioEngine & { beats: [string, number | undefined][] };
  }

  it("R506 each repaint's reading beats through the audio engine, once per second", () => {
    const engine = recordingEngine("running");
    setAudioEngineForTests(engine);
    const { rerender } = renderHook(({ ms, own }: { ms: number | null; own: boolean }) => {
      useTurnClockAlarm(ms, own);
    }, { initialProps: { ms: 12_300, own: true } });
    rerender({ ms: 12_100, own: true });
    rerender({ ms: 11_900, own: true });
    rerender({ ms: 11_300, own: true });
    expect(engine.beats).toEqual([
      ["heartbeat", 300],
      ["heartbeat", 300],
    ]);
  });

  it("R506 the opponent's clock and a locked context stay silent, and nothing throws without audio", () => {
    const locked = recordingEngine("locked");
    setAudioEngineForTests(locked);
    const { rerender } = renderHook(({ ms, own }: { ms: number | null; own: boolean }) => {
      useTurnClockAlarm(ms, own);
    }, { initialProps: { ms: 5_200, own: true } });
    rerender({ ms: 5_100, own: false });
    expect(locked.beats).toEqual([]);

    setAudioEngineForTests(null); // jsdom: the default engine has no AudioContext
    expect(() => renderHook(() => {
      useTurnClockAlarm(4_100, true);
    })).not.toThrow();
  });
});
