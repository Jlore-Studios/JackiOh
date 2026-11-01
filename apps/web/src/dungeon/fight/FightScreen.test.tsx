// B22–B26 (DR14 part E, #2706): a dungeon run's fight screen. The run's setup comes from a mocked
// `runFightSetup` (part C's binding), the worker is a scripted host as in `routes/practice.test.tsx`,
// and the run is a fake `Dungeon` whose `act` records what the screen sends it.

import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Mock } from "vitest";

import { AI_DIFFICULTY, HUMAN_HANDICAP } from "@jackioh/engine/config";
import type { RunAction, RunContent, RunFightSetup, RunState } from "@jackioh/run";

import { PRACTICE_PACING_FAST } from "../../practice/config.ts";
import type { PracticeHost } from "../../practice/host.ts";
import type { PracticeRequestBody, PracticeResponse, PracticeSnapshot, PracticeStartConfig } from "../../practice/protocol.ts";
import { baseView, emptySide } from "../../test/fixtures.ts";
import type { Dungeon } from "../store.ts";
import { FightScreen } from "./FightScreen.tsx";
import { fightTestid } from "./testids.ts";

const plan = vi.hoisted(() => ({ setup: null as unknown, asked: [] as { state: unknown; content: unknown }[] }));

vi.mock("../../wasm/index.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../wasm/index.ts")>();
  return {
    ...actual,
    runFightSetup: (state: unknown, content: unknown) => {
      plan.asked.push({ state, content });
      return plan.setup;
    },
  };
});

const SETUP: RunFightSetup = {
  seed: "RUNSED-2-1",
  humanDeck: ["core-001", "core-002"],
  humanHandicap: { ...HUMAN_HANDICAP, deckSize: 2, heroHealth: 17 },
  humanMarks: [{ index: 1, radiant: true, upgrades: 0 }],
  enemyDeck: ["core-003"],
  enemyHandicap: { ...AI_DIFFICULTY.medium, deckSize: 1, heroHealth: 30 },
  enemy: { id: "cave-troll", name: "Cave Troll", heroHealth: 30, deck: ["core-003"], artifacts: ["looming-presence"] },
};

const RUN = {
  seed: "RUNSED",
  hp: 17,
  deck: [],
  artifacts: [{ id: "lucky-coin", level: 2 }],
  blessings: ["blessing-of-dawn"],
  phase: { phase: "fight" },
} as unknown as RunState;

const CONTENT = {
  pool: [],
  starterAccess: [],
  enemies: [],
  passives: [
    { id: "lucky-coin", name: "Lucky Coin", kind: "economic", text: "Gain 5 more Coins after each fight.", tier: "bronze" },
    { id: "blessing-of-dawn", name: "Blessing of Dawn", kind: "blessing", text: "Heal 3 after each fight.", tier: null },
    { id: "looming-presence", name: "Looming Presence", kind: "enemy", text: "Starts with 1 more mana and 1 more card.", tier: null },
  ],
  events: [],
  curses: [],
} as unknown as RunContent;

const LOST: RunAction = { type: "fightOver", won: false, hp: 0, damageDealt: 4, healingDone: 0, cardsDrawn: 5, turns: 3 };

function snapshotFor(result: PracticeSnapshot["view"]["result"] = null): PracticeSnapshot {
  return {
    view: baseView({
      viewer: "p1",
      turn: 1,
      active: "p1",
      phase: "main",
      you: emptySide("p1"),
      opponent: emptySide("p2", { hand: { count: 4 } }),
      result,
    }),
    legal: result === null ? [{ type: "endTurn" }, { type: "concede" }] : [],
    aiToAct: false,
    error: null,
  };
}

type FightHost = {
  factory: Mock<() => PracticeHost>;
  requests: PracticeRequestBody[];
  starts(): PracticeStartConfig[];
  reports(): number;
};

