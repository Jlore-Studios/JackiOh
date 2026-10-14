// The client's door onto the Duos co-op routes (#936 SPEC S6): one fetch wrapper per route, and the
// ping vocabulary the `/ws/duos` socket carries (S8). CLAUDE.md rule 7: nothing here decides a rule;
// a refusal is the server's, thrown as the repo's `ApiRequestError` with its status and code.
//
// THE TOKEN. These wrappers take none (S8's signatures): each reads the tab's session when it is
// called (`net/session.ts`), so a renewal the gate made meanwhile is the one sent, and a 401 renews
// once and retries (`callWithRenewal`, R194) as a screen's own request does. No session, or one that
// cannot be renewed, is a 401 refusal like any other.

import type { Difficulty } from "@jackioh/engine/config";

import type { DuosSeat } from "../wire/generated/DuosSeat.ts";
import type { DuosView } from "../wire/generated/DuosView.ts";
import { ApiRequestError, apiRequest } from "./api.ts";
import { callWithRenewal } from "./gate.ts";
import { readSession } from "./session.ts";

/** What a signed-out (or switched) caller is refused with, in a player's words. */
const SIGNED_OUT = "Sign in again to play together.";

/** `call` with the session's bearer, renewed once on a 401 (R194); no usable session is a 401. */
async function authed<T>(call: (token: string) => Promise<T>): Promise<T> {
  const session = readSession();
  if (session === null) throw new ApiRequestError(401, { code: "unauthorized", message: SIGNED_OUT });
  const outcome = await callWithRenewal(session.accessToken, call);
  if (outcome.kind === "ok") return outcome.value;
  throw new ApiRequestError(401, { code: "unauthorized", message: SIGNED_OUT });
}

// ---------------------------------------------------------------------------------------------
// Rooms and matches (S6)
// ---------------------------------------------------------------------------------------------

/** A player named in a room: the profile id keys it, the username is what is shown (R1436). */
export type DuosRoomPlayer = { profileId: string; username: string };

/** `GET /api/duos/rooms/:code`: the host (A1), the guest (A2) once one joined, and the match it made. */
export type DuosRoom = {
  code: string;
  difficulty: Difficulty;
  host: DuosRoomPlayer;
  guest: DuosRoomPlayer | null;
  matchId: string | null;
};

/** `POST /api/duos/rooms`: exactly one of `deckId` and `random: true`; the answer is the room's code. */
export function createDuosRoom(body: {
  difficulty: Difficulty;
  deckId?: string;
  random?: boolean;
  leanNewest?: boolean;
}): Promise<{ code: string }> {
  return authed((token) => apiRequest<{ code: string }>("/api/duos/rooms", { method: "POST", token, body }));
}

/** `GET /api/duos/rooms/:code`: 404 once the room expired (DUOS_ROOM_TTL_MS) or was cancelled. */
export function getDuosRoom(code: string): Promise<DuosRoom> {
  return authed((token) => apiRequest<DuosRoom>(`/api/duos/rooms/${encodeURIComponent(code)}`, { token }));
}

/** `POST /api/duos/rooms/:code/join`: the match it starts, or 404, or 409 `full` / `own_room`. */
export function joinDuosRoom(
  code: string,
  body: { deckId?: string; random?: boolean; leanNewest?: boolean },
): Promise<{ matchId: string }> {
  return authed((token) =>
    apiRequest<{ matchId: string }>(`/api/duos/rooms/${encodeURIComponent(code)}/join`, { method: "POST", token, body }),
  );
}

/** `DELETE /api/duos/rooms/:code`: the host takes back a room nobody has joined (204). */
export async function cancelDuosRoom(code: string): Promise<void> {
  await authed((token) => apiRequest<null>(`/api/duos/rooms/${encodeURIComponent(code)}`, { method: "DELETE", token }));
}

/** `GET /api/duos/live` (B42): the caller's live co-op match, or null. */
export function getDuosLive(): Promise<{ matchId: string | null }> {
  return authed((token) => apiRequest<{ matchId: string | null }>("/api/duos/live", { token }));
}

/** `GET /api/duos/matches/:matchId/replay` (B41): the caller's seat's view after each step; 404 to anyone else. */
export function getDuosReplay(matchId: string): Promise<{ seat: DuosSeat; steps: DuosView[] }> {
  return authed((token) =>
    apiRequest<{ seat: DuosSeat; steps: DuosView[] }>(`/api/duos/matches/${encodeURIComponent(matchId)}/replay`, {
      token,
    }),
  );
}

// ---------------------------------------------------------------------------------------------
// Pings (S6's socket frames, B39, B64): cosmetic, team-only, never in a view or the log
// ---------------------------------------------------------------------------------------------

export type PingKind = "thumbsUp" | "thumbsDown" | "portal" | "careful" | "killThis";

/** Every ping, in the order the wheel offers them. */
export const PING_KINDS: readonly PingKind[] = ["thumbsUp", "thumbsDown", "portal", "careful", "killThis"];

/**
 * What a ping points at, in one of the two duels: a hero, a unit or backrow zone (`lane` 0-based,
 * 0..4, where the board's zone testids count 1..5), or a card in a hand by its index. A hand target
 * names the sender's own team's side, whose hands the team can see (`p1` for team A).
 */
export type PingTarget =
  | { duel: 0 | 1; at: "hero"; side: "p1" | "p2" }
  | { duel: 0 | 1; at: "unit" | "backrow"; side: "p1" | "p2"; lane: number }
  | { duel: 0 | 1; at: "hand"; side: "p1" | "p2"; index: number };

/** A ping as the board shows it: who sent it, what and where, and when it landed (epoch ms, `Date.now()`). */
export type ReceivedPing = { from: DuosSeat; ping: PingKind; target: PingTarget; at: number };
