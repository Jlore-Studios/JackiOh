// `auth/AccountError.tsx`: the Try again every screen offers after a failed read (#1691), and the
// wait a rate limit stated (R192) that holds it.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { AccountError, TryAgainButton } from "./AccountError.tsx";

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["setInterval", "clearInterval", "Date"] });
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe("a failed read's Try again", () => {
  it("#1691 counts a stated wait down and holds Try again until it has passed", () => {
    const onRetry = vi.fn();
    render(<AccountError message="slow down" retryAfterMs={42_000} onRetry={onRetry} testId="err" retryTestId="retry" />);

    expect(screen.getByTestId("err")).toHaveTextContent("You can try again in 42 s.");
    const retry = screen.getByTestId("retry");
    expect(retry).toBeDisabled();
    expect(retry).toHaveTextContent(/Try again in \d+ s/);

    act(() => {
      vi.advanceTimersByTime(42_000);
    });
    expect(screen.getByTestId("err")).not.toHaveTextContent("You can try again in");
    expect(retry).toBeEnabled();
    expect(retry).toHaveTextContent(/^Try again$/);
    fireEvent.click(retry);
    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  it("#1691 with no stated wait offers Try again at once, its children beside it", () => {
    const onRetry = vi.fn();
    render(
      <AccountError message="The server could not be reached." onRetry={onRetry} testId="err" retryTestId="retry">
        <a href="/" data-testid="home">
          Home
        </a>
      </AccountError>,
    );

    expect(screen.getByTestId("err")).toHaveTextContent("The server could not be reached.");
    expect(screen.getByTestId("err")).toHaveAttribute("role", "alert");
    const retry = screen.getByTestId("retry");
    expect(retry).toBeEnabled();
    expect(retry.parentElement).toBe(screen.getByTestId("home").parentElement);
    fireEvent.click(retry);
    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  it("#1691 TryAgainButton holds itself through its own wait", () => {
    const onRetry = vi.fn();
    render(<TryAgainButton retryAfterMs={5_000} onRetry={onRetry} testId="again" />);

    const again = screen.getByTestId("again");
    expect(again).toBeDisabled();
    expect(again).toHaveTextContent(/Try again in \d+ s/);
    act(() => {
      vi.advanceTimersByTime(5_000);
    });
    expect(again).toBeEnabled();
    fireEvent.click(again);
    expect(onRetry).toHaveBeenCalledTimes(1);
  });
});
