// R4582000–R4582002 (issue #4582): an Infinite Assault fight's screen. The Assault's setup comes
// from a mocked `assaultFightSetup` (part B's binding), the worker is a scripted host as in
// `routes/practice.test.tsx`, and the attempt's `onAction` records what the screen sends it.

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Mock } from "vitest";

import { AI_DIFFICULTY, HUMAN_HANDICAP } from "@jackioh/engine/config";
import type { AssaultAction, AssaultDay, AssaultFightSetup, AssaultLevel, AssaultState } from "@jackioh/run";
import type { ModifierView } from "@jackioh/shared";

import { paths } from "../../net/navigate.ts";
import { at } from "../../test/stubServer.ts";

import { PRACTICE_PACING_FAST, type PracticePacing } from "../../practice/config.ts";
import type { PracticeHost } from "../../practice/host.ts";
import type { PracticeRequestBody, PracticeResponse, PracticeSnapshot, PracticeStartConfig } from "../../practice/protocol.ts";
import { CONCEDE_BODY } from "../../game/ConfirmConcede.tsx";
import { modifierListTestid } from "../../game/ModifierList.tsx";
import { testid } from "../../game/contract.ts";
import { baseView, emptySide, opponentModifiers } from "../../test/fixtures.ts";
import { AssaultFightScreen } from "./FightScreen.tsx";
import { assaultFightTestid } from "./testids.ts";

const plan = vi.hoisted(() => ({ setup: null as unknown, asked: [] as { state: unknown; day: unknown }[] }));

vi.mock("../../wasm/index.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../wasm/index.ts")>();
  return {
    ...actual,
    assaultFightSetup: (state: unknown, day: unknown) => {
      plan.asked.push({ state, day });
      return plan.setup;
    },
  };
});

const LEVEL: AssaultLevel = {
  level: 3,
  enemy: "Rogue Model",
  doctrine: null,
  commander: null,
  escalations: [],
  upgrades: [
    { kind: "heroArmor", stacks: 2 },
    { kind: "radiant", stacks: 1 },
  ],
  rarities: [20, 5, 2, 1, 0],
  deck: ["core-003"],
  marks: [],
  handicap: { deckSize: 1, manaBonus: 0, manaCap: 4, extraOpeningCards: 0, extraDrawsPerTurn: 0, heroHealth: 30 },
};

const SETUP: AssaultFightSetup = {
  seed: "ASSAULTSED-1-3",
  humanDeck: ["core-001", "core-002"],
  humanHandicap: { ...HUMAN_HANDICAP, deckSize: 2, heroHealth: 17 },
  enemyDeck: ["core-003"],
  enemyHandicap: { ...AI_DIFFICULTY.medium, deckSize: 1, heroHealth: 30 },
  enemyMarks: [{ index: 1, radiant: true, upgrades: 0 }],
  level: LEVEL,
};

const STATE = {
  setup: { date: "2026-11-23", deck: [], source: "saved", startLevel: 1, attempt: 1 },
  level: 3,
  phase: "fight",
  beaten: [1, 2],
  turns: 0,
} as unknown as AssaultState;

const DAY = {
  date: "2026-11-23",
  seed: "assault-2026-11-23",
  levels: [],
} as unknown as AssaultDay;

const WON: AssaultAction = { kind: "fightOver", won: true, turns: 7 };
const LOST: AssaultAction = { kind: "fightOver", won: false, turns: 3 };

/** The level's enemy is its commander: its name is on its hero. */
const COMMANDER_SETUP: AssaultFightSetup = {
  ...SETUP,
  level: { ...LEVEL, enemy: "Commander Vex", commander: "Commander Vex" },
};

/** The modifiers each side's view lists (R169). */
type SideModifiers = { you?: ModifierView[]; opponent?: ModifierView[] };

