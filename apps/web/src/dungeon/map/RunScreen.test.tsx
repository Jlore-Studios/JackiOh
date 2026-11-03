// B27–B31 (DR14 part F, #2707): the dungeon run's screens between fights, rendering `RunScreen`
// with a fake `Dungeon`. Only part C's three run reads are mocked (`runMaxHp`, `runScore`,
// `runEmbers`); the faces draw the compiled-in catalog, so the cards are real ones, and the fake
// store's `act` records every `RunAction` the screens send and answers the configured refusal.

import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Mock } from "vitest";

import type {
  RunAction,
  RunContent,
  RunNode,
  RunPhase,
  RunState,
  RunStats,
} from "@jackioh/run";

import { fightTestid } from "../fight/testids.ts";
import type { Dungeon } from "../store.ts";
import { RUN_STAT_LABELS } from "./RunOverScreen.tsx";
import { RunScreen } from "./RunScreen.tsx";
import { dungeonMapTestid } from "./testids.ts";

const plan = vi.hoisted(() => ({ maxHp: 20, score: 130, embers: 13 }));

vi.mock("../../wasm/index.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../wasm/index.ts")>();
  return {
    ...actual,
    runMaxHp: (): number => plan.maxHp,
    runScore: (): number => plan.score,
    runEmbers: (): number => plan.embers,
  };
});

const CONTENT: RunContent = {
  pool: [
    { id: "core-001", name: "Big D-fender", rarity: "Common", set: "Core", cardType: "Unit", tags: [] },
    { id: "core-002", name: "Speciesist", rarity: "Common", set: "Core", cardType: "Unit", tags: [] },
    { id: "core-003", name: "Right-house defender", rarity: "Common", set: "Core", cardType: "Unit", tags: [] },
    { id: "core-004", name: "Gary the Gambler", rarity: "Common", set: "Core", cardType: "Unit", tags: [] },
  ],
  starterAccess: ["core-001", "core-002", "core-003", "core-004"],
  enemies: [],
  passives: [
    {
      id: "test-relic-might",
      name: "Might Relic",
      kind: "economic",
      text: "Gain 1 more Coin after each fight.",
      tier: "silver",
      effect: { type: "coinsPerFight", n: 1 },
      starter: true,
      unobtainable: false,
      god: null,
    },
    {
      id: "test-relic-skip",
      name: "Skip Relic",
      kind: "economic",
      text: "A pack may be kept empty, for Coins.",
      tier: "bronze",
      effect: { type: "skip", coins: 5 },
      starter: false,
      unobtainable: false,
      god: null,
    },
    {
      id: "test-boss-crown",
      name: "Boss Crown",
      kind: "enemy",
      text: "The final boss's own.",
      tier: "diamond",
      effect: null,
      starter: false,
      unobtainable: true,
      god: null,
    },
    {
      id: "test-blessing-dawn",
      name: "Blessing of Dawn",
      kind: "blessing",
      text: "Heal 3 after each fight.",
      tier: null,
      effect: null,
      starter: false,
      unobtainable: false,
      god: "Dawn",
    },
  ],
  events: [
    {
      id: "test-event",
      text: "A stranger offers a deal.",
      options: [
        { text: "Pay 5 Coins", hp: 0, coins: -5, tickets: 0, card: false, artifact: false },
        { text: "Walk away", hp: 0, coins: 0, tickets: 0, card: false, artifact: false },
      ],
    },
  ],
  curses: [],
};

const STAT_KEYS = Object.keys(RUN_STAT_LABELS) as (keyof RunStats)[];

/** Every stat distinct, so the run-over screen's rows each prove their own value. */
function statsWith(values: Partial<RunStats> = {}): RunStats {
  const fell: Record<string, number> = {};
  for (const key of STAT_KEYS) fell[key] = 0;
  return { ...fell, ...values } as unknown as RunStats;
}

function node(kind: RunNode["kind"], extra: Partial<RunNode> = {}): RunNode {
  return { kind, enemy: null, event: null, artifact: null, ...extra };
}

function enemy(name: string): RunNode["enemy"] {
  return { id: `enemy-${name}`, name, heroHealth: 20, deck: ["core-001"], artifacts: [] };
}

