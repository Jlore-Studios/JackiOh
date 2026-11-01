// R1953001 (#1953): a held cosmetic token is offered where a locked cosmetic is shown, a card's own
// cosmetics view and the account's tiles, and only where its tier covers the cosmetic (R1305003):
// the offer redeems it through `POST /api/tokens/redeem-cosmetic` and the cosmetic shows owned once
// the screen's ownership reads again; a refusal shows the server's words and changes nothing.
// R2480000 (#2480): the offer asks first; Confirm redeems, Cancel and Escape spend nothing.

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { resetCosmeticsForTests, type OwnedByCard } from "../cards/cosmetics.ts";
import { ViewerAccountCosmetics, ViewerFullArt, resetOwnedFullArtForTests } from "../cards/ownedFullArt.ts";
import { resetLocaleForTests, setLocale } from "../i18n/i18n.ts";
import type { TokenBarsResponse } from "../net/api.ts";
import { collectionOf, tokenBars } from "../shop/fixtures.ts";
import { refusal, stubServer, type Reply } from "../test/stubServer.ts";
import { COSMETICS } from "../wire/generated/cosmetics.ts";
import AccountCosmeticTiles from "./AccountCosmeticTiles.tsx";
import { collectionTestid as T } from "./collection.ts";
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

function held(over: Partial<HeldTokens> = {}): HeldTokens {
  return { card: 0, cosmeticRare: 0, cosmeticEpic: 0, cosmeticPack: 0, ...over };
}

function barsHolding(tokens: Partial<HeldTokens>): TokenBarsResponse {
  return { ...tokenBars(), tokens: held(tokens) };
}

