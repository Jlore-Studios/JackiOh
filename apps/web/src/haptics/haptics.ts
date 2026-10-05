// Haptics (R669, issue #259): a short buzz on a phone for three moments of the viewer's own game,
// read off the same event stream the sound director resolves, at the moment its sound plays.
//
//   drop  the viewer's own card lands (its `cardPlayed`: a play, a cast or a Trap set)
//   hit   damage lands on anything (a `damage` of more than 0)
//   turn  the viewer's own turn starts (`turnStarted`)
//
// None of it is a rule (CLAUDE.md rule 7): it reads the event and the viewer's `PlayerView`, which
// already name every one of these moments to that seat, and says nothing a sound does not. It buzzes
// only where `navigator.vibrate` exists (Android's browsers; iOS has none), while the vibration
// switch is on, and never under "Reduce motion" (the panel's switch, the effects' override or the
// system preference). One buzz at most every HAPTIC_MIN_GAP_MS, so a board wipe is one tick, not a
// rattle. It never throws.

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { reducedMotionNow } from "../game/animations.ts";
import { readHapticsSettings } from "./settings.ts";

export type HapticMoment = "drop" | "hit" | "turn";

/** Each moment's vibration pattern in ms (on, off, on, …): ticks short enough to feel, not hear. */
export const HAPTIC_PATTERNS: { readonly [K in HapticMoment]: readonly number[] } = {
  drop: [12],
  hit: [24],
  turn: [16, 70, 16],
};

/** A buzz this soon after the last one is skipped. */
export const HAPTIC_MIN_GAP_MS = 120;

/** The moment an event is to the viewer, or null for one that buzzes nothing. */
export function hapticFor(event: GameEvent, view: PlayerView): HapticMoment | null {
  switch (event.type) {
    case "cardPlayed":
      return event.player === view.viewer ? "drop" : null;
    case "damage":
      return event.amount > 0 ? "hit" : null;
    case "turnStarted":
      return event.player === view.viewer ? "turn" : null;
    default:
      return null;
  }
}

export type HapticsOptions = {
  /** `navigator.vibrate`, bound, or null where the device has none. */
  vibrate?: ((pattern: number[]) => boolean) | null;
  /** Milliseconds, for the gap. */
  now?: () => number;
  /** Whether motion is reduced right now. */
  reducedMotion?: () => boolean;
};

export type Haptics = {
  /** Hear one event as it is resolved, with the view it was planned against. */
  onEvent(event: GameEvent, view: PlayerView): void;
};

function browserVibrate(): ((pattern: number[]) => boolean) | null {
  try {
    const nav = typeof navigator === "undefined" ? undefined : navigator;
    return typeof nav?.vibrate === "function" ? (pattern) => nav.vibrate(pattern) : null;
  } catch {
    return null;
  }
}

export function createHaptics(options: HapticsOptions = {}): Haptics {
  const vibrate = options.vibrate === undefined ? browserVibrate() : options.vibrate;
  const now = options.now ?? (() => performance.now());
  const reduced = options.reducedMotion ?? (() => reducedMotionNow());
  let last: number | null = null;
  return {
    onEvent(event, view) {
      try {
        if (vibrate === null) return;
        const moment = hapticFor(event, view);
        if (moment === null) return;
        if (!readHapticsSettings().vibration || reduced()) return;
        const t = now();
        if (last !== null && t - last < HAPTIC_MIN_GAP_MS) return;
        last = t;
        vibrate([...HAPTIC_PATTERNS[moment]]);
      } catch {
        // A buzz is never worth an error: drop it, keep the game.
      }
    },
  };
}
