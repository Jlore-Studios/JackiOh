// Play together's REST surface (#936, SPEC S7): friends, requests, blocks, presence, challenges,
// chat and shared replays, one wrapper per route and typed with S7's types.
//
// CLAUDE.md rule 7: nothing here decides a rule. Who may befriend, challenge, chat with or watch whom
// is the server's (B43–B53); a refusal is thrown as `ApiRequestError` (`net/api.ts`) carrying the
// status and the body's code (`not_found`, `self`, `already_friends`, `busy`, `too_long`, …) and the
// server's own sentence, which a screen shows as it is.
//
// Every route is `AuthLevel::Active` and the caller is the signed-in account, so no function takes a
// token: each reads the tab's session (`net/session.ts`) and renews it once on a 401 (R194,
// `callWithRenewal`). No session, or a session that has ended, is the 401 a signed-out request gets.

import { apiRequest, type RequestOptions } from "./api.ts";
import { withSession } from "./gate.ts";

/** Another player as the social routes name one: the id for requests and keys, the username to show (R1436). */
export type PlayerRef = { profileId: string; username: string };

/** B47: `inMatch` while a live 1v1 match, series or Duos match holds the friend. `watchable`: B50. */
export type Friend = PlayerRef & {
  status: "offline" | "online" | "inMatch";
  matchId: string | null;
  watchable: boolean;
  since: string;
};

export type FriendRequest = { requestId: string; from: PlayerRef; to: PlayerRef; createdAt: string };

export type Challenge = {
  challengeId: string;
  from: PlayerRef;
  to: PlayerRef;
  roomCode: string;
  mode: "bo1" | "bo3" | "random";
  expiresAt: string;
};

/** `from` and `to` are profile ids. */
export type ChatMessage = { id: string; from: string; to: string; body: string; createdAt: string; read: boolean };

export type SharedReplay = { matchId: string; sharer: PlayerRef; createdAt: string; endedAt: string | null };

/** One social request as the signed-in account, renewed once on a 401 (`withSession`). */
function social<T>(path: string, options: Omit<RequestOptions, "token"> = {}): Promise<T> {
  return withSession((token) => apiRequest<T>(path, { ...options, token }));
}

const part = encodeURIComponent;

// Friends and requests (B43–B45)

export function getFriends(): Promise<{
  friends: Friend[];
  incoming: FriendRequest[];
  outgoing: FriendRequest[];
  blocked: PlayerRef[];
}> {
  return social<{ friends: Friend[]; incoming: FriendRequest[]; outgoing: FriendRequest[]; blocked: PlayerRef[] }>(
    "/api/friends",
  );
}

/** By exact `name#tag`. 404 `not_found` for an unknown name and a blocked pair alike (B43). */
export function sendFriendRequest(username: string): Promise<{ requestId: string }> {
  return social<{ requestId: string }>("/api/friends/requests", { method: "POST", body: { username } });
}

export function acceptFriendRequest(requestId: string): Promise<{ friend: Friend }> {
  return social<{ friend: Friend }>(`/api/friends/requests/${part(requestId)}/accept`, { method: "POST" });
}

export async function declineFriendRequest(requestId: string): Promise<void> {
  await social<null>(`/api/friends/requests/${part(requestId)}/decline`, { method: "POST" });
}

/** The sender takes its request back. */
export async function cancelFriendRequest(requestId: string): Promise<void> {
  await social<null>(`/api/friends/requests/${part(requestId)}`, { method: "DELETE" });
}

export async function removeFriend(profileId: string): Promise<void> {
  await social<null>(`/api/friends/${part(profileId)}`, { method: "DELETE" });
}

// Blocks (B46)

/** Also ends the friendship and any request either way. */
export async function blockPlayer(profileId: string): Promise<void> {
  await social<null>("/api/blocks", { method: "POST", body: { profileId } });
}

export async function unblockPlayer(profileId: string): Promise<void> {
  await social<null>(`/api/blocks/${part(profileId)}`, { method: "DELETE" });
}

// Presence (B47)

/** The heartbeat `usePresenceHeartbeat` sends every `PRESENCE_HEARTBEAT_MS`. */
export async function heartbeat(): Promise<void> {
  await social<null>("/api/presence", { method: "POST" });
}

// Challenges (B48, B49)

/** The body `POST /api/rooms` takes (a `ModeChoice`), plus the friend. */
export function createChallenge(
  body: Record<string, unknown> & { friendId: string },
): Promise<{ challengeId: string; roomCode: string }> {
  return social<{ challengeId: string; roomCode: string }>("/api/challenges", { method: "POST", body });
}

/** The caller's open, unexpired challenges, both ways. */
export function getChallenges(): Promise<{ incoming: Challenge[]; outgoing: Challenge[] }> {
  return social<{ incoming: Challenge[]; outgoing: Challenge[] }>("/api/challenges");
}

export async function declineChallenge(challengeId: string): Promise<void> {
  await social<null>(`/api/challenges/${part(challengeId)}/decline`, { method: "POST" });
}

/** The challenger takes it back; the server frees the room. */
export async function cancelChallenge(challengeId: string): Promise<void> {
  await social<null>(`/api/challenges/${part(challengeId)}`, { method: "DELETE" });
}

// Chat (B53)

/** A page of the conversation, newest last, before `before` (an ISO time) when given; it marks them read. */
export function getChat(friendId: string, before?: string): Promise<{ messages: ChatMessage[] }> {
  const query = before === undefined ? "" : `?${new URLSearchParams({ before }).toString()}`;
  return social<{ messages: ChatMessage[] }>(`/api/chat/${part(friendId)}${query}`);
}

export function sendChat(friendId: string, body: string): Promise<{ message: ChatMessage }> {
  return social<{ message: ChatMessage }>(`/api/chat/${part(friendId)}`, { method: "POST", body: { body } });
}

/** Unread messages by the friend who sent them. */
export function getUnread(): Promise<{ unread: Record<string, number> }> {
  return social<{ unread: Record<string, number> }>("/api/chat/unread");
}

export async function reportMessage(messageId: string, reason: string): Promise<void> {
  await social<null>(`/api/chat/messages/${part(messageId)}/report`, { method: "POST", body: { reason } });
}

// Shared replays (B52)

export async function shareReplay(matchId: string, friendId: string): Promise<void> {
  await social<null>(`/api/replays/${part(matchId)}/shares`, { method: "POST", body: { friendId } });
}

export async function unshareReplay(matchId: string, friendId: string): Promise<void> {
  await social<null>(`/api/replays/${part(matchId)}/shares/${part(friendId)}`, { method: "DELETE" });
}

/** The replays friends have shared with the caller. */
export function getSharedReplays(): Promise<{ shared: SharedReplay[] }> {
  return social<{ shared: SharedReplay[] }>("/api/replays/shared");
}
