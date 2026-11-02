// R2784000: `/dev/fx`, the motif gallery: it offers every drawn motif, every flavor's palette, every
// gallery scale and every card whose table entry names a motif, and each plays something. R2785000: it
// plays a card as the fixture set piece, which holds the board until a click skips it.

import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";

import { FX_GALLERY_SCALES, FX_INTENSITY_SCALE } from "../../fx/constants.ts";
import { GALLERY_SET_PIECE, galleryCues } from "../../fx/motifs/gallery.ts";
import { BURST_BUDGET_MS } from "../../game/animations.ts";
import { FX_DRAWN_MOTIFS } from "../../fx/motifs/names.ts";
import { fullBoardView } from "../../test/fixtures.ts";
import FxGallery, { FLAVORS, fxGalleryTestid, galleryCards } from "./fx.tsx";

afterEach(cleanup);

function optionsOf(testid: string): string[] {
  return [...screen.getByTestId(testid).querySelectorAll("option")].map((option) => option.value);
}

describe("R2784000 the /dev/fx gallery", () => {
  it("R2784000 the gallery lists every drawn motif, flavor, scale and example", () => {
    render(<FxGallery />);
    expect(screen.getByTestId(fxGalleryTestid.root)).toBeInTheDocument();
    expect(optionsOf(fxGalleryTestid.motif)).toEqual([...FX_DRAWN_MOTIFS]);
    expect(optionsOf(fxGalleryTestid.palette)).toEqual(FLAVORS);
    expect(FLAVORS).toHaveLength(15);
    expect(optionsOf(fxGalleryTestid.scale)).toEqual(FX_GALLERY_SCALES.map(String));
    const cards = optionsOf(fxGalleryTestid.card);
    expect(cards).toEqual(galleryCards().map((card) => card.defId));
    // The twelve examples but #88 Twisting Nether, now a set piece, and the five board clears that take `wipe` (R2800000).
    expect(cards).toHaveLength(16);
    expect(cards).toContain("classic-071");
    // Playing a motif and a card's entry draws without throwing, whatever is picked.
    fireEvent.change(screen.getByTestId(fxGalleryTestid.motif), { target: { value: "lance" } });
    fireEvent.change(screen.getByTestId(fxGalleryTestid.palette), { target: { value: "Tech" } });
    fireEvent.change(screen.getByTestId(fxGalleryTestid.scale), { target: { value: "2" } });
    fireEvent.click(screen.getByTestId(fxGalleryTestid.play));
    fireEvent.change(screen.getByTestId(fxGalleryTestid.card), { target: { value: "classic-071" } });
    fireEvent.click(screen.getByTestId(fxGalleryTestid.playCard));
    expect(screen.getByTestId(fxGalleryTestid.root)).toBeInTheDocument();
  });

  it("R2784000 every motif plays in every palette at every scale on the fixture board", () => {
    const view = fullBoardView();
    for (const motif of FX_DRAWN_MOTIFS) {
      for (const flavor of FLAVORS) {
        for (const scale of FX_GALLERY_SCALES) {
          expect(galleryCues(motif, flavor, scale, view, FX_INTENSITY_SCALE.normal).length, `${motif} ${flavor} ×${String(scale)}`).toBeGreaterThan(0);
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
});
