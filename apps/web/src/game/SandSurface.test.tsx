// #1954: one press on the empty field is one tap — one mark set and one sound — and each
// playmat taps in its family's look and sound.
// #2332: an occupied zone's own element taps too; only its card and the board's controls keep a press.
// R3348000: every tile answers a press with a ripple in its half's playmat family and that family's
// sound, and a legal tile answers harder, without changing what the press does in the game.

import { useRef, type ReactElement } from "react";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { RAPID_SFX } from "../audio/constants.ts";
import { setAudioEngineForTests } from "../audio/engine.ts";
import { fakeAudioEngine, type FakeAudioEngine } from "../audio/test/fakeAudio.ts";
import type { SfxId } from "../audio/types.ts";
import { DEFAULT_CARD_COSMETICS, cosmeticsOf } from "../cards/cosmetics.ts";
import type { SeatLoadouts } from "../cards/loadouts.ts";
import { resetFxSettingsForTests, setFxSettings } from "../fx/settings.ts";
import { HAPTIC_PATTERNS, resetTapHapticsForTests } from "../haptics/haptics.ts";
import { resetHapticsSettingsForTests, writeHapticsSettings } from "../haptics/settings.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { baseView, fullBoardView } from "../test/fixtures.ts";
import { TILE_FEEL } from "./arena/config.ts";
import Board from "./Board.tsx";
import { testid, type ClickTarget } from "./contract.ts";
import SandSurface, {
  TAP_FAMILY,
  TAP_FEEL,
  TAP_SFX,
  tapFamilyOf,
  tapMarks,
  type TapFamily,
} from "./SandSurface.tsx";

/** The issue's table, hardcoded here so a drift in either direction fails. */
const FAMILY_OF: Record<string, TapFamily> = {
  "table-tavern": "sand",
  "table-desert-sand": "sand",
  "table-wood": "wood",
  "table-chessboard": "wood",
  "table-felt": "felt",
  "table-marble": "stone",
  "table-obsidian-slab": "stone",
  "table-neon-grid": "neon",
  "table-ancient-map": "parchment",
  "table-astral": "astral",
  "table-stinkyscape": "neon",
};

const SFX_OF: Record<TapFamily, SfxId> = {
  sand: "sand",
  wood: "tapWood",
  felt: "tapFelt",
  stone: "tapStone",
  neon: "tapNeon",
  parchment: "tapParchment",
  astral: "tapAstral",
};

const MARK_OF: Record<TapFamily, string> = {
  sand: "sand-mark--grain",
  wood: "tap-mark--wood",
  felt: "tap-mark--felt",
  stone: "tap-mark--stone",
  neon: "tap-mark--neon",
  parchment: "tap-mark--parchment",
  astral: "tap-mark--astral",
};

const FAMILIES: readonly TapFamily[] = ["sand", "wood", "felt", "stone", "neon", "parchment", "astral"];

let audio: FakeAudioEngine;

beforeEach(() => {
  audio = fakeAudioEngine();
  setAudioEngineForTests(audio);
});

afterEach(() => {
  cleanup();
  setAudioEngineForTests(null);
  vi.restoreAllMocks();
  vi.useRealTimers();
  localStorage.clear();
  resetFxSettingsForTests();
  __resetSettingsForTests();
  resetHapticsSettingsForTests();
  resetTapHapticsForTests();
  Reflect.deleteProperty(navigator, "vibrate");
});

function Field({ table, theirTable }: { table?: string; theirTable?: string }): ReactElement {
  const field = useRef<HTMLDivElement>(null);
  return (
    <div data-testid="field" ref={field}>
      <SandSurface field={field} disabled={false} table={table} theirTable={theirTable} />
    </div>
  );
}

/**
 * The tests' bounds: a 100×100 field at the origin, so a client point taps the same point. #2015: `y`
 * under 50 is the opponent's half, so `press` taps the viewer's by default.
 */
function stubBounds(field: HTMLElement): void {
  Object.defineProperty(field, "getBoundingClientRect", {
    configurable: true,
    value: () => ({ left: 0, top: 0, width: 100, height: 100 }),
  });
}

function press(field: HTMLElement, x = 30, y = 70, pointerId = 1): void {
  fireEvent.pointerDown(field, { clientX: x, clientY: y, pointerId });
}

function markCount(field: HTMLElement): number {
  return field.querySelectorAll(".sand-mark, .tap-mark").length;
}

