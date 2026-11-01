// The replay player (#511) over a fake source of one seeded game's steps (`test/replay.ts`), the
// player's own contract: no log, seed or state reaches it. The rail's modifier list follows the step
// on the board (R169).

import { CATALOG } from "@jackioh/cards";
import type { ReplayStep } from "@jackioh/engine";
import type { CardView, PendingOption, PendingView, PlayerView, SideView } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { INSPECT_HOVER, closeInspect } from "../cards/index.ts";
import { HOVER_DELAY_MS } from "../cards/inspect/index.ts";
import { resetFxSettingsForTests, setFxSettings } from "../fx/settings.ts";
import { ANIMATIONS, newEventsSince } from "../game/animations.ts";
import { CatalogContext, lookupFromDefs } from "../game/catalog.ts";
import { testid } from "../game/contract.ts";
import Game from "../game/Game.tsx";
import { modifierListTestid } from "../game/ModifierList.tsx";
import { revealTestid } from "../game/reveal.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { opponentModifiers, pendingFor, waitingPending, yourModifiers } from "../test/fixtures.ts";
import { fakeReplaySource, foldedReplaySteps } from "../test/replay.ts";
import type { FakeReplaySource } from "../test/replay.ts";
import { expectSkeleton } from "../test/skeleton.ts";
import { REPLAY_PLAY_GAP_MS } from "./config.ts";
import ReplayPlayer, { replayTestid } from "./ReplayPlayer.tsx";
import { REPLAY_FAILURE_TEXT, ReplayFailure } from "./source.ts";

const steps = foldedReplaySteps();
const LAST = steps.length - 1;
const TURN_STARTS = steps.flatMap((step, index) => (index === 0 || steps[index - 1]?.turn !== step.turn ? [index] : []));
const lookup = lookupFromDefs(CATALOG);

/** The cards of a hand a view shows: the viewer's own always, the opponent's only once the game is over (R434). */
function cardsOf(hand: SideView["hand"]): CardView[] {
  return Array.isArray(hand) ? hand : [];
}

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  cleanup();
  closeInspect();
  vi.useRealTimers();
  window.localStorage.clear();
  __resetSettingsForTests();
  resetFxSettingsForTests();
});

/** Lets the promises a move waits on settle, without moving the clock. */
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

async function open(source: FakeReplaySource = fakeReplaySource(steps), onLeave: () => void = vi.fn()): Promise<FakeReplaySource> {
  render(
    <CatalogContext.Provider value={lookup}>
      <ReplayPlayer source={source} onLeave={onLeave} />
    </CatalogContext.Provider>,
  );
  await flush();
  return source;
}

async function press(id: string): Promise<void> {
  fireEvent.click(screen.getByTestId(id));
  await flush();
}

async function key(init: KeyboardEventInit): Promise<void> {
  fireEvent.keyDown(window, init);
  await flush();
}

async function jump(index: number): Promise<void> {
  fireEvent.change(screen.getByTestId(replayTestid.scrubber), { target: { value: String(index) } });
  await flush();
}

function shownStep(): number {
  return Number(screen.getByTestId(replayTestid.player).getAttribute("data-step"));
}

/** Which mount of the board shows it: it changes when the board is mounted afresh. */
function shownBoard(): number {
  return Number(screen.getByTestId(replayTestid.player).getAttribute("data-board"));
}

function gaps(): number {
  return document.querySelectorAll('[data-event="gap"]').length;
}

