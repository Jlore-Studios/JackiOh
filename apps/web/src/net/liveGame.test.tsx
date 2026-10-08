// R765: the player's online game, followed from anywhere in the client (`liveGame.ts`): the queue
// this tab joined, followed off `/play`, and the live game the menus' banner offers the way back to.

import { act, cleanup, render, renderHook, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { MATCH_FOUND_NAV_DELAY_MS, SERIES_POLL_SECONDS } from "@jackioh/server-config";
import type { MeResponse } from "./api.ts";
import type { Account } from "./gate.ts";
import {
  MATCH_FOUND_STATUS,
  QUEUE_STORAGE_KEY,
  forgetQueued,
  liveGameOf,
  readQueued,
  rememberQueued,
  useLiveGame,
  useQueueFollow,
  useQueued,
} from "./liveGame.ts";
import { navigate, paths } from "./navigate.ts";
import { E2E_SESSION_STORAGE_KEY } from "./session.ts";

vi.mock("./navigate.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./navigate.ts")>();
  return { ...actual, navigate: vi.fn() };
});

const POLL_MS = SERIES_POLL_SECONDS * 1000;

function me(currentMatchId: string | null, currentSeriesId: string | null = null): MeResponse {
  return {
    profile: { id: "p1", status: "active" },
    needsInviteCode: false,
    emailVerified: true,
    currentMatchId,
    currentSeriesId,
    email: "player@example.com",
  };
}

function ready(answer: MeResponse, token = "tok-1"): Account {
  return { kind: "ready", token, me: answer };
}

/** Let resolved promises run, inside `act`. */
async function flush(): Promise<void> {
  await act(async () => {
    for (let i = 0; i < 10; i += 1) await Promise.resolve();
  });
}

/** Move the fake clock on, then let what it set off settle. */
async function advance(ms: number): Promise<void> {
  await act(async () => {
    vi.advanceTimersByTime(ms);
    for (let i = 0; i < 10; i += 1) await Promise.resolve();
  });
}

function signIn(): void {
  window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: "e2e-token" }));
}

beforeEach(() => {
  vi.mocked(navigate).mockReset();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
  window.localStorage.clear();
  window.sessionStorage.clear();
});

describe("R765 the live game an account read names", () => {
  it("R765 liveGameOf reads the board, the series screen between games, or nothing; the board wins", () => {
    expect(liveGameOf(me("m-1"))).toEqual({ kind: "match", id: "m-1", path: paths.match("m-1") });
    expect(liveGameOf(me(null, "s-1"))).toEqual({ kind: "series", id: "s-1", path: paths.series("s-1") });
    expect(liveGameOf(me("m-1", "s-1"))?.path).toBe(paths.match("m-1"));
    expect(liveGameOf(me(null))).toBeNull();
    expect(liveGameOf({ currentMatchId: "" })).toBeNull();
  });

  it("R765 useLiveGame is the account's game, and none for an account that is not ready", () => {
    const read = vi.fn<(token: string) => Promise<MeResponse>>();
    expect(renderHook(() => useLiveGame(ready(me("m-1")), read)).result.current?.path).toBe("/match/m-1");
    for (const account of [{ kind: "loading" }, { kind: "anonymous" }, { kind: "error", message: "down" }] as Account[]) {
      expect(renderHook(() => useLiveGame(account, read)).result.current).toBeNull();
    }
    expect(renderHook(() => useLiveGame(ready(me(null)), read)).result.current).toBeNull();
  });

  it("R765 a live game is read again every SERIES_POLL_SECONDS, and is gone once a read says it ended", async () => {
    vi.useFakeTimers();
    const read = vi.fn<(token: string) => Promise<MeResponse>>().mockResolvedValue(me("m-1"));
    const account = ready(me("m-1"));
    const { result } = renderHook(() => useLiveGame(account, read));
    expect(read).not.toHaveBeenCalled();

    await advance(POLL_MS);
    expect(read).toHaveBeenCalledTimes(1);
    expect(read).toHaveBeenCalledWith("tok-1");
    expect(result.current?.path).toBe("/match/m-1");

    // A read that fails changes nothing on screen.
    read.mockRejectedValueOnce(new Error("network down"));
    await advance(POLL_MS);
    expect(result.current?.path).toBe("/match/m-1");

    read.mockResolvedValue(me(null));
    await advance(POLL_MS);
    expect(result.current).toBeNull();

    // With no game left there is nothing to watch: no more reads.
    const reads = read.mock.calls.length;
    await advance(POLL_MS * 3);
    expect(read).toHaveBeenCalledTimes(reads);
  });

  it("R765 an account with no live game is not polled", async () => {
    vi.useFakeTimers();
    const read = vi.fn<(token: string) => Promise<MeResponse>>().mockResolvedValue(me("m-1"));
    renderHook(() => useLiveGame(ready(me(null)), read));
    await advance(POLL_MS * 3);
    expect(read).not.toHaveBeenCalled();
  });
});