function snapshotFor(
  result: PracticeSnapshot["view"]["result"] = null,
  aiToAct = false,
  modifiers: SideModifiers = {},
  lethal = false,
): PracticeSnapshot {
  return {
    view: baseView({
      viewer: "p1",
      turn: 1,
      active: aiToAct ? "p2" : "p1",
      phase: "main",
      you: emptySide("p1", { modifiers: modifiers.you ?? [] }),
      opponent: emptySide("p2", { hand: { count: 4 }, modifiers: modifiers.opponent ?? [] }),
      result,
    }),
    legal: result === null && !aiToAct ? [{ type: "endTurn" }, { type: "concede" }] : [],
    aiToAct,
    error: null,
    ...(lethal ? { lethal: true as const } : {}),
  };
}

type AssaultFightHost = {
  factory: Mock<() => PracticeHost>;
  requests: PracticeRequestBody[];
  starts(): PracticeStartConfig[];
  resumes(): PracticeStartConfig[];
  reports(): number;
};

/**
 * The worker, scripted: a conceded game is lost, an `endTurn` act ends it won;
 * `assaultReport` answers each of `reports` in turn with an error, then the action matching how the
 * game ended. `resume` answers `started` only when asked, else `failed`; the first `startFails`
 * starts fail. Every snapshot lists `modifiers` on its sides' views (R169), and `lethal` lights
 * the lethal hint on each (R2102000).
 */
function fightHost(
  reports: ("action" | "error")[] = [],
  resume: "started" | "failed" = "failed",
  startFails = 0,
  modifiers: SideModifiers = {},
  lethal = false,
): AssaultFightHost {
  const requests: PracticeRequestBody[] = [];
  let id = 0;
  let failures = startFails;
  let ended: "won" | "lost" | null = null;
  const host: PracticeHost = {
    request(body) {
      requests.push(body);
      id += 1;
      const mine = id;
      let response: PracticeResponse = { id: mine, type: "snapshot", snapshot: snapshotFor(null, false, modifiers, lethal) };
      if (body.type === "resume") {
        response =
          resume === "started"
            ? { id: mine, type: "started", snapshot: snapshotFor(null, false, modifiers, lethal), defs: {}, aiSeat: "p2" }
            : { id: mine, type: "failed", message: "no practice game is saved on this device" };
      }
      if (body.type === "start") {
        if (failures > 0) {
          failures -= 1;
          response = { id: mine, type: "failed", message: "the fight could not be dealt" };
        } else {
          response = { id: mine, type: "started", snapshot: snapshotFor(null, false, modifiers, lethal), defs: {}, aiSeat: "p2" };
        }
      }
      if (body.type === "act" && body.action.type === "concede") {
        ended = "lost";
        response = { id: mine, type: "snapshot", snapshot: snapshotFor({ winner: "p2", reason: "concede" }, false, modifiers, lethal) };
      }
      if (body.type === "act" && body.action.type === "endTurn") {
        ended = "won";
        response = { id: mine, type: "snapshot", snapshot: snapshotFor({ winner: "p1", reason: "hero-death" }, false, modifiers, lethal) };
      }
      if (body.type === "assaultReport") {
        response =
          reports.shift() === "error"
            ? { id: mine, type: "assaultReport", error: "the fight does not fold" }
            : { id: mine, type: "assaultReport", action: ended === "won" ? WON : LOST };
      }
      return Promise.resolve(response);
    },
    dispose() {},
  };
  return {
    factory: vi.fn(() => host),
    requests,
    starts: () => requests.flatMap((body) => (body.type === "start" ? [body.config] : [])),
    resumes: () => requests.flatMap((body) => (body.type === "resume" ? [body.config] : [])),
    reports: () => requests.filter((body) => body.type === "assaultReport").length,
  };
}

function renderFight(
  host: AssaultFightHost,
  onAction: Mock<(action: AssaultAction) => void> = vi.fn(),
  onLeave?: () => void,
  pacing: PracticePacing = PRACTICE_PACING_FAST,
): void {
  render(<AssaultFightScreen state={STATE} day={DAY} onAction={onAction} onLeave={onLeave} hostFactory={host.factory} pacing={pacing} />);
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
  document.body.removeAttribute("data-speaking");
  document.body.removeAttribute("data-showcase");
  vi.useRealTimers();
  vi.unstubAllGlobals();
  window.localStorage.clear();
});

