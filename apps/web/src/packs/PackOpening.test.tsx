// The pack-opening scene and the card packs panel (#1308, SPEC §14.3, §14.4), from fixture responses:
// each slot's outcome, the packs held on the panel before and after an opening, skip-all, reduced
// motion, the pack object, the token bars pinned at the top from the answer and played once every card
// shows (#1663, R1953000, R1999000), batch opening, a refused open, the page told of an opening and asking
// for a fresh read (#1309), and every press waiting while another spend of the tab is out (R2378000).
// Nothing here rolls a pack: the fixtures are what `GET /api/packs` and
// `POST /api/packs/open` answer, and the audio engine is the recording fake the audio suite's own
// wiring test uses.

import { CATALOG } from "@jackioh/cards";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { GLOBAL_PACK_NAME } from "@jackioh/server-config";

import { setAudioEngineForTests } from "../audio/engine.ts";
import { fakeAudioEngine, type FakeAudioEngine } from "../audio/test/fakeAudio.ts";
import {
  DEFAULT_CARD_COSMETICS,
  readCardPicks,
  readCosmetics,
  resetCosmeticsForTests,
  writeCardFullArt,
  writeCardPick,
  writeCosmetics,
} from "../cards/cosmetics.ts";
import { flightsMs } from "../economy/bars.ts";
import { resetFxSettingsForTests } from "../fx/settings.ts";
import { resetLocaleForTests, setLocale } from "../i18n/i18n.ts";
import type { OpenedPackResponse, PacksResponse } from "../net/api.ts";
import { whileSpending } from "../net/spending.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { expectEmptyState } from "../test/empty.ts";
import { expectSkeleton } from "../test/skeleton.ts";
import { refusal, stubServer, type Reply } from "../test/stubServer.ts";
import { openedPack, packsInfo } from "./fixtures.ts";
import PackOpening, { PackScene, type PackSceneProps } from "./PackOpening.tsx";
import {
  PACK_BURSTS,
  PACK_HOLD_MS,
  PACK_SWIPE_PX,
  PACK_TEAR_MS,
  bestRarity,
  cosmeticName,
  holdsToFlip,
  packTestid as T,
  percent,
  playStinger,
  revealCues,
  slotOutcome,
  withOpening,
} from "./packs.ts";

const TOKEN = "token-1";
const NO_PACK = "no unopened pack of that kind";
const GARY = "Gary the Gambler full art";
/** What the fixture's spent duplicate, slot 3, says: the 4 points the server gave each bar. */
const SPENT = "Duplicate: +4 card token bar, +4 cosmetic token bar";

let engine: FakeAudioEngine;

beforeEach(() => {
  engine = fakeAudioEngine();
  setAudioEngineForTests(engine);
  __resetSettingsForTests();
  resetFxSettingsForTests();
});

afterEach(() => {
  cleanup();
  setAudioEngineForTests(null);
  __resetSettingsForTests();
  resetFxSettingsForTests();
  resetLocaleForTests();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  try {
    window.localStorage.clear();
  } catch {
    // Storage is optional.
  }
});

describe("#1308 the pack model", () => {
  it("percent says a rate to a tenth of a point", () => {
    expect(percent(0.29)).toBe("29%");
    expect(percent(0.082)).toBe("8.2%");
    expect(percent(0.005)).toBe("0.5%");
  });

  it("the best rarity is the highest slot's, and Epic and above hold to flip", () => {
    const { slots } = openedPack();
    expect(bestRarity(slots)).toBe("Mythic");
    expect(bestRarity(slots.slice(1))).toBe("Epic");
    expect(bestRarity(slots.slice(2, 4))).toBe("Rare");
    expect(bestRarity(slots.slice(4))).toBe("Common");
    expect(bestRarity([])).toBe("Common");
    expect(["Common", "Rare", "Epic", "Legendary", "Mythic"].map((rarity) => holdsToFlip(rarity as "Common"))).toEqual([
      false,
      false,
      true,
      true,
      true,
    ]);
  });

  it("each grant says what the slot became", () => {
    const [, , cosmetic, , card] = openedPack().slots;
    expect(slotOutcome(card!, 0)).toBe("New card");
    expect(slotOutcome(cosmetic!, 0)).toBe(`New cosmetic: ${GARY}`);
    expect(cosmeticName("full-art-gary-the-gambler")).toBe(GARY);
    expect(cosmeticName("not-in-the-registry")).toBe("not-in-the-registry");
    expect(
      slotOutcome(
        {
          rarity: "Common",
          cardId: "core-004",
          grant: { kind: "cosmetic", cosmeticId: "not-in-the-registry", cosmeticRarity: "Common" },
        },
        0,
      ),
    ).toBe("New cosmetic: not-in-the-registry");
  });

  it("R1953000 a spent duplicate says the points the server gave each bar", () => {
    const bars = openedPack().slots[3]!;
    expect(slotOutcome(bars, 4)).toBe(SPENT);
    expect(slotOutcome(bars, 10)).toBe("Duplicate: +10 card token bar, +10 cosmetic token bar");
  });

  it("an opening moves the panel to the server's counters and count", () => {
    const info = packsInfo();
    const after = withOpening(info, openedPack());
    expect(after.pity).toEqual({ epic: 0, legendary: 14, mythic: 0 });
    expect(after.unopened).toEqual([
      { pack: GLOBAL_PACK_NAME, count: 1 },
      { pack: "Core", count: 1 },
    ]);
    expect(after.odds).toEqual(info.odds);
    expect(info.pity, "the response it was given is not changed").toEqual({ epic: 7, legendary: 13, mythic: 21 });

    const last = withOpening(after, { ...openedPack(), unopened: 0 });
    expect(last.unopened).toEqual([{ pack: GLOBAL_PACK_NAME, count: 1 }]);
  });

  it("a reveal's effects grow with its rarity", () => {
    const at = { kind: "testid", testid: "pack-slot-0" };
    expect(revealCues("Common", "pack-slot-0", 1)).toEqual([]);

    const rare = revealCues("Rare", "pack-slot-0", 1);
    expect(rare).toEqual([
      { kind: "burst", preset: "sparkle", at, delayMs: 0, count: PACK_BURSTS.Rare.count, spread: "area", power: PACK_BURSTS.Rare.power },
    ]);
    const epic = revealCues("Epic", "pack-slot-0", 1);
    expect(epic.map((cue) => cue.kind === "burst" && cue.preset)).toEqual(["arcane"]);

    const legendary = revealCues("Legendary", "pack-slot-0", 1);
    expect(legendary.map((cue) => cue.kind)).toEqual(["rays", "burst"]);
    expect(legendary[0]).toMatchObject({ kind: "rays", tone: "legendary", at });
    expect(legendary[1]).toMatchObject({ kind: "burst", preset: "gold" });
    const mythic = revealCues("Mythic", "pack-slot-0", 1);
    expect(mythic[0]).toMatchObject({ kind: "rays", tone: "mythic" });
    expect(mythic[1]).toMatchObject({ kind: "burst", preset: "prismatic" });

    const counts = [rare, epic, legendary, mythic].map((cues) => {
      const burst = cues.find((cue) => cue.kind === "burst");
      return burst?.kind === "burst" ? burst.count : 0;
    });
    expect(counts).toEqual([PACK_BURSTS.Rare.count, PACK_BURSTS.Epic.count, PACK_BURSTS.Legendary.count, PACK_BURSTS.Mythic.count]);
    expect([...counts].sort((a, b) => a - b)).toEqual(counts);

    const half = revealCues("Epic", "pack-slot-0", 0.5)[0];
    expect(half?.kind === "burst" ? half.count : 0, "the count follows the intensity").toBe(PACK_BURSTS.Epic.count / 2);
  });

  it("each rarity plays its stinger", () => {
    for (const rarity of ["Common", "Rare", "Epic", "Legendary", "Mythic"] as const) playStinger(rarity);
    expect(engine.playSfx.mock.calls).toEqual([
      ["sting"],
      ["sting", { tier: "rare" }],
      ["sting", { tier: "epic" }],
      ["entrance"],
      ["entrance", { mythic: true }],
    ]);
  });
});

