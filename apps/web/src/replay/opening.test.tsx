// R768000, issue #768: how a result screen opens a replay. "Watch the replay" opens at step 0;
// "Share the last turn" opens at the last step with "Share as GIF" open and that turn's GIF already
// being made, once. A finished match's replay, whose length the client does not hold, loads its
// first page first (`openOnlineMatchReplay`, `OpenedReplay`) and says why when it cannot.

import { CATALOG } from "@jackioh/cards";
import { REPLAY_PAGE_STEPS } from "@jackioh/engine/config";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CatalogContext, lookupFromDefs } from "../game/catalog.ts";
import { ApiRequestError, getReplaySteps } from "../net/api.ts";
import { fakeReplaySource, foldedReplaySteps } from "../test/replay.ts";
import { REPLAY_OPEN_ATTEMPTS, REPLAY_OPEN_RETRY_MS } from "./config.ts";
import { canShareFile, exportGif, shareGif } from "./gif/export.ts";
import type { GifExportResult } from "./gif/export.ts";
import OpenedReplay from "./OpenedReplay.tsx";
import ReplayPlayer, { replayTestid } from "./ReplayPlayer.tsx";
import { gifTestid } from "./ShareGif.tsx";
import { openOnlineMatchReplay } from "./sources.ts";

vi.mock("./gif/export.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./gif/export.ts")>();
  return { ...actual, exportGif: vi.fn(), canShareFile: vi.fn(), shareGif: vi.fn() };
});

vi.mock("../net/api.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/api.ts")>();
  return { ...actual, getReplaySteps: vi.fn() };
});

const steps = foldedReplaySteps();
const last = steps.length - 1;
const lookup = lookupFromDefs(CATALOG);
const exporting = vi.mocked(exportGif);
const GIF_FILE = new File([Uint8Array.from([0x47, 0x49, 0x46])], "jackioh-replay-last.gif", { type: "image/gif" });
const READY: GifExportResult = { kind: "ready", file: GIF_FILE, attempt: { preset: "landscape", fps: 10 } };
/** The steps of the folded game's last turn. */
const LAST_TURN = steps.flatMap((step) => (step.turn === steps[last]?.turn ? [step.step] : []));

beforeEach(() => {
  exporting.mockReset();
  vi.mocked(canShareFile).mockReset();
  vi.mocked(shareGif).mockReset();
  vi.mocked(getReplaySteps).mockReset();
});

afterEach(() => {
  cleanup();
});

function serveSteps(): void {
  vi.mocked(getReplaySteps).mockImplementation((_token, matchId, from, count) =>
    Promise.resolve({ matchId, from, steps: steps.slice(from, from + Math.min(count, REPLAY_PAGE_STEPS)), total: steps.length }),
  );
}

