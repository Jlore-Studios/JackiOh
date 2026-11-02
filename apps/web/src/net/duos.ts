// The client's door onto the Duos co-op routes (#936 SPEC S6): one fetch wrapper per route, and the
// ping vocabulary the `/ws/duos` socket carries (S8). CLAUDE.md rule 7: nothing here decides a rule;
// a refusal is the server's, thrown as the repo's `ApiRequestError` with its status and code. The
// wrappers take no token (S8's signatures): `withSession` reads the tab's session at each call (R194).

import type { Difficulty } from "@jackioh/engine/config";
import type { LeanPick } from "@jackioh/shared";

import type { DuosSeat } from "../wire/generated/DuosSeat.ts";
import type { DuosView } from "../wire/generated/DuosView.ts";
import { apiRequest } from "./api.ts";
import { withSession } from "./gate.ts";

// Rooms and matches (S6)

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

/**
 * S6: exactly one of `deckId` and `random: true`; a random deal may carry the player's "More cards
 * from" pick for their own deck (`leanSet`, R924000, R2031000), absent for none.
 */
export type DuosDeckChoice = { deckId?: string; random?: boolean; leanSet?: LeanPick };

/** `POST /api/duos/rooms`. */
export function createDuosRoom(body: { difficulty: Difficulty } & DuosDeckChoice): Promise<{ code: string }> {
  return withSession((token) => apiRequest<{ code: string }>("/api/duos/rooms", { method: "POST", token, body }));
}

/** `GET /api/duos/rooms/:code`: 404 once the room expired (DUOS_ROOM_TTL_MS) or was canceled. */
export function getDuosRoom(code: string): Promise<DuosRoom> {
  return withSession((token) => apiRequest<DuosRoom>(`/api/duos/rooms/${encodeURIComponent(code)}`, { token }));
}

/** `POST /api/duos/rooms/:code/join`: the match it starts, or 404, or 409 `full` / `own_room`. */
export function joinDuosRoom(code: string, body: DuosDeckChoice): Promise<{ matchId: string }> {
  return withSession((token) =>
    apiRequest<{ matchId: string }>(`/api/duos/rooms/${encodeURIComponent(code)}/join`, { method: "POST", token, body }),
  );
}

/** `DELETE /api/duos/rooms/:code`: the host takes back a room nobody has joined (204). */
export async function cancelDuosRoom(code: string): Promise<void> {
  await withSession((token) => apiRequest<null>(`/api/duos/rooms/${encodeURIComponent(code)}`, { method: "DELETE", token }));
}

/** `GET /api/duos/live` (B42): the caller's live co-op match, or null. */
export function getDuosLive(): Promise<{ matchId: string | null }> {
  return withSession((token) => apiRequest<{ matchId: string | null }>("/api/duos/live", { token }));
}

/** `GET /api/duos/matches/:matchId/replay` (B41): the caller's seat's view after each step; 404 to anyone else. */
export function getDuosReplay(matchId: string): Promise<{ seat: DuosSeat; steps: DuosView[] }> {
  return withSession((token) =>
    apiRequest<{ seat: DuosSeat; steps: DuosView[] }>(`/api/duos/matches/${encodeURIComponent(matchId)}/replay`, {
      token,
    }),
  );
}

/** `POST /api/duos/matches/:matchId/again` (R2092000): the new match's id once both seats offered, else null. */
export type DuosAgainOffer = { matchId: string | null };

/** `GET /api/duos/matches/:matchId/again` (R2092000). */
export type DuosAgainStatus = { youOffered: boolean; teammateOffered: boolean; teammateHere: boolean; matchId: string | null };

/** R2092000: record this seat's Play again offer, carrying the device's lean pick for a dealt seat (`leanSet`, R924000, R2031000). */
export function offerDuosAgain(matchId: string, lean: LeanPick | null): Promise<DuosAgainOffer> {
  return withSession((token) =>
    apiRequest<DuosAgainOffer>(`/api/duos/matches/${encodeURIComponent(matchId)}/again`, {
      method: "POST",
      token,
      body: lean === null ? {} : { leanSet: lean },
    }),
  );
}

export function getDuosAgain(matchId: string): Promise<DuosAgainStatus> {
  return withSession((token) => apiRequest<DuosAgainStatus>(`/api/duos/matches/${encodeURIComponent(matchId)}/again`, { token }));
}

// Pings (S6's socket frames, B39, B64): cosmetic, team-only, never in a view or the log

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

/** A ping as the board shows it; `at` is when it landed (epoch ms, `Date.now()`). */
export type ReceivedPing = { from: DuosSeat; ping: PingKind; target: PingTarget; at: number };
