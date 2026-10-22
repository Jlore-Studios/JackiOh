// R1313000: the six full-art faces (R1017001) are owned cosmetics, one per hero portrait's card. Once
// ownership decides (the economy live, in a production build), a copy prints its full art only where
// it is the viewer's own and the viewer owns it; every other copy, the opponent's among them, prints
// its normal face. With no ownership mounted (practice, hotseat, the Almanac, the landing page's
// fan), and before the economy is live, every full art prints as before.

import { act, cleanup, fireEvent, render, renderHook, screen, waitFor } from "@testing-library/react";
import type { ReactElement, ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { PlayerView } from "@jackioh/shared";

import Board from "../game/Board.tsx";
import { CatalogContext, lookupFromDefs } from "../game/catalog.ts";
import type { OwnedCosmeticsResponse } from "../net/api.ts";
import { baseView, card, emptySide, resetIds } from "../test/fixtures.ts";
import { COSMETIC_KINDS, COSMETICS } from "../wire/generated/cosmetics.ts";
import { CardFace } from "./CardFace.tsx";
import { resetCosmeticsForTests, useCanEquip, writeCardFullArt, type CosmeticId, type OwnedByCard } from "./cosmetics.ts";
import {
  E2E_FULL_ART_GATED_KEY,
  FULL_ART_GATED,
  FULL_ART_NAMES,
  FULL_ART_OF_CARD,
  FullArtAllowed,
  FullArtOwned,
  type FullArtOwnership,
  NO_FULL_ART,
  OwnCopy,
  fullArtGated,
  ownsFullArt,
} from "./fullArt.ts";
import { HoverPreview } from "./inspect/HoverPreview.tsx";
import { closeInspect } from "./inspect/store.ts";
import { INSPECT_FACE, INSPECT_SHEET } from "./inspect/testids.ts";
import { faceModel } from "./model.ts";
import { ViewerFullArt, ownedCardCosmetics, readOwnedFullArt, resetOwnedFullArtForTests } from "./ownedFullArt.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "./settings.ts";

const ANCHOR = { left: 600, top: 200, width: 100, height: 140 };

/** Longer than any inspect delay (hover or long-press), as in game/Card.test.tsx. */
const PAST_ANY_INSPECT_DELAY_MS = 2_000;

const DEFS = Object.values(CATALOG);
const HEROES = DEFS.filter((def) => FULL_ART_NAMES.has(def.name));
const HERO_IDS = HEROES.map((def) => def.id);
/** The ownership of the full art of each card in `ids`, as `GET /api/cosmetics` reads once mapped. */
function owningFullArt(ids: Iterable<string>): OwnedByCard {
  return new Map([...ids].map((id) => [id, new Set([FULL_ART_OF_CARD.get(id) ?? `no full art for ${id}`])]));
}
const EVERY_HERO = owningFullArt(HERO_IDS);
/** Mr. Vanilla, whose full art the viewer owns below, and Big D-fender, whose they do not. */
const OWNED = "core-008";
const NOT_OWNED = "core-001";

function heroFace(id: string) {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`no ${id}`);
  return faceModel({ defId: id, def, radiant: false });
}

/** Whether the full face of `id` that `wrap` mounts prints full art. */
function printsFullArt(id: string, worn: boolean, wrap: (face: ReactElement) => ReactElement = (face) => face): boolean {
  const { container } = render(wrap(<CardFace face={heroFace(id)} layout="full" worn={worn} />));
  const cf = container.querySelector<HTMLElement>(".cf");
  if (cf === null) throw new Error("no face");
  const full = cf.getAttribute("data-full-art") === "true";
  cleanup();
  return full;
}

afterEach(() => {
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
  localStorage.clear();
  resetCosmeticsForTests();
  cleanup();
  resetOwnedFullArtForTests();
  closeInspect();
  vi.useRealTimers();
});

