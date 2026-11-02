// R1833000: a cosmetic's look where a surface grants or sells one. A full art, and a per-card
// cosmetic with no card to draw it on, have none.

import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { DEFAULT_CARD_COSMETICS } from "../cards/cosmetics.ts";
import { CosmeticLook, hasLook } from "./CosmeticLook.tsx";

/** A Common card the catalog has, which wears every cosmetic foil (no rarity foil of its own). */
const CARD = "core-004";
/** A Mythic card, whose own foil a cosmetic foil never covers. */
const MYTHIC = "core-096";

afterEach(() => {
  cleanup();
});

describe("R1833000 a cosmetic's look", () => {
  it("R1833000 CosmeticLook draws each kind's look and hasLook says when there is one", () => {
    // A per-card cosmetic needs a card the catalog has; an account one, a portrait among them, needs none.
    for (const id of ["foil-void-shimmer", "finish-starlit", "border-celestial"]) {
      expect(hasLook(id, CARD), id).toBe(true);
      expect(hasLook(id, null), id).toBe(false);
      expect(hasLook(id, "no-such-card"), id).toBe(false);
    }
    for (const id of ["table-astral", "field-cosmic-drift", "card-back-nebula", "portrait-gary"]) expect(hasLook(id, null), id).toBe(true);
    expect(hasLook("full-art-gary-the-gambler", "core-004")).toBe(false);
    expect(hasLook("no-such-cosmetic", CARD)).toBe(false);

    // A foil on its card's face, moving, as a chase does (R1953001); a moving finish or border too.
    const { container, rerender } = render(<CosmeticLook id="foil-void-shimmer" cardId={CARD} />);
    const look = (): HTMLElement => {
      const found = container.querySelector<HTMLElement>(".cosmetic-look");
      if (found === null) throw new Error("no look");
      return found;
    };
    expect(look()).toHaveAttribute("data-testid", "cosmetic-look-foil-void-shimmer");
    expect(look().querySelector(".cf")).toHaveAttribute("data-cosmetic-foil", "foil-void-shimmer");
    expect(look().querySelector(".cf")).toHaveAttribute("data-cosmetic-motion", "animated");
    rerender(<CosmeticLook id="finish-starlit" cardId={CARD} />);
    expect(look().querySelector(".cf")).toHaveAttribute("data-finish", "finish-starlit");
    expect(look().querySelector(".cf")).toHaveAttribute("data-cosmetic-motion", "animated");
    expect(look().querySelector(".cf")).not.toHaveAttribute("data-cosmetic-foil");
    rerender(<CosmeticLook id="border-celestial" cardId={CARD} />);
    expect(look().querySelector(".cf")).toHaveAttribute("data-frame", "border-celestial");
    // A Mythic card keeps its own foil, as it does in a game.
    rerender(<CosmeticLook id="foil-void-shimmer" cardId={MYTHIC} />);
    expect(look().querySelector(".cf")).not.toHaveAttribute("data-cosmetic-foil");

    // A playmat is a mini board over no backdrop; a backdrop, a mini board on the default playmat.
    rerender(<CosmeticLook id="table-astral" cardId={null} />);
    expect(look().querySelector(".collection-miniboard")).toHaveAttribute("data-table", "table-astral");
    expect(look().querySelector(".board-backdrop")).toBeNull();
    rerender(<CosmeticLook id="field-cosmic-drift" cardId={CARD} />);
    expect(look().querySelector(".collection-miniboard")).toHaveAttribute("data-table", DEFAULT_CARD_COSMETICS.table);
    expect(look().querySelector(".board-backdrop")).toHaveAttribute("data-backdrop", "field-cosmic-drift");
    rerender(<CosmeticLook id="card-back-nebula" cardId={null} />);
    expect(look().querySelector(".cf-back")).toHaveAttribute("data-card-back", "card-back-nebula");
    expect(look().querySelector(".cf")).toBeNull();
    // A portrait's art (R1827000).
    rerender(<CosmeticLook id="portrait-gary" cardId={null} />);
    expect(container.querySelector('[data-portrait-art="gary"]')).toHaveClass("pack-slot__portrait");
    expect(container.querySelector(".cosmetic-look")).toBeNull();

    rerender(<CosmeticLook id="full-art-gary-the-gambler" cardId="core-004" />);
    expect(container.innerHTML).toBe("");
    rerender(<CosmeticLook id="foil-void-shimmer" cardId={null} />);
    expect(container.innerHTML).toBe("");
  });
});
