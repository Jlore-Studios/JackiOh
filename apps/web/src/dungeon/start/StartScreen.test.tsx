// B14–B19 (DR14 part D, #2705): the Dungeon Run's start screen. The run's rules are `crates/run`'s, so
// `../wasm/index.ts`'s run bindings are the fixtures' scripted ones and the `Dungeon` is a plain object
// whose verbs record what the screen sends them; the store's own behavior is `../store.test.ts`'s.
// Time is frozen at a known UTC day so the daily seed is the one the fixtures' `runDailySeed` makes.

import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { DUNGEON_PROGRESS_KEY } from "../config.ts";
import { __resetDungeonForTests, type Dungeon } from "../store.ts";
import { dungeonTestid } from "../testids.ts";
import {
  CONTENT,
  EXTRACTABLE,
  LOCKED,
  STARTERS,
  resetBindings,
  runState,
  seedFrom,
  unlocks,
  wasmBindings,
} from "../test/fixtures.ts";
import { StartScreen } from "./StartScreen.tsx";

vi.mock("../../wasm/index.ts", async (importOriginal) => ({
  ...(await importOriginal<Record<string, unknown>>()),
  ...(await import("../test/fixtures.ts")).wasmBindings,
}));

const TODAY = "2026-11-01";

function dungeonOf(over: Partial<Dungeon> = {}): Dungeon {
  return {
    content: CONTENT,
    progress: { unlocks: unlocks(), embers: 0 },
    run: null,
    startedAt: null,
    start: vi.fn(() => null),
    act: vi.fn(() => null),
    abandon: vi.fn(),
    buyLevel: vi.fn(() => null),
    ...over,
  };
}

function show(dungeon: Dungeon = dungeonOf(), onEnter: () => void = vi.fn()): { dungeon: Dungeon; onEnter: () => void } {
  render(<StartScreen dungeon={dungeon} onEnter={onEnter} />);
  return { dungeon, onEnter };
}

const byId = (id: string): HTMLElement => screen.getByTestId(id);

/** Press these cards, in this order. */
function pick(ids: readonly string[]): void {
  for (const id of ids) fireEvent.click(byId(dungeonTestid.card(id)));
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(new Date("2026-11-01T12:00:00Z"));
  window.localStorage.clear();
  __resetDungeonForTests();
  resetBindings();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
  __resetDungeonForTests();
});

describe("B14 the modes", () => {
  it("B14 Sandbox and disabled Campaign", () => {
    show();

    expect(byId(dungeonTestid.modeSandbox)).toBeChecked();
    expect(byId(dungeonTestid.modeSandbox)).toBeEnabled();
    expect(byId(dungeonTestid.modeCampaign)).toBeDisabled();
    expect(byId(dungeonTestid.modeCampaign)).not.toBeChecked();
    expect(byId(dungeonTestid.campaignNote)).toHaveTextContent("Sign in to play the Campaign: coming with accounts.");
  });
});

describe("B15 the seed", () => {
  it("B15 daily random and typed seeds", () => {
    show();

    // The day's run is chosen to begin with, from today's date (UTC) and the day's first run number.
    expect(byId(dungeonTestid.seedDaily)).toBeChecked();
    expect(wasmBindings.runDailySeed).toHaveBeenLastCalledWith(TODAY, 1);
    expect(byId(dungeonTestid.seedValue)).toHaveTextContent(`Seed: ${seedFrom(`daily:${TODAY}:1`)}`);

    // A random seed is drawn from a random number, and drawn again on every press.
    const drawn = (): string[] => wasmBindings.runSeed.mock.calls.map(([source]) => source).filter((source) => source !== "length");
    const before = drawn().length;
    fireEvent.click(byId(dungeonTestid.seedRandom));
    expect(byId(dungeonTestid.seedRandom)).toBeChecked();
    expect(drawn()).toHaveLength(before + 1);
    const first = byId(dungeonTestid.seedValue).textContent;
    expect(first).toBe(`Seed: ${seedFrom(drawn().at(-1) ?? "")}`);
    fireEvent.click(byId(dungeonTestid.seedRandom));
    expect(drawn()).toHaveLength(before + 2);
    expect(byId(dungeonTestid.seedValue).textContent).toBe(`Seed: ${seedFrom(drawn().at(-1) ?? "")}`);
    expect(byId(dungeonTestid.seedValue).textContent).not.toBe(first);

    // A typed seed is six capital letters: anything else is refused, and the run cannot start on it.
    pick(STARTERS);
    fireEvent.click(byId(dungeonTestid.seedTyped));
    const input = byId(dungeonTestid.seedInput);
    for (const wrong of ["ABC", "abcdef", "ABCDEFG", "AB1DEF", "AB DEF"]) {
      fireEvent.change(input, { target: { value: wrong } });
      expect(byId(dungeonTestid.seedError)).toHaveTextContent("A seed is 6 capital letters, A to Z.");
      expect(byId(dungeonTestid.startRun)).toBeDisabled();
    }
    fireEvent.change(input, { target: { value: "QWERTY" } });
    expect(screen.queryByTestId(dungeonTestid.seedError)).toBeNull();
    expect(byId(dungeonTestid.seedValue)).toHaveTextContent("Seed: QWERTY");
    expect(byId(dungeonTestid.startRun)).toBeEnabled();
  });

  it("B15 the run starts on the seed chosen", () => {
    const { dungeon } = show();
    pick(STARTERS);

    fireEvent.click(byId(dungeonTestid.startRun));
    expect(dungeon.start).toHaveBeenLastCalledWith({
      seed: seedFrom(`daily:${TODAY}:1`),
      starter: STARTERS,
      artifacts: [],
      curses: [],
    });

    fireEvent.click(byId(dungeonTestid.seedTyped));
    fireEvent.change(byId(dungeonTestid.seedInput), { target: { value: "QWERTY" } });
    fireEvent.click(byId(dungeonTestid.startRun));
    expect(dungeon.start).toHaveBeenLastCalledWith(expect.objectContaining({ seed: "QWERTY" }));
  });

  it("B15 the day's number moves on when the day's earlier runs were started on this device", () => {
    window.localStorage.setItem(
      DUNGEON_PROGRESS_KEY,
      JSON.stringify({ v: 1, unlocks: unlocks(), embers: 0, daily: { date: TODAY, started: 2 } }),
    );
    show();

    expect(wasmBindings.runDailySeed).toHaveBeenLastCalledWith(TODAY, 3);
    expect(byId(dungeonTestid.seedValue)).toHaveTextContent(`Seed: ${seedFrom(`daily:${TODAY}:3`)}`);
  });
});