describe("R1313000 the full-art faces are owned cosmetics", () => {
  it("R1313000 the registry holds one Epic full art for each hero portrait's card, and none is a pick", () => {
    const fullArts = COSMETICS.filter((cosmetic) => cosmetic.kind === "fullArt");
    const cards = fullArts.flatMap((cosmetic) => ("card" in cosmetic ? [cosmetic.card] : []));
    expect([...cards].sort()).toEqual([...HERO_IDS].sort());
    expect(new Set(cards.map((id) => CATALOG[id]?.name))).toEqual(FULL_ART_NAMES);
    for (const cosmetic of fullArts) {
      expect(cosmetic.rarity, cosmetic.id).toBe("Epic");
      expect(cosmetic.default, cosmetic.id).toBe(false);
    }
    expect((COSMETIC_KINDS as readonly string[]).includes("fullArt")).toBe(false);
  });

  it("R1313000 ownership gates full art only once the economy is live, in a production build", () => {
    expect(fullArtGated(false, "production")).toBe(false);
    expect(fullArtGated(true, "production")).toBe(true);
    // A dev or e2e build (`--mode development`) and the tests treat every full art as owned.
    expect(fullArtGated(true, "development")).toBe(false);
    expect(fullArtGated(true, "test")).toBe(false);
    expect(FULL_ART_GATED).toBe(false);
    expect(ownsFullArt("every", NOT_OWNED, false)).toBe(true);
    expect(ownsFullArt(owningFullArt([OWNED]), OWNED, true)).toBe(true);
    expect(ownsFullArt(owningFullArt([OWNED]), OWNED, false)).toBe(false);
    expect(ownsFullArt(owningFullArt([OWNED]), NOT_OWNED, true)).toBe(false);
  });

  it("R1310004 a development build gates full art only when the e2e key asks", () => {
    expect(E2E_FULL_ART_GATED_KEY).toBe("jackioh.e2e.fullArtGated");
    for (const e2eGated of [false, true]) {
      // A production build is gated while the economy is live, whatever the key says.
      expect(fullArtGated(true, "production", e2eGated)).toBe(true);
      // A development build (dev and e2e) only when the e2e run asks.
      expect(fullArtGated(true, "development", e2eGated)).toBe(e2eGated);
      // The tests never are.
      expect(fullArtGated(true, "test", e2eGated)).toBe(false);
      // And no build is while the economy is off.
      for (const mode of ["production", "development", "test"]) {
        expect(fullArtGated(false, mode, e2eGated), mode).toBe(false);
      }
    }
    // This test build reads no key at all.
    expect(FULL_ART_GATED).toBe(false);
  });

  it("R1017001 R1313000 with the economy off every hero's copy prints full art, yours and the opponent's, and nothing is read", () => {
    const read = vi.fn((): Promise<OwnedByCard> => Promise.resolve(NO_FULL_ART));
    for (const id of HERO_IDS) {
      for (const worn of [true, false]) {
        const wrap = (face: ReactElement) => (
          <ViewerFullArt token="e2e-token-p1" gated={false} read={read}>
            {face}
          </ViewerFullArt>
        );
        expect(printsFullArt(id, worn, wrap), `${id} worn=${String(worn)}`).toBe(true);
      }
    }
    expect(read).not.toHaveBeenCalled();
  });

  it("R1313000 with the economy live your own copy prints full art only once you own it", async () => {
    const read = vi.fn((): Promise<OwnedByCard> => Promise.resolve(owningFullArt([OWNED])));
    const { container } = render(
      <ViewerFullArt token="e2e-token-p1" gated read={read}>
        <CardFace face={heroFace(OWNED)} layout="full" worn />
        <CardFace face={heroFace(NOT_OWNED)} layout="full" worn />
      </ViewerFullArt>,
    );
    const [owned, other] = [...container.querySelectorAll<HTMLElement>(".cf")];
    if (owned === undefined || other === undefined) throw new Error("two faces");
    await waitFor(() => expect(owned.getAttribute("data-full-art")).toBe("true"));
    expect(other.hasAttribute("data-full-art")).toBe(false);
    // Its words print, as any card's do.
    expect(other.querySelector(".card-type")).not.toBeNull();
    expect(read).toHaveBeenCalledTimes(1);
    expect(read).toHaveBeenCalledWith("e2e-token-p1");
  });

  it("R1313000 with the economy live nothing counts as owned before the read answers, after it fails, or with no token", async () => {
    let answer: (owned: OwnedByCard) => void = () => {};
    const pending = vi.fn(() => new Promise<OwnedByCard>((resolve) => (answer = resolve)));
    const { container } = render(
      <ViewerFullArt token="e2e-token-p1" gated read={pending}>
        <CardFace face={heroFace(OWNED)} layout="full" worn />
      </ViewerFullArt>,
    );
    const face = container.querySelector<HTMLElement>(".cf");
    expect(face?.hasAttribute("data-full-art")).toBe(false);
    await act(async () => answer(owningFullArt([OWNED])));
    expect(face?.getAttribute("data-full-art")).toBe("true");
    cleanup();

    const failing = vi.fn(() => Promise.reject(new Error("offline")));
    const failed = render(
      <ViewerFullArt token="e2e-token-p1" gated read={failing}>
        <CardFace face={heroFace(OWNED)} layout="full" worn />
      </ViewerFullArt>,
    );
    await act(async () => {});
    expect(failing).toHaveBeenCalledTimes(1);
    expect(failed.container.querySelector(".cf")?.hasAttribute("data-full-art")).toBe(false);
    cleanup();

    // A pending account's screen: no token, so nothing is read and nothing is owned.
    const unread = vi.fn((): Promise<OwnedByCard> => Promise.resolve(EVERY_HERO));
    expect(
      printsFullArt(OWNED, true, (face) => (
        <ViewerFullArt token={null} gated read={unread}>
          {face}
        </ViewerFullArt>
      )),
    ).toBe(false);
    expect(unread).not.toHaveBeenCalled();
  });

  it("R1313000 the opponent's copy prints its normal face, even of a full art the viewer owns", () => {
    for (const id of HERO_IDS) {
      const owned = (face: ReactElement) => <FullArtOwned owned={EVERY_HERO}>{face}</FullArtOwned>;
      expect(printsFullArt(id, false, owned), id).toBe(false);
      expect(printsFullArt(id, true, owned), id).toBe(true);
    }
  });

  it("R1313000 on a match board your own hand's hero prints the full art you own, and so does its inspect sheet", () => {
    vi.useFakeTimers();
    const view = (): PlayerView => {
      resetIds();
      return baseView({
        you: emptySide("p1", { hand: [card({ defId: OWNED }), card({ defId: NOT_OWNED })] }),
        opponent: emptySide("p2", { hand: { count: 3 } }),
      });
    };
    const board = (wearCosmetics: boolean) => (
      <CatalogContext.Provider value={lookupFromDefs(CATALOG)}>
        <FullArtOwned owned={owningFullArt([OWNED])}>
          <Board view={view()} wearCosmetics={wearCosmetics} />
        </FullArtOwned>
      </CatalogContext.Provider>
    );
    const handFace = (id: string): HTMLElement => {
      const face = screen.getByTestId("hand-you").querySelector<HTMLElement>(`.card[data-def-id="${id}"] > .cf`);
      if (face === null) throw new Error(`no ${id} in hand`);
      return face;
    };

    render(board(true));
    expect(handFace(OWNED).getAttribute("data-full-art")).toBe("true");
    expect(handFace(NOT_OWNED).hasAttribute("data-full-art")).toBe(false);

    const root = handFace(OWNED).closest<HTMLElement>(".card");
    if (root === null) throw new Error("no card root");
    fireEvent.pointerDown(root, { pointerType: "touch", clientX: 10, clientY: 10 });
    act(() => {
      vi.advanceTimersByTime(PAST_ANY_INSPECT_DELAY_MS);
    });
    expect(screen.getByTestId(INSPECT_SHEET).querySelector(".cf")?.getAttribute("data-full-art")).toBe("true");
    cleanup();
    closeInspect();

    // A board that wears nothing (a replay, a watched game) has no own copy: its hero prints its words.
    render(board(false));
    expect(handFace(OWNED).hasAttribute("data-full-art")).toBe(false);
  });

  it("R1313000 your own card's hover preview follows your ownership; another's prints its words", () => {
    const preview = (own: boolean): boolean => {
      render(
        <FullArtOwned owned={owningFullArt([OWNED])}>
          <OwnCopy own={own}>
            <HoverPreview face={heroFace(OWNED)} anchor={ANCHOR} />
          </OwnCopy>
        </FullArtOwned>,
      );
      const full = screen.getByTestId(INSPECT_FACE).querySelector(".cf")?.getAttribute("data-full-art") === "true";
      cleanup();
      return full;
    };
    expect(preview(true)).toBe(true);
    expect(preview(false)).toBe(false);
    // An inner OwnCopy never takes back an outer one's word.
    expect(
      printsFullArt(OWNED, false, (face) => (
        <FullArtOwned owned={owningFullArt([OWNED])}>
          <OwnCopy own>
            <OwnCopy own={false}>{face}</OwnCopy>
          </OwnCopy>
        </FullArtOwned>
      )),
    ).toBe(true);
  });

  it("R1313000 the Full-art cards setting off turns every full art off, owned or not", () => {
    writeCardSettings({ fullArt: false });
    for (const id of HERO_IDS) {
      expect(printsFullArt(id, true), id).toBe(false);
      expect(printsFullArt(id, true, (face) => <FullArtOwned owned={EVERY_HERO}>{face}</FullArtOwned>), id).toBe(false);
    }
  });

  it("R1313000 practice, hotseat, the landing page's fan and the Almanac mount no ownership, so every copy prints full art", () => {
    for (const id of HERO_IDS) {
      expect(printsFullArt(id, false), id).toBe(true);
      expect(printsFullArt(id, true), id).toBe(true);
    }
  });

  it("R1313000 R1655000 ownedCardCosmetics keeps each card's own per-card cosmetics and nothing else", () => {
    const owned: OwnedCosmeticsResponse = {
      cards: [
        { cardId: "core-008", cosmeticId: "full-art-mr-vanilla" },
        // Gary's full art on another card is not Gary's, nor that card's.
        { cardId: "core-002", cosmeticId: "full-art-gary-the-gambler" },
        // A foil is a per-card cosmetic too, owned on whichever card the row names.
        { cardId: "core-001", cosmeticId: "foil-rainbow" },
        { cardId: "core-008", cosmeticId: "border-bone" },
        // An account cosmetic and an id the registry lacks are no card's.
        { cardId: "core-004", cosmeticId: "table-felt" },
        { cardId: "core-004", cosmeticId: "foil-aurora" },
      ],
      account: ["full-art-tempo-timmy", "table-felt"],
    };
    expect(ownedCardCosmetics(owned)).toEqual(
      new Map([
        ["core-008", new Set(["full-art-mr-vanilla", "border-bone"])],
        ["core-001", new Set(["foil-rainbow"])],
      ]),
    );
    expect(ownedCardCosmetics({ cards: [], account: [] }).size).toBe(0);
  });

  it("R1313000 readOwnedFullArt reads GET /api/cosmetics once per session, again for another, and again after a failure", async () => {
    const answer: OwnedCosmeticsResponse = { cards: [{ cardId: "core-008", cosmeticId: "full-art-mr-vanilla" }], account: [] };
    const fetchOwned = vi.fn(() => Promise.resolve(answer));
    expect(await readOwnedFullArt("token-a", fetchOwned)).toEqual(owningFullArt(["core-008"]));
    await readOwnedFullArt("token-a", fetchOwned);
    expect(fetchOwned).toHaveBeenCalledTimes(1);

    // A renewed token of the same provider session reads nothing new; another session reads again.
    const renewed = [sessionToken("s-1", "first"), sessionToken("s-1", "renewed")] as const;
    await readOwnedFullArt(renewed[0], fetchOwned);
    await readOwnedFullArt(renewed[1], fetchOwned);
    expect(fetchOwned).toHaveBeenCalledTimes(2);
    await readOwnedFullArt(sessionToken("s-2", "other"), fetchOwned);
    expect(fetchOwned).toHaveBeenCalledTimes(3);

    // A failed read is not kept: the next asks again.
    const failing = vi.fn(() => Promise.reject(new Error("offline")));
    await expect(readOwnedFullArt("token-b", failing)).rejects.toThrow("offline");
    await expect(readOwnedFullArt("token-b", failing)).rejects.toThrow("offline");
    expect(failing).toHaveBeenCalledTimes(2);
  });
});

