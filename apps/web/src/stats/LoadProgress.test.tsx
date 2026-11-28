// R5186000, R5219000: the card statistics' load progress on /stats (stats/LoadProgress.tsx).

import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { ApiUnreachableError, type StatsLoading } from "../net/api.ts";
import {
  STATS_LOADING_AFTER_MS,
  STATS_LOADING_GUESS_MS,
  STATS_LOADING_POLL_MS,
  STATS_LOADING_SHARE_AT_EXPECTED,
  STATS_LOADING_SHARE_MAX,
  STATS_LOADING_TICK_MS,
} from "./config.ts";
import { askAgain, clockOf, loadShare, loadWords, StatsLoadProgress } from "./LoadProgress.tsx";

/** Halfway through the time to expect, on the card totals. */
const HALFWAY: StatsLoading = { loading: true, step: 2, steps: 3, elapsedMs: 31_000, expectedMs: 62_000 };

/** No load under way, the last one having taken a minute and two seconds. */
const AT_REST: StatsLoading = { loading: false, step: 0, steps: 3, elapsedMs: 0, expectedMs: 62_000 };

/** How long a test waits on the bar before it calls it stuck. */
const SLOW = { timeout: 10_000 } as const;

/** Thirty minutes, the longest load these tests follow. */
const HALF_HOUR_MS = 1_800_000;

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.useRealTimers();
});

/** The bar's fill, as the percentage its width draws. */
function drawn(): number {
  const fill = screen.getByRole("progressbar").firstElementChild;
  return fill instanceof HTMLElement ? Number.parseFloat(fill.style.width) : Number.NaN;
}

/** Moves the page's clock on by `ms`, letting what it wakes run. */
async function wait(ms: number): Promise<void> {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(ms);
  });
}

describe("R5219000 the card statistics' load progress always moves", () => {
  it("R5219000 rises on every tick, ever slower, three quarters of the way at the time to expect, never full", () => {
    const atExpected = STATS_LOADING_SHARE_MAX * STATS_LOADING_SHARE_AT_EXPECTED;
    expect(loadShare(62_000, 62_000)).toBeCloseTo(atExpected);
    expect(loadShare(STATS_LOADING_GUESS_MS, null)).toBeCloseTo(atExpected);
    expect(loadShare(0, 62_000)).toBe(0);
    let before = 0;
    let step = Infinity;
    for (let ms = STATS_LOADING_TICK_MS; ms <= HALF_HOUR_MS; ms += STATS_LOADING_TICK_MS) {
      const share = loadShare(ms, 62_000);
      expect(share).toBeGreaterThan(before);
      expect(share - before).toBeLessThanOrEqual(step);
      expect(share).toBeLessThan(STATS_LOADING_SHARE_MAX);
      step = share - before;
      before = share;
    }
  });

  it("R5219000 says the step and the time, and almost done past the time to expect", () => {
    expect(clockOf(62_000)).toBe("1:02");
    expect(clockOf(305_000)).toBe("5:05");
    expect(loadWords(HALFWAY, 31_000)).toBe("Step 2 of 3: Reading the card totals · 0:31 of about 1:02");
    expect(loadWords(HALFWAY, 90_000)).toBe("Step 2 of 3: Reading the card totals · 1:30 · almost done");
    expect(loadWords({ ...HALFWAY, step: 1, expectedMs: null }, 31_000)).toBe(
      "Step 1 of 3: Reading the games played · 0:31 so far",
    );
    expect(loadWords(null, 12_000)).toBe("Loading card statistics… · 0:12 so far");
  });

  it("R5186000 asks again after a read that found no answer in time only while the server reports a load", async () => {
    const timedOut = new ApiUnreachableError(new Error("no answer within 75 s"));
    await expect(askAgain(timedOut, () => Promise.resolve(HALFWAY))).resolves.toBe(true);
    await expect(askAgain(timedOut, () => Promise.resolve(AT_REST))).resolves.toBe(false);
    await expect(askAgain(timedOut, () => Promise.reject(new Error("down")))).resolves.toBe(false);
    await expect(askAgain(new Error("500"), () => Promise.resolve(HALFWAY))).resolves.toBe(false);
  });

  it("R5219000 the bar shows after the wait before the server answers, and grows on every tick", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    // The server never answers: the bar runs on the page's clock alone.
    const unanswered: ((answer: Response) => void)[] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn(
        () =>
          new Promise<Response>((resolve) => {
            unanswered.push(resolve);
          }),
      ),
    );
    render(<StatsLoadProgress />);
    expect(screen.queryByRole("progressbar")).toBeNull();
    await wait(STATS_LOADING_AFTER_MS + STATS_LOADING_TICK_MS);
    expect(screen.getByText(/^Loading card statistics… · 0:0\d so far$/u)).toBeTruthy();
    const widths = [drawn()];
    for (let tick = 0; tick < 5; tick += 1) {
      await wait(STATS_LOADING_TICK_MS);
      widths.push(drawn());
    }
    widths.forEach((width, i) => {
      if (i > 0) expect(width).toBeGreaterThan(widths[i - 1] ?? Infinity);
    });
  }, SLOW.timeout);

  it("R5219000 the bar follows the server's load and never draws less when an answer would lower it", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    let asked = 0;
    vi.stubGlobal(
      "fetch",
      vi.fn(() => {
        asked += 1;
        return Promise.resolve(new Response(JSON.stringify(asked === 1 ? HALFWAY : AT_REST), { status: 200 }));
      }),
    );
    render(<StatsLoadProgress />);
    await wait(STATS_LOADING_AFTER_MS);
    await vi.waitFor(() => {
      expect(screen.getByText(/^Step 2 of 3: Reading the card totals · 0:3\d of about 1:02$/u)).toBeTruthy();
    }, SLOW);
    const high = drawn();
    expect(high).toBeGreaterThan(50);
    await wait(STATS_LOADING_POLL_MS);
    await vi.waitFor(() => {
      expect(screen.getByText(/^Loading card statistics… · /u)).toBeTruthy();
    }, SLOW);
    expect(drawn()).toBeGreaterThanOrEqual(high);
  }, SLOW.timeout);
});