describe("R4582000 a level's fight starts a practice game from assaultFightSetup", () => {
  it("R4582000 the game starts from assaultFightSetup's setup, the AI at p2, the enemy's name and level on the rail", async () => {
    const host = fightHost();
    renderFight(host);
    await screen.findByTestId(assaultFightTestid.screen);

    expect(plan.asked[0]).toEqual({ state: STATE, day: DAY });
    expect(host.resumes()).toEqual([expect.objectContaining({ seed: SETUP.seed, humanSeat: "p1", assault: SETUP })]);
    expect(host.starts()).toEqual([expect.objectContaining({ seed: SETUP.seed, humanSeat: "p1", assault: SETUP })]);
    expect(screen.getByTestId(assaultFightTestid.enemy)).toHaveTextContent("Rogue Model");
    expect(screen.getByTestId(assaultFightTestid.level)).toHaveTextContent("Level 3");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Fight against Rogue Model");
  });

  it("R4582000 a state on no fight starts no game and says so", async () => {
    plan.setup = null;
    const host = fightHost();
    renderFight(host);
    expect(await screen.findByTestId(assaultFightTestid.noSetup)).toHaveAttribute("role", "alert");
    expect(host.factory).not.toHaveBeenCalled();
  });
});

describe("R4582000 the fight's end goes to the attempt", () => {
  it("R4582000 a lost fight's report reaches onAction on Continue, once", async () => {
    const host = fightHost();
    const onAction = vi.fn<(action: AssaultAction) => void>();
    renderFight(host, onAction);
    await screen.findByTestId(assaultFightTestid.screen);
    expect(host.reports()).toBe(0);

    concedeOnBoard();
    const proceed = await screen.findByTestId(assaultFightTestid.continue);
    await waitFor(() => {
      expect(proceed).toBeEnabled();
    });
    expect(host.reports()).toBe(1);
    expect(onAction).not.toHaveBeenCalled();

    fireEvent.click(proceed);
    expect(onAction).toHaveBeenCalledTimes(1);
    expect(onAction).toHaveBeenCalledWith(LOST);
    expect(proceed).toBeDisabled();
  });

  it("R4582000 a won fight's report reaches onAction on Continue", async () => {
    const host = fightHost();
    const onAction = vi.fn<(action: AssaultAction) => void>();
    renderFight(host, onAction);
    await screen.findByTestId(assaultFightTestid.screen);

    fireEvent.click(screen.getByTestId("end-turn"));
    const proceed = await screen.findByTestId(assaultFightTestid.continue);
    await waitFor(() => {
      expect(proceed).toBeEnabled();
    });

    fireEvent.click(proceed);
    expect(onAction).toHaveBeenCalledTimes(1);
    expect(onAction).toHaveBeenCalledWith(WON);
  });

  it("R4582000 a report the worker could not read shows why, and Retry asks again", async () => {
    const host = fightHost(["error"]);
    const onAction = vi.fn<(action: AssaultAction) => void>();
    renderFight(host, onAction);
    await screen.findByTestId(assaultFightTestid.screen);
    concedeOnBoard();

    expect(await screen.findByTestId(assaultFightTestid.reportError)).toHaveTextContent("the fight does not fold");
    expect(screen.queryByTestId(assaultFightTestid.continue)).toBeNull();
    fireEvent.click(screen.getByTestId(assaultFightTestid.reportRetry));

    const proceed = await screen.findByTestId(assaultFightTestid.continue);
    await waitFor(() => {
      expect(proceed).toBeEnabled();
    });
    expect(host.reports()).toBe(2);
    fireEvent.click(proceed);
    expect(onAction).toHaveBeenCalledWith(LOST);
  });

  it("R4582000 the board draws no Offer draw, and the lethal hint shows when the worker finds one", async () => {
    renderFight(fightHost([], "failed", 0, {}, true));
    await screen.findByTestId(assaultFightTestid.screen);

    expect(screen.queryByTestId("offer-draw")).toBeNull();
    expect(screen.getByTestId("concede")).toBeInTheDocument();
    expect(await screen.findByTestId("lethal-hint")).toHaveTextContent("You have lethal");
  });

  it("R4582000 Concede says the attempt ends", async () => {
    const host = fightHost();
    renderFight(host);
    await screen.findByTestId(assaultFightTestid.screen);

    // Asking sends nothing; the question says what a lost fight costs, not the usual game-only body.
    fireEvent.click(screen.getByTestId("concede"));
    const dialog = screen.getByTestId("concede-dialog");
    expect(dialog).toHaveAccessibleDescription("Your opponent wins, and this attempt ends here.");
    expect(dialog).not.toHaveTextContent(CONCEDE_BODY);
    expect(host.reports()).toBe(0);
  });
});

