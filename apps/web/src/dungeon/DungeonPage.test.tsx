// B19 (DR14 part D, #2705): the Dungeon Run page switches between the run's screens. The store is the
// real one over the fixtures' scripted run bindings; the fight (part E) and the map (part F) screens are
// stand-ins that show which one the page drew and hand the page's `Dungeon` to the test.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { DUNGEON_RUN_KEY } from "./config.ts";
import DungeonPage from "./DungeonPage.tsx";
import { __resetDungeonForTests, type Dungeon } from "./store.ts";
import { dungeonTestid } from "./testids.ts";
import { STARTERS, resetBindings, setupOf, unlocks } from "./test/fixtures.ts";

vi.mock("../wasm/index.ts", async (importOriginal) => ({
  ...(await importOriginal<Record<string, unknown>>()),
  ...(await import("./test/fixtures.ts")).wasmBindings,
}));
vi.mock("../net/gate.ts", async (importOriginal) => ({
  ...(await importOriginal<Record<string, unknown>>()),
  useAccount: () => ({ kind: "anonymous" }),
}));

/** The `Dungeon` the page last handed a screen. */
const shown = vi.hoisted(() => ({ dungeon: null as unknown }));
const dungeonShown = (): Dungeon => shown.dungeon as Dungeon;

vi.mock("./fight/FightScreen.tsx", () => ({
  FightScreen: ({ dungeon }: { dungeon: Dungeon }): ReactElement => {
    shown.dungeon = dungeon;
    return <p data-testid="stand-in-fight">{dungeon.run?.phase.phase}</p>;
  },
}));
vi.mock("./map/RunScreen.tsx", () => ({
  RunScreen: ({ dungeon }: { dungeon: Dungeon }): ReactElement => {
    shown.dungeon = dungeon;
    return <p data-testid="stand-in-run">{dungeon.run?.phase.phase}</p>;
  },
}));

const byId = (id: string): HTMLElement => screen.getByTestId(id);

/** The run kept on the device as a page left it: started, and sent these actions. */
function keepRun(log: readonly object[]): void {
  window.localStorage.setItem(
    DUNGEON_RUN_KEY,
    JSON.stringify({ v: 1, file: { setup: setupOf(), unlocks: unlocks(), log }, customCards: [], startedAt: Date.now(), daily: false }),
  );
}

beforeEach(() => {
  window.localStorage.clear();
  __resetDungeonForTests();
  resetBindings();
  shown.dungeon = null;
});

afterEach(() => {
  cleanup();
  __resetDungeonForTests();
});

describe("B19 the page's screens", () => {
  it("B19 a folded run waits for Continue then follows fight versus other phases", () => {
    keepRun([{ type: "go", col: 0 }]);
    render(<DungeonPage />);

    // The run kept on the device was folded back (in a fight), and the page offers it, not drops into it.
    expect(byId(dungeonTestid.screen)).toBeInTheDocument();
    expect(byId(dungeonTestid.start)).toBeInTheDocument();
    expect(byId(dungeonTestid.saved)).toBeInTheDocument();
    expect(screen.queryByTestId("stand-in-fight")).toBeNull();
    expect(screen.queryByTestId("stand-in-run")).toBeNull();

    // Continue enters it: a fight is the fight screen.
    fireEvent.click(byId(dungeonTestid.continueRun));
    expect(byId("stand-in-fight")).toHaveTextContent("fight");
    expect(screen.queryByTestId(dungeonTestid.start)).toBeNull();

    // The fight is won: the run is on the map, which is the run screen, with the same run.
    act(() => {
      dungeonShown().act({ type: "fightOver", won: true, hp: 20, damageDealt: 5, healingDone: 0, cardsDrawn: 3, turns: 4 });
    });
    expect(screen.queryByTestId("stand-in-fight")).toBeNull();
    expect(byId("stand-in-run")).toHaveTextContent("map");
    expect(dungeonShown().run?.row).toBe(0);

    // A lost fight ends the run, and the run-over screen is the run screen's: it is not left on its own.
    act(() => {
      dungeonShown().act({ type: "go", col: 0 });
    });
    expect(byId("stand-in-fight")).toBeInTheDocument();
    act(() => {
      dungeonShown().act({ type: "fightOver", won: false, hp: 0, damageDealt: 1, healingDone: 0, cardsDrawn: 2, turns: 3 });
    });
    expect(byId("stand-in-run")).toHaveTextContent("dead");

    // Leaving it returns to the start screen, with the setup offered.
    act(() => {
      dungeonShown().abandon();
    });
    expect(byId(dungeonTestid.start)).toBeInTheDocument();
    expect(byId(dungeonTestid.startRun)).toBeInTheDocument();
    expect(screen.queryByTestId("stand-in-run")).toBeNull();
  });

  it("B19 a folded run can be given up on the start screen", () => {
    keepRun([]);
    render(<DungeonPage />);

    fireEvent.click(byId(dungeonTestid.abandonRun));

    expect(screen.queryByTestId(dungeonTestid.saved)).toBeNull();
    expect(byId(dungeonTestid.startRun)).toBeInTheDocument();
    expect(window.localStorage.getItem(DUNGEON_RUN_KEY)).toBeNull();
  });

  it("B19 a run started on the start screen is entered at once", () => {
    render(<DungeonPage />);
    expect(byId(dungeonTestid.start)).toBeInTheDocument();
    expect(screen.queryByTestId(dungeonTestid.saved)).toBeNull();

    for (const id of STARTERS) fireEvent.click(byId(dungeonTestid.card(id)));
    fireEvent.click(byId(dungeonTestid.startRun));

    expect(screen.queryByTestId(dungeonTestid.start)).toBeNull();
    expect(byId("stand-in-run")).toHaveTextContent("map");
    expect(window.localStorage.getItem(DUNGEON_RUN_KEY)).not.toBeNull();
  });

  it("B19 a finished run still on the screen is not left for the start screen when the page opens again", () => {
    keepRun([]);
    const first = render(<DungeonPage />);
    fireEvent.click(byId(dungeonTestid.continueRun));
    act(() => {
      dungeonShown().act({ type: "abandon" });
    });
    expect(byId("stand-in-run")).toHaveTextContent("dead");

    // The player leaves the page and comes back: the run-over screen is still what they find.
    first.unmount();
    render(<DungeonPage />);
    expect(byId("stand-in-run")).toHaveTextContent("dead");
    expect(screen.queryByTestId(dungeonTestid.start)).toBeNull();
  });
});