function props(over: Partial<PackSceneProps> = {}): PackSceneProps {
  return {
    pack: "Core",
    opened: null,
    busy: false,
    error: null,
    left: 0,
    onTear: vi.fn(),
    onNext: vi.fn(),
    onClose: vi.fn(),
    ...over,
  };
}

function advance(ms: number): void {
  act(() => {
    vi.advanceTimersByTime(ms);
  });
}

/** The scene with the server's answer in, its pack torn open and its five cards face down. */
function torn(over: Partial<PackSceneProps> = {}): PackSceneProps {
  const given = props({ opened: openedPack(), ...over });
  render(<PackScene {...given} />);
  advance(PACK_TEAR_MS);
  return given;
}

const revealed = (index: number): boolean => screen.queryByTestId(T.outcome(index)) !== null;

describe("#1308 the pack-opening scene", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  it("the sealed pack opens on a tap or a swipe", () => {
    const given = props();
    render(<PackScene {...given} />);
    const pack = screen.getByTestId(T.pack);
    expect(pack).toHaveAttribute("data-state", "sealed");
    expect(pack).toHaveTextContent("Swipe or tap to open");

    fireEvent.click(pack);
    expect(given.onTear).toHaveBeenCalledTimes(1);
    fireEvent.pointerDown(pack, { clientX: 0, pointerId: 1 });
    fireEvent.pointerUp(pack, { clientX: PACK_SWIPE_PX, pointerId: 1 });
    expect(given.onTear).toHaveBeenCalledTimes(2);
    fireEvent.pointerDown(pack, { clientX: 0, pointerId: 1 });
    fireEvent.pointerUp(pack, { clientX: PACK_SWIPE_PX - 1, pointerId: 1 });
    expect(given.onTear, "a short drag is not a swipe").toHaveBeenCalledTimes(2);
    expect(screen.queryByTestId(T.slots)).toBeNull();
  });

  it("the sealed pack waits for the server: busy, it says so and cannot be torn again", () => {
    const given = props({ busy: true });
    render(<PackScene {...given} />);
    const pack = screen.getByTestId(T.pack);
    expect(pack).toBeDisabled();
    expect(pack).toHaveAttribute("data-state", "sealed");
    expect(pack).toHaveTextContent("Opening…");
    expect(screen.getByTestId(T.close)).toBeDisabled();
    expect(screen.getByTestId(T.skipAll)).toBeDisabled();
  });

  it("the best rarity tints the pack's glow before the reveal", () => {
    render(<PackScene {...props({ opened: openedPack() })} />);
    const pack = screen.getByTestId(T.pack);
    expect(pack).toHaveAttribute("data-state", "tearing");
    expect(pack).toHaveAttribute("data-best", "Mythic");
    expect(screen.queryByTestId(T.slots)).toBeNull();

    advance(PACK_TEAR_MS);
    expect(screen.queryByTestId(T.pack)).toBeNull();
    for (let index = 0; index < 5; index += 1) {
      expect(screen.getByTestId(T.slot(index))).toHaveAttribute("data-revealed", "false");
      expect(screen.getByTestId(T.flip(index))).toBeInTheDocument();
    }
    expect(screen.getByTestId(T.slot(0))).toHaveAttribute("data-rarity", "Mythic");
  });

  it("cards are revealed one at a time, each saying what it became", () => {
    torn();
    fireEvent.click(screen.getByTestId(T.flip(4)));
    expect(revealed(4)).toBe(true);
    expect([0, 1, 2, 3].some(revealed), "only the card turned shows").toBe(false);
    expect(screen.getByTestId(T.outcome(4))).toHaveTextContent("New card");
    expect(screen.getByTestId(T.slot(4))).toHaveTextContent(CATALOG["core-002"]!.name);
    expect(engine.playSfx).toHaveBeenCalledWith("sting");

    fireEvent.click(screen.getByTestId(T.flip(3)));
    // R1953000: the server's points for the slot, from the opening's answer.
    expect(screen.getByTestId(T.outcome(3))).toHaveTextContent(SPENT);
    expect(engine.playSfx).toHaveBeenCalledWith("sting", { tier: "rare" });

    fireEvent.click(screen.getByTestId(T.flip(2)));
    expect(screen.getByTestId(T.outcome(2))).toHaveTextContent(`New cosmetic: ${GARY}`);
    expect(screen.getByTestId(T.slot(2))).toHaveTextContent(CATALOG["core-004"]!.name);
    expect(screen.getByTestId(T.slot(2))).toHaveAttribute("data-grant", "cosmetic");
    expect(screen.queryByTestId(T.next), "cards are still face down").toBeNull();
    expect(screen.getByTestId(T.close)).toHaveTextContent("Close");
  });

  it("R1827000 a slot that dropped a portrait shows its name and its art", () => {
    const opened = openedPack();
    const [first, second, third, ...rest] = opened.slots;
    torn({
      opened: {
        ...opened,
        slots: [
          first!,
          second!,
          { ...third!, grant: { kind: "cosmetic", cosmeticId: "portrait-gary", cosmeticRarity: "Epic" } },
          ...rest,
        ],
      },
    });
    fireEvent.click(screen.getByTestId(T.flip(2)));
    expect(screen.getByTestId(T.outcome(2))).toHaveTextContent("New cosmetic: Gary the Gambler portrait");
    const slot = screen.getByTestId(T.slot(2));
    const art = slot.querySelector('.pack-slot__face [data-portrait-art="gary"]');
    expect(art).not.toBeNull();
    expect(art).toHaveClass("pack-slot__portrait");
    // The portrait is in the slot card's place: an account cosmetic sits on no card.
    expect(slot.querySelector(".pack-slot__face .cf")).toBeNull();
  });

  it("R1833000 a slot that dropped a new chase shows its name, rarity and look", () => {
    const opened = openedPack();
    const [first, second, third, fourth, ...rest] = opened.slots;
    torn({
      opened: {
        ...opened,
        slots: [
          first!,
          second!,
          // A Mythic foil on the slot's card, and a Legendary card back on the account.
          { ...third!, grant: { kind: "cosmetic", cosmeticId: "foil-void-shimmer", cosmeticRarity: "Mythic" } },
          { ...fourth!, grant: { kind: "cosmetic", cosmeticId: "card-back-clockwork", cosmeticRarity: "Legendary" } },
          ...rest,
        ],
      },
    });
    fireEvent.click(screen.getByTestId(T.flip(2)));
    fireEvent.click(screen.getByTestId(T.flip(3)));
    const foil = screen.getByTestId(T.slot(2));
    expect(screen.getByTestId(T.outcome(2))).toHaveTextContent("New cosmetic: Void shimmer");
    expect(foil).toHaveTextContent("Mythic · Chase");
    // Its own rarity, never its card's (R1333000).
    expect(foil).toHaveAttribute("data-cosmetic-rarity", "Mythic");
    // The slot's card, wearing the foil, in the card's place.
    const face = foil.querySelector(".pack-slot__face .cosmetic-look .cf");
    expect(face).toHaveAttribute("data-cosmetic-foil", "foil-void-shimmer");
    expect(face).toHaveTextContent(CATALOG[third!.cardId]?.name ?? "no card");
    // A card back sits on no card: its look is the back.
    const back = screen.getByTestId(T.slot(3));
    expect(screen.getByTestId(T.outcome(3))).toHaveTextContent("New cosmetic: Clockwork");
    expect(back).toHaveTextContent("Legendary · Chase");
    expect(back.querySelector(".pack-slot__face .cf-back")).toHaveAttribute("data-card-back", "card-back-clockwork");
    expect(back.querySelector(".pack-slot__face .cf")).toBeNull();
    // A card's slot shows its card as before, with no cosmetic rarity.
    expect(screen.getByTestId(T.slot(0))).not.toHaveAttribute("data-cosmetic-rarity");
  });

  it("Epic and above flip only after a hold", () => {
    torn();
    const epic = screen.getByTestId(T.flip(1));
    expect(epic).toHaveAccessibleName("Hold to reveal card 2");
    expect(screen.getByTestId(T.flip(4))).toHaveAccessibleName("Reveal card 5");

    fireEvent.click(epic, { detail: 1 });
    expect(revealed(1), "a click is not a hold").toBe(false);

    fireEvent.pointerDown(epic);
    expect(screen.getByTestId(T.slot(1))).toHaveAttribute("data-holding", "true");
    advance(PACK_HOLD_MS - 1);
    fireEvent.pointerUp(epic);
    advance(PACK_HOLD_MS);
    expect(revealed(1), "released early").toBe(false);
    expect(screen.getByTestId(T.slot(1))).not.toHaveAttribute("data-holding");

    fireEvent.pointerDown(epic);
    fireEvent.pointerLeave(epic);
    advance(PACK_HOLD_MS);
    expect(revealed(1), "a pointer that left").toBe(false);

    fireEvent.pointerDown(epic);
    advance(PACK_HOLD_MS);
    expect(revealed(1)).toBe(true);
    expect(engine.playSfx).toHaveBeenCalledWith("sting", { tier: "epic" });
    expect(screen.getByTestId(T.outcome(1))).toHaveTextContent("New card");

    // A keyboard press has no pointer to hold with, so it turns the Mythic.
    fireEvent.click(screen.getByTestId(T.flip(0)), { detail: 0 });
    expect(revealed(0)).toBe(true);
    expect(engine.playSfx).toHaveBeenCalledWith("entrance", { mythic: true });
  });

  it("skip-all reveals all five at once", () => {
    const given = props({ opened: openedPack() });
    render(<PackScene {...given} />);
    expect(screen.getByTestId(T.close)).toHaveTextContent("Close");
    fireEvent.click(screen.getByTestId(T.skipAll));

    for (let index = 0; index < 5; index += 1) expect(revealed(index), `slot ${String(index)}`).toBe(true);
    expect(screen.queryByTestId(T.pack)).toBeNull();
    expect(screen.queryByTestId(T.skipAll)).toBeNull();
    expect(screen.getByTestId(T.close)).toHaveTextContent("Done");
    expect(engine.playSfx.mock.calls, "one sting for the best card").toEqual([["entrance", { mythic: true }]]);
    expect(given.onTear, "the pack was open already").not.toHaveBeenCalled();

    advance(PACK_TEAR_MS * 2);
    expect(engine.playSfx).toHaveBeenCalledTimes(1);
  });

  it("skip-all on a sealed pack opens it and shows everything when the answer comes", () => {
    const given = props();
    const { rerender } = render(<PackScene {...given} />);
    fireEvent.click(screen.getByTestId(T.skipAll));
    expect(given.onTear).toHaveBeenCalledTimes(1);
    expect(screen.queryByTestId(T.slots)).toBeNull();

    rerender(<PackScene {...given} busy />);
    rerender(<PackScene {...given} opened={openedPack()} />);
    for (let index = 0; index < 5; index += 1) expect(revealed(index)).toBe(true);
    expect(screen.getByTestId(T.close)).toHaveTextContent("Done");
    expect(engine.playSfx.mock.calls).toEqual([["entrance", { mythic: true }]]);
  });

  it("reduce motion skips the animation: the results show at once", () => {
    writeSettings({ reduceMotion: true });
    render(<PackScene {...props({ opened: openedPack() })} />);
    expect(screen.getByTestId(T.scene)).toHaveAttribute("data-motion", "reduce");
    expect(screen.queryByTestId(T.pack), "no tear").toBeNull();
    expect(screen.queryByTestId(T.fx), "no effects").toBeNull();
    for (let index = 0; index < 5; index += 1) {
      expect(revealed(index)).toBe(true);
      expect(screen.queryByTestId(T.flip(index)), "no flip").toBeNull();
    }
    expect(screen.getByTestId(T.close)).toHaveTextContent("Done");
    expect(engine.playSfx.mock.calls).toEqual([["entrance", { mythic: true }]]);
  });

  it("motion on draws the effects layer, and a card still turns with it", () => {
    torn();
    expect(screen.getByTestId(T.scene)).toHaveAttribute("data-motion", "full");
    const layer = screen.getByTestId(T.fx);
    expect(layer.querySelector("canvas")).not.toBeNull();
    fireEvent.click(screen.getByTestId(T.flip(3)));
    expect(revealed(3)).toBe(true);
  });

  it("a pack with packs left offers the next, the last offers Done", () => {
    const more = props({ opened: openedPack(), left: 1 });
    const { unmount } = render(<PackScene {...more} />);
    expect(screen.queryByTestId(T.next), "not before every card shows").toBeNull();
    fireEvent.click(screen.getByTestId(T.skipAll));
    const next = screen.getByTestId(T.next);
    expect(next).toHaveTextContent("Next pack (1 left)");
    fireEvent.click(next);
    expect(more.onNext).toHaveBeenCalledTimes(1);
    expect(screen.getByTestId(T.close)).toHaveTextContent("Done");
    fireEvent.click(screen.getByTestId(T.close));
    expect(more.onClose).toHaveBeenCalledTimes(1);
    unmount();

    render(<PackScene {...props({ opened: openedPack(), left: 0 })} />);
    fireEvent.click(screen.getByTestId(T.skipAll));
    expect(screen.queryByTestId(T.next)).toBeNull();
    expect(screen.getByTestId(T.close)).toHaveTextContent("Done");
  });

  it("a failed open shows the server's words and keeps the pack sealed", () => {
    render(<PackScene {...props({ error: NO_PACK })} />);
    expect(screen.getByRole("alert")).toHaveTextContent(NO_PACK);
    expect(screen.getByTestId(T.error)).toHaveTextContent(NO_PACK);
    const pack = screen.getByTestId(T.pack);
    expect(pack).toHaveAttribute("data-state", "sealed");
    expect(pack).not.toBeDisabled();
    expect(screen.queryByTestId(T.slots)).toBeNull();
    expect(screen.getByTestId(T.close)).not.toBeDisabled();
  });

  it("a refused skip-all leaves the next try a plain tear", () => {
    const given = props();
    const { rerender } = render(<PackScene {...given} />);
    fireEvent.click(screen.getByTestId(T.skipAll));
    rerender(<PackScene {...given} error={NO_PACK} />);
    rerender(<PackScene {...given} />);
    rerender(<PackScene {...given} opened={openedPack()} />);
    expect(screen.getByTestId(T.pack)).toHaveAttribute("data-state", "tearing");
    expect(screen.queryByTestId(T.slots)).toBeNull();
  });

  it("the sealed pack is a PackObject at scene size: pressed under the pointer, tearing once the server has answered", () => {
    const given = props();
    const { rerender } = render(<PackScene {...given} />);
    const object = screen.getByTestId(T.object);
    expect(screen.getByTestId(T.pack)).toContainElement(object);
    expect(object).toHaveAttribute("data-pack", "Core");
    expect(object).toHaveAttribute("data-size", "scene");
    expect(object).toHaveAttribute("data-state", "idle");
    expect(object).toHaveTextContent("Core pack");

    fireEvent.pointerDown(screen.getByTestId(T.pack), { clientX: 0, pointerId: 1 });
    expect(screen.getByTestId(T.object)).toHaveAttribute("data-state", "pressed");
    fireEvent.pointerUp(screen.getByTestId(T.pack), { clientX: 0, pointerId: 1 });
    expect(screen.getByTestId(T.object)).toHaveAttribute("data-state", "idle");

    fireEvent.pointerDown(screen.getByTestId(T.pack), { clientX: 0, pointerId: 1 });
    fireEvent.pointerCancel(screen.getByTestId(T.pack), { pointerId: 1 });
    expect(screen.getByTestId(T.object), "a canceled press lets go").toHaveAttribute("data-state", "idle");

    rerender(<PackScene {...given} opened={openedPack()} />);
    expect(screen.getByTestId(T.object)).toHaveAttribute("data-state", "tearing");
    advance(PACK_TEAR_MS);
    expect(screen.queryByTestId(T.object), "the cards replace the pack").toBeNull();
  });

  it("under reduced motion the pack object holds still and the scene still reveals every slot", () => {
    writeSettings({ reduceMotion: true });
    const given = props();
    const { rerender } = render(<PackScene {...given} />);
    expect(screen.getByTestId(T.object)).toHaveAttribute("data-motion", "reduce");

    rerender(<PackScene {...given} opened={openedPack()} />);
    expect(screen.getByTestId(T.scene)).toHaveAttribute("data-motion", "reduce");
    for (let index = 0; index < 5; index += 1) expect(revealed(index), `slot ${String(index)}`).toBe(true);
    expect(screen.queryByTestId(T.object)).toBeNull();
  });

  it("R1953000 the HUD sits at the bars' points before while the cards turn, and plays once every card shows", () => {
    render(<PackScene {...props()} />);
    expect(screen.queryByTestId(T.bars), "nothing before the server answers").toBeNull();
    expect(screen.getByTestId(T.scene)).not.toHaveAttribute("data-hud");
    cleanup();

    torn();
    const bars = screen.getByTestId(T.bars);
    expect(bars).toHaveClass("opening-hud");
    expect(screen.getByTestId(T.scene), "the scene starts below the HUD").toHaveAttribute("data-hud", "true");
    const card = (): HTMLElement => within(bars).getByTestId(`${T.bars}-card`);
    const cosmetic = (): HTMLElement => within(bars).getByTestId(`${T.bars}-cosmetic`);

    // Epic and above hold to turn; a keyboard press (no pointer) turns them at once.
    for (const index of [4, 3, 2, 1]) {
      fireEvent.click(screen.getByTestId(T.flip(index)), { detail: 0 });
      expect(bars, `after card ${String(index)}`).toHaveAttribute("data-phase", "waiting");
      expect(card()).toHaveTextContent("3 / 12");
      expect(cosmetic()).toHaveTextContent("5 / 30");
      expect(bars.querySelector(".bar-flight")).toBeNull();
    }
    fireEvent.click(screen.getByTestId(T.flip(0)), { detail: 0 });

    // The spent duplicate's points fly from its card, tinted by its rarity; then the bars fill.
    expect(bars).toHaveAttribute("data-phase", "flights");
    expect(bars.querySelectorAll(".bar-flight")).toHaveLength(1);
    const flight = within(bars).getByTestId(`${T.bars}-flight-3`);
    expect(flight).toHaveTextContent("+4");
    expect(flight).toHaveAttribute("data-rarity", "Rare");
    advance(flightsMs(1));
    expect(bars).toHaveAttribute("data-phase", "fill");
    expect(card()).toHaveAttribute("data-state", "filling");
    expect(card()).toHaveTextContent("7 / 12");
    expect(cosmetic()).toHaveTextContent("9 / 30");
  });

  it("R1953000 skip-all plays the HUD at once, and a reduced-motion scene shows it filled with the cards", () => {
    render(<PackScene {...props({ opened: openedPack() })} />);
    expect(screen.getByTestId(T.bars)).toHaveAttribute("data-phase", "waiting");
    fireEvent.click(screen.getByTestId(T.skipAll));
    expect(screen.getByTestId(T.bars)).toHaveAttribute("data-phase", "flights");
    cleanup();

    writeSettings({ reduceMotion: true });
    render(<PackScene {...props({ opened: openedPack() })} />);
    const bars = screen.getByTestId(T.bars);
    expect(bars).toHaveAttribute("data-phase", "fill");
    expect(bars).toHaveAttribute("data-motion", "still");
    expect(bars.querySelector(".bar-flight")).toBeNull();
    expect(within(bars).getByTestId(`${T.bars}-card`)).toHaveAttribute("data-state", "resting");
    expect(within(bars).getByTestId(`${T.bars}-card`).querySelector("progress")).toHaveProperty("value", 7);
  });

  it("R1999000 an opening of all-new cards flies a flight from every slot and fills both bars", () => {
    // Every slot a new card, and the server's points for each, its card rarity's: since R1999000 a
    // new card adds them as a spent duplicate does.
    const fixture = openedPack();
    const slotPoints = [10, 3, 1, 2, 1];
    const opened: OpenedPackResponse = {
      ...fixture,
      slots: fixture.slots.map((slot) => ({ ...slot, grant: { kind: "card" as const } })),
      bars: {
        card: { before: 3, after: 20, size: 40, pays: 0 },
        cosmetic: { before: 5, after: 22, size: 40, pays: 0 },
        cosmeticPaid: [],
        slotPoints,
      },
    };
    render(<PackScene {...props({ opened })} />);
    fireEvent.click(screen.getByTestId(T.skipAll));
    for (let index = 0; index < 5; index += 1) {
      expect(screen.getByTestId(T.outcome(index))).toHaveTextContent("New card");
    }
    const bars = screen.getByTestId(T.bars);
    expect(bars).toHaveAttribute("data-phase", "flights");
    expect(bars.querySelectorAll(".bar-flight")).toHaveLength(5);
    opened.slots.forEach((slot, index) => {
      const flight = within(bars).getByTestId(`${T.bars}-flight-${String(index)}`);
      expect(flight, `slot ${String(index)}`).toHaveTextContent(`+${String(slotPoints[index])}`);
      expect(flight, `slot ${String(index)}`).toHaveAttribute("data-rarity", slot.rarity);
    });
    advance(flightsMs(5));
    expect(bars).toHaveAttribute("data-phase", "fill");
    expect(within(bars).getByTestId(`${T.bars}-card`)).toHaveTextContent("20 / 40");
    expect(within(bars).getByTestId(`${T.bars}-cosmetic`)).toHaveTextContent("22 / 40");
  });

  it("R1953000 an opening that moved neither bar still shows both, with nothing to fly", () => {
    // No slot gave the bars a point: since R1999000 the server answers none such, but the HUD draws
    // whatever it is told. The fixture's spent duplicate is a new card here.
    const still = { before: 4, after: 4, size: 12, pays: 0 };
    const fixture = openedPack();
    const opened: OpenedPackResponse = {
      ...fixture,
      slots: fixture.slots.map((slot) => (slot.grant.kind === "bars" ? { ...slot, grant: { kind: "card" } } : slot)),
      bars: { card: still, cosmetic: still, cosmeticPaid: [], slotPoints: [0, 0, 0, 0, 0] },
    };
    render(<PackScene {...props({ opened })} />);
    fireEvent.click(screen.getByTestId(T.skipAll));
    expect(revealed(0)).toBe(true);
    const bars = screen.getByTestId(T.bars);
    expect(bars).toHaveAttribute("data-phase", "fill");
    expect(bars.querySelector(".bar-flight")).toBeNull();
    expect(within(bars).getByTestId(`${T.bars}-card`)).toHaveTextContent("4 / 12");
    expect(within(bars).getByTestId(`${T.bars}-cosmetic`)).toHaveTextContent("4 / 12");
  });

  it("the scene is a dialog named for its pack", () => {
    render(<PackScene {...props()} />);
    const dialog = screen.getByRole("dialog", { name: "Opening Core pack" });
    expect(within(dialog).getByRole("heading", { name: "Opening Core pack" })).toBeInTheDocument();
    cleanup();
    render(<PackScene {...props({ pack: GLOBAL_PACK_NAME })} />);
    expect(screen.getByRole("dialog", { name: "Opening Global pack" })).toBeInTheDocument();
  });
});

