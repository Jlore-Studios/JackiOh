// The Dungeon Run page's tests' fixtures (DR14 part D, #2705): the run's numbers and content as `crates/run`
// states them, a run's state, and the browser's run bindings (part C's `wasm/index.ts`) as scripted
// stand-ins. The tests mock `../wasm/index.ts` with `wasmBindings`, so the page's own code runs on
// answers the test controls; the run's real rules are proved in `crates/run`.

import { vi } from "vitest";

import type { CustomCard } from "@jackioh/shared";
import type {
  RunAction,
  RunConstants,
  RunContent,
  RunFile,
  RunSetup,
  RunState,
  RunStats,
  RunUnlocks,
} from "@jackioh/run";

/** DR3's ladder, its 10 cards and its 3 artifacts worth 8 points, and the five tiers' points. */
export const CONSTANTS: RunConstants = {
  slotLadder: [
    [5, 4, 0, 0, 0, 1],
    [4, 4, 1, 0, 0, 1],
    [3, 4, 2, 0, 0, 1],
    [2, 3, 3, 1, 0, 1],
    [1, 2, 3, 2, 1, 1],
    [0, 2, 3, 2, 1, 2],
  ],
  starterSize: 10,
  starterArtifactsMax: 3,
  starterArtifactPoints: 8,
  tierPoints: [
    ["bronze", 1],
    ["silver", 2],
    ["gold", 3],
    ["platinum", 4],
    ["diamond", 5],
  ],
};

/** The Embers each level costs from the one before it: `LEVEL_EMBERS[0]` buys level 2. */
export const LEVEL_EMBERS = [10, 25, 45, 70, 100];

/** Ten starting cards of the catalog's Training Grounds, so their faces draw as the deck builder draws them. */
export const STARTERS = [
  "traininggrounds-001",
  "traininggrounds-002",
  "traininggrounds-004",
  "traininggrounds-006",
  "traininggrounds-007",
  "traininggrounds-008",
  "traininggrounds-011",
  "traininggrounds-017",
  "traininggrounds-020",
  "traininggrounds-023",
];
/** A card in the pool a mini-boss extraction unlocks, and one that stays locked. */
export const EXTRACTABLE = "traininggrounds-003";
export const LOCKED = "traininggrounds-005";

function poolCard(id: string, name: string, rarity: "Common" | "Rare", cardType: "Unit" | "Spell" | "Field Spell"): RunContent["pool"][number] {
  return { id, name, rarity, set: "Training Grounds", cardType, tags: [] };
}

/** Twelve catalog cards: the ten starters, one a run could unlock and one it has not. */
export const CONTENT: RunContent = {
  pool: [
    poolCard("traininggrounds-001", "Spikey Steven", "Common", "Unit"),
    poolCard("traininggrounds-002", "Personal Trainer", "Common", "Unit"),
    poolCard(EXTRACTABLE, "Group leader", "Common", "Unit"),
    poolCard("traininggrounds-004", "Boulder Fist Ogre", "Common", "Unit"),
    poolCard(LOCKED, "Midlane Menace", "Common", "Unit"),
    poolCard("traininggrounds-006", "Chosen by the Sun", "Common", "Unit"),
    poolCard("traininggrounds-007", "Chosen by the Moon", "Common", "Unit"),
    poolCard("traininggrounds-008", "Clobberer", "Common", "Unit"),
    poolCard("traininggrounds-011", "Thwomp", "Common", "Spell"),
    poolCard("traininggrounds-017", "Flaming Felinor", "Rare", "Unit"),
    poolCard("traininggrounds-020", "Recruitment Posters", "Rare", "Field Spell"),
    poolCard("traininggrounds-023", "Quadrupling Felinors", "Rare", "Unit"),
  ],
  starterAccess: STARTERS,
  enemies: [],
  passives: [
    { id: "lucky-coin", name: "Lucky Coin", kind: "economic", text: "Gain 5 more Coins after each fight.", tier: "bronze", effect: null, starter: true, unobtainable: false, god: null },
    { id: "steel-tooth", name: "Steel Tooth", kind: "combat", text: "Start each fight with 1 more mana.", tier: "gold", effect: null, starter: true, unobtainable: false, god: null },
    { id: "sun-idol", name: "Sun Idol", kind: "economic", text: "Heal 5 after each fight.", tier: "diamond", effect: null, starter: true, unobtainable: false, god: null },
    { id: "moon-idol", name: "Moon Idol", kind: "combat", text: "Draw 1 more card.", tier: "platinum", effect: null, starter: true, unobtainable: false, god: null },
    { id: "locked-sigil", name: "Locked Sigil", kind: "combat", text: "Opens with a final boss.", tier: "silver", effect: null, starter: false, unobtainable: false, god: null },
    { id: "looming-presence", name: "Looming Presence", kind: "enemy", text: "Starts with 1 more mana.", tier: null, effect: null, starter: false, unobtainable: true, god: null },
    { id: "blessing-of-dawn", name: "Blessing of Dawn", kind: "blessing", text: "Heal 3 after each fight.", tier: null, effect: null, starter: false, unobtainable: true, god: "dawn" },
  ],
  events: [],
  curses: [
    { id: "heavy-hands", name: "Heavy Hands", text: "Enemies start with 2 more health.", points: 2 },
    { id: "frail", name: "Frail", text: "You start each fight with less health.", points: 3 },
  ],
};

