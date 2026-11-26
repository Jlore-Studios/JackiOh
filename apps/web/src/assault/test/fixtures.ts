// Scripted frozen-surface answers for the Assault screen and store tests. The Rust parts prove the
// rules; these fixtures prove that the browser renders and persists their answers unchanged.

import { vi } from "vitest";

import type {
  AssaultAction,
  AssaultConstants,
  AssaultContent,
  AssaultDay,
  AssaultFile,
  AssaultRecord,
  AssaultSetup,
  AssaultState,
} from "@jackioh/run";

const HANDICAP = {
  deckSize: 25,
  manaBonus: 0,
  manaCap: 4,
  extraOpeningCards: 0,
  extraDrawsPerTurn: 0,
};

export const CONTENT: AssaultContent = {
  cards: [],
  doctrines: [
    {
      id: "firewall",
      name: "Firewall",
      line: "Walls, shields and Traps punish attacks.",
      enemies: ["Packet Filter", "Bastion Node"],
    },
  ],
  commanders: [
    {
      id: "bastion",
      name: "Bastion.exe",
      rank: 1,
      doctrine: "firewall",
      cards: ["core-001", "core-002"],
    },
  ],
};

export const DAY: AssaultDay = {
  date: "2026-11-23",
  seed: "assault-2026-11-23",
  levels: [
    {
      level: 1,
      enemy: "Packet Filter",
      doctrine: "firewall",
      commander: null,
      escalations: [],
      upgrades: [],
      rarities: [19, 6, 0, 0, 0],
      deck: ["core-001", "core-002"],
      marks: [],
      handicap: HANDICAP,
    },
    {
      level: 2,
      enemy: "Bastion Node",
      doctrine: "firewall",
      commander: "bastion",
      escalations: [{ kind: "cards" }, { kind: "upgrade", upgrade: "heroArmor" }],
      upgrades: [{ kind: "heroArmor", stacks: 1 }],
      rarities: [17, 7, 1, 0, 0],
      deck: ["core-001", "core-002"],
      marks: [],
      handicap: { ...HANDICAP, heroArmor: 5 },
    },
  ],
};

export const CONSTANTS: AssaultConstants = {
  levels: 12,
  commanderLevels: [2, 6, 12],
  checkpoints: [2, 4, 8],
  historyDays: 7,
};

export const RECORD: AssaultRecord = {
  date: DAY.date,
  best: 1,
  checkpoints: [2],
  attempts: 2,
  repelledOn: null,
};

export function setupOf(over: Partial<AssaultSetup> = {}): AssaultSetup {
  return {
    date: DAY.date,
    deck: ["core-001", "core-002"],
    source: "dayDeck",
    startLevel: 1,
    attempt: 1,
    ...over,
  };
}

export function stateOf(over: Partial<AssaultState> = {}): AssaultState {
  return {
    setup: setupOf(),
    level: 1,
    phase: "briefing",
    beaten: [],
    turns: 0,
    ...over,
  };
}

function dayOf(date: string): AssaultDay {
  return { ...DAY, date, seed: `assault-${date}` };
}

function reduceStandIn(state: AssaultState, action: AssaultAction): { state: AssaultState } | { error: string } {
  switch (action.kind) {
    case "fight":
      return state.phase === "briefing" ? { state: { ...state, phase: "fight" } } : { error: "the level is not at its briefing" };
    case "fightOver":
      if (state.phase !== "fight") return { error: "there is no fight to report" };
      if (!action.won) return { state: { ...state, phase: "lost", turns: state.turns + action.turns } };
      if (state.level >= DAY.levels.length) {
        return { state: { ...state, phase: "repelled", beaten: [...state.beaten, state.level], turns: state.turns + action.turns } };
      }
      return {
        state: {
          ...state,
          phase: "briefing",
          level: state.level + 1,
          beaten: [...state.beaten, state.level],
          turns: state.turns + action.turns,
        },
      };
    case "abandon":
      return { state: { ...state, phase: "abandoned" } };
  }
}

function foldStandIn(file: AssaultFile): { state: AssaultState } | { error: string } {
  let state = stateOf({ setup: file.setup, level: file.setup.startLevel });
  for (const action of file.actions) {
    const next = reduceStandIn(state, action);
    if ("error" in next) return next;
    state = next.state;
  }
  return { state };
}

function recordStandIn(record: AssaultRecord, state: AssaultState): AssaultRecord {
  const best = Math.max(record.best, ...state.beaten, 0);
  const checkpoints = [...new Set([...record.checkpoints, ...CONSTANTS.checkpoints.filter((level) => best >= level)])];
  return {
    ...record,
    best,
    checkpoints,
    attempts: Math.max(record.attempts, state.setup.attempt),
    repelledOn: state.phase === "repelled" ? (record.repelledOn ?? state.setup.attempt) : record.repelledOn,
  };
}

export const bindingFixtures = {
  assaultContent: vi.fn((): AssaultContent => CONTENT),
  assaultContentAt: vi.fn((_version: string): AssaultContent | null => CONTENT),
  assaultDayDeck: vi.fn((_date: string): string[] => ["core-001", "core-002"]),
  assaultDaySeed: vi.fn((date: string): string => `assault-${date}`),
  assaultDay: vi.fn((date: string, _content: AssaultContent): AssaultDay => dayOf(date)),
  assaultCreate: vi.fn((setup: AssaultSetup, _record: AssaultRecord, _day: AssaultDay) => ({
    state: stateOf({ setup, level: setup.startLevel }),
  })),
  assaultReduce: vi.fn((state: AssaultState, _day: AssaultDay, action: AssaultAction) => reduceStandIn(state, action)),
  assaultFightSetup: vi.fn(() => null),
  assaultFightReport: vi.fn(() => ({ action: { kind: "fightOver", won: true, turns: 4 } as AssaultAction })),
  assaultRecord: vi.fn(recordStandIn),
  assaultFold: vi.fn((file: AssaultFile, _day: AssaultDay) => foldStandIn(file)),
  assaultConstants: vi.fn((): AssaultConstants => CONSTANTS),
};

export function resetBindings(): void {
  bindingFixtures.assaultContent.mockReset().mockReturnValue(CONTENT);
  bindingFixtures.assaultContentAt.mockReset().mockReturnValue(CONTENT);
  bindingFixtures.assaultDayDeck.mockReset().mockReturnValue(["core-001", "core-002"]);
  bindingFixtures.assaultDaySeed.mockReset().mockImplementation((date) => `assault-${date}`);
  bindingFixtures.assaultDay.mockReset().mockImplementation((date) => dayOf(date));
  bindingFixtures.assaultCreate.mockReset().mockImplementation((setup) => ({ state: stateOf({ setup, level: setup.startLevel }) }));
  bindingFixtures.assaultReduce.mockReset().mockImplementation((state, _day, action) => reduceStandIn(state, action));
  bindingFixtures.assaultFightSetup.mockReset().mockReturnValue(null);
  bindingFixtures.assaultFightReport.mockReset().mockReturnValue({ action: { kind: "fightOver", won: true, turns: 4 } });
  bindingFixtures.assaultRecord.mockReset().mockImplementation(recordStandIn);
  bindingFixtures.assaultFold.mockReset().mockImplementation((file) => foldStandIn(file));
  bindingFixtures.assaultConstants.mockReset().mockReturnValue(CONSTANTS);
}
