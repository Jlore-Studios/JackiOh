// R2784000 #2792: `/dev/fx`, the motif gallery: it offers every drawn motif, every flavor's
// palette and sub-look, every gallery scale and every card whose table entry it can play, and
// each plays something. R2785000: it plays a card as the fixture set piece, which holds the board
// until a click skips it. #2796: it plays every Meditative and Meditative+ entry.

import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";

import type { PlayerView } from "@jackioh/shared";

import { planAuras } from "../../fx/aura/plan.ts";
import { planCardFx, planCardMotif } from "../../fx/cardFx.ts";
import { FX_CARD_FILES, parseFxCardTable } from "../../fx/cards/tables.ts";
import { FX_GALLERY_SCALES, FX_INTENSITY_SCALE } from "../../fx/constants.ts";
import { FLAVOR_SUB_LOOKS, FX_PALETTES } from "../../fx/flavors.ts";
import { createFxMemory } from "../../fx/memory.ts";
import { GALLERY_SET_PIECE, galleryCues, galleryHookCues, galleryPlaysOnBoard, galleryShownView, type GalleryCard } from "../../fx/motifs/gallery.ts";
import type { FxPlanEnv } from "../../fx/types.ts";
import { BURST_BUDGET_MS, planEntries } from "../../game/animations.ts";
import { FX_DRAWN_MOTIFS } from "../../fx/motifs/names.ts";
import { fullBoardView } from "../../test/fixtures.ts";
import FxGallery, { fxGalleryTestid, galleryCards } from "./fx.tsx";

afterEach(cleanup);

function optionsOf(testid: string): string[] {
  return [...screen.getByTestId(testid).querySelectorAll("option")].map((option) => option.value);
}

/** #2796: every Meditative and Meditative+ id whose entry is not `none`. */
function meditativeDrawn(): string[] {
  return FX_CARD_FILES.filter((file) => file.file === "meditative.json5" || file.file === "meditativeplus.json5").flatMap((file) =>
    [...parseFxCardTable(file.file, file.source)].flatMap(([id, entry]) => (entry.kind === "none" ? [] : [id])),
  );
}

/** Whether the board draws the card's play: its motif on some entry, or its recipe on some event, planned as FxLayer plans (remember, then plan). */
function boardDraws(card: GalleryCard, view: PlayerView): boolean {
  const shown = galleryShownView(view, card);
  const env: FxPlanEnv = { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory() };
  let draws = false;
  for (const entry of planEntries(shown.events, shown, false)) {
    env.memory.remember(entry.events);
    if (planCardMotif(entry, entry.view, env) !== null) draws = true;
    for (const event of entry.events) if (planCardFx(event, { entry, view: entry.view, env, D: entry.durationMs }) !== null) draws = true;
  }
  return draws;
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
    // #2796: every Meditative and Meditative+ entry that is not `none`, among the other sets' cards.
    for (const id of meditativeDrawn()) expect(cards, id).toContain(id);
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

  it("R2784000 #2796 the gallery plays every Meditative and Meditative+ entry", () => {
    const view = fullBoardView();
    const drawn = new Set(meditativeDrawn());
    const cards = galleryCards().filter((card) => drawn.has(card.defId));
    expect(cards.map((card) => card.defId)).toEqual([...drawn]);
    for (const card of cards) {
      if (card.hook === "aura") {
        // An entry that is only an aura shows it, and nothing else, as it stands in its zone.
        expect(planAuras(galleryShownView(view, card), new Map()).map((aura) => aura.defId), card.defId).toContain(card.defId);
      } else if (galleryPlaysOnBoard(card)) {
        expect(boardDraws(card, view), `${card.defId} ${card.hook}: ${card.fixture}`).toBe(true);
      } else {
        expect(galleryHookCues(card, view, FX_INTENSITY_SCALE.normal).length, `${card.defId} ${card.hook}: ${card.fixture}`).toBeGreaterThan(0);
      }
      // A Field card's aura stands in the first backrow zone while its play shows.
      const entry = FX_CARD_FILES.flatMap((file) => [...parseFxCardTable(file.file, file.source)]).find(([id]) => id === card.defId)?.[1];
      if (entry?.kind === "hooks" && entry.aura !== undefined) {
        expect(planAuras(galleryShownView(view, card), new Map()).map((aura) => aura.defId), card.defId).toContain(card.defId);
      }
    }
    // The effects off plays no hook's motif.
    const hook = cards.find((card) => !galleryPlaysOnBoard(card));
    expect(hook).toBeDefined();
    if (hook !== undefined) expect(galleryHookCues(hook, view, FX_INTENSITY_SCALE.off)).toEqual([]);
  });
});