/** The worker, scripted: a conceded game is lost; `runReport` answers each of `reports` in turn, then the action. */
function fightHost(reports: ("action" | "error")[] = []): FightHost {
  const requests: PracticeRequestBody[] = [];
  let id = 0;
  const host: PracticeHost = {
    request(body) {
      requests.push(body);
      id += 1;
      const mine = id;
      let response: PracticeResponse = { id: mine, type: "snapshot", snapshot: snapshotFor() };
      if (body.type === "start") response = { id: mine, type: "started", snapshot: snapshotFor(), defs: {}, aiSeat: "p2" };
      if (body.type === "act" && body.action.type === "concede") {
        response = { id: mine, type: "snapshot", snapshot: snapshotFor({ winner: "p2", reason: "concede" }) };
      }
      if (body.type === "runReport") {
        response =
          reports.shift() === "error"
            ? { id: mine, type: "runReport", error: "the fight does not fold" }
            : { id: mine, type: "runReport", action: LOST };
      }
      return Promise.resolve(response);
    },
    dispose() {},
  };
  return {
    factory: vi.fn(() => host),
    requests,
    starts: () => requests.flatMap((body) => (body.type === "start" ? [body.config] : [])),
    reports: () => requests.filter((body) => body.type === "runReport").length,
  };
}

function fakeDungeon(act: Mock<(action: RunAction) => string | null>, startedAt: number | null = Date.now() - 65_000): Dungeon {
  return {
    content: CONTENT,
    progress: { unlocks: {}, embers: 0 },
    run: RUN,
    startedAt,
    start: vi.fn(() => null),
    act,
    abandon: vi.fn(),
    buyLevel: vi.fn(() => null),
  } as unknown as Dungeon;
}

function renderFight(host: FightHost, dungeon: Dungeon): void {
  render(<FightScreen dungeon={dungeon} hostFactory={host.factory} pacing={PRACTICE_PACING_FAST} />);
}

/** Only the confirmation dialog's Concede sends the action. */
function concedeOnBoard(): void {
  fireEvent.click(screen.getByTestId("concede"));
  fireEvent.click(screen.getByTestId("concede-confirm"));
}

beforeEach(() => {
  plan.setup = SETUP;
  plan.asked = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(() => Promise.reject(new Error("a fight needs no server"))),
  );
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  window.localStorage.clear();
});

describe("B22 a fight node starts a practice game from runFightSetup", () => {
  it("B22 the game starts from the run's setup, the AI playing the enemy, whose name is shown", async () => {
    const host = fightHost();
    renderFight(host, fakeDungeon(vi.fn(() => null)));
    await screen.findByTestId(fightTestid.screen);

    expect(plan.asked[0]).toEqual({ state: RUN, content: CONTENT });
    expect(host.starts()).toEqual([expect.objectContaining({ seed: SETUP.seed, humanSeat: "p1", run: SETUP })]);
    expect(host.starts()[0]).not.toHaveProperty("customCards");
    expect(screen.getByTestId(fightTestid.enemy)).toHaveTextContent("Cave Troll");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Fight against Cave Troll");
  });

  it("B22 a run on no fight it can set up starts no game and says so", async () => {
    plan.setup = null;
    const host = fightHost();
    renderFight(host, fakeDungeon(vi.fn(() => null)));
    expect(await screen.findByTestId(fightTestid.noSetup)).toHaveAttribute("role", "alert");
    expect(host.factory).not.toHaveBeenCalled();
  });
});

describe("B23 both sides' relics beside their heroes", () => {
  it("B23 each side's tray sits beside its hero, and its button opens each one's name and text", async () => {
    renderFight(fightHost(), fakeDungeon(vi.fn(() => null)));
    await screen.findByTestId(fightTestid.screen);

    const own = screen.getByTestId(fightTestid.tray("own"));
    expect(own.closest(".hero-aside")?.parentElement).toHaveClass("seat-you");
    expect(within(own).getByTestId(fightTestid.trayIcon("own", "lucky-coin"))).toHaveAttribute("data-tier", "bronze");
    expect(within(own).getByTestId(fightTestid.trayIcon("own", "blessing-of-dawn"))).toHaveAttribute("data-tier", "none");
    const enemy = screen.getByTestId(fightTestid.tray("enemy"));
    expect(enemy.closest(".hero-aside")?.parentElement).toHaveClass("seat-opponent");

    fireEvent.click(screen.getByRole("button", { name: "Your relics and blessings: 2" }));
    const coin = screen.getByTestId(fightTestid.trayItem("own", "lucky-coin"));
    expect(coin).toHaveTextContent("Lucky Coin");
    expect(coin).toHaveTextContent("Level 2");
    expect(coin).toHaveTextContent("Gain 5 more Coins after each fight.");

    fireEvent.click(screen.getByRole("button", { name: "The enemy's relics: 1" }));
    expect(screen.getByTestId(fightTestid.trayItem("enemy", "looming-presence"))).toHaveTextContent("Looming Presence");
  });
});

