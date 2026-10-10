// Audible final-30-second turn-clock cues (R439, R506).
//
// It observes only the server clock (R79; CLAUDE.md rule 7), for the viewer's running turn after
// audio unlock (§10.11 Autoplay).
//
// Schedule each second once ahead of repainting; skip beats late beyond CLOCK_ALARM_LATE_MS and
// reset when a clock moves up.

import {
  CLOCK_ALARM_BEAT_MS,
  CLOCK_ALARM_FROM_MS,
  CLOCK_ALARM_LATE_MS,
  CLOCK_ALARM_LOOKAHEAD_MS,
  CLOCK_ALARM_SHARP_FROM_MS,
} from "./constants.ts";
import type { AudioEngine, AudioSettings, SfxParams } from "./types.ts";

/** Last scheduled or skipped second. */
export type ClockAlarmMemory = { readonly lastSecond: number | null };

export const CLOCK_ALARM_IDLE: ClockAlarmMemory = { lastSecond: null };

export type ClockAlarmInput = {
  remainingMs: number | null;
  /** Viewer-owned, running clock (R79). */
  own: boolean;
  audible: boolean;
};

export type ClockAlarmBeat = {
  second: number;
  id: "heartbeat" | "clockTick";
  params?: SfxParams;
  /** Delay from this reading, never below 0. */
  delayMs: number;
};

const FROM_SECONDS = CLOCK_ALARM_FROM_MS / CLOCK_ALARM_BEAT_MS;
const SHARP_SECONDS = CLOCK_ALARM_SHARP_FROM_MS / CLOCK_ALARM_BEAT_MS;

/**
 * The sound for the moment the clock reaches `second` seconds left: the heartbeat, or in the last
 * ten seconds a tick whose `amount` climbs from 1 (ten left) to 10 (one left).
 */
export function alarmBeatFor(second: number): Pick<ClockAlarmBeat, "id" | "params"> {
  if (second > SHARP_SECONDS) return { id: "heartbeat" };
  return { id: "clockTick", params: { amount: SHARP_SECONDS + 1 - second } };
}

export function planClockAlarm(
  memory: ClockAlarmMemory,
  input: ClockAlarmInput,
): { memory: ClockAlarmMemory; beats: ClockAlarmBeat[] } {
  const { remainingMs, own, audible } = input;
  if (!own || remainingMs === null || !Number.isFinite(remainingMs)) return { memory: CLOCK_ALARM_IDLE, beats: [] };

  let last = memory.lastSecond;
  // A clock moving up starts a new countdown.
  if (last !== null && remainingMs > last * CLOCK_ALARM_BEAT_MS + CLOCK_ALARM_LOOKAHEAD_MS) last = null;

  const top = Math.min(FROM_SECONDS, Math.floor((remainingMs + CLOCK_ALARM_LATE_MS) / CLOCK_ALARM_BEAT_MS));
  const bottom = Math.max(1, Math.ceil((remainingMs - CLOCK_ALARM_LOOKAHEAD_MS) / CLOCK_ALARM_BEAT_MS));
  const beats: ClockAlarmBeat[] = [];
  for (let second = top; second >= bottom; second -= 1) {
    if (last !== null && second >= last) continue;
    // Do not save inaudible beats for later.
    if (audible) {
      beats.push({ second, ...alarmBeatFor(second), delayMs: Math.max(0, remainingMs - second * CLOCK_ALARM_BEAT_MS) });
    }
    last = second;
  }
  return { memory: { lastSecond: last }, beats };
}

export function alarmAudible(engine: Pick<AudioEngine, "state">, settings: AudioSettings): boolean {
  return engine.state() === "running" && !settings.muted && settings.master > 0 && settings.sfx > 0;
}

export function stepClockAlarm(
  engine: Pick<AudioEngine, "state" | "playSfx">,
  settings: AudioSettings,
  memory: ClockAlarmMemory,
  remainingMs: number | null,
  own: boolean,
): ClockAlarmMemory {
  const plan = planClockAlarm(memory, { remainingMs, own, audible: alarmAudible(engine, settings) });
  for (const beat of plan.beats) engine.playSfx(beat.id, beat.params, beat.delayMs);
  return plan.memory;
}
