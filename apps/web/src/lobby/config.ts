// The lobby screen's own numbers (CLAUDE.md rule 9): pacing, never a rule. The countdown's length is the
// server's (`LOBBY_START_COUNTDOWN_SECONDS`, R3730002), read from `@jackioh/server-config`.

/** R3730000: how often an open lobby reads itself again while the social socket is down. */
export const LOBBY_POLL_MS = 2_000;

/** How often the Start's countdown redraws. */
export const LOBBY_COUNTDOWN_TICK_MS = 250;
