// The Field Guide and the discovery coach on screen (FieldGuide.tsx, Coach.tsx, tracker.ts): R2205000
// and R2205002, on a fixture moment and a fake controller, so nothing here depends on the lessons.

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { ActionBody, GameEvent, PlayerView } from "@jackioh/shared";

import { PRACTICE_IDLE_STATE, type PracticeControllerState } from "../practice/controller.ts";
import type { PracticeSnapshot } from "../practice/protocol.ts";
import { baseView, card, emptySide, resetIds } from "../test/fixtures.ts";
import { Coach } from "./Coach.tsx";
import type { LessonScript } from "./coach.ts";
import { FIELD_GUIDE } from "./discoveries.ts";
import { FieldGuideButton } from "./FieldGuide.tsx";
import { __resetTutorialProgressForTests, markFound, readTutorialProgress } from "./progress.ts";
import { moment } from "./steps.ts";
import { tutorialTestid } from "./testids.ts";
import { createCoachTracker, type CoachSource, type CoachTracker } from "./tracker.ts";

const VANILLA = "core-008";

const SCRIPT: LessonScript = {
  lessonId: "fake",
  teaches: ["mana"],
  tips: [],
  steps: [
    moment({
      id: "afford",
      title: "What can you afford?",
      text: "You have 1 mana crystal. What can you afford?",
      discovers: ["mana"],
      hints: ["Your cards show their cost in the corner.", "Mr. Vanilla costs 1: drag it onto a lane."],
      hintAnchor: { kind: "handCard", defId: VANILLA },
    }),
  ],
};

type FakeSource = CoachSource & { push(snapshot: PracticeSnapshot): void };

function fakeSource(): FakeSource {
  let state: PracticeControllerState = { ...PRACTICE_IDLE_STATE, phase: "playing" };
  const listeners = new Set<() => void>();
  return {
    getState: () => state,
    subscribe(fn) {
      listeners.add(fn);
      return () => {
        listeners.delete(fn);
      };
    },
    setHold: vi.fn<(reason: string, held: boolean) => void>(),
    push(snapshot) {
      state = { ...state, snapshot };
      for (const fn of [...listeners]) fn();
    },
  };
}

function myTurn(events: GameEvent[] = []): PlayerView {
  return baseView({ turn: 3, active: "p1", events, you: emptySide("p1", { hand: [card({ instanceId: "h1", defId: VANILLA, cost: 1 })] }) });
}

function snap(view: PlayerView, legal: ActionBody[] = [{ type: "endTurn" }]): PracticeSnapshot {
  return { view, legal, aiToAct: false, error: null };
}

async function arrive(source: FakeSource, snapshot: PracticeSnapshot): Promise<void> {
  await act(async () => {
    source.push(snapshot);
    await Promise.resolve();
  });
}

let tracker: CoachTracker | null = null;

beforeEach(() => {
  resetIds();
  window.localStorage.clear();
  __resetTutorialProgressForTests();
});

afterEach(() => {
  cleanup();
  tracker?.dispose();
  tracker = null;
  window.localStorage.clear();
  __resetTutorialProgressForTests();
});

describe("R2205000 the discovery coach on screen", () => {
  it("R2205000 a moment's bubble asks, offers Hint as a focusable button, and each press shows the next hint", async () => {
    const source = fakeSource();
    source.push(snap(myTurn()));
    tracker = createCoachTracker(source, SCRIPT);
    render(<Coach tracker={tracker} boardRoot={null} />);
    await act(async () => {
      await Promise.resolve();
    });
    const bubble = screen.getByTestId(tutorialTestid.coach);
    expect(bubble).toHaveAttribute("data-coach-mode", "step");
    expect(bubble).toHaveTextContent("You have 1 mana crystal. What can you afford?");
    expect(within(bubble).queryByText("Got it"), "a moment is moved on by finding it, not by Got it").toBeNull();

    const hint = screen.getByTestId(tutorialTestid.coachHint);
    expect(hint.tagName).toBe("BUTTON");
    fireEvent.click(hint);
    expect(bubble).toHaveTextContent("Your cards show their cost in the corner.");
    fireEvent.click(screen.getByTestId(tutorialTestid.coachHint));
    expect(bubble).toHaveTextContent("Mr. Vanilla costs 1: drag it onto a lane.");
    expect(screen.queryByTestId(tutorialTestid.coachHint), "no hint is left to ask for").toBeNull();
  });

  it("R2205000 R2205002 a discovery shows its rule with Got it, and the tracker hands it to the device's Field Guide", async () => {
    const source = fakeSource();
    source.push(snap(myTurn()));
    const onFound = vi.fn<(ids: readonly string[]) => void>();
    tracker = createCoachTracker(source, SCRIPT, { onFound });
    render(<Coach tracker={tracker} boardRoot={null} />);
    await arrive(source, snap(myTurn([{ type: "manaSpent", player: "p1", amount: 1, for: "play" } as GameEvent])));

    const bubble = screen.getByTestId(tutorialTestid.coach);
    expect(bubble).toHaveAttribute("data-coach-mode", "discovery");
    expect(bubble).toHaveTextContent("Discovery");
    expect(bubble).toHaveTextContent("You found Mana");
    expect(onFound).toHaveBeenCalledWith(["mana"]);
    fireEvent.click(screen.getByTestId(tutorialTestid.coachAck));
    expect(screen.queryByTestId(tutorialTestid.coachAck)).toBeNull();
  });
});

describe("R2205002 the Field Guide", () => {
  it("R2205002 opens as a dialog that takes focus, lists every entry, and closes on Escape with focus back on its button", () => {
    markFound(["mana"]);
    render(<FieldGuideButton testid={tutorialTestid.fieldGuidePath} />);
    const button = screen.getByTestId(tutorialTestid.fieldGuidePath);
    expect(button).toHaveAccessibleName(`Field Guide 1/${String(FIELD_GUIDE.length)}`);
    button.focus();
    fireEvent.click(button);

    const dialog = screen.getByRole("dialog", { name: "Field Guide" });
    expect(dialog).toHaveAttribute("aria-modal", "true");
    expect(screen.getByTestId(tutorialTestid.fieldGuideClose)).toHaveFocus();
    expect(within(dialog).getAllByRole("listitem")).toHaveLength(FIELD_GUIDE.length);
    const mana = screen.getByTestId(tutorialTestid.fieldGuideEntry("mana"));
    expect(mana).toHaveAttribute("data-found", "true");
    expect(mana).toHaveTextContent("each card costs mana");
    const cry = screen.getByTestId(tutorialTestid.fieldGuideEntry("cry"));
    expect(cry).toHaveAttribute("data-found", "false");
    expect(cry, "a mechanic not met keeps its rule hidden").not.toHaveTextContent("it happens when you play the unit");

    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(button).toHaveFocus();
  });

  it("R2205002 Close closes it too, and what it lists follows the device's progress", () => {
    render(<FieldGuideButton testid={tutorialTestid.fieldGuideHud} compact />);
    act(() => {
      markFound(["cry"]);
    });
    expect(readTutorialProgress().found).toEqual(["cry"]);
    fireEvent.click(screen.getByTestId(tutorialTestid.fieldGuideHud));
    expect(screen.getByTestId(tutorialTestid.fieldGuideEntry("cry"))).toHaveAttribute("data-found", "true");
    fireEvent.click(screen.getByTestId(tutorialTestid.fieldGuideClose));
    expect(screen.queryByRole("dialog")).toBeNull();
  });
});