function playedIds(): SfxId[] {
  return audio.playSfx.mock.calls.map(([id]) => id);
}

/** #2332: `fullBoardView`, every zone holding a card, on the marble playmat; returns its stubbed field. */
function boardField(): HTMLElement {
  const mounted = render(
    <Board view={fullBoardView()} wearCosmetics deckCosmetics={{ table: "table-marble", field: null, cardBack: null }} />,
  );
  const field = mounted.container.querySelector<HTMLElement>(".field");
  if (field === null) throw new Error("fixture must draw a field");
  stubBounds(field);
  return field;
}

describe("SandSurface taps", () => {
  it.each([undefined, "table-astral"] as const)(
    "#1954 a press taps once, and a held drag across the field after it adds no mark and no sound (%s)",
    (table) => {
      render(<Field table={table} />);
      const field = screen.getByTestId("field");
      stubBounds(field);

      press(field);
      const marks = markCount(field);
      expect(marks).toBeGreaterThan(0);
      expect(audio.playSfx).toHaveBeenCalledTimes(1);

      for (let step = 0; step < 10; step += 1) {
        fireEvent.pointerMove(field, { clientX: 30 + step * 5, clientY: 40, pointerId: 1, buttons: 1 });
      }
      expect(markCount(field)).toBe(marks);
      expect(audio.playSfx).toHaveBeenCalledTimes(1);
    },
  );

  it("#1954 quick separate presses each tap, and the build piles up", () => {
    render(<Field />);
    const field = screen.getByTestId("field");
    stubBounds(field);

    press(field);
    press(field, 35, 45, 2);
    expect(audio.playSfx).toHaveBeenCalledTimes(2);
    expect(audio.playSfx.mock.calls[1]?.[1]?.sandBuild).toBe(2);
  });

  it("#1954 every playmat taps in its family's sound and mark", () => {
    const tables = cosmeticsOf("table").map((cosmetic) => cosmetic.id);
    expect([...tables].sort()).toEqual(Object.keys(FAMILY_OF).sort());
    expect({ ...TAP_FAMILY }).toEqual(FAMILY_OF);

    for (const id of tables) {
      const mounted = render(<Field table={id} />);
      const field = screen.getByTestId("field");
      stubBounds(field);
      audio.playSfx.mockClear();

      press(field);
      const family = FAMILY_OF[id] ?? "sand";
      expect(playedIds()).toEqual([SFX_OF[family]]);
      expect(field.querySelector(`.${MARK_OF[family]}`)).not.toBeNull();
      for (const other of FAMILIES) {
        if (other === family) continue;
        expect(field.querySelector(`.${MARK_OF[other]}`), `${id} draws no ${other} mark`).toBeNull();
      }
      if (family === "sand") {
        expect(field.querySelector(".tap-mark")).toBeNull();
      } else {
        expect(field.querySelector(".sand-mark")).toBeNull();
      }
      mounted.unmount();
    }
  });

  it("#1954 an unknown playmat taps as sand", () => {
    render(<Field table="table-unknown" />);
    const field = screen.getByTestId("field");
    stubBounds(field);

    press(field);
    expect(playedIds()).toEqual(["sand"]);
    expect(field.querySelector(".sand-mark--grain")).not.toBeNull();

    expect(tapFamilyOf(undefined)).toBe("sand");
    for (const id of Object.values(TAP_SFX)) {
      expect(RAPID_SFX.has(id), `${id} skips the retrigger and voice limits`).toBe(true);
    }
  });

  it("#1954 Board hands the worn playmat to the field", () => {
    const mounted = render(
      <Board view={fullBoardView()} wearCosmetics deckCosmetics={{ table: "table-neon-grid", field: null, cardBack: null }} />,
    );
    const field = mounted.container.querySelector<HTMLElement>(".field");
    if (field === null) throw new Error("fixture must draw a field");
    stubBounds(field);

    press(field);
    expect(field.querySelector(".tap-mark--neon")).not.toBeNull();
    expect(playedIds()).toEqual(["tapNeon"]);
  });

  it("#2015 each half taps in its own playmat: yours below the center line, sand above with no opponent's", () => {
    render(<Field table="table-marble" />);
    const field = screen.getByTestId("field");
    stubBounds(field);

    press(field, 30, 70);
    expect(playedIds()).toEqual(["tapStone"]);
    expect(field.querySelector(".tap-mark--stone")).not.toBeNull();
    expect(field.querySelector(".sand-mark")).toBeNull();

    press(field, 30, 20, 2);
    expect(playedIds()).toEqual(["tapStone", "sand"]);
    expect(field.querySelector(".sand-mark--grain")).not.toBeNull();
  });

  it("#2015 the opponent's half taps in their playmat, and an absent one of yours taps as sand", () => {
    render(<Field theirTable="table-astral" />);
    const field = screen.getByTestId("field");
    stubBounds(field);

    press(field, 30, 20);
    expect(playedIds()).toEqual(["tapAstral"]);
    expect(field.querySelector(".tap-mark--astral")).not.toBeNull();

    press(field, 30, 70, 2);
    expect(playedIds()).toEqual(["tapAstral", "sand"]);
    expect(field.querySelector(".sand-mark--grain")).not.toBeNull();
  });

  it("#2015 Board hands the opponent's playmat to the field's upper half", () => {
    const loadouts: SeatLoadouts = {
      p1: { seat: { ...DEFAULT_CARD_COSMETICS }, cards: {} },
      p2: { seat: { ...DEFAULT_CARD_COSMETICS, table: "table-astral" }, cards: {} },
    };
    const mounted = render(
      <Board
        view={fullBoardView()}
        wearCosmetics
        deckCosmetics={{ table: "table-marble", field: null, cardBack: null }}
        loadouts={loadouts}
      />,
    );
    const field = mounted.container.querySelector<HTMLElement>(".field");
    if (field === null) throw new Error("fixture must draw a field");
    stubBounds(field);

    press(field, 30, 20);
    press(field, 30, 70, 2);
    expect(playedIds()).toEqual(["tapAstral", "tapStone"]);
    expect(field.querySelector(".tap-mark--astral")).not.toBeNull();
    expect(field.querySelector(".tap-mark--stone")).not.toBeNull();
  });

  it("#1954 neon's hue and astral's stars come from variationFor, never Math.random", () => {
    const random = vi.spyOn(Math, "random");
    const neon = tapMarks("neon", 30, 40, 7, 1);
    const astral = tapMarks("astral", 30, 40, 7, 1);

    render(<Field table="table-neon-grid" />);
    const field = screen.getByTestId("field");
    stubBounds(field);
    press(field);

    expect(random).not.toHaveBeenCalled();
    expect(tapMarks("neon", 30, 40, 7, 1)).toEqual(neon);
    expect(tapMarks("astral", 30, 40, 7, 1)).toEqual(astral);

    expect(neon).toHaveLength(1);
    expect(TAP_FEEL.neonColors).toContain(neon[0]?.tint);
    const hues = new Set(Array.from({ length: 200 }, (_, index) => tapMarks("neon", 30, 40, index + 1, 1)[0]?.tint));
    expect(hues.size).toBe(TAP_FEEL.neonColors.length);

    expect(astral).toHaveLength(TAP_FEEL.astralStars);
    for (const star of astral) {
      expect(TAP_FEEL.astralColors).toContain(star.tint);
      expect(Math.hypot(star.dx ?? Number.NaN, star.dy ?? Number.NaN)).toBeLessThanOrEqual(TAP_FEEL.astralSpreadPx);
    }
    expect(new Set(astral.map((star) => star.dx)).size).toBeGreaterThan(1);
  });

  it("#2332 a press on an occupied zone's own element, beside its card, taps in that half's playmat", () => {
    const field = boardField();
    const zone = screen.getByTestId(testid.zone("you", "units", 1));
    expect(zone.querySelector(".card")).not.toBeNull();

    press(zone);
    expect(field.querySelector(".tap-mark--stone")).not.toBeNull();
    expect(playedIds()).toEqual([TAP_SFX[TAP_FAMILY["table-marble"]]]);
  });

  it("#2332 a press inside an occupied zone's card leaves no mark on the mat, and R3348000 the tile answers it", () => {
    const field = boardField();
    const zone = screen.getByTestId(testid.zone("you", "units", 1));
    const sprite = zone.querySelector<HTMLElement>(".card")?.querySelector<HTMLElement>("*");
    if (sprite === null || sprite === undefined) throw new Error("a unit's card must draw its face");

    press(sprite);
    expect(markCount(field)).toBe(0);
    expect(field.querySelectorAll(".tile-answer[data-occupied='true']")).toHaveLength(1);
    expect(playedIds()).toEqual([TAP_SFX[TAP_FAMILY["table-marble"]]]);
  });

  it("#2332 a press on a legal zone leaves no mark on the mat: the press plays into it, and R3348000 the tile answers", () => {
    const legal = testid.zone("you", "units", 1);
    const mounted = render(<Board view={baseView()} highlight={{ legal: new Set([legal]), selected: new Set() }} />);
    const field = mounted.container.querySelector<HTMLElement>(".field");
    if (field === null) throw new Error("fixture must draw a field");
    stubBounds(field);
    const zone = screen.getByTestId(legal);
    expect(zone.getAttribute("role")).toBe("button");

    press(zone);
    expect(markCount(field)).toBe(0);
    expect(field.querySelectorAll(".tile-answer")).toHaveLength(1);
  });
});

