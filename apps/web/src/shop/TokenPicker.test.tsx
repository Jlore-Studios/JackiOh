// The token picker (#1309), from fixture responses: the card picker listing only unowned cards,
// both bars and the cycle's next step, the Rare and Epic filters over the shipped registry, and a
// redemption telling the page while a refused one changes nothing, and the page's next revision
// reading the bars again. The tier rules live beside them as pure tests over crafted
// registries, where Legendary, Mythic, chase, starter and owned entries can all appear.

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";

import type { CardDef } from "@jackioh/shared";

import type { TokenBarsResponse } from "../net/api.ts";
import { refusal, stubServer, type Reply } from "../test/stubServer.ts";
import { collectionOf, ownedCosmeticsOf, tokenBars } from "./fixtures.ts";
import {
  fittingCards,
  quantitiesOf,
  redeemableCosmetics,
  shopTestid as T,
  unownedCards,
  type RegistryCosmetic,
} from "./shop.ts";
import TokenPicker from "./TokenPicker.tsx";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

const TOKEN = "token-1";

/** A crafted registry entry. */
function entry(over: Partial<RegistryCosmetic> & { id: string }): RegistryCosmetic {
  return {
    kind: "foil",
    name: over.id,
    rarity: "Common",
    default: false,
    starter: false,
    chase: false,
    ...over,
  };
}

describe("#1309 the picker model", () => {
  it("the card picker lists only unowned, non-token cards of sets that ship", () => {
    const defs: CardDef[] = [
      { ...CATALOG["core-002"]!, token: false },
      { ...CATALOG["core-002"]!, id: "boss-001", set: "Boss", token: false },
      { ...CATALOG["core-002"]!, id: "tok-001", token: true },
    ];
    // core-001 is owned at MAX_COPIES by the fixture; core-002 is not.
    const ids = unownedCards(collectionOf(), defs).map((def) => def.id);
    expect(ids).toEqual(["core-002"]);
  });

  it("a Rare token lists unowned Common and Rare cosmetics, an Epic token adds Epic", () => {
    const registry: RegistryCosmetic[] = [
      entry({ id: "common-foil", rarity: "Common" }),
      entry({ id: "rare-foil", rarity: "Rare" }),
      entry({ id: "epic-foil", rarity: "Epic" }),
      entry({ id: "legendary-foil", rarity: "Legendary" }),
      entry({ id: "mythic-table", kind: "table", rarity: "Mythic" }),
      entry({ id: "chase-foil", rarity: "Rare", chase: true }),
      entry({ id: "starter-foil", rarity: "Common", starter: true }),
      entry({ id: "default-foil", rarity: "Rare", default: true }),
    ];
    const quantities = quantitiesOf(collectionOf());
    const owned = ownedCosmeticsOf({ cards: [], account: [] });
    expect(redeemableCosmetics("Rare", owned, quantities, registry).map((cosmetic) => cosmetic.id)).toEqual([
      "common-foil",
      "rare-foil",
    ]);
    expect(redeemableCosmetics("Epic", owned, quantities, registry).map((cosmetic) => cosmetic.id)).toEqual([
      "common-foil",
      "rare-foil",
      "epic-foil",
    ]);
  });

  it("neither token lists Legendary, Mythic, chase, starter or owned cosmetics", () => {
    const registry: RegistryCosmetic[] = [
      entry({ id: "legendary-foil", rarity: "Legendary" }),
      entry({ id: "mythic-foil", rarity: "Mythic" }),
      entry({ id: "epic-chase", rarity: "Epic", chase: true }),
      entry({ id: "epic-starter", rarity: "Epic", starter: true }),
      entry({ id: "rare-table", kind: "table", rarity: "Rare" }),
      entry({ id: "rare-foil", rarity: "Rare" }),
    ];
    // Only core-001 is owned, and it already wears the foil; the table is on the account.
    const quantities = quantitiesOf(collectionOf({ entries: [{ cardId: "core-001", quantity: 1 }] }));
    const owned = ownedCosmeticsOf({
      cards: [{ cardId: "core-001", cosmeticId: "rare-foil" }],
      account: ["rare-table"],
    });
    for (const tier of ["Rare", "Epic"] as const) {
      expect(redeemableCosmetics(tier, owned, quantities, registry)).toEqual([]);
    }
    const open = ownedCosmeticsOf({ cards: [], account: [] });
    expect(redeemableCosmetics("Rare", open, quantities, registry).map((cosmetic) => cosmetic.id)).toEqual([
      "rare-table",
      "rare-foil",
    ]);
  });

  it("a full art fits only its named card, owned", () => {
    const fullArt = entry({ id: "fa", kind: "fullArt", rarity: "Epic", card: "core-001" });
    expect(fittingCards(fullArt, quantitiesOf(collectionOf()))).toEqual(["core-001"]);
    expect(fittingCards(fullArt, quantitiesOf(collectionOf({ entries: [{ cardId: "core-002", quantity: 1 }] })))).toEqual(
      [],
    );
  });
});