/** The map's two acts: rows 0–3 end at the mini-boss, rows 4–6 at the final boss. */
const MAP: RunState["map"] = [
  [node("fight", { enemy: enemy("Cave Troll") }), node("elite", { enemy: enemy("Elite Guard") })],
  [node("shop"), node("campfire"), node("event", { event: "test-event" }), node("shrine"), node("chest")],
  [node("campfire")],
  [node("miniBoss", { enemy: enemy("Mini Boss"), artifact: "test-relic-might" })],
  [node("fight", { enemy: enemy("Road Thug") }), node("ghost", { enemy: enemy("Ghost") })],
  [node("campfire")],
  [node("boss", { enemy: enemy("Final Boss"), artifact: "test-boss-crown" })],
];

const DECK: RunState["deck"] = [
  { id: "core-001", radiant: false, buffs: 0 },
  { id: "core-002", radiant: true, buffs: 2 },
  { id: "core-003", radiant: false, buffs: 0 },
];

function runWith(phase: RunPhase, extra: Partial<RunState> = {}): RunState {
  return {
    seed: "ABCDEF",
    rngCursor: 0,
    map: MAP,
    row: 0,
    col: 0,
    hp: 7,
    deck: DECK,
    artifacts: [{ id: "test-relic-might", level: 2 }],
    blessings: [],
    curses: [],
    coins: 11,
    tickets: 3,
    extractedCards: [],
    extractedArtifacts: [],
    phase,
    stats: statsWith(),
    ...extra,
  };
}

function fakeDungeon(
  run: RunState,
  refusal: string | null = null,
): { dungeon: Dungeon; act: Mock<(action: RunAction) => string | null>; abandon: Mock<() => void> } {
  const act = vi.fn((_action: RunAction): string | null => refusal);
  const abandon = vi.fn((): void => {});
  const dungeon = {
    content: CONTENT,
    progress: { unlocks: { level: 1, cards: [], artifacts: [], owned: null }, embers: 0 },
    run,
    startedAt: 1_000,
    start: vi.fn(() => null),
    act,
    abandon,
    buyLevel: vi.fn(() => null),
  } as unknown as Dungeon;
  return { dungeon, act, abandon };
}

beforeEach(() => {
  plan.maxHp = 20;
  plan.score = 130;
  plan.embers = 13;
});

afterEach(() => {
  cleanup();
  window.localStorage.clear();
});

/** B30: HP, Coins, Tickets, the deck, the relic held and the score, on every screen. */
function expectHud(): void {
  expect(screen.getByTestId(dungeonMapTestid.hp)).toHaveTextContent("7 / 20");
  expect(screen.getByTestId(dungeonMapTestid.coins)).toHaveTextContent("11");
  expect(screen.getByTestId(dungeonMapTestid.tickets)).toHaveTextContent("3");
  expect(screen.getByTestId(dungeonMapTestid.deckSize)).toHaveTextContent("3");
  const chip = screen.getByTestId(dungeonMapTestid.hudArtifact(0));
  expect(chip).toHaveTextContent("Might Relic");
  expect(chip).toHaveTextContent("Level 2");
  expect(screen.getByTestId(dungeonMapTestid.score)).toHaveTextContent("130");
}

describe("B24 the run's time on screen between fights", () => {
  // B24
  it("B24 the HUD shows the run's time from its start until the run is over", () => {
    vi.useFakeTimers({ now: 66_000, toFake: ["Date"] });
    try {
      const { dungeon } = fakeDungeon(runWith({ phase: "map" }));
      render(<RunScreen dungeon={dungeon} />);
      expect(within(screen.getByTestId(dungeonMapTestid.hud)).getByTestId(fightTestid.timer)).toHaveTextContent("1:05");
      cleanup();
      render(<RunScreen dungeon={fakeDungeon(runWith({ phase: "dead" })).dungeon} />);
      expect(screen.queryByTestId(fightTestid.timer)).not.toBeInTheDocument();
    } finally {
      vi.useRealTimers();
    }
  });
});

