// R1655000: a card's own pick of a foil, finish or border is worn only where the viewer owns it on
// that card (cosmetics.ts `useCosmeticAttributes`), else the kind's default (R1953001). Every shipped
// one is a starter, which every account owns, so the registry here gains one border that is not.

import { CATALOG } from "@jackioh/cards";
import { cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { faceModel } from "./model.ts";
import { CardFace } from "./CardFace.tsx";
import { resetCosmeticsForTests, writeCardPick, writeCosmetics, type CosmeticId } from "./cosmetics.ts";
import { FullArtOwned } from "./fullArt.ts";

vi.mock("../wire/generated/cosmetics.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../wire/generated/cosmetics.ts")>();
  const future = {
    id: "border-future",
    kind: "border",
    name: "Future trim",
    rarity: "Epic",
    default: false,
    starter: false,
    chase: false,
  } as const;
  return { ...actual, COSMETICS: [...actual.COSMETICS, future] };
});

const CARD = "core-004";
const ACCOUNT_BORDER = "border-bone";
/** The mocked entry: an id the generated registry's type does not name. */
const OWN_BORDER = "border-future" as CosmeticId;

function frame(owned: Parameters<typeof FullArtOwned>[0]["owned"]): string | null {
  const { container } = render(
    <FullArtOwned owned={owned}>
      <CardFace face={faceModel({ defId: CARD, def: CATALOG[CARD], radiant: false })} worn />
    </FullArtOwned>,
  );
  const found = container.querySelector<HTMLElement>(".cf")?.getAttribute("data-frame") ?? null;
  cleanup();
  return found;
}

beforeEach(() => {
  localStorage.clear();
  resetCosmeticsForTests();
});

afterEach(() => {
  cleanup();
  resetCosmeticsForTests();
  localStorage.clear();
});

describe("R1655000 a card's own border is worn behind ownership", () => {
  it("R1655000 a border the ledger does not list on the card is not worn: the kind's default shows, until the ledger lists it", () => {
    // R1953001: the account's border is not worn in its place.
    writeCosmetics({ border: ACCOUNT_BORDER });
    writeCardPick(CARD, "border", OWN_BORDER);

    expect(frame(new Map()), "nothing owned past the starters").toBeNull();
    expect(frame(new Map([["core-005", new Set([OWN_BORDER])]])), "owned on another card").toBeNull();
    expect(frame(new Map([[CARD, new Set([OWN_BORDER])]]))).toBe(OWN_BORDER);
    // Practice, hotseat and the e2e builds treat every cosmetic as owned (R1310004).
    expect(frame("every")).toBe(OWN_BORDER);
  });
});
