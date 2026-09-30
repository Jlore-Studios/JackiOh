/**
 * `DELETE /api/account` (src/api/auth.ts): a player deletes their own account.
 *
 * The contract the web client builds against: the route is authenticated like every other
 * (`user`, so a pending account can leave too), answers 204 with no body when the account is
 * gone, and answers the API's one error shape otherwise. What the delete does to each table in
 * Postgres is `test/sql/06_account_deletion.sql` and `test/db/contract.ts`; here the question is
 * the route's own: who may call it, what it refuses, and the order it does its two deletes in.
 */

import { SignJWT } from "jose";
import { beforeEach, describe, expect, it } from "vitest";

import { createAuthRoutes, createSupabaseAuth, type AdminDeletion, type AdminLookup } from "../../src/api/auth";
import { ApiError, createRouter, type Router } from "../../src/api/http";
import type { ResultRow, SeriesRow } from "../../src/api/ports";
import { createTestDeps, jsonRequest, readJson, type TestDeps } from "../fakes/deps";

type ErrorBody = { error: { code: string; message: string } };

const PROFILE = "p-leaving";
const OTHER = "p-staying";

let deps: TestDeps;
let router: Router;
let token: string;

function profileWith(target: TestDeps, id: string, status: "active" | "pending" | "banned"): string {
  const userId = `user-${id}`;
  target.store.seedProfile({ id, userId, status });
  return target.auth.addUser({ userId, email: `${id}@example.test` });
}

function remove(bearer: string | null = token): Promise<Response> {
  return router(jsonRequest("DELETE", "/api/account", undefined, bearer === null ? {} : { token: bearer }));
}

async function errorOf(response: Response): Promise<ErrorBody["error"]> {
  return (await readJson<ErrorBody>(response)).error;
}

function resultRow(matchId: string, winner: string): ResultRow {
  return {
    matchId,
    players: [PROFILE, OTHER],
    winnerProfileId: winner,
    reason: "concede",
    turns: 4,
    endedAt: deps.timers.now(),
    ratingBefore: [1000, 1000],
    ratingAfter: [1016, 984],
  };
}

beforeEach(() => {
  deps = createTestDeps();
  router = createRouter(createAuthRoutes(), deps);
  token = profileWith(deps, PROFILE, "active");
  profileWith(deps, OTHER, "active");
});

describe("DELETE /api/account", () => {
  it("is declared `user`, beside the other account routes", () => {
    const declared = createAuthRoutes().find((entry) => entry.method === "DELETE" && entry.path === "/api/account");
    expect(declared?.auth).toBe("user");
  });

  it("answers 204 with no body, removes the profile's own rows, and signs the token out", async () => {
    const at = deps.timers.now();
    await deps.store.decks.upsert(
      { id: "deck-1", profileId: PROFILE, name: "Mine", cards: [], catalogVersion: "test-1", createdAt: at, updatedAt: at },
      10,
    );
    await deps.store.tutorial.merge({ profileId: PROFILE, completed: ["basics"], hiddenChoice: null, at }, 32);
    await deps.store.collection.upsertQuantities(PROFILE, [{ cardId: "core-001", quantity: 1 }]);
    await deps.store.tickets.insert({
      id: "ticket-1",
      profileId: PROFILE,
      rating: 1000,
      mode: "bo1",
      deck: [],
      trio: null,
      catalogVersion: "test-1",
      enqueuedAt: at,
      status: "open",
      matchId: null,
    });
    await deps.store.rooms.create({
      code: "ROOM22",
      hostProfileId: PROFILE,
      mode: "bo1",
      hostDeck: [],
      hostTrio: null,
      catalogVersion: "test-1",
      createdAt: at,
      expiresAt: at + 60_000,
      guestProfileId: null,
      matchId: null,
    });

    const response = await remove();
    expect(response.status).toBe(204);
    expect(await response.text()).toBe("");

    expect(await deps.store.profiles.getById(PROFILE)).toBeNull();
    expect(await deps.store.decks.list(PROFILE)).toEqual([]);
    expect(await deps.store.tutorial.get(PROFILE)).toBeNull();
    expect(await deps.store.collection.get(PROFILE)).toEqual([]);
    expect(await deps.store.tickets.openForProfile(PROFILE)).toBeNull();
    expect(await deps.store.rooms.get("ROOM22")).toBeNull();
    expect(await deps.auth.verifyAccessToken(token)).toBeNull();

    // The same token is refused from now on, as any unknown token is.
    const again = await remove();
    expect(again.status).toBe(401);
    expect(deps.log.entries.some((entry) => entry.event === "account.deleted")).toBe(true);
  });

  it("keeps the other player's record: a finished match they lost is still a loss", async () => {
    await deps.store.results.insert(resultRow("match-1", PROFILE));
    expect((await remove()).status).toBe(204);
    expect(await deps.store.results.recordFor(OTHER)).toEqual({ wins: 0, losses: 1, draws: 0 });
  });

  it("lets a pending account delete itself too", async () => {
    const pending = profileWith(deps, "p-pending", "pending");
    expect((await remove(pending)).status).toBe(204);
    expect(await deps.store.profiles.getById("p-pending")).toBeNull();
  });

  it("refuses a caller with no token with 401 and the usual error shape", async () => {
    const response = await remove(null);
    expect(response.status).toBe(401);
    expect((await errorOf(response)).code).toBe("unauthorized");
  });

  it("refuses a player in a live match with 409 and deletes nothing", async () => {
    await deps.store.profiles.setInMatch(PROFILE, "match-live");
    const response = await remove();
    expect(response.status).toBe(409);
    const error = await errorOf(response);
    expect(error.code).toBe("already_in_match");
    expect(error.message).toMatch(/concede/i);
    expect(await deps.store.profiles.getById(PROFILE)).not.toBeNull();
    expect(await deps.auth.verifyAccessToken(token)).not.toBeNull();
  });

  it("refuses a player in a Conquest series that is not over with 409 and deletes nothing", async () => {
    const series = deps.store.series;
    deps.store.series = {
      ...series,
      activeFor: async (profileId) => (profileId === PROFILE ? ({ id: "series-1" } as SeriesRow) : null),
    };
    const response = await remove();
    expect(response.status).toBe(409);
    expect((await errorOf(response)).code).toBe("conflict");
    expect(await deps.store.profiles.getById(PROFILE)).not.toBeNull();
  });

  it("answers 503 on a server whose auth provider cannot delete users, and deletes nothing", async () => {
    delete (deps.auth as { deleteUser?: unknown }).deleteUser;
    const response = await remove();
    expect(response.status).toBe(503);
    expect((await errorOf(response)).code).toBe("unavailable");
    expect(await deps.store.profiles.getById(PROFILE)).not.toBeNull();
  });

  it("can be retried when the provider fails after the profile is gone", async () => {
    const realDelete = deps.auth.deleteUser;
    deps.auth.deleteUser = async () => {
      throw new ApiError("unavailable", "try again");
    };
    const failed = await remove();
    expect(failed.status).toBe(503);
    expect(await deps.store.profiles.getById(PROFILE)).toBeNull();

    // The sign-in still works, so the next call gets a fresh pending profile and removes that.
    deps.auth.deleteUser = realDelete;
    expect((await remove()).status).toBe(204);
    expect(await deps.store.profiles.getByUserId(`user-${PROFILE}`)).toBeNull();
    expect(await deps.auth.verifyAccessToken(token)).toBeNull();
  });
});