describe("B27 the map shows the current act and only the next row goes", () => {
  // B27
  it("B27 shows the current act and only sends go for the next row", () => {
    const { dungeon, act } = fakeDungeon(runWith({ phase: "map" }));
    render(<RunScreen dungeon={dungeon} />);

    // The whole first act: rows 0–3, each node's kind with its icon and label.
    for (const row of [0, 1, 2, 3]) expect(screen.getByTestId(dungeonMapTestid.mapRow(row))).toBeInTheDocument();
    expect(screen.queryByTestId(dungeonMapTestid.mapRow(4))).not.toBeInTheDocument();
    const kinds: [number, number, string, string][] = [
      [0, 0, "Fight", "⚔️"],
      [0, 1, "Elite", "🛡️"],
      [1, 0, "Shop", "🛒"],
      [1, 1, "Campfire", "🔥"],
      [1, 2, "Event", "❓"],
      [1, 3, "Shrine", "⛩️"],
      [1, 4, "Chest", "🎁"],
      [2, 0, "Campfire", "🔥"],
      [3, 0, "Mini-boss", "🐲"],
    ];
    for (const [row, col, label, icon] of kinds) {
      const held = within(screen.getByTestId(dungeonMapTestid.mapNode(row, col)));
      expect(held.getByText(label)).toBeInTheDocument();
      expect(screen.getByTestId(dungeonMapTestid.mapNode(row, col))).toHaveTextContent(icon);
    }
    // The fight's name, the boss's preseeded relic, and where the player stands.
    expect(screen.getByTestId(dungeonMapTestid.mapNode(0, 0))).toHaveTextContent("Cave Troll");
    expect(screen.getByTestId(dungeonMapTestid.mapArtifact(3, 0))).toHaveTextContent("Might Relic");
    expect(screen.queryByTestId(dungeonMapTestid.mapArtifact(0, 0))).not.toBeInTheDocument();
    expect(
      within(screen.getByTestId(dungeonMapTestid.mapNode(0, 0))).getByTestId(dungeonMapTestid.mapCurrent),
    ).toBeInTheDocument();

    // Only the next row's nodes can be chosen.
    for (const col of [0, 1, 2, 3, 4]) {
      expect(screen.getByTestId(dungeonMapTestid.mapNode(1, col))).toBeEnabled();
    }
    for (const [row, col] of [[0, 0], [0, 1], [2, 0], [3, 0]] as const) {
      expect(screen.getByTestId(dungeonMapTestid.mapNode(row, col))).toBeDisabled();
    }
    fireEvent.click(screen.getByTestId(dungeonMapTestid.mapNode(1, 2)));
    expect(act).toHaveBeenCalledTimes(1);
    expect(act).toHaveBeenCalledWith({ type: "go", col: 2 });
    expectHud();
  });

  // B27
  it("B27 past a beaten boss the next act shows, with no marker on the old one", () => {
    const { dungeon, act } = fakeDungeon(runWith({ phase: "map" }, { row: 3, col: 0 }));
    render(<RunScreen dungeon={dungeon} />);

    expect(screen.queryByTestId(dungeonMapTestid.mapRow(0))).not.toBeInTheDocument();
    for (const row of [4, 5, 6]) expect(screen.getByTestId(dungeonMapTestid.mapRow(row))).toBeInTheDocument();
    expect(screen.queryByTestId(dungeonMapTestid.mapCurrent)).not.toBeInTheDocument();
    const ghost = screen.getByTestId(dungeonMapTestid.mapNode(4, 1));
    expect(ghost).toHaveTextContent("Ghost");
    expect(ghost).toHaveTextContent("👻");
    expect(ghost).toBeEnabled();
    const boss = screen.getByTestId(dungeonMapTestid.mapNode(6, 0));
    expect(boss).toHaveTextContent("Final boss");
    expect(boss).toHaveTextContent("👑");
    expect(boss).toBeDisabled();
    expect(screen.getByTestId(dungeonMapTestid.mapArtifact(6, 0))).toHaveTextContent("Boss Crown");
    expect(screen.getByTestId(dungeonMapTestid.mapNode(5, 0))).toBeDisabled();
    fireEvent.click(screen.getByTestId(dungeonMapTestid.mapNode(4, 0)));
    expect(act).toHaveBeenCalledWith({ type: "go", col: 0 });
  });
});

