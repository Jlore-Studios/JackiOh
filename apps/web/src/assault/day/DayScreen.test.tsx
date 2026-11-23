import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import type { SavedDecks } from "../../practice/PracticeSetup.tsx";
import type { Assault } from "../store.ts";
import { assaultTestid } from "../testids.ts";
import { CONSTANTS, CONTENT, DAY, RECORD, bindingFixtures } from "../test/fixtures.ts";
import { DayScreen } from "./DayScreen.tsx";

vi.mock("../../wasm/index.ts", async (importOriginal) => ({
  ...(await importOriginal<Record<string, unknown>>()),
  ...(await import("../test/fixtures.ts")).bindingFixtures,
}));

const NO_SAVED: SavedDecks = { kind: "none" };

function assault(over: Partial<Assault> = {}): Assault {
  return {
    constants: CONSTANTS,
    content: CONTENT,
    day: DAY,
    state: null,
    record: RECORD,
    records: { [DAY.date]: RECORD },
    fight: null,
    start: vi.fn(() => null),
    act: vi.fn(() => null),
    dismiss: vi.fn(),
    ...over,
  };
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(new Date("2026-11-23T12:00:00Z"));
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  bindingFixtures.assaultDayDeck.mockClear();
  vi.useRealTimers();
});

it("the ladder names every level, its enemy and its commander marks", () => {
  render(<DayScreen assault={assault()} saved={NO_SAVED} />);

  expect(screen.getByTestId(assaultTestid.level(1))).toHaveTextContent("Packet Filter");
  expect(screen.getByText("Next Assault in 12:00:00")).toBeInTheDocument();
  const commander = screen.getByTestId(assaultTestid.level(2));
  expect(commander).toHaveTextContent("Bastion Node");
  expect(commander).toHaveTextContent("Commander");
  expect(commander).toHaveTextContent("Checkpoint");
});

it("the record shows best and checkpoints, and Start offers level 1 and the highest checkpoint", () => {
  const start = vi.fn(() => null);
  render(<DayScreen assault={assault({ start })} saved={NO_SAVED} />);

  expect(screen.getByTestId(assaultTestid.record)).toHaveTextContent("Best: level 1");
  expect(screen.getByTestId(assaultTestid.record)).toHaveTextContent("Checkpoints: 2");
  expect(screen.getByTestId(assaultTestid.startLevel(1))).toBeInTheDocument();
  expect(screen.getByTestId(assaultTestid.startLevel(2))).toBeChecked();
  expect(screen.getByTestId(assaultTestid.start)).toHaveTextContent("Start at level 2");

  fireEvent.click(screen.getByTestId(assaultTestid.start));
  expect(start).toHaveBeenCalledWith(expect.objectContaining({ date: DAY.date, startLevel: 2, source: "dayDeck" }));
  expect(within(screen.getByTestId(assaultTestid.history)).getByText(DAY.date, { exact: false })).toBeInTheDocument();
});
