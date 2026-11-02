// The client's router (#1756): a link or a redirect to another screen opens it at the top, while the
// browser's Back and Forward, which restore the position themselves, are left to the browser.

import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { loginPath, navigate, paths, usePathname } from "./navigate.ts";

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
  document.body.replaceChildren();
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
