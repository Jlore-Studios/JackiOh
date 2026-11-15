// #3619: a crash in this browser reaches the owners: three triggers, one report each, no hidden information.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  CLIENT_ERROR_PATH,
  CLIENT_ERROR_REPORTS_PER_PAGE,
  installClientErrorReports,
  reportClientError,
  resetClientErrorReportsForTests,
  routeOf,
} from "./clientErrors.ts";
import { clearSession, writeSession } from "./session.ts";

const KEYS = ["trigger", "message", "stack", "route", "catalogVersion"];

function failure(message = "boom", stack?: string): Error {
  return Object.assign(new Error(message), {
    stack: stack ?? `Error: ${message}\n    at f (${window.location.origin}/assets/index-abc.js:1:2)`,
  });
}

let fetchMock: ReturnType<typeof vi.fn>;

function sentBodies(): Record<string, unknown>[] {
  return fetchMock.mock.calls.map((call) => JSON.parse((call[1] as { body: string }).body) as Record<string, unknown>);
}

beforeEach(() => {
  resetClientErrorReportsForTests();
  fetchMock = vi.fn(() => Promise.resolve(new Response(JSON.stringify({ reported: true }), { status: 200 })));
  vi.stubGlobal("fetch", fetchMock);
  installClientErrorReports();
});

afterEach(() => {
  vi.unstubAllGlobals();
  clearSession();
  window.history.replaceState(null, "", "/");
});

describe("#3619 a crash reaches the owners", () => {
  it("an uncaught error sends one report", () => {
    window.dispatchEvent(new ErrorEvent("error", { error: failure() }));
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(String(fetchMock.mock.calls[0]?.[0])).toContain(CLIENT_ERROR_PATH);
    expect((fetchMock.mock.calls[0]?.[1] as { method: string }).method).toBe("POST");
    const body = sentBodies()[0] ?? {};
    expect(body.trigger).toBe("error");
    expect(body.message).toBe("boom");
    for (const key of Object.keys(body)) expect(KEYS).toContain(key);
  });

  it("an unhandled rejection sends one report", () => {
    window.dispatchEvent(Object.assign(new Event("unhandledrejection"), { reason: failure("rejected") }));
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const body = sentBodies()[0] ?? {};
    expect(body.trigger).toBe("unhandledrejection");
    expect(body.message).toBe("rejected");
  });

  it("a report carries no hidden information", () => {
    window.history.replaceState(null, "", "/lobby/ABCD?room=SECRET#frag");
    writeSession({ accessToken: "token-xyz" });
    reportClientError("boundary", failure());
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const init = fetchMock.mock.calls[0]?.[1] as { body: string; headers: Record<string, string> };
    for (const secret of ["ABCD", "SECRET", "frag", "token-xyz"]) expect(init.body).not.toContain(secret);
    expect(sentBodies()[0]?.route).toBe("/lobby/:id");
    expect(init.headers.authorization).toBe("Bearer token-xyz");
  });

  it("chunk load failures and extension errors send nothing", () => {
    reportClientError(
      "boundary",
      new TypeError("Failed to fetch dynamically imported module: /assets/login-3f9a.js"),
    );
    reportClientError(
      "error",
      failure("from an extension", "Error: x\n    at g (chrome-extension://abcdef/content.js:1:2)"),
    );
    reportClientError("error", failure("no stack", ""));
    reportClientError("unhandledrejection", "a string");
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("at most CLIENT_ERROR_REPORTS_PER_PAGE reports go out, and a repeat counts once", () => {
    for (let n = 0; n < 5; n += 1) reportClientError("error", failure(`distinct ${n}`));
    expect(fetchMock).toHaveBeenCalledTimes(CLIENT_ERROR_REPORTS_PER_PAGE);

    fetchMock.mockClear();
    resetClientErrorReportsForTests();
    reportClientError("error", failure("same"));
    reportClientError("error", failure("same"));
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it("a failing report is silent", async () => {
    fetchMock.mockImplementation(() => Promise.reject(new TypeError("Failed to fetch")));
    const unhandled = vi.fn();
    window.addEventListener("unhandledrejection", unhandled);
    reportClientError("error", failure());
    for (let turn = 0; turn < 5; turn += 1) await Promise.resolve();
    window.removeEventListener("unhandledrejection", unhandled);
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(unhandled).not.toHaveBeenCalled();
  });

  it("routeOf names a pattern, never an id", () => {
    expect(routeOf("/play")).toBe("/play");
    expect(routeOf("/dungeon/stats")).toBe("/dungeon/stats");
    expect(routeOf("/match/xyz")).toBe("/match/:id");
    expect(routeOf("/nothing/here/now")).toBe("unknown");
  });
});