describe("B28 the relic comes before the packs, and the pack keeps its count", () => {
  type RewardPacks = Extract<RunPhase, { phase: "reward" }>["packs"];
  const DEFAULT_PACKS: RewardPacks = [
    { type: "boss" },
    { type: "random" },
    { type: "set", set: "Core" },
    { type: "cardType", cardType: "Unit" },
  ];
  const packs = (packList: RewardPacks): RunPhase => ({ phase: "reward", artifact: null, packs: packList });
  const dropped = (): RunPhase => ({ phase: "reward", artifact: "test-relic-might", packs: [] });

  // B28
  it("B28 takes or leaves the relic before the four packs show", () => {
    const first = fakeDungeon(runWith(dropped()));
    render(<RunScreen dungeon={first.dungeon} />);
    expect(screen.getByTestId(dungeonMapTestid.rewardArtifact)).toHaveTextContent("Might Relic");
    expect(screen.queryByTestId(dungeonMapTestid.packs)).not.toBeInTheDocument();
    fireEvent.click(screen.getByTestId(dungeonMapTestid.artifactTake));
    expect(first.act).toHaveBeenCalledWith({ type: "takeArtifact", take: true });
    expectHud();
    cleanup();

    const second = fakeDungeon(runWith(dropped()));
    render(<RunScreen dungeon={second.dungeon} />);
    fireEvent.click(screen.getByTestId(dungeonMapTestid.artifactLeave));
    expect(second.act).toHaveBeenCalledWith({ type: "takeArtifact", take: false });
  });

  // B28
  it("B28 the four packs are labelled by kind and open by index", () => {
    const { dungeon, act } = fakeDungeon(runWith(packs(DEFAULT_PACKS)));
    render(<RunScreen dungeon={dungeon} />);
    expect(screen.getByTestId(dungeonMapTestid.pack(0))).toHaveTextContent("Boss deck");
    expect(screen.getByTestId(dungeonMapTestid.pack(1))).toHaveTextContent("Random");
    expect(screen.getByTestId(dungeonMapTestid.pack(2))).toHaveTextContent("Set: Core");
    expect(screen.getByTestId(dungeonMapTestid.pack(3))).toHaveTextContent("Type: Unit");
    fireEvent.click(screen.getByTestId(dungeonMapTestid.pack(2)));
    expect(act).toHaveBeenCalledWith({ type: "openPack", index: 2 });
    expectHud();
    cleanup();

    const tagged = fakeDungeon(
      runWith(
        packs([
          { type: "tag", tag: "Pack tag" },
          { type: "rarity", rarity: "Rare" },
          { type: "boss" },
          { type: "random" },
        ]),
      ),
    );
    render(<RunScreen dungeon={tagged.dungeon} />);
    expect(screen.getByTestId(dungeonMapTestid.pack(0))).toHaveTextContent("Tag: Pack tag");
    expect(screen.getByTestId(dungeonMapTestid.pack(1))).toHaveTextContent("Rarity: Rare");
    fireEvent.click(screen.getByTestId(dungeonMapTestid.pack(1)));
    expect(tagged.act).toHaveBeenCalledWith({ type: "openPack", index: 1 });
  });

  // B28
  it("B28 the pack keeps exactly keep cards, and no Skip without the passive", () => {
    const { dungeon, act } = fakeDungeon(
      runWith({ phase: "pack", cards: ["core-001", "core-002", "core-003", "core-004"], keep: 2 }),
    );
    render(<RunScreen dungeon={dungeon} />);
    const keep = screen.getByTestId(dungeonMapTestid.packKeep);
    expect(keep).toBeDisabled();
    fireEvent.click(screen.getByTestId(dungeonMapTestid.packCard("core-001")));
    expect(keep).toBeDisabled();
    fireEvent.click(screen.getByTestId(dungeonMapTestid.packCard("core-003")));
    expect(keep).toBeEnabled();
    // Past keep, a third card does not join the keep.
    fireEvent.click(screen.getByTestId(dungeonMapTestid.packCard("core-002")));
    expect(screen.getByTestId(dungeonMapTestid.packCard("core-002"))).toHaveAttribute("aria-pressed", "false");
    fireEvent.click(keep);
    expect(act).toHaveBeenCalledWith({ type: "keep", cards: ["core-001", "core-003"] });
    expect(screen.queryByTestId(dungeonMapTestid.packSkip)).not.toBeInTheDocument();
  });

  // B28
  it("B28 with a Skip passive the pack may be kept empty", () => {
    const { dungeon, act } = fakeDungeon(
      runWith(
        { phase: "pack", cards: ["core-001", "core-002", "core-003", "core-004"], keep: 2 },
        { artifacts: [{ id: "test-relic-skip", level: 1 }] },
      ),
    );
    render(<RunScreen dungeon={dungeon} />);
    fireEvent.click(screen.getByTestId(dungeonMapTestid.packSkip));
    expect(act).toHaveBeenCalledWith({ type: "keep", cards: [] });
  });
});

