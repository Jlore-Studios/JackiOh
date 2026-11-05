// The dashboard's requests: same-origin, the bearer token, JSON bodies, the server's error shape read
// verbatim, and the server's own timeout.

import { describe, expect, it, vi } from "vitest";

import { adminApi, apiRequest, ApiError, ApiUnreachableError, messageOf, UNREACHABLE_MESSAGE } from "./api.ts";
import { fakeFetch } from "./test/fakes.ts";
import { API_REQUEST_TIMEOUT_SECONDS, MS_PER_SECOND } from "./wire.ts";

describe("apiRequest", () => {
  it("sends a same-origin request with the bearer token, and a JSON body only when there is one", async () => {
    const fake = fakeFetch({
      "GET /api/admin/me": { body: { profileId: "p-1" } },
      "POST /api/admin/news": { status: 201, body: { post: { id: "n-1" } } },
    });
    expect(await apiRequest("/api/admin/me", "token-1")).toEqual({ profileId: "p-1" });
    expect(await apiRequest("/api/admin/news", "token-1", "POST", { title: "T" })).toEqual({ post: { id: "n-1" } });
    const [read, write] = fake.calls;
    expect(read).toEqual({ method: "GET", url: "/api/admin/me", headers: { authorization: "Bearer token-1" }, body: undefined });
    expect(write?.headers).toEqual({ authorization: "Bearer token-1", "content-type": "application/json" });
    expect(write?.body).toEqual({ title: "T" });
  });

  it("answers null for an empty body", async () => {
    fakeFetch({ "DELETE /api/x": { status: 204 } });
    expect(await apiRequest("/api/x", "t", "DELETE")).toBeNull();
  });

  it("throws the server's refusal with its status, code and message word for word", async () => {
    fakeFetch({
      "PUT /api/admin/rating": {
        status: 400,
        body: { error: { code: "invalid_request", message: "A rating is a number from 0 to 4000." } },
      },
    });
    const refusal = await apiRequest("/api/admin/rating", "t", "PUT", { ladder: "random", rating: -1 }).catch((cause: unknown) => cause);
    expect(refusal).toBeInstanceOf(ApiError);
    expect(refusal).toMatchObject({ status: 400, code: "invalid_request", message: "A rating is a number from 0 to 4000." });
    expect(messageOf(refusal)).toBe("A rating is a number from 0 to 4000.");
  });

  it("names the status of a refusal not in the server's shape", async () => {
    vi.stubGlobal("fetch", vi.fn(() => Promise.resolve(new Response("<html>bad gateway</html>", { status: 502 }))));
    await expect(apiRequest("/api/admin/me", "t")).rejects.toMatchObject({ status: 502, code: "internal", message: "The server answered 502." });
  });

  it("calls a failed transport unreachable, without the browser's words", async () => {
    vi.stubGlobal("fetch", vi.fn(() => Promise.reject(new TypeError("Failed to fetch"))));
    const failure = await apiRequest("/api/admin/me", "t").catch((cause: unknown) => cause);
    expect(failure).toBeInstanceOf(ApiUnreachableError);
    expect(messageOf(failure)).toBe(UNREACHABLE_MESSAGE);
  });

  it("gives up, and aborts the request, after the server's API_REQUEST_TIMEOUT_SECONDS", async () => {
    vi.useFakeTimers();
    let signal: AbortSignal | undefined;
    vi.stubGlobal(
      "fetch",
      vi.fn((_path: string, init: RequestInit) => {
        signal = init.signal ?? undefined;
        return new Promise<Response>(() => undefined);
      }),
    );
    const failure = apiRequest("/api/admin/me", "t").catch((cause: unknown) => cause);
    await vi.advanceTimersByTimeAsync(API_REQUEST_TIMEOUT_SECONDS * MS_PER_SECOND - 1);
    expect(signal?.aborted).toBe(false);
    await vi.advanceTimersByTimeAsync(1);
    expect(await failure).toBeInstanceOf(ApiUnreachableError);
    expect(signal?.aborted).toBe(true);
  });
});

describe("adminApi", () => {
  /** Each call, the request it sends: method, path and body. */
  const CASES: [string, (api: ReturnType<typeof adminApi>) => Promise<unknown>, string, string, unknown][] = [
    ["me", (api) => api.me(), "GET", "/api/admin/me", undefined],
    ["rating", (api) => api.rating(), "GET", "/api/admin/rating", undefined],
    ["setRating", (api) => api.setRating("constructed", 1800), "PUT", "/api/admin/rating", { ladder: "constructed", rating: 1800 }],
    ["resetRating", (api) => api.resetRating("random"), "POST", "/api/admin/rating/reset", { ladder: "random" }],
    ["leaderboard", (api) => api.leaderboard("random"), "GET", "/api/admin/leaderboard?ladder=random", undefined],
    ["news", (api) => api.news(), "GET", "/api/admin/news", undefined],
    ["postNews", (api) => api.postNews("Title", "One sentence."), "POST", "/api/admin/news", { title: "Title", summary: "One sentence." }],
    ["takeDownNews", (api) => api.takeDownNews("a/b"), "DELETE", "/api/admin/news/a%2Fb", undefined],
    ["audit", (api) => api.audit(), "GET", "/api/admin/audit", undefined],
    ["wallet", (api) => api.wallet(), "GET", "/api/wallet", undefined],
    ["quests", (api) => api.quests(), "GET", "/api/quests", undefined],
    ["giveXp", (api) => api.giveXp("Classic", 250), "POST", "/api/dev/xp", { setId: "Classic", xp: 250 }],
    ["giveJlore", (api) => api.giveJlore(500), "POST", "/api/dev/jlore", { jlore: 500 }],
    ["fill", (api) => api.fill(), "POST", "/api/dev/fill", {}],
    ["unlockCosmetics", (api) => api.unlockCosmetics(), "POST", "/api/dev/cosmetics", {}],
    ["clear", (api) => api.clear("counters"), "POST", "/api/dev/clear/counters", {}],
    ["fillBar", (api) => api.fillBar("cosmetic"), "POST", "/api/dev/bar/cosmetic", {}],
    ["giveQuest", (api) => api.giveQuest(), "POST", "/api/dev/quests", {}],
    ["completeQuest", (api) => api.completeQuest("q 1"), "POST", "/api/dev/quests/q%201/complete", {}],
  ];

  it.each(CASES)("%s sends its route and body with a fresh token", async (_name, send, method, path, body) => {
    const fake = fakeFetch({ [`${method} ${path}`]: { body: {} } });
    const token = vi.fn(() => Promise.resolve("fresh-token"));
    await send(adminApi(token));
    expect(token).toHaveBeenCalledTimes(1);
    expect(fake.calls).toHaveLength(1);
    expect(fake.calls[0]).toMatchObject({ method, url: path, body });
    expect(fake.calls[0]?.headers["authorization"]).toBe("Bearer fresh-token");
  });
});