/** A board of empty tiles on `table`, a few of them legal, with the field's bounds stubbed. */
function tileBoard(options: {
  table?: string;
  loadouts?: SeatLoadouts;
  legal?: readonly string[];
  onClick?: (target: ClickTarget) => void;
}): HTMLElement {
  const mounted = render(
    <Board
      view={baseView()}
      wearCosmetics
      deckCosmetics={{ table: options.table ?? "table-marble", field: null, cardBack: null }}
      {...(options.loadouts === undefined ? {} : { loadouts: options.loadouts })}
      highlight={{ legal: new Set(options.legal ?? []), selected: new Set() }}
      {...(options.onClick === undefined ? {} : { onClick: options.onClick })}
    />,
  );
  const field = mounted.container.querySelector<HTMLElement>(".field");
  if (field === null) throw new Error("fixture must draw a field");
  stubBounds(field);
  return field;
}

const YOURS = testid.zone("you", "units", 2);
const THEIRS = testid.zone("opponent", "units", 2);

describe("R3348000 every tile answers a press", () => {
  it("R3348000 an illegal tile gets a ripple in its half's family and one sound, from the mat's own tap", () => {
    for (const id of Object.keys(FAMILY_OF)) {
      const field = tileBoard({ table: id });
      const family = FAMILY_OF[id] ?? "sand";
      audio.playSfx.mockClear();

      press(screen.getByTestId(YOURS));
      const answers = field.querySelectorAll(`.tile-answer[data-family="${family}"]`);
      expect(answers, `${id} ripples in ${family}`).toHaveLength(1);
      expect(answers[0]?.getAttribute("data-strength")).toBe("base");
      expect(playedIds(), `${id} sounds once, in ${family}`).toEqual([SFX_OF[family]]);
      cleanup();
    }
  });

  it("R3348000 a legal tile answers harder: sand-build 4 and a strong ripple, with one sound", () => {
    const field = tileBoard({ legal: [YOURS] });
    const zone = screen.getByTestId(YOURS);
    expect(zone.getAttribute("role")).toBe("button");

    press(zone);
    expect(field.querySelectorAll(".tile-answer[data-strength='strong']")).toHaveLength(1);
    expect(audio.playSfx).toHaveBeenCalledTimes(1);
    expect(audio.playSfx.mock.calls[0]?.[0]).toBe("tapStone");
    expect(audio.playSfx.mock.calls[0]?.[1]?.sandBuild).toBe(TILE_FEEL.legalBuild);
    expect(TILE_FEEL.legalBuild).toBeGreaterThan(TILE_FEEL.baseBuild);
  });

  it("R3348000 an occupied tile ripples round its card, and a press on its card answers too", () => {
    const field = boardField();
    press(screen.getByTestId(testid.zone("you", "units", 1)));
    const answer = field.querySelector(".tile-answer");
    expect(answer?.getAttribute("data-occupied")).toBe("true");
    expect(answer?.getAttribute("data-family")).toBe("stone");
  });

  it("R3348000 each half answers in its own playmat, the opponent's tile in theirs", () => {
    const loadouts: SeatLoadouts = {
      p1: { seat: { ...DEFAULT_CARD_COSMETICS }, cards: {} },
      p2: { seat: { ...DEFAULT_CARD_COSMETICS, table: "table-astral" }, cards: {} },
    };
    const field = tileBoard({ table: "table-marble", loadouts, legal: [THEIRS, YOURS] });

    press(screen.getByTestId(THEIRS));
    press(screen.getByTestId(YOURS), 30, 70, 2);
    const families = [...field.querySelectorAll(".tile-answer")].map((answer) => answer.getAttribute("data-family"));
    expect(families).toEqual(["astral", "stone"]);
    expect(playedIds()).toEqual(["tapAstral", "tapStone"]);
  });

  it("R3348000 a legal tile still plays when pressed, and an illegal one does nothing in the game", () => {
    const onClick = vi.fn<(target: ClickTarget) => void>();
    tileBoard({ legal: [YOURS], onClick });
    const legal = screen.getByTestId(YOURS);
    const illegal = screen.getByTestId(testid.zone("you", "units", 3));

    press(legal);
    fireEvent.click(legal);
    expect(onClick).toHaveBeenCalledTimes(1);
    expect(onClick).toHaveBeenCalledWith({ on: "zone", side: "you", row: "units", lane: 2 });

    onClick.mockClear();
    press(illegal, 30, 70, 2);
    fireEvent.click(illegal);
    expect(onClick).not.toHaveBeenCalled();
  });

  it("R3348000 a press is never stopped or default-prevented, so the game keeps it", () => {
    tileBoard({ legal: [YOURS] });
    const press = new MouseEvent("pointerdown", { bubbles: true, cancelable: true, clientX: 30, clientY: 70 });
    const stop = vi.spyOn(press, "stopPropagation");
    screen.getByTestId(YOURS).dispatchEvent(press);
    expect(press.defaultPrevented).toBe(false);
    expect(stop).not.toHaveBeenCalled();
  });

  it("R3348000 an answer clears after TILE_FEEL.rippleMs, and no more than TILE_FEEL.maximum show at once", () => {
    vi.useFakeTimers();
    const field = tileBoard({});
    const zone = screen.getByTestId(YOURS);

    press(zone);
    expect(field.querySelectorAll(".tile-answer")).toHaveLength(1);
    act(() => {
      vi.advanceTimersByTime(TILE_FEEL.rippleMs - 1);
    });
    expect(field.querySelectorAll(".tile-answer")).toHaveLength(1);
    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(field.querySelectorAll(".tile-answer")).toHaveLength(0);

    for (let tap = 0; tap < TILE_FEEL.maximum + 5; tap += 1) press(zone, 30, 70, tap + 1);
    expect(field.querySelectorAll(".tile-answer")).toHaveLength(TILE_FEEL.maximum);
  });

  it("R3348000 reduced motion, or effects off, draw the answer still and keep the sound", () => {
    const field = tileBoard({});
    press(screen.getByTestId(YOURS));
    expect(field.querySelector(".tile-answer")?.getAttribute("data-play")).toBe("move");

    writeSettings({ reduceMotion: true });
    press(screen.getByTestId(YOURS), 30, 70, 2);
    expect([...field.querySelectorAll(".tile-answer")].at(-1)?.getAttribute("data-play")).toBe("still");

    __resetSettingsForTests();
    setFxSettings({ intensity: "off" });
    press(screen.getByTestId(YOURS), 30, 70, 3);
    expect([...field.querySelectorAll(".tile-answer")].at(-1)?.getAttribute("data-play")).toBe("still");
    expect(audio.playSfx).toHaveBeenCalledTimes(3);
  });

  it("R3348000 a touch press buzzes, a mouse press does not, and the Vibration switch silences it", () => {
    const vibrate = vi.fn((_pattern: number[]) => true);
    Object.defineProperty(navigator, "vibrate", { configurable: true, value: vibrate });
    let t = 0;
    vi.spyOn(performance, "now").mockImplementation(() => (t += 1000));
    tileBoard({ legal: [YOURS] });

    fireEvent.pointerDown(screen.getByTestId(testid.zone("you", "units", 3)), { clientX: 30, clientY: 70, pointerId: 1, pointerType: "mouse" });
    expect(vibrate).not.toHaveBeenCalled();

    fireEvent.pointerDown(screen.getByTestId(testid.zone("you", "units", 3)), { clientX: 30, clientY: 70, pointerId: 2, pointerType: "touch" });
    expect(vibrate).toHaveBeenLastCalledWith([...HAPTIC_PATTERNS.tile]);

    fireEvent.pointerDown(screen.getByTestId(YOURS), { clientX: 30, clientY: 70, pointerId: 3, pointerType: "touch" });
    expect(vibrate).toHaveBeenLastCalledWith([...HAPTIC_PATTERNS.tileLegal]);

    vibrate.mockClear();
    writeHapticsSettings({ vibration: false });
    fireEvent.pointerDown(screen.getByTestId(YOURS), { clientX: 30, clientY: 70, pointerId: 4, pointerType: "touch" });
    expect(vibrate).not.toHaveBeenCalled();
  });
});
