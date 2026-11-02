// R1754000: a page that holds work a move would lose registers a leave guard (`guardLeaving`), and
// `navigate`, the browser's Back and Forward and the queue's move to a pairing all wait for it.
// A separate file from `liveGame.test.tsx`, which mocks `navigate`.

import { act, cleanup, render, renderHook, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { MATCH_FOUND_NAV_DELAY_MS, SERIES_POLL_SECONDS } from "@jackioh/server-config";
import type { MeResponse } from "./api.ts";
import { MATCH_FOUND_STATUS, readQueued, rememberQueued, useQueueFollow } from "./liveGame.ts";
import { guardLeaving, leaveFirst, navigate, paths, usePathname } from "./navigate.ts";
import { at, signIn } from "../test/stubServer.ts";

function me(currentMatchId: string | null): MeResponse {
  return {
    profile: { id: "p1", status: "active" },
    needsInviteCode: false,
    emailVerified: true,
    currentMatchId,
    currentSeriesId: null,
    currentDuosMatchId: null,
    email: "player@example.com",
  };
}

/** Move the fake clock on, then let what it set off settle. */
async function advance(ms: number): Promise<void> {
  await act(async () => {
    vi.advanceTimersByTime(ms);
    for (let i = 0; i < 10; i += 1) await Promise.resolve();
  });
}

function Follower({ path, read }: { path: string; read: (token: string) => Promise<MeResponse> }) {
  const found = useQueueFollow(path, read);
  return found === null ? null : <p data-testid="found">{MATCH_FOUND_STATUS}</p>;
}

const releases: (() => void)[] = [];

/** Registers a guard that keeps the moves it is handed. */
function guard(): (() => void)[] {
  const held: (() => void)[] = [];
  releases.push(
    guardLeaving((leave) => {
      held.push(leave);
    }),
  );
  return held;
}

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
  at("/");
});

afterEach(() => {
  for (const release of releases.splice(0)) release();
  cleanup();
  vi.useRealTimers();
  window.localStorage.clear();
  window.sessionStorage.clear();
});

describe("R1754000 a page that holds work a move would lose is asked before it is left", () => {
  it("R1754000 a guard holds every navigate off its page and hands over the move; a forced move and a released guard go at once", () => {
    at(paths.creator);
    const held = guard();

    expect(navigate("/decks")).toBe(false);
    expect(window.location.pathname).toBe(paths.creator);
    expect(held).toHaveLength(1);

    expect(navigate(paths.creator)).toBe(true);
    expect(held).toHaveLength(1);

    held[0]?.();
    expect(window.location.pathname).toBe("/decks");

    expect(navigate(paths.play, { force: true })).toBe(true);
    expect(window.location.pathname).toBe(paths.play);
    expect(held).toHaveLength(1);

    for (const release of releases.splice(0)) release();
    expect(navigate("/decks")).toBe(true);
    expect(window.location.pathname).toBe("/decks");
  });

  it("R1754000 leaveFirst runs at once unguarded, else through the hold", () => {
    at(paths.creator);
    const ran = vi.fn();

    expect(leaveFirst(ran)).toBe(true);
    expect(ran).toHaveBeenCalledTimes(1);

    const held = guard();
    expect(leaveFirst(ran)).toBe(false);
    expect(ran).toHaveBeenCalledTimes(1);
    expect(held).toHaveLength(1);

    held[0]?.();
    expect(ran).toHaveBeenCalledTimes(2);

    for (const release of releases.splice(0)) release();
    expect(leaveFirst(ran)).toBe(true);
    expect(ran).toHaveBeenCalledTimes(3);
  });

  it("R1754000 the browser's Back off a guarded page puts its address back, and the held move goes back unheld", async () => {
    at("/decks");
    window.history.pushState(null, "", paths.creator);
    const { result } = renderHook(() => usePathname());
    const held = guard();

    await traverse(() => {
      window.history.back();
    });
    expect(window.location.pathname).toBe(paths.creator);
    expect(result.current).toBe(paths.creator);
    expect(held).toHaveLength(1);

    await traverse(() => {
      held[0]?.();
    });
    expect(window.location.pathname).toBe("/decks");
    expect(result.current).toBe("/decks");
    expect(held).toHaveLength(1);
  });

  it("R1754000 the browser's Forward off a guarded page is held too, and the held move goes forward", async () => {
    at(paths.creator);
    window.history.pushState(null, "", "/decks");
    await traverse(() => {
      window.history.back();
    });
    const { result } = renderHook(() => usePathname());
    const held = guard();
    expect(result.current).toBe(paths.creator);

    await traverse(() => {
      window.history.forward();
    });
    expect(window.location.pathname).toBe(paths.creator);
    expect(result.current).toBe(paths.creator);
    expect(held).toHaveLength(1);

    await traverse(() => {
      held[0]?.();
    });
    await act(async () => {
      await Promise.resolve();
    });
    expect(window.location.pathname).toBe("/decks");
    expect(result.current).toBe("/decks");
    expect(held).toHaveLength(1);
  });

  it("R1754000 a held browser move made once goes once", async () => {
    at("/decks");
    window.history.pushState(null, "", paths.creator);
    renderHook(() => usePathname());
    const held = guard();
    await traverse(() => {
      window.history.back();
    });

    const back = vi.spyOn(window.history, "back");
    await traverse(() => {
      held[0]?.();
    });
    expect(window.location.pathname).toBe("/decks");
    expect(back).toHaveBeenCalledTimes(1);
    held[0]?.();
    expect(back).toHaveBeenCalledTimes(1);
    back.mockRestore();
  });

  it("R1754000 a pairing's move off a guarded page is held, and the queue is left neither waiting nor announcing", async () => {
    vi.useFakeTimers();
    signIn();
    rememberQueued("bo1");
    at(paths.creator);
    const held = guard();
    const read = vi.fn<(token: string) => Promise<MeResponse>>().mockResolvedValue(me("m-9"));

    render(<Follower path={paths.creator} read={read} />);
    await advance(0);
    expect(screen.getByTestId("found")).toHaveTextContent(MATCH_FOUND_STATUS);
    expect(readQueued()).toBeNull();

    await advance(MATCH_FOUND_NAV_DELAY_MS);
    expect(held).toHaveLength(1);
    expect(window.location.pathname).toBe(paths.creator);
    expect(screen.queryByTestId("found")).toBeNull();

    const reads = read.mock.calls.length;
    await advance(SERIES_POLL_SECONDS * 1000 * 3);
    expect(read).toHaveBeenCalledTimes(reads);

    act(() => {
      held[0]?.();
    });
    expect(window.location.pathname).toBe("/match/m-9");
  });
});
