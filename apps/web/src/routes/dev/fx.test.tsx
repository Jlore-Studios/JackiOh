// R2784000 #2792: `/dev/fx`, the motif gallery: it offers every drawn motif, every flavor's
// palette and sub-look, every gallery scale and every card whose table entry names a motif, and
// each plays something; #2793 and #2798: every Core and Stinkyscape part-two entry other than `none`
// plays as its card's play. R2785000: it plays a card as the fixture set piece, which holds the board
// until a click skips it.

import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";

import { planCardFx, planCardMotif } from "../../fx/cardFx.ts";
import { planAuras } from "../../fx/aura/plan.ts";
import { FX_CARD_FILES, FX_CARD_TABLE, parseFxCardTable } from "../../fx/cards/tables.ts";
import { FX_GALLERY_SCALES, FX_INTENSITY_SCALE } from "../../fx/constants.ts";
import { planFx } from "../../fx/cues.ts";
import { FLAVOR_SUB_LOOKS, FX_PALETTES } from "../../fx/flavors.ts";
import { createFxMemory } from "../../fx/memory.ts";
import { motifScale } from "../../fx/motifs/build.ts";
import {
  GALLERY_SET_PIECE,
  SYSTEM_MOMENTS,
  galleryCardEvents,
  galleryCues,
  galleryEntry,
  galleryFieldView,
  galleryPlayOf,
  systemMomentFixture,
} from "../../fx/motifs/gallery.ts";
import { hasSetPiece } from "../../fx/setPieces.ts";
import type { FxPlanEnv } from "../../fx/types.ts";
import { BURST_BUDGET_MS, planEntries } from "../../game/animations.ts";
import { FX_DRAWN_MOTIFS } from "../../fx/motifs/names.ts";
import { fullBoardView, withEvents } from "../../test/fixtures.ts";
import { CATALOG } from "../../wire/cards.ts";
import FxGallery, { fxGalleryTestid, galleryCards } from "./fx.tsx";

afterEach(cleanup);

function optionsOf(testid: string): string[] {
  return [...screen.getByTestId(testid).querySelectorAll("option")].map((option) => option.value);
}