describe("R768000 a result screen's way into a replay", () => {
  it("R768000 Watch the replay opens at step 0 and Share the last turn at the last step", async () => {
    const { unmount } = render(
      <CatalogContext.Provider value={lookup}>
        <ReplayPlayer source={fakeReplaySource(steps)} onLeave={vi.fn()} />
      </CatalogContext.Provider>,
    );
    await waitFor(() => expect(screen.getByTestId(replayTestid.player)).toHaveAttribute("data-step", "0"));
    unmount();

    render(
      <CatalogContext.Provider value={lookup}>
        <ReplayPlayer source={fakeReplaySource(steps)} onLeave={vi.fn()} start="last" />
      </CatalogContext.Provider>,
    );
    await waitFor(() => expect(screen.getByTestId(replayTestid.player)).toHaveAttribute("data-step", String(last)));
    expect(screen.getByTestId(replayTestid.end)).toBeDisabled();
    expect(screen.getByTestId(gifTestid.toggle), "Share as GIF waits to be opened").toHaveAttribute("aria-expanded", "false");
    expect(exporting).not.toHaveBeenCalled();
  });

  it("R768000 Share the last turn opens Share as GIF and makes the last turn's GIF, once", async () => {
    exporting.mockResolvedValue(READY);
    vi.mocked(canShareFile).mockReturnValue(false);
    render(
      <CatalogContext.Provider value={lookup}>
        <ReplayPlayer source={fakeReplaySource(steps)} onLeave={vi.fn()} start="last" shareOnOpen />
      </CatalogContext.Provider>,
    );
    await waitFor(() => expect(exporting).toHaveBeenCalledTimes(1));
    expect(screen.getByTestId(replayTestid.player)).toHaveAttribute("data-step", String(last));
    expect((exporting.mock.calls[0]?.[0].clip ?? []).map((step) => step.step)).toEqual(LAST_TURN);
    await waitFor(() => expect(screen.getByTestId(gifTestid.toggle)).toHaveAttribute("aria-expanded", "true"));
    expect(await screen.findByTestId(gifTestid.share)).toBeInTheDocument();

    // A step back and forward again is the viewer's own move: nothing is made again.
    fireEvent.click(screen.getByTestId(replayTestid.backStep));
    await waitFor(() => expect(screen.getByTestId(replayTestid.player)).toHaveAttribute("data-step", String(last - 1)));
    fireEvent.click(screen.getByTestId(replayTestid.forwardStep));
    await waitFor(() => expect(screen.getByTestId(replayTestid.player)).toHaveAttribute("data-step", String(last)));
    expect(exporting).toHaveBeenCalledTimes(1);
  });

  it("R768000 a finished match's replay learns its length from its first page, which step 0 reuses", async () => {
    serveSteps();
    const source = await openOnlineMatchReplay("token-1", "m-1");
    expect(source.total).toBe(steps.length);
    expect(getReplaySteps).toHaveBeenCalledWith("token-1", "m-1", 0, REPLAY_PAGE_STEPS);
    expect(await source.page(0, 4)).toEqual(steps.slice(0, 4));
    expect(getReplaySteps).toHaveBeenCalledTimes(1);
    expect(await source.page(REPLAY_PAGE_STEPS, 4)).toEqual(steps.slice(REPLAY_PAGE_STEPS, REPLAY_PAGE_STEPS + 4));
    expect(getReplaySteps).toHaveBeenLastCalledWith("token-1", "m-1", REPLAY_PAGE_STEPS, 4);
  });

  it("R768000 a match whose result is still being written is asked again, and a match that never finishes says so", async () => {
    const waits: number[] = [];
    const wait = (ms: number): Promise<void> => {
      waits.push(ms);
      return Promise.resolve();
    };
    const notYet = new ApiRequestError(409, { code: "conflict", message: "this match has not finished" });
    serveSteps();
    vi.mocked(getReplaySteps).mockRejectedValueOnce(notYet).mockRejectedValueOnce(notYet);
    const source = await openOnlineMatchReplay("token-1", "m-1", wait);
    expect(source.total).toBe(steps.length);
    expect(waits).toEqual([REPLAY_OPEN_RETRY_MS, REPLAY_OPEN_RETRY_MS]);

    vi.mocked(getReplaySteps).mockReset().mockRejectedValue(notYet);
    waits.length = 0;
    await expect(openOnlineMatchReplay("token-1", "m-2", wait)).rejects.toMatchObject({ reason: "error" });
    expect(getReplaySteps).toHaveBeenCalledTimes(REPLAY_OPEN_ATTEMPTS);
    expect(waits).toHaveLength(REPLAY_OPEN_ATTEMPTS - 1);
  });

  it("R768000 OpenedReplay loads the source, then plays it; a refused first page says why, with the way back", async () => {
    serveSteps();
    const { unmount } = render(
      <CatalogContext.Provider value={lookup}>
        <OpenedReplay load={() => openOnlineMatchReplay("token-1", "m-1")} onLeave={vi.fn()} />
      </CatalogContext.Provider>,
    );
    expect(screen.getByTestId(replayTestid.loading)).toBeInTheDocument();
    await waitFor(() => expect(screen.getByTestId(replayTestid.player)).toHaveAttribute("data-step", "0"));
    unmount();

    vi.mocked(getReplaySteps).mockRejectedValue(
      new ApiRequestError(422, { code: "replay_unavailable", message: "this replay cannot be shown: earlier_patch", details: { reason: "earlier_patch" } }),
    );
    const onLeave = vi.fn();
    render(<OpenedReplay load={() => openOnlineMatchReplay("token-1", "m-2")} onLeave={onLeave} />);
    expect(await screen.findByTestId(replayTestid.failure)).toHaveTextContent("This game was played on an earlier patch.");
    fireEvent.click(screen.getByTestId(replayTestid.back));
    expect(onLeave).toHaveBeenCalledTimes(1);
  });
});
