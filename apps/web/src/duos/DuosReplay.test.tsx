// #965, SPEC §13.9, B41: a finished co-op game's replay. The screen reads `getDuosReplay` once, steps
// the seat's `DuosView`s with the replay player's controls (a round standing for a turn), draws each
// on `DuosBoard`'s replay mode, and says why when the server refuses, as a 1v1 replay does (R768).
// The views are fixtures: the screen computes nothing from them (CLAUDE.md rule 7).

import { CATALOG } from "@jackioh/cards";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { closeInspect } from "../cards/index.ts";
import { resetFxSettingsForTests } from "../fx/settings.ts";
import { CatalogContext, lookupFromDefs } from "../game/catalog.ts";
import { ApiRequestError } from "../net/api.ts";
import { getDuosReplay } from "../net/duos.ts";
import { REPLAY_OPEN_ATTEMPTS, REPLAY_OPEN_RETRY_MS, REPLAY_PLAY_GAP_MS } from "../replay/config.ts";
import { replayTestid } from "../replay/ReplayPlayer.tsx";
import { REPLAY_FAILURE_TEXT } from "../replay/source.ts";
import { onlineFailure } from "../replay/sources.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { baseView } from "../test/fixtures.ts";
import type { DuosTeamView } from "../wire/generated/DuosTeamView.ts";
import type { DuosView } from "../wire/generated/DuosView.ts";
import DuosReplay, { nextRound, openDuosReplay, previousRound, type DuosReplaySteps } from "./DuosReplay.tsx";
import { duosTestid } from "./testids.ts";

vi.mock("../net/duos.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/duos.ts")>();
  return { ...actual, getDuosReplay: vi.fn() };
});

const lookup = lookupFromDefs(CATALOG);
const MATCH = "dm-1";

/** Each step's round: a round holds several steps, as a live game's rounds hold several actions. */
const ROUNDS = [0, 1, 1, 1, 2, 2, 3];
const LAST = ROUNDS.length - 1;
const CONCEDED: DuosView["result"] = { outcome: "win", winner: "b", reason: "concede" };

function teamView(team: "a" | "b"): DuosTeamView {
  return {
    team,
    pool: 20,
    poolCap: 20,
    sides: [
      { own: 10, spent: false },
      { own: 10, spent: false },
    ],
    ended: [false, false],
    passesLeft: [0, 0],
    passCost: 1,
  };
}

function step(round: number, result: DuosView["result"] = null): DuosView {
  return {
    seat: "a1",
    own: baseView(),
    teammate: baseView(),
    team: teamView("a"),
    enemy: teamView("b"),
    round,
    concedeOfferedBy: null,
    result,
  };
}

const STEPS: DuosView[] = ROUNDS.map((round, index) => step(round, index === LAST ? CONCEDED : null));

function serve(steps: readonly DuosView[] = STEPS): void {
  vi.mocked(getDuosReplay).mockResolvedValue({ seat: "a1", steps: [...steps] });
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.mocked(getDuosReplay).mockReset();
});

afterEach(() => {
  cleanup();
  closeInspect();
  vi.useRealTimers();
  window.localStorage.clear();
  __resetSettingsForTests();
  resetFxSettingsForTests();
});

/** Lets the promises the screen waits on settle, without moving the clock. */
async function flush(): Promise<void> {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(0);
  });
}

async function advance(ms: number): Promise<void> {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(ms);
  });
}

async function open(onLeave: () => void = vi.fn()): Promise<void> {
  render(
    <CatalogContext.Provider value={lookup}>
      <DuosReplay matchId={MATCH} onLeave={onLeave} />
    </CatalogContext.Provider>,
  );
  await flush();
}

async function press(id: string): Promise<void> {
  fireEvent.click(screen.getByTestId(id));
  await flush();
}

async function jump(index: number): Promise<void> {
  fireEvent.change(screen.getByTestId(replayTestid.scrubber), { target: { value: String(index) } });
  await flush();
}

