// The client's router (#1756): a link or a redirect to another screen opens it at the top, while the
// browser's Back and Forward, which restore the position themselves, are left to the browser.

import { createElement, type ReactElement } from "react";
import { act, render, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useMarkBoardShown } from "../game/boardShown.ts";
import { guardLeaving, loginPath, navigate, paths, usePathname } from "./navigate.ts";
import { UPDATE_CHECK_MIN_MS, checkForUpdate, fullLoadPage, resetUpdateForTests } from "./update.ts";

// jsdom's `location.assign` and `location.replace` are non-writable own properties, so the full
// load a move becomes with an update waiting is watched through this seam instead.
vi.mock("./update.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./update.ts")>();
  return { ...actual, fullLoadPage: vi.fn() };
});

const fullLoad = vi.mocked(fullLoadPage);

/** The pathname the page was at when each `window.scrollTo` ran. */
let scrolledAt: string[] = [];

async function traverse(step: () => void): Promise<void> {
  await act(async () => {
    const popped = new Promise((resolve) => {
      window.addEventListener("popstate", resolve, { once: true });
    });
    step();
    await popped;
  });
}

beforeEach(() => {
  window.history.replaceState(null, "", paths.collection);
  scrolledAt = [];
  vi.spyOn(window, "scrollTo").mockImplementation(() => {
    scrolledAt.push(window.location.pathname);
  });
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  resetUpdateForTests();
  document.body.replaceChildren();
  document.head.querySelectorAll('script[type="module"]').forEach((el) => el.remove());
});

describe("#1756 navigate opens a new screen at the top", () => {
  it("a link to another screen scrolls the window to the top once, after the push", () => {
    navigate(paths.replays);
    expect(window.location.pathname).toBe(paths.replays);
    expect(window.scrollTo).toHaveBeenCalledWith(0, 0);
    // After the push, so the entry left behind keeps its own position for Back.
    expect(scrolledAt).toEqual([paths.replays]);
  });

  it("a redirect to another screen scrolls to the top too", () => {
    navigate(loginPath({ reason: "expired" }), { replace: true });
    expect(window.location.pathname).toBe(paths.login);
    expect(scrolledAt).toEqual([paths.login]);
  });

  it("a change of the query alone keeps the scroll, as the Almanac's detail and the Collection's sections do", () => {
    navigate(paths.collectionSection("cards"));
    expect(window.location.search).toBe("?section=cards");
    window.history.replaceState(null, "", paths.almanac);
    navigate(paths.almanacCard("core-001"), { replace: true });
    expect(window.location.search).toBe("?card=core-001");
    navigate(paths.almanac, { replace: true });
    expect(scrolledAt).toEqual([]);
  });

  it("a navigation to where the page already is scrolls nothing", () => {
    navigate(paths.collection);
    expect(scrolledAt).toEqual([]);
  });

  it("leaves an element's own scroll alone, as the log's, the chat's and a panel's", () => {
    const list = document.createElement("div");
    document.body.append(list);
    list.scrollTop = 40;
    navigate(paths.replays);
    expect(list.scrollTop).toBe(40);
  });

  it("Back and Forward re-render the route and never scroll: the browser restores the position", async () => {
    const { result } = renderHook(() => usePathname());
    act(() => {
      navigate(paths.replays);
    });
    expect(result.current).toBe(paths.replays);

    await traverse(() => window.history.back());
    expect(result.current).toBe(paths.collection);
    await traverse(() => window.history.forward());
    expect(result.current).toBe(paths.replays);
    expect(scrolledAt).toEqual([paths.replays]);
  });
});

function BoardProbe(): ReactElement | null {
  useMarkBoardShown();
  return null;
}

describe("R4605000 an update waiting", () => {
  const RUNNING = "/assets/index-old.js";
  const NEWER = "/assets/index-new.js";

  /** A stale tab, through the tested path: the deployed entry is newer than the running one. */
  async function waitForUpdate(): Promise<void> {
    const script = document.createElement("script");
    script.type = "module";
    script.src = RUNNING;
    document.head.append(script);
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => ({
        ok: true,
        json: async () => ({ entry: NEWER }),
      })),
    );
    expect(await checkForUpdate(UPDATE_CHECK_MIN_MS, "production")).toBe(true);
  }

  it("R4605000 a menu move with an update waiting loads the target page", async () => {
    await waitForUpdate();
    navigate(paths.shop);
    expect(fullLoad).toHaveBeenCalledWith(paths.shop, false);
    expect(fullLoad).toHaveBeenCalledTimes(1);
    expect(window.location.pathname).toBe(paths.collection);
  });

  it("R4605000 a replace move with an update waiting replaces the page", async () => {
    await waitForUpdate();
    navigate(paths.shop, { replace: true });
    expect(fullLoad).toHaveBeenCalledWith(paths.shop, true);
    expect(fullLoad).toHaveBeenCalledTimes(1);
    expect(window.location.pathname).toBe(paths.collection);
  });

  it("R4605000 with no update waiting a move stays in place", async () => {
    navigate(paths.shop);
    expect(fullLoad).not.toHaveBeenCalled();
    expect(window.location.pathname).toBe(paths.shop);
  });

  it("R4605000 with an update waiting but a board up the move stays in place", async () => {
    await waitForUpdate();
    let unmount = (): void => undefined;
    await act(async () => {
      ({ unmount } = render(createElement(BoardProbe)));
    });
    try {
      navigate(paths.shop);
      expect(fullLoad).not.toHaveBeenCalled();
      expect(window.location.pathname).toBe(paths.shop);
    } finally {
      act(() => {
        unmount();
      });
    }
  });

  it("R4605000 the leave guard holds the move before any full load", async () => {
    await waitForUpdate();
    const held: (() => void)[] = [];
    const release = guardLeaving((leave) => {
      held.push(leave);
    });
    try {
      expect(navigate(paths.shop)).toBe(false);
      expect(fullLoad).not.toHaveBeenCalled();
      expect(window.location.pathname).toBe(paths.collection);
      held[0]?.();
      expect(fullLoad).toHaveBeenCalledWith(paths.shop, false);
    } finally {
      release();
    }
  });
});
