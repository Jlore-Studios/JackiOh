// The hook `game/Clock.tsx` calls for the turn clock's last 30 seconds (R439, R506; clockAlarm.ts
// has the schedule). Clock passes the clock as it shows it on each repaint, `own` while it is the
// viewer's own turn and not paused.
// It never throws: with no AudioContext (jsdom), or before the first gesture, it stays silent.

import { useEffect, useRef } from "react";

import { CLOCK_ALARM_IDLE, stepClockAlarm, type ClockAlarmMemory } from "./clockAlarm.ts";
import { getAudioEngine } from "./engine.ts";
import { readAudioSettings } from "./settings.ts";

export function useTurnClockAlarm(remainingMs: number | null, own: boolean): void {
  const memory = useRef<ClockAlarmMemory>(CLOCK_ALARM_IDLE);
  useEffect(() => {
    try {
      memory.current = stepClockAlarm(getAudioEngine(), readAudioSettings(), memory.current, remainingMs, own);
    } catch {
      // Sound is never a rule: a failure drops the beat and keeps the clock.
    }
  }, [remainingMs, own]);
}
