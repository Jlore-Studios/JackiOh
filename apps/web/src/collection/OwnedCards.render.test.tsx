// #4511: the owned-cards grid builds each tile's face once and reuses it. Opening or closing the
// hover preview builds no tile's face, a scroll step builds faces only for the tiles it newly
// mounts, and a language switch rebuilds every mounted tile's face (R711000). The engine's `face`
// is counted (but still answered, so the grid draws as ever).

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import { resetCosmeticsForTests } from "../cards/cosmetics.ts";
import { HOVER_DELAY_MS } from "../cards/inspect/constants.ts";
import { resetOwnedFullArtForTests } from "../cards/ownedFullArt.ts";
import { resetLocaleForTests, setLocale } from "../i18n/i18n.ts";
import type { FaceCall } from "../wire/generated/FaceCall.ts";
import { collectionTestid as T } from "./collection.ts";
import { collectionOwning } from "./fixtures.ts";
import { LOOK_HOVER_CLOSE_MS } from "./LookHover.tsx";
import OwnedCards from "./OwnedCards.tsx";

/** Every `face` call the grid made since the calls were last cleared, in order. */
const faceCalls = vi.hoisted(() => [] as FaceCall[]);

vi.mock("../wasm/index.ts", async (importOriginal) => {
  const real = await importOriginal<typeof import("../wasm/index.ts")>();
  return {
    ...real,
    face: (request: FaceCall) => {
      faceCalls.push(request);
      return real.face(request);
    },
  };
});

/** Big D-fender, a Core Unit that prints full art. */
const A = "core-001";
/** Speciesist, a Core Unit. */
const B = "core-002";
/** Stockpile, a Core Spell. */
const C = "core-005";
/** Bread and Butter, a Core Field Trap. */
const D = "core-018";
/** A fifth Core card a scroll step newly mounts. */
const E = "core-019";

function defOf(id: string): CardDef {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`the catalog has no ${id}`);
  return def;
}

/** The grid of `ids`, each owned. */
function grid(ids: readonly string[]): void {
  render(<OwnedCards collection={collectionOwning(ids)} defs={ids.map(defOf)} />);
}

function tile(id: string): HTMLElement {
  return screen.getByTestId(T.card(id));
}

/** The `card` calls naming `id` since the calls were last cleared. */
function tileCalls(id: string): number {
  return faceCalls.filter((request) => request.call === "card" && request.source.defId === id).length;
}

beforeEach(() => {
  localStorage.clear();
  resetCosmeticsForTests();
  resetOwnedFullArtForTests();
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  localStorage.clear();
  resetCosmeticsForTests();
  resetOwnedFullArtForTests();
  faceCalls.length = 0;
  resetLocaleForTests();
});

describe("#4511 the owned-cards grid builds each tile's face once", () => {
  it("#4511 opening and closing the hover preview on one tile builds no tile's face", () => {
    grid([A, B, C, D]);
    for (const id of [A, B, C, D]) expect(tileCalls(id)).toBeGreaterThan(0);
    faceCalls.length = 0;

    fireEvent.pointerEnter(tile(A), { pointerType: "mouse" });
    act(() => {
      vi.advanceTimersByTime(HOVER_DELAY_MS);
    });
    expect(screen.getByTestId(T.lookHover).getAttribute("data-item")).toBe(A);
    // The panel draws the hovered card once; no tile rebuilds.
    expect(tileCalls(A)).toBe(1);
    for (const id of [B, C, D]) expect(tileCalls(id)).toBe(0);

    fireEvent.pointerLeave(tile(A), { pointerType: "mouse" });
    act(() => {
      vi.advanceTimersByTime(LOOK_HOVER_CLOSE_MS);
    });
    expect(screen.queryByTestId(T.lookHover)).toBeNull();
    expect(faceCalls.filter((request) => request.call === "card")).toHaveLength(1);
  });

  it("#4511 a scroll step builds faces only for the tiles it newly mounts", () => {
    const ids = [A, B, C, D];
    const { rerender } = render(<OwnedCards collection={collectionOwning(ids)} defs={ids.map(defOf)} />);
    faceCalls.length = 0;

    const next = [B, C, D, E];
    rerender(<OwnedCards collection={collectionOwning(next)} defs={next.map(defOf)} />);
    expect(screen.queryByTestId(T.card(A))).toBeNull();
    expect(screen.getByTestId(T.card(E))).toBeInTheDocument();
    const fresh = faceCalls.filter((request) => request.call === "card").map((request) => request.source.defId);
    expect(fresh.length).toBeGreaterThan(0);
    expect(new Set(fresh)).toEqual(new Set([E]));
  });

  it("#4511 a language switch rebuilds every mounted tile's face", async () => {
    grid([A, B, C, D]);
    faceCalls.length = 0;

    await act(async () => {
      await setLocale("zh-CN");
    });
    for (const id of [A, B, C, D]) expect(tileCalls(id)).toBeGreaterThan(0);
  });
});
