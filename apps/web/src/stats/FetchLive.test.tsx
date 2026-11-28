import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { devStatsRefresh, type MeResponse, type StatsLoading } from "../net/api.ts";
import { keptAccount } from "../net/gate.ts";
import { readSession } from "../net/session.ts";
import { FetchLive } from "./FetchLive.tsx";
import { statsTestid } from "./testids.ts";

vi.mock("../net/api.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/api.ts")>();
  return { ...actual, devStatsRefresh: vi.fn() };
});

vi.mock("../net/gate.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/gate.ts")>();
  return { ...actual, keptAccount: vi.fn() };
});

vi.mock("../net/session.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/session.ts")>();
  return { ...actual, readSession: vi.fn() };
});

const TOKEN = "token-1";

const LOADING: StatsLoading = { loading: false, step: 0, steps: 3, elapsedMs: 0, expectedMs: null };

function signedInAs(isDev: boolean): void {
  vi.mocked(readSession).mockReturnValue({ accessToken: TOKEN } as ReturnType<typeof readSession>);
  vi.mocked(keptAccount).mockReturnValue({ isDev } as MeResponse);
}

afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});

describe("R5274000 Fetch live", () => {
  it("R5274000 shows for a dev account alone", () => {
    signedInAs(false);
    render(<FetchLive reload={vi.fn()} />);
    expect(screen.queryByTestId(statsTestid.fetchLive)).toBeNull();

    vi.mocked(readSession).mockReturnValue(null);
    render(<FetchLive reload={vi.fn()} />);
    expect(screen.queryByTestId(statsTestid.fetchLive)).toBeNull();
  });

  it("R5274000 asks the server to let the figures go, then reloads the page", async () => {
    signedInAs(true);
    vi.mocked(devStatsRefresh).mockResolvedValue(LOADING);
    const reload = vi.fn();
    render(<FetchLive reload={reload} />);
    fireEvent.click(screen.getByTestId(statsTestid.fetchLive));
    expect(devStatsRefresh).toHaveBeenCalledWith(TOKEN);
    await waitFor(() => {
      expect(reload).toHaveBeenCalledTimes(1);
    });
  });

  it("R5274000 says so and reloads nothing when the server refuses", async () => {
    signedInAs(true);
    vi.mocked(devStatsRefresh).mockRejectedValue(new Error("forbidden"));
    const reload = vi.fn();
    render(<FetchLive reload={reload} />);
    fireEvent.click(screen.getByTestId(statsTestid.fetchLive));
    await waitFor(() => {
      expect(screen.getByTestId(statsTestid.fetchLive).textContent).toBe("Couldn't fetch live. Try again");
    });
    expect(reload).not.toHaveBeenCalled();
  });
});
