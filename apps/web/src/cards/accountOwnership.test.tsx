// R1833000: the account's playmats, backdrops and card backs are earned: while ownership decides
// (`AccountCosmeticsOwned`, which ownedFullArt.ts mounts from `GET /api/cosmetics`) the pickers offer
// only the defaults, the starters and what it owns; with nothing deciding (practice, hotseat,
// R1310004's builds), all. R1953001: no face wears the account-wide foil, finish or border.

import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";

import AccountCosmeticTiles from "../collection/AccountCosmeticTiles.tsx";
import { collectionTestid } from "../collection/collection.ts";
import Board from "../game/Board.tsx";
import DeckCosmeticsPicker from "../game/deckbuilder/DeckCosmeticsPicker.tsx";
import type { DeckItem } from "../game/deckbuilder/sync.ts";
import { DECK_COSMETICS_BUTTON } from "../game/deckbuilder/testids.ts";
import type { OwnedCosmeticsResponse } from "../net/api.ts";
import { fullBoardView } from "../test/fixtures.ts";
import { CardFace } from "./CardFace.tsx";
import {
  AccountCosmeticsOwned,
  DECK_SLOT_KINDS,
  DEFAULT_CARD_COSMETICS,
  cosmeticsOf,
  deckCosmeticsOf,
  normalizeCosmetics,
  ownsAccountCosmetic,
  readCosmetics,
  resetCosmeticsForTests,
  useOwnsAccountCosmetic,
  wornDeckCosmetics,
  writeCardPick,
  writeCosmetics,
  type AccountOwnership,
  type DeckSlotKind,
  type OwnedByCard,
} from "./cosmetics.ts";
import { FullArtOwned, NO_FULL_ART } from "./fullArt.ts";
import { faceModel } from "./model.ts";
import {
  NO_ACCOUNT_COSMETICS,
  ViewerAccountCosmetics,
  ownedAccountCosmetics,
  readOwnedAccountCosmetics,
  readOwnedFullArt,
  resetOwnedFullArtForTests,
} from "./ownedFullArt.ts";

/** #1833's: a Mythic playmat, an Epic backdrop and a Legendary card back, none of them starters. */
const ASTRAL = "table-astral";
const SNOW = "field-snow";
const CLOCKWORK = "card-back-clockwork";

function everyAccounts(kind: DeckSlotKind | "foil" | "finish" | "border"): string[] {
  return cosmeticsOf(kind)
    .filter((cosmetic) => cosmetic.default || cosmetic.starter)
    .map((cosmetic) => cosmetic.id);
}

function under(owned: AccountOwnership | null, element: ReactElement): ReactElement {
  return owned === null ? element : <AccountCosmeticsOwned owned={owned}>{element}</AccountCosmeticsOwned>;
}

function deck(fields: Partial<DeckItem> = {}): DeckItem {
  return { id: "d1", name: "Deck", cards: [], portrait: null, createdAt: 0, updatedAt: 0, ...fields };
}

/** The deck picker's options of `kind`, the panel open. */
function pickerOptions(kind: DeckSlotKind): string[] {
  const select = screen.getByTestId<HTMLSelectElement>(`deck-cosmetic-${kind}`);
  return [...select.options].map((option) => option.value);
}

beforeEach(() => {
  localStorage.clear();
  resetCosmeticsForTests();
});

afterEach(() => {
  cleanup();
  resetCosmeticsForTests();
  resetOwnedFullArtForTests();
  localStorage.clear();
});