export function unlocks(over: Partial<RunUnlocks> = {}): RunUnlocks {
  return { level: 1, cards: [], artifacts: [], owned: null, ...over };
}

export const ZERO_STATS: RunStats = {
  nodesVisited: 0,
  fightsWon: 0,
  elitesWon: 0,
  ghostsWon: 0,
  miniBossesWon: 0,
  bossesWon: 0,
  damageDealt: 0,
  healingDone: 0,
  cardsDrawn: 0,
  turns: 0,
  damageTaken: 0,
  healed: 0,
  coinsEarned: 0,
  coinsSpent: 0,
  ticketsEarned: 0,
  ticketsSpent: 0,
  cardsKept: 0,
  cardsBought: 0,
  cardsRemoved: 0,
  cardsMadeRadiant: 0,
  cardsBuffed: 0,
  packsSkipped: 0,
  artifactsTaken: 0,
  artifactsLeveled: 0,
  blessingsTaken: 0,
  eventsSeen: 0,
  chestsOpened: 0,
};

export function runState(over: Partial<RunState> = {}): RunState {
  return {
    seed: "ABCDEF",
    rngCursor: 0,
    map: [],
    row: null,
    col: 0,
    hp: 30,
    deck: [],
    artifacts: [],
    blessings: [],
    curses: [],
    coins: 0,
    tickets: 0,
    extractedCards: [],
    extractedArtifacts: [],
    phase: { phase: "map" },
    stats: ZERO_STATS,
    ...over,
  };
}

/** A setup the stand-in run accepts: ten starters and nothing else. */
export function setupOf(over: Partial<RunSetup> = {}): RunSetup {
  return { seed: "ABCDEF", starter: STARTERS, artifacts: [], curses: [], ...over };
}

/** A card as the account keeps it. */
export const IMP: CustomCard = {
  id: "custom-ember-imp",
  version: 2,
  frame: { name: "Ember Imp", type: "Unit", cost: 1, rarity: "Common", attack: 1, health: 2, text: "" },
  source: "",
};

/** Six capital letters from any text: a deterministic stand-in for `runSeed`, which the Rust draws from. */
export function seedFrom(text: string): string {
  let state = 7;
  for (const ch of text) state = (state * 31 + ch.charCodeAt(0)) >>> 0;
  let seed = "";
  for (let i = 0; i < 6; i++) {
    state = (Math.imul(state, 1103515245) + 12345) >>> 0;
    seed += String.fromCharCode(65 + ((state >>> 16) % 26));
  }
  return seed;
}

/**
 * The stand-in's own rules, as little as a test needs: Go enters a fight, Abandon and a lost fight end
 * the run, anything else moves the cursor on, and "refuse" is refused.
 */