describe("R4582001 a kept fight is picked up, and Menu asks before leaving", () => {
  it("R4582001 a kept fight is picked up on its turn, and no fresh game is dealt", async () => {
    const host = fightHost([], "started");
    renderFight(host);
    await screen.findByTestId(assaultFightTestid.screen);

    expect(host.resumes()).toEqual([expect.objectContaining({ seed: SETUP.seed, humanSeat: "p1", assault: SETUP })]);
    expect(host.starts(), "the kept fight is picked up, not dealt fresh").toHaveLength(0);
  });

  it("R4582001 a fight no save folds for is dealt fresh from its seed", async () => {
    const host = fightHost([], "failed");
    renderFight(host);
    await screen.findByTestId(assaultFightTestid.screen);

    expect(host.resumes()).toHaveLength(1);
    expect(host.starts()).toEqual([expect.objectContaining({ seed: SETUP.seed, humanSeat: "p1", assault: SETUP })]);
  });

  it("R4582001 Try again deals the fight fresh, never resuming", async () => {
    const host = fightHost([], "failed", 1);
    renderFight(host);

    // The kept fight misses and the first deal fails: the stopped screen, with no game running.
    expect(await screen.findByTestId(assaultFightTestid.failed)).toBeInTheDocument();
    expect(host.resumes()).toHaveLength(1);
    expect(host.starts()).toHaveLength(1);

    fireEvent.click(screen.getByTestId(assaultFightTestid.retry));
    await screen.findByTestId(assaultFightTestid.screen);
    expect(host.starts(), "Try again deals the same fight fresh").toHaveLength(2);
    expect(host.resumes(), "Try again never resumes").toHaveLength(1);
  });

  it("R4582001 mid-fight Menu asks first: Stay keeps the fight, Leave goes back to the day screen", async () => {
    const host = fightHost();
    const onLeave = vi.fn();
    renderFight(host, vi.fn(), onLeave);
    await screen.findByTestId(assaultFightTestid.screen);

    fireEvent.click(screen.getByTestId(assaultFightTestid.menu));
    const dialog = await screen.findByTestId(assaultFightTestid.leaveDialog);
    expect(dialog).toHaveAttribute("role", "alertdialog");
    expect(dialog).toHaveTextContent("It waits here and picks up where you left it.");
    expect(screen.getByTestId(assaultFightTestid.leaveStay)).toHaveFocus();

    // Stay keeps the fight: still no report asked for, and the same game carries on.
    fireEvent.click(screen.getByTestId(assaultFightTestid.leaveStay));
    expect(screen.queryByTestId(assaultFightTestid.leaveDialog)).toBeNull();
    expect(screen.getByTestId(assaultFightTestid.screen)).toBeInTheDocument();
    expect(onLeave).not.toHaveBeenCalled();
    expect(host.reports()).toBe(0);
    expect(host.starts()).toHaveLength(1);

    fireEvent.click(screen.getByTestId(assaultFightTestid.menu));
    fireEvent.click(await screen.findByTestId(assaultFightTestid.leaveConfirm));
    expect(onLeave).toHaveBeenCalledTimes(1);
  });

  it("R4582001 without a way back, Menu's Leave goes to the Play menu", async () => {
    at("/assault");
    const host = fightHost();
    renderFight(host);
    await screen.findByTestId(assaultFightTestid.screen);

    fireEvent.click(screen.getByTestId(assaultFightTestid.menu));
    const leave = await screen.findByTestId(assaultFightTestid.leaveConfirm);
    expect(leave).toHaveClass("button-danger");
    fireEvent.click(leave);
    expect(window.location.pathname).toBe(paths.play);
    expect(screen.getByTestId(assaultFightTestid.screen)).toBeInTheDocument();
    expect(host.reports()).toBe(0);
    expect(host.starts()).toHaveLength(1);
    at("/");
  });

  it("R4582001 once the fight has its result, Menu sends the report and leaves without asking", async () => {
    const host = fightHost();
    const onAction = vi.fn<(action: AssaultAction) => void>();
    const onLeave = vi.fn();
    renderFight(host, onAction, onLeave);
    await screen.findByTestId(assaultFightTestid.screen);
    concedeOnBoard();
    const proceed = await screen.findByTestId(assaultFightTestid.continue);
    await waitFor(() => {
      expect(proceed).toBeEnabled();
    });

    fireEvent.click(screen.getByTestId(assaultFightTestid.menu));
    expect(onAction).toHaveBeenCalledWith(LOST);
    expect(onLeave).toHaveBeenCalledTimes(1);
    expect(screen.queryByTestId(assaultFightTestid.leaveDialog)).toBeNull();
  });
});

