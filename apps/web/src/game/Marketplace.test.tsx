// R1853003, R2250002: the Marketplace's shop (Marketplace.tsx), mounted by Game and read off the view and
// `legal` alone. The view is a fixture shaped as the engine's `marketplace_view` builds it (each of the
// viewer's offers' index, face, price and copies left, the engine's `reason` on one it refuses, the
// viewer's priced hand cards that sell, R1861000); the shop must send exactly the `marketBuy` or
// `marketSell` `legal` lists, and nothing for an offer or a sale it does not (CLAUDE.md rule 7).
// R2127000: Back closes the open shop, as Escape does.

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { ActionBody, MarketplaceView, PlayerView } from "@jackioh/shared";

import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import { testid } from "./contract.ts";
import Game from "./Game.tsx";
import { guardLeaving, paths, usePathname } from "../net/navigate.ts";
import { pressBack } from "../test/history.ts";
import { baseView, card, emptySide, pendingFor } from "../test/fixtures.ts";
import { at } from "../test/stubServer.ts";

afterEach(cleanup);

const lookup = lookupFromDefs(CATALOG);

const BUY_0: ActionBody = { type: "marketBuy", offer: 0 };
const SELL_R9: ActionBody = { type: "marketSell", instanceId: "r9" };
const END_TURN: ActionBody = { type: "endTurn" };
const REASON = "Zao Gao costs 60 Credits and you have 45";

/** Mr. Vanilla at 40 Credits, a Radiant Zao Gao at 60 the engine refuses, and the hand's `r9` quoted at 2. */
const MARKET: MarketplaceView = {
  offers: [
    { offer: 0, defId: "core-008", price: 40, left: 2 },
    { offer: 1, defId: "core-080", radiant: true, price: 60, left: 1, reason: REASON },
  ],
  sales: [{ instanceId: "r9", defId: "fx-res-rare", credits: 2 }],
};

/** p1's main phase with 45 Credits and a card that sells in hand, at `market` (absent: still closed). */
function marketView(market?: MarketplaceView): PlayerView {
  return baseView({
    you: emptySide("p1", { hand: [card({ instanceId: "r9", defId: "fx-res-rare" })], marketCredits: 45 }),
    ...(market === undefined ? {} : { marketplace: market }),
  });
}

function renderGame(view: PlayerView, legal: readonly ActionBody[]) {
  const onAction = vi.fn<(body: ActionBody) => void>();
  render(
    <CatalogContext.Provider value={lookup}>
      <Game view={view} legal={legal} onAction={onAction} />
    </CatalogContext.Provider>,
  );
  return onAction;
}

/** The Marketplace's actions sent, in order (Game sends R345's preference on its own). */
function marketSent(onAction: ReturnType<typeof renderGame>): ActionBody[] {
  return onAction.mock.calls.map(([body]) => body).filter((body) => body.type === "marketBuy" || body.type === "marketSell");
}

function openShop(): HTMLElement {
  fireEvent.click(screen.getByTestId(testid.marketplace));
  return screen.getByTestId(testid.marketplacePanel);
}

function button(testId: string): HTMLButtonElement {
  return screen.getByTestId(testId) as HTMLButtonElement;
}

describe("R1853003 the Marketplace's shop", () => {
  it("R1853003 renders nothing while the view carries no marketplace", () => {
    renderGame(marketView(), [END_TURN]);
    expect(screen.queryByTestId(testid.marketplace)).toBeNull();
    expect(screen.queryByTestId(testid.marketplacePanel)).toBeNull();
    expect(screen.queryByTestId(testid.marketSell("r9"))).toBeNull();
  });

  it("R1853003 a listed offer's Buy sends exactly its marketBuy", () => {
    const onAction = renderGame(marketView(MARKET), [BUY_0, SELL_R9, END_TURN]);
    expect(screen.queryByTestId(testid.marketplacePanel)).toBeNull();
    const panel = openShop();
    expect(panel).toHaveTextContent("45 Credits");
    expect(panel).toHaveTextContent("New offers at the start of each of your turns");
    const buy = button(testid.marketBuy(0));
    expect(buy.disabled).toBe(false);
    expect(buy.getAttribute("data-legal")).toBe("true");
    expect(buy.title).toBe("");
    const lot = buy.closest("li");
    expect(lot).toHaveTextContent("Mr. Vanilla");
    expect(lot).toHaveTextContent("40 Credits");
    expect(lot).toHaveTextContent("2 left");
    fireEvent.click(buy);
    const sent = marketSent(onAction);
    expect(sent).toEqual([BUY_0]);
    expect(sent[0]).toBe(BUY_0);
    fireEvent.click(screen.getByTestId(testid.marketplace));
    expect(screen.queryByTestId(testid.marketplacePanel)).toBeNull();
  });

  it("R1853003 an offer the engine refuses is greyed with its reason and sends nothing", () => {
    const onAction = renderGame(marketView(MARKET), [BUY_0, END_TURN]);
    openShop();
    const refused = button(testid.marketBuy(1));
    expect(refused.disabled).toBe(true);
    expect(refused.getAttribute("data-legal")).toBe("false");
    expect(refused.title).toBe(REASON);
    const lot = refused.closest("li");
    expect(lot).toHaveTextContent("60 Credits");
    expect(lot).toHaveTextContent("1 left");
    expect(lot?.querySelector('[data-radiant-face="true"]')).not.toBeNull();
    fireEvent.click(refused);
    expect(marketSent(onAction)).toEqual([]);
    cleanup();
    // A lot `legal` does not list and the view gives no reason for (the viewer may not act) is greyed
    // with no tooltip.
    const waiting = renderGame(marketView(MARKET), []);
    openShop();
    const idle = button(testid.marketBuy(0));
    expect(idle.disabled).toBe(true);
    expect(idle.title).toBe("");
    fireEvent.click(idle);
    expect(marketSent(waiting)).toEqual([]);
  });

  it("R1853003 R1861000 a priced hand card's Sell sends its marketSell", () => {
    const onAction = renderGame(marketView(MARKET), [BUY_0, SELL_R9, END_TURN]);
    openShop();
    const sell = button(testid.marketSell("r9"));
    expect(sell.disabled).toBe(false);
    expect(sell).toHaveTextContent("Sell fx-res-rare for 2 Credits");
    fireEvent.click(sell);
    const sent = marketSent(onAction);
    expect(sent).toEqual([SELL_R9]);
    expect(sent[0]).toBe(SELL_R9);
    cleanup();
    // A sale `legal` does not list is greyed and sends nothing.
    const unlisted = renderGame(marketView(MARKET), [END_TURN]);
    openShop();
    const greyed = button(testid.marketSell("r9"));
    expect(greyed.disabled).toBe(true);
    fireEvent.click(greyed);
    expect(marketSent(unlisted)).toEqual([]);
  });
});

