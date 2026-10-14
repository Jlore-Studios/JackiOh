// The Duos client's own numbers (CLAUDE.md rule 9): pacing and presentation, never a rule. The
// server's (the round clock, the ping limits, PING_SHOW_MS) come from `@jackioh/server-config`, the
// engine's (DUOS_OWN_HEALTH) from `@jackioh/engine/config`.

/**
 * How long the card bar (the ping wheel and the pass control, B62, B64) stays once the pointer has
 * left its card, so the pointer can travel onto the bar without it closing under it.
 */
export const DUOS_HOVER_GRACE_MS = 350;

/** How often the co-op panel asks whether its open room has been joined (B60). */
export const COOP_ROOM_POLL_MS = 2_000;

/** The `/ws/duos` reconnect backoff, as `game/net.ts`'s for `/ws/match`; the last entry repeats. */
export const DUOS_RECONNECT_DELAYS_MS: readonly number[] = [250, 500, 1000, 2000, 5000];

/** R1441's jitter: each backoff gains up to this fraction of itself, drawn at random. */
export const DUOS_RECONNECT_JITTER = 0.25;

/** The close codes that settle a refusal (no reconnect), as `/ws/match`'s. */
export const DUOS_REFUSAL_CLOSE_CODES: readonly number[] = [4401, 4403, 4404];

/** R679's voided match: nothing to reconnect to. */
export const DUOS_VOIDED_CLOSE_CODE = 4410;

/** The pings a board keeps, newest last; anything older faded long ago (PING_SHOW_MS). */
export const DUOS_PINGS_KEPT = 16;

/** How often the round clock redraws its seconds. */
export const DUOS_CLOCK_TICK_MS = 250;

/** Where a tab remembers the co-op room it opened, so the panel picks the wait up after a reload (B60). */
export const COOP_ROOM_KEY = "jackioh.duos.room";
