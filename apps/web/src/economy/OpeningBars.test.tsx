// The opening's two bars and the reward reveal (#1658, R1953000), from crafted answers: both bars
// pinned at the top as a HUD, unmoved ones included; each spent slot's points flying into it, labeled
// with the server's points and tinted by rarity; then each bar filling from its points before to its
// points after, a full one pulsing as it bursts and revealing what its fill paid; and the reveal
// flying its items out one by one with the pack opening's sting, or all at once with one sting under
// reduced motion, which flies and pulses nothing. The stylesheet is read as text, as
// `FillBar.test.tsx` reads it, because Vitest stubs CSS imports. The audio engine is the recording
// fake the pack scene's tests use; fake timers move everything.

import { act, cleanup, render, screen } from "@testing-library/react";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { setAudioEngineForTests } from "../audio/engine.ts";
import { fakeAudioEngine, type FakeAudioEngine } from "../audio/test/fakeAudio.ts";
import { resetFxSettingsForTests, setFxSettings } from "../fx/settings.ts";
import type { OpeningBarsAnswer } from "../net/api.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import {
  BAR_FLIGHT_MS,
  BAR_FLIGHT_STAGGER_MS,
  BURST_MS,
  REVEAL_FLY_MS,
  REVEAL_STAGGER_MS,
  flightsMs,
  flightsOf,
  segmentMs,
  type BarFlight,
  type RevealItem,
} from "./bars.ts";
import OpeningBars from "./OpeningBars.tsx";
import RewardReveal from "./RewardReveal.tsx";

let engine: FakeAudioEngine;