describe("B16 the starting deck", () => {
  it("B16 slots unlocked card faces setup refusal and relic limits", () => {
    // The run refuses a deck of the wrong size, as `run_check_setup` does.
    wasmBindings.runCheckSetup.mockImplementation(
      (setup) => `a starting deck holds 10 cards, not ${String(setup.starter.length)}`,
    );
    const view = show();

    // The level's slots, from the ladder the run states: level 1 has no Epic, Legendary or Mythic slot.
    expect(byId(dungeonTestid.slot("Common"))).toHaveTextContent("Common × 5");
    expect(byId(dungeonTestid.slot("Rare"))).toHaveTextContent("Rare × 4");
    expect(byId(dungeonTestid.slot("Anything"))).toHaveTextContent("Anything × 1");
    expect(screen.queryByTestId(dungeonTestid.slot("Epic"))).toBeNull();
    expect(wasmBindings.runConstants).toHaveBeenCalled();

    // Only the unlocked cards are offered, each drawn as a card face of the catalog.
    const cards = byId(dungeonTestid.cards);
    for (const id of STARTERS) expect(within(cards).getByTestId(dungeonTestid.card(id))).toBeInTheDocument();
    expect(screen.queryByTestId(dungeonTestid.card(EXTRACTABLE))).toBeNull();
    expect(screen.queryByTestId(dungeonTestid.card(LOCKED))).toBeNull();
    expect(within(byId(dungeonTestid.card(STARTERS[0] ?? ""))).getByText("Spikey Steven")).toBeInTheDocument();

    // The deck's count is out of the size the run states.
    expect(byId(dungeonTestid.chosenCount)).toHaveTextContent("Cards: 0 of 10");
    expect(byId(dungeonTestid.startRun)).toBeDisabled();
    pick(STARTERS.slice(0, 3));
    expect(byId(dungeonTestid.chosenCount)).toHaveTextContent("Cards: 3 of 10");
    expect(byId(dungeonTestid.card(STARTERS[0] ?? ""))).toHaveAttribute("aria-pressed", "true");

    // What the run refuses is shown as the run says it, and the run does not start on it.
    pick(STARTERS.slice(3, 4));
    expect(byId(dungeonTestid.setupRefusal)).toHaveTextContent("a starting deck holds 10 cards, not 4");
    expect(byId(dungeonTestid.startRun)).toBeDisabled();
    expect(view.dungeon.start).not.toHaveBeenCalled();
  });

  it("B16 a deck that stands starts the run, and one the run then refuses says why", () => {
    wasmBindings.runCheckSetup.mockImplementation((setup) =>
      setup.starter.length === 10 ? null : `a starting deck holds 10 cards, not ${String(setup.starter.length)}`,
    );
    const start = vi.fn<Dungeon["start"]>(() => "Finish or abandon the run in progress first.");
    const onEnter = vi.fn();
    show(dungeonOf({ start }), onEnter);
    pick(STARTERS);
    expect(screen.queryByTestId(dungeonTestid.setupRefusal)).toBeNull();
    expect(byId(dungeonTestid.chosenCount)).toHaveTextContent("Cards: 10 of 10");

    // The deck is as many cards as it holds: another is not taken, and the run is asked for nothing.
    fireEvent.click(byId(dungeonTestid.startRun));
    expect(byId(dungeonTestid.setupRefusal)).toHaveTextContent("Finish or abandon the run in progress first.");
    expect(onEnter).not.toHaveBeenCalled();

    start.mockReturnValue(null);
    fireEvent.click(byId(dungeonTestid.startRun));
    expect(onEnter).toHaveBeenCalledTimes(1);
    expect(screen.queryByTestId(dungeonTestid.setupRefusal)).toBeNull();
  });

  it("B16 an extracted card joins the cards offered, and no more than the deck's size can be taken", () => {
    show(dungeonOf({ progress: { unlocks: unlocks({ cards: [EXTRACTABLE, STARTERS[0] ?? ""] }), embers: 0 } }));

    expect(byId(dungeonTestid.card(EXTRACTABLE))).toBeInTheDocument();
    expect(screen.queryByTestId(dungeonTestid.card(LOCKED))).toBeNull();
    // A card both a starting card and extracted is offered once.
    expect(screen.getAllByTestId(dungeonTestid.card(STARTERS[0] ?? ""))).toHaveLength(1);
    pick(STARTERS);
    expect(byId(dungeonTestid.card(EXTRACTABLE))).toBeDisabled();
    // A card taken can be put back.
    fireEvent.click(byId(dungeonTestid.card(STARTERS[1] ?? "")));
    expect(byId(dungeonTestid.chosenCount)).toHaveTextContent("Cards: 9 of 10");
    expect(byId(dungeonTestid.card(EXTRACTABLE))).toBeEnabled();
  });

  it("B16 relics are shown with their points out of the most, at most three, and offer only those unlocked", () => {
    show();

    // The starting relics a run may take: a combat or an economic one that starts unlocked; no enemy's,
    // no blessing, and not the one a final boss has still to give.
    for (const id of ["lucky-coin", "steel-tooth", "sun-idol", "moon-idol"]) {
      expect(byId(dungeonTestid.relic(id))).toBeInTheDocument();
    }
    for (const id of ["locked-sigil", "looming-presence", "blessing-of-dawn"]) {
      expect(screen.queryByTestId(dungeonTestid.relic(id))).toBeNull();
    }
    expect(byId(dungeonTestid.relic("sun-idol"))).toHaveTextContent("Points: 5");
    expect(byId(dungeonTestid.relicCount)).toHaveTextContent("Relics: 0 of 3");
    expect(byId(dungeonTestid.relicPoints)).toHaveTextContent("Relic points: 0 of 8");

    fireEvent.click(byId(dungeonTestid.relic("lucky-coin")));
    fireEvent.click(byId(dungeonTestid.relic("steel-tooth")));
    fireEvent.click(byId(dungeonTestid.relic("sun-idol")));
    expect(byId(dungeonTestid.relicCount)).toHaveTextContent("Relics: 3 of 3");
    // Their points are the tiers' (1 + 3 + 5), shown over the eight; the run says whether that stands.
    expect(byId(dungeonTestid.relicPoints)).toHaveTextContent("Relic points: 9 of 8");
    expect(byId(dungeonTestid.relic("moon-idol"))).toBeDisabled();

    fireEvent.click(byId(dungeonTestid.relic("sun-idol")));
    expect(byId(dungeonTestid.relicPoints)).toHaveTextContent("Relic points: 4 of 8");
    expect(byId(dungeonTestid.relic("moon-idol"))).toBeEnabled();
  });

  it("B16 an extracted relic is offered, and the relics taken go to the run", () => {
    wasmBindings.runCheckSetup.mockReturnValue(null);
    const { dungeon } = show(dungeonOf({ progress: { unlocks: unlocks({ artifacts: ["locked-sigil"] }), embers: 0 } }));
    pick(STARTERS);

    fireEvent.click(byId(dungeonTestid.relic("locked-sigil")));
    fireEvent.click(byId(dungeonTestid.relic("lucky-coin")));
    expect(byId(dungeonTestid.relicPoints)).toHaveTextContent("Relic points: 3 of 8");
    fireEvent.click(byId(dungeonTestid.startRun));
    expect(dungeon.start).toHaveBeenCalledWith(expect.objectContaining({ artifacts: ["locked-sigil", "lucky-coin"] }));
  });
});

