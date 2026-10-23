// The opening's two bars and the reward reveal (#1658), from crafted answers: each bar fills from its
// points before to its points after and reveals what its fill paid as it bursts, a bar the opening did
// not move is not drawn, and the reveal flies its items out one by one with the pack opening's sting,
// or all at once with one sting under reduced motion. The audio engine is the recording fake the pack
// scene's tests use; fake timers move everything.

import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi, type Mock } from "vitest";

import { setAudioEngineForTests } from "../audio/engine.ts";
import type { AudioEngine } from "../audio/types.ts";
import { resetFxSettingsForTests } from "../fx/settings.ts";
import type { OpeningBarsAnswer } from "../net/api.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { BURST_MS, REVEAL_FLY_MS, REVEAL_STAGGER_MS, segmentMs, type RevealItem } from "./bars.ts";
import OpeningBars from "./OpeningBars.tsx";
import RewardReveal from "./RewardReveal.tsx";

type FakeEngine = { [K in keyof AudioEngine]: Mock<AudioEngine[K]> };

/** The same stand-in the pack scene's tests use: every member a spy, every sink call accepted. */
function fakeEngine(): FakeEngine {
  return {
    state: vi.fn<AudioEngine["state"]>(() => "running"),
    unlock: vi.fn<AudioEngine["unlock"]>(),
    preloadVoices: vi.fn<AudioEngine["preloadVoices"]>(),
    setBusy: vi.fn<AudioEngine["setBusy"]>(),
    log: vi.fn<AudioEngine["log"]>(() => []),
    clearLog: vi.fn<AudioEngine["clearLog"]>(),
    contextsCreated: vi.fn<AudioEngine["contextsCreated"]>(() => 0),
    speaking: vi.fn<AudioEngine["speaking"]>(() => false),
    subscribeSpeaking: vi.fn<AudioEngine["subscribeSpeaking"]>(() => () => undefined),
    musicOutput: vi.fn<AudioEngine["musicOutput"]>(() => null),
    subscribeState: vi.fn<AudioEngine["subscribeState"]>(() => () => undefined),
    dispose: vi.fn<AudioEngine["dispose"]>(),
    playSfx: vi.fn<AudioEngine["playSfx"]>(() => true),
    playVoice: vi.fn<AudioEngine["playVoice"]>(() => true),
    playEffect: vi.fn<AudioEngine["playEffect"]>(() => true),
    playPickup: vi.fn<AudioEngine["playPickup"]>(() => true),
  };
}

let engine: FakeEngine;

beforeEach(() => {
  vi.useFakeTimers();
  engine = fakeEngine();
  setAudioEngineForTests(engine);
  __resetSettingsForTests();
  resetFxSettingsForTests();
});

afterEach(() => {
  cleanup();
  setAudioEngineForTests(null);
  vi.restoreAllMocks();
  vi.useRealTimers();
  __resetSettingsForTests();
  resetFxSettingsForTests();
  window.localStorage.clear();
});

const ID = "opening";

function advance(ms: number): void {
  act(() => {
    vi.advanceTimersByTime(ms);
  });
}

/** An opening where the card bar paid once and the cosmetic bar paid one Epic token. */
function paid(over: Partial<OpeningBarsAnswer> = {}): OpeningBarsAnswer {
  return {
    card: { before: 18, after: 3, size: 20, pays: 1 },
    cosmetic: { before: 140, after: 10, size: 150, pays: 1 },
    cosmeticPaid: ["epic"],
    ...over,
  };
}

/** An opening that moved neither bar. */
function still(): OpeningBarsAnswer {
  return {
    card: { before: 5, after: 5, size: 20, pays: 0 },
    cosmetic: { before: 30, after: 30, size: 150, pays: 0 },
    cosmeticPaid: [],
  };
}

/** The rewards on screen, by kind. */
function kinds(): (string | null)[] {
  return Array.from(screen.getByTestId(`${ID}-rewards`).querySelectorAll("li")).map((item) => item.getAttribute("data-kind"));
}

/** Both bars' bursts, then the time every reward waits for its turn. */
function playOut(): void {
  advance(segmentMs({ from: 18, to: 20, burst: true }, 20));
  advance(REVEAL_STAGGER_MS);
  advance(REVEAL_STAGGER_MS);
}