describe("R765 the queue this tab joined", () => {
  it("R765 is remembered per tab, by mode, and forgotten", () => {
    expect(readQueued()).toBeNull();
    rememberQueued("bo3");
    expect(readQueued()).toBe("bo3");
    expect(window.sessionStorage.getItem(QUEUE_STORAGE_KEY)).toBe("bo3");
    expect(window.localStorage.getItem(QUEUE_STORAGE_KEY), "never shared with other tabs").toBeNull();
    forgetQueued();
    expect(readQueued()).toBeNull();
  });

  it("R765 anything else in storage is no queue, and storage that throws costs nothing", () => {
    window.sessionStorage.setItem(QUEUE_STORAGE_KEY, "ranked");
    expect(readQueued()).toBeNull();
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("full");
    });
    expect(() => {
      rememberQueued("bo1");
    }).not.toThrow();
    expect(readQueued()).toBeNull();
  });

  it("R765 useQueued follows every change this tab makes", () => {
    const { result } = renderHook(() => useQueued());
    expect(result.current).toBeNull();
    act(() => {
      rememberQueued("random");
    });
    expect(result.current).toBe("random");
    act(() => {
      forgetQueued();
    });
    expect(result.current).toBeNull();
  });
});

describe("R765 a pairing takes the player to the game from any screen", () => {
  function Follower({ path, read }: { path: string; read: (token: string) => Promise<MeResponse> }) {
    const found = useQueueFollow(path, read);
    return found === null ? null : <p data-testid="found">{MATCH_FOUND_STATUS}</p>;
  }

  it("R765 a tab that is not queued reads nothing", async () => {
    vi.useFakeTimers();
    signIn();
    const read = vi.fn<(token: string) => Promise<MeResponse>>().mockResolvedValue(me("m-1"));
    render(<Follower path={paths.decks} read={read} />);
    await advance(POLL_MS * 3);
    expect(read).not.toHaveBeenCalled();
    expect(navigate).not.toHaveBeenCalled();
  });

  it("R765 on /play the lobby watches for itself, so the follower reads nothing there", async () => {
    vi.useFakeTimers();
    signIn();
    rememberQueued("bo1");
    const read = vi.fn<(token: string) => Promise<MeResponse>>().mockResolvedValue(me("m-1"));
    render(<Follower path={paths.play} read={read} />);
    await advance(POLL_MS * 3);
    expect(read).not.toHaveBeenCalled();
    expect(readQueued()).toBe("bo1");
  });

  it("R765 elsewhere it reads the account every SERIES_POLL_SECONDS, announces the pairing, then takes the player there", async () => {
    vi.useFakeTimers();
    signIn();
    rememberQueued("bo1");
    const read = vi.fn<(token: string) => Promise<MeResponse>>().mockResolvedValue(me(null));
    render(<Follower path={paths.practice} read={read} />);

    await flush();
    expect(read).toHaveBeenCalledTimes(1);
    expect(read).toHaveBeenCalledWith("e2e-token");
    await advance(POLL_MS);
    expect(read).toHaveBeenCalledTimes(2);
    expect(screen.queryByTestId("found")).toBeNull();

    read.mockResolvedValue(me("m-9"));
    await advance(POLL_MS);
    expect(screen.getByTestId("found")).toHaveTextContent(MATCH_FOUND_STATUS);
    expect(readQueued(), "paired: out of the queue").toBeNull();
    expect(navigate).not.toHaveBeenCalled();

    await advance(MATCH_FOUND_NAV_DELAY_MS);
    expect(navigate).toHaveBeenCalledWith("/match/m-9");
    expect(screen.queryByTestId("found")).toBeNull();
    const reads = read.mock.calls.length;
    await advance(POLL_MS * 3);
    expect(read).toHaveBeenCalledTimes(reads);
  });

  it("R765 a Conquest pairing goes to the series screen", async () => {
    vi.useFakeTimers();
    signIn();
    rememberQueued("bo3");
    const read = vi.fn<(token: string) => Promise<MeResponse>>().mockResolvedValue(me(null, "s-4"));
    render(<Follower path={paths.landing} read={read} />);
    await flush();
    await advance(MATCH_FOUND_NAV_DELAY_MS);
    expect(navigate).toHaveBeenCalledWith("/series/s-4");
  });

  it("R765 a read that fails is ignored and retried; a tab with no session forgets the queue", async () => {
    vi.useFakeTimers();
    signIn();
    rememberQueued("random");
    const read = vi.fn<(token: string) => Promise<MeResponse>>().mockRejectedValue(new Error("network down"));
    render(<Follower path={paths.decks} read={read} />);
    await flush();
    await advance(POLL_MS);
    expect(read).toHaveBeenCalledTimes(2);
    expect(readQueued()).toBe("random");
    expect(navigate).not.toHaveBeenCalled();
    cleanup();

    window.localStorage.clear();
    render(<Follower path={paths.decks} read={read} />);
    await flush();
    expect(readQueued()).toBeNull();
  });
});
