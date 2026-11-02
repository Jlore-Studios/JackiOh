// R1953001 (#1953): a held cosmetic token is offered in the locked cosmetic's fullscreen preview
// (R2730000), and only where its tier covers the cosmetic (R1305003): the offer redeems it through
// `POST /api/tokens/redeem-cosmetic` and the cosmetic shows owned once the screen's ownership reads
// again; a refusal shows the server's words and changes nothing. R2730000 (#2730): a held card
// token is offered in an unowned card's preview through `POST /api/tokens/redeem-card`, and the
// collection is read again, so the card reads owned in the preview and the grid.
// R2480000 (#2480): the offer asks first; Confirm redeems, Cancel and Escape spend nothing.

import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { resetCosmeticsForTests, type OwnedByCard } from "../cards/cosmetics.ts";
import { ViewerAccountCosmetics, ViewerFullArt, resetOwnedFullArtForTests } from "../cards/ownedFullArt.ts";
import { resetLocaleForTests, setLocale } from "../i18n/i18n.ts";
import type { CollectionResponse, TokenBarsResponse } from "../net/api.ts";
import { collectionOf, tokenBars } from "../shop/fixtures.ts";
import { settle } from "../test/settle.ts";
import { refusal, stubServer, type Reply } from "../test/stubServer.ts";
import { COSMETICS } from "../wire/generated/cosmetics.ts";
import AccountCosmeticTiles from "./AccountCosmeticTiles.tsx";
import { collectionTestid as T } from "./collection.ts";
import { collectionOwning } from "./fixtures.ts";
import OwnedCards from "./OwnedCards.tsx";
import { offerTier, type HeldTokens } from "./TokenOffer.tsx";

const TOKEN = "token-1";

beforeEach(() => {
  localStorage.clear();
  resetCosmeticsForTests();
  resetOwnedFullArtForTests();
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  localStorage.clear();
  resetCosmeticsForTests();
  resetOwnedFullArtForTests();
  resetLocaleForTests();
});

/** Held tokens, none but those given. */
function held(over: Partial<HeldTokens> = {}): HeldTokens {
  return { card: 0, cosmeticRare: 0, cosmeticEpic: 0, cosmeticPack: 0, ...over };
}

/** The bars answer, holding `tokens`. */
function barsHolding(tokens: Partial<HeldTokens>): TokenBarsResponse {
  return { ...tokenBars(), tokens: held(tokens) };
}

/**
 * `GET /api/tokens` holding `tokens`, both redemptions answering `redeem`, and `GET
 * /api/collection` answering `collection()` where given.
 */
function tokenServer(
  tokens: Partial<HeldTokens>,
  redeem: Reply,
  collection?: () => CollectionResponse,
): ReturnType<typeof stubServer> {
  return stubServer((call) => {
    if (call.method === "GET" && call.path === "/api/tokens") return { status: 200, body: barsHolding(tokens) };
    if (call.method === "POST" && call.path === "/api/tokens/redeem-cosmetic") return redeem;
    if (call.method === "POST" && call.path === "/api/tokens/redeem-card") return redeem;
    if (collection !== undefined && call.method === "GET" && call.path === "/api/collection") {
      return { status: 200, body: collection() };
    }
    return undefined;
  });
}

function entry(id: string): (typeof COSMETICS)[number] {
  const found = COSMETICS.find((cosmetic) => cosmetic.id === id);
  if (found === undefined) throw new Error(`no cosmetic ${id}`);
  return found;
}

/** The card collection fixture owns at MAX_COPIES, alone in its set's grid. */
const CARD = CATALOG["core-001"]!;

/** The Cards section's card view of `CARD`, under the screen's ownership. */
function cardViewOf(read: (token: string) => Promise<OwnedByCard>): void {
  render(
    <ViewerFullArt token={TOKEN} gated read={read}>
      <OwnedCards collection={collectionOf()} defs={[CARD]} token={TOKEN} />
    </ViewerFullArt>,
  );
  fireEvent.click(screen.getByTestId(T.setToggle("Core")));
  fireEvent.click(screen.getByTestId(T.card("core-001")));
}

/** The preview of the view's strip `itemId`, opened with the inspect key. */
function previewOf(itemId: string): HTMLElement {
  fireEvent.keyDown(screen.getByTestId(T.stripItem(itemId)), { key: "i" });
  return screen.getByTestId(T.preview);
}