type Answers = { info?: Reply; open?: (call: number) => Reply };

/** The two routes, answered from the fixtures unless a test says otherwise. */
function packsServer(answers: Answers = {}): ReturnType<typeof stubServer> {
  let opens = 0;
  return stubServer((call) => {
    if (call.method === "GET" && call.path === "/api/packs") return answers.info ?? { status: 200, body: packsInfo() };
    if (call.method === "POST" && call.path === "/api/packs/open") {
      opens += 1;
      return answers.open?.(opens) ?? { status: 200, body: openedPack() };
    }
    return undefined;
  });
}

async function panel(): Promise<void> {
  render(<PackOpening token={TOKEN} />);
  await waitFor(() => {
    expect(screen.queryByTestId(T.loading)).toBeNull();
  });
}

const text = (id: string): string => screen.getByTestId(id).textContent ?? "";

describe("#1308 the card packs panel", () => {
  it("the panel lists the packs held, before opening and after, and shows neither pity nor odds", async () => {
    const server = packsServer();
    await panel();

    expect(screen.getByRole("heading", { name: "Card packs" })).toBeInTheDocument();
    expect(text(T.heldRow("Core"))).toContain("Core pack: 2 unopened");
    expect(text(T.heldRow(GLOBAL_PACK_NAME))).toContain("Global pack: 1 unopened");
    expect(screen.queryByTestId(T.openAll(GLOBAL_PACK_NAME)), "one pack has no open-all").toBeNull();
    expect(server.to("GET", "/api/packs")).toHaveLength(1);
    expect(server.to("GET", "/api/packs")[0]?.headers.authorization).toBe(`Bearer ${TOKEN}`);

    const shownNeither = (): void => {
      expect(screen.queryByTestId(T.pityPanel)).toBeNull();
      expect(screen.queryByTestId(T.odds)).toBeNull();
      expect(screen.queryByTestId(T.duplicate)).toBeNull();
      expect(screen.queryByRole("heading", { name: "Pity" })).toBeNull();
      expect(screen.queryByRole("table")).toBeNull();
      expect(screen.getByTestId(T.panel).querySelector('[data-testid^="packs-pity"], [data-testid^="packs-odds"]')).toBeNull();
    };
    shownNeither();

    fireEvent.click(screen.getByTestId(T.open("Core")));
    fireEvent.click(screen.getByTestId(T.skipAll));
    expect(await screen.findByTestId(T.outcome(0))).toHaveTextContent("New card");

    const opens = server.to("POST", "/api/packs/open");
    expect(opens).toHaveLength(1);
    expect(opens[0]?.body).toEqual({ pack: "Core" });
    expect(opens[0]?.headers.authorization).toBe(`Bearer ${TOKEN}`);
    expect(text(T.heldRow("Core"))).toContain("Core pack: 1 unopened");
    shownNeither();
  });

  it("an account with no packs is told so", async () => {
    packsServer({ info: { status: 200, body: { ...packsInfo(), unopened: [] } satisfies PacksResponse } });
    await panel();
    expect(screen.getByTestId(T.none)).toHaveTextContent("No unopened packs.");
    expectEmptyState(screen.getByTestId(T.none));
    expect(screen.queryByTestId(T.held)).toBeNull();
  });

  it("a failed open shows a message and leaves the pack unopened on screen", async () => {
    const server = packsServer({ open: () => refusal(409, "conflict", NO_PACK) });
    await panel();
    fireEvent.click(screen.getByTestId(T.open("Core")));
    const pack = screen.getByTestId(T.pack);
    fireEvent.click(pack);

    expect(await screen.findByTestId(T.error)).toHaveTextContent(NO_PACK);
    expect(screen.getByTestId(T.pack)).toHaveAttribute("data-state", "sealed");
    expect(screen.queryByTestId(T.slots)).toBeNull();
    expect(server.to("POST", "/api/packs/open")).toHaveLength(1);

    fireEvent.click(screen.getByTestId(T.close));
    expect(screen.queryByTestId(T.scene)).toBeNull();
    expect(text(T.heldRow("Core"))).toContain("Core pack: 2 unopened");
  });

  it("#2053 a failed read says why in the reader's language", async () => {
    await setLocale("zh-CN");
    packsServer({ info: { status: 500 } });
    render(<PackOpening token={TOKEN} />);
    expect(await screen.findByTestId(T.loadError)).toHaveTextContent("服务器返回了 500");
  });

  it("#2053 a failed open says why in the reader's language", async () => {
    await setLocale("zh-CN");
    packsServer({ open: () => ({ status: 500 }) });
    await panel();
    fireEvent.click(screen.getByTestId(T.open("Core")));
    fireEvent.click(screen.getByTestId(T.pack));

    expect(await screen.findByTestId(T.error)).toHaveTextContent("服务器返回了 500");
    expect(screen.getByTestId(T.pack)).toHaveAttribute("data-state", "sealed");
  });

  it("a tap and a swipe on one pack open it once", async () => {
    const server = packsServer();
    await panel();
    fireEvent.click(screen.getByTestId(T.open("Core")));
    const pack = screen.getByTestId(T.pack);
    fireEvent.pointerDown(pack, { clientX: 0, pointerId: 1 });
    fireEvent.pointerUp(pack, { clientX: PACK_SWIPE_PX, pointerId: 1 });
    fireEvent.click(pack);
    await screen.findByTestId(T.pack);
    expect(server.to("POST", "/api/packs/open")).toHaveLength(1);
  });

  it("batch opening opens the packs in a row, with skip-all between them", async () => {
    const server = packsServer({
      open: (call): Reply => ({ status: 200, body: { ...openedPack(), unopened: call === 1 ? 1 : 0 } satisfies OpenedPackResponse }),
    });
    await panel();
    expect(screen.getByTestId(T.openAll("Core"))).toHaveTextContent("Open all 2");

    fireEvent.click(screen.getByTestId(T.openAll("Core")));
    fireEvent.click(screen.getByTestId(T.skipAll));
    await screen.findByTestId(T.outcome(0));
    const next = screen.getByTestId(T.next);
    expect(next).toHaveTextContent("Next pack (1 left)");

    fireEvent.click(next);
    expect(screen.queryByTestId(T.outcome(0)), "a fresh pack, sealed").toBeNull();
    expect(screen.getByTestId(T.pack)).toHaveAttribute("data-state", "sealed");
    fireEvent.click(screen.getByTestId(T.skipAll));
    await screen.findByTestId(T.outcome(0));
    expect(screen.queryByTestId(T.next)).toBeNull();
    expect(screen.getByTestId(T.close)).toHaveTextContent("Done");
    expect(server.to("POST", "/api/packs/open")).toHaveLength(2);

    fireEvent.click(screen.getByTestId(T.close));
    expect(screen.queryByTestId(T.heldRow("Core"))).toBeNull();
    expect(screen.getByTestId(T.heldRow(GLOBAL_PACK_NAME))).toBeInTheDocument();
  });

  it("a batch stops when the server holds fewer packs than it was told", async () => {
    packsServer({ open: () => ({ status: 200, body: { ...openedPack(), unopened: 0 } satisfies OpenedPackResponse }) });
    await panel();
    fireEvent.click(screen.getByTestId(T.openAll("Core")));
    fireEvent.click(screen.getByTestId(T.skipAll));
    await screen.findByTestId(T.outcome(0));
    expect(screen.queryByTestId(T.next)).toBeNull();
  });

  it("a failed read says why", async () => {
    packsServer({ info: refusal(503, "unavailable", "the packs are not reachable") });
    render(<PackOpening token={TOKEN} />);
    expect(await screen.findByTestId(T.loadError)).toHaveTextContent("the packs are not reachable");
    expect(screen.queryByTestId(T.held)).toBeNull();
    expect(screen.queryByTestId(T.loading)).toBeNull();
  });

  it("#1485 a failed read that works on the page's next revision shows the shop and drops the alert", async () => {
    const answers: Answers = { info: refusal(503, "unavailable", "the packs are not reachable") };
    packsServer(answers);
    const { rerender } = render(<PackOpening token={TOKEN} />);
    expect(await screen.findByTestId(T.loadError)).toHaveTextContent("the packs are not reachable");

    answers.info = { status: 200, body: packsInfo() };
    rerender(<PackOpening token={TOKEN} revision={1} />);

    await screen.findByTestId(T.held);
    expect(text(T.heldRow("Core"))).toContain("Core pack: 2 unopened");
    expect(screen.queryByTestId(T.loadError)).toBeNull();
    expect(screen.queryByTestId(T.retry)).toBeNull();
  });

  it("#1485 Try again reads once more, cannot be pressed twice while the read is out, and drops the alert once it works", async () => {
    const answers: Answers = { info: refusal(503, "unavailable", "the packs are not reachable") };
    const server = packsServer(answers);
    render(<PackOpening token={TOKEN} />);
    await waitFor(() => {
      expect(screen.getByTestId(T.retry)).toBeEnabled();
    });
    expect(server.to("GET", "/api/packs")).toHaveLength(1);

    answers.info = { status: 200, body: packsInfo() };
    fireEvent.click(screen.getByTestId(T.retry));
    // The read is out: the button waits for it, and a second press sends nothing.
    expect(screen.getByTestId(T.retry)).toBeDisabled();
    fireEvent.click(screen.getByTestId(T.retry));

    await screen.findByTestId(T.held);
    expect(text(T.heldRow("Core"))).toContain("Core pack: 2 unopened");
    expect(screen.queryByTestId(T.loadError)).toBeNull();
    expect(server.to("GET", "/api/packs")).toHaveLength(2);
  });

  it("says it is loading until the server answers", async () => {
    packsServer();
    render(<PackOpening token={TOKEN} />);
    expect(screen.getByTestId(T.loading)).toHaveTextContent("Loading your packs…");
    expectSkeleton(screen.getByTestId(T.loading), "rows");
    await screen.findByTestId(T.held);
    expect(screen.queryByTestId(T.loading)).toBeNull();
  });

  it("#1309 an opening tells the page, a refused one does not, and the page's next revision reads the packs again", async () => {
    const onChanged = vi.fn<() => void>();
    const server = packsServer({
      open: (call) => (call === 1 ? refusal(409, "conflict", NO_PACK) : { status: 200, body: openedPack() }),
    });
    const { rerender } = render(<PackOpening token={TOKEN} onChanged={onChanged} />);
    await screen.findByTestId(T.held);

    fireEvent.click(screen.getByTestId(T.open("Core")));
    fireEvent.click(screen.getByTestId(T.pack));
    expect(await screen.findByTestId(T.error)).toHaveTextContent(NO_PACK);
    expect(onChanged, "a refusal changed nothing to tell the page").not.toHaveBeenCalled();

    fireEvent.click(screen.getByTestId(T.close));
    fireEvent.click(screen.getByTestId(T.open("Core")));
    fireEvent.click(screen.getByTestId(T.skipAll));
    expect(await screen.findByTestId(T.outcome(0))).toHaveTextContent("New card");
    expect(onChanged).toHaveBeenCalledTimes(1);

    // Another panel bought a pack: the page asks this one to read again.
    expect(server.to("GET", "/api/packs")).toHaveLength(1);
    rerender(<PackOpening token={TOKEN} revision={1} onChanged={onChanged} />);
    await waitFor(() => {
      expect(server.to("GET", "/api/packs")).toHaveLength(2);
    });
  });
});

