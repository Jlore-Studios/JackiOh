// R5186000: the card statistics' load progress on /stats (stats/LoadProgress.tsx).

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { ApiUnreachableError, type StatsLoading } from "../net/api.ts";
import { STATS_LOADING_SHARE_MAX } from "./config.ts";
import { askAgain, clockOf, loadShare, loadWords, StatsLoadProgress } from "./LoadProgress.tsx";

/** Halfway through the time to expect, on the card totals. */
const HALFWAY: StatsLoading = { loading: true, step: 2, steps: 3, elapsedMs: 31_000, expectedMs: 62_000 };

/** Long enough for the bar's first ask, which waits `STATS_LOADING_AFTER_MS`. */
const SLOW = { timeout: 10_000 } as const;

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("R5186000 the card statistics' load progress", () => {
  it("R5186000 fills the bar by the time gone of the time to expect, or by the steps done, never full before the end", () => {
    expect(loadShare(HALFWAY)).toBeCloseTo(0.5);
    expect(loadShare({ ...HALFWAY, expectedMs: null })).toBeCloseTo(1 / 3);
    expect(loadShare({ ...HALFWAY, elapsedMs: 600_000 })).toBe(STATS_LOADING_SHARE_MAX);
    expect(loadShare({ ...HALFWAY, loading: false })).toBe(0);
  });

  it("R5186000 says the step, the time so far and the time to expect", () => {
    expect(clockOf(62_000)).toBe("1:02");
    expect(clockOf(305_000)).toBe("5:05");
    expect(loadWords(HALFWAY)).toBe("Step 2 of 3: Reading the card totals · 0:31 of about 1:02");
    expect(loadWords({ ...HALFWAY, step: 1, expectedMs: null })).toBe("Step 1 of 3: Reading the games played · 0:31 so far");
  });

  it("R5186000 asks again after a read that found no answer in time only while the server reports a load", async () => {
    const timedOut = new ApiUnreachableError(new Error("no answer within 75 s"));
    await expect(askAgain(timedOut, () => Promise.resolve(HALFWAY))).resolves.toBe(true);
    await expect(askAgain(timedOut, () => Promise.resolve({ ...HALFWAY, loading: false }))).resolves.toBe(false);
    await expect(askAgain(timedOut, () => Promise.reject(new Error("down")))).resolves.toBe(false);
    await expect(askAgain(new Error("500"), () => Promise.resolve(HALFWAY))).resolves.toBe(false);
  });

  it("R5186000 draws the bar from the server's answer, and nothing while no load is under way", async () => {
    const fetchMock = vi.fn((_input: RequestInfo | URL) =>
      Promise.resolve(new Response(JSON.stringify(HALFWAY), { status: 200 })),
    );
    vi.stubGlobal("fetch", fetchMock);
    render(<StatsLoadProgress />);
    const bar = await screen.findByRole("progressbar", {}, SLOW);
    expect(bar.getAttribute("aria-valuenow")).toBe("50");
    expect(screen.getByText("Step 2 of 3: Reading the card totals · 0:31 of about 1:02")).toBeTruthy();
    expect(new URL(String(fetchMock.mock.calls[0]?.[0]), window.location.origin).pathname).toBe("/api/stats/loading");
    cleanup();

    vi.stubGlobal(
      "fetch",
      vi.fn(() => Promise.resolve(new Response(JSON.stringify({ ...HALFWAY, loading: false }), { status: 200 }))),
    );
    const { container } = render(<StatsLoadProgress />);
    await new Promise((resolve) => setTimeout(resolve, 2_000));
    expect(container.innerHTML).toBe("");
  }, SLOW.timeout);
});
