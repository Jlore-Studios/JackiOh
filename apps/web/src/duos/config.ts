// The Duos client's own numbers (CLAUDE.md rule 9): pacing and presentation, never a rule. The
// server's come from `@jackioh/server-config`, the engine's from `@jackioh/engine/config`.

/** How long the card bar (B62, B64) stays once the pointer has left its card, so the pointer can reach it. */
export const DUOS_HOVER_GRACE_MS = 350;

/** How often the co-op panel asks whether its open room has been joined (B60). */
export const COOP_ROOM_POLL_MS = 2_000;

/** The pings a board keeps, newest last; anything older faded long ago (PING_SHOW_MS). */
export const DUOS_PINGS_KEPT = 16;

export const DUOS_CLOCK_TICK_MS = 250;

/** Where a tab remembers the co-op room it opened, so the panel picks the wait up after a reload (B60). */
export const COOP_ROOM_KEY = "jackioh.duos.room";

/** Where the device remembers the Both boards choice across reloads (R2095000). */
export const DUOS_BOTH_KEY = "jackioh.duos.both";
