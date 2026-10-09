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

/** The hold at an effects speed, never below the two fades. */
export function showcaseHoldMs(speed: number): number {
  return Math.max(2 * SHOWCASE_FADE_MS, Math.round(SHOWCASE_HOLD_MS / normalizeSpeed(speed)));
}

/**
 * R502: a card cast on draw stays up longer than a play, divided by the effects speed, and never past
 * SHOWCASE_CAST_HOLD_CAP_MS: it holds practice's AI (`data-showcase`) until PRACTICE_SHOWCASE_HOLD_MAX_MS
 * (practice/config.ts), which the cap stays under.
 */
export const SHOWCASE_CAST_HOLD_MS = 1800;
export const SHOWCASE_CAST_HOLD_CAP_MS = 3600;

export function showcaseCastHoldMs(speed: number): number {
  return Math.min(SHOWCASE_CAST_HOLD_CAP_MS, Math.max(2 * SHOWCASE_FADE_MS, Math.round(SHOWCASE_CAST_HOLD_MS / normalizeSpeed(speed))));
}

/**
 * A card another card cast stays up until the runner reaches the next cast, and never longer than
 * this (divided by the effects speed, capped like a cast on draw). `CAST_BUDGET_MS` gives each cast
 * about two seconds, so the card is up for the whole of its beat.
 */
export const SHOWCASE_MULTICAST_HOLD_MS = 2200;

export function showcaseMulticastHoldMs(speed: number): number {
  return Math.min(SHOWCASE_CAST_HOLD_CAP_MS, Math.max(2 * SHOWCASE_FADE_MS, Math.round(SHOWCASE_MULTICAST_HOLD_MS / normalizeSpeed(speed))));
}

/** R502: how long the card takes to burst out of its drawer's Deck pile, inside the hold (showcase.css); none under reduced motion. */
export const SHOWCASE_BURST_MS = 420;

/**
 * R502, R436: a cast on draw, a Call to Chaos roll and every card another card casts wait for the
 * animation runner to reach their event. The longest they wait past the runner's last progress: a
 * whole burst at the slowest effects speed, and one hold more. Every entry the runner starts restarts
 * the wait while anything is gated, so a long burst (Jogg's Box's ten casts) never times out. The
 * runner going idle, draining or resetting lets them go at once.
 */
export const SHOWCASE_GATE_MAX_MS = Math.ceil(BURST_BUDGET_MS / FX_SPEED_MIN) + SHOWCASE_HOLD_MS;

/** R502: the words a cast on draw is held up with. */
export const CAST_ON_DRAW_TEXT = {
  ribbon: "Cast on draw!",
  you: "You drew",
  opponent: "Opponent drew",
  hidden: "Opponent drew a card",
  said: "cast on draw",
} as const;

/** The words a card another card cast is held up with. */
export const MULTICAST_TEXT = {
  /** "Cast by Jogg's Box" (the caster's name), or this when the view hides the caster. */
  by: "Cast by",
  unknown: "Cast by a card",
  /** The ordinal badge's prefix and what the live region calls the cast: "Jogg's Box cast Fireball (3)". */
  ordinal: "#",
  said: "cast",
} as const;

/** R436: the words the static reveal and the live region say a roll with. */
export const CHAOS_TEXT = { title: "Call to Chaos", said: "Call to Chaos rolled" } as const;

/**
 * Test ids. None starts with `card-` or `hand-card-`, so `cy.fieldCardByName` and
 * `cy.handCardByName` can never resolve to the showcase (B20). e2e/support/testids.ts mirrors them.
 */
export const showcaseTestid = {
  /** The showcase itself, present only while a play is held up. Carries `data-showcase`. */
  root: "showcase",
  caption: "showcase-caption",
  /** The face of a card the view names. */
  face: "showcase-face",
  /** The back drawn for a card the view hides (R97, R227). */
  back: "showcase-back",
  /** R370: on that back, the cost the view gives a card set face down. */
  cost: "showcase-cost",
  /** The polite live region that says what was played; always mounted, empty between plays. */
  live: "showcase-live",
  /** R502: the "Cast on draw!" ribbon across a card cast as it was drawn. */
  ribbon: "showcase-ribbon",
  /** On a card another card cast, which of its casts it is; `data-ordinal` carries the number. */
  ordinal: "showcase-ordinal",
  /**
   * R436: the still list of what a Call to Chaos rolled, drawn only while the effects layer draws
   * nothing (reduced motion, or the effects off); one `chaosLine` per effect.
   */
  chaos: "chaos-banner",
  chaosLine: "chaos-banner-line",
  /** R436: the polite live region that says what a Call to Chaos rolled, on both seats, in every mode. */
  chaosLive: "chaos-live",
} as const;

/**
 * `data-showcase`: what is being held up. The practice route holds the AI's next step while it is set.
 * "cast" is a card cast as it was drawn, on either seat (R502); "multicast" a card another card cast.
 */
export type ShowcaseKind = "played" | "set" | "hidden" | "cast" | "multicast";
