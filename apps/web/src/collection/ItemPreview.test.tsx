// R2730000 (#2730): holding (LONG_PRESS_MS, touch or mouse) or the inspect key on any account
// cosmetic tile, card-cosmetic strip item or owned-cards grid card opens its fullscreen preview, and
// nothing else carries a spend button. A plain tap or click still equips or opens, as today, and a
// hold fires no click.

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { LONG_PRESS_MS } from "../cards/inspect/constants.ts";
import {
  AccountCosmeticsOwned,
  readCardPicks,
  readCosmetics,
  resetCosmeticsForTests,
} from "../cards/cosmetics.ts";
import { ViewerFullArt, resetOwnedFullArtForTests } from "../cards/ownedFullArt.ts";
import type { TokenBarsResponse } from "../net/api.ts";
import { collectionOf, tokenBars } from "../shop/fixtures.ts";
import { settle } from "../test/settle.ts";
import { stubServer, type Reply } from "../test/stubServer.ts";
import AccountCosmeticTiles from "./AccountCosmeticTiles.tsx";
import { collectionTestid as T } from "./collection.ts";
import OwnedCards from "./OwnedCards.tsx";
import type { HeldTokens } from "./TokenOffer.tsx";

const TOKEN = "token-1";

/** Held tokens, none but those given. */
function held(over: Partial<HeldTokens> = {}): HeldTokens {
  return { card: 0, cosmeticRare: 0, cosmeticEpic: 0, cosmeticPack: 0, ...over };
}

/** `GET /api/tokens` holding `tokens`. */
function tokenServer(tokens: Partial<HeldTokens>, redeem?: Reply): ReturnType<typeof stubServer> {
  return stubServer((call) => {
    if (call.method === "GET" && call.path === "/api/tokens") {
      return { status: 200, body: { ...tokenBars(), tokens: held(tokens) } satisfies TokenBarsResponse };
    }
    if (redeem !== undefined && call.method === "POST" && call.path === "/api/tokens/redeem-card") return redeem;
    if (redeem !== undefined && call.method === "POST" && call.path === "/api/tokens/redeem-cosmetic") return redeem;
    return undefined;
  });
}

beforeEach(() => {
  localStorage.clear();
  resetCosmeticsForTests();
  resetOwnedFullArtForTests();
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  localStorage.clear();
  resetCosmeticsForTests();
  resetOwnedFullArtForTests();
});

/** A hold past LONG_PRESS_MS, then the lift and its swallowed click. */
function hold(element: HTMLElement, pointerType = "mouse"): void {
  fireEvent.pointerDown(element, { pointerType, button: 0, pointerId: 1, clientX: 100, clientY: 100 });
  act(() => {
    vi.advanceTimersByTime(LONG_PRESS_MS);
  });
  fireEvent.pointerUp(element, { pointerType, button: 0, pointerId: 1, clientX: 100, clientY: 100 });
  fireEvent.click(element);
}

/** A press short of LONG_PRESS_MS, then the lift and its click. */
function tap(element: HTMLElement): void {
  fireEvent.pointerDown(element, { pointerType: "mouse", button: 0, pointerId: 1, clientX: 100, clientY: 100 });
  act(() => {
    vi.advanceTimersByTime(100);
  });
  fireEvent.pointerUp(element, { pointerType: "mouse", button: 0, pointerId: 1, clientX: 100, clientY: 100 });
  fireEvent.click(element);
}

/** core-002, which the collection fixture does not own. */
const LOCKED_CARD = CATALOG["core-002"]!;
/** core-001, which the collection fixture owns. */
const OWNED_CARD = CATALOG["core-001"]!;

function tiles(): void {
  render(
    <AccountCosmeticsOwned owned={new Set()}>
      <AccountCosmeticTiles />
    </AccountCosmeticsOwned>,
  );
}

function tileButton(id: string): HTMLElement {
  return within(screen.getByTestId(T.tile(id))).getByRole("button");
}

