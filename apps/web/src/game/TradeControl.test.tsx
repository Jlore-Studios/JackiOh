// The Trade control (R1838000, TradeControl.tsx): the Duos pass control's bar over a Tradable hand
// card, read off the view and `legal` alone. Pointing at a hand card `legal` lists a `trade` for, or
// whose view carries a `tradeReason`, puts the bar on it with the price and the Trades left the view
// gives; it sends the listed `trade` and nothing else (CLAUDE.md rule 7).

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { ActionBody, PlayerView } from "@jackioh/shared";

import { testid } from "./contract.ts";
import Game from "./Game.tsx";
import { offersTrade, tradeOfferFor } from "./TradeControl.tsx";
import { baseView, card, emptySide } from "../test/fixtures.ts";

const TRADE_H1: ActionBody = { type: "trade", instanceId: "h1" };
const END_TURN: ActionBody = { type: "endTurn" };
const REASON = "you have made this turn's 1 Trade";

/** p1's main phase holding Prestige (`h1`) and a Stockpile (`h2`); `left` Trades left this turn. */
function holding(over: { left?: number; reason?: string; trades?: boolean } = {}): PlayerView {
  const { left = 1, reason, trades = true } = over;
  return baseView({
    you: emptySide("p1", {
      hand: [
        card({ instanceId: "h1", defId: "meditative-053", cost: 1, ...(reason === undefined ? {} : { tradeReason: reason }) }),
        card({ instanceId: "h2", defId: "core-005", cost: 1 }),
      ],
    }),
    ...(trades ? { trades: { cost: 0, left } } : {}),
  });
}

function renderGame(view: PlayerView, legal: readonly ActionBody[]) {
  const onAction = vi.fn<(body: ActionBody) => void>();
  render(<Game view={view} legal={legal} onAction={onAction} />);
  return onAction;
}

/** Point at a hand card as a mouse does, the bar's own trigger (cardBar.ts). */
function pointAt(instanceId: string): void {
  fireEvent.pointerOver(screen.getByTestId(testid.handCard(instanceId)), { pointerType: "mouse" });
  act(() => {
    vi.advanceTimersByTime(0);
  });
}

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe("R1838000 the Trade offer is the view's and legal's", () => {
  it("R1838000 lists the hand cards legal lets trade, and the reasons the view gives the others", () => {
    const offer = tradeOfferFor(holding(), [TRADE_H1, END_TURN]);
    expect(offer).not.toBeNull();
    expect(offer?.cost).toBe(0);
    expect(offer?.left).toBe(1);
    expect(offer === null ? false : offersTrade(offer, "h1")).toBe(true);
    expect(offer === null ? true : offersTrade(offer, "h2")).toBe(false);
    const spent = tradeOfferFor(holding({ left: 0, reason: REASON }), [END_TURN]);
    expect(spent?.legal.size).toBe(0);
    expect(spent?.reasons.get("h1")).toBe(REASON);
    // No Tradable card in hand: the view carries no `trades`, and nothing is offered.
    expect(tradeOfferFor(holding({ trades: false }), [TRADE_H1])).toBeNull();
  });
});

describe("R1838000 the Trade bar on a 1v1 board", () => {
  it("R1838000 on a hand card legal lets trade, the control sends the listed trade", () => {
    const onAction = renderGame(holding(), [TRADE_H1, END_TURN]);
    pointAt("h1");
    expect(screen.getByTestId(testid.tradeBar)).toBeTruthy();
    const control = screen.getByTestId(testid.trade("h1")) as HTMLButtonElement;
    expect(control.disabled).toBe(false);
    expect(control.textContent).toContain("Trade");
    expect(control.textContent).toContain("0 mana · 1 left");
    fireEvent.click(control);
    expect(onAction).toHaveBeenCalledWith(TRADE_H1);
  });

  it("R1838000 a card the engine refuses is greyed with its reason and sends nothing", () => {
    const onAction = renderGame(holding({ left: 0, reason: REASON }), [END_TURN]);
    pointAt("h1");
    const control = screen.getByTestId(testid.trade("h1")) as HTMLButtonElement;
    expect(control.disabled).toBe(true);
    expect(control.getAttribute("data-legal")).toBe("false");
    expect(control.title).toBe(REASON);
    fireEvent.click(control);
    expect(onAction).not.toHaveBeenCalledWith(expect.objectContaining({ type: "trade" }));
  });

  it("R1838000 a hand card with no Trade on offer gets no bar", () => {
    renderGame(holding(), [TRADE_H1, END_TURN]);
    pointAt("h2");
    expect(screen.queryByTestId(testid.tradeBar)).toBeNull();
    expect(screen.queryByTestId(testid.trade("h2"))).toBeNull();
  });
});
