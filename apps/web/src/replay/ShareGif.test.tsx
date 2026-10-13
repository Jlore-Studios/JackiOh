// "Share as GIF" in the replay player (#513, R513000): the panel in a real player over a folded game,
// with the export itself stubbed (its own tests are `gif/export.test.ts`'s). It holds what the viewer
// can do: make a GIF of the shown turn or of a range marked on the scrubber, watch its progress,
// cancel, be told a clip is too long, and share or download the result. The state sits above the board,
// so the jumps these tests make, each of which mounts the board afresh, keep it.

import { CATALOG } from "@jackioh/cards";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CatalogContext, lookupFromDefs } from "../game/catalog.ts";
import { fakeReplaySource, foldedReplaySteps } from "../test/replay.ts";
import { REPLAY_GIF_MAX_STEPS } from "./config.ts";
import { canShareFile, exportGif, shareGif } from "./gif/export.ts";
import type { GifExportResult } from "./gif/export.ts";
import ReplayPlayer, { replayTestid } from "./ReplayPlayer.tsx";
import { gifTestid } from "./ShareGif.tsx";
import { clampClip } from "./steps.ts";

vi.mock("./gif/export.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./gif/export.ts")>();
  return { ...actual, exportGif: vi.fn(), canShareFile: vi.fn(), shareGif: vi.fn() };
});

const steps = foldedReplaySteps();
const lookup = lookupFromDefs(CATALOG);
const exporting = vi.mocked(exportGif);

/** The folded game's turns, each as its first and last step. */
const TURNS = [...new Set(steps.map((step) => step.turn))].map((turn) => {
  const indices = steps.flatMap((step, index) => (step.turn === turn ? [index] : []));
  return { turn, first: indices[0] ?? 0, last: indices.at(-1) ?? 0 };
});
/** A turn of at least three steps, and a step inside it that is not its first. */
const TURN = TURNS.find((turn) => turn.last - turn.first >= 2) ?? TURNS[0];
const INSIDE = (TURN?.first ?? 0) + 1;

const GIF_FILE = new File([Uint8Array.from([0x47, 0x49, 0x46])], "jackioh-replay-1-2.gif", { type: "image/gif" });
const READY: GifExportResult = { kind: "ready", file: GIF_FILE, attempt: { preset: "landscape", fps: 10 } };

beforeEach(() => {
  exporting.mockReset();
  vi.mocked(canShareFile).mockReset();
  vi.mocked(shareGif).mockReset();
});

afterEach(() => {
  cleanup();
});

async function open(): Promise<void> {
  render(
    <CatalogContext.Provider value={lookup}>
      <ReplayPlayer source={fakeReplaySource(steps)} onLeave={vi.fn()} />
    </CatalogContext.Provider>,
  );
  await screen.findByTestId(replayTestid.controls);
}

async function jump(index: number): Promise<void> {
  fireEvent.change(screen.getByTestId(replayTestid.scrubber), { target: { value: String(index) } });
  await waitFor(() => expect(screen.getByTestId(replayTestid.player)).toHaveAttribute("data-step", String(index)));
}

function click(id: string): void {
  fireEvent.click(screen.getByTestId(id));
}

/** Opens the panel and presses Make GIF; resolves once the export has been asked for. */
async function make(): Promise<void> {
  if (screen.queryByTestId(gifTestid.make) === null) click(gifTestid.toggle);
  click(gifTestid.make);
  await waitFor(() => expect(exporting).toHaveBeenCalled());
}

function askedSteps(call = 0): number[] {
  return (exporting.mock.calls[call]?.[0].clip ?? []).map((step) => step.step);
}

