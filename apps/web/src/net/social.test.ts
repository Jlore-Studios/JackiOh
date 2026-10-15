// #936 S7/S8: `net/social.ts`, one fetch wrapper per social route. Each test asserts what goes on the
// wire (method, path, query, JSON body) and what comes back: the parsed answer, or the repo's
// `ApiRequestError` carrying the refusal's status and code (B43, B45, B46, B48, B52, B53).
//
// The wrappers take no token (SPEC S8), so the signed-in session is put where `net/session.ts`
// keeps it before each test.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ApiRequestError } from "./api.ts";
import { clearSession, writeSession } from "./session.ts";
import {
  acceptFriendRequest,
  blockPlayer,
  cancelChallenge,
  cancelFriendRequest,
  createChallenge,
  declineChallenge,
  declineFriendRequest,
  getChallenges,
  getChat,
  getFriends,
  getSharedReplays,
  getUnread,
  heartbeat,
  removeFriend,
  reportMessage,
  sendChat,
  sendFriendRequest,
  shareReplay,
  unblockPlayer,
  unshareReplay,
} from "./social.ts";
import type { Challenge, ChatMessage, Friend, FriendRequest, PlayerRef, SharedReplay } from "./social.ts";
import { must, refusal, stubServer, type Call, type Reply } from "../test/stubServer.ts";

const TOKEN = "tok-social-1";

const ME: PlayerRef = { profileId: "p-me", username: "Me#1" };
const ANN: PlayerRef = { profileId: "f-ann", username: "Ann#2" };

const ANN_FRIEND: Friend = { ...ANN, status: "online", matchId: null, watchable: false, since: "2026-10-01T00:00:00.000Z" };
const REQUEST: FriendRequest = { requestId: "rq-1", from: ANN, to: ME, createdAt: "2026-10-14T09:00:00.000Z" };
const CHALLENGE: Challenge = {
  challengeId: "ch-1",
  from: ANN,
  to: ME,
  roomCode: "HJK234",
  mode: "bo1",
  expiresAt: "2026-10-14T09:02:00.000Z",
};
const MESSAGE: ChatMessage = {
  id: "msg-1",
  from: ME.profileId,
  to: ANN.profileId,
  body: "gg",
  createdAt: "2026-10-14T09:00:00.000Z",
  read: false,
};
const SHARED: SharedReplay = { matchId: "m-1", sharer: ANN, createdAt: "2026-10-14T09:00:00.000Z", endedAt: "2026-10-14T08:50:00.000Z" };

beforeEach(() => {
  writeSession({ accessToken: TOKEN });
});

