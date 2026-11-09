// B27–B31 (DR14 part F, #2707): the dungeon run's screens between fights, rendering `RunScreen`
// with a fake `Dungeon`. Only part C's three run reads are mocked (`runMaxHp`, `runScore`,
// `runEmbers`); the faces draw the compiled-in catalog, so the cards are real ones, the shop's prices
// and the campfire's count come from the real module (`runShopPrices`, `runConstants`), and the fake
// store's `act` records every `RunAction` the screens send and answers the configured refusal.

import { act as flush, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Mock } from "vitest";

import type {
  RunAction,
  RunContent,
  RunNode,
  RunNodeKind,
  RunPhase,
  RunState,
  RunStats,
} from "@jackioh/run";

import { navTestid } from "../../routes/nav.tsx";
import { readSheet } from "../../test/setup.ts";
import { at } from "../../test/stubServer.ts";
import { runConstants, runShopPrices } from "../../wasm/index.ts";
import { fightTestid } from "../fight/testids.ts";
import type { Dungeon } from "../store.ts";
import { expectOnePageTitle } from "../../test/pageTitle.ts";
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
      id: "test-relic-haggle",
      name: "Haggle Relic",
      kind: "economic",
      text: "Shop Coin prices fall by 20%.",
      tier: "bronze",
      effect: { type: "shopDiscount", percent: 20 },
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
  curses: [
    {
      id: "test-curse-heavy",
      name: "Heavy Hands",
      text: "Enemies start each fight with 5 more health.",
      points: 2,
    },
  ],
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

