// The turn clock's last 30 seconds, heard (R439, R506). A tense heartbeat once a second while the
// viewer's own turn clock runs through its final CLOCK_ALARM_FROM_MS, and in the last
// CLOCK_ALARM_SHARP_FROM_MS a tick that grows sharper every second, all through the effects bus.
//
// It decides nothing (CLAUDE.md rule 7): the clock is the server's (R79), `game/Clock.tsx` reads it,
// and this only listens to the number Clock shows. The other seat's clock never beats, a paused
// clock is the caller's `own: false`, and nothing sounds while sound is muted, the master or effects
// volume is 0, or before the first gesture has unlocked the context (§10.11 Autoplay).
//
// SCHEDULING. Clock repaints every 200 ms, so the remaining time arrives in steps. Each beat belongs
// to a whole second: second `s` sounds the moment the clock reaches `s × CLOCK_ALARM_BEAT_MS`. On
// each reading, every beat whose moment is at most CLOCK_ALARM_LOOKAHEAD_MS ahead is scheduled that
// far ahead on the audio clock, so the beats land on the second and not on the repaint; one whose
// moment passed at most CLOCK_ALARM_LATE_MS ago plays at once, and an older one is skipped rather
// than played late. A beat is scheduled once (`lastSecond`); a clock that goes back up is a new
// countdown. The planner is pure; `stepClockAlarm` hands its beats to an engine.

import {
  CLOCK_ALARM_BEAT_MS,
  CLOCK_ALARM_FROM_MS,
  CLOCK_ALARM_LATE_MS,
  CLOCK_ALARM_LOOKAHEAD_MS,
  CLOCK_ALARM_SHARP_FROM_MS,
} from "./constants.ts";
import type { AudioEngine, AudioSettings, SfxParams } from "./types.ts";

/** What the alarm remembers between readings: the last second it has scheduled (or passed over). */
export type ClockAlarmMemory = { readonly lastSecond: number | null };

/** No countdown under way. */
export const CLOCK_ALARM_IDLE: ClockAlarmMemory = { lastSecond: null };

export type ClockAlarmInput = {
  /** The viewer's turn clock as Clock shows it, in ms, or null when no clock runs. */
  remainingMs: number | null;
  /** True only while it is the viewer's own turn clock and it is running (not paused, R79). */
  own: boolean;
  /** True when a beat can be heard: the context runs, and neither mute nor a 0 volume silences it. */
  audible: boolean;
};

export type ClockAlarmBeat = {
  /** The seconds left at the beat's moment. */
  second: number;
  id: "heartbeat" | "clockTick";
  params?: SfxParams;
  /** From this reading to the beat's moment, never below 0. */
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

/** The beats to schedule on this reading, and the memory to keep. Pure. */
export function planClockAlarm(
  memory: ClockAlarmMemory,
  input: ClockAlarmInput,
): { memory: ClockAlarmMemory; beats: ClockAlarmBeat[] } {
  const { remainingMs, own, audible } = input;
  if (!own || remainingMs === null || !Number.isFinite(remainingMs)) return { memory: CLOCK_ALARM_IDLE, beats: [] };

  let last = memory.lastSecond;
  // The clock went back up past the last beat: a new turn, a new countdown.
  if (last !== null && remainingMs > last * CLOCK_ALARM_BEAT_MS + CLOCK_ALARM_LOOKAHEAD_MS) last = null;

  const top = Math.min(FROM_SECONDS, Math.floor((remainingMs + CLOCK_ALARM_LATE_MS) / CLOCK_ALARM_BEAT_MS));
  const bottom = Math.max(1, Math.ceil((remainingMs - CLOCK_ALARM_LOOKAHEAD_MS) / CLOCK_ALARM_BEAT_MS));
  const beats: ClockAlarmBeat[] = [];
  for (let second = top; second >= bottom; second -= 1) {
    if (last !== null && second >= last) continue;
    // A beat that cannot be heard is passed over, not saved up for later.
    if (audible) {
      beats.push({ second, ...alarmBeatFor(second), delayMs: Math.max(0, remainingMs - second * CLOCK_ALARM_BEAT_MS) });
    }
    last = second;
  }
  return { memory: { lastSecond: last }, beats };
}

/** Whether a beat would be heard: the context runs, and mute, master 0 or effects 0 do not silence it. */
export function alarmAudible(engine: Pick<AudioEngine, "state">, settings: AudioSettings): boolean {
  return engine.state() === "running" && !settings.muted && settings.master > 0 && settings.sfx > 0;
}

/** One reading: plans against `memory`, sends the beats to `engine`, and returns the memory to keep. */
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
