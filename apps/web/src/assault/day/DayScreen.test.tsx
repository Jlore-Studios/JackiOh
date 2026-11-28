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
    start: vi.fn(() => null),
    act: vi.fn(() => null),
    hold: vi.fn(),
    tryAgain: vi.fn(() => null),
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

it("the ladder marks every level beaten today with a Beaten badge", () => {
  const levels = [...DAY.levels, ...DAY.levels.slice(0, 1).map((level) => ({ ...level, level: 3, enemy: "Quarantine Wall" }))];
  render(<DayScreen assault={assault({ day: { ...DAY, levels }, record: { ...RECORD, best: 2 } })} saved={NO_SAVED} />);

  for (const beaten of [1, 2]) {
    const row = screen.getByTestId(assaultTestid.level(beaten));
    expect(within(row).getByText("Beaten")).toBeInTheDocument();
    expect(row).toHaveAttribute("data-beaten", "true");
  }
  const unbeaten = screen.getByTestId(assaultTestid.level(3));
  expect(within(unbeaten).queryByText("Beaten")).toBeNull();
  expect(unbeaten).not.toHaveAttribute("data-beaten");
});

// #5005 (docs/implicit-states.md §16, H2): the store refuses a Start sent past UTC midnight and moves the
// screen on to the new day; the screen's own picks follow it.
it("R4581000 a day screen moved on to a new day offers that day's deck and a start level its record holds", () => {
  bindingFixtures.assaultDayDeck.mockImplementation((date: string) => [`deck-of-${date}`]);
  const start = vi.fn(() => null);
  const { rerender } = render(<DayScreen assault={assault({ start })} saved={NO_SAVED} />);
  expect(screen.getByTestId(assaultTestid.startLevel(2))).toBeChecked();

  const next = "2026-11-24";
  const nextRecord = { date: next, best: 0, checkpoints: [], attempts: 0, repelledOn: null };
  const nextDay = assault({
    start,
    day: { ...DAY, date: next, seed: `assault-${next}` },
    record: nextRecord,
    records: { [DAY.date]: RECORD, [next]: nextRecord },
  });
  rerender(<DayScreen assault={nextDay} saved={NO_SAVED} />);
  fireEvent.click(screen.getByTestId(assaultTestid.start));

  expect(start).toHaveBeenLastCalledWith({ date: next, deck: [`deck-of-${next}`], source: "dayDeck", startLevel: 1 });
  bindingFixtures.assaultDayDeck.mockReset().mockReturnValue(["core-001", "core-002"]);
});