describe("R4582002 the fight shows what the Assault holds", () => {
  it("R4582002 the enemy's upgrades are listed beside its hero, each by name, stacks and effect", async () => {
    renderFight(fightHost());
    await screen.findByTestId(assaultFightTestid.screen);

    fireEvent.click(screen.getByRole("button", { name: "Boosts: 2" }));
    const armor = screen.getByTestId(assaultFightTestid.upgrade("heroArmor"));
    expect(armor).toHaveTextContent("Hardened Shell");
    expect(armor).toHaveTextContent("×2");
    expect(armor).toHaveTextContent("Its hero starts with Armor.");
    const radiant = screen.getByTestId(assaultFightTestid.upgrade("radiant"));
    expect(radiant).toHaveTextContent("Fine-Tuned");
    expect(radiant).toHaveTextContent("More of its cards start Radiant.");
  });

  it("R4582002 a level with no upgrade shows no tray", async () => {
    plan.setup = { ...SETUP, level: { ...LEVEL, upgrades: [] } };
    renderFight(fightHost());
    await screen.findByTestId(assaultFightTestid.screen);

    expect(screen.queryByTestId(assaultFightTestid.upgrades)).toBeNull();
  });

  it("R4582002 a Commander's name is on its hero; a normal enemy's hero keeps the default label", async () => {
    plan.setup = COMMANDER_SETUP;
    renderFight(fightHost());
    await screen.findByTestId(assaultFightTestid.screen);
    expect(screen.getByTestId(testid.hero("opponent")).querySelector(".hero-seat")).toHaveTextContent("Commander Vex");

    cleanup();
    plan.setup = SETUP;
    renderFight(fightHost());
    await screen.findByTestId(assaultFightTestid.screen);
    expect(screen.getByTestId(testid.hero("opponent")).querySelector(".hero-seat")).toHaveTextContent("Opponent");
  });

  it("R4582002 the rail lists a live modifier under the enemy's name", async () => {
    renderFight(fightHost([], "failed", 0, { opponent: opponentModifiers }));
    await screen.findByTestId(assaultFightTestid.screen);

    const chip = screen.getByTestId(modifierListTestid.chip);
    expect(chip).toHaveAttribute("data-count", String(opponentModifiers.length));
    fireEvent.click(chip);
    const panel = await screen.findByTestId(modifierListTestid.panel);

    const side = panel.querySelector('[data-side="opponent"]');
    expect(side?.querySelector(".modifier-list__who")).toHaveTextContent("Rogue Model");
    expect(side?.querySelector(".modifier-list__who")?.textContent).toBe(screen.getByTestId(assaultFightTestid.enemy).textContent);
    for (const modifier of opponentModifiers) expect(side).toHaveTextContent(modifier.label);
    expect(panel.querySelector('[data-side="you"]')).toBeNull();
  });
});