describe("B17 the curses", () => {
  it("B17 curse tiers stack", () => {
    wasmBindings.runCheckSetup.mockReturnValue(null);
    const { dungeon } = show();
    pick(STARTERS);

    expect(byId(dungeonTestid.cursePoints)).toHaveTextContent("Curse points: 0");
    expect(byId(dungeonTestid.curse("heavy-hands"))).toHaveTextContent("Points: 2");
    expect(byId(dungeonTestid.curse("frail"))).toHaveTextContent("Points: 3");

    fireEvent.click(byId(dungeonTestid.curse("heavy-hands")));
    expect(byId(dungeonTestid.cursePoints)).toHaveTextContent("Curse points: 2");
    // A second curse stacks on the first, and both stay taken.
    fireEvent.click(byId(dungeonTestid.curse("frail")));
    expect(byId(dungeonTestid.cursePoints)).toHaveTextContent("Curse points: 5");
    expect(byId(dungeonTestid.curse("heavy-hands"))).toHaveAttribute("aria-pressed", "true");
    expect(byId(dungeonTestid.curse("frail"))).toHaveAttribute("aria-pressed", "true");

    fireEvent.click(byId(dungeonTestid.startRun));
    expect(dungeon.start).toHaveBeenCalledWith(expect.objectContaining({ curses: ["heavy-hands", "frail"] }));

    fireEvent.click(byId(dungeonTestid.curse("heavy-hands")));
    expect(byId(dungeonTestid.cursePoints)).toHaveTextContent("Curse points: 3");
  });
});