describe("R2378000 one spend at a time", () => {
  /** The server's words for a spend sent past the ones waiting their turn. */
  const BUSY = "your last purchase or opening is still going through; try again in a moment";

  let release: () => void = () => undefined;
  let outside: Promise<void> = Promise.resolve();

  /** Another spend of the tab's (a Shop buy, say), out until `settleOutside`. */
  function spendOutside(): void {
    act(() => {
      outside = whileSpending(
        () =>
          new Promise<void>((resolve) => {
            release = resolve;
          }),
      );
    });
  }

  async function settleOutside(): Promise<void> {
    await act(async () => {
      release();
      await outside;
    });
  }

  afterEach(async () => {
    await settleOutside();
  });

  it("R2378000 while another spend is out, Open and Open all wait, turning a wheel", async () => {
    const server = packsServer();
    await panel();
    const buttons = [T.open("Core"), T.openAll("Core"), T.open(GLOBAL_PACK_NAME)];
    for (const id of buttons) expect(screen.getByTestId(id), id).toHaveAttribute("aria-busy", "false");

    spendOutside();
    for (const id of buttons) {
      expect(screen.getByTestId(id), id).toBeDisabled();
      expect(screen.getByTestId(id), id).toHaveAttribute("aria-busy", "true");
      expect(screen.getByTestId(id), id).toHaveClass("spend-wait");
    }
    fireEvent.click(screen.getByTestId(T.open("Core")));
    expect(screen.queryByTestId(T.scene), "a waiting Open opens nothing").toBeNull();

    await settleOutside();
    for (const id of buttons) {
      expect(screen.getByTestId(id), id).toBeEnabled();
      expect(screen.getByTestId(id), id).toHaveAttribute("aria-busy", "false");
    }
    expect(server.to("POST", "/api/packs/open")).toHaveLength(0);
  });

  it("R2378000 the sealed pack waits while another spend is out, then opens with one request", async () => {
    const server = packsServer();
    await panel();
    fireEvent.click(screen.getByTestId(T.open("Core")));
    spendOutside();

    const pack = screen.getByTestId(T.pack);
    expect(pack).toBeDisabled();
    expect(screen.getByTestId(T.skipAll)).toBeDisabled();
    fireEvent.click(pack);
    // A disabled button still hears the pointer: the swipe asks to open, and the panel says no.
    fireEvent.pointerDown(pack, { clientX: 0, pointerId: 1 });
    fireEvent.pointerUp(pack, { clientX: PACK_SWIPE_PX, pointerId: 1 });
    expect(server.to("POST", "/api/packs/open")).toHaveLength(0);
    expect(screen.getByTestId(T.pack)).toHaveAttribute("data-state", "sealed");

    await settleOutside();
    expect(screen.getByTestId(T.pack)).toBeEnabled();
    fireEvent.click(screen.getByTestId(T.skipAll));
    expect(await screen.findByTestId(T.outcome(0))).toHaveTextContent("New card");
    expect(server.to("POST", "/api/packs/open")).toHaveLength(1);
  });

  it("R2378000 Next pack waits while another spend is out", async () => {
    packsServer({
      open: (call): Reply => ({ status: 200, body: { ...openedPack(), unopened: call === 1 ? 1 : 0 } satisfies OpenedPackResponse }),
    });
    await panel();
    fireEvent.click(screen.getByTestId(T.openAll("Core")));
    fireEvent.click(screen.getByTestId(T.skipAll));
    await screen.findByTestId(T.outcome(0));

    spendOutside();
    expect(screen.getByTestId(T.next)).toBeDisabled();
    fireEvent.click(screen.getByTestId(T.next));
    expect(screen.getByTestId(T.outcome(0)), "the opened pack stays").toBeInTheDocument();

    await settleOutside();
    fireEvent.click(screen.getByTestId(T.next));
    expect(screen.getByTestId(T.pack)).toHaveAttribute("data-state", "sealed");
  });

  it("R2378000 a refusal past the ones waiting their turn shows the server's words and leaves the pack sealed", async () => {
    const server = packsServer({ open: () => refusal(429, "rate_limited", BUSY) });
    await panel();
    fireEvent.click(screen.getByTestId(T.open("Core")));
    fireEvent.click(screen.getByTestId(T.pack));

    expect(await screen.findByTestId(T.error)).toHaveTextContent(BUSY);
    expect(screen.getByTestId(T.pack)).toHaveAttribute("data-state", "sealed");
    expect(screen.getByTestId(T.pack), "nothing is out, so it can be torn again").toBeEnabled();
    expect(server.to("POST", "/api/packs/open")).toHaveLength(1);
  });
});