/** The step on the board, as the screen says it. */
function shownStep(): number {
  return Number(screen.getByTestId(duosTestid.replay).getAttribute("data-step"));
}

/** Which mount of the board shows it: it changes when the board is mounted afresh. */
function shownBoard(): number {
  return Number(screen.getByTestId(duosTestid.replay).getAttribute("data-board"));
}

describe("the co-op replay screen", () => {
  it("B41 opens at step 0 with the replay controls, reading the replay once", async () => {
    serve();
    render(
      <CatalogContext.Provider value={lookup}>
        <DuosReplay matchId={MATCH} onLeave={vi.fn()} />
      </CatalogContext.Provider>,
    );
    expect(screen.getByTestId(replayTestid.loading)).toBeInTheDocument();
    await flush();
    expect(screen.queryByTestId(replayTestid.loading)).toBeNull();
    expect(shownStep()).toBe(0);
    expect(screen.getByTestId(replayTestid.readout)).toHaveTextContent(`Step 1 of ${String(ROUNDS.length)} · Round 0`);
    const toolbar = screen.getByTestId(replayTestid.controls);
    expect(toolbar).toHaveAttribute("role", "toolbar");
    const labels = [...toolbar.querySelectorAll("button")].map((control) => control.getAttribute("aria-label"));
    expect(labels).toEqual(["To the start", "Back a turn", "Back a step", "Play", "Forward a step", "Forward a turn", "To the end"]);
    expect(getDuosReplay).toHaveBeenCalledTimes(1);
    expect(getDuosReplay).toHaveBeenCalledWith(MATCH);

    await press(replayTestid.forwardStep);
    await press(replayTestid.end);
    await press(replayTestid.start);
    expect(getDuosReplay, "the steps are held: no move asks the server again").toHaveBeenCalledTimes(1);
  });

  it("B41 each control reaches its step, a round standing for a turn", async () => {
    serve();
    await open();
    for (const id of [replayTestid.start, replayTestid.backTurn, replayTestid.backStep]) expect(screen.getByTestId(id), id).toBeDisabled();
    for (const id of [replayTestid.play, replayTestid.forwardStep, replayTestid.forwardTurn, replayTestid.end]) {
      expect(screen.getByTestId(id), id).toBeEnabled();
    }

    await press(replayTestid.forwardStep);
    expect(shownStep()).toBe(1);
    expect(screen.getByTestId(replayTestid.readout)).toHaveTextContent("Step 2 of 7 · Round 1");
    await press(replayTestid.forwardTurn);
    expect(shownStep(), "the first step of the next round").toBe(4);
    expect(screen.getByTestId(replayTestid.readout)).toHaveTextContent("Round 2");
    await press(replayTestid.forwardTurn);
    expect(shownStep()).toBe(6);

    for (const id of [replayTestid.play, replayTestid.forwardStep, replayTestid.forwardTurn, replayTestid.end]) {
      expect(screen.getByTestId(id), id).toBeDisabled();
    }
    await press(replayTestid.backStep);
    expect(shownStep()).toBe(5);
    await press(replayTestid.forwardTurn);
    expect(shownStep()).toBe(6);

    await press(replayTestid.backTurn);
    expect(shownStep(), "the start of the round the step is in, or of the one before it").toBe(4);
    await press(replayTestid.backTurn);
    expect(shownStep()).toBe(1);
    await press(replayTestid.backTurn);
    expect(shownStep()).toBe(0);

    await press(replayTestid.end);
    expect(shownStep()).toBe(LAST);
    await press(replayTestid.start);
    expect(shownStep()).toBe(0);
  });

  it("B41 a step forward keeps the board and any other move mounts a fresh one", async () => {
    serve();
    await open();
    const first = shownBoard();
    await press(replayTestid.forwardStep);
    expect(shownBoard(), "a step forward carries the board on").toBe(first);
    await press(replayTestid.forwardStep);
    expect(shownBoard()).toBe(first);
    await press(replayTestid.backStep);
    const afterBack = shownBoard();
    expect(afterBack, "a step back mounts afresh").toBeGreaterThan(first);
    await jump(5);
    expect(shownBoard(), "a jump mounts afresh").toBeGreaterThan(afterBack);
    expect(screen.getByTestId(replayTestid.scrubber), "the controls outlive the board").toBeInTheDocument();
  });

  it("B41 the scrubber jumps to its value", async () => {
    serve();
    await open();
    const scrubber = screen.getByTestId(replayTestid.scrubber);
    expect(scrubber).toHaveAttribute("max", String(LAST));
    await jump(5);
    expect(shownStep()).toBe(5);
    expect(screen.getByTestId(replayTestid.scrubber)).toHaveValue("5");
    await jump(0);
    expect(shownStep()).toBe(0);
  });

  it("B41 play advances one step per gap and stops at the last step", async () => {
    // Reduced motion drains the board's animations at once, so only the gap is left to wait out.
    writeSettings({ reduceMotion: true });
    serve();
    await open();
    const play = (): HTMLElement => screen.getByTestId(replayTestid.play);
    await press(replayTestid.play);
    expect(play()).toHaveAttribute("aria-pressed", "true");
    expect(play()).toHaveAttribute("aria-label", "Pause");
    await advance(REPLAY_PLAY_GAP_MS - 1);
    expect(shownStep(), "not before the gap").toBe(0);
    await advance(1);
    expect(shownStep()).toBe(1);

    // A move of the viewer's own stops it.
    await press(replayTestid.forwardStep);
    expect(play()).toHaveAttribute("aria-pressed", "false");
    await advance(4 * REPLAY_PLAY_GAP_MS);
    expect(shownStep()).toBe(2);

    // Playing runs out at the last step and stops there.
    await jump(LAST - 1);
    await press(replayTestid.play);
    await advance(REPLAY_PLAY_GAP_MS);
    expect(shownStep()).toBe(LAST);
    expect(play()).toHaveAttribute("aria-pressed", "false");
    expect(play()).toBeDisabled();
  });

  it("B41 the last step names the team's result and draws no result panel", async () => {
    serve();
    await open();
    await press(replayTestid.end);
    expect(screen.queryByTestId(duosTestid.result)).toBeNull();
    expect(screen.getByTestId(duosTestid.round)).toHaveTextContent("Your team lost");
    await press(replayTestid.start);
    expect(screen.getByTestId(duosTestid.round)).not.toHaveTextContent("Your team lost");
  });

  it("B41 nothing live is offered on the replay's board", async () => {
    serve();
    await open();
    expect(screen.getByTestId(duosTestid.board)).toBeInTheDocument();
    expect(screen.getByTestId("game")).toHaveAttribute("data-replay", "true");
    expect(screen.queryByTestId(duosTestid.concede)).toBeNull();
    expect(screen.queryByTestId(duosTestid.waiting)).toBeNull();
    expect(screen.queryByTestId(duosTestid.cardActions)).toBeNull();
  });

  it("B41 a refusal reads as a 1v1 replay's does, with the way back", async () => {
    const refusals: { error: ApiRequestError; text: string }[] = [
      { error: new ApiRequestError(410, { code: "gone", message: "expired" }), text: REPLAY_FAILURE_TEXT.expired },
      {
        error: new ApiRequestError(422, {
          code: "replay_unavailable",
          message: "this replay cannot be shown: earlier_patch",
          details: { reason: "earlier_patch" },
        }),
        text: REPLAY_FAILURE_TEXT.earlier_patch,
      },
      {
        error: new ApiRequestError(422, {
          code: "replay_unavailable",
          message: "this replay cannot be shown: rules_changed",
          details: { reason: "rules_changed" },
        }),
        text: REPLAY_FAILURE_TEXT.rules_changed,
      },
    ];
    // R768: an unknown match and a non-participant's read the same, and say it as a 1v1 replay's 404 does.
    const unknown = new ApiRequestError(404, { code: "not_found", message: "no such co-op match" });
    refusals.push({ error: unknown, text: "no such co-op match" });
    expect(onlineFailure(unknown).reason).toBe("error");

    for (const { error, text } of refusals) {
      vi.mocked(getDuosReplay).mockReset().mockRejectedValue(error);
      const onLeave = vi.fn();
      const { unmount } = render(
        <CatalogContext.Provider value={lookup}>
          <DuosReplay matchId={MATCH} onLeave={onLeave} />
        </CatalogContext.Provider>,
      );
      await flush();
      expect(screen.getByTestId(replayTestid.failure), error.code).toHaveTextContent(text);
      expect(screen.queryByTestId(duosTestid.replay)).toBeNull();
      fireEvent.click(screen.getByTestId(replayTestid.back));
      expect(onLeave).toHaveBeenCalledTimes(1);
      unmount();
    }
  });

  it("B41 the way back is on the board's rail while the replay plays", async () => {
    serve();
    const onLeave = vi.fn();
    await open(onLeave);
    fireEvent.click(screen.getByTestId(replayTestid.back));
    expect(onLeave).toHaveBeenCalledTimes(1);
  });
});