/** A tile's button, which the inspect key reaches. */
function tileButton(id: string): HTMLElement {
  return within(screen.getByTestId(T.tile(id))).getByRole("button");
}

/** The preview of the tile `id`, opened with the inspect key on its button. */
function tilePreviewOf(id: string): HTMLElement {
  fireEvent.keyDown(tileButton(id), { key: "i" });
  return screen.getByTestId(T.preview);
}

function closePreview(): void {
  fireEvent.keyDown(document.body, { key: "Escape" });
  expect(screen.queryByTestId(T.preview)).toBeNull();
}

describe("R1953001 a held cosmetic token, offered in the preview", () => {
  it("R1953001 a locked Epic cosmetic offers a held Epic token, redeems it and shows it owned", async () => {
    const server = tokenServer({ cosmeticEpic: 2 }, { status: 200, body: barsHolding({ cosmeticEpic: 1 }) });
    const read = vi
      .fn<(token: string) => Promise<OwnedByCard>>()
      .mockResolvedValueOnce(new Map())
      .mockResolvedValue(new Map([["core-001", new Set(["foil-galaxy"])]]));
    cardViewOf(read);

    const preview = previewOf("foil-galaxy");
    const offer = await within(preview).findByTestId(T.tokenOffer("foil-galaxy"));
    expect(offer).toHaveTextContent("Unlock with an Epic token (2 held)");
    expect(screen.getByTestId(T.stripItem("foil-galaxy"))).toHaveAttribute("data-owned", "false");

    fireEvent.click(offer);
    fireEvent.click(screen.getByTestId(T.tokenOfferConfirm("foil-galaxy")));

    await waitFor(() => {
      expect(screen.getByTestId(T.stripItem("foil-galaxy"))).toHaveAttribute("data-owned", "true");
    });
    expect(server.to("POST", "/api/tokens/redeem-cosmetic")).toHaveLength(1);
    expect(server.to("POST", "/api/tokens/redeem-cosmetic")[0]?.body).toEqual({ token: "Epic", cosmeticId: "foil-galaxy", cardId: "core-001" });
    expect(read, "the screen's ownership is read again").toHaveBeenCalledTimes(2);
    expect(within(preview).queryByTestId(T.tokenOffer("foil-galaxy"))).toBeNull();
    closePreview();
    // The answer's held count is the one the other offers now say.
    expect(await within(previewOf("border-runes")).findByTestId(T.tokenOffer("border-runes"))).toHaveTextContent(
      "Unlock with an Epic token (1 held)",
    );
  });

  it("R1953001 a Rare token covers Common and Rare, not Epic", async () => {
    const rare = held({ cosmeticRare: 1 });
    expect(offerTier(entry("foil-glitter"), rare)).toBe("Rare");
    expect(offerTier(entry("foil-sunburst"), rare)).toBe("Rare");
    expect(offerTier(entry("foil-galaxy"), rare)).toBeNull();
    // An Epic token covers Epic and below; where both are held, the cheaper one that covers it is offered.
    const epic = held({ cosmeticEpic: 1 });
    expect(offerTier(entry("foil-glitter"), epic)).toBe("Epic");
    expect(offerTier(entry("foil-galaxy"), epic)).toBe("Epic");
    const both = held({ cosmeticRare: 1, cosmeticEpic: 1 });
    expect(offerTier(entry("foil-sunburst"), both)).toBe("Rare");
    expect(offerTier(entry("foil-galaxy"), both)).toBe("Epic");
    // Nothing held, or unread, offers nothing; nor does a default or a starter, which every account owns.
    expect(offerTier(entry("foil-sunburst"), held())).toBeNull();
    expect(offerTier(entry("foil-sunburst"), null)).toBeNull();
    expect(offerTier(entry("foil-none"), both)).toBeNull();
    expect(offerTier(entry("foil-holo-sweep"), both)).toBeNull();

    tokenServer({ cosmeticRare: 3 }, refusal(500, "internal", "unused"));
    cardViewOf(() => Promise.resolve(new Map()));
    expect(await within(previewOf("foil-sunburst")).findByTestId(T.tokenOffer("foil-sunburst"))).toHaveTextContent(
      "Unlock with a Rare token (3 held)",
    );
    expect(await within(previewOf("foil-glitter")).findByTestId(T.tokenOffer("foil-glitter"))).toHaveTextContent(
      "Unlock with a Rare token (3 held)",
    );
    closePreview();
    expect(screen.getByTestId(T.stripItem("foil-galaxy"))).toHaveAttribute("data-owned", "false");
    expect(within(previewOf("foil-galaxy")).queryByTestId(T.tokenOffer("foil-galaxy"))).toBeNull();
  });

  it("R1953001 a chase never shows the offer", async () => {
    const plenty = held({ cosmeticRare: 9, cosmeticEpic: 9 });
    const chases = COSMETICS.filter((cosmetic) => cosmetic.chase);
    expect(chases.length).toBeGreaterThan(0);
    for (const chase of chases) expect(offerTier(chase, plenty), chase.id).toBeNull();

    tokenServer({ cosmeticRare: 9, cosmeticEpic: 9 }, refusal(500, "internal", "unused"));
    cardViewOf(() => Promise.resolve(new Map()));
    expect(await within(previewOf("foil-galaxy")).findByTestId(T.tokenOffer("foil-galaxy"))).toBeInTheDocument();
    closePreview();
    for (const id of ["foil-molten-gold", "foil-void-shimmer", "finish-obsidian", "finish-starlit", "border-filigree", "border-celestial"]) {
      expect(screen.getByTestId(T.stripItem(id)), id).toHaveAttribute("data-owned", "false");
      expect(within(previewOf(id)).queryByTestId(T.tokenOffer(id)), id).toBeNull();
      closePreview();
    }
  });

  it("R2655000 a locked Epic full art offers an Epic token, and its animated and signed full arts none", async () => {
    const plenty = held({ cosmeticRare: 9, cosmeticEpic: 9 });
    // Every card's Epic full art is one an Epic token buys; no token buys a Legendary or Mythic one.
    const fullArts = COSMETICS.filter((cosmetic) => cosmetic.kind === "fullArt");
    expect(fullArts).toHaveLength(21 * 3);
    for (const fullArt of fullArts) expect(offerTier(fullArt, plenty), fullArt.id).toBe(fullArt.rarity === "Epic" ? "Epic" : null);
    expect(offerTier(entry("full-art-league-of-losers"), held({ cosmeticRare: 9 }))).toBeNull();

    tokenServer({ cosmeticEpic: 2 }, refusal(500, "internal", "unused"));
    cardViewOf(() => Promise.resolve(new Map()));
    expect(await within(previewOf("full-art-big-d-fender")).findByTestId(T.tokenOffer("full-art-big-d-fender"))).toHaveTextContent(
      "Unlock with an Epic token (2 held)",
    );
    closePreview();
    for (const id of ["full-art-big-d-fender-animated", "full-art-big-d-fender-signed"]) {
      expect(screen.getByTestId(T.stripItem(id)), id).toHaveAttribute("data-owned", "false");
      expect(within(previewOf(id)).queryByTestId(T.tokenOffer(id)), id).toBeNull();
      closePreview();
    }
  });

  it("R1953001 a refusal shows the server's words and changes nothing", async () => {
    const server = tokenServer({ cosmeticEpic: 1 }, refusal(409, "conflict", "that cosmetic is already owned there"));
    const read = vi.fn<(token: string) => Promise<OwnedByCard>>().mockResolvedValue(new Map());
    cardViewOf(read);

    const preview = previewOf("foil-galaxy");
    fireEvent.click(await within(preview).findByTestId(T.tokenOffer("foil-galaxy")));
    fireEvent.click(screen.getByTestId(T.tokenOfferConfirm("foil-galaxy")));

    expect(await within(preview).findByTestId(T.tokenOfferError("foil-galaxy"))).toHaveTextContent("that cosmetic is already owned there");
    expect(within(preview).getByTestId(T.tokenOffer("foil-galaxy"))).toHaveTextContent("Unlock with an Epic token (1 held)");
    expect(screen.getByTestId(T.stripItem("foil-galaxy"))).toHaveAttribute("data-owned", "false");
    expect(server.to("POST", "/api/tokens/redeem-cosmetic")).toHaveLength(1);
    expect(read, "nothing was granted, so ownership is not read again").toHaveBeenCalledTimes(1);
  });

  it("#2053 a refusal says why in the reader's language", async () => {
    await setLocale("zh-CN");
    tokenServer({ cosmeticEpic: 1 }, { status: 500 });
    cardViewOf(vi.fn<(token: string) => Promise<OwnedByCard>>().mockResolvedValue(new Map()));

    const preview = previewOf("foil-galaxy");
    fireEvent.click(await within(preview).findByTestId(T.tokenOffer("foil-galaxy")));
    fireEvent.click(screen.getByTestId(T.tokenOfferConfirm("foil-galaxy")));

    expect(await within(preview).findByTestId(T.tokenOfferError("foil-galaxy"))).toHaveTextContent("服务器返回了 500");
  });

  it("R1953001 a locked playmat tile offers an Epic token in its preview", async () => {
    const server = tokenServer({ cosmeticEpic: 1 }, { status: 200, body: barsHolding({}) });
    const read = vi
      .fn<(token: string) => Promise<ReadonlySet<string>>>()
      .mockResolvedValueOnce(new Set())
      .mockResolvedValue(new Set(["table-marble"]));
    render(
      <ViewerAccountCosmetics token={TOKEN} gated read={read}>
        <AccountCosmeticTiles token={TOKEN} />
      </ViewerAccountCosmetics>,
    );

    const preview = tilePreviewOf("table-marble");
    const offer = await within(preview).findByTestId(T.tokenOffer("table-marble"));
    expect(offer).toHaveTextContent("Unlock with an Epic token (1 held)");
    // A chase tile is locked with no offer, and a starter tile is owned.
    expect(screen.getByTestId(T.tile("table-astral"))).toHaveAttribute("data-owned", "false");
    closePreview();
    tilePreviewOf("table-astral");
    await settle();
    expect(within(screen.getByTestId(T.preview)).queryByTestId(T.tokenOffer("table-astral"))).toBeNull();
    closePreview();
    expect(screen.queryByTestId(T.tokenOffer("table-felt"))).toBeNull();

    tilePreviewOf("table-marble");
    fireEvent.click(await within(screen.getByTestId(T.preview)).findByTestId(T.tokenOffer("table-marble")));
    fireEvent.click(screen.getByTestId(T.tokenOfferConfirm("table-marble")));

    await waitFor(() => {
      expect(screen.getByTestId(T.tile("table-marble"))).toHaveAttribute("data-owned", "true");
    });
    // An account cosmetic names no card.
    expect(server.to("POST", "/api/tokens/redeem-cosmetic")[0]?.body).toEqual({ token: "Epic", cosmeticId: "table-marble" });
    // The answer holds no token now, so no locked tile offers one.
    fireEvent.keyDown(document.body, { key: "Escape" });
    expect(document.querySelector('[data-testid^="collection-token-offer-"]')).toBeNull();
  });

  it("R1953001 nothing is offered without the session's token, and the held tokens are not read", async () => {
    const server = tokenServer({ cosmeticEpic: 1 }, refusal(500, "internal", "unused"));
    render(
      <ViewerAccountCosmetics token={TOKEN} gated read={() => Promise.resolve(new Set())}>
        <AccountCosmeticTiles />
      </ViewerAccountCosmetics>,
    );
    await settle();

    expect(screen.getByTestId(T.tile("table-marble"))).toHaveAttribute("data-owned", "false");
    tilePreviewOf("table-marble");
    expect(within(screen.getByTestId(T.preview)).queryByTestId(T.tokenOffer("table-marble"))).toBeNull();
    expect(server.to("GET", "/api/tokens")).toHaveLength(0);
  });
});