describe("the replay player", () => {
  it("opens on the first step and reads its readout", async () => {
    render(
      <CatalogContext.Provider value={lookup}>
        <ReplayPlayer source={fakeReplaySource(steps)} onLeave={vi.fn()} />
      </CatalogContext.Provider>,
    );
    expect(screen.getByTestId(replayTestid.loading)).toBeInTheDocument();
    expectSkeleton(screen.getByTestId(replayTestid.loading), "panel");
    await flush();
    expect(screen.queryByTestId(replayTestid.loading)).toBeNull();
    expect(screen.queryByText("Loading the replay…")).toBeNull();
    expect(shownStep()).toBe(0);
    expect(screen.getByTestId(replayTestid.readout)).toHaveTextContent(`Step 1 of ${String(steps.length)} · Turn ${String(steps[0]?.turn)}`);
    const toolbar = screen.getByTestId(replayTestid.controls);
    expect(toolbar).toHaveAttribute("role", "toolbar");
    const labels = [...toolbar.querySelectorAll("button")].map((control) => control.getAttribute("aria-label"));
    expect(labels).toEqual(["To the start", "Back a turn", "Back a step", "Play", "Forward a step", "Forward a turn", "To the end"]);
    expect(TURN_STARTS.length, "the folded game spans enough turns").toBeGreaterThanOrEqual(3);
  });

  it("each control reaches the expected step", async () => {
    await open();
    const ids = {
      backs: [replayTestid.start, replayTestid.backTurn, replayTestid.backStep],
      forwards: [replayTestid.play, replayTestid.forwardStep, replayTestid.forwardTurn, replayTestid.end],
    };
    const [, second = 0, third = 0] = TURN_STARTS;

    for (const id of ids.backs) expect(screen.getByTestId(id), id).toBeDisabled();
    for (const id of ids.forwards) expect(screen.getByTestId(id), id).toBeEnabled();

    await press(replayTestid.forwardStep);
    expect(shownStep()).toBe(1);
    expect(screen.getByTestId(replayTestid.readout)).toHaveTextContent(`Step 2 of ${String(steps.length)}`);
    for (const id of ids.backs) expect(screen.getByTestId(id), id).toBeEnabled();

    await press(replayTestid.forwardTurn);
    expect(shownStep()).toBe(second);
    expect(screen.getByTestId(replayTestid.readout)).toHaveTextContent(`Turn ${String(steps[second]?.turn)}`);
    await press(replayTestid.forwardTurn);
    expect(shownStep()).toBe(third);

    // Two steps into that turn: back a turn is its own first step, and again the turn before's.
    await press(replayTestid.forwardStep);
    await press(replayTestid.forwardStep);
    expect(shownStep()).toBe(third + 2);
    await press(replayTestid.backTurn);
    expect(shownStep()).toBe(third);
    await press(replayTestid.backTurn);
    expect(shownStep()).toBe(second);

    await press(replayTestid.backStep);
    expect(shownStep()).toBe(second - 1);
    await press(replayTestid.start);
    expect(shownStep()).toBe(0);

    await press(replayTestid.end);
    expect(shownStep()).toBe(LAST);
    expect(screen.getByTestId(replayTestid.readout)).toHaveTextContent(`Step ${String(steps.length)} of ${String(steps.length)}`);
    for (const id of ids.forwards) expect(screen.getByTestId(id), id).toBeDisabled();
    for (const id of ids.backs) expect(screen.getByTestId(id), id).toBeEnabled();

    // In the last turn there is no turn to go forward to: the end.
    await press(replayTestid.backStep);
    expect(shownStep()).toBe(LAST - 1);
    await press(replayTestid.forwardTurn);
    expect(shownStep()).toBe(LAST);
  });

  it("the scrubber jumps to its value", async () => {
    await open();
    const scrubber = screen.getByTestId(replayTestid.scrubber);
    expect(scrubber).toHaveAttribute("min", "0");
    expect(scrubber).toHaveAttribute("max", String(LAST));
    expect(scrubber).toHaveValue(String(0));
    // Into the second page, which has to be fetched, and back to the first.
    await jump(20);
    expect(shownStep()).toBe(20);
    expect(scrubber).toHaveValue("20");
    await jump(5);
    expect(shownStep()).toBe(5);
    await jump(LAST);
    expect(shownStep()).toBe(LAST);
  });

  it("the scrubber moves the board when it is let go, not at every tick, and a move keeps focus on its control", async () => {
    await open();
    const scrubber = screen.getByTestId(replayTestid.scrubber);
    const board = shownBoard();
    fireEvent.pointerDown(scrubber);
    fireEvent.change(scrubber, { target: { value: "7" } });
    fireEvent.change(scrubber, { target: { value: "20" } });
    await flush();
    expect(scrubber, "the same scrubber, still under the pointer").toBe(screen.getByTestId(replayTestid.scrubber));
    expect(scrubber).toHaveValue("20");
    expect(shownStep(), "nothing moved yet").toBe(0);
    expect(shownBoard()).toBe(board);
    fireEvent.pointerUp(scrubber);
    await flush();
    expect(shownStep()).toBe(20);
    expect(screen.getByTestId(replayTestid.scrubber)).toHaveValue("20");

    // The board is new elements after a jump; the control that made it is given the focus again.
    const forward = screen.getByTestId(replayTestid.backStep);
    forward.focus();
    expect(document.activeElement).toBe(forward);
    await press(replayTestid.backStep);
    expect(shownStep()).toBe(19);
    expect(screen.getByTestId(replayTestid.backStep)).not.toBe(forward);
    expect(document.activeElement).toBe(screen.getByTestId(replayTestid.backStep));
  });

  it("the arrow keys move a step and shift+arrow a turn", async () => {
    await open();
    const [, second = 0, third = 0] = TURN_STARTS;
    await key({ key: "ArrowRight" });
    expect(shownStep()).toBe(1);
    await key({ key: "ArrowRight" });
    expect(shownStep()).toBe(2);
    await key({ key: "ArrowLeft" });
    expect(shownStep()).toBe(1);
    await key({ key: "ArrowRight", shiftKey: true });
    expect(shownStep()).toBe(second);
    await key({ key: "ArrowRight", shiftKey: true });
    expect(shownStep()).toBe(third);
    await key({ key: "ArrowLeft", shiftKey: true });
    expect(shownStep()).toBe(second);
    await key({ key: "ArrowLeft", shiftKey: true });
    await key({ key: "ArrowLeft" });
    await key({ key: "ArrowLeft" });
    expect(shownStep()).toBe(0);
    await key({ key: "ArrowLeft" });
    expect(shownStep()).toBe(0);
    await jump(LAST);
    await key({ key: "ArrowRight" });
    await key({ key: "ArrowRight", shiftKey: true });
    expect(shownStep()).toBe(LAST);
  });

  it("leaves a key to the control that has it, and to the browser's own shortcuts", async () => {
    await open();
    // The scrubber moves itself with the arrows; the player does not move it twice.
    fireEvent.keyDown(screen.getByTestId(replayTestid.scrubber), { key: "ArrowRight" });
    await flush();
    expect(shownStep()).toBe(0);
    for (const modifier of [{ altKey: true }, { ctrlKey: true }, { metaKey: true }]) {
      await key({ key: "ArrowRight", ...modifier });
      expect(shownStep(), JSON.stringify(modifier)).toBe(0);
    }
    await key({ key: "ArrowUp" });
    expect(shownStep()).toBe(0);
  });

  it("play advances one step per gap at the effects speed and pause stops it", async () => {
    // Reduced motion drains the board's animations at once, so only the gap is left to wait out.
    writeSettings({ reduceMotion: true });
    await open();
    // Looked up each time: a jump mounts the board, and its controls, afresh.
    const play = (): HTMLElement => screen.getByTestId(replayTestid.play);
    expect(play()).toHaveAttribute("aria-pressed", "false");
    await press(replayTestid.play);
    expect(play()).toHaveAttribute("aria-pressed", "true");
    expect(play()).toHaveAttribute("aria-label", "Pause");

    await advance(REPLAY_PLAY_GAP_MS - 1);
    expect(shownStep(), "not before the gap").toBe(0);
    await advance(1);
    expect(shownStep()).toBe(1);
    await advance(REPLAY_PLAY_GAP_MS);
    expect(shownStep()).toBe(2);

    act(() => {
      setFxSettings({ speed: 2 });
    });
    await advance(REPLAY_PLAY_GAP_MS / 2 - 1);
    expect(shownStep()).toBe(2);
    await advance(1);
    expect(shownStep()).toBe(3);
    await advance(REPLAY_PLAY_GAP_MS / 2);
    expect(shownStep()).toBe(4);

    await press(replayTestid.play);
    expect(play()).toHaveAttribute("aria-pressed", "false");
    await advance(4 * REPLAY_PLAY_GAP_MS);
    expect(shownStep(), "paused").toBe(4);

    await press(replayTestid.play);
    await press(replayTestid.forwardStep);
    expect(shownStep()).toBe(5);
    expect(play()).toHaveAttribute("aria-pressed", "false");
    await advance(4 * REPLAY_PLAY_GAP_MS);
    expect(shownStep()).toBe(5);

    await jump(LAST - 1);
    await press(replayTestid.play);
    await advance(REPLAY_PLAY_GAP_MS / 2);
    expect(shownStep()).toBe(LAST);
    expect(play()).toHaveAttribute("aria-pressed", "false");
    expect(play()).toBeDisabled();
  });

  it("play waits for the board's animation before it starts the next step", async () => {
    const animated = firstAnimatedStep();
    await open();
    await jump(animated - 1);
    await press(replayTestid.play);
    // The step forward is taken at the gap; its animation then holds the one after.
    await advance(REPLAY_PLAY_GAP_MS);
    expect(shownStep()).toBe(animated);
    expect(screen.getByTestId("animation-queue")).toBeInTheDocument();
    await advance(REPLAY_PLAY_GAP_MS);
    expect(shownStep(), "the animation is still in flight").toBe(animated);
  });
});

