// #1954: one press on the empty field is one tap — one mark set and one sound — and each
// playmat taps in its family's look and sound. The drag trail is gone: holding or dragging
// after a press adds no mark and no `playSfx` call.

import { useRef, type ReactElement } from "react";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { RAPID_SFX } from "../audio/constants.ts";
import { setAudioEngineForTests } from "../audio/engine.ts";
import { fakeAudioEngine, type FakeAudioEngine } from "../audio/test/fakeAudio.ts";
import type { SfxId } from "../audio/types.ts";
import { cosmeticsOf } from "../cards/cosmetics.ts";
import { fullBoardView } from "../test/fixtures.ts";
import Board from "./Board.tsx";
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

/** Sand draws its grain; every other family draws its own tap mark. */
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

function Field({ table }: { table?: string }): ReactElement {
  const field = useRef<HTMLDivElement>(null);
  return (
    <div data-testid="field" ref={field}>
      <SandSurface field={field} disabled={false} table={table} />
    </div>
  );
}

/** The Board test's bounds: a 100×100 field at the origin, so client (30, 40) taps (30, 40). */
function stubBounds(field: HTMLElement): void {
  Object.defineProperty(field, "getBoundingClientRect", {
    configurable: true,
    value: () => ({ left: 0, top: 0, width: 100, height: 100 }),
  });
}

function press(field: HTMLElement, x = 30, y = 40, pointerId = 1): void {
  fireEvent.pointerDown(field, { clientX: x, clientY: y, pointerId });
}

function markCount(field: HTMLElement): number {
  return field.querySelectorAll(".sand-mark, .tap-mark").length;
}

function playedIds(): SfxId[] {
  return audio.playSfx.mock.calls.map(([id]) => id);
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
    expect(TAP_FEEL.neonColours).toContain(neon[0]?.tint);
    const hues = new Set(Array.from({ length: 200 }, (_, index) => tapMarks("neon", 30, 40, index + 1, 1)[0]?.tint));
    expect(hues.size).toBe(TAP_FEEL.neonColours.length);

    expect(astral).toHaveLength(TAP_FEEL.astralStars);
    for (const star of astral) {
      expect(TAP_FEEL.astralColours).toContain(star.tint);
      expect(Math.hypot(star.dx ?? Number.NaN, star.dy ?? Number.NaN)).toBeLessThanOrEqual(TAP_FEEL.astralSpreadPx);
    }
    expect(new Set(astral.map((star) => star.dx)).size).toBeGreaterThan(1);
  });
});