describe("R1833000 the account cosmetics a picker offers and a side wears", () => {
  it("R1833000 ownsAccountCosmetic owns every one where nothing decides, else the defaults, the starters and what the account lists", () => {
    for (const kind of DECK_SLOT_KINDS) {
      for (const cosmetic of cosmeticsOf(kind)) {
        expect(ownsAccountCosmetic("every", cosmetic.id), cosmetic.id).toBe(true);
        expect(ownsAccountCosmetic(NO_ACCOUNT_COSMETICS, cosmetic.id), cosmetic.id).toBe(cosmetic.default || cosmetic.starter);
      }
    }
    expect(ownsAccountCosmetic(new Set([ASTRAL]), ASTRAL)).toBe(true);
    expect(ownsAccountCosmetic(new Set([ASTRAL]), SNOW)).toBe(false);
    // An id the registry does not know is never owned while ownership decides.
    expect(ownsAccountCosmetic(new Set(["table-unknown"]), "table-unknown")).toBe(false);

    // A per-card cosmetic or an unknown id in the account list is no account cosmetic.
    const owned: OwnedCosmeticsResponse = { cards: [{ cardId: "core-004", cosmeticId: "foil-galaxy" }], account: [ASTRAL, "foil-galaxy", "table-unknown"] };
    expect(ownedAccountCosmetics(owned)).toEqual(new Set([ASTRAL]));
  });

  it("R1833000 the tiles lock unowned ones while ownership decides, and none otherwise", () => {
    const { rerender } = render(under(NO_ACCOUNT_COSMETICS, <AccountCosmeticTiles />));
    for (const kind of DECK_SLOT_KINDS) {
      const row = within(screen.getByTestId(collectionTestid.tiles(kind)));
      const owned = row.getAllByRole("listitem").filter((tile) => tile.getAttribute("data-owned") === "true");
      expect(owned.map((tile) => tile.getAttribute("data-testid"))).toEqual(everyAccounts(kind).map((id) => collectionTestid.tile(id)));
    }
    const astral = screen.getByTestId(collectionTestid.tile(ASTRAL));
    expect(astral).toHaveAttribute("data-owned", "false");
    fireEvent.click(within(astral).getByRole("button"));
    expect(readCosmetics().table).toBe(DEFAULT_CARD_COSMETICS.table);

    rerender(under(new Set([ASTRAL]), <AccountCosmeticTiles />));
    expect(screen.getByTestId(collectionTestid.tile(ASTRAL))).toHaveAttribute("data-owned", "true");
    expect(screen.getByTestId(collectionTestid.tile(SNOW))).toHaveAttribute("data-owned", "false");

    // Nothing deciding: every tile is owned.
    rerender(under(null, <AccountCosmeticTiles />));
    for (const kind of DECK_SLOT_KINDS) {
      for (const cosmetic of cosmeticsOf(kind)) {
        expect(screen.getByTestId(collectionTestid.tile(cosmetic.id)), cosmetic.id).toHaveAttribute("data-owned", "true");
      }
    }
  });

  it("R1833000 the deck picker offers starters and owned ones while ownership decides, every one otherwise", () => {
    const onCosmetic = vi.fn();
    const { rerender } = render(under(NO_ACCOUNT_COSMETICS, <DeckCosmeticsPicker deck={deck()} onCosmetic={onCosmetic} />));
    fireEvent.click(screen.getByTestId(DECK_COSMETICS_BUTTON));
    for (const kind of DECK_SLOT_KINDS) expect(pickerOptions(kind), kind).toEqual(["", ...everyAccounts(kind)]);

    rerender(under(new Set([ASTRAL, CLOCKWORK]), <DeckCosmeticsPicker deck={deck()} onCosmetic={onCosmetic} />));
    expect(pickerOptions("table")).toEqual(["", ...everyAccounts("table"), ASTRAL]);
    expect(pickerOptions("field")).toEqual(["", ...everyAccounts("field")]);
    expect(pickerOptions("cardBack")).toEqual(["", ...everyAccounts("cardBack"), CLOCKWORK]);
    fireEvent.change(screen.getByTestId("deck-cosmetic-table"), { target: { value: ASTRAL } });
    expect(onCosmetic).toHaveBeenCalledWith("table", ASTRAL);

    // Nothing deciding: a picker of its own, which opens closed, offers every one.
    rerender(under(null, <DeckCosmeticsPicker deck={deck()} onCosmetic={onCosmetic} />));
    fireEvent.click(screen.getByTestId(DECK_COSMETICS_BUTTON));
    for (const kind of DECK_SLOT_KINDS) expect(pickerOptions(kind), kind).toEqual(["", ...cosmeticsOf(kind).map((cosmetic) => cosmetic.id)]);
  });

  it("R1833000 an unowned stored pick renders as its kind's default", () => {
    const owns = (id: string): boolean => ownsAccountCosmetic(NO_ACCOUNT_COSMETICS, id);
    const account = normalizeCosmetics({ table: ASTRAL, field: SNOW, cardBack: "card-back-lacquer" });
    // The account's picks: the unowned ones draw the default, the starter stays.
    expect(wornDeckCosmetics(true, null, account, owns)).toEqual({
      table: DEFAULT_CARD_COSMETICS.table,
      field: DEFAULT_CARD_COSMETICS.field,
      cardBack: "card-back-lacquer",
    });
    // A deck's own pick the account does not own draws the default too, and the account's does not step in.
    const decked = deckCosmeticsOf({ cardBackCosmetic: CLOCKWORK });
    expect(wornDeckCosmetics(true, decked, account, owns).cardBack).toBe(DEFAULT_CARD_COSMETICS.cardBack);
    expect(wornDeckCosmetics(true, decked, account, (id) => ownsAccountCosmetic(new Set([CLOCKWORK, ASTRAL]), id))).toEqual({
      table: ASTRAL,
      field: DEFAULT_CARD_COSMETICS.field,
      cardBack: CLOCKWORK,
    });
    // Nothing deciding.
    expect(wornDeckCosmetics(true, decked, account)).toEqual({ table: ASTRAL, field: SNOW, cardBack: CLOCKWORK });

    // R1953001: an account-wide foil is worn on no card; a card's own earned foil only where the ledger lists it there.
    const def = CATALOG["core-004"];
    if (def === undefined) throw new Error("no card");
    const foilOf = (owned: OwnedByCard): string | null => {
      const { container } = render(
        <FullArtOwned owned={owned}>
          <CardFace face={faceModel({ defId: "core-004", def, radiant: false })} worn />
        </FullArtOwned>,
      );
      const found = container.querySelector(".cf")?.getAttribute("data-cosmetic-foil") ?? null;
      cleanup();
      return found;
    };
    writeCosmetics({ foil: "foil-void-shimmer" });
    expect(foilOf(new Map([["core-004", new Set(["foil-void-shimmer"])]]))).toBeNull();
    writeCardPick("core-004", "foil", "foil-void-shimmer");
    expect(foilOf(NO_FULL_ART)).toBeNull();
    expect(foilOf(new Map([["core-005", new Set(["foil-void-shimmer"])]]))).toBeNull();
    expect(foilOf(new Map([["core-004", new Set(["foil-void-shimmer"])]]))).toBe("foil-void-shimmer");

    // The tiles show the default equipped, and so does the deck picker, which is what the board draws.
    writeCosmetics({ table: ASTRAL });
    const { unmount } = render(under(NO_ACCOUNT_COSMETICS, <AccountCosmeticTiles />));
    expect(screen.getByTestId(collectionTestid.tile(DEFAULT_CARD_COSMETICS.table))).toHaveAttribute("data-equipped", "true");
    expect(screen.getByTestId(collectionTestid.tile(ASTRAL))).toHaveAttribute("data-equipped", "false");
    unmount();
    render(under(NO_ACCOUNT_COSMETICS, <DeckCosmeticsPicker deck={deck({ tableCosmetic: ASTRAL })} onCosmetic={vi.fn()} />));
    fireEvent.click(screen.getByTestId(DECK_COSMETICS_BUTTON));
    expect(screen.getByTestId<HTMLSelectElement>("deck-cosmetic-table").value).toBe(DEFAULT_CARD_COSMETICS.table);
    cleanup();

    // The board: the default playmat for an unowned pick.
    const board = (owned: AccountOwnership): void => {
      cleanup();
      render(under(owned, <Board view={fullBoardView()} wearCosmetics deckCosmetics={deckCosmeticsOf({ tableCosmetic: ASTRAL })} />));
    };
    board(NO_ACCOUNT_COSMETICS);
    expect(document.querySelector(".field")?.hasAttribute("data-table")).toBe(false);
    board(new Set([ASTRAL]));
    expect(document.querySelector(".field")?.getAttribute("data-table")).toBe(ASTRAL);
  });

  it("R1833000 ViewerAccountCosmetics reads once while gated and owns every one otherwise", async () => {
    const probe: { owns: (id: string) => boolean } = { owns: () => false };
    function Probe(): null {
      probe.owns = useOwnsAccountCosmetic();
      return null;
    }
    const answer: OwnedCosmeticsResponse = { cards: [{ cardId: "core-008", cosmeticId: "full-art-mr-vanilla" }], account: [ASTRAL] };
    const fetchOwned = vi.fn(() => Promise.resolve(answer));
    const read = (token: string): Promise<ReadonlySet<string>> => readOwnedAccountCosmetics(token, fetchOwned);

    // Gated: nothing past the starters until the read answers, then the account's own.
    const { rerender } = render(
      <ViewerAccountCosmetics token="e2e-token-p1" gated read={read}>
        <Probe />
      </ViewerAccountCosmetics>,
    );
    expect(probe.owns(ASTRAL)).toBe(false);
    expect(probe.owns("table-felt")).toBe(true);
    await waitFor(() => {
      expect(probe.owns(ASTRAL)).toBe(true);
    });
    expect(probe.owns(SNOW)).toBe(false);
    // The full-art reader shares the session's one answer: no second request.
    expect(await readOwnedFullArt("e2e-token-p1", fetchOwned)).toEqual(new Map([["core-008", new Set(["full-art-mr-vanilla"])]]));
    expect(fetchOwned).toHaveBeenCalledTimes(1);

    // No token (a pending account's screen) owns only the starters and reads nothing.
    rerender(
      <ViewerAccountCosmetics token={null} gated read={read}>
        <Probe />
      </ViewerAccountCosmetics>,
    );
    expect(probe.owns(ASTRAL)).toBe(false);
    expect(fetchOwned).toHaveBeenCalledTimes(1);

    const unread = vi.fn(read);
    rerender(
      <ViewerAccountCosmetics token="e2e-token-p2" gated={false} read={unread}>
        <Probe />
      </ViewerAccountCosmetics>,
    );
    expect(probe.owns(SNOW)).toBe(true);
    expect(unread).not.toHaveBeenCalled();
  });
});