/** The first step from the third on (the first two log almost nothing) whose own events animate. */
function firstAnimatedStep(): number {
  const found = steps.findIndex(
    (step, index) =>
      index >= 3 &&
      newEventsSince(steps[index - 1]?.view.events ?? [], step.view.events).some((event) => ANIMATIONS[event.type].durationMs > 0),
  );
  if (found < 0) throw new Error("no step of the folded game animates");
  return found;
}

describe("the board a replay draws", () => {
  it("the board offers no legal action or glow, any prompt only to look at, and clicks change nothing", async () => {
    await open();
    const quiet = (): void => {
      // R2211000: a prompt is drawn on a replay, read-only; none of it can be answered.
      expect(screen.queryByTestId("prompt-submit")).toBeNull();
      for (const modal of screen.queryAllByTestId("prompt-modal")) {
        expect(modal.hasAttribute("data-read-only") || modal.hasAttribute("data-prompt-waiting")).toBe(true);
      }
      expect(document.querySelector("[data-glow]")).toBeNull();
      expect(document.querySelector('[data-legal="true"]')).toBeNull();
      expect(screen.getByTestId(testid.endTurn)).toBeDisabled();
    };
    // Step 0 is the mulligan, which a live board answers with a prompt.
    expect(steps[0]?.view.pending).not.toBeNull();
    quiet();
    expect(screen.getByTestId("game")).toHaveAttribute("data-replay", "true");

    // A step where the viewer holds the turn and a hand, which a live board lights up.
    const mine = steps.findIndex(
      (step) => step.view.pending === null && step.view.active === step.view.viewer && cardsOf(step.view.you.hand).length > 0,
    );
    expect(mine, "the folded game has a step to play from").toBeGreaterThan(0);
    await jump(mine);
    expect(screen.queryByTestId("prompt-scrim")).toBeNull();
    quiet();

    const board = screen.getByTestId("game");
    const before = board.innerHTML;
    const hand = cardsOf(steps[mine]?.view.you.hand ?? { count: 0 })[0];
    if (hand === undefined) throw new Error("no card in hand");
    for (const id of [testid.handCard(hand.instanceId), testid.endTurn, testid.power, testid.concede, testid.hero("you"), testid.hero("opponent")]) {
      const target = screen.queryByTestId(id);
      if (target !== null) fireEvent.click(target);
    }
    await flush();
    expect(shownStep()).toBe(mine);
    quiet();
    expect(screen.queryByTestId(testid.concedeDialog)).toBeNull();
    // The one thing a click does is the hand's own lift of the card it touched, a look and not a move.
    expect(board.innerHTML.replaceAll(' data-lifted="true"', "")).toBe(before);

    // Looking still works: resting the pointer on a card in hand opens it large.
    fireEvent.pointerEnter(screen.getByTestId(testid.handCard(hand.instanceId)), { pointerType: "mouse" });
    act(() => {
      vi.advanceTimersByTime(HOVER_DELAY_MS);
    });
    expect(screen.getByTestId(INSPECT_HOVER)).toBeInTheDocument();
  });

  it("a step forward runs the animation queue; back and a jump remount without animating and the log reports no gap", async () => {
    const animated = firstAnimatedStep();
    await open();
    if (animated - 1 > 0) await jump(animated - 1);
    const board = shownBoard();
    expect(screen.queryByTestId("animation-queue")).toBeNull();

    // Forward: the same board takes the next view, and the queue plays its events.
    await press(replayTestid.forwardStep);
    expect(shownStep()).toBe(animated);
    expect(shownBoard()).toBe(board);
    expect(screen.getByTestId("animation-queue")).toBeInTheDocument();
    await advance(10_000);
    expect(screen.queryByTestId("animation-queue")).toBeNull();
    expect(gaps()).toBe(0);
    expect(document.querySelectorAll("[data-event]").length, "the log has lines to read").toBeGreaterThan(0);

    // Back: a fresh board on the earlier view, with nothing in flight.
    await press(replayTestid.backStep);
    expect(shownStep()).toBe(animated - 1);
    expect(shownBoard()).toBe(board + 1);
    expect(screen.queryByTestId("animation-queue")).toBeNull();
    expect(gaps()).toBe(0);
    expect(document.querySelectorAll("[data-event]").length).toBeGreaterThan(0);

    // A jump, forward or back, is the same.
    await jump(LAST);
    expect(shownBoard()).toBe(board + 2);
    expect(screen.queryByTestId("animation-queue")).toBeNull();
    expect(gaps()).toBe(0);
    await jump(animated);
    expect(shownBoard()).toBe(board + 3);
    expect(screen.queryByTestId("animation-queue")).toBeNull();
    expect(gaps()).toBe(0);

    // And a step forward after them carries on from the board they left.
    await press(replayTestid.forwardStep);
    expect(shownBoard()).toBe(board + 3);
  });

  it("a board handed an older view reads it as a gap, which is why a step back remounts it", () => {
    // Two steps whose event windows share nothing: the later is shown, then the earlier is handed over.
    let pair: [number, number] | null = null;
    for (let later = 1; later < steps.length && pair === null; later += 1) {
      const earlier = later - 1;
      const events = steps[earlier]?.view.events ?? [];
      if (events.length > 0 && newEventsSince(steps[later]?.view.events ?? [], events).length === events.length) pair = [earlier, later];
    }
    if (pair === null) throw new Error("no step of the folded game follows one whose events it shares none of");
    const [earlier, later] = pair;
    const board = (index: number) => (
      <CatalogContext.Provider value={lookup}>
        <Game view={steps[index]?.view as PlayerView} legal={[]} onAction={vi.fn()} replay />
      </CatalogContext.Provider>
    );
    const { rerender } = render(board(later));
    expect(gaps()).toBe(0);
    rerender(board(earlier));
    // The board takes the older view once its queue has played the events it takes for new.
    act(() => {
      vi.advanceTimersByTime(10_000);
    });
    expect(gaps()).toBe(1);
  });

  it("the final step shows both hands", async () => {
    await open();
    await press(replayTestid.end);
    expect(shownStep()).toBe(LAST);
    const view = steps[LAST]?.view;
    if (view === undefined) throw new Error("no last step");
    const theirs = cardsOf(view.opponent.hand);
    const yours = cardsOf(view.you.hand);
    expect(theirs.length, "the opponent's hand is shown, and holds cards").toBeGreaterThan(0);
    expect(yours.length, "the viewer holds cards at the end").toBeGreaterThan(0);

    expect(screen.getByTestId(testid.result)).toBeInTheDocument();
    for (const card of theirs) expect(screen.getByTestId(revealTestid.handCard(card.instanceId))).toBeInTheDocument();
    for (const card of yours) expect(screen.getByTestId(testid.handCard(card.instanceId))).toBeInTheDocument();
  });
});