describe("B24 the run's timer on screen", () => {
  it("B24 the board's rail shows the time since the run started", async () => {
    renderFight(fightHost(), fakeDungeon(vi.fn(() => null)));
    await screen.findByTestId(fightTestid.screen);
    const timer = screen.getByTestId(fightTestid.timer);
    expect(timer.closest(".board-status")).not.toBeNull();
    expect(timer).toHaveTextContent(/Run time 1:0\d/);
  });
});

describe("B25, B26 the fight's end goes to the run", () => {
  it("B25 when the game ends the page asks the worker for runReport, and Continue sends its FightOver to the run", async () => {
    const host = fightHost();
    const act = vi.fn<(action: RunAction) => string | null>(() => null);
    renderFight(host, fakeDungeon(act));
    await screen.findByTestId(fightTestid.screen);
    expect(host.reports()).toBe(0);

    concedeOnBoard();
    const proceed = await screen.findByTestId(fightTestid.continue);
    await waitFor(() => {
      expect(proceed).toBeEnabled();
    });
    expect(host.reports()).toBe(1);
    expect(act).not.toHaveBeenCalled();

    fireEvent.click(proceed);
    expect(act).toHaveBeenCalledTimes(1);
    expect(act).toHaveBeenCalledWith(LOST);
    expect(proceed).toBeDisabled();
  });

  it("B26 a lost fight sends the run a lost FightOver, and there is no Play again: one game, then the run", async () => {
    const host = fightHost();
    const act = vi.fn<(action: RunAction) => string | null>(() => null);
    renderFight(host, fakeDungeon(act));
    await screen.findByTestId(fightTestid.screen);
    concedeOnBoard();
    const proceed = await screen.findByTestId(fightTestid.continue);
    await waitFor(() => {
      expect(proceed).toBeEnabled();
    });
    expect(screen.queryByText(/Play again/)).toBeNull();
    expect(screen.queryByTestId("practice-play-again")).toBeNull();

    fireEvent.click(proceed);
    expect(act).toHaveBeenCalledWith(expect.objectContaining({ type: "fightOver", won: false }));
    expect(host.starts()).toHaveLength(1);
  });

  it("B25 a report the worker could not read shows why, and Retry asks again", async () => {
    const host = fightHost(["error"]);
    const act = vi.fn<(action: RunAction) => string | null>(() => null);
    renderFight(host, fakeDungeon(act));
    await screen.findByTestId(fightTestid.screen);
    concedeOnBoard();

    expect(await screen.findByTestId(fightTestid.reportError)).toHaveTextContent("the fight does not fold");
    expect(screen.queryByTestId(fightTestid.continue)).toBeNull();
    fireEvent.click(screen.getByTestId(fightTestid.reportRetry));

    const proceed = await screen.findByTestId(fightTestid.continue);
    await waitFor(() => {
      expect(proceed).toBeEnabled();
    });
    expect(host.reports()).toBe(2);
    fireEvent.click(proceed);
    expect(act).toHaveBeenCalledWith(LOST);
  });

  it("B25 the run's refusal of the report shows, and Continue may be pressed again", async () => {
    const act = vi.fn<(action: RunAction) => string | null>(() => "the run is not in a fight");
    renderFight(fightHost(), fakeDungeon(act));
    await screen.findByTestId(fightTestid.screen);
    concedeOnBoard();
    const proceed = await screen.findByTestId(fightTestid.continue);
    await waitFor(() => {
      expect(proceed).toBeEnabled();
    });

    fireEvent.click(proceed);
    expect(screen.getByTestId(fightTestid.actRefusal)).toHaveTextContent("the run is not in a fight");
    expect(proceed).toBeEnabled();
  });
});