describe("#1757 the pack-opening scene's focus and keys", () => {
  /** fireEvent returns false when a handler called preventDefault. */
  function tabFrom(element: Element, shift = false): boolean {
    return fireEvent.keyDown(element, { key: "Tab", shiftKey: shift });
  }

  it("#1757 the scene takes the focus when it opens, and Tab and Shift+Tab wrap inside it", () => {
    render(
      <>
        <button type="button" data-testid="behind">
          behind
        </button>
        <PackScene {...props()} />
      </>,
    );
    const scene = screen.getByTestId(T.scene);
    const pack = screen.getByTestId(T.pack);
    const close = screen.getByTestId(T.close);
    expect(scene).toHaveFocus();

    expect(tabFrom(scene)).toBe(false);
    expect(pack).toHaveFocus();
    expect(tabFrom(pack, true)).toBe(false);
    expect(close).toHaveFocus();
    expect(tabFrom(close)).toBe(false);
    expect(pack).toHaveFocus();

    scene.focus();
    expect(tabFrom(scene, true)).toBe(false);
    expect(close).toHaveFocus();

    pack.focus();
    expect(tabFrom(pack), "a Tab between two controls inside is the browser's").toBe(true);

    const behind = screen.getByTestId("behind");
    behind.focus();
    expect(tabFrom(behind)).toBe(false);
    expect(pack).toHaveFocus();
  });

  it("#1757 while an open is out nothing in it can take the focus, so Tab and Shift+Tab keep it on the scene", () => {
    render(<PackScene {...props({ busy: true })} />);
    const scene = screen.getByTestId(T.scene);
    expect(scene).toHaveFocus();

    expect(tabFrom(scene)).toBe(false);
    expect(scene).toHaveFocus();
    expect(tabFrom(scene, true)).toBe(false);
    expect(scene).toHaveFocus();
  });

  it("#1757 Escape closes the scene, but not while an open is out, when Close is disabled too", () => {
    const given = props({ busy: true });
    const { rerender } = render(<PackScene {...given} />);
    const scene = screen.getByTestId(T.scene);

    fireEvent.keyDown(scene, { key: "Escape" });
    expect(given.onClose).not.toHaveBeenCalled();
    expect(screen.getByTestId(T.close)).toBeDisabled();
    expect(scene).toHaveFocus();

    rerender(<PackScene {...given} busy={false} />);
    expect(screen.getByTestId(T.close)).toBeEnabled();
    fireEvent.keyDown(scene, { key: "Escape" });
    expect(given.onClose).toHaveBeenCalledTimes(1);
  });

  it("#1757 under reduced motion the scene takes the focus and Escape closes it the same", () => {
    writeSettings({ reduceMotion: true });
    const given = props({ opened: openedPack() });
    render(<PackScene {...given} />);
    const scene = screen.getByTestId(T.scene);
    expect(scene).toHaveAttribute("data-motion", "reduce");
    expect(scene).toHaveFocus();

    fireEvent.keyDown(scene, { key: "Escape" });
    expect(given.onClose).toHaveBeenCalledTimes(1);
  });

  it("#1757 focus goes back to the Open button that opened the scene, on Escape and on Done", async () => {
    packsServer();
    await panel();
    const opener = screen.getByTestId(T.open("Core"));
    opener.focus();
    fireEvent.click(opener);
    expect(screen.getByTestId(T.scene)).toHaveFocus();

    fireEvent.keyDown(screen.getByTestId(T.scene), { key: "Escape" });
    expect(screen.queryByTestId(T.scene)).toBeNull();
    expect(opener).toHaveFocus();

    fireEvent.click(opener);
    fireEvent.click(screen.getByTestId(T.skipAll));
    await screen.findByTestId(T.outcome(0));
    expect(screen.getByTestId(T.close)).toHaveTextContent("Done");
    fireEvent.click(screen.getByTestId(T.close));
    expect(screen.queryByTestId(T.scene)).toBeNull();
    expect(screen.getByTestId(T.open("Core")), "Core still holds a pack, so its row stays").toHaveFocus();
  });

  it("#1757 the next pack of a batch takes the focus again", async () => {
    packsServer({
      open: (call): Reply => ({ status: 200, body: { ...openedPack(), unopened: call === 1 ? 1 : 0 } satisfies OpenedPackResponse }),
    });
    await panel();
    fireEvent.click(screen.getByTestId(T.openAll("Core")));
    fireEvent.click(screen.getByTestId(T.skipAll));
    await screen.findByTestId(T.outcome(0));
    fireEvent.click(screen.getByTestId(T.next));

    expect(screen.getByTestId(T.pack)).toHaveAttribute("data-state", "sealed");
    expect(screen.getByTestId(T.scene)).toHaveFocus();
  });
});

