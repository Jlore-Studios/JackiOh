// Play together's numbers (#936, SPEC S7, S8). The server's are its own (`crates/server/src/config.rs`,
// read through `@jackioh/server-config`); only the client's pacing is stated here (CLAUDE.md rule 9).

import { PRESENCE_HEARTBEAT_MS, SOCIAL_POLL_MS } from "@jackioh/server-config";

/**
 * Once a heartbeat, the pace a friend's status changes at. The read is the social layer's dearest, so no
 * screen asks faster, and off Play together only while the friends panel is open (`FriendsButton.tsx`).
 */
export const FRIENDS_POLL_MS = PRESENCE_HEARTBEAT_MS;

/**
 * The server's, which the e2e specs wait on too: one read of the unread counts and challenges per tab,
 * however many screens show them (`socialPoll.ts`, #1831). Open chat keeps its own (`CHAT_POLL_MS`).
 */
export { SOCIAL_POLL_MS };

/** The watch socket (B50): how long after an unexpected close it opens again, and how often it tries. */
export const WATCH_RECONNECT_MS = 2_000;
export const WATCH_RECONNECT_ATTEMPTS = 5;
