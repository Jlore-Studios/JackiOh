// #936 S6/S8: `net/duos.ts`, one fetch wrapper per Duos co-op route. Each test asserts what goes on
// the wire (method, path, JSON body) and what comes back: the parsed answer, or the repo's
// `ApiRequestError` carrying the refusal's status and code (B33, B34, B41, B42).

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ApiRequestError } from "./api.ts";
import { cancelDuosRoom, createDuosRoom, getDuosLive, getDuosReplay, getDuosRoom, joinDuosRoom } from "./duos.ts";
import { clearSession, writeSession } from "./session.ts";
import { must, refusal, stubServer, type Call, type Reply } from "../test/stubServer.ts";

const TOKEN = "tok-duos-1";

const HOST = { profileId: "p-me", username: "Me#1" };
const GUEST = { profileId: "f-ann", username: "Ann#2" };

beforeEach(() => {
  writeSession({ accessToken: TOKEN });
});

afterEach(() => {
  clearSession();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

async function sent(run: () => Promise<unknown>, reply: Reply): Promise<{ call: Call; result: unknown }> {
  const server = stubServer(() => reply);
  const result = await run();
  expect(server.calls, "exactly one request").toHaveLength(1);
  return { call: must(server.calls[0], "the request"), result };
}

async function refused(run: () => Promise<unknown>, reply: Reply): Promise<ApiRequestError> {
  stubServer(() => reply);
  const outcome = await run().then(
    () => null,
    (error: unknown) => error,
  );
  expect(outcome, "the wrapper rejects on a refusal").toBeInstanceOf(ApiRequestError);
  return outcome as ApiRequestError;
}

describe("B33 B34 co-op rooms", () => {
  it("B33 createDuosRoom posts its body to /api/duos/rooms and resolves with the room code", async () => {
    const body = { difficulty: "hard" as const, deckId: "d-aggro" };
    const { call, result } = await sent(() => createDuosRoom(body), { status: 201, body: { code: "HJK234" } });
    expect(call.method).toBe("POST");
    expect(call.path).toBe("/api/duos/rooms");
    expect(call.body).toEqual(body);
    expect(result).toEqual({ code: "HJK234" });
  });

  it("B33 createDuosRoom sends a random-deck room as {difficulty, random: true}, untouched", async () => {
    const body = { difficulty: "easy" as const, random: true, leanNewest: true };
    const { call } = await sent(() => createDuosRoom(body), { status: 201, body: { code: "MNP567" } });
    expect(call.body).toEqual(body);
  });

  it("B33 the request carries the signed-in session's bearer token", async () => {
    const { call } = await sent(() => createDuosRoom({ difficulty: "easy", random: true }), { status: 201, body: { code: "HJK234" } });
    expect(call.headers.authorization).toBe(`Bearer ${TOKEN}`);
  });

  it("B33 a refused room creation rejects with ApiRequestError carrying the status and code", async () => {
    const error = await refused(
      () => createDuosRoom({ difficulty: "easy", deckId: "d-missing" }),
      refusal(422, "invalid_deck", "That deck cannot be played."),
    );
    expect(error.status).toBe(422);
    expect(error.code).toBe("invalid_deck");
  });

  it("B33 a pending account's room creation is refused (rooms are AuthLevel::Active)", async () => {
    const error = await refused(
      () => createDuosRoom({ difficulty: "medium", random: true }),
      refusal(403, "account_pending", "Redeem an invite code first."),
    );
    expect(error.status).toBe(403);
    expect(error.code).toBe("account_pending");
  });

  it("B33 a pending account's join is refused (rooms are AuthLevel::Active)", async () => {
    const error = await refused(() => joinDuosRoom("HJK234", { random: true }), refusal(403, "account_pending", "Redeem an invite code first."));
    expect(error.status).toBe(403);
    expect(error.code).toBe("account_pending");
  });

  it("B33 getDuosRoom reads GET /api/duos/rooms/:code and resolves with the room", async () => {
    const room = { code: "HJK234", difficulty: "medium", host: HOST, guest: null, matchId: null };
    const { call, result } = await sent(() => getDuosRoom("HJK234"), { status: 200, body: room });
    expect(call.method).toBe("GET");
    expect(call.path).toBe("/api/duos/rooms/HJK234");
    expect(result).toEqual(room);
  });

  it("B33 getDuosRoom resolves with the guest and the match once the guest has joined", async () => {
    const room = { code: "HJK234", difficulty: "medium", host: HOST, guest: GUEST, matchId: "dm-1" };
    const { result } = await sent(() => getDuosRoom("HJK234"), { status: 200, body: room });
    expect(result).toEqual(room);
  });

  it("B34 an unknown or expired room rejects with 404", async () => {
    const error = await refused(() => getDuosRoom("ZZZ999"), refusal(404, "not_found", "No such room."));
    expect(error.status).toBe(404);
  });

  it("B33 joinDuosRoom posts its body to /api/duos/rooms/:code/join and resolves with the match", async () => {
    const body = { deckId: "d-control" };
    const { call, result } = await sent(() => joinDuosRoom("HJK234", body), { status: 200, body: { matchId: "dm-1" } });
    expect(call.method).toBe("POST");
    expect(call.path).toBe("/api/duos/rooms/HJK234/join");
    expect(call.body).toEqual(body);
    expect(result).toEqual({ matchId: "dm-1" });
  });

  it("B33 a third account joining a full room rejects with 409 full", async () => {
    const error = await refused(() => joinDuosRoom("HJK234", { random: true }), refusal(409, "full", "That room is full."));
    expect(error.status).toBe(409);
    expect(error.code).toBe("full");
  });

  it("B33 the host joining their own room rejects with 409 own_room", async () => {
    const error = await refused(() => joinDuosRoom("HJK234", { random: true }), refusal(409, "own_room", "That is your own room."));
    expect(error.status).toBe(409);
    expect(error.code).toBe("own_room");
  });

  it("B34 joining a room that does not exist (or expired) rejects with 404", async () => {
    const error = await refused(() => joinDuosRoom("ZZZ999", { random: true }), refusal(404, "not_found", "No such room."));
    expect(error.status).toBe(404);
  });

  it("B34 cancelDuosRoom deletes /api/duos/rooms/:code", async () => {
    const { call, result } = await sent(() => cancelDuosRoom("HJK234"), { status: 204 });
    expect(call.method).toBe("DELETE");
    expect(call.path).toBe("/api/duos/rooms/HJK234");
    expect(result).toBeFalsy();
  });

  it("B34 cancelling a room once a guest has joined (or not the host) rejects with the server's status", async () => {
    const error = await refused(() => cancelDuosRoom("HJK234"), refusal(409, "conflict", "The match has started."));
    expect(error.status).toBe(409);
  });
});

describe("B42 the live co-op match", () => {
  it("B42 getDuosLive reads GET /api/duos/live and resolves with the live match", async () => {
    const { call, result } = await sent(() => getDuosLive(), { status: 200, body: { matchId: "dm-1" } });
    expect(call.method).toBe("GET");
    expect(call.path).toBe("/api/duos/live");
    expect(result).toEqual({ matchId: "dm-1" });
  });

  it("B42 getDuosLive resolves with a null match when none is live", async () => {
    const { result } = await sent(() => getDuosLive(), { status: 200, body: { matchId: null } });
    expect(result).toEqual({ matchId: null });
  });

  it("B42 a refused live read rejects rather than resolving to no match", async () => {
    const error = await refused(() => getDuosLive(), refusal(401, "unauthorized", "Sign in first."));
    expect(error.status).toBe(401);
  });
});

describe("B41 the co-op replay", () => {
  it("B41 getDuosReplay reads GET /api/duos/matches/:matchId/replay and resolves with the seat and its steps", async () => {
    const answer = { seat: "a2", steps: [] };
    const { call, result } = await sent(() => getDuosReplay("dm-1"), { status: 200, body: answer });
    expect(call.method).toBe("GET");
    expect(call.path).toBe("/api/duos/matches/dm-1/replay");
    expect(result).toEqual(answer);
  });

  it("B41 a non-player asking for the replay rejects with 404", async () => {
    const error = await refused(() => getDuosReplay("dm-1"), refusal(404, "not_found", "No such match."));
    expect(error.status).toBe(404);
    expect(error.code).toBe("not_found");
  });
});
