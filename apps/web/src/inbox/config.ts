// The inbox's numbers (R3729002, CLAUDE.md rule 9). The server's are its own, read through
// `@jackioh/server-config`; only the client's pacing and its own keeping are stated here.

import { DUOS_ROOM_TTL_MS } from "@jackioh/server-config";

/** How often the lists the inbox reads beside the social read (daily games, friend requests, shared replays) are read again. */
export const INBOX_POLL_MS = 60_000;

/**
 * The least time between two unforced reads of them: a screen that mounts the bell, or a social read that
 * lands, asks no oftener than this. A change of the player's own (an answer, the panel opening) reads at once.
 */
export const INBOX_MIN_GAP_MS = 30_000;

/**
 * How long a friend's lobby invite stays in the inbox: as long as the room it names lives. The `lobbyInvite`
 * frame carries no expiry, and the server answers a lapsed room with its own words when it is opened.
 */
export const INBOX_INVITE_KEEP_MS = DUOS_ROOM_TTL_MS;

/** Where this browser keeps the shared replays the player has seen listed, so their badge counts them once. */
export const INBOX_SEEN_KEY = "jackioh.inbox.seen";

/** The most seen replays kept; the server lists at most 50, so the oldest marks go first. */
export const INBOX_SEEN_MAX = 100;

/** How often an open inbox and the bell redraw the time a challenge has left (a challenge lives for two minutes). */
export const INBOX_TICK_MS = 1_000;

/** The clock a challenge's time left reads on: sixty seconds to the minute, the seconds in two digits ("1:05 left"). */
export const SECONDS_PER_MINUTE = 60;
export const INBOX_CLOCK_DIGITS = 2;