/** `steps` with no modifier live on either side but at step `at`, where both carry R169's fixture badges. */
function withModifiersAt(at: number): ReplayStep[] {
  return steps.map((step, index) => ({
    ...step,
    view: {
      ...step.view,
      you: { ...step.view.you, modifiers: index === at ? yourModifiers : [] },
      opponent: { ...step.view.opponent, modifiers: index === at ? opponentModifiers : [] },
    },
  }));
}

/** `steps` with `pending` at step `at`, as that step's view carries it. */
function withPendingAt(at: number, pending: PendingView): ReplayStep[] {
  return steps.map((step, index) => (index === at ? { ...step, view: { ...step.view, pending } } : step));
}

/** The first step after step 0 that is no prompt, no mulligan and no end: a plain turn's step. */
function laterStep(): number {
  const found = steps.findIndex(
    (step, index) => index > 0 && step.view.pending === null && step.view.mulligan === undefined && step.view.result === null,
  );
  if (found < 0) throw new Error("no later step of the folded game is a plain one");
  return found;
}

describe("R2211000 a replay shows the seat's own choices, read-only", () => {
  const THREE: PendingOption[] = [
    { key: "mode:core-043", label: "Flood", defId: "core-043" },
    { key: "mode:core-055", label: "Archivist", defId: "core-055" },
    { key: "mode:core-066", label: "Lava Golem", defId: "core-066" },
  ];
  const modal = (): HTMLElement => screen.getByTestId("prompt-modal");
  const options = (): HTMLElement[] => [...document.querySelectorAll<HTMLElement>("[data-testid^='prompt-option-']")];

  it("R2211000 step 0's mulligan is drawn folded and read-only, and step, Play and jump work from it", async () => {
    const opening = steps[0]?.view.pending;
    if (opening === null || opening === undefined || !opening.forYou) throw new Error("the folded game does not open on the viewer's prompt");
    await open();
    expect(shownStep()).toBe(0);
    expect(modal()).toHaveAttribute("data-prompt-kind", "mulligan");
    expect(modal()).toHaveAttribute("data-read-only", "true");
    expect(modal()).toHaveAttribute("data-collapsed", "true");
    expect(options()).toEqual([]);

    fireEvent.click(screen.getByTestId("prompt-peek"));
    expect(options()).toHaveLength(opening.options.length);
    expect(options().length).toBeGreaterThan(0);
    const pressed = options().map((option) => option.getAttribute("aria-pressed"));
    for (const option of options()) {
      expect(option).toHaveAttribute("aria-disabled", "true");
      fireEvent.click(option);
    }
    await flush();
    expect(options().map((option) => option.getAttribute("aria-pressed"))).toEqual(pressed);
    expect(screen.queryByTestId("prompt-submit")).toBeNull();
    expect(shownStep()).toBe(0);

    await press(replayTestid.forwardStep);
    expect(shownStep()).toBe(1);
    await press(replayTestid.backStep);
    expect(shownStep()).toBe(0);
    expect(modal()).toHaveAttribute("data-collapsed", "true");

    await jump(LAST);
    expect(shownStep()).toBe(LAST);
    await press(replayTestid.start);
    expect(shownStep()).toBe(0);

    await press(replayTestid.play);
    await advance(REPLAY_PLAY_GAP_MS);
    expect(shownStep()).toBe(1);
  });

  it("R2211000 the seat's own Discover mid-game shows the cards it offered, inert", async () => {
    const at = laterStep();
    await open(fakeReplaySource(withPendingAt(at, pendingFor("discover", THREE))));
    await jump(at);
    expect(shownStep()).toBe(at);
    expect(modal()).toHaveAttribute("data-read-only", "true");
    expect(modal()).toHaveAttribute("data-collapsed", "true");

    fireEvent.click(screen.getByTestId("prompt-peek"));
    expect(options()).toHaveLength(3);
    for (const option of options()) {
      expect(option).toHaveAttribute("aria-disabled", "true");
      fireEvent.click(option);
    }
    await flush();
    expect(shownStep()).toBe(at);
    expect(screen.queryByTestId("prompt-submit")).toBeNull();
    expect(screen.queryByTestId("prompt-cancel")).toBeNull();
  });

  it("R2211000 a step where the other seat is choosing shows the waiting panel and no option", async () => {
    const at = laterStep();
    await open(fakeReplaySource(withPendingAt(at, waitingPending)));
    await jump(at);
    expect(modal()).toHaveAttribute("data-prompt-waiting", "waiting");
    expect(modal()).toHaveTextContent("Your opponent is choosing…");
    expect(options()).toEqual([]);
    expect(screen.queryByTestId("prompt-peek")).toBeNull();
  });
});