describe("R2127000 Back closes the Marketplace", () => {
  /** Reads the path: its `usePathname` installs `navigate.ts`'s `popstate` listener, where the leave guard is read. */
  function Path(): ReactElement {
    return <p data-testid="path">{usePathname()}</p>;
  }

  function tree(view: PlayerView, legal: readonly ActionBody[], onAction: (body: ActionBody) => void): ReactElement {
    return (
      <CatalogContext.Provider value={lookup}>
        <Game view={view} legal={legal} onAction={onAction} />
        <Path />
      </CatalogContext.Provider>
    );
  }

  /** The match's address with the deck builder's under it, as a player arrives in a game. */
  function arrive(): void {
    at("/decks");
    window.history.pushState(null, "", paths.match("m1"));
  }

  it("R2127000 one Back closes the open Marketplace and keeps the match's address without asking the leave guard; the next Back meets it", async () => {
    arrive();
    const held: (() => void)[] = [];
    const release = guardLeaving((leave) => {
      held.push(leave);
    });
    try {
      render(tree(marketView(MARKET), [BUY_0, SELL_R9, END_TURN], vi.fn()));
      openShop();
      button(testid.marketBuy(0)).focus();
      await pressBack();
      expect(screen.queryByTestId(testid.marketplacePanel)).toBeNull();
      expect(screen.getByTestId(testid.marketplace)).toHaveFocus();
      expect(window.location.pathname).toBe(paths.match("m1"));
      expect(held).toHaveLength(0);
      // The layer is spent: the next Back is the page's, and the guard holds it.
      await pressBack();
      expect(held).toHaveLength(1);
      expect(window.location.pathname).toBe(paths.match("m1"));
    } finally {
      release();
    }
  });

  it("R2127000 Escape closes the Marketplace once and leaves no layer: the next Back meets the leave guard", async () => {
    arrive();
    const held: (() => void)[] = [];
    const release = guardLeaving((leave) => {
      held.push(leave);
    });
    try {
      render(tree(marketView(MARKET), [BUY_0, SELL_R9, END_TURN], vi.fn()));
      openShop();
      const buy = button(testid.marketBuy(0));
      buy.focus();
      fireEvent.keyDown(buy, { key: "Escape" });
      expect(screen.queryByTestId(testid.marketplacePanel)).toBeNull();
      expect(screen.getByTestId(testid.marketplace)).toHaveFocus();
      await pressBack();
      expect(held).toHaveLength(1);
      expect(window.location.pathname).toBe(paths.match("m1"));
    } finally {
      release();
    }
  });

  it("R2127000 with a prompt drawn over the open Marketplace, Back closes the Marketplace and the prompt stays unanswered", async () => {
    arrive();
    const onAction = vi.fn<(body: ActionBody) => void>();
    const { rerender } = render(tree(marketView(MARKET), [END_TURN], onAction));
    openShop();
    rerender(
      tree(
        {
          ...marketView(MARKET),
          pending: pendingFor("discover", [
            { key: "mode:core-043", label: "Flood", defId: "core-043" },
            { key: "mode:core-055", label: "Archivist", defId: "core-055" },
          ]),
        },
        [END_TURN],
        onAction,
      ),
    );
    expect(screen.getByTestId("prompt-modal")).toBeInTheDocument();
    await pressBack();
    expect(screen.queryByTestId(testid.marketplacePanel)).toBeNull();
    expect(screen.getByTestId("prompt-modal")).toBeInTheDocument();
    expect(onAction.mock.calls.filter(([body]) => body.type === "answer")).toEqual([]);
    expect(marketSent(onAction)).toEqual([]);
    expect(window.location.pathname).toBe(paths.match("m1"));
    // A prompt is never a layer: the next Back leaves the match's page.
    await pressBack();
    expect(window.location.pathname).toBe("/decks");
  });
});
