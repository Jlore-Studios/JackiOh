// #3619: the route boundary tells the owners of a crash, and says nothing of a lazy chunk that failed.

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { shellTestid } from "../auth/testids.ts";
import { resetClientErrorReportsForTests } from "../net/clientErrors.ts";

const { RouteErrorBoundary } = await import("../main.tsx");

function Thrower({ error }: { error: Error }): never {
  throw error;
}

let fetchMock: ReturnType<typeof vi.fn>;

beforeEach(() => {
  resetClientErrorReportsForTests();
  fetchMock = vi.fn(() => Promise.resolve(new Response("{}", { status: 200 })));
  vi.stubGlobal("fetch", fetchMock);
  vi.spyOn(console, "error").mockImplementation(() => undefined);
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("#3619 the boundary reports a crash", () => {
  it("the boundary sends one report", () => {
    const error = Object.assign(new Error("render blew up"), {
      stack: `Error: render blew up\n    at Screen (${window.location.origin}/assets/index-abc.js:1:2)`,
    });
    render(
      <RouteErrorBoundary>
        <Thrower error={error} />
      </RouteErrorBoundary>,
    );
    expect(screen.getByTestId(shellTestid.loadFailed)).toBeInTheDocument();
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const body = JSON.parse((fetchMock.mock.calls[0]?.[1] as { body: string }).body) as { trigger: string };
    expect(body.trigger).toBe("boundary");
  });

  it("a lazy chunk that failed to load sends nothing", () => {
    const error = new TypeError("Failed to fetch dynamically imported module: /assets/login-3f9a.js");
    render(
      <RouteErrorBoundary>
        <Thrower error={error} />
      </RouteErrorBoundary>,
    );
    expect(screen.getByTestId(shellTestid.loadFailed)).toBeInTheDocument();
    // R4605000: the boundary checks /version.json on a chunk failure; that static read is not a
    // report. Nothing reaches the owners: no call posts to /api/client-errors.
    const requested = fetchMock.mock.calls.map((call) => String(call[0]));
    expect(requested.filter((url) => url.includes("/api/client-errors"))).toEqual([]);
  });
});
