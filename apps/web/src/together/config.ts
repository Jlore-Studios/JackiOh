// Play together's numbers (#936, SPEC S7, S8). The server's are its own (`crates/server/src/config.rs`,
// read through `@jackioh/server-config`); only the client's pacing is stated here, as CLAUDE.md rule 9
// asks.

import { PRESENCE_HEARTBEAT_MS, SOCIAL_POLL_MS } from "@jackioh/server-config";

/**
 * How often a screen that shows the friends list (statuses, requests, the blocked list) reads it
 * again: once a heartbeat (`PRESENCE_HEARTBEAT_MS`), the pace a friend's status changes at. A read is
 * one transaction on the server however many friends there are, but it is still the social layer's
 * dearest, so no screen asks for it faster, and a match screen asks only while its chat list is open.
 */
export const FRIENDS_POLL_MS = PRESENCE_HEARTBEAT_MS;

/**
 * How often the unread counts and the challenges are read again (the challenge toast, on every
 * signed-in screen, and Play together): the server's `SOCIAL_POLL_MS`, which the e2e specs wait on too.
 * An open conversation keeps its own, faster pace (`CHAT_POLL_MS`, `ChatPanel.tsx`).
 */
export { SOCIAL_POLL_MS };

/** The watch socket (B50): how long after an unexpected close it opens again, and how often it tries. */
export const WATCH_RECONNECT_MS = 2_000;
export const WATCH_RECONNECT_ATTEMPTS = 5;