describe("R2480000 spending a cosmetic token asks first", () => {
  it("R2480000 the first click sends nothing and asks in the card view, naming the cosmetic and the token, with Cancel focused", async () => {
    const server = tokenServer({ cosmeticEpic: 2 }, refusal(500, "internal", "unused"));
    cardViewOf(() => Promise.resolve(new Map()));

    const preview = previewOf("foil-galaxy");
    fireEvent.click(await within(preview).findByTestId(T.tokenOffer("foil-galaxy")));

    const dialog = screen.getByTestId(T.tokenOfferDialog("foil-galaxy"));
    expect(screen.getByRole("alertdialog", { name: "Unlock Galaxy? This spends 1 Epic token." })).toBe(dialog);
    expect(preview).toContainElement(dialog);
    expect(screen.getByTestId(T.cardView)).toBeInTheDocument();
    expect(within(preview).queryByTestId(T.tokenOffer("foil-galaxy"))).toBeNull();
    expect(screen.getByTestId(T.tokenOfferCancel("foil-galaxy"))).toHaveFocus();
    expect(server.to("POST", "/api/tokens/redeem-cosmetic")).toHaveLength(0);
  });

  it("R2480000 a Rare token's question names the Rare token", async () => {
    tokenServer({ cosmeticRare: 1 }, refusal(500, "internal", "unused"));
    cardViewOf(() => Promise.resolve(new Map()));

    const preview = previewOf("foil-sunburst");
    fireEvent.click(await within(preview).findByTestId(T.tokenOffer("foil-sunburst")));

    expect(screen.getByRole("alertdialog", { name: "Unlock Sunburst? This spends 1 Rare token." })).toBeInTheDocument();
  });

  it("R2480000 Confirm sends exactly one redemption, of the token the question named", async () => {
    const server = tokenServer({ cosmeticEpic: 1 }, { status: 200, body: barsHolding({}) });
    const read = vi
      .fn<(token: string) => Promise<OwnedByCard>>()
      .mockResolvedValueOnce(new Map())
      .mockResolvedValue(new Map([["core-001", new Set(["foil-sunburst"])]]));
    cardViewOf(read);

    const preview = previewOf("foil-sunburst");
    fireEvent.click(await within(preview).findByTestId(T.tokenOffer("foil-sunburst")));
    expect(screen.getByTestId(T.tokenOfferDialog("foil-sunburst"))).toHaveTextContent("Unlock Sunburst? This spends 1 Epic token.");

    fireEvent.click(screen.getByTestId(T.tokenOfferConfirm("foil-sunburst")));

    await waitFor(() => {
      expect(screen.getByTestId(T.stripItem("foil-sunburst"))).toHaveAttribute("data-owned", "true");
    });
    expect(server.to("POST", "/api/tokens/redeem-cosmetic")).toHaveLength(1);
    expect(server.to("POST", "/api/tokens/redeem-cosmetic")[0]?.body).toEqual({ token: "Epic", cosmeticId: "foil-sunburst", cardId: "core-001" });
    expect(screen.queryByTestId(T.tokenOfferDialog("foil-sunburst"))).toBeNull();
  });

  it("R2480000 Cancel and Escape send nothing and put the offer back, and Escape leaves the card view open", async () => {
    const server = tokenServer({ cosmeticEpic: 1 }, refusal(500, "internal", "unused"));
    const read = vi.fn<(token: string) => Promise<OwnedByCard>>().mockResolvedValue(new Map());
    cardViewOf(read);

    const preview = previewOf("foil-galaxy");
    fireEvent.click(await within(preview).findByTestId(T.tokenOffer("foil-galaxy")));
    fireEvent.click(screen.getByTestId(T.tokenOfferCancel("foil-galaxy")));

    expect(screen.queryByTestId(T.tokenOfferDialog("foil-galaxy"))).toBeNull();
    const offer = within(preview).getByTestId(T.tokenOffer("foil-galaxy"));
    expect(offer).toHaveTextContent("Unlock with an Epic token (1 held)");
    expect(offer).toHaveFocus();

    fireEvent.click(offer);
    fireEvent.keyDown(screen.getByTestId(T.tokenOfferCancel("foil-galaxy")), { key: "Escape" });

    expect(screen.queryByTestId(T.tokenOfferDialog("foil-galaxy"))).toBeNull();
    expect(screen.getByTestId(T.cardView)).toBeInTheDocument();
    expect(screen.getByTestId(T.preview)).toBeInTheDocument();
    expect(within(preview).getByTestId(T.tokenOffer("foil-galaxy"))).toHaveFocus();

    expect(server.to("POST", "/api/tokens/redeem-cosmetic")).toHaveLength(0);
    expect(screen.getByTestId(T.stripItem("foil-galaxy"))).toHaveAttribute("data-owned", "false");
    expect(read, "nothing was granted, so ownership is not read again").toHaveBeenCalledTimes(1);
  });

  it("R2480000 a locked tile asks in the tile's preview", async () => {
    const server = tokenServer({ cosmeticEpic: 1 }, refusal(500, "internal", "unused"));
    render(
      <ViewerAccountCosmetics token={TOKEN} gated read={() => Promise.resolve(new Set())}>
        <AccountCosmeticTiles token={TOKEN} />
      </ViewerAccountCosmetics>,
    );

    const preview = tilePreviewOf("table-marble");
    fireEvent.click(await within(preview).findByTestId(T.tokenOffer("table-marble")));

    const dialog = screen.getByTestId(T.tokenOfferDialog("table-marble"));
    expect(screen.getByRole("alertdialog", { name: "Unlock Marble? This spends 1 Epic token." })).toBe(dialog);
    expect(preview).toContainElement(dialog);
    expect(screen.getByTestId(T.tokenOfferCancel("table-marble"))).toHaveFocus();
    expect(server.to("POST", "/api/tokens/redeem-cosmetic")).toHaveLength(0);
    expect(screen.getByTestId(T.tile("table-marble"))).toHaveAttribute("data-owned", "false");
  });
});

