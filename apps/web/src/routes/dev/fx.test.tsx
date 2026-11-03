// R2784000 #2792: `/dev/fx`, the motif gallery: it offers every drawn motif, every flavor's
// palette and sub-look, every gallery scale and every card whose table entry names a motif, and
// each plays something. R2785000: it plays a card as the fixture set piece, which holds the board
// until a click skips it. R2795000: it plays every Classic+ entry, a Field card wearing its aura.

import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";

import { planAuras } from "../../fx/aura/plan.ts";
import { isEntranceKey } from "../../fx/cards/bespoke.ts";
import { FX_CARD_TABLE } from "../../fx/cards/tables.ts";
import { FX_GALLERY_SCALES, FX_INTENSITY_SCALE } from "../../fx/constants.ts";
import { planFx } from "../../fx/cues.ts";
import { FLAVOR_SUB_LOOKS, FX_PALETTES } from "../../fx/flavors.ts";
import { createFxMemory } from "../../fx/memory.ts";
import { GALLERY_SET_PIECE, galleryCardEvents, galleryCues, galleryHookCues, galleryShownView } from "../../fx/motifs/gallery.ts";
import { FX_CARD_HOOKS, FX_DRAWN_MOTIFS, isDrawnMotif } from "../../fx/motifs/names.ts";
import { hasSetPiece } from "../../fx/setPieces.ts";
import type { FxCue } from "../../fx/types.ts";
import { BURST_BUDGET_MS, planEntries } from "../../game/animations.ts";
import { createPlayTracker } from "../../game/runs.ts";
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
    // R2795000: every entry but `none`, a bespoke entrance and one with no drawn hook (an aura alone).
    const offered = [...FX_CARD_TABLE].flatMap(([defId, entry]) => {
      if (entry.kind === "none" || (entry.kind === "bespoke" && isEntranceKey(entry.key))) return [];
      if (entry.kind === "hooks" && !FX_CARD_HOOKS.some((hook) => isDrawnMotif(entry.hooks[hook]?.motif ?? ""))) return [];
      return [defId];
    });
    expect(cards).toEqual(offered);
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

  it("R2785000 the gallery plays a card as the fixture set piece, which holds the board until a click skips it", () => {
    vi.useFakeTimers();
    try {
      render(<FxGallery />);
      // A card with no set piece of its own: the first offered (core-017 Flood) holds for its vortex.
      fireEvent.change(screen.getByTestId(fxGalleryTestid.card), { target: { value: "classic-071" } });
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

  it("R2795000 the gallery plays every Classic+ entry", () => {
    render(<FxGallery />);
    const offered = optionsOf(fxGalleryTestid.card);
    const view = fullBoardView();
    const classicPlus = galleryCards().filter((card) => card.defId.startsWith("classicplus-"));
    const given = [...FX_CARD_TABLE].filter(([defId, entry]) => defId.startsWith("classicplus-") && entry.kind !== "none");
    expect(classicPlus.map((card) => card.defId)).toEqual(given.map(([defId]) => defId));
    for (const card of classicPlus) {
      expect(offered, card.defId).toContain(card.defId);
      let cues: FxCue[];
      if (card.hook === "death" || card.hook === "trigger" || card.hook === "activate") {
        cues = galleryHookCues(card, view, FX_INTENSITY_SCALE.normal);
      } else {
        const events = galleryCardEvents(card, view);
        const env = { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory() };
        cues = planEntries(events, withEvents(view, events), false, createPlayTracker(), hasSetPiece).flatMap((entry) => {
          env.memory.remember(entry.events);
          return planFx(entry, entry.view, env);
        });
      }
      expect(cues.length, card.defId).toBeGreaterThan(0);
      if (card.hook === "setPiece") expect(cues.some((cue) => cue.kind === "setPiece"), card.defId).toBe(true);
    }
  });

  it("R2795000 a Field card's gallery view stands it face-up, wearing its aura", () => {
    const view = fullBoardView();
    const fields = galleryCards().filter((card) => card.defId.startsWith("classicplus-") && (CATALOG[card.defId]?.type === "Field Spell" || CATALOG[card.defId]?.type === "Field Trap"));
    expect(fields.length).toBeGreaterThanOrEqual(10);
    const wears = (shown: typeof view, defId: string): boolean => planAuras(shown, new Map()).some((aura) => aura.defId === defId);
    for (const card of fields) {
      const shown = galleryShownView(view, card, 1);
      expect(shown.you.backrow[0], card.defId).toEqual(expect.objectContaining({ defId: card.defId, faceDown: false }));
      expect(wears(view, card.defId), card.defId).toBe(false);
      expect(wears(shown, card.defId), card.defId).toBe(true);
    }
    // Any other card leaves the board as it stands.
    const spell = galleryCards().find((card) => card.defId === "classicplus-025");
    expect(spell && galleryShownView(view, spell).you).toBe(view.you);
  });
});
