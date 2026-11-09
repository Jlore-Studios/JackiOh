// R3335000: the Collection's Starship tab: the look every Starship the account launches wears, then a row of tiles per
// kind, each the equipped look with its own pick in place. An owned tile equips with no request; a locked one says how
// it is earned (R3335002: its Fleet track level, or reaching Jlorious) and offers no token.

import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { starshipArtOf } from "../cards/art/starshipHulls.ts";
import { STARSHIP_LOOK_STANDARD } from "../cards/art/starshipLooks.ts";
import { starshipArtUri } from "../cards/art/svg.ts";
import {
  AccountCosmeticsOwned,
  DEFAULT_CARD_COSMETICS,
  STARSHIP_SLOT_KINDS,
  cosmeticsOf,
  readCosmetics,
  resetCosmeticsForTests,
  writeCosmetics,
} from "../cards/cosmetics.ts";
import { collectionTestid as T } from "./collection.ts";
import StarshipLooks from "./StarshipLooks.tsx";
import { STARSHIP_PREVIEW_NAME, STARSHIP_PREVIEW_PIECES } from "./StarshipPreview.tsx";

beforeEach(() => {
  localStorage.clear();
  resetCosmeticsForTests();
});

afterEach(() => {
  cleanup();
  localStorage.clear();
  resetCosmeticsForTests();
});

/** The picture a tile or the equipped preview draws. */
function pictureOf(element: HTMLElement): HTMLImageElement {
  const found = element.querySelector<HTMLImageElement>("img.collection-starship");
  if (found === null) throw new Error("no Starship picture");
  return found;
}

describe("R3335000 the Starship looks", () => {
  it("R3335000 equips owned picks, locks the rest", () => {
    // The account owns two Fleet rewards; the standards are every account's.
    render(
      <AccountCosmeticsOwned owned={new Set(["starship-hull-scout", "starship-livery-hazard"])}>
        <StarshipLooks />
      </AccountCosmeticsOwned>,
    );

    // A row per kind, every one of its registry entries a tile, its standard first and equipped.
    for (const kind of STARSHIP_SLOT_KINDS) {
      const row = screen.getByTestId(T.tiles(kind));
      const tiles = within(row).getAllByRole("listitem");
      expect(tiles.map((tile) => tile.getAttribute("data-testid")), kind).toEqual(cosmeticsOf(kind).map((cosmetic) => T.tile(cosmetic.id)));
      expect(tiles[0], kind).toHaveAttribute("data-equipped", "true");
      expect(tiles[0], kind).toHaveAttribute("data-owned", "true");
    }
    const equipped = screen.getByTestId(T.starshipEquipped);
    expect(pictureOf(equipped).src).toBe(starshipArtUri(starshipArtOf(STARSHIP_PREVIEW_PIECES, true, STARSHIP_PREVIEW_NAME, false, STARSHIP_LOOK_STANDARD)));

    // Each tile draws the equipped look with its own pick in place.
    const scout = screen.getByTestId(T.tile("starship-hull-scout"));
    expect(pictureOf(scout)).toHaveAttribute("data-hull", "starship-hull-scout");
    expect(pictureOf(scout)).toHaveAttribute("data-livery", DEFAULT_CARD_COSMETICS.starshipLivery);

    // An owned tile equips with a click, and the preview and every other tile wear it.
    fireEvent.click(within(scout).getByRole("button"));
    expect(readCosmetics().starshipHull).toBe("starship-hull-scout");
    expect(scout).toHaveAttribute("data-equipped", "true");
    expect(pictureOf(equipped)).toHaveAttribute("data-hull", "starship-hull-scout");
    expect(pictureOf(screen.getByTestId(T.tile("starship-livery-hazard")))).toHaveAttribute("data-hull", "starship-hull-scout");
    fireEvent.click(within(screen.getByTestId(T.tile("starship-livery-hazard"))).getByRole("button"));
    expect(readCosmetics().starshipLivery).toBe("starship-livery-hazard");

    // A locked tile equips nothing, says how it is earned and offers no token.
    const corsair = screen.getByTestId(T.tile("starship-hull-corsair"));
    expect(corsair).toHaveAttribute("data-owned", "false");
    expect(within(corsair).getByTestId(T.tileEarn("starship-hull-corsair"))).toHaveTextContent("Fleet track level 15");
    expect(within(corsair).getByRole("button")).toHaveAttribute("aria-disabled", "true");
    fireEvent.click(within(corsair).getByRole("button"));
    expect(readCosmetics().starshipHull).toBe("starship-hull-scout");
    expect(within(screen.getByTestId(T.tile("starship-hull-jlorious"))).getByTestId(T.tileEarn("starship-hull-jlorious"))).toHaveTextContent(
      "Jlorious on the ladder",
    );
    expect(document.querySelector('[data-testid^="collection-token-offer-"]')).toBeNull();
  });

  it("R3335000 a stored pick the account does not own shows as its kind's standard", () => {
    writeCosmetics({ starshipTrail: "starship-trail-singularity" });
    render(
      <AccountCosmeticsOwned owned={new Set()}>
        <StarshipLooks />
      </AccountCosmeticsOwned>,
    );
    expect(screen.getByTestId(T.tile("starship-trail-standard"))).toHaveAttribute("data-equipped", "true");
    expect(screen.getByTestId(T.tile("starship-trail-singularity"))).toHaveAttribute("data-equipped", "false");
    expect(pictureOf(screen.getByTestId(T.starshipEquipped))).toHaveAttribute("data-trail", "starship-trail-standard");
    cleanup();
    // With no ledger above it (practice, hotseat), every one is owned and the pick is worn.
    render(<StarshipLooks />);
    expect(pictureOf(screen.getByTestId(T.starshipEquipped))).toHaveAttribute("data-trail", "starship-trail-singularity");
    expect(screen.queryByTestId(T.tileEarn("starship-trail-singularity"))).toBeNull();
  });
});