describe("#1658 the opening's two bars", () => {
  it("draws both bars from the answer, each starting where the opening found it", () => {
    render(<OpeningBars bars={paid()} testid={ID} />);

    const card = screen.getByTestId(`${ID}-card`);
    const cosmetic = screen.getByTestId(`${ID}-cosmetic`);
    expect(card).toHaveTextContent("Card token bar");
    expect(card.querySelector("progress")).toHaveProperty("value", 3);
    expect(card.querySelector("progress")).toHaveProperty("max", 20);
    expect(card).toHaveAttribute("data-state", "filling");
    expect(cosmetic).toHaveTextContent("Cosmetic token bar");
    expect(cosmetic.querySelector("progress")).toHaveProperty("value", 10);
    expect(cosmetic.querySelector("progress")).toHaveProperty("max", 150);
    expect(kinds()).toEqual([]);
  });

  it("with card.pays 1 and cosmeticPaid ['epic'] it reveals a card token and an Epic cosmetic token", () => {
    render(<OpeningBars bars={paid()} testid={ID} />);

    playOut();

    expect(screen.getByTestId(`${ID}-card`)).toHaveAttribute("data-bursts", "1");
    expect(screen.getByTestId(`${ID}-cosmetic`)).toHaveAttribute("data-bursts", "1");
    expect(kinds().sort()).toEqual(["cardToken", "cosmeticToken"]);
    const card = screen.getByTestId(`${ID}-rewards-card-1`);
    expect(card).toHaveTextContent("Card token");
    const epic = screen.getByTestId(`${ID}-rewards-cosmetic-1`);
    expect(epic).toHaveTextContent("Epic token");
    expect(epic).toHaveAttribute("data-rarity", "Epic");
    // Each flew out with its sting: the Epic one's tier and the card token's.
    expect(engine.playSfx).toHaveBeenCalledWith("sting", { tier: "epic" });
    expect(engine.playSfx).toHaveBeenCalledWith("sting", { tier: "rare" });
    expect(engine.playSfx).toHaveBeenCalledTimes(2);
  });

  it("a Rare token and a cosmetic pack are what the cosmetic bar's next fills reveal", () => {
    render(
      <OpeningBars
        bars={paid({
          card: { before: 5, after: 9, size: 20, pays: 0 },
          cosmetic: { before: 100, after: 10, size: 150, pays: 2 },
          cosmeticPaid: ["rare", "cosmeticPack"],
        })}
        testid={ID}
      />,
    );

    advance(segmentMs({ from: 100, to: 150, burst: true }, 150));
    advance(REVEAL_STAGGER_MS);
    expect(screen.getByTestId(`${ID}-rewards-cosmetic-1`)).toHaveTextContent("Rare token");
    advance(BURST_MS);
    advance(segmentMs({ from: 0, to: 150, burst: true }, 150));
    advance(REVEAL_STAGGER_MS);

    expect(kinds()).toEqual(["cosmeticToken", "cosmeticPack"]);
    expect(screen.getByTestId(`${ID}-rewards-cosmetic-2`)).toHaveTextContent("Cosmetic pack");
  });

  it("a bar that moved without paying fills to its points and reveals nothing", () => {
    render(<OpeningBars bars={paid({ card: { before: 5, after: 9, size: 20, pays: 0 }, cosmetic: still().cosmetic, cosmeticPaid: [] })} testid={ID} />);

    expect(screen.queryByTestId(`${ID}-cosmetic`)).toBeNull();
    const card = screen.getByTestId(`${ID}-card`);
    advance(segmentMs({ from: 5, to: 9, burst: false }, 20));
    advance(60_000);

    expect(card).toHaveAttribute("data-state", "resting");
    expect(card).toHaveAttribute("data-bursts", "0");
    expect(kinds()).toEqual([]);
    expect(engine.playSfx).not.toHaveBeenCalled();
  });

  it("a bar that paid and kept the same points is drawn all the same", () => {
    render(
      <OpeningBars
        bars={paid({ card: { before: 5, after: 5, size: 20, pays: 1 }, cosmetic: still().cosmetic, cosmeticPaid: [] })}
        testid={ID}
      />,
    );
    expect(screen.getByTestId(`${ID}-card`)).toBeInTheDocument();
    expect(screen.queryByTestId(`${ID}-cosmetic`)).toBeNull();
  });

  it("draws nothing for an answer where neither bar moved", () => {
    const { container } = render(<OpeningBars bars={still()} testid={ID} />);

    expect(container).toBeEmptyDOMElement();
    expect(screen.queryByTestId(ID)).toBeNull();
    advance(60_000);
    expect(engine.playSfx).not.toHaveBeenCalled();
  });

  it("a new answer plays from its start and shows its own rewards, not the last one's", () => {
    const { rerender } = render(<OpeningBars bars={paid()} testid={ID} />);
    playOut();
    expect(kinds()).toHaveLength(2);

    rerender(<OpeningBars bars={paid({ cosmetic: still().cosmetic, cosmeticPaid: [] })} testid={ID} />);
    expect(kinds()).toEqual([]);
    expect(screen.getByTestId(`${ID}-card`)).toHaveAttribute("data-state", "filling");

    advance(segmentMs({ from: 18, to: 20, burst: true }, 20));
    advance(REVEAL_STAGGER_MS);
    expect(kinds()).toEqual(["cardToken"]);
  });

  it("under Reduce motion both bars show their points and every reward shows at once with one sting", () => {
    writeSettings({ reduceMotion: true });
    render(<OpeningBars bars={paid()} testid={ID} />);

    expect(screen.getByTestId(`${ID}-card`)).toHaveAttribute("data-state", "resting");
    expect(screen.getByTestId(`${ID}-cosmetic`)).toHaveAttribute("data-state", "resting");
    expect(screen.getByTestId(`${ID}-card`).querySelector("progress")).toHaveProperty("value", 3);
    expect(kinds().sort()).toEqual(["cardToken", "cosmeticToken"]);
    // The strongest of the two stings, played once.
    expect(engine.playSfx).toHaveBeenCalledTimes(1);
    expect(engine.playSfx).toHaveBeenCalledWith("sting", { tier: "epic" });
  });
});

