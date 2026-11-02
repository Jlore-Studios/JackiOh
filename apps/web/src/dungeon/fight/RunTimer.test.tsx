// B24 (DR14 part E, #2706): the run's timer on screen, the time since the run started.

import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { FIGHT_TIMER_TICK_MS } from "./config.ts";
import { RunTimer, formatRunTime } from "./RunTimer.tsx";
import { fightTestid } from "./testids.ts";

const T0 = Date.UTC(2026, 10, 1, 20, 0, 0);

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe("B24 the run's timer", () => {
  it("B24 formatRunTime writes minutes and whole seconds, floored and never negative", () => {
    expect(formatRunTime(0)).toBe("0:00");
    expect(formatRunTime(65_000)).toBe("1:05");
    expect(formatRunTime(65_999)).toBe("1:05");
    expect(formatRunTime(3_725_000)).toBe("62:05");
    expect(formatRunTime(-5)).toBe("0:00");
  });

  it("B24 a timer shows the run's time on screen and goes on as it runs", () => {
    vi.useFakeTimers({ now: T0 });
    render(<RunTimer startedAt={T0 - 65_000} />);
    expect(screen.getByTestId(fightTestid.timer)).toHaveTextContent("Run time 1:05");
    act(() => {
      vi.advanceTimersByTime(3 * FIGHT_TIMER_TICK_MS);
    });
    expect(screen.getByTestId(fightTestid.timer)).toHaveTextContent("Run time 1:08");
  });

  it("B24 a run with no start reads 0:00", () => {
    vi.useFakeTimers({ now: T0 });
    render(<RunTimer startedAt={null} />);
    expect(screen.getByTestId(fightTestid.timer)).toHaveTextContent("0:00");
  });
});