describe("B29 extraction is a card or a relic, and the boss's own is refused", () => {
  // B29
  it("B29 extracts a card of the deck by index, or nothing", () => {
    const { dungeon, act } = fakeDungeon(runWith({ phase: "extractCard" }));
    render(<RunScreen dungeon={dungeon} />);
    fireEvent.click(screen.getByTestId(dungeonMapTestid.extractCardChoice(1)));
    expect(act).toHaveBeenCalledWith({ type: "extract", index: 1 });
    fireEvent.click(screen.getByTestId(dungeonMapTestid.extractNone));
    expect(act).toHaveBeenCalledWith({ type: "extract", index: null });
    expectHud();
  });

  // B29
  it("B29 the final boss's own relic is sent and its refusal is shown", () => {
    const refusal = "test-boss-crown is a boss's own and cannot be extracted";
    const { dungeon, act } = fakeDungeon(
      runWith(
        { phase: "extractArtifact" },
        { artifacts: [{ id: "test-boss-crown", level: 1 }, { id: "test-relic-might", level: 2 }] },
      ),
      refusal,
    );
    render(<RunScreen dungeon={dungeon} />);
    const own = screen.getByTestId(dungeonMapTestid.extractArtifactChoice(0));
    expect(own).toHaveTextContent("Boss Crown");
    expect(own).toBeEnabled();
    fireEvent.click(own);
    expect(act).toHaveBeenCalledWith({ type: "extract", index: 0 });
    expect(screen.getByTestId(dungeonMapTestid.refusal)).toHaveTextContent(refusal);
    fireEvent.click(screen.getByTestId(dungeonMapTestid.extractArtifactChoice(1)));
    expect(act).toHaveBeenCalledWith({ type: "extract", index: 1 });
  });
});

describe("B30 the tiles send their actions under the always-on HUD", () => {
  // B30
  it("B30 the shop buys, upgrades and leaves, naming prices and the refusal", () => {
    const { dungeon, act } = fakeDungeon(
      runWith({
        phase: "shop",
        stock: [
          { good: { type: "card", id: "core-004" }, price: 50, sold: false },
          { good: { type: "artifact", id: "test-relic-skip" }, price: 120, sold: true },
        ],
      }),
      "11 Coins needed, 5 held",
    );
    render(<RunScreen dungeon={dungeon} />);
    expect(screen.getByTestId(dungeonMapTestid.shopStock(0))).toHaveTextContent("50 Coins");
    expect(screen.getByTestId(dungeonMapTestid.shopStock(1))).toHaveTextContent("120 Coins");
    expect(screen.getByTestId(dungeonMapTestid.shopStock(1))).toHaveTextContent("Sold");
    expect(screen.queryByTestId(dungeonMapTestid.shopBuy(1))).not.toBeInTheDocument();
    fireEvent.click(screen.getByTestId(dungeonMapTestid.shopBuy(0)));
    expect(act).toHaveBeenCalledWith({ type: "buy", index: 0 });
    expect(screen.getByTestId(dungeonMapTestid.refusal)).toHaveTextContent("11 Coins needed, 5 held");
    fireEvent.click(screen.getByTestId(dungeonMapTestid.shopRemove(1)));
    expect(act).toHaveBeenCalledWith({ type: "removeCard", index: 1 });
    fireEvent.click(screen.getByTestId(dungeonMapTestid.shopRadiant(0)));
    expect(act).toHaveBeenCalledWith({ type: "makeRadiant", index: 0 });
    fireEvent.click(screen.getByTestId(dungeonMapTestid.shopBuff(2)));
    expect(act).toHaveBeenCalledWith({ type: "buff", index: 2 });
    fireEvent.click(screen.getByTestId(dungeonMapTestid.shopLevel(0)));
    expect(act).toHaveBeenCalledWith({ type: "levelArtifact", index: 0 });
    fireEvent.click(screen.getByTestId(dungeonMapTestid.shopLeave));
    expect(act).toHaveBeenCalledWith({ type: "leave" });
    expectHud();
  });

  // B30
  it("B30 the campfire heals, Radiants, buffs and removes the toggled cards", () => {
    const { dungeon, act } = fakeDungeon(runWith({ phase: "campfire" }));
    render(<RunScreen dungeon={dungeon} />);
    fireEvent.click(screen.getByTestId(dungeonMapTestid.campfireHeal));
    expect(act).toHaveBeenCalledWith({ type: "heal" });
    fireEvent.click(screen.getByTestId(dungeonMapTestid.campfireRadiant(1)));
    expect(act).toHaveBeenCalledWith({ type: "makeRadiant", index: 1 });
    fireEvent.click(screen.getByTestId(dungeonMapTestid.campfireBuff(0)));
    expect(act).toHaveBeenCalledWith({ type: "buff", index: 0 });
    fireEvent.click(screen.getByTestId(dungeonMapTestid.campfireRemoveCard(0)));
    fireEvent.click(screen.getByTestId(dungeonMapTestid.campfireRemoveCard(2)));
    fireEvent.click(screen.getByTestId(dungeonMapTestid.campfireRemove));
    expect(act).toHaveBeenCalledWith({ type: "removeCards", indices: [0, 2] });
    expectHud();
  });

  // B30
  it("B30 the event shows its text and sends the chosen option", () => {
    const { dungeon, act } = fakeDungeon(runWith({ phase: "event", id: "test-event" }));
    render(<RunScreen dungeon={dungeon} />);
    expect(screen.getByTestId(dungeonMapTestid.event)).toHaveTextContent("A stranger offers a deal.");
    expect(screen.getByTestId(dungeonMapTestid.eventOption(0))).toHaveTextContent("Pay 5 Coins");
    fireEvent.click(screen.getByTestId(dungeonMapTestid.eventOption(1)));
    expect(act).toHaveBeenCalledWith({ type: "choose", index: 1 });
    expectHud();
  });

  // B30
  it("B30 the shrine sends the blessing taken or left", () => {
    const { dungeon, act } = fakeDungeon(
      runWith({ phase: "shrine", offers: ["test-blessing-dawn", "test-relic-might"] }),
    );
    render(<RunScreen dungeon={dungeon} />);
    expect(screen.getByTestId(dungeonMapTestid.shrineOffer(0))).toHaveTextContent("Blessing of Dawn");
    expect(screen.getByTestId(dungeonMapTestid.shrineOffer(1))).toHaveTextContent("Might Relic");
    fireEvent.click(screen.getByTestId(dungeonMapTestid.shrineOffer(0)));
    expect(act).toHaveBeenCalledWith({ type: "bless", index: 0 });
    fireEvent.click(screen.getByTestId(dungeonMapTestid.shrineLeave));
    expect(act).toHaveBeenCalledWith({ type: "bless", index: null });
    expectHud();
  });
});

