// Every number and test id the opponent's-play showcase uses (CLAUDE.md rule 9).

import { BURST_BUDGET_MS } from "../animations.ts";
import { FX_SPEED_MIN } from "../../fx/constants.ts";
import { normalizeSpeed } from "../../fx/settings.ts";

/** How long the opponent's played card stays up at the default effects speed, divided by the effects speed (R201). */
export const SHOWCASE_HOLD_MS = 1000;

/** The most plays waiting their turn: a hotseat hand-over brings a whole turn's at once; the rest are in the log. */
export const SHOWCASE_QUEUE_MAX = 3;

/** The fade in and the fade out, inside the hold (showcase.css); none under reduced motion. */
export const SHOWCASE_FADE_MS = 160;

export function showcaseHoldMs(speed: number): number {
  return Math.max(2 * SHOWCASE_FADE_MS, Math.round(SHOWCASE_HOLD_MS / normalizeSpeed(speed)));
}

/** R502: cap cast-on-draw holds below the practice AI's `data-showcase` maximum (practice/config.ts). */
export const SHOWCASE_CAST_HOLD_MS = 1800;
export const SHOWCASE_CAST_HOLD_CAP_MS = 3600;

export function showcaseCastHoldMs(speed: number): number {
  return Math.min(SHOWCASE_CAST_HOLD_CAP_MS, Math.max(2 * SHOWCASE_FADE_MS, Math.round(SHOWCASE_CAST_HOLD_MS / normalizeSpeed(speed))));
}

/** A card cast by another holds until the next cast, with the cast-on-draw cap. */
export const SHOWCASE_MULTICAST_HOLD_MS = 2200;

export function showcaseMulticastHoldMs(speed: number): number {
  return Math.min(SHOWCASE_CAST_HOLD_CAP_MS, Math.max(2 * SHOWCASE_FADE_MS, Math.round(SHOWCASE_MULTICAST_HOLD_MS / normalizeSpeed(speed))));
}

/** R502: how long the card takes to burst out of its drawer's Deck pile, inside the hold (showcase.css); none under reduced motion. */
export const SHOWCASE_BURST_MS = 420;

/** R502, R436: runner-gated entries wait through a full slow burst; every entry restarts the gate. */
export const SHOWCASE_GATE_MAX_MS = Math.ceil(BURST_BUDGET_MS / FX_SPEED_MIN) + SHOWCASE_HOLD_MS;

/** R502: the words a cast on draw is held up with. */
export const CAST_ON_DRAW_TEXT = {
  ribbon: "Cast on draw!",
  you: "You drew",
  opponent: "Opponent drew",
  hidden: "Opponent drew a card",
  said: "cast on draw",
} as const;

export const MULTICAST_TEXT = {
  by: "Cast by",
  unknown: "Cast by a card",
  ordinal: "#",
  said: "cast",
} as const;

/** R436: the words the static reveal and the live region say a roll with. */
export const CHAOS_TEXT = { title: "Call to Chaos", said: "Call to Chaos rolled" } as const;

/** Test ids avoid card prefixes (B20); e2e/support/testids.ts mirrors them. */
export const showcaseTestid = {
  root: "showcase",
  caption: "showcase-caption",
  face: "showcase-face",
  /** The back drawn for a card the view hides (R97, R227). */
  back: "showcase-back",
  /** R370: on that back, the cost the view gives a card set face down. */
  cost: "showcase-cost",
  live: "showcase-live",
  /** R502: the "Cast on draw!" ribbon across a card cast as it was drawn. */
  ribbon: "showcase-ribbon",
  ordinal: "showcase-ordinal",
  /** R436: static Call to Chaos result, one line per effect, when effects do not draw. */
  chaos: "chaos-banner",
  chaosLine: "chaos-banner-line",
  /** R436: the polite live region that says what a Call to Chaos rolled, on both seats, in every mode. */
  chaosLive: "chaos-live",
} as const;

/** `data-showcase` holds the practice AI; casts apply to either seat (R502). */
export type ShowcaseKind = "played" | "set" | "hidden" | "cast" | "multicast";
