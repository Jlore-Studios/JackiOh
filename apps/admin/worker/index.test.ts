// @vitest-environment node
// The Worker hands the site's Worker only the routes the dashboard calls, and answers every other
// `/api/*` path 404 itself, without forwarding it.

import { describe, expect, it, vi } from "vitest";

import worker, { forwards, type Env } from "./index.ts";

const ORIGIN = "https://admin.jackioh.com";

const ALLOWED: [string, string][] = [
  ["GET", "/api/auth/me"],
  ["GET", "/api/admin/me"],
  ["GET", "/api/admin/rating"],
  ["PUT", "/api/admin/rating"],
  ["POST", "/api/admin/rating/reset"],
  ["GET", "/api/admin/leaderboard"],
  ["GET", "/api/admin/news"],
  ["POST", "/api/admin/news"],
  ["DELETE", "/api/admin/news/post-1"],
  ["GET", "/api/admin/audit"],
  ["POST", "/api/dev/xp"],
  ["POST", "/api/dev/jlore"],
  ["POST", "/api/dev/fill"],
  ["POST", "/api/dev/clear/everything"],
  ["POST", "/api/dev/bar/card"],
  ["POST", "/api/dev/quests"],
  ["POST", "/api/dev/quests/q-1/complete"],
  ["GET", "/api/quests"],
  ["GET", "/api/wallet"],
];

const REFUSED: [string, string][] = [
  // Another method on a GET-only route.
  ["POST", "/api/auth/me"],
  ["DELETE", "/api/quests"],
  ["POST", "/api/wallet"],
  // The rest of the server's surface.
  ["GET", "/api/profile"],
  ["DELETE", "/api/account"],
  ["POST", "/api/auth/signin"],
  ["GET", "/api/auth/me/extra"],
  ["POST", "/api/quests/q-1/replace"],
  ["GET", "/api/leaderboard"],
  ["GET", "/api/news"],
  ["GET", "/api/catalog"],
  ["GET", "/api/"],
  ["GET", "/api"],
  // A prefix without its slash, or one only inside the path.
  ["GET", "/api/admin"],
  ["GET", "/api/adminx/me"],
  ["POST", "/api/devx"],
  ["GET", "/x/api/admin/me"],
  // Not the server's at all: static files never reach the Worker, and the probes are the site's.
  ["GET", "/"],
  ["GET", "/healthz"],
  ["GET", "/readyz"],
  ["GET", "/ws/match"],
];

describe("forwards", () => {
  it.each(ALLOWED)("hands on %s %s", (method, path) => {
    expect(forwards(method, path)).toBe(true);
  });

  it.each(REFUSED)("refuses %s %s", (method, path) => {
    expect(forwards(method, path)).toBe(false);
  });
});

function harness(): { env: Env; site: ReturnType<typeof vi.fn> } {
  const site = vi.fn((_request: Request) => Promise.resolve(new Response("from the site", { status: 200 })));
  return { env: { SITE: { fetch: site } as unknown as Fetcher }, site };
}

/** A request as the Worker receives one; the runtime's own carries `cf` properties a test has no use for. */
const incoming = (path: string, init: RequestInit): Parameters<typeof worker.fetch>[0] =>
  new Request(`${ORIGIN}${path}`, init) as Parameters<typeof worker.fetch>[0];

const call = (env: Env, method: string, path: string): Promise<Response> => worker.fetch(incoming(path, { method }), env);

describe("the Worker's fetch", () => {
  it("hands an allowed request to the site's Worker as it came: method, path, query, headers and body", async () => {
    const { env, site } = harness();
    const request = incoming("/api/admin/leaderboard?ladder=random", {
      method: "GET",
      headers: { authorization: "Bearer token-1" },
    });
    const answer = await worker.fetch(request, env);
    expect(await answer.text()).toBe("from the site");
    expect(site).toHaveBeenCalledTimes(1);
    expect(site.mock.calls[0]?.[0]).toBe(request);
  });

  it.each(REFUSED.filter(([, path]) => path.startsWith("/api")))(
    "answers %s %s 404 in the server's error shape and never forwards it",
    async (method, path) => {
      const { env, site } = harness();
      const answer = await call(env, method, path);
      expect(answer.status).toBe(404);
      const body = (await answer.json()) as { error: { code: string; message: string } };
      expect(body.error.code).toBe("not_found");
      expect(body.error.message.length).toBeGreaterThan(0);
      expect(site).not.toHaveBeenCalled();
    },
  );

  it("never forwards a path that only reaches an allowed prefix through dot segments", async () => {
    const { env, site } = harness();
    for (const path of ["/api/admin/../account", "/api/admin/%2e%2e/account", "/api/dev/./../../profile"]) {
      const answer = await call(env, "DELETE", path);
      expect(answer.status, path).toBe(404);
    }
    expect(site).not.toHaveBeenCalled();
  });
});