function base64url(text: string): string {
  return btoa(text).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

/** A token of one provider session: `session_id` is what tells a renewal from a new sign-in. */
function sessionToken(sessionId: string, tag: string): string {
  return [base64url(JSON.stringify({ alg: "HS256" })), base64url(JSON.stringify({ sub: "user-1", session_id: sessionId, tag })), "sig"].join(".");
}

describe("R1655000 a full art is a pick of its card, worn behind ownership", () => {
  const OWNED_ART = FULL_ART_OF_CARD.get(OWNED) as CosmeticId;
  const OTHER_ART = FULL_ART_OF_CARD.get(NOT_OWNED) as CosmeticId;

  /** What `useCanEquip` says under `owned`, or under no provider at all. */
  function canEquipUnder(owned: FullArtOwnership | null): (defId: string, id: CosmeticId) => boolean {
    const wrapper = owned === null ? undefined : ({ children }: { children: ReactNode }) => <FullArtOwned owned={owned}>{children}</FullArtOwned>;
    return renderHook(() => useCanEquip(), { wrapper }).result.current;
  }

  it("R1655000 useCanEquip says no to a full art the viewer does not own, and yes to what every account owns", () => {
    expect(OWNED_ART).toBeDefined();
    const none = canEquipUnder(NO_FULL_ART);
    expect(none(OWNED, OWNED_ART)).toBe(false);
    // A default and a starter are owned on every card, with no row.
    expect(none(OWNED, "foil-none")).toBe(true);
    expect(none(OWNED, "border-standard")).toBe(true);
    expect(none(OWNED, "foil-rainbow")).toBe(true);
    expect(none(NOT_OWNED, "border-wood")).toBe(true);

    // A ledger row makes it the viewer's on that card, and on no other: a full art is for its own card.
    const owning = canEquipUnder(owningFullArt([OWNED]));
    expect(owning(OWNED, OWNED_ART)).toBe(true);
    expect(owning(NOT_OWNED, OTHER_ART)).toBe(false);
    expect(owning(NOT_OWNED, OWNED_ART)).toBe(false);
    expect(owning(OWNED, OTHER_ART)).toBe(false);

    // A ledger row for a card's foil counts on that card alone; unknown ids never do.
    const foiled = canEquipUnder(new Map([[OWNED, new Set(["foil-ledger"])]]));
    expect(foiled(OWNED, "foil-ledger" as CosmeticId)).toBe(false);

    // Practice, hotseat and R1310004's builds mount no ownership: every cosmetic counts as owned.
    const every = canEquipUnder(null);
    expect(every(OWNED, OWNED_ART)).toBe(true);
    expect(every(NOT_OWNED, OTHER_ART)).toBe(true);
    expect(canEquipUnder("every")(OWNED, OWNED_ART)).toBe(true);
  });

  it("R1655000 an own copy's full-art pick is worn only where its full art is owned, or every cosmetic counts as owned", () => {
    writeCardSettings({ fullArt: false });
    writeCardFullArt(OWNED, true);
    writeCardFullArt(NOT_OWNED, true);
    const under = (owned: FullArtOwnership | null) => (face: ReactElement) =>
      owned === null ? face : <FullArtOwned owned={owned}>{face}</FullArtOwned>;

    // Practice and hotseat (no ownership), and the builds that treat every cosmetic as owned: worn.
    expect(printsFullArt(OWNED, true, under(null))).toBe(true);
    expect(printsFullArt(NOT_OWNED, true, under("every"))).toBe(true);
    // Where ownership decides, only the full art the viewer owns, on its own copy.
    expect(printsFullArt(OWNED, true, under(owningFullArt([OWNED])))).toBe(true);
    expect(printsFullArt(NOT_OWNED, true, under(owningFullArt([OWNED])))).toBe(false);
    expect(printsFullArt(OWNED, true, under(NO_FULL_ART))).toBe(false);
    // Another's copy of the owned card is not the viewer's own: it draws the setting (off).
    expect(printsFullArt(OWNED, false, under(owningFullArt([OWNED])))).toBe(false);
    // A tutorial lesson prints none, a pick or not.
    expect(printsFullArt(OWNED, true, (face) => <FullArtAllowed allowed={false}>{face}</FullArtAllowed>)).toBe(false);
  });

  it("R1655000 a card's pick beats the Full-art cards setting on its own copy and leaves every other copy to it", () => {
    // Setting on, the card's pick off: the viewer's copy hides it; any other copy follows the setting.
    writeCardFullArt(OWNED, false);
    expect(printsFullArt(OWNED, true)).toBe(false);
    expect(printsFullArt(OWNED, false)).toBe(true);
    // Another card with no pick follows the setting.
    expect(printsFullArt(NOT_OWNED, true)).toBe(true);
    // Setting off, the pick on: the viewer's copy prints it; the copy that is not theirs draws the setting.
    writeCardSettings({ fullArt: false });
    writeCardFullArt(OWNED, true);
    expect(printsFullArt(OWNED, true)).toBe(true);
    expect(printsFullArt(OWNED, false)).toBe(false);
    expect(printsFullArt(NOT_OWNED, true)).toBe(false);
    // Clearing the pick returns the card to the setting.
    writeCardFullArt(OWNED, null);
    expect(printsFullArt(OWNED, true)).toBe(false);
  });

  it("R1655000 with the economy live the pick of a full art prints once the read answers that it is owned", async () => {
    writeCardSettings({ fullArt: false });
    writeCardFullArt(OWNED, true);
    writeCardFullArt(NOT_OWNED, true);
    const read = vi.fn((): Promise<OwnedByCard> => Promise.resolve(owningFullArt([OWNED])));
    const { container } = render(
      <ViewerFullArt token="e2e-token-p1" gated read={read}>
        <CardFace face={heroFace(OWNED)} layout="full" worn />
        <CardFace face={heroFace(NOT_OWNED)} layout="full" worn />
      </ViewerFullArt>,
    );
    const [owned, other] = [...container.querySelectorAll<HTMLElement>(".cf")];
    if (owned === undefined || other === undefined) throw new Error("two faces");
    expect(owned.hasAttribute("data-full-art")).toBe(false);
    await waitFor(() => expect(owned.getAttribute("data-full-art")).toBe("true"));
    expect(other.hasAttribute("data-full-art")).toBe(false);
  });
});