/** The node's drawn icon (#3229): an SVG tagged with its kind, in place of a system emoji. */
function expectIcon(button: HTMLElement, kind: RunNodeKind): void {
  expect(button.querySelector(`svg[data-icon="${kind}"]`)).toBeInTheDocument();
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
  timeMs: number | null = null,
): { dungeon: Dungeon; act: Mock<(action: RunAction) => string | null>; abandon: Mock<() => void> } {
  const act = vi.fn((_action: RunAction): string | null => refusal);
  const abandon = vi.fn((): void => {});
  const dungeon = {
    content: CONTENT,
    progress: { unlocks: { level: 1, cards: [], artifacts: [], owned: null }, embers: 0 },
    run,
    startedAt: 1_000,
    timeMs,
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
function expectHud(coins = "11", tickets = "3"): void {
  expect(screen.getByTestId(dungeonMapTestid.hp)).toHaveTextContent("7 / 20");
  expect(screen.getByTestId(dungeonMapTestid.coins)).toHaveTextContent(coins);
  expect(screen.getByTestId(dungeonMapTestid.tickets)).toHaveTextContent(tickets);
  expect(screen.getByTestId(dungeonMapTestid.deckSize)).toHaveTextContent("3");
  const chip = screen.getByTestId(dungeonMapTestid.hudArtifact(0));
  expect(chip).toHaveTextContent("Might Relic");
  expect(chip).toHaveTextContent("Level 2");
  expect(screen.getByTestId(dungeonMapTestid.score)).toHaveTextContent("130");
}

describe("R3223000 the run screen's title", () => {
  it("R3223000 the screen has one visible h1, the PageTitle", () => {
    render(<RunScreen dungeon={fakeDungeon(runWith({ phase: "map" })).dungeon} />);
    expectOnePageTitle("Dungeon run");
  });
});

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

describe("R2972000 the run map links back to the Play menu", () => {
  // R2972000
  it("R2972000 the run map links back to the Play menu and keeps the run", () => {
    at("/");
    const { dungeon, act, abandon } = fakeDungeon(runWith({ phase: "map" }));
    render(<RunScreen dungeon={dungeon} />);
    const back = screen.getByTestId(navTestid.back);
    expect(back).toHaveAttribute("data-route", "/play");
    fireEvent.click(back);
    expect(window.location.pathname).toBe("/play");
    // Following it sends nothing: the run stays saved, and the start screen offers Continue.
    expect(act).not.toHaveBeenCalled();
    expect(abandon).not.toHaveBeenCalled();
    at("/");
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
    const kinds: [number, number, string, RunNodeKind][] = [
      [0, 0, "Fight", "fight"],
      [0, 1, "Elite", "elite"],
      [1, 0, "Shop", "shop"],
      [1, 1, "Campfire", "campfire"],
      [1, 2, "Event", "event"],
      [1, 3, "Shrine", "shrine"],
      [1, 4, "Chest", "chest"],
      [2, 0, "Campfire", "campfire"],
      [3, 0, "Mini-boss", "miniBoss"],
    ];
    for (const [row, col, label, kind] of kinds) {
      const held = within(screen.getByTestId(dungeonMapTestid.mapNode(row, col)));
      expect(held.getByText(label)).toBeInTheDocument();
      expectIcon(screen.getByTestId(dungeonMapTestid.mapNode(row, col)), kind);
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
    expectIcon(ghost, "ghost");
    expect(ghost).toBeEnabled();
    const boss = screen.getByTestId(dungeonMapTestid.mapNode(6, 0));
    expect(boss).toHaveTextContent("Final boss");
    expectIcon(boss, "boss");
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
    // A purse that pays for every service: a service the run cannot pay is disabled (below).
    const { dungeon, act } = fakeDungeon(
      runWith(
        {
          phase: "shop",
          stock: [
            { good: { type: "card", id: "core-004" }, price: 50, sold: false },
            { good: { type: "artifact", id: "test-relic-skip" }, price: 120, sold: true },
          ],
        },
        { coins: 900, tickets: 90 },
      ),
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
    expectHud("900", "90");
  });

  /** A shop of one card for 50 Coins, the purse as given and `artifacts` held. */
  function shopRun(extra: Partial<RunState> = {}): RunState {
    return runWith(
      { phase: "shop", stock: [{ good: { type: "card", id: "core-004" }, price: 50, sold: false }] },
      extra,
    );
  }

  // B30
  it("B30 the shop's services name their price, relic discount included", () => {
    const rich = { coins: 900, tickets: 90 };
    const plain = shopRun(rich);
    const haggled = shopRun({
      ...rich,
      artifacts: [
        { id: "test-relic-might", level: 2 },
        { id: "test-relic-haggle", level: 1 },
      ],
    });
    const plainPrices = runShopPrices(plain, CONTENT);
    const haggledPrices = runShopPrices(haggled, CONTENT);
    expect(haggledPrices.removeCoins).toBeLessThan(plainPrices.removeCoins);
    expect(haggledPrices.stockCoins[0]).toBeLessThan(50);
    expect(plainPrices.stockCoins).toEqual([50]);

    for (const [run, prices] of [
      [plain, plainPrices],
      [haggled, haggledPrices],
    ] as const) {
      render(<RunScreen dungeon={fakeDungeon(run).dungeon} />);
      const heading = (name: string): HTMLElement => screen.getByRole("heading", { name });
      expect(heading(`Remove a card for ${String(prices.removeCoins)} Coins`)).toBeInTheDocument();
      expect(heading(`Make a card Radiant for ${String(prices.radiantTickets)} Tickets`)).toBeInTheDocument();
      expect(heading(`Buff a card for ${String(prices.buffTickets)} Tickets`)).toBeInTheDocument();
      expect(screen.getByTestId(dungeonMapTestid.shopLevel(0))).toHaveTextContent(
        `Level up for ${String(prices.levelTickets[0])} Tickets`,
      );
      // The stock shows what Buy spends, not the item's price before the discount.
      expect(screen.getByTestId(dungeonMapTestid.shopStock(0))).toHaveTextContent(
        `${String(prices.stockCoins[0])} Coins`,
      );
      cleanup();
    }
  });

  // B30
  it("B30 a service the run cannot pay is disabled", () => {
    const services = [
      dungeonMapTestid.shopRemove(0),
      dungeonMapTestid.shopRadiant(0),
      dungeonMapTestid.shopBuff(0),
      dungeonMapTestid.shopLevel(0),
    ];
    // Nothing in the purse: every service is out of reach, but Buy stays, and the run says why.
    const { dungeon } = fakeDungeon(shopRun({ coins: 0, tickets: 0 }));
    render(<RunScreen dungeon={dungeon} />);
    for (const id of services) expect(screen.getByTestId(id)).toBeDisabled();
    expect(screen.getByTestId(dungeonMapTestid.shopBuy(0))).toBeEnabled();
    cleanup();

    render(<RunScreen dungeon={fakeDungeon(shopRun({ coins: 900, tickets: 90 })).dungeon} />);
    for (const id of services) expect(screen.getByTestId(id)).toBeEnabled();
  });

  // B30
  it("B30 a relic at its top level shows no price and no Level up button", () => {
    const run = shopRun({ coins: 900, tickets: 90, artifacts: [{ id: "test-relic-might", level: 99 }] });
    expect(runShopPrices(run, CONTENT).levelTickets).toEqual([null]);
    render(<RunScreen dungeon={fakeDungeon(run).dungeon} />);
    expect(screen.getByTestId(dungeonMapTestid.shopLevelMax(0))).toHaveTextContent("Highest level");
    expect(screen.queryByTestId(dungeonMapTestid.shopLevel(0))).not.toBeInTheDocument();
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

  // R2972000
  it("R2972000 B30 the campfire waits for exactly campfireRemove cards", () => {
    const { campfireRemove } = runConstants();
    // One card more than a campfire removes, so the pick has somewhere to overflow to.
    const ids = ["core-001", "core-002", "core-003", "core-004"];
    const deck = Array.from({ length: campfireRemove + 1 }, (_, place) => ({
      id: ids[place % ids.length] ?? "core-001",
      radiant: false,
      buffs: 0,
    }));
    const { dungeon, act } = fakeDungeon(runWith({ phase: "campfire" }, { deck }));
    render(<RunScreen dungeon={dungeon} />);
    expect(screen.getByRole("heading", { name: `Remove ${String(campfireRemove)} cards` })).toBeInTheDocument();

    const remove = screen.getByTestId(dungeonMapTestid.campfireRemove);
    const card = (place: number): HTMLElement => screen.getByTestId(dungeonMapTestid.campfireRemoveCard(place));
    expect(remove).toBeDisabled();
    for (let place = 0; place < campfireRemove; place += 1) {
      expect(remove).toBeDisabled();
      fireEvent.click(card(place));
    }
    expect(remove).toBeEnabled();

    // A further pick is ignored: that card is out of reach and stays unpicked.
    const extra = card(campfireRemove);
    expect(extra).toBeDisabled();
    fireEvent.click(extra);
    expect(extra).toHaveAttribute("aria-pressed", "false");
    expect(remove).toBeEnabled();

    // Putting one back disables the button again, and the card put back is the one to pick.
    fireEvent.click(card(0));
    expect(remove).toBeDisabled();
    expect(extra).toBeEnabled();
    fireEvent.click(card(0));
    expect(remove).toBeEnabled();

    // The run reads the indices as a set: they go in the order they were picked, card 0 last now.
    fireEvent.click(remove);
    expect(act).toHaveBeenCalledTimes(1);
    const sent = act.mock.calls[0]?.[0];
    expect(sent).toEqual({ type: "removeCards", indices: expect.any(Array) });
    expect(sent?.type === "removeCards" ? [...sent.indices].sort((a, b) => a - b) : null).toEqual(
      Array.from({ length: campfireRemove }, (_, place) => place),
    );
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
  it("B31 the run's end shows its time and turns beside the score", () => {
    const dead = runWith({ phase: "dead" }, { stats: statsWith({ turns: 17 }) });
    render(<RunScreen dungeon={fakeDungeon(dead, null, 65_000).dungeon} />);
    const score = screen.getByTestId(dungeonMapTestid.endScore);
    const time = screen.getByTestId(dungeonMapTestid.endTime);
    const turns = screen.getByTestId(dungeonMapTestid.endTurns);
    expect(time).toHaveTextContent("Time: 1:05");
    expect(turns).toHaveTextContent("Turns: 17");
    expect(time.parentElement).toBe(score.parentElement);
    expect(turns.parentElement).toBe(score.parentElement);
    cleanup();

    // A run whose time is not known shows no time, and still its turns.
    render(<RunScreen dungeon={fakeDungeon(dead).dungeon} />);
    expect(screen.queryByTestId(dungeonMapTestid.endTime)).not.toBeInTheDocument();
    expect(screen.getByTestId(dungeonMapTestid.endTurns)).toHaveTextContent("Turns: 17");
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

describe("#3501 the run's panel", () => {
  const FULL = {
    artifacts: [
      { id: "test-relic-might", level: 2 },
      { id: "test-relic-skip", level: 1 },
    ],
    blessings: ["test-blessing-dawn"],
    curses: ["test-curse-heavy"],
  };

  /** Renders the run on its map and opens the panel; returns it, scoped to its dialog. */
  function openPanel(extra: Partial<RunState> = FULL, phase: RunPhase = { phase: "map" }): ReturnType<typeof within> {
    render(<RunScreen dungeon={fakeDungeon(runWith(phase, extra)).dungeon} />);
    fireEvent.click(screen.getByTestId(dungeonMapTestid.yourRun));
    return within(screen.getByRole("dialog", { name: "Your run" }));
  }

  /** Replaces the clipboard, which jsdom lacks, for the call. */
  async function withClipboard(writeText: (text: string) => Promise<void>, body: () => Promise<void>): Promise<void> {
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    try {
      await body();
    } finally {
      Reflect.deleteProperty(navigator, "clipboard");
    }
  }

  it("#3501 Your run lists every deck card with its marks, each relic, blessing and curse with its text, and the seed", () => {
    const panel = openPanel();
    DECK.forEach((card, index) => {
      const face = panel.getByTestId(dungeonMapTestid.panelCard(index));
      expect(face).toHaveAttribute("data-card", card.id);
      // Only the Radiant card is dealt on the Radiant face.
      expect(face.querySelector('[data-radiant-face="true"]') !== null).toBe(card.radiant);
    });
    expect(panel.getByTestId(dungeonMapTestid.panelCard(1))).toHaveTextContent("Buffs: 2");
    expect(panel.getByTestId(dungeonMapTestid.panelCard(0))).not.toHaveTextContent("Buffs");
    expect(panel.getByTestId(dungeonMapTestid.panelCard(2))).not.toHaveTextContent("Buffs");
    expect(panel.getByRole("heading", { name: "Deck: 3" })).toBeInTheDocument();

    const might = panel.getByTestId(dungeonMapTestid.panelArtifact(0));
    expect(might).toHaveTextContent("Might Relic");
    expect(might).toHaveTextContent("Level 2");
    expect(might).toHaveTextContent("Gain 1 more Coin after each fight.");
    const skip = panel.getByTestId(dungeonMapTestid.panelArtifact(1));
    expect(skip).toHaveTextContent("Skip Relic");
    expect(skip).toHaveTextContent("Level 1");
    expect(skip).toHaveTextContent("A pack may be kept empty, for Coins.");
    const blessing = panel.getByTestId(dungeonMapTestid.panelBlessing(0));
    expect(blessing).toHaveTextContent("Blessing of Dawn");
    expect(blessing).toHaveTextContent("Heal 3 after each fight.");
    const curse = panel.getByTestId(dungeonMapTestid.panelCurse(0));
    expect(curse).toHaveTextContent("Heavy Hands");
    expect(curse).toHaveTextContent("Enemies start each fight with 5 more health.");
    expect(curse).toHaveTextContent("Points: 2");
    expect(panel.getByTestId(dungeonMapTestid.panelSeed)).toHaveTextContent("ABCDEF");
  });

  it("#3501 a run holding none says so, and a curse the content lacks shows its id", () => {
    const empty = openPanel({ artifacts: [], blessings: [], curses: [] });
    expect(empty.getByTestId(dungeonMapTestid.panelArtifacts)).toHaveTextContent("No relics");
    expect(empty.getByTestId(dungeonMapTestid.panelBlessings)).toHaveTextContent("No blessings");
    expect(empty.getByTestId(dungeonMapTestid.panelCurses)).toHaveTextContent("No curses");
    cleanup();

    const gone = openPanel({ curses: ["gone-curse"] });
    expect(gone.getByTestId(dungeonMapTestid.panelCurses)).toHaveTextContent("gone-curse");
  });

  it("#3501 an empty deck says so instead of drawing an empty grid", () => {
    const panel = openPanel({ deck: [] });
    expect(panel.getByText("No cards yet.")).toBeInTheDocument();
    expect(panel.queryByTestId(dungeonMapTestid.panelDeck)).not.toBeInTheDocument();
    expect(panel.getByRole("heading", { name: "Deck: 0" })).toBeInTheDocument();
  });

  it("#3501 the panel takes focus on its ✕, Escape closes it and the focus returns to Your run", () => {
    openPanel();
    expect(screen.getByTestId(dungeonMapTestid.panelClose)).toHaveFocus();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.getByTestId(dungeonMapTestid.yourRun)).toHaveFocus();

    fireEvent.click(screen.getByTestId(dungeonMapTestid.yourRun));
    expect(screen.getByRole("dialog", { name: "Your run" })).toBeInTheDocument();
    fireEvent.click(screen.getByTestId(dungeonMapTestid.panelClose));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("#3501 Copy seed writes the seed to the clipboard and says Copied", async () => {
    const writeText = vi.fn(async (_text: string): Promise<void> => {});
    await withClipboard(writeText, async () => {
      const panel = openPanel();
      const copy = panel.getByTestId(dungeonMapTestid.panelSeedCopy);
      expect(copy).toHaveTextContent("Copy seed");
      await flush(async () => {
        fireEvent.click(copy);
        await Promise.resolve();
      });
      await waitFor(() => {
        expect(copy).toHaveTextContent("Copied");
      });
      expect(writeText).toHaveBeenCalledWith("ABCDEF");
    });
  });

  it("#3501 a refused clipboard leaves the button as Copy seed", async () => {
    const writeText = vi.fn(async (_text: string): Promise<void> => {
      throw new Error("refused");
    });
    await withClipboard(writeText, async () => {
      const panel = openPanel();
      const copy = panel.getByTestId(dungeonMapTestid.panelSeedCopy);
      await flush(async () => {
        fireEvent.click(copy);
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
      expect(writeText).toHaveBeenCalledWith("ABCDEF");
      expect(copy).toHaveTextContent("Copy seed");
    });
  });

  it("#3501 the panel is on the run-over screen too", () => {
    render(<RunScreen dungeon={fakeDungeon(runWith({ phase: "won" })).dungeon} />);
    expect(screen.getByTestId(dungeonMapTestid.yourRun)).toBeInTheDocument();
  });

  it("#3501 the run-over screen shows the run's seed", () => {
    render(<RunScreen dungeon={fakeDungeon(runWith({ phase: "dead" }, { seed: "QWERTY" })).dungeon} />);
    expect(screen.getByTestId(dungeonMapTestid.endSeed)).toHaveTextContent("Seed: QWERTY");
  });

  it("#3501 at phone width the panel scrolls and nothing overflows sideways", () => {
    // jsdom lays nothing out, so the sheet's rules are the proof: the panel never outgrows its screen, hides
    // what runs past its side, and has no height of its own, so the scrim scrolls it; the deck's columns
    // shrink to three at a phone's width.
    const sheet = readSheet("src/dungeon/map/run-panel.css");
    expect(sheet).toContain("width: min(720px, 100%)");
    expect(sheet).toContain("overflow-x: hidden");
    expect(sheet).toContain("repeat(auto-fill, minmax(96px, 1fr))");
    expect(sheet).not.toContain("max-height");
  });
});
