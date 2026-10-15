// Play together's numbers (#936, SPEC S7, S8). The server's are its own (`crates/server/src/config.rs`,
// read through `@jackioh/server-config`); only the client's pacing is stated here, as CLAUDE.md rule 9
// asks.

import { CHAT_POLL_MS } from "@jackioh/server-config";

/**
 * How often a social screen reads again what friends may have changed: the friends list and its
 * statuses, unread counts, and the challenge toast's challenges. One cadence for the social layer's
 * live reads, the chat's own (`CHAT_POLL_MS`), so a reply and a challenge arrive alike.
 */
export const SOCIAL_POLL_MS = CHAT_POLL_MS;

/** The watch socket (B50): how long after an unexpected close it opens again, and how often it tries. */
export const WATCH_RECONNECT_MS = 2_000;
export const WATCH_RECONNECT_ATTEMPTS = 5;
