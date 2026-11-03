// #1954: one press on the empty field is one tap — one mark set and one sound — and each
// playmat taps in its family's look and sound.
// #2332: an occupied zone's own element taps too; only its card and the board's controls keep a press.

import { useRef, type ReactElement } from "react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { RAPID_SFX } from "../audio/constants.ts";
import { setAudioEngineForTests } from "../audio/engine.ts";
import { fakeAudioEngine, type FakeAudioEngine } from "../audio/test/fakeAudio.ts";
import type { SfxId } from "../audio/types.ts";
import { DEFAULT_CARD_COSMETICS, cosmeticsOf } from "../cards/cosmetics.ts";
import type { SeatLoadouts } from "../cards/loadouts.ts";
import { baseView, fullBoardView } from "../test/fixtures.ts";
import Board from "./Board.tsx";
import { testid } from "./contract.ts";
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

  it("#2332 a press inside an occupied zone's card does not tap", () => {
    const field = boardField();
    const zone = screen.getByTestId(testid.zone("you", "units", 1));
    const sprite = zone.querySelector<HTMLElement>(".card")?.querySelector<HTMLElement>("*");
    if (sprite === null || sprite === undefined) throw new Error("a unit's card must draw its face");

    press(sprite);
    expect(markCount(field)).toBe(0);
    expect(audio.playSfx).not.toHaveBeenCalled();
  });

  it("#2332 a press on a legal zone does not tap: the press plays into it", () => {
    const legal = testid.zone("you", "units", 1);
    const mounted = render(<Board view={baseView()} highlight={{ legal: new Set([legal]), selected: new Set() }} />);
    const field = mounted.container.querySelector<HTMLElement>(".field");
    if (field === null) throw new Error("fixture must draw a field");
    stubBounds(field);
    const zone = screen.getByTestId(legal);
    expect(zone.getAttribute("role")).toBe("button");

    press(zone);
    expect(markCount(field)).toBe(0);
    expect(audio.playSfx).not.toHaveBeenCalled();
  });
});
