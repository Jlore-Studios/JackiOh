// Play together's REST surface (#936, SPEC S7), one wrapper per route.
//
// CLAUDE.md rule 7: who may befriend, challenge, chat with or watch whom is the server's (B43–B53); a
// refusal is an `ApiRequestError` whose sentence a screen shows as it is. Every route is
// `AuthLevel::Active`, so no function takes a token: each reads the tab's session and renews it once on
// a 401 (R194). No session is the 401 a signed-out request gets.

import { apiRequest, type RequestOptions, type SendOptions } from "./api.ts";
import { withSession } from "./gate.ts";
import { cachedRead, rememberRead, updateCachedRead } from "./readCache.ts";
import { readSession } from "./session.ts";

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

/** A page of a conversation, newest last; `nextCursor` asks for the page before it, null when nothing older is left (R1384000). */
export type ChatPage = { messages: ChatMessage[]; nextCursor: string | null };

export type SharedReplay = { matchId: string; sharer: PlayerRef; createdAt: string; endedAt: string | null };

function social<T>(path: string, options: Omit<RequestOptions, "token"> = {}): Promise<T> {
  return withSession((token) => apiRequest<T>(path, { ...options, token }));
}

const part = encodeURIComponent;

// Friends and requests (B43–B45)

/** R2125000: the friends list's path, the key of its kept answer (`net/readCache.ts`). */
export const FRIENDS_PATH = "/api/friends";

export type FriendsList = {
  friends: Friend[];
  incoming: FriendRequest[];
  outgoing: FriendRequest[];
  blocked: PlayerRef[];
};

/** R2125000: the friends list this tab last held for the signed-in account, or undefined. */
export function cachedFriends(): FriendsList | undefined {
  const session = readSession();
  return session === null ? undefined : cachedRead<FriendsList>(session.accessToken, FRIENDS_PATH);
}

/** Kept for the next visit unless `keep` is false: the watch and the replay share are no menu that opens on its last answer. */
export function getFriends({ keep = true }: { keep?: boolean } = {}): Promise<FriendsList> {
  return withSession(async (token) => {
    const answer = await apiRequest<FriendsList>(FRIENDS_PATH, { token });
    if (keep) rememberRead(token, FRIENDS_PATH, answer);
    return answer;
  });
}

/** By exact `name#tag`. 404 `not_found` for an unknown name and a blocked pair alike (B43). */
export function sendFriendRequest(username: string): Promise<{ requestId: string }> {
  return social<{ requestId: string }>("/api/friends/requests", { method: "POST", body: { username } });
}

export function acceptFriendRequest(requestId: string): Promise<{ friend: Friend }> {
  return withSession(async (token) => {
    const answer = await apiRequest<{ friend: Friend }>(`/api/friends/requests/${part(requestId)}/accept`, {
      method: "POST",
      token,
    });
    // R2125000: the next visit shows what this answer left.
    updateCachedRead<FriendsList>(token, FRIENDS_PATH, (before) => ({
      ...before,
      friends: before.friends.some((friend) => friend.profileId === answer.friend.profileId)
        ? before.friends.map((friend) => (friend.profileId === answer.friend.profileId ? answer.friend : friend))
        : [...before.friends, answer.friend],
      incoming: before.incoming.filter((request) => request.requestId !== requestId),
    }));
    return answer;
  });
}

export async function declineFriendRequest(requestId: string): Promise<void> {
  await social<null>(`/api/friends/requests/${part(requestId)}/decline`, { method: "POST" });
}

/** The sender takes its request back. */
export async function cancelFriendRequest(requestId: string): Promise<void> {
  await social<null>(`/api/friends/requests/${part(requestId)}`, { method: "DELETE" });
}

export async function removeFriend(profileId: string, options: SendOptions = {}): Promise<void> {
  await withSession(async (token) => {
    await apiRequest<null>(`/api/friends/${part(profileId)}`, { method: "DELETE", ...options, token });
    updateCachedRead<FriendsList>(token, FRIENDS_PATH, (before) => ({
      ...before,
      friends: before.friends.filter((friend) => friend.profileId !== profileId),
    }));
  });
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

/** Sent by `usePresenceHeartbeat` every `PRESENCE_HEARTBEAT_MS`. */
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

/** The newest page, or the one before `cursor` (a `nextCursor` as answered, never built here); it marks them read. */
export function getChat(friendId: string, cursor?: string): Promise<ChatPage> {
  const query = cursor === undefined ? "" : `?${new URLSearchParams({ before: cursor }).toString()}`;
  return social<ChatPage>(`/api/chat/${part(friendId)}${query}`);
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

/** R1435000: the friends the caller has shared this replay with, who may still read it. */
export function getReplayShares(matchId: string): Promise<{ friendIds: string[] }> {
  return social<{ friendIds: string[] }>(`/api/replays/${part(matchId)}/shares`);
}

/** The replays friends have shared with the caller. */
export function getSharedReplays(): Promise<{ shared: SharedReplay[] }> {
  return social<{ shared: SharedReplay[] }>("/api/replays/shared");
}