describe("opening a co-op replay", () => {
  it("B41 a match whose result is still being written is asked again, REPLAY_OPEN_ATTEMPTS times in all", async () => {
    const waits: number[] = [];
    const wait = (ms: number): Promise<void> => {
      waits.push(ms);
      return Promise.resolve();
    };
    const notYet = new ApiRequestError(409, { code: "conflict", message: "this co-op match has not finished" });
    serve();
    vi.mocked(getDuosReplay).mockRejectedValueOnce(notYet).mockRejectedValueOnce(notYet);
    const opened = await openDuosReplay(MATCH, wait);
    expect(opened).toEqual(STEPS);
    expect(waits).toEqual([REPLAY_OPEN_RETRY_MS, REPLAY_OPEN_RETRY_MS]);

    vi.mocked(getDuosReplay).mockReset().mockRejectedValue(notYet);
    waits.length = 0;
    await expect(openDuosReplay(MATCH, wait)).rejects.toMatchObject({ reason: "error" });
    expect(getDuosReplay).toHaveBeenCalledTimes(REPLAY_OPEN_ATTEMPTS);
    expect(waits).toHaveLength(REPLAY_OPEN_ATTEMPTS - 1);
  });

  it("B41 any other refusal rejects after one call, and an empty answer is an error", async () => {
    const wait = vi.fn<(ms: number) => Promise<void>>();
    vi.mocked(getDuosReplay).mockRejectedValue(new ApiRequestError(404, { code: "not_found", message: "no such co-op match" }));
    await expect(openDuosReplay(MATCH, wait)).rejects.toMatchObject({ reason: "error", message: "no such co-op match" });
    expect(getDuosReplay).toHaveBeenCalledTimes(1);
    expect(wait).not.toHaveBeenCalled();

    serve([]);
    await expect(openDuosReplay(MATCH, wait)).rejects.toMatchObject({ reason: "error" });
  });
});

describe("a round standing for a turn", () => {
  const steps = STEPS as unknown as DuosReplaySteps;

  it("B41 nextRound is the first step of a later round, or the last step", () => {
    expect(ROUNDS.map((_, at) => nextRound(steps, at))).toEqual([1, 4, 4, 4, 6, 6, 6]);
  });

  it("B41 previousRound is the first step of the round the step is in, or of the one before it", () => {
    expect(ROUNDS.map((_, at) => previousRound(steps, at))).toEqual([0, 0, 1, 1, 1, 4, 4]);
  });
});