describe("R2730000 a card token is spent from the locked card's preview", () => {
  /** The grid's card `cardId` with the set open, under no ownership provider. */
  function gridOf(cardIds: readonly string[]): void {
    render(<OwnedCards collection={collectionOf()} defs={cardIds.map((id) => CATALOG[id]!)} token={TOKEN} />);
    fireEvent.click(screen.getByTestId(T.setToggle("Core")));
  }

  /** The preview of the grid's locked `cardId`, opened with the inspect key. */
  function cardPreviewOf(cardId: string): HTMLElement {
    fireEvent.keyDown(screen.getByTestId(T.card(cardId)), { key: "i" });
    return screen.getByTestId(T.preview);
  }

  it("R2730000 the preview of an owned cosmetic, a chase, a default or a starter shows no unlock button", async () => {
    tokenServer({ cosmeticEpic: 1 }, refusal(500, "internal", "unused"));
    render(
      <ViewerAccountCosmetics token={TOKEN} gated read={() => Promise.resolve(new Set(["table-marble"]))}>
        <AccountCosmeticTiles token={TOKEN} />
      </ViewerAccountCosmetics>,
    );

    // The positive control: a locked Epic playmat offers the held Epic token.
    tilePreviewOf("table-chessboard");
    expect(await within(screen.getByTestId(T.preview)).findByTestId(T.tokenOffer("table-chessboard"))).toHaveTextContent(
      "Unlock with an Epic token (1 held)",
    );
    closePreview();
    // Owned, a chase, a default and a starter show none.
    for (const id of ["table-marble", "table-astral", "table-tavern", "table-felt"]) {
      tilePreviewOf(id);
      await settle();
      expect(screen.getByTestId(T.preview)).toHaveAttribute("data-item", id);
      expect(within(screen.getByTestId(T.preview)).queryByTestId(T.tokenOffer(id)), id).toBeNull();
      closePreview();
    }
  });

  it("R2730000 a locked card's preview offers a card token only while one is held", async () => {
    let cardHeld = 1;
    const server = stubServer((call) => {
      if (call.method === "GET" && call.path === "/api/tokens") {
        return { status: 200, body: barsHolding({ card: cardHeld }) };
      }
      return undefined;
    });
    gridOf(["core-001", "core-002"]);

    expect(await within(cardPreviewOf("core-002")).findByTestId(T.tokenOffer("core-002"))).toHaveTextContent(
      "Unlock with a card token (1 held)",
    );
    expect(server.to("GET", "/api/tokens")).toHaveLength(1);
    closePreview();

    // Spent elsewhere, the token is gone: the locked card's preview offers nothing.
    cardHeld = 0;
    cleanup();
    gridOf(["core-001", "core-002"]);
    await settle();
    expect(within(cardPreviewOf("core-002")).queryByTestId(T.tokenOffer("core-002"))).toBeNull();
    closePreview();
    // An owned card's preview never offers one.
    expect(within(cardPreviewOf("core-001")).queryByTestId(T.tokenOffer("core-001"))).toBeNull();
  });

  it("R2730000 a card token asks first with Cancel focused; Cancel and Escape send nothing and leave the preview open", async () => {
    const server = tokenServer({ card: 1 }, refusal(500, "internal", "unused"));
    gridOf(["core-002"]);

    const preview = cardPreviewOf("core-002");
    fireEvent.click(await within(preview).findByTestId(T.tokenOffer("core-002")));

    const dialog = screen.getByTestId(T.tokenOfferDialog("core-002"));
    expect(screen.getByRole("alertdialog", { name: "Unlock Bigot? This spends 1 card token." })).toBe(dialog);
    expect(screen.getByTestId(T.tokenOfferCancel("core-002"))).toHaveFocus();
    expect(server.to("POST", "/api/tokens/redeem-card")).toHaveLength(0);

    fireEvent.click(screen.getByTestId(T.tokenOfferCancel("core-002")));
    expect(screen.queryByTestId(T.tokenOfferDialog("core-002"))).toBeNull();
    expect(screen.getByTestId(T.preview)).toBeInTheDocument();
    expect(within(preview).getByTestId(T.tokenOffer("core-002"))).toHaveFocus();

    fireEvent.click(within(preview).getByTestId(T.tokenOffer("core-002")));
    fireEvent.keyDown(screen.getByTestId(T.tokenOfferCancel("core-002")), { key: "Escape" });
    expect(screen.queryByTestId(T.tokenOfferDialog("core-002"))).toBeNull();
    expect(screen.getByTestId(T.preview)).toBeInTheDocument();
    expect(server.to("POST", "/api/tokens/redeem-card")).toHaveLength(0);
    expect(screen.getByTestId(T.card("core-002"))).toHaveAttribute("data-owned", "false");
  });

  it("R2730000 Confirm sends one redeem-card with an idempotency key, and the card reads owned in the preview and the grid", async () => {
    let owned = collectionOf();
    const server = tokenServer({ card: 1 }, { status: 200, body: barsHolding({}) }, () => owned);
    gridOf(["core-002"]);

    const preview = cardPreviewOf("core-002");
    fireEvent.click(await within(preview).findByTestId(T.tokenOffer("core-002")));
    // The redemption grants the card, so the collection's re-read owns it.
    owned = collectionOwning(["core-001", "core-002", "core-004", "core-051-1"]);
    fireEvent.click(screen.getByTestId(T.tokenOfferConfirm("core-002")));

    await waitFor(() => {
      expect(preview).toHaveAttribute("data-owned", "true");
    });
    expect(screen.getByTestId(T.card("core-002"))).toHaveAttribute("data-owned", "true");
    expect(within(preview).getByTestId(T.previewStatus)).toHaveTextContent("Owned");
    expect(server.to("POST", "/api/tokens/redeem-card")).toHaveLength(1);
    expect(server.to("POST", "/api/tokens/redeem-card")[0]?.body).toEqual({ cardId: "core-002" });
    expect(server.to("POST", "/api/tokens/redeem-card")[0]?.headers["idempotency-key"]).toMatch(/[0-9a-f-]{8,}/);
    expect(server.to("GET", "/api/collection")).toHaveLength(1);
    expect(screen.queryByTestId(T.tokenOfferDialog("core-002"))).toBeNull();
  });

  it("R2730000 a refused card token shows the server's words and changes nothing", async () => {
    const server = tokenServer({ card: 1 }, refusal(409, "conflict", "that card is already owned"), () => collectionOf());
    gridOf(["core-002"]);

    const preview = cardPreviewOf("core-002");
    fireEvent.click(await within(preview).findByTestId(T.tokenOffer("core-002")));
    fireEvent.click(screen.getByTestId(T.tokenOfferConfirm("core-002")));

    expect(await within(preview).findByTestId(T.tokenOfferError("core-002"))).toHaveTextContent("that card is already owned");
    expect(within(preview).getByTestId(T.tokenOffer("core-002"))).toHaveTextContent("Unlock with a card token (1 held)");
    expect(preview).toHaveAttribute("data-owned", "false");
    expect(screen.getByTestId(T.card("core-002"))).toHaveAttribute("data-owned", "false");
    expect(server.to("POST", "/api/tokens/redeem-card")).toHaveLength(1);
    expect(server.to("GET", "/api/collection"), "nothing was granted, so the collection is not read again").toHaveLength(0);
  });
});
