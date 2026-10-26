// R1853003: the Marketplace's shop (Marketplace.tsx), mounted by Game and read off the view and `legal`
// alone. The view is a fixture shaped as the engine's `marketplace_view` builds it (each lot's index,
// face, price and copies left, the engine's `reason` on a lot it refuses, the viewer's priced tray
// Resources); the shop must send exactly the `marketBuy` or `marketSell` `legal` lists, and nothing
// for a lot or a sale it does not (CLAUDE.md rule 7).

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { ActionBody, MarketplaceView, PlayerView } from "@jackioh/shared";

import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import { testid } from "./contract.ts";
import Game from "./Game.tsx";
import { baseView, card, emptySide } from "../test/fixtures.ts";

afterEach(cleanup);

const lookup = lookupFromDefs(CATALOG);

const BUY_0: ActionBody = { type: "marketBuy", offer: 0 };
const SELL_R9: ActionBody = { type: "marketSell", instanceId: "r9" };
const END_TURN: ActionBody = { type: "endTurn" };
const REASON = "Zao Gao costs 60 Credits and you have 45";

/** Mr. Vanilla at 40 Credits, a Radiant Zao Gao at 60 the engine refuses, and the tray's `r9` quoted at 2. */
const MARKET: MarketplaceView = {
  rerollsOn: 15,
  offers: [
    { offer: 0, defId: "core-008", price: 40, left: 2 },
    { offer: 1, defId: "core-080", radiant: true, price: 60, left: 1, reason: REASON },
  ],
  sales: [{ instanceId: "r9", defId: "fx-res-rare", credits: 2 }],
};

/** p1's main phase with 45 Credits and a Resource in the tray, at `market` (absent: still closed). */
function marketView(market?: MarketplaceView): PlayerView {
  return baseView({
    you: emptySide("p1", { resourceTray: [card({ instanceId: "r9", defId: "fx-res-rare" })], marketCredits: 45 }),
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

/** Opens the shop with its toggle. */
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
    // Closed until the toggle opens it; it says the Credits and the next re-roll the view gives.
    expect(screen.queryByTestId(testid.marketplacePanel)).toBeNull();
    const panel = openShop();
    expect(panel).toHaveTextContent("45 Credits");
    expect(panel).toHaveTextContent("New stock on turn 15");
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
    // The toggle closes it again.
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
    // A Radiant lot is drawn on the face it is sold on.
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

  it("R1853003 a priced tray Resource's Sell sends its marketSell", () => {
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
