// The client's door onto the private lobbies (SPEC §9.5, §13.9; R3730000 to R3730006): one fetch wrapper
// per route. CLAUDE.md rule 7: nothing here decides a rule; the seats, the picks, Ready and the Start are
// the server's, and a refusal is the repo's `ApiRequestError`, its reason in `details.reason`. The wrappers
// take no token: `withSession` reads the tab's session at each call (R194).

import type { Difficulty } from "@jackioh/engine/config";
import type { LeanPick } from "@jackioh/shared";

import { apiRequest, type ModeChoice, type QueueMode } from "../net/api.ts";
import { withSession } from "../net/gate.ts";
import type { PlayerRef } from "../net/social.ts";

/** R3730003: what the host sets, everyone sees: a 1v1 lobby's mode or a co-op lobby's difficulty. */
export type LobbySettings = { format: "duel"; mode: QueueMode } | { format: "coop"; difficulty: Difficulty };

export type LobbyFormat = LobbySettings["format"];

/** A seat's deck state (R3730001): nothing picked yet, picked, or picked and ready. */
export type LobbyDeckState = "choosing" | "chosen" | "ready";

/** One seat as the server shows it; a 1v1 opponent's `deck` is null whatever they picked (rule 7). */
export type LobbySeat = {
  role: "host" | "guest";
  player: (PlayerRef & { status: "offline" | "online" | "inMatch" }) | null;
  state: LobbyDeckState;
  deck: string | null;
  random: boolean;
};

/** A co-op pick: a saved deck or a random one, with its lean (R924000). */
export type CoopPick = { deckId: string } | { random: true; leanSet?: LeanPick };

/** The caller's own pick, as they sent it: a 1v1 pick is the queue's choice. */
export type LobbyPick = ModeChoice;

/** `GET /api/lobbies/:code` for a lobby (R3730000). */
export type LobbyView = {
  kind: "lobby";
  code: string;
  settings: LobbySettings;
  expiresAt: number;
  you: "host" | "guest" | null;
  yourPick: LobbyPick | null;
  seats: [LobbySeat, LobbySeat];
  startedAt: number | null;
  matchId: string | null;
  seriesId: string | null;
};

/** R3730006: with no lobby under the code, the classic 1v1 room still open there. */
export type RoomView = { kind: "room"; code: string; mode: QueueMode; host: PlayerRef };

/** R3730006: or the unjoined co-op room. */
export type CoopRoomView = { kind: "coopRoom"; code: string; difficulty: Difficulty; host: PlayerRef };

export type LobbyRead = LobbyView | RoomView | CoopRoomView;

/** `GET /api/lobbies`: the caller's lobbies not started, newest first (R3730004). */
export type MyLobby = { code: string; settings: LobbySettings; you: "host" | "guest"; expiresAt: number };

function lobbyPath(code: string, verb?: string): string {
  const base = `/api/lobbies/${encodeURIComponent(code)}`;
  return verb === undefined ? base : `${base}/${verb}`;
}

function post<T>(path: string, body: unknown = {}): Promise<T> {
  return withSession((token) => apiRequest<T>(path, { method: "POST", token, body }));
}

/** `POST /api/lobbies`: `201 {code}`. */
export function createLobby(settings: LobbySettings): Promise<{ code: string }> {
  return post<{ code: string }>("/api/lobbies", settings);
}

export function getMyLobbies(): Promise<{ lobbies: MyLobby[] }> {
  return withSession((token) => apiRequest<{ lobbies: MyLobby[] }>("/api/lobbies", { token }));
}

/** 404 for a lobby closed, full, started or blocked, and for a code nothing holds. */
export function getLobby(code: string): Promise<LobbyRead> {
  return withSession((token) => apiRequest<LobbyRead>(lobbyPath(code), { token }));
}

export async function enterLobby(code: string): Promise<void> {
  await post<unknown>(lobbyPath(code, "enter"));
}

export async function leaveLobby(code: string): Promise<void> {
  await post<unknown>(lobbyPath(code, "leave"));
}

export async function removeFromLobby(code: string, profileId: string): Promise<void> {
  await post<unknown>(lobbyPath(code, "remove"), { profileId });
}

/** The host's: `{mode}` for a 1v1 lobby, `{difficulty}` for a co-op one (R3730003). */
export async function setLobbySettings(code: string, change: { mode: QueueMode } | { difficulty: Difficulty }): Promise<void> {
  await post<unknown>(lobbyPath(code, "settings"), change);
}

/** R3730001: a 1v1 lobby takes the queue's choice, a co-op lobby a co-op room's. */
export async function pickLobby(code: string, pick: ModeChoice | CoopPick): Promise<void> {
  await post<unknown>(lobbyPath(code, "pick"), pick);
}

export async function readyLobby(code: string, ready: boolean): Promise<void> {
  await post<unknown>(lobbyPath(code, "ready"), { ready });
}

/** R3730005: a friend into the empty seat, a live toast. */
export async function inviteToLobby(code: string, profileId: string): Promise<void> {
  await post<unknown>(lobbyPath(code, "invite"), { profileId });
}

/** R3730002: the host's Start, once both seats are ready. */
export function startLobby(code: string): Promise<{ matchId: string | null; seriesId: string | null }> {
  return post<{ matchId: string | null; seriesId: string | null }>(lobbyPath(code, "start"));
}