describe("#1658 the reward reveal", () => {
  const ITEMS: readonly RevealItem[] = [
    { key: "a", kind: "cardToken", label: "Card token" },
    { key: "b", kind: "cosmeticToken", label: "Epic token", rarity: "Epic" },
    { key: "c", kind: "jlore", label: "100 Jlore" },
  ];

  function shownKeys(): string[] {
    return Array.from(screen.getByTestId("reveal").querySelectorAll("li")).map((item) => item.textContent ?? "");
  }

  it("flies the items out one by one in the order given, each with its sting, and is done after the last", () => {
    const onDone = vi.fn<() => void>();
    render(<RewardReveal items={ITEMS} testid="reveal" onDone={onDone} />);
    expect(shownKeys()).toEqual([]);

    advance(REVEAL_STAGGER_MS);
    expect(shownKeys()).toEqual(["Card token"]);
    expect(engine.playSfx).toHaveBeenLastCalledWith("sting", { tier: "rare" });

    advance(REVEAL_STAGGER_MS);
    expect(shownKeys()).toEqual(["Card token", "Epic token"]);
    expect(engine.playSfx).toHaveBeenLastCalledWith("sting", { tier: "epic" });

    advance(REVEAL_STAGGER_MS);
    expect(shownKeys()).toEqual(["Card token", "Epic token", "100 Jlore"]);
    expect(engine.playSfx).toHaveBeenLastCalledWith("sting");
    expect(engine.playSfx).toHaveBeenCalledTimes(3);
    expect(onDone).not.toHaveBeenCalled();

    advance(REVEAL_FLY_MS);
    expect(onDone).toHaveBeenCalledTimes(1);
    advance(60_000);
    expect(onDone).toHaveBeenCalledTimes(1);
    expect(engine.playSfx).toHaveBeenCalledTimes(3);
  });

  it("an item handed in later flies out in turn, and a key shown once never flies out again", () => {
    const onDone = vi.fn<() => void>();
    const { rerender } = render(<RewardReveal items={ITEMS.slice(0, 1)} testid="reveal" onDone={onDone} />);
    advance(REVEAL_STAGGER_MS);
    advance(REVEAL_FLY_MS);
    expect(onDone).toHaveBeenCalledTimes(1);

    rerender(<RewardReveal items={ITEMS.slice(0, 2)} testid="reveal" onDone={onDone} />);
    expect(shownKeys()).toEqual(["Card token"]);
    advance(REVEAL_STAGGER_MS);
    expect(shownKeys()).toEqual(["Card token", "Epic token"]);
    advance(REVEAL_FLY_MS);

    expect(onDone).toHaveBeenCalledTimes(2);
    expect(engine.playSfx).toHaveBeenCalledTimes(2);
  });

  it("an item handed in before the last one's flight ends holds done back until it too has flown", () => {
    const onDone = vi.fn<() => void>();
    const { rerender } = render(<RewardReveal items={ITEMS.slice(0, 1)} testid="reveal" onDone={onDone} />);
    advance(REVEAL_STAGGER_MS);
    advance(REVEAL_FLY_MS - 1);
    rerender(<RewardReveal items={ITEMS.slice(0, 2)} testid="reveal" onDone={onDone} />);
    advance(1);
    expect(onDone).not.toHaveBeenCalled();

    advance(REVEAL_STAGGER_MS);
    advance(REVEAL_FLY_MS);
    expect(onDone).toHaveBeenCalledTimes(1);
  });

  it("no items is an empty list, with no sting and no done", () => {
    const onDone = vi.fn<() => void>();
    render(<RewardReveal items={[]} testid="reveal" onDone={onDone} />);
    advance(60_000);
    expect(shownKeys()).toEqual([]);
    expect(engine.playSfx).not.toHaveBeenCalled();
    expect(onDone).not.toHaveBeenCalled();
  });

  it("under Reduce motion every item shows at once with the strongest one's sting, and onDone is called", () => {
    writeSettings({ reduceMotion: true });
    const onDone = vi.fn<() => void>();
    render(<RewardReveal items={ITEMS} testid="reveal" onDone={onDone} />);

    expect(shownKeys()).toEqual(["Card token", "Epic token", "100 Jlore"]);
    expect(engine.playSfx).toHaveBeenCalledTimes(1);
    expect(engine.playSfx).toHaveBeenCalledWith("sting", { tier: "epic" });
    expect(onDone).toHaveBeenCalledTimes(1);

    advance(60_000);
    expect(engine.playSfx).toHaveBeenCalledTimes(1);
    expect(onDone).toHaveBeenCalledTimes(1);
  });
});