beforeEach(() => {
  vi.useFakeTimers();
  engine = fakeAudioEngine();
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

/**
 * An opening whose spent slots, the second and the fourth, gave each bar 5 and 10 points: the card
 * bar paid once and the cosmetic bar paid one Epic token.
 */
function paid(over: Partial<OpeningBarsAnswer> = {}): OpeningBarsAnswer {
  return {
    card: { before: 18, after: 13, size: 20, pays: 1 },
    cosmetic: { before: 145, after: 10, size: 150, pays: 1 },
    cosmeticPaid: ["epic"],
    slotPoints: [0, 5, 0, 10],
    ...over,
  };
}

/** An opening that moved neither bar: no slot was spent. */
function still(): OpeningBarsAnswer {
  return {
    card: { before: 5, after: 5, size: 20, pays: 0 },
    cosmetic: { before: 30, after: 30, size: 150, pays: 0 },
    cosmeticPaid: [],
    slotPoints: [0, 0, 0, 0],
  };
}

/** The slots' rarities, the spent ones a Legendary and a Mythic. */
const RARITIES = ["Common", "Legendary", "Rare", "Mythic"] as const;

/** An answer's flights, each from the slot `slot-<index>`. */
function flightsFor(bars: OpeningBarsAnswer): BarFlight[] {
  return flightsOf(bars, RARITIES, (index) => `slot-${String(index)}`);
}

/** The rewards on screen, by kind. */
function kinds(): (string | null)[] {
  return Array.from(screen.getByTestId(`${ID}-rewards`).querySelectorAll("li")).map((item) => item.getAttribute("data-kind"));
}

/** A bar's native progress, where it rests for screen readers. */
function progress(name: "card" | "cosmetic"): number | undefined {
  return screen.getByTestId(`${ID}-${name}`).querySelector("progress")?.value;
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
    expect(card.querySelector("progress")).toHaveProperty("value", 13);
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
          card: { before: 5, after: 215, size: 300, pays: 0 },
          cosmetic: { before: 100, after: 10, size: 150, pays: 2 },
          cosmeticPaid: ["rare", "cosmeticPack"],
          slotPoints: [0, 10, 0, 200],
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

  it("R2728000 the cosmetic bar waits on its step's size, fills each step at its own and reveals a Legendary token", () => {
    const bars = paid({
      card: { before: 5, after: 9, size: 20, pays: 0 },
      cosmetic: { before: 60, after: 10, size: 50, pays: 2 },
      cosmeticPaid: ["epic", "legendary"],
      cosmeticSizes: [75, 100, 50],
    });
    const { rerender } = render(<OpeningBars bars={bars} play={false} testid={ID} />);
    const cosmetic = (): HTMLElement => screen.getByTestId(`${ID}-cosmetic`);

    // Held back, it rests at its points before on the bar of the step it stood at.
    expect(cosmetic().querySelector("progress")).toHaveProperty("max", 75);
    expect(cosmetic()).toHaveTextContent("60 / 75");

    rerender(<OpeningBars bars={bars} testid={ID} />);
    advance(segmentMs({ from: 60, to: 75, burst: true }, 75));
    advance(REVEAL_STAGGER_MS);
    expect(screen.getByTestId(`${ID}-rewards-cosmetic-1`)).toHaveTextContent("Epic token");
    advance(BURST_MS);
    advance(segmentMs({ from: 0, to: 100, burst: true }, 100));
    advance(REVEAL_STAGGER_MS);

    expect(kinds()).toEqual(["cosmeticToken", "cosmeticToken"]);
    const legendary = screen.getByTestId(`${ID}-rewards-cosmetic-2`);
    expect(legendary).toHaveTextContent("Legendary token");
    expect(legendary).toHaveAttribute("data-rarity", "Legendary");
    expect(engine.playSfx).toHaveBeenCalledWith("entrance");

    // It rests at the answer's points after, on the step after's bar.
    advance(BURST_MS);
    advance(segmentMs({ from: 0, to: 10, burst: false }, 50));
    expect(cosmetic()).toHaveAttribute("data-state", "resting");
    expect(cosmetic()).toHaveAttribute("data-bursts", "2");
    expect(cosmetic().querySelector("progress")).toHaveProperty("max", 50);
    expect(progress("cosmetic")).toBe(10);
  });

  it("bars that moved without paying fill to their points and reveal nothing", () => {
    render(
      <OpeningBars
        bars={paid({
          card: { before: 5, after: 9, size: 20, pays: 0 },
          cosmetic: { before: 30, after: 34, size: 150, pays: 0 },
          cosmeticPaid: [],
          slotPoints: [0, 4, 0, 0],
        })}
        testid={ID}
      />,
    );

    const card = screen.getByTestId(`${ID}-card`);
    const cosmetic = screen.getByTestId(`${ID}-cosmetic`);
    advance(segmentMs({ from: 5, to: 9, burst: false }, 20));
    advance(60_000);

    for (const bar of [card, cosmetic]) {
      expect(bar).toHaveAttribute("data-state", "resting");
      expect(bar).toHaveAttribute("data-bursts", "0");
    }
    expect(progress("cosmetic")).toBe(34);
    expect(kinds()).toEqual([]);
    expect(engine.playSfx).not.toHaveBeenCalled();
  });

  it("a new answer plays from its start and shows its own rewards, not the last one's", () => {
    const { rerender } = render(<OpeningBars bars={paid()} testid={ID} />);
    playOut();
    expect(kinds()).toHaveLength(2);

    rerender(<OpeningBars bars={paid({ cosmetic: { before: 30, after: 45, size: 150, pays: 0 }, cosmeticPaid: [] })} testid={ID} />);
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
    expect(progress("card")).toBe(13);
    expect(kinds().sort()).toEqual(["cardToken", "cosmeticToken"]);
    // The strongest of the two stings, played once.
    expect(engine.playSfx).toHaveBeenCalledTimes(1);
    expect(engine.playSfx).toHaveBeenCalledWith("sting", { tier: "epic" });
  });
});

/* ------------------------------------------------------------------------ the stylesheet --- */

/** `economy.css` without its comments: vitest stubs CSS imports, so the file is read from the test's working directory. */
const sheet: string = (() => {
  for (const candidate of ["src/economy/economy.css", "apps/web/src/economy/economy.css"]) {
    const path = resolve(process.cwd(), candidate);
    if (existsSync(path)) return readFileSync(path, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
  }
  throw new Error(`economy.css not found from ${process.cwd()}`);
})();

/** The declarations of the sheet's rule for `selector`, whether at the top or in the motion gate. */
function ruleBody(selector: string): string {
  const at = sheet.indexOf(`${selector} {`);
  expect(at, selector).toBeGreaterThanOrEqual(0);
  return sheet.slice(sheet.indexOf("{", at) + 1, sheet.indexOf("}", at));
}

describe("#1953 the opening's HUD", () => {
  /** The HUD with its answer, held back so a test can lay out what it measures, then played. */
  function heldThenPlayed(bars: OpeningBarsAnswer): void {
    const { rerender } = render(
      <>
        <div data-testid="slot-1" />
        <OpeningBars bars={bars} flights={flightsFor(bars)} play={false} testid={ID} />
      </>,
    );
    screen.getByTestId("slot-1").getBoundingClientRect = () => ({ left: 100, top: 400, width: 100, height: 140 }) as DOMRect;
    screen.getByTestId(ID).getBoundingClientRect = () => ({ left: 300, top: 0, width: 400, height: 120 }) as DOMRect;
    rerender(
      <>
        <div data-testid="slot-1" />
        <OpeningBars bars={bars} flights={flightsFor(bars)} play testid={ID} />
      </>,
    );
  }

  it("R1953000 renders both bars in the HUD pinned at the top", () => {
    render(<OpeningBars bars={still()} flights={flightsFor(still())} testid={ID} />);

    // An opening that moved neither bar still shows both, at rest, with nothing to fly.
    const hud = screen.getByTestId(ID);
    expect(hud).toHaveClass("opening-bars", "opening-hud");
    expect(screen.getByTestId(`${ID}-card`)).toHaveTextContent("Card token bar");
    expect(screen.getByTestId(`${ID}-card`)).toHaveTextContent("5 / 20");
    expect(screen.getByTestId(`${ID}-cosmetic`)).toHaveTextContent("Cosmetic token bar");
    expect(screen.getByTestId(`${ID}-cosmetic`)).toHaveTextContent("30 / 150");
    expect(hud.querySelector(".bar-flight")).toBeNull();
    advance(60_000);
    expect(screen.getByTestId(`${ID}-card`)).toHaveAttribute("data-state", "resting");
    expect(engine.playSfx).not.toHaveBeenCalled();

    // Pinned across the top of the viewport, with no transform or filter to hold its flights.
    const body = ruleBody(".opening-hud");
    expect(body).toMatch(/position:\s*fixed/);
    expect(body).toMatch(/(^|;)\s*top:\s*0\s*;/);
    expect(body).not.toMatch(/transform|filter/);
  });

  it("R1953000 plays one flight per spent duplicate with the server's points", () => {
    heldThenPlayed(paid());

    const hud = screen.getByTestId(ID);
    expect(hud).toHaveAttribute("data-motion", "full");
    expect(hud).toHaveAttribute("data-phase", "flights");
    expect(hud.style.getPropertyValue("--flight-ms")).toBe(`${String(BAR_FLIGHT_MS)}ms`);
    // One flight for each slot the server gave points, its label the server's number, none for the others.
    expect(hud.querySelectorAll(".bar-flight")).toHaveLength(2);
    expect(screen.queryByTestId(`${ID}-flight-0`)).toBeNull();
    expect(screen.queryByTestId(`${ID}-flight-2`)).toBeNull();
    const five = screen.getByTestId(`${ID}-flight-1`);
    const ten = screen.getByTestId(`${ID}-flight-3`);
    expect(five).toHaveTextContent(/^\+5$/);
    expect(five).toHaveAttribute("data-rarity", "Legendary");
    expect(ten).toHaveTextContent(/^\+10$/);
    expect(ten).toHaveAttribute("data-rarity", "Mythic");
    // Each takes off from its slot's center for the HUD's, one stagger after the one before.
    expect(five.style.getPropertyValue("--flight-x")).toBe("150px");
    expect(five.style.getPropertyValue("--flight-y")).toBe("470px");
    expect(five.style.getPropertyValue("--flight-dx")).toBe("350px");
    expect(five.style.getPropertyValue("--flight-dy")).toBe("-410px");
    expect(five.style.getPropertyValue("--flight-delay")).toBe("0ms");
    expect(ten.style.getPropertyValue("--flight-delay")).toBe(`${String(BAR_FLIGHT_STAGGER_MS)}ms`);
    // A slot not on the page sends its points from the HUD itself.
    expect(ten.style.getPropertyValue("--flight-x")).toBe("500px");
    expect(ten.style.getPropertyValue("--flight-dx")).toBe("0px");

    // The bars rest where the opening found them until the last flight lands.
    expect(progress("card")).toBe(18);
    expect(progress("cosmetic")).toBe(145);
    advance(flightsMs(2) - 1);
    expect(hud).toHaveAttribute("data-phase", "flights");
    expect(screen.getByTestId(`${ID}-card`)).toHaveAttribute("data-state", "resting");

    advance(1);
    expect(hud).toHaveAttribute("data-phase", "fill");
    expect(hud.querySelector(".bar-flight")).toBeNull();
    expect(screen.getByTestId(`${ID}-card`)).toHaveAttribute("data-state", "filling");
    expect(progress("card")).toBe(13);
    expect(progress("cosmetic")).toBe(10);
  });

  it("R1953000 a held-back opening rests at the points before and flies nothing until it plays", () => {
    render(<OpeningBars bars={paid()} flights={flightsFor(paid())} play={false} testid={ID} />);
    advance(60_000);

    expect(screen.getByTestId(ID)).toHaveAttribute("data-phase", "waiting");
    expect(screen.getByTestId(ID).querySelector(".bar-flight")).toBeNull();
    expect(screen.getByTestId(`${ID}-card`)).toHaveTextContent("18 / 20");
    expect(screen.getByTestId(`${ID}-cosmetic`)).toHaveTextContent("145 / 150");
    expect(kinds()).toEqual([]);
  });

  it("R1953000 a full bar pulses before its reward", () => {
    render(<OpeningBars bars={paid()} flights={flightsFor(paid())} testid={ID} />);
    advance(flightsMs(2));

    // The cosmetic bar reaches full first: it pulses and glows, and its reward has not flown out yet.
    advance(segmentMs({ from: 145, to: 150, burst: true }, 150));
    const hud = screen.getByTestId(ID);
    const cosmetic = screen.getByTestId(`${ID}-cosmetic`);
    expect(hud).toHaveAttribute("data-motion", "full");
    expect(hud.style.getPropertyValue("--pulse-ms")).toBe(`${String(BURST_MS)}ms`);
    expect(cosmetic).toHaveAttribute("data-state", "bursting");
    expect(cosmetic.querySelector(".fill-bar__burst")).not.toBeNull();
    expect(kinds()).toEqual([]);

    // Then the reward reveal plays, and then the bar carries its overflow over from 0.
    advance(REVEAL_STAGGER_MS);
    expect(kinds()).toEqual(["cosmeticToken"]);
    expect(cosmetic).toHaveAttribute("data-state", "bursting");
    advance(BURST_MS - REVEAL_STAGGER_MS);
    expect(cosmetic).toHaveAttribute("data-state", "filling");
    expect(cosmetic.querySelector<HTMLElement>(".fill-bar__fill")!.style.transform).toBe("scaleX(0)");

    // The pulse plays only on a bursting bar of a HUD in full motion, inside the gate of both switches.
    expect(ruleBody(':root:not([data-reduce-motion="true"]) .opening-hud[data-motion="full"] .fill-bar[data-state="bursting"] .fill-bar__track')).toMatch(
      /animation:\s*opening-hud-pulse\s+var\(--pulse-ms\)/,
    );
    expect(ruleBody(':root:not([data-reduce-motion="true"]) .bar-flight')).toMatch(/animation:\s*bar-flight\s+var\(--flight-ms\)/);
  });

  it("R1953000 reduced motion plays no flight or pulse, just the final fill", () => {
    // Reduce motion: the bars rest at the points after at once, every reward shown.
    writeSettings({ reduceMotion: true });
    render(<OpeningBars bars={paid()} flights={flightsFor(paid())} testid={ID} />);
    expect(screen.getByTestId(ID)).toHaveAttribute("data-motion", "still");
    expect(screen.getByTestId(ID)).toHaveAttribute("data-phase", "fill");
    expect(screen.getByTestId(ID).querySelector(".bar-flight")).toBeNull();
    expect(screen.getByTestId(`${ID}-card`)).toHaveAttribute("data-state", "resting");
    expect(progress("card")).toBe(13);
    expect(progress("cosmetic")).toBe(10);
    expect(kinds().sort()).toEqual(["cardToken", "cosmeticToken"]);
    cleanup();
    writeSettings({ reduceMotion: false });

    // The effects off: nothing flies, sweeps or pulses; the bars show their final fill at once.
    setFxSettings({ intensity: "off" });
    render(<OpeningBars bars={paid()} flights={flightsFor(paid())} testid={ID} />);
    const hud = screen.getByTestId(ID);
    expect(hud).toHaveAttribute("data-motion", "still");
    expect(hud).toHaveAttribute("data-phase", "fill");
    expect(hud.querySelector(".bar-flight")).toBeNull();
    for (const bar of ["card", "cosmetic"]) {
      expect(screen.getByTestId(`${ID}-${bar}`)).toHaveAttribute("data-state", "resting");
      expect(screen.getByTestId(`${ID}-${bar}`).querySelector(".fill-bar__burst")).toBeNull();
    }
    expect(progress("card")).toBe(13);
    expect(progress("cosmetic")).toBe(10);
    advance(segmentMs({ from: 145, to: 150, burst: true }, 150));
    expect(screen.getByTestId(`${ID}-cosmetic`)).toHaveAttribute("data-state", "resting");
    expect(screen.getByTestId(`${ID}-cosmetic`).querySelector(".fill-bar__burst")).toBeNull();
    // Every fill is reported at once; the reveal then shows its rewards in its own time.
    advance(REVEAL_STAGGER_MS);
    advance(REVEAL_STAGGER_MS);
    expect(kinds().sort()).toEqual(["cardToken", "cosmeticToken"]);
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