describe("R1955000 a revealed new cosmetic is equipped from the scene", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    resetCosmeticsForTests();
  });

  afterEach(() => {
    resetCosmeticsForTests();
  });

  /** The opened fixture with `grants` in place of slots 2 and 3's: slot 2 is `core-004`, slot 3 is `core-007`. */
  function granting(...grants: OpenedPackResponse["slots"][number]["grant"][]): OpenedPackResponse {
    const opened = openedPack();
    return { ...opened, slots: opened.slots.map((slot, index) => (grants[index - 2] === undefined ? slot : { ...slot, grant: grants[index - 2]! })) };
  }

  it("R1955000 a per-card cosmetic's Equip shows once its slot is turned and writes that card's pick", () => {
    torn({ opened: granting({ kind: "cosmetic", cosmeticId: "border-stone", cosmeticRarity: "Common" }) });
    expect(screen.queryByTestId(T.equip(2)), "face down").toBeNull();

    fireEvent.click(screen.getByTestId(T.flip(2)));
    const equip = screen.getByTestId(T.equip(2));
    expect(equip).toHaveTextContent("Equip");
    expect(equip).toBeEnabled();

    fireEvent.click(equip);
    expect(readCardPicks("core-004").border).toBe("border-stone");
    expect(readCosmetics().border, "the account's pick is not touched").toBe(DEFAULT_CARD_COSMETICS.border);
    expect(screen.getByTestId(T.equip(2))).toHaveTextContent("Equipped");
    expect(screen.getByTestId(T.equip(2))).toBeDisabled();
  });

  it("R1955000 a full art's Equip turns that card's full art on", () => {
    torn();
    fireEvent.click(screen.getByTestId(T.flip(2)));
    expect(readCardPicks("core-004").fullArt).toBeUndefined();

    fireEvent.click(screen.getByTestId(T.equip(2)));
    expect(readCardPicks("core-004").fullArt).toBe(true);
    expect(screen.getByTestId(T.equip(2))).toHaveTextContent("Equipped");
    expect(screen.getByTestId(T.equip(2))).toBeDisabled();
  });

  it("R2655000 a chase full art's Equip picks that tier", () => {
    // The card wears its Epic already: the Mythic it just drew is still one to equip.
    writeCardFullArt("core-004", true);
    torn({ opened: granting({ kind: "cosmetic", cosmeticId: "full-art-gary-the-gambler-signed", cosmeticRarity: "Mythic" }) });
    fireEvent.click(screen.getByTestId(T.flip(2)));
    expect(screen.getByTestId(T.equip(2))).toHaveTextContent("Equip");
    expect(screen.getByTestId(T.equip(2))).toBeEnabled();

    fireEvent.click(screen.getByTestId(T.equip(2)));
    expect(readCardPicks("core-004").fullArt).toBe("full-art-gary-the-gambler-signed");
    expect(screen.getByTestId(T.equip(2))).toHaveTextContent("Equipped");
    expect(screen.getByTestId(T.equip(2))).toBeDisabled();
  });

  it("R1955000 skip-all shows every Equip at once; an account cosmetic writes the account's pick, a portrait, a card and bars offer none", () => {
    const opened = openedPack();
    const account = (cosmeticId: string): OpenedPackResponse["slots"][number] => ({
      rarity: "Common",
      cardId: "core-002",
      grant: { kind: "cosmetic", cosmeticId, cosmeticRarity: "Epic" },
    });
    render(
      <PackScene
        {...props({
          opened: {
            ...opened,
            slots: [
              account("table-marble"),
              account("field-snow"),
              account("card-back-tidal"),
              account("portrait-gary"),
              opened.slots[3]!,
              opened.slots[4]!,
            ],
          },
        })}
      />,
    );
    fireEvent.click(screen.getByTestId(T.skipAll));

    for (const index of [0, 1, 2]) expect(screen.getByTestId(T.equip(index))).toHaveTextContent("Equip");
    fireEvent.click(screen.getByTestId(T.equip(0)));
    fireEvent.click(screen.getByTestId(T.equip(1)));
    fireEvent.click(screen.getByTestId(T.equip(2)));
    expect(readCosmetics()).toMatchObject({ table: "table-marble", field: "field-snow", cardBack: "card-back-tidal" });
    expect(readCardPicks("core-002"), "an account cosmetic is no card's pick").toEqual({});
    for (const index of [0, 1, 2]) expect(screen.getByTestId(T.equip(index))).toHaveTextContent("Equipped");
    for (const index of [3, 4, 5]) expect(screen.queryByTestId(T.equip(index))).toBeNull();
  });

  it("R1955000 a cosmetic the store already holds shows Equipped when revealed", () => {
    writeCardPick("core-004", "border", "border-stone");
    writeCosmetics({ cardBack: "card-back-tidal" });
    torn({
      opened: granting(
        { kind: "cosmetic", cosmeticId: "border-stone", cosmeticRarity: "Common" },
        { kind: "cosmetic", cosmeticId: "card-back-tidal", cosmeticRarity: "Epic" },
      ),
    });
    fireEvent.click(screen.getByTestId(T.flip(2)));
    fireEvent.click(screen.getByTestId(T.flip(3)));
    for (const index of [2, 3]) {
      expect(screen.getByTestId(T.equip(index))).toHaveTextContent("Equipped");
      expect(screen.getByTestId(T.equip(index))).toBeDisabled();
    }
  });

  it("R1955000 under reduced motion the Equip shows with the results at once", () => {
    writeSettings({ reduceMotion: true });
    render(<PackScene {...props({ opened: openedPack() })} />);
    expect(screen.getByTestId(T.equip(2))).toHaveTextContent("Equip");
    for (const index of [0, 1, 3, 4]) expect(screen.queryByTestId(T.equip(index))).toBeNull();
  });
});