function reduceStandIn(state: RunState, action: RunAction): { state: RunState } | { error: string } {
  if (state.phase.phase === "won" || state.phase.phase === "dead") return { error: "the run is over" };
  switch (action.type) {
    case "abandon":
      return { state: { ...state, phase: { phase: "dead" } } };
    case "go":
      return { state: { ...state, row: (state.row ?? -1) + 1, col: action.col, phase: { phase: "fight" } } };
    case "fightOver":
      return { state: { ...state, phase: action.won ? { phase: "map" } : { phase: "dead" } } };
    case "leave":
      return { error: "there is nothing to leave" };
    default:
      return { state: { ...state, rngCursor: state.rngCursor + 1 } };
  }
}

function createStandIn(setup: RunSetup): { state: RunState } {
  return { state: runState({ seed: setup.seed, curses: setup.curses }) };
}

function foldStandIn(file: RunFile): { state: RunState } | { error: string } {
  let state = createStandIn(file.setup).state;
  for (const [step, action] of file.log.entries()) {
    const next = reduceStandIn(state, action);
    if ("error" in next) return { error: `step ${String(step)}: ${next.error}` };
    state = next.state;
  }
  return { state };
}

/**
 * The browser's run bindings, scripted. A test spreads them into its `../wasm/index.ts` mock and sets
 * what it needs on each (`wasmBindings.runCheckSetup.mockReturnValue("…")`); `resetBindings` puts every
 * default back.
 */
export const wasmBindings = {
  runContent: vi.fn((_customCards: CustomCard[]): RunContent => CONTENT),
  runCheckSetup: vi.fn((_setup: RunSetup, _unlocks: RunUnlocks, _content: RunContent): string | null => null),
  runCreate: vi.fn((setup: RunSetup, _unlocks: RunUnlocks, _content: RunContent): { state: RunState } | { error: string } =>
    createStandIn(setup),
  ),
  runReduce: vi.fn((state: RunState, _content: RunContent, action: RunAction): { state: RunState } | { error: string } =>
    reduceStandIn(state, action),
  ),
  runFold: vi.fn((file: RunFile, _content: RunContent): { state: RunState } | { error: string } => foldStandIn(file)),
  runScore: vi.fn((_state: RunState, _content: RunContent): number => 120),
  runEmbers: vi.fn((score: number): number => Math.floor(score / 10)),
  runLevelCost: vi.fn((level: number): number | null => LEVEL_EMBERS[level - 1] ?? null),
  runConstants: vi.fn((): RunConstants => CONSTANTS),
  runSeed: vi.fn((source: string): string => seedFrom(source)),
  runDailySeed: vi.fn((date: string, n: number): string => seedFrom(`daily:${date}:${String(n)}`)),
};

/** Every binding back to its default answer, and every call forgotten. */
export function resetBindings(): void {
  wasmBindings.runContent.mockReset().mockImplementation(() => CONTENT);
  wasmBindings.runCheckSetup.mockReset().mockImplementation(() => null);
  wasmBindings.runCreate.mockReset().mockImplementation((setup) => createStandIn(setup));
  wasmBindings.runReduce.mockReset().mockImplementation((state, _content, action) => reduceStandIn(state, action));
  wasmBindings.runFold.mockReset().mockImplementation((file) => foldStandIn(file));
  wasmBindings.runScore.mockReset().mockImplementation(() => 120);
  wasmBindings.runEmbers.mockReset().mockImplementation((score) => Math.floor(score / 10));
  wasmBindings.runLevelCost.mockReset().mockImplementation((level) => LEVEL_EMBERS[level - 1] ?? null);
  wasmBindings.runConstants.mockReset().mockImplementation(() => CONSTANTS);
  wasmBindings.runSeed.mockReset().mockImplementation((source) => seedFrom(source));
  wasmBindings.runDailySeed.mockReset().mockImplementation((date, n) => seedFrom(`daily:${date}:${String(n)}`));
}