describe("B18 the starting deck's level", () => {
  it("B18 Embers buy the next level", () => {
    const buyLevel = vi.fn<Dungeon["buyLevel"]>(() => null);
    show(dungeonOf({ progress: { unlocks: unlocks(), embers: 30 }, buyLevel }));

    expect(byId(dungeonTestid.embers)).toHaveTextContent("Embers: 30");
    expect(byId(dungeonTestid.level)).toHaveTextContent("Starting deck level 1");
    // The price is the run's own for the level (`runLevelCost`).
    expect(wasmBindings.runLevelCost).toHaveBeenCalledWith(1);
    expect(byId(dungeonTestid.buyLevel)).toHaveTextContent("Raise to level 2 for 10 Embers");

    fireEvent.click(byId(dungeonTestid.buyLevel));
    expect(buyLevel).toHaveBeenCalledTimes(1);
    expect(screen.queryByTestId(dungeonTestid.buyRefusal)).toBeNull();
  });

  it("B18 a purchase the store refuses says why, and the top level has nothing more to buy", () => {
    const buyLevel = vi.fn<Dungeon["buyLevel"]>(() => "You need 10 Embers to raise your starting deck.");
    const view = render(<StartScreen dungeon={dungeonOf({ buyLevel })} onEnter={vi.fn()} />);

    fireEvent.click(byId(dungeonTestid.buyLevel));
    expect(byId(dungeonTestid.buyRefusal)).toHaveTextContent("You need 10 Embers to raise your starting deck.");

    view.rerender(<StartScreen dungeon={dungeonOf({ progress: { unlocks: unlocks({ level: 6 }), embers: 500 } })} onEnter={vi.fn()} />);
    expect(screen.queryByTestId(dungeonTestid.buyLevel)).toBeNull();
    expect(screen.getByText("Your starting deck is at its top level.")).toBeInTheDocument();
    expect(byId(dungeonTestid.slot("Anything"))).toHaveTextContent("Anything × 2");
    expect(screen.queryByTestId(dungeonTestid.slot("Common"))).toBeNull();
  });
});

describe("B19 a run in progress", () => {
  it("B19 a saved run offers continue or abandon", () => {
    const abandon = vi.fn();
    const onEnter = vi.fn();
    show(dungeonOf({ run: runState({ phase: { phase: "map" } }), abandon }), onEnter);

    expect(byId(dungeonTestid.saved)).toHaveTextContent("You have a run in progress.");
    // The setup is not offered beside it: a new run starts once this one is finished or given up.
    expect(screen.queryByTestId(dungeonTestid.startRun)).toBeNull();
    expect(screen.queryByTestId(dungeonTestid.seedDaily)).toBeNull();
    // The Embers stay in reach.
    expect(byId(dungeonTestid.embers)).toBeInTheDocument();

    fireEvent.click(byId(dungeonTestid.continueRun));
    expect(onEnter).toHaveBeenCalledTimes(1);
    expect(abandon).not.toHaveBeenCalled();
    fireEvent.click(byId(dungeonTestid.abandonRun));
    expect(abandon).toHaveBeenCalledTimes(1);
    expect(onEnter).toHaveBeenCalledTimes(1);
  });

  it("B19 a finished run is no run in progress", () => {
    show(dungeonOf({ run: runState({ phase: { phase: "dead" } }) }));

    expect(screen.queryByTestId(dungeonTestid.saved)).toBeNull();
    expect(byId(dungeonTestid.startRun)).toBeInTheDocument();
  });
});