describe("B31 the run-over screen settles the run", () => {
  // B31
  it("B31 shows score, Embers, extracts and every run statistic at run over", () => {
    const values: Record<string, number> = {};
    STAT_KEYS.forEach((key, index) => {
      values[key] = index + 1;
    });
    const { dungeon, abandon } = fakeDungeon(
      runWith(
        { phase: "won" },
        {
          extractedCards: ["core-001"],
          extractedArtifacts: ["test-relic-might"],
          stats: statsWith(values as Partial<RunStats>),
        },
      ),
    );
    render(<RunScreen dungeon={dungeon} />);
    expect(screen.getByTestId(dungeonMapTestid.endScore)).toHaveTextContent("130");
    expect(screen.getByTestId(dungeonMapTestid.endEmbers)).toHaveTextContent("13");
    expect(screen.getByTestId(dungeonMapTestid.extractedCards)).toHaveTextContent("Big D-fender");
    expect(screen.getByTestId(dungeonMapTestid.extractedArtifacts)).toHaveTextContent("Might Relic");
    expect(STAT_KEYS).toHaveLength(27);
    STAT_KEYS.forEach((key, index) => {
      const row = screen.getByTestId(dungeonMapTestid.stat(key));
      expect(row).toHaveTextContent(RUN_STAT_LABELS[key]);
      expect(row).toHaveTextContent(String(index + 1));
    });
    expect(screen.getByTestId(dungeonMapTestid.stat("artifactsTaken"))).toHaveTextContent("Relics taken");
    fireEvent.click(screen.getByTestId(dungeonMapTestid.newRun));
    expect(abandon).toHaveBeenCalledTimes(1);
    const link = screen.getByTestId(dungeonMapTestid.statsLink);
    expect(link).toHaveAttribute("href", "/dungeon/stats");
    expectHud();
  });

  // B31
  it("B31 a dead run with no extracts names its empty states", () => {
    const { dungeon } = fakeDungeon(runWith({ phase: "dead" }));
    render(<RunScreen dungeon={dungeon} />);
    expect(screen.getByTestId(dungeonMapTestid.runOver)).toHaveTextContent("Defeat");
    expect(screen.getByTestId(dungeonMapTestid.extractedCards)).toHaveTextContent("No cards extracted.");
    expect(screen.getByTestId(dungeonMapTestid.extractedArtifacts)).toHaveTextContent("No relics extracted.");
  });
});