describe("R2730000 holding or inspecting an item previews it fullscreen", () => {
  it("R2730000 holding an owned tile by mouse or touch opens its preview and equips nothing; a short press still equips", async () => {
    tiles();
    await settle();
    expect(tileButton("card-back-lacquer")).not.toHaveFocus();

    // A mouse hold opens the preview and equips nothing.
    hold(tileButton("card-back-lacquer"), "mouse");
    const preview = screen.getByTestId(T.preview);
    expect(preview).toHaveAttribute("data-item", "card-back-lacquer");
    expect(preview).toHaveAttribute("data-owned", "true");
    expect(within(preview).getByTestId(T.previewStatus)).toHaveTextContent("Owned");
    expect(readCosmetics().cardBack).not.toBe("card-back-lacquer");

    // Close gives the focus back to the tile that opened it.
    fireEvent.click(within(preview).getByTestId(T.previewClose));
    expect(screen.queryByTestId(T.preview)).toBeNull();
    expect(tileButton("card-back-lacquer")).toHaveFocus();

    // A touch hold opens it too.
    hold(tileButton("card-back-lacquer"), "touch");
    expect(screen.getByTestId(T.preview)).toHaveAttribute("data-item", "card-back-lacquer");
    expect(readCosmetics().cardBack).not.toBe("card-back-lacquer");
    fireEvent.click(within(screen.getByTestId(T.preview)).getByTestId(T.previewClose));

    // A short press still equips.
    tap(tileButton("card-back-lacquer"));
    expect(readCosmetics().cardBack).toBe("card-back-lacquer");
    expect(screen.queryByTestId(T.preview)).toBeNull();
  });

  it("R2730000 holding a grid card opens its preview, not its view; a click opens the view", async () => {
    render(<OwnedCards collection={collectionOf()} defs={[LOCKED_CARD]} />);
    await settle();
    fireEvent.click(screen.getByTestId(T.setToggle("Core")));
    const card = screen.getByTestId(T.card("core-002"));
    expect(card).toHaveAttribute("data-owned", "false");

    hold(card);
    expect(screen.getByTestId(T.preview)).toHaveAttribute("data-item", "core-002");
    expect(within(screen.getByTestId(T.preview)).getByTestId(T.previewStatus)).toHaveTextContent("Locked");
    expect(screen.queryByTestId(T.cardView)).toBeNull();

    // Escape closes the preview.
    fireEvent.keyDown(document.body, { key: "Escape" });
    expect(screen.queryByTestId(T.preview)).toBeNull();

    // A click opens the view, as today.
    fireEvent.click(screen.getByTestId(T.card("core-002")));
    expect(screen.getByTestId(T.cardView)).toBeInTheDocument();
  });

  it("R2730000 holding a strip item shows the card wearing it and writes no pick; Escape closes the preview, then the view", async () => {
    tokenServer({});
    render(
      <ViewerFullArt token={TOKEN} gated read={() => Promise.resolve(new Map())}>
        <OwnedCards collection={collectionOf()} defs={[OWNED_CARD]} token={TOKEN} />
      </ViewerFullArt>,
    );
    await settle();
    fireEvent.click(screen.getByTestId(T.setToggle("Core")));
    fireEvent.click(screen.getByTestId(T.card("core-001")));
    const item = screen.getByTestId(T.stripItem("foil-galaxy"));

    hold(item);
    const preview = screen.getByTestId(T.preview);
    expect(preview).toHaveAttribute("data-item", "foil-galaxy");
    // The preview shows the card wearing the item, and no pick was written.
    expect(within(preview).getByTestId(T.cardViewPreview)).toHaveAttribute("data-previewing", "foil-galaxy");
    expect(readCardPicks("core-001")).toEqual({});

    // Escape closes the preview first, then the view.
    fireEvent.keyDown(document.body, { key: "Escape" });
    expect(screen.queryByTestId(T.preview)).toBeNull();
    expect(screen.getByTestId(T.cardView)).toBeInTheDocument();
    fireEvent.keyDown(document.body, { key: "Escape" });
    expect(screen.queryByTestId(T.cardView)).toBeNull();
    expect(readCardPicks("core-001")).toEqual({});
  });

  it("R2730000 the inspect key on a focused tile, strip item or card opens the preview", async () => {
    tokenServer({ cosmeticEpic: 1, card: 1 });
    render(
      <AccountCosmeticsOwned owned={new Set()}>
        <AccountCosmeticTiles token={TOKEN} />
      </AccountCosmeticsOwned>,
    );
    await settle();
    fireEvent.keyDown(tileButton("table-marble"), { key: "i" });
    expect(screen.getByTestId(T.preview)).toHaveAttribute("data-item", "table-marble");
    fireEvent.keyDown(document.body, { key: "Escape" });

    cleanup();
    render(
      <ViewerFullArt token={TOKEN} gated read={() => Promise.resolve(new Map())}>
        <OwnedCards collection={collectionOf()} defs={[OWNED_CARD]} token={TOKEN} />
      </ViewerFullArt>,
    );
    await settle();
    fireEvent.click(screen.getByTestId(T.setToggle("Core")));
    fireEvent.click(screen.getByTestId(T.card("core-001")));
    fireEvent.keyDown(screen.getByTestId(T.stripItem("foil-galaxy")), { key: "I" });
    expect(screen.getByTestId(T.preview)).toHaveAttribute("data-item", "foil-galaxy");
    fireEvent.keyDown(document.body, { key: "Escape" });

    cleanup();
    render(<OwnedCards collection={collectionOf()} defs={[LOCKED_CARD]} />);
    await settle();
    fireEvent.click(screen.getByTestId(T.setToggle("Core")));
    fireEvent.keyDown(screen.getByTestId(T.card("core-002")), { key: "F10", shiftKey: true });
    expect(screen.getByTestId(T.preview)).toHaveAttribute("data-item", "core-002");
  });

  it("R2730000 the preview traps Tab, closes on Escape, Close and the backdrop, and gives the focus back", async () => {
    tiles();
    await settle();
    const opener = tileButton("table-tavern");
    opener.focus();
    fireEvent.keyDown(opener, { key: "i" });
    const preview = screen.getByTestId(T.preview);
    const close = within(preview).getByTestId(T.previewClose);
    expect(close).toHaveFocus();

    // Tab on the last button wraps to the first (here the only one).
    fireEvent.keyDown(preview, { key: "Tab" });
    expect(close).toHaveFocus();

    // The backdrop closes it and gives the focus back.
    fireEvent.click(within(preview).getByTestId(T.previewScrim));
    expect(screen.queryByTestId(T.preview)).toBeNull();
    expect(opener).toHaveFocus();

    // Close closes it too.
    fireEvent.keyDown(opener, { key: "i" });
    fireEvent.click(within(screen.getByTestId(T.preview)).getByTestId(T.previewClose));
    expect(screen.queryByTestId(T.preview)).toBeNull();
    expect(opener).toHaveFocus();

    // So does Escape.
    fireEvent.keyDown(opener, { key: "i" });
    expect(screen.getByTestId(T.preview)).toBeInTheDocument();
    fireEvent.keyDown(document.body, { key: "Escape" });
    expect(screen.queryByTestId(T.preview)).toBeNull();
    expect(opener).toHaveFocus();
  });

  it("R2730000 no tile, strip item or grid card carries a spend button, whatever tokens are held", async () => {
    tokenServer({ cosmeticEpic: 1, card: 1 });
    render(
      <>
        <AccountCosmeticsOwned owned={new Set()}>
          <AccountCosmeticTiles token={TOKEN} />
        </AccountCosmeticsOwned>
        <ViewerFullArt token={TOKEN} gated read={() => Promise.resolve(new Map())}>
          <OwnedCards collection={collectionOf()} defs={[OWNED_CARD, LOCKED_CARD]} token={TOKEN} />
        </ViewerFullArt>
      </>,
    );
    await settle();
    fireEvent.click(screen.getByTestId(T.setToggle("Core")));

    // No preview open: no spend button anywhere.
    expect(document.querySelector('[data-testid^="collection-token-offer-"]')).toBeNull();

    // A tile's offer lives in its preview alone.
    fireEvent.keyDown(tileButton("table-marble"), { key: "i" });
    let preview = screen.getByTestId(T.preview);
    await settle();
    expect(within(preview).getByTestId(T.tokenOffer("table-marble"))).toBeInTheDocument();
    expect(document.querySelectorAll('[data-testid^="collection-token-offer-"]')).toHaveLength(1);
    fireEvent.keyDown(document.body, { key: "Escape" });
    expect(document.querySelector('[data-testid^="collection-token-offer-"]')).toBeNull();

    // A grid card's offer lives in its preview alone.
    fireEvent.keyDown(screen.getByTestId(T.card("core-002")), { key: "i" });
    preview = screen.getByTestId(T.preview);
    await settle();
    expect(within(preview).getByTestId(T.tokenOffer("core-002"))).toBeInTheDocument();
    expect(document.querySelectorAll('[data-testid^="collection-token-offer-"]')).toHaveLength(1);
    expect(screen.queryByTestId(T.cardView)).toBeNull();
    fireEvent.keyDown(document.body, { key: "Escape" });

    // A strip item's offer lives in its preview alone.
    fireEvent.click(screen.getByTestId(T.card("core-001")));
    fireEvent.keyDown(screen.getByTestId(T.stripItem("foil-galaxy")), { key: "i" });
    preview = screen.getByTestId(T.preview);
    await settle();
    expect(within(preview).getByTestId(T.tokenOffer("foil-galaxy"))).toBeInTheDocument();
    for (const offer of document.querySelectorAll<HTMLElement>('[data-testid^="collection-token-offer-"]')) {
      expect(preview).toContainElement(offer);
    }
  });
});