afterEach(() => {
  clearSession();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

/** Run one wrapper against a server that answers `reply`; the one request it made, and its result. */
async function sent(run: () => Promise<unknown>, reply: Reply): Promise<{ call: Call; result: unknown }> {
  const server = stubServer(() => reply);
  const result = await run();
  expect(server.calls, "exactly one request").toHaveLength(1);
  return { call: must(server.calls[0], "the request"), result };
}

/** Run one wrapper against a server that refuses it; the error it rejected with. */
async function refused(run: () => Promise<unknown>, reply: Reply): Promise<ApiRequestError> {
  stubServer(() => reply);
  const outcome = await run().then(
    () => null,
    (error: unknown) => error,
  );
  expect(outcome, "the wrapper rejects on a refusal").toBeInstanceOf(ApiRequestError);
  return outcome as ApiRequestError;
}

/** A route whose S7 body is "—": nothing, or at most an empty object. */
function expectNoBody(call: Call): void {
  expect(call.body ?? {}).toEqual({});
}

const NO_CONTENT: Reply = { status: 204 };

describe("B43 B44 B45 friend requests", () => {
  it("B43 sendFriendRequest posts {username} to /api/friends/requests and resolves with the request id", async () => {
    const { call, result } = await sent(() => sendFriendRequest("Ann#2"), { status: 201, body: { requestId: "rq-9" } });
    expect(call.method).toBe("POST");
    expect(call.path).toBe("/api/friends/requests");
    expect(call.body).toEqual({ username: "Ann#2" });
    expect(result).toEqual({ requestId: "rq-9" });
  });

  it("B43 the request carries the signed-in session's bearer token", async () => {
    const { call } = await sent(() => sendFriendRequest("Ann#2"), { status: 201, body: { requestId: "rq-9" } });
    expect(call.headers.authorization).toBe(`Bearer ${TOKEN}`);
  });

  it("B43 an unknown name (or a block either way) rejects with 404 not_found", async () => {
    const error = await refused(() => sendFriendRequest("Nobody#9"), refusal(404, "not_found", "No player by that name."));
    expect(error.status).toBe(404);
    expect(error.code).toBe("not_found");
  });

  it.each(["self", "already_friends", "already_requested", "friend_cap"])(
    "B43 a 409 %s refusal rejects carrying status 409 and that code",
    async (code) => {
      const error = await refused(() => sendFriendRequest("Ann#2"), refusal(409, code, `refused: ${code}`));
      expect(error.status).toBe(409);
      expect(error.code).toBe(code);
    },
  );

  it("B43 more requests than the hourly limit reject with 429", async () => {
    const error = await refused(() => sendFriendRequest("Ann#2"), refusal(429, "rate_limited", "Slow down."));
    expect(error.status).toBe(429);
    expect(error.code).toBe("rate_limited");
  });

  it("B45 acceptFriendRequest posts to /api/friends/requests/:id/accept and resolves with the friend", async () => {
    const { call, result } = await sent(() => acceptFriendRequest("rq-1"), { status: 200, body: { friend: ANN_FRIEND } });
    expect(call.method).toBe("POST");
    expect(call.path).toBe("/api/friends/requests/rq-1/accept");
    expectNoBody(call);
    expect(result).toEqual({ friend: ANN_FRIEND });
  });

  it("B45 accepting a request that is gone rejects with 404", async () => {
    const error = await refused(() => acceptFriendRequest("rq-gone"), refusal(404, "not_found", "No such request."));
    expect(error.status).toBe(404);
  });

  it("B45 declineFriendRequest posts to /api/friends/requests/:id/decline", async () => {
    const { call, result } = await sent(() => declineFriendRequest("rq-1"), NO_CONTENT);
    expect(call.method).toBe("POST");
    expect(call.path).toBe("/api/friends/requests/rq-1/decline");
    expectNoBody(call);
    expect(result).toBeFalsy();
  });

  it("B45 declining a request that is gone rejects with 404", async () => {
    const error = await refused(() => declineFriendRequest("rq-gone"), refusal(404, "not_found", "No such request."));
    expect(error.status).toBe(404);
  });

  it("B45 cancelFriendRequest deletes /api/friends/requests/:id", async () => {
    const { call, result } = await sent(() => cancelFriendRequest("rq-1"), NO_CONTENT);
    expect(call.method).toBe("DELETE");
    expect(call.path).toBe("/api/friends/requests/rq-1");
    expect(result).toBeFalsy();
  });

  it("B45 removeFriend deletes /api/friends/:profileId", async () => {
    const { call } = await sent(() => removeFriend("f-ann"), NO_CONTENT);
    expect(call.method).toBe("DELETE");
    expect(call.path).toBe("/api/friends/f-ann");
  });

  it("B45 a refused cancel rejects with ApiRequestError carrying the status and code", async () => {
    const error = await refused(() => cancelFriendRequest("rq-gone"), refusal(404, "not_found", "No such request."));
    expect(error.status).toBe(404);
    expect(error.code).toBe("not_found");
  });

  it("B45 a refused removal rejects with ApiRequestError carrying the status and code", async () => {
    const error = await refused(() => removeFriend("p-stranger"), refusal(404, "not_found", "Not a friend."));
    expect(error.status).toBe(404);
    expect(error.code).toBe("not_found");
  });

  it("B55 getFriends reads GET /api/friends and resolves with its four lists", async () => {
    const answer = { friends: [ANN_FRIEND], incoming: [REQUEST], outgoing: [], blocked: [] };
    const { call, result } = await sent(() => getFriends(), { status: 200, body: answer });
    expect(call.method).toBe("GET");
    expect(call.path).toBe("/api/friends");
    expect(result).toEqual(answer);
  });

  it("B55 a refused read of the friends list rejects with the server's status, never resolves empty", async () => {
    const error = await refused(() => getFriends(), refusal(403, "account_pending", "Redeem an invite code first."));
    expect(error.status).toBe(403);
    expect(error.code).toBe("account_pending");
  });
});

describe("B46 blocks", () => {
  it("B46 blockPlayer posts {profileId} to /api/blocks", async () => {
    const { call, result } = await sent(() => blockPlayer("f-ann"), NO_CONTENT);
    expect(call.method).toBe("POST");
    expect(call.path).toBe("/api/blocks");
    expect(call.body).toEqual({ profileId: "f-ann" });
    expect(result).toBeFalsy();
  });

  it("B46 unblockPlayer deletes /api/blocks/:profileId", async () => {
    const { call } = await sent(() => unblockPlayer("f-ann"), NO_CONTENT);
    expect(call.method).toBe("DELETE");
    expect(call.path).toBe("/api/blocks/f-ann");
  });

  it("B46 a refused block rejects with ApiRequestError carrying the status and code", async () => {
    const error = await refused(() => blockPlayer("p-me"), refusal(422, "invalid", "You cannot block yourself."));
    expect(error.status).toBe(422);
    expect(error.code).toBe("invalid");
  });

  it("B46 a refused unblock rejects with ApiRequestError carrying the status and code", async () => {
    const error = await refused(() => unblockPlayer("p-nobody"), refusal(404, "not_found", "Not blocked."));
    expect(error.status).toBe(404);
  });
});

describe("B47 presence", () => {
  it("B47 heartbeat posts to /api/presence with no body", async () => {
    const { call, result } = await sent(() => heartbeat(), NO_CONTENT);
    expect(call.method).toBe("POST");
    expect(call.path).toBe("/api/presence");
    expectNoBody(call);
    expect(result).toBeFalsy();
  });

  it("B47 a refused heartbeat rejects with ApiRequestError", async () => {
    const error = await refused(() => heartbeat(), refusal(401, "unauthorized", "Sign in first."));
    expect(error.status).toBe(401);
  });
});

describe("B48 B49 challenges", () => {
  it("B48 createChallenge posts the room body plus friendId to /api/challenges and resolves with the challenge and room", async () => {
    const body = { mode: "bo1", deckId: "d-aggro", friendId: "f-ann" };
    const { call, result } = await sent(() => createChallenge(body), {
      status: 201,
      body: { challengeId: "ch-1", roomCode: "HJK234" },
    });
    expect(call.method).toBe("POST");
    expect(call.path).toBe("/api/challenges");
    expect(call.body).toEqual(body);
    expect(result).toEqual({ challengeId: "ch-1", roomCode: "HJK234" });
  });

  it("B48 challenging someone who is not a friend rejects with 404", async () => {
    const error = await refused(
      () => createChallenge({ mode: "random", friendId: "p-stranger" }),
      refusal(404, "not_found", "Not a friend."),
    );
    expect(error.status).toBe(404);
  });

  it.each(["busy", "offline", "already_challenged"])("B48 a 409 %s refusal rejects carrying status 409 and that code", async (code) => {
    const error = await refused(() => createChallenge({ mode: "random", friendId: "f-ann" }), refusal(409, code, `refused: ${code}`));
    expect(error.status).toBe(409);
    expect(error.code).toBe(code);
  });

  it("B49 getChallenges reads GET /api/challenges and resolves with incoming and outgoing", async () => {
    const answer = { incoming: [CHALLENGE], outgoing: [] };
    const { call, result } = await sent(() => getChallenges(), { status: 200, body: answer });
    expect(call.method).toBe("GET");
    expect(call.path).toBe("/api/challenges");
    expect(result).toEqual(answer);
  });

  it("B49 declineChallenge posts to /api/challenges/:id/decline", async () => {
    const { call, result } = await sent(() => declineChallenge("ch-1"), NO_CONTENT);
    expect(call.method).toBe("POST");
    expect(call.path).toBe("/api/challenges/ch-1/decline");
    expectNoBody(call);
    expect(result).toBeFalsy();
  });

  it("B49 cancelChallenge deletes /api/challenges/:id", async () => {
    const { call } = await sent(() => cancelChallenge("ch-1"), NO_CONTENT);
    expect(call.method).toBe("DELETE");
    expect(call.path).toBe("/api/challenges/ch-1");
  });

  it("B49 declining a challenge that is gone rejects with 404", async () => {
    const error = await refused(() => declineChallenge("ch-gone"), refusal(404, "not_found", "No such challenge."));
    expect(error.status).toBe(404);
  });

  it("B49 cancelling a challenge that is gone rejects with 404", async () => {
    const error = await refused(() => cancelChallenge("ch-gone"), refusal(404, "not_found", "No such challenge."));
    expect(error.status).toBe(404);
    expect(error.code).toBe("not_found");
  });

  it("B49 a refused read of the challenges rejects rather than resolving empty", async () => {
    const error = await refused(() => getChallenges(), refusal(401, "unauthorized", "Sign in first."));
    expect(error.status).toBe(401);
    expect(error.code).toBe("unauthorized");
  });
});

describe("B53 chat", () => {
  it("B53 getChat reads GET /api/chat/:friendId with no query when no cursor is given", async () => {
    const { call, result } = await sent(() => getChat("f-ann"), { status: 200, body: { messages: [MESSAGE] } });
    expect(call.method).toBe("GET");
    expect(call.path).toBe("/api/chat/f-ann");
    expect(call.query.has("before")).toBe(false);
    expect(result).toEqual({ messages: [MESSAGE] });
  });

  it("B53 getChat passes its cursor as ?before=<iso>", async () => {
    const before = "2026-10-14T08:00:00.000Z";
    const { call } = await sent(() => getChat("f-ann", before), { status: 200, body: { messages: [] } });
    expect(call.path).toBe("/api/chat/f-ann");
    expect(call.query.get("before")).toBe(before);
  });

  it("B53 sendChat posts {body} to /api/chat/:friendId and resolves with the stored message", async () => {
    const { call, result } = await sent(() => sendChat("f-ann", "gg"), { status: 201, body: { message: MESSAGE } });
    expect(call.method).toBe("POST");
    expect(call.path).toBe("/api/chat/f-ann");
    expect(call.body).toEqual({ body: "gg" });
    expect(result).toEqual({ message: MESSAGE });
  });

  it.each(["empty", "too_long"])("B53 a 422 %s refusal rejects carrying status 422 and that code", async (code) => {
    const error = await refused(() => sendChat("f-ann", code === "empty" ? "   " : "x".repeat(501)), refusal(422, code, `refused: ${code}`));
    expect(error.status).toBe(422);
    expect(error.code).toBe(code);
  });

  it("B53 chat to someone who is not a friend (or blocked) rejects with 404", async () => {
    const error = await refused(() => sendChat("p-stranger", "hi"), refusal(404, "not_found", "Not a friend."));
    expect(error.status).toBe(404);
  });

  it("B53 more messages than the per-minute limit reject with 429", async () => {
    const error = await refused(() => sendChat("f-ann", "hi"), refusal(429, "rate_limited", "Slow down."));
    expect(error.status).toBe(429);
  });

  it("B53 getUnread reads GET /api/chat/unread", async () => {
    const { call, result } = await sent(() => getUnread(), { status: 200, body: { unread: { "f-ann": 3 } } });
    expect(call.method).toBe("GET");
    expect(call.path).toBe("/api/chat/unread");
    expect(result).toEqual({ unread: { "f-ann": 3 } });
  });

  it("B53 reportMessage posts {reason} to /api/chat/messages/:id/report", async () => {
    const { call, result } = await sent(() => reportMessage("msg-1", "spam"), NO_CONTENT);
    expect(call.method).toBe("POST");
    expect(call.path).toBe("/api/chat/messages/msg-1/report");
    expect(call.body).toEqual({ reason: "spam" });
    expect(result).toBeFalsy();
  });

  it("B53 reading a conversation with someone who is not a friend rejects with the server's status", async () => {
    const error = await refused(() => getChat("p-stranger"), refusal(404, "not_found", "Not a friend."));
    expect(error.status).toBe(404);
    expect(error.code).toBe("not_found");
  });

  it("B53 a refused unread read rejects rather than resolving to no unread", async () => {
    const error = await refused(() => getUnread(), refusal(401, "unauthorized", "Sign in first."));
    expect(error.status).toBe(401);
  });

  it("B53 reporting a message that is not in the reporter's conversations rejects with the server's status", async () => {
    const error = await refused(() => reportMessage("msg-other", "spam"), refusal(404, "not_found", "No such message."));
    expect(error.status).toBe(404);
    expect(error.code).toBe("not_found");
  });
});

describe("B52 replay shares", () => {
  it("B52 shareReplay posts {friendId} to /api/replays/:matchId/shares", async () => {
    const { call, result } = await sent(() => shareReplay("m-1", "f-ann"), NO_CONTENT);
    expect(call.method).toBe("POST");
    expect(call.path).toBe("/api/replays/m-1/shares");
    expect(call.body).toEqual({ friendId: "f-ann" });
    expect(result).toBeFalsy();
  });

  it("B52 sharing a match that is not yours, or with a non-friend, rejects with 404", async () => {
    const error = await refused(() => shareReplay("m-other", "f-ann"), refusal(404, "not_found", "No such match."));
    expect(error.status).toBe(404);
  });

  it("B52 unshareReplay deletes /api/replays/:matchId/shares/:friendId", async () => {
    const { call } = await sent(() => unshareReplay("m-1", "f-ann"), NO_CONTENT);
    expect(call.method).toBe("DELETE");
    expect(call.path).toBe("/api/replays/m-1/shares/f-ann");
  });

  it("B52 getSharedReplays reads GET /api/replays/shared", async () => {
    const { call, result } = await sent(() => getSharedReplays(), { status: 200, body: { shared: [SHARED] } });
    expect(call.method).toBe("GET");
    expect(call.path).toBe("/api/replays/shared");
    expect(result).toEqual({ shared: [SHARED] });
  });

  it("B52 a refused unshare rejects with ApiRequestError carrying the status and code", async () => {
    const error = await refused(() => unshareReplay("m-other", "f-ann"), refusal(404, "not_found", "No such share."));
    expect(error.status).toBe(404);
    expect(error.code).toBe("not_found");
  });

  it("B52 a refused read of the shared replays rejects rather than resolving empty", async () => {
    const error = await refused(() => getSharedReplays(), refusal(403, "account_pending", "Redeem an invite code first."));
    expect(error.status).toBe(403);
    expect(error.code).toBe("account_pending");
  });
});
