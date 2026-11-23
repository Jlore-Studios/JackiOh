// A screen whose code fails to load (docs/polish/5-sign-in.md, B40): every route but the landing page
// is a lazy chunk, whose import fails on a flaky network and after a deploy for a tab left open.

import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { clearConsumedAuthRedirect } from "../auth/redirect.ts";
import { shellTestid } from "../auth/testids.ts";
import { navigate, paths } from "../net/navigate.ts";
import { UPDATE_RELOAD_KEY, reloadPage } from "../net/update.ts";

vi.mock("./login.tsx", () => {
  throw new TypeError("Failed to fetch dynamically imported module: /assets/login-3f9a.js");
});

// jsdom's `location.reload` is a non-writable own property, so the reload a stale chunk failure
// becomes is watched through this seam instead.
vi.mock("../net/update.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/update.ts")>();
  return { ...actual, reloadPage: vi.fn() };
});

const reloaded = vi.mocked(reloadPage);

const { App, RouteErrorBoundary } = await import("../main.tsx");

afterEach(() => {
  cleanup();
  clearConsumedAuthRedirect();
  vi.restoreAllMocks();
});

describe("B40 a screen that fails to load", () => {
  it("B40 shows a panel with a reload and a real link home instead of a blank page", async () => {
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    window.history.replaceState(null, "", paths.login);
    render(<App />);

    const panel = await screen.findByTestId(shellTestid.loadFailed);
    expect(panel.textContent).toMatch(/didn't load/);
    expect(screen.getByTestId(shellTestid.reload)).toBeInTheDocument();
    // A real link, not an in-app move: a fresh page load fetches the current build.
    expect(screen.getByTestId(shellTestid.loadFailedHome)).toHaveAttribute("href", paths.landing);
  });

  it("B40 moving to another screen clears the panel", async () => {
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    window.history.replaceState(null, "", paths.login);
    render(<App />);
    await screen.findByTestId(shellTestid.loadFailed);

    act(() => {
      navigate(paths.landing);
    });
    expect(screen.queryByTestId(shellTestid.loadFailed)).toBeNull();
  });
});

describe("R193 an emailed link on /login whose screen fails to load", () => {
  it("R193 its tokens are gone from the address bar at the first render, and stay gone after the failure", async () => {
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    const fragment = new URLSearchParams({
      access_token: "header.payload.signature",
      refresh_token: "live-refresh-token-from-the-email",
      expires_in: "3600",
      token_type: "bearer",
      type: "signup",
    });
    window.history.replaceState(null, "", `${paths.login}#${fragment.toString()}`);
    render(<App />);

    // The first render: the login chunk has not run (it never will), and the boot scrub has.
    expect(window.location.href).not.toContain("refresh_token");

    await screen.findByTestId(shellTestid.loadFailed);
    // "Reload" reloads this very URL, which must not carry the tokens.
    expect(window.location.href).not.toContain("live-refresh-token-from-the-email");
    expect(window.location.pathname).toBe(paths.login);
  });
});

describe("R4605000 a screen that fails on a stale build", () => {
  const RUNNING = "/assets/index-old.js";
  const NEWER = "/assets/index-new.js";

  // A screen whose chunk fails the way a stale build's does. The file-level mock above makes the
  // import itself reject with Vitest's own mock error, so the chunk message is thrown on render,
  // exactly what the boundary reads in production.
  function StaleChunk(): null {
    throw new TypeError("Failed to fetch dynamically imported module: /assets/login-3f9a.js");
  }

  function failedScreen(): void {
    render(
      <RouteErrorBoundary>
        <StaleChunk />
      </RouteErrorBoundary>,
    );
  }

  beforeEach(() => {
    const script = document.createElement("script");
    script.type = "module";
    script.src = RUNNING;
    document.head.append(script);
    window.sessionStorage.clear();
  });

  afterEach(() => {
    document.head.querySelectorAll('script[type="module"]').forEach((el) => el.remove());
    vi.unstubAllGlobals();
  });

  /** The deployed build names `entry`. */
  function served(entry: string): void {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => ({
        ok: true,
        json: async () => ({ entry }),
      })),
    );
  }

  it("R4605000 a chunk failure on a stale build loads the new version once", async () => {
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    served(NEWER);
    failedScreen();

    expect(await screen.findByText("JackiOh was just updated. Loading the new version…")).toBeInTheDocument();
    expect(screen.getByTestId(shellTestid.loadFailed)).toBeInTheDocument();
    expect(screen.queryByTestId(shellTestid.reload)).toBeNull();
    expect(reloaded).toHaveBeenCalledTimes(1);
  });

  it("R4605000 a second failure for the same entry shows today's panel", async () => {
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    // A fresh page after the reload above: the entry is remembered, the build still stale.
    window.sessionStorage.setItem(UPDATE_RELOAD_KEY, NEWER);
    served(NEWER);
    failedScreen();

    const panel = await screen.findByTestId(shellTestid.loadFailed);
    expect(panel.textContent).toMatch(/didn't load/);
    expect(screen.getByTestId(shellTestid.reload)).toBeInTheDocument();
    expect(reloaded).not.toHaveBeenCalled();
  });

  it("R4605000 a chunk failure on the current build shows today's panel and wording", async () => {
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    served(RUNNING);
    failedScreen();

    expect(
      await screen.findByText("This screen didn't load. Check your connection, then reload the page."),
    ).toBeInTheDocument();
    expect(screen.getByTestId(shellTestid.reload)).toBeInTheDocument();
    expect(reloaded).not.toHaveBeenCalled();
  });
});
