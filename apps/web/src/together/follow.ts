// The challenger's follow (#936, B48, B49): a challenge is a room, and the challenger's game starts when
// the friend joins it. The shell's toast (`ChallengeToast.tsx`) watches for that game while the server
// lists an open challenge of the player's; this covers the moments before its next read does, so a
// friend who joins at once still brings the challenger along. A sent challenge is watched for until it
// would have lapsed (`CHALLENGE_TTL_MS`), or until a game is found.

import { CHALLENGE_TTL_MS } from "@jackioh/server-config";

let followUntil = 0;

/** A challenge was just sent from this tab. */
export function followChallenge(now: number = Date.now()): void {
  followUntil = now + CHALLENGE_TTL_MS;
}

/** The game was found, or there is nothing left to wait for. */
export function stopFollowingChallenge(): void {
  followUntil = 0;
}

/** Whether a challenge this tab sent may still be waiting on its friend. */
export function followingChallenge(now: number = Date.now()): boolean {
  return now < followUntil;
}