describe("R169 the modifier list in the replay", () => {
  it("R169 the rail's chip follows the step on the board and hides on a step with no modifier", async () => {
    await open(fakeReplaySource(withModifiersAt(1)));
    expect(shownStep()).toBe(0);
    expect(screen.queryByTestId(modifierListTestid.chip)).toBeNull();

    await press(replayTestid.forwardStep);
    expect(shownStep()).toBe(1);
    const chip = screen.getByTestId(modifierListTestid.chip);
    expect(chip).toHaveAttribute("data-count", String(yourModifiers.length + opponentModifiers.length));

    fireEvent.click(chip);
    const panel = screen.getByTestId(modifierListTestid.panel);
    for (const modifier of yourModifiers) {
      expect(panel.querySelector('[data-side="you"]')).toHaveTextContent(modifier.label);
    }
    for (const modifier of opponentModifiers) {
      expect(panel.querySelector('[data-side="opponent"]')).toHaveTextContent(modifier.label);
    }
    expect(Array.from(panel.querySelectorAll(".modifier-list__who"), (who) => who.textContent)).toEqual(["You", "Opponent"]);

    await press(replayTestid.backStep);
    expect(shownStep()).toBe(0);
    expect(screen.queryByTestId(modifierListTestid.chip)).toBeNull();
    expect(screen.queryByTestId(modifierListTestid.panel)).toBeNull();
  });
});

