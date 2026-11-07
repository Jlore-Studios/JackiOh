// Haptics (R669) buzz for a viewer's own plays and turns, plus damage.
// Public events only, so they neither enforce rules nor reveal information (CLAUDE.md rule 7).
// R3348000, R3348001: a touch press on a tile or a corner prop buzzes too (`buzz`, `tapHaptic`), on its
// own gap, so a tile never swallows the game's own drop.

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { reducedMotionNow } from "../game/animations.ts";
import { readHapticsSettings } from "./settings.ts";

export type HapticMoment = "drop" | "hit" | "turn" | "touch" | "pickup" | "tile" | "tileLegal" | "corner" | "cornerBig" | "cornerRare";

export const HAPTIC_PATTERNS: { readonly [K in HapticMoment]: readonly number[] } = {
  drop: [12],
  hit: [24],
  turn: [16, 70, 16],
  touch: [8],
  /** #3347: lifting a card from the hand, lighter than the drop that ends the drag. */
  pickup: [8],
  // A press on a tile (a legal one answers harder) and on a corner prop (a bigger payoff, a rare one).
  tile: [6],
  tileLegal: [12],
  corner: [8],
  cornerBig: [14, 40, 14],
  cornerRare: [10, 30, 10, 30, 24],
};

export const HAPTIC_MIN_GAP_MS = 120;

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
  vibrate?: ((pattern: number[]) => boolean) | null;
  now?: () => number;
  reducedMotion?: () => boolean;
};

export type Haptics = {
  onEvent(event: GameEvent, view: PlayerView): void;
  touch(): void;
  /** #3347: a buzz for a moment the player's own input makes (a pick-up), through the same gates as an event's. */
  moment(moment: HapticMoment): void;
  /** R3348000: a press's own buzz, behind the same switch, reduced motion and vibrate guards as `onEvent`, on its own gap. */
  buzz(moment: HapticMoment): void;
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
  function play(moment: HapticMoment): void {
    try {
      if (vibrate === null) return;
      if (!readHapticsSettings().vibration || reduced()) return;
      const t = now();
      if (last !== null && t - last < HAPTIC_MIN_GAP_MS) return;
      last = t;
      vibrate([...HAPTIC_PATTERNS[moment]]);
    } catch {
      // Haptics must not interrupt the game.
    }
  }
  let lastTap: number | null = null;
  return {
    buzz(moment) {
      try {
        if (vibrate === null) return;
        if (!readHapticsSettings().vibration || reduced()) return;
        const t = now();
        if (lastTap !== null && t - lastTap < HAPTIC_MIN_GAP_MS) return;
        lastTap = t;
        vibrate([...HAPTIC_PATTERNS[moment]]);
      } catch {
        // Haptics must not interrupt the game.
      }
    },
    onEvent(event, view) {
      try {
        const moment = hapticFor(event, view);
        if (moment !== null) play(moment);
      } catch {
        // Haptics must not interrupt the game.
      }
    },
    touch() {
      play("touch");
    },
    moment: play,
  };
}

let shared: Haptics | null = null;

export function touchHaptic(): void {
  sharedHaptics().touch();
}

/**
 * #3347: the one `Haptics` of the page, so the game's events and the drag's pick-up share a gap clock
 * (`HAPTIC_MIN_GAP_MS`) and never buzz over each other. R3348000: a touch press's `buzz` comes from it too,
 * on its own gap (`lastTap`).
 */
export function sharedHaptics(): Haptics {
  // The vibrate hook looks `navigator.vibrate` up at each buzz, as a device may gain it after this is made.
  shared ??= createHaptics({
    vibrate: (pattern) => {
      const nav = typeof navigator === "undefined" ? undefined : navigator;
      return typeof nav?.vibrate === "function" ? nav.vibrate(pattern) : false;
    },
  });
  return shared;
}

export function resetSharedHapticsForTests(): void {
  shared = null;
}

/** R3348000: a touch press's buzz, from the one player the page shares. */
export function tapHaptic(moment: HapticMoment): void {
  sharedHaptics().buzz(moment);
}

/** R3348000: a test's reset of the player `tapHaptic` buzzes through, which is the page's one. */
export function resetTapHapticsForTests(): void {
  resetSharedHapticsForTests();
}