describe("the Supabase provider's deleteUser", () => {
  const PROJECT_URL = "https://project.supabase.test";
  const SECRET = "hs256-secret-used-only-by-this-test";

  async function tokenFor(userId: string): Promise<string> {
    return new SignJWT({ email: `${userId}@example.test` })
      .setProtectedHeader({ alg: "HS256" })
      .setSubject(userId)
      .setIssuer(`${PROJECT_URL}/auth/v1`)
      .setAudience("authenticated")
      .sign(new TextEncoder().encode(SECRET));
  }

  function provider(deletion: () => AdminDeletion): {
    auth: ReturnType<typeof createSupabaseAuth>;
    deleted: string[];
    lookup: (next: AdminLookup) => void;
  } {
    const deleted: string[] = [];
    let reply: AdminLookup = {
      kind: "ok",
      user: { id: "user-a", email: "user-a@example.test", email_confirmed_at: "2026-01-01T00:00:00.000Z" },
    };
    const auth = createSupabaseAuth({
      url: PROJECT_URL,
      secretKey: "secret-key",
      jwtSecret: SECRET,
      keySet: () => {
        throw new Error("this test publishes no JWKS");
      },
      fetchImpl: () => {
        throw new Error("no network in a unit test");
      },
      clientFactory: () => ({
        password: null,
        admin: {
          getUserById: async () => reply,
          deleteUser: async (userId) => {
            deleted.push(userId);
            return deletion();
          },
        },
      }),
    });
    return { auth, deleted, lookup: (next) => (reply = next) };
  }

  it("deletes through the admin API and stops honouring the user's token at once", async () => {
    const { auth, deleted, lookup } = provider(() => "deleted");
    const token = await tokenFor("user-a");
    // Verified once, so the confirmed email is remembered for a while.
    expect((await auth.verifyAccessToken(token))?.emailVerified).toBe(true);

    await auth.deleteUser?.("user-a");
    expect(deleted).toEqual(["user-a"]);
    lookup({ kind: "missing" });
    // Without forgetting the remembered email this would still verify until the memory lapsed.
    expect(await auth.verifyAccessToken(token)).toBeNull();
  });

  it("counts a user that is already gone as deleted", async () => {
    const { auth } = provider(() => "missing");
    await expect(auth.deleteUser?.("user-a")).resolves.toBeUndefined();
  });

  it("throws unavailable when the provider does not answer", async () => {
    const { auth } = provider(() => "unavailable");
    await expect(auth.deleteUser?.("user-a")).rejects.toMatchObject({ code: "unavailable" });
  });
});