describe("R2784000 #2792 the /dev/fx gallery and sub-look", () => {
  it("R2784000 #2792 the gallery lists every drawn motif, palette and sub-look, scale and example", () => {
    render(<FxGallery />);
    expect(screen.getByTestId(fxGalleryTestid.root)).toBeInTheDocument();
    expect(optionsOf(fxGalleryTestid.motif)).toEqual([...FX_DRAWN_MOTIFS]);
    expect(optionsOf(fxGalleryTestid.palette)).toEqual([...FX_PALETTES]);
    const subs = Object.values(FLAVOR_SUB_LOOKS).reduce((n, looks) => n + Object.keys(looks ?? {}).length, 0);
    expect(optionsOf(fxGalleryTestid.palette)).toHaveLength(15 + subs);
    expect(optionsOf(fxGalleryTestid.palette)).toContain("Tech/rocket");
    expect(optionsOf(fxGalleryTestid.scale)).toEqual(FX_GALLERY_SCALES.map(String));
    const cards = optionsOf(fxGalleryTestid.card);
    expect(cards).toEqual(galleryCards().map((card) => card.defId));
    // #2793: Core's 91 entries other than `none` (76 on the board, 7 through the director, 5 recipes, 3 set
    // pieces); #2798: Stinkyscape part two's 83; then the other sets' 12 motif cards, 7 aura-only Field
    // Spells, Crushing Walls' recipe, 4 set pieces and Training Grounds #30's `death: wipe` through the director.
    expect(cards).toHaveLength(199);
    expect(cards).toContain("classic-071");
    // Playing a motif and a card's entry draws without throwing, whatever is picked.
    fireEvent.change(screen.getByTestId(fxGalleryTestid.motif), { target: { value: "lance" } });
    fireEvent.change(screen.getByTestId(fxGalleryTestid.palette), { target: { value: "Tech/rocket" } });
    fireEvent.change(screen.getByTestId(fxGalleryTestid.scale), { target: { value: "2" } });
    fireEvent.click(screen.getByTestId(fxGalleryTestid.play));
    fireEvent.change(screen.getByTestId(fxGalleryTestid.card), { target: { value: "classic-071" } });
    fireEvent.click(screen.getByTestId(fxGalleryTestid.playCard));
    expect(screen.getByTestId(fxGalleryTestid.root)).toBeInTheDocument();
  });

  it("R2784000 #2792 every motif plays in every palette and sub-look at every scale on the fixture board", () => {
    const view = fullBoardView();
    for (const motif of FX_DRAWN_MOTIFS) {
      for (const palette of FX_PALETTES) {
        for (const scale of FX_GALLERY_SCALES) {
          expect(galleryCues(motif, palette, scale, view, FX_INTENSITY_SCALE.normal).length, `${motif} ${palette} ×${String(scale)}`).toBeGreaterThan(0);
        }
      }
      // The effects off plays nothing.
      expect(galleryCues(motif, "Fire", 1, view, FX_INTENSITY_SCALE.off)).toEqual([]);
    }
  });

  it("R2784000 the gallery lists and plays every system moment", () => {
    render(<FxGallery />);
    expect(optionsOf(fxGalleryTestid.moment)).toEqual([...SYSTEM_MOMENTS]);
    const base = fullBoardView();
    for (const moment of SYSTEM_MOMENTS) {
      const events = systemMomentFixture(moment, base);
      expect(events.length, `${moment} has a fixture`).toBeGreaterThan(0);
      const entry = galleryEntry(events, base);
      const memory = createFxMemory();
      memory.remember(entry.events);
      expect(planFx(entry, base, { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory }).length, `${moment} plans`).toBeGreaterThan(0);
    }
    fireEvent.change(screen.getByTestId(fxGalleryTestid.moment), { target: { value: "glitched" } });
    fireEvent.click(screen.getByTestId(fxGalleryTestid.playMoment));
    expect(screen.getByTestId(fxGalleryTestid.root)).toBeInTheDocument();
  });

  /**
   * Every entry of `ids` other than none has a gallery play, in table order, and each plays its motif, recipe
   * or set piece as the board's effects layer plans it; a Field card's aura stands where the gallery lays it.
   */
  function expectGalleryPlays(ids: ReadonlySet<string>, count: number): void {
    const view = fullBoardView();
    const intensity = FX_INTENSITY_SCALE.normal;
    const drawable = [...FX_CARD_TABLE].filter(([defId, entry]) => ids.has(defId) && entry.kind !== "none");
    expect(drawable).toHaveLength(count);
    expect(galleryCards().filter((card) => ids.has(card.defId)).map((card) => card.defId)).toEqual(drawable.map(([defId]) => defId));
    for (const [defId, entry] of drawable) {
      // R2787000: a Field card's aura stands in the backrow zone the gallery lays it in.
      if (entry.kind === "hooks" && entry.aura !== undefined && galleryFieldView(defId, view) !== view) {
        expect(planAuras(galleryFieldView(defId, view), new Map()).map((aura) => aura.defId), defId).toContain(defId);
      }
      const play = galleryPlayOf(defId, entry);
      if (play === null) throw new Error(`${defId} has no gallery play`);
      if (play.kind === "aura") continue;
      if (play.kind === "director") {
        expect(galleryCues(play.motif, play.palette, motifScale(defId, view, 1), view, intensity).length, defId).toBeGreaterThan(0);
        continue;
      }
      // Planned as the board's effects layer plans it: remember, then plan.
      const events = galleryCardEvents(defId, play, view, 1);
      const env: FxPlanEnv = { intensity, card: () => undefined, memory: createFxMemory() };
      const entries = planEntries(events, withEvents(galleryFieldView(defId, view), events), false, undefined, hasSetPiece);
      let drawn = false;
      for (const planned of entries) {
        env.memory.remember(planned.events);
        if (play.kind === "board" && planCardMotif(planned, planned.view, env) !== null) drawn = true;
        if (play.kind === "recipe" && planned.events.some((event) => planCardFx(event, { entry: planned, view: planned.view, env, D: planned.durationMs }) !== null)) drawn = true;
        if (play.kind === "setPiece" && planned.wipe?.by === defId) drawn = true;
      }
      expect(drawn, `${defId} ${play.kind}`).toBe(true);
    }
  }

  it("R2784000 #2793 the gallery has a play for every Core entry other than none, and each plays its motif, recipe or set piece", () => {
    expectGalleryPlays(new Set(Object.keys(CATALOG).filter((id) => CATALOG[id]?.set === "Core")), 91);
  });

  it("R2784000 #2798 the gallery has a play for every Stinkyscape part-two entry other than none, and each plays its motif, recipe, set piece or aura", () => {
    const file = FX_CARD_FILES.find((candidate) => candidate.file === "stinkyscape-2.json5");
    if (file === undefined) throw new Error("no stinkyscape-2.json5");
    expectGalleryPlays(new Set(parseFxCardTable(file.file, file.source).keys()), 83);
  });

  it("R2784000 #2793 the gallery plays a card of each kind, a Field Spell standing in the backrow with its aura", () => {
    vi.useFakeTimers();
    // jsdom lays nothing out: give every zone a box, so the aura layer measures the Field Spell's side.
    const box = vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue(new DOMRect(0, 0, 100, 100));
    try {
      render(<FxGallery />);
      // A cry motif, a trigger through the director, a recipe, a set piece and a Field Spell's cast and aura.
      for (const defId of ["core-002", "core-013", "core-083", "core-017", "core-014"]) {
        fireEvent.change(screen.getByTestId(fxGalleryTestid.card), { target: { value: defId } });
        fireEvent.click(screen.getByTestId(fxGalleryTestid.playCard));
        act(() => {
          vi.advanceTimersByTime(BURST_BUDGET_MS * 4);
        });
      }
      expect(document.querySelector('[data-aura-look="storm"]')).not.toBeNull();
    } finally {
      box.mockRestore();
      vi.useRealTimers();
    }
  });

  it("R2800000 the gallery plays Twisting Nether as its own set piece", () => {
    vi.useFakeTimers();
    try {
      render(<FxGallery />);
      const game = screen.getByTestId("game");
      fireEvent.change(screen.getByTestId(fxGalleryTestid.card), { target: { value: "core-088" } });
      fireEvent.click(screen.getByTestId(fxGalleryTestid.playCard));
      // The play's own entry, then its wipe, which holds the board as the Nether.
      const inFlight = (): string | null => screen.queryByTestId("animation-queue")?.getAttribute("data-animating") ?? null;
      for (let waited = 0; waited < BURST_BUDGET_MS && inFlight() !== "destroyed"; waited += 10) {
        act(() => {
          vi.advanceTimersByTime(10);
        });
      }
      expect(game).toHaveAttribute("data-set-piece", "nether");
    } finally {
      vi.useRealTimers();
    }
  });

  it("R2785000 the gallery plays a card as the fixture set piece, which holds the board until a click skips it", () => {
    vi.useFakeTimers();
    try {
      render(<FxGallery />);
      const game = screen.getByTestId("game");
      fireEvent.click(screen.getByTestId(fxGalleryTestid.playSetPiece));
      expect(game).toHaveAttribute("data-set-piece", GALLERY_SET_PIECE);
      expect(screen.getByTestId("set-piece-skip")).toHaveTextContent("Tap to skip");
      fireEvent.click(game);
      expect(game).not.toHaveAttribute("data-set-piece");
      expect(screen.queryByTestId("set-piece-skip")).toBeNull();

      // Once the board has caught up, a plain play holds nothing.
      act(() => {
        vi.advanceTimersByTime(BURST_BUDGET_MS * 4);
      });
      fireEvent.click(screen.getByTestId(fxGalleryTestid.playCard));
      expect(game.querySelector("[data-animating]")).not.toBeNull();
      expect(game).not.toHaveAttribute("data-set-piece");
    } finally {
      vi.useRealTimers();
    }
  });
});