describe("Share as GIF", () => {
  it("is closed until it is asked for, and opens beneath the toolbar", async () => {
    await open();
    expect(screen.queryByTestId(gifTestid.make)).toBeNull();
    const toggle = screen.getByTestId(gifTestid.toggle);
    expect(toggle).toHaveTextContent("Share as GIF");
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    click(gifTestid.toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByTestId(gifTestid.clip)).toHaveTextContent("Clip: this turn");
    expect(screen.getByTestId(gifTestid.turn)).toBeDisabled();
    expect(screen.getByTestId(gifTestid.shape)).toHaveValue("landscape");
    // The toolbar's own buttons are the player's, and the panel is not among them.
    expect(screen.getByTestId(replayTestid.controls)).not.toContainElement(screen.getByTestId(gifTestid.panel));
  });

  it("Share as GIF exports the shown turn by default and shows its progress", async () => {
    await open();
    await jump(INSIDE);
    let report: (frame: number) => void = () => undefined;
    let finish: (result: GifExportResult) => void = () => undefined;
    exporting.mockImplementation((_input, hooks) => {
      report = (frame) => {
        hooks.onProgress({ frame, frames: 12, attempt: 1, attempts: 3 });
      };
      return new Promise((resolve) => {
        finish = resolve;
      });
    });
    await make();

    const [first, last] = clampClip(TURN?.first ?? 0, TURN?.last ?? 0);
    expect(askedSteps()).toEqual(Array.from({ length: last - first + 1 }, (_, i) => first + i));
    expect(exporting.mock.calls[0]?.[0]).toMatchObject({ preset: "landscape", lookup });
    expect(screen.getByTestId(gifTestid.make)).toBeDisabled();
    expect(screen.getByTestId(gifTestid.progress)).toBeInTheDocument();

    report(5);
    await waitFor(() => expect(within(screen.getByTestId(gifTestid.panel)).getByRole("status")).toHaveTextContent("Drawing frame 5 of 12"));
    expect(screen.getByTestId(gifTestid.progress)).toHaveAttribute("max", "12");
    expect(screen.getByTestId(gifTestid.progress)).toHaveAttribute("value", "5");

    // The viewer moving the board on does not lose the export.
    await jump(INSIDE + 1);
    expect(screen.getByTestId(gifTestid.progress)).toHaveAttribute("value", "5");

    finish(READY);
    await waitFor(() => expect(screen.queryByTestId(gifTestid.progress)).toBeNull());
    expect(screen.getByTestId(gifTestid.share)).toBeInTheDocument();
  });

  it("the picked shape is the size tried first", async () => {
    await open();
    exporting.mockResolvedValue({ kind: "tooBig" });
    click(gifTestid.toggle);
    fireEvent.change(screen.getByTestId(gifTestid.shape), { target: { value: "square" } });
    await make();
    expect(exporting.mock.calls[0]?.[0].preset).toBe("square");
  });

  it("a range marked on the scrubber is the clip", async () => {
    await open();
    exporting.mockResolvedValue({ kind: "tooBig" });
    click(gifTestid.toggle);
    await jump(3);
    click(gifTestid.start);
    expect(screen.getByTestId(gifTestid.clip)).toHaveTextContent("Clip: steps 4 to 4");
    await jump(7);
    click(gifTestid.end);
    expect(screen.getByTestId(gifTestid.clip)).toHaveTextContent("Clip: steps 4 to 8");
    expect(screen.getByTestId(gifTestid.turn)).toBeEnabled();
    await make();
    expect(askedSteps()).toEqual([3, 4, 5, 6, 7]);

    // Marked the other way round it is the same clip, and a clip is capped.
    await jump(2);
    click(gifTestid.end);
    expect(screen.getByTestId(gifTestid.clip)).toHaveTextContent("Clip: steps 3 to 4");
    await jump(steps.length - 1);
    click(gifTestid.end);
    // The start mark is still on index 3 and the end has moved to the last step.
    const [lo, hi] = clampClip(3, steps.length - 1);
    expect(hi - lo + 1).toBeLessThanOrEqual(REPLAY_GIF_MAX_STEPS);
    expect(screen.getByTestId(gifTestid.clip)).toHaveTextContent(`Clip: steps ${String(lo + 1)} to ${String(hi + 1)}`);

    // This turn puts the marks away.
    click(gifTestid.turn);
    expect(screen.getByTestId(gifTestid.clip)).toHaveTextContent("Clip: this turn");
  });

  it("a clip over every cap says Pick a shorter clip", async () => {
    await open();
    exporting.mockResolvedValue({ kind: "tooBig" });
    await make();
    expect(await screen.findByRole("alert")).toHaveTextContent("Pick a shorter clip");
    expect(screen.getByTestId(gifTestid.tooBig)).toBeInTheDocument();
    expect(screen.queryByTestId(gifTestid.share)).toBeNull();
    expect(screen.getByTestId(gifTestid.make)).toBeEnabled();
    // Changing the clip clears it.
    click(gifTestid.start);
    expect(screen.queryByTestId(gifTestid.tooBig)).toBeNull();
  });

  it("an export that fails says why", async () => {
    await open();
    exporting.mockRejectedValue(new Error("the raster fell over"));
    await make();
    expect(await screen.findByTestId(gifTestid.failed)).toHaveTextContent("the raster fell over");
  });

  it("Cancel aborts the export", async () => {
    await open();
    let signal: AbortSignal | undefined;
    exporting.mockImplementation((_input, hooks) => {
      signal = hooks.signal;
      return new Promise((_resolve, reject) => {
        hooks.signal.addEventListener("abort", () => {
          reject(new DOMException("cancelled", "AbortError"));
        });
      });
    });
    await make();
    expect(signal?.aborted).toBe(false);
    click(gifTestid.cancel);
    expect(signal?.aborted).toBe(true);
    await waitFor(() => expect(screen.queryByTestId(gifTestid.progress)).toBeNull());
    expect(screen.queryByTestId(gifTestid.failed)).toBeNull();
    expect(screen.getByTestId(gifTestid.make)).toBeEnabled();
  });

  it("leaving the player aborts an export in flight", async () => {
    await open();
    let signal: AbortSignal | undefined;
    exporting.mockImplementation((_input, hooks) => {
      signal = hooks.signal;
      return new Promise(() => undefined);
    });
    await make();
    cleanup();
    expect(signal?.aborted).toBe(true);
  });

  it("a made GIF is shared or downloaded from its button", async () => {
    await open();
    exporting.mockResolvedValue(READY);
    vi.mocked(canShareFile).mockReturnValue(true);
    vi.mocked(shareGif).mockResolvedValue("shared");
    await make();
    const share = await screen.findByTestId(gifTestid.share);
    expect(share).toHaveTextContent("Share");
    expect(share).not.toHaveTextContent("Download");
    expect(canShareFile).toHaveBeenCalledWith(GIF_FILE);
    fireEvent.click(share);
    expect(shareGif).toHaveBeenCalledWith(GIF_FILE);

    // Where the device cannot share a file, the button downloads.
    vi.mocked(canShareFile).mockReturnValue(false);
    vi.mocked(shareGif).mockResolvedValue("downloaded");
    click(gifTestid.make);
    await waitFor(() => expect(screen.getByTestId(gifTestid.share)).toHaveTextContent("Download"));
    fireEvent.click(screen.getByTestId(gifTestid.share));
    expect(shareGif).toHaveBeenCalledTimes(2);

    // A dismissed share sheet is not a failure.
    vi.mocked(canShareFile).mockReturnValue(true);
    vi.mocked(shareGif).mockRejectedValue(new DOMException("dismissed", "AbortError"));
    click(gifTestid.make);
    await waitFor(() => expect(screen.getByTestId(gifTestid.share)).toHaveTextContent("Share"));
    fireEvent.click(screen.getByTestId(gifTestid.share));
    await waitFor(() => expect(shareGif).toHaveBeenCalledTimes(3));
    expect(screen.queryByTestId(gifTestid.failed)).toBeNull();
  });
});