/** `GET /api/tokens` holding `tokens`, and the redemption's `redeem` answer. */
function tokenServer(tokens: Partial<HeldTokens>, redeem: Reply): ReturnType<typeof stubServer> {
  return stubServer((call) => {
    if (call.method === "GET" && call.path === "/api/tokens") return { status: 200, body: barsHolding(tokens) };
    if (call.method === "POST" && call.path === "/api/tokens/redeem-cosmetic") return redeem;
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

describe("R1953001 a held cosmetic token, offered in place", () => {
  it("R1953001 a locked Epic cosmetic offers a held Epic token, redeems it and shows it owned", async () => {
    const server = tokenServer({ cosmeticEpic: 2 }, { status: 200, body: barsHolding({ cosmeticEpic: 1 }) });
    const read = vi
      .fn<(token: string) => Promise<OwnedByCard>>()
      .mockResolvedValueOnce(new Map())
      .mockResolvedValue(new Map([["core-001", new Set(["foil-galaxy"])]]));
    cardViewOf(read);

    const offer = await screen.findByTestId(T.tokenOffer("foil-galaxy"));
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
    expect(screen.queryByTestId(T.tokenOffer("foil-galaxy"))).toBeNull();
    // The answer's held count is the one the other offers now say.
    expect(screen.getByTestId(T.tokenOffer("border-runes"))).toHaveTextContent("Unlock with an Epic token (1 held)");
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
    expect(await screen.findByTestId(T.tokenOffer("foil-sunburst"))).toHaveTextContent("Unlock with a Rare token (3 held)");
    expect(screen.getByTestId(T.tokenOffer("foil-glitter"))).toHaveTextContent("Unlock with a Rare token (3 held)");
    expect(screen.queryByTestId(T.tokenOffer("foil-galaxy"))).toBeNull();
  });

  it("R1953001 a chase never shows the offer", async () => {
    const plenty = held({ cosmeticRare: 9, cosmeticEpic: 9 });
    const chases = COSMETICS.filter((cosmetic) => cosmetic.chase);
    expect(chases.length).toBeGreaterThan(0);
    for (const chase of chases) expect(offerTier(chase, plenty), chase.id).toBeNull();

    tokenServer({ cosmeticRare: 9, cosmeticEpic: 9 }, refusal(500, "internal", "unused"));
    cardViewOf(() => Promise.resolve(new Map()));
    expect(await screen.findByTestId(T.tokenOffer("foil-galaxy"))).toBeInTheDocument();
    for (const id of ["foil-molten-gold", "foil-void-shimmer", "finish-obsidian", "finish-starlit", "border-filigree", "border-celestial"]) {
      expect(screen.getByTestId(T.stripItem(id)), id).toHaveAttribute("data-owned", "false");
      expect(screen.queryByTestId(T.tokenOffer(id)), id).toBeNull();
    }
  });

  it("R1953001 a refusal shows the server's words and changes nothing", async () => {
    const server = tokenServer({ cosmeticEpic: 1 }, refusal(409, "conflict", "that cosmetic is already owned there"));
    const read = vi.fn<(token: string) => Promise<OwnedByCard>>().mockResolvedValue(new Map());
    cardViewOf(read);

    fireEvent.click(await screen.findByTestId(T.tokenOffer("foil-galaxy")));
    fireEvent.click(screen.getByTestId(T.tokenOfferConfirm("foil-galaxy")));

    expect(await screen.findByTestId(T.tokenOfferError("foil-galaxy"))).toHaveTextContent("that cosmetic is already owned there");
    expect(screen.getByTestId(T.tokenOffer("foil-galaxy"))).toHaveTextContent("Unlock with an Epic token (1 held)");
    expect(screen.getByTestId(T.stripItem("foil-galaxy"))).toHaveAttribute("data-owned", "false");
    expect(server.to("POST", "/api/tokens/redeem-cosmetic")).toHaveLength(1);
    expect(read, "nothing was granted, so ownership is not read again").toHaveBeenCalledTimes(1);
  });

  it("#2053 a refusal says why in the reader's language", async () => {
    await setLocale("zh-CN");
    tokenServer({ cosmeticEpic: 1 }, { status: 500 });
    cardViewOf(vi.fn<(token: string) => Promise<OwnedByCard>>().mockResolvedValue(new Map()));

    fireEvent.click(await screen.findByTestId(T.tokenOffer("foil-galaxy")));
    fireEvent.click(screen.getByTestId(T.tokenOfferConfirm("foil-galaxy")));

    expect(await screen.findByTestId(T.tokenOfferError("foil-galaxy"))).toHaveTextContent("服务器返回了 500");
  });

  it("R1953001 a locked playmat tile offers an Epic token", async () => {
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

    const offer = await screen.findByTestId(T.tokenOffer("table-marble"));
    expect(offer).toHaveTextContent("Unlock with an Epic token (1 held)");
    expect(screen.getByTestId(T.tile("table-astral"))).toHaveAttribute("data-owned", "false");
    expect(screen.queryByTestId(T.tokenOffer("table-astral"))).toBeNull();
    expect(screen.queryByTestId(T.tokenOffer("table-felt"))).toBeNull();

    fireEvent.click(offer);
    fireEvent.click(screen.getByTestId(T.tokenOfferConfirm("table-marble")));

    await waitFor(() => {
      expect(screen.getByTestId(T.tile("table-marble"))).toHaveAttribute("data-owned", "true");
    });
    // An account cosmetic names no card.
    expect(server.to("POST", "/api/tokens/redeem-cosmetic")[0]?.body).toEqual({ token: "Epic", cosmeticId: "table-marble" });
    // The answer holds no token now, so no locked tile offers one.
    expect(document.querySelector('[data-testid^="collection-token-offer-"]')).toBeNull();
  });

  it("R1953001 nothing is offered without the session's token, and the held tokens are not read", () => {
    const server = tokenServer({ cosmeticEpic: 1 }, refusal(500, "internal", "unused"));
    render(
      <ViewerAccountCosmetics token={TOKEN} gated read={() => Promise.resolve(new Set())}>
        <AccountCosmeticTiles />
      </ViewerAccountCosmetics>,
    );

    expect(screen.getByTestId(T.tile("table-marble"))).toHaveAttribute("data-owned", "false");
    expect(screen.queryByTestId(T.tokenOffer("table-marble"))).toBeNull();
    expect(server.to("GET", "/api/tokens")).toHaveLength(0);
  });
});

describe("R2480000 spending a cosmetic token asks first", () => {
  it("R2480000 the first click sends nothing and asks in the card view, naming the cosmetic and the token, with Cancel focused", async () => {
    const server = tokenServer({ cosmeticEpic: 2 }, refusal(500, "internal", "unused"));
    cardViewOf(() => Promise.resolve(new Map()));

    fireEvent.click(await screen.findByTestId(T.tokenOffer("foil-galaxy")));

    const dialog = screen.getByTestId(T.tokenOfferDialog("foil-galaxy"));
    expect(screen.getByRole("alertdialog", { name: "Unlock Galaxy? This spends 1 Epic token." })).toBe(dialog);
    expect(screen.getByTestId(T.cardView)).toContainElement(dialog);
    expect(screen.queryByTestId(T.tokenOffer("foil-galaxy"))).toBeNull();
    expect(screen.getByTestId(T.tokenOfferCancel("foil-galaxy"))).toHaveFocus();
    expect(server.to("POST", "/api/tokens/redeem-cosmetic")).toHaveLength(0);
  });

  it("R2480000 a Rare token's question names the Rare token", async () => {
    tokenServer({ cosmeticRare: 1 }, refusal(500, "internal", "unused"));
    cardViewOf(() => Promise.resolve(new Map()));

    fireEvent.click(await screen.findByTestId(T.tokenOffer("foil-sunburst")));

    expect(screen.getByRole("alertdialog", { name: "Unlock Sunburst? This spends 1 Rare token." })).toBeInTheDocument();
  });

  it("R2480000 Confirm sends exactly one redemption, of the token the question named", async () => {
    const server = tokenServer({ cosmeticEpic: 1 }, { status: 200, body: barsHolding({}) });
    const read = vi
      .fn<(token: string) => Promise<OwnedByCard>>()
      .mockResolvedValueOnce(new Map())
      .mockResolvedValue(new Map([["core-001", new Set(["foil-sunburst"])]]));
    cardViewOf(read);

    fireEvent.click(await screen.findByTestId(T.tokenOffer("foil-sunburst")));
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

    fireEvent.click(await screen.findByTestId(T.tokenOffer("foil-galaxy")));
    fireEvent.click(screen.getByTestId(T.tokenOfferCancel("foil-galaxy")));

    expect(screen.queryByTestId(T.tokenOfferDialog("foil-galaxy"))).toBeNull();
    const offer = screen.getByTestId(T.tokenOffer("foil-galaxy"));
    expect(offer).toHaveTextContent("Unlock with an Epic token (1 held)");
    expect(offer).toHaveFocus();

    fireEvent.click(offer);
    fireEvent.keyDown(screen.getByTestId(T.tokenOfferCancel("foil-galaxy")), { key: "Escape" });

    expect(screen.queryByTestId(T.tokenOfferDialog("foil-galaxy"))).toBeNull();
    expect(screen.getByTestId(T.cardView)).toBeInTheDocument();
    expect(screen.getByTestId(T.tokenOffer("foil-galaxy"))).toHaveFocus();

    expect(server.to("POST", "/api/tokens/redeem-cosmetic")).toHaveLength(0);
    expect(screen.getByTestId(T.stripItem("foil-galaxy"))).toHaveAttribute("data-owned", "false");
    expect(read, "nothing was granted, so ownership is not read again").toHaveBeenCalledTimes(1);
  });

  it("R2480000 a locked tile asks in the tile's own place", async () => {
    const server = tokenServer({ cosmeticEpic: 1 }, refusal(500, "internal", "unused"));
    render(
      <ViewerAccountCosmetics token={TOKEN} gated read={() => Promise.resolve(new Set())}>
        <AccountCosmeticTiles token={TOKEN} />
      </ViewerAccountCosmetics>,
    );

    fireEvent.click(await screen.findByTestId(T.tokenOffer("table-marble")));

    const dialog = screen.getByTestId(T.tokenOfferDialog("table-marble"));
    expect(screen.getByRole("alertdialog", { name: "Unlock Marble? This spends 1 Epic token." })).toBe(dialog);
    expect(screen.getByTestId(T.tile("table-marble"))).toContainElement(dialog);
    expect(screen.getByTestId(T.tokenOfferCancel("table-marble"))).toHaveFocus();
    expect(server.to("POST", "/api/tokens/redeem-cosmetic")).toHaveLength(0);
    expect(screen.getByTestId(T.tile("table-marble"))).toHaveAttribute("data-owned", "false");
  });
});