/** The picker's reads, answered from the fixtures unless a test says otherwise. */
function tokenServer(answers: { bars?: Reply; redeem?: Reply } = {}): ReturnType<typeof stubServer> {
  return stubServer((call) => {
    if (call.method === "GET" && call.path === "/api/tokens") {
      return answers.bars ?? { status: 200, body: tokenBars() };
    }
    if (call.method === "GET" && call.path === "/api/collection") return { status: 200, body: collectionOf() };
    if (call.method === "GET" && call.path === "/api/cosmetics") return { status: 200, body: ownedCosmeticsOf() };
    if (call.method === "POST" && call.path === "/api/tokens/redeem-card") {
      return (
        answers.redeem ?? {
          status: 200,
          body: { ...tokenBars(), tokens: { ...tokenBars().tokens, card: 0 } } satisfies TokenBarsResponse,
        }
      );
    }
    return undefined;
  });
}

describe("#1309 the token picker", () => {
  it("the card picker lists only unowned cards", async () => {
    tokenServer();
    render(<TokenPicker token={TOKEN} />);

    expect(await screen.findByTestId(T.cardBar)).toHaveTextContent("149 / 150");
    expect(screen.getByTestId(T.cosmeticBar)).toHaveTextContent("30 / 150");
    expect(screen.getByTestId(T.nextStep)).toHaveTextContent("Epic token");
    expect(screen.getByTestId(T.heldCard)).toHaveTextContent("1");

    // core-002 is unowned; core-001 and core-004 are owned at MAX_COPIES; core-051-1 is a token.
    expect(screen.getByTestId(T.cardPick("core-002"))).toBeInTheDocument();
    expect(screen.queryByTestId(T.cardPick("core-001"))).toBeNull();
    expect(screen.queryByTestId(T.cardPick("core-004"))).toBeNull();
    expect(screen.queryByTestId(T.cardPick("core-051-1"))).toBeNull();
  });

  it("a Rare token lists no Epic, Legendary or Mythic cosmetic", async () => {
    tokenServer();
    render(<TokenPicker token={TOKEN} />);
    await screen.findByTestId(T.tokenPanel);

    // The shipped registry's only unowned, non-starter cosmetics are Epic full arts.
    expect(screen.getByTestId(T.cosmeticNone)).toHaveTextContent("No cosmetics to pick at this tier.");
    expect(screen.queryByTestId(T.cosmeticPick("full-art-big-d-fender"))).toBeNull();
    expect(screen.getByTestId(T.tierRare)).toBeEnabled();
    expect(screen.getByTestId(T.tierEpic), "no Epic token is held").toBeDisabled();
  });

  it("an Epic token adds the Epic full arts, and never Legendary or Mythic", async () => {
    const bars = tokenBars();
    tokenServer({
      bars: {
        status: 200,
        body: { ...bars, tokens: { ...bars.tokens, cosmeticEpic: 1 } } satisfies TokenBarsResponse,
      },
    });
    render(<TokenPicker token={TOKEN} />);
    await screen.findByTestId(T.tokenPanel);

    fireEvent.click(screen.getByTestId(T.tierEpic));
    // core-001 is owned, so its full art shows with a preview; Gary's is owned on core-004.
    expect(screen.getByTestId(T.cosmeticPick("full-art-big-d-fender"))).toBeInTheDocument();
    expect(screen.getByTestId(T.cosmeticPreview("full-art-big-d-fender"))).toHaveTextContent("Big D-fender");
    expect(screen.queryByTestId(T.cosmeticPick("full-art-gary-the-gambler"))).toBeNull();
    // The shipped registry's Legendary and Mythic cosmetics are all starters: never listed.
    expect(screen.queryByText("Cracked ice")).toBeNull();
    expect(screen.queryByText("Gilded edge")).toBeNull();
  });

  it("a refused redemption shows its reason and changes nothing on screen", async () => {
    const server = tokenServer({ redeem: refusal(409, "conflict", "that card is already owned") });
    const onChanged = vi.fn<() => void>();
    render(<TokenPicker token={TOKEN} onChanged={onChanged} />);
    await screen.findByTestId(T.cardPick("core-002"));

    fireEvent.click(screen.getByTestId(T.cardPick("core-002")));

    expect(await screen.findByTestId(T.tokenError)).toHaveTextContent("that card is already owned");
    expect(screen.getByTestId(T.cardPick("core-002"))).toBeInTheDocument();
    expect(screen.getByTestId(T.heldCard)).toHaveTextContent("1");
    expect(server.to("POST", "/api/tokens/redeem-card")).toHaveLength(1);
    expect(server.to("POST", "/api/tokens/redeem-card")[0]?.body).toEqual({ cardId: "core-002" });
    expect(onChanged, "a refusal changed nothing to tell the page").not.toHaveBeenCalled();
  });

  it("a redeemed card token moves the held count from the answer and tells the page", async () => {
    const server = tokenServer();
    const onChanged = vi.fn<() => void>();
    render(<TokenPicker token={TOKEN} onChanged={onChanged} />);
    await screen.findByTestId(T.cardPick("core-002"));

    fireEvent.click(screen.getByTestId(T.cardPick("core-002")));

    await waitFor(() => {
      expect(screen.getByTestId(T.heldCard)).toHaveTextContent("Card tokens held: 0");
    });
    expect(onChanged).toHaveBeenCalledTimes(1);
    expect(server.to("GET", "/api/collection"), "the collection is read back").toHaveLength(2);
  });

  it("the page's next revision reads the bars again", async () => {
    // The bars move at the server after the first read (another panel opened a pack).
    let bars = tokenBars();
    const server = stubServer((call) => {
      if (call.method === "GET" && call.path === "/api/tokens") return { status: 200, body: bars };
      if (call.method === "GET" && call.path === "/api/collection") return { status: 200, body: collectionOf() };
      if (call.method === "GET" && call.path === "/api/cosmetics") return { status: 200, body: ownedCosmeticsOf() };
      return undefined;
    });
    const { rerender } = render(<TokenPicker token={TOKEN} />);
    expect(await screen.findByTestId(T.cardBar)).toHaveTextContent("149 / 150");

    bars = { ...bars, cardBar: { ...bars.cardBar, points: 3 } };
    rerender(<TokenPicker token={TOKEN} revision={1} />);

    await waitFor(() => {
      expect(screen.getByTestId(T.cardBar)).toHaveTextContent("3 / 150");
    });
    expect(server.to("GET", "/api/collection")).toHaveLength(2);
    expect(server.to("GET", "/api/cosmetics")).toHaveLength(2);
  });

  it("#1485 a failed read that works on the page's next revision shows the shop and drops the alert", async () => {
    const answers: { bars?: Reply } = { bars: refusal(503, "unavailable", "the token bars are not reachable") };
    tokenServer(answers);
    const { rerender } = render(<TokenPicker token={TOKEN} />);
    expect(await screen.findByTestId(T.tokenLoadError)).toHaveTextContent("the token bars are not reachable");

    answers.bars = { status: 200, body: tokenBars() };
    rerender(<TokenPicker token={TOKEN} revision={1} />);

    expect(await screen.findByTestId(T.cardBar)).toHaveTextContent("149 / 150");
    expect(screen.queryByTestId(T.tokenLoadError)).toBeNull();
    expect(screen.queryByTestId(T.tokenRetry)).toBeNull();
  });

  it("#1485 Try again reads once more, cannot be pressed twice while the read is out, and drops the alert once it works", async () => {
    const answers: { bars?: Reply } = { bars: refusal(503, "unavailable", "the token bars are not reachable") };
    const server = tokenServer(answers);
    render(<TokenPicker token={TOKEN} />);
    await waitFor(() => {
      expect(screen.getByTestId(T.tokenRetry)).toBeEnabled();
    });
    expect(server.to("GET", "/api/tokens")).toHaveLength(1);

    answers.bars = { status: 200, body: tokenBars() };
    fireEvent.click(screen.getByTestId(T.tokenRetry));
    // The read is out: the button waits for it, and a second press sends nothing.
    expect(screen.getByTestId(T.tokenRetry)).toBeDisabled();
    fireEvent.click(screen.getByTestId(T.tokenRetry));

    expect(await screen.findByTestId(T.cardBar)).toHaveTextContent("149 / 150");
    expect(screen.queryByTestId(T.tokenLoadError)).toBeNull();
    expect(server.to("GET", "/api/tokens")).toHaveLength(2);
  });
});