describe("leaving, paging and failing", () => {
  it("Back calls the leave handler", async () => {
    const onLeave = vi.fn();
    await open(fakeReplaySource(steps), onLeave);
    await press(replayTestid.back);
    expect(onLeave).toHaveBeenCalledTimes(1);
  });

  it("each failure shows its message", async () => {
    for (const reason of ["earlier_patch", "rules_changed", "expired"] as const) {
      const onLeave = vi.fn();
      await open(fakeReplaySource(steps, { failFrom: 0, failure: new ReplayFailure(reason) }), onLeave);
      expect(screen.getByTestId(replayTestid.failure), reason).toHaveTextContent(REPLAY_FAILURE_TEXT[reason]);
      expect(screen.queryByTestId("game")).toBeNull();
      await press(replayTestid.back);
      expect(onLeave, `${reason}: Back still leaves`).toHaveBeenCalledTimes(1);
      cleanup();
    }
    expect(REPLAY_FAILURE_TEXT).toEqual({
      earlier_patch: "This game was played on an earlier patch.",
      rules_changed: "This game was played under older rules.",
      expired: "This replay has expired.",
    });

    await open(fakeReplaySource(steps, { failFrom: 0, failure: new ReplayFailure("error", "Couldn't reach JackiOh.") }));
    expect(screen.getByTestId(replayTestid.failure)).toHaveTextContent("Couldn't reach JackiOh.");
    cleanup();

    // A page that fails later, reached with a jump: the board gives way to the reason.
    const second = 16;
    await open(fakeReplaySource(steps, { failFrom: second, failure: new ReplayFailure("expired") }));
    expect(shownStep()).toBe(0);
    await press(replayTestid.end);
    expect(screen.getByTestId(replayTestid.failure)).toHaveTextContent("This replay has expired.");
  });

  it("a failure while playing stops it and says why", async () => {
    writeSettings({ reduceMotion: true });
    await open(fakeReplaySource(steps, { failFrom: 16, failure: new ReplayFailure("rules_changed") }));
    await jump(15);
    await press(replayTestid.play);
    await advance(REPLAY_PLAY_GAP_MS);
    expect(screen.getByTestId(replayTestid.failure)).toHaveTextContent("This game was played under older rules.");
  });

  it("pages are fetched ahead of the shown step", async () => {
    const source = await open();
    expect(source.calls).toEqual([0, 16]);
    // Stepping through the first page asks for nothing more; reaching the second asks for the third.
    await jump(15);
    expect(source.calls).toEqual([0, 16]);
    await press(replayTestid.forwardStep);
    expect(shownStep()).toBe(16);
    expect(source.calls).toEqual([0, 16, 32]);
  });

  it("#1568 a page that could not be had offers Try again, which asks for it again and goes on", async () => {
    const healthy = fakeReplaySource(steps);
    let down = false;
    const flaky: FakeReplaySource = {
      total: healthy.total,
      calls: healthy.calls,
      page: (from, count) =>
        down ? Promise.reject(new ReplayFailure("error", "Couldn't reach JackiOh.")) : healthy.page(from, count),
    };
    await open(flaky);
    down = true;
    await press(replayTestid.end);

    expect(screen.getByTestId(replayTestid.failure)).toHaveTextContent("Couldn't reach JackiOh.");
    expect(screen.getByTestId(replayTestid.retry)).toBeInTheDocument();

    down = false;
    await press(replayTestid.retry);
    expect(screen.queryByTestId(replayTestid.failure)).toBeNull();
    expect(screen.queryByTestId(replayTestid.retry)).toBeNull();
    expect(shownStep()).toBe(LAST);
  });

  it("#1568 an expired replay offers no Try again", async () => {
    await open(fakeReplaySource(steps, { failFrom: 0, failure: new ReplayFailure("expired") }));

    expect(screen.getByTestId(replayTestid.failure)).toHaveTextContent("This replay has expired.");
    expect(screen.queryByTestId(replayTestid.retry)).toBeNull();
  });
});
