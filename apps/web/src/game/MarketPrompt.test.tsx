// ME-MARKET (Meditative #42 CN Flea Market, R1000–R1002): the night market's stall. The view is a
// fixture shaped as the engine's `viewFor` builds a `market` prompt (lots `mode:<defId>` with their
// price as `cost`, barters `instance:<id>` with their yuan written negative, Leave `none`, the yuan
// left as `budget`); the stall must send exactly `{ type: "answer", choiceId, selection }`.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { PendingOption, PlayerView } from "@jackioh/shared";

import { INSPECT_SHEET, closeInspect } from "../cards/index.ts";
import { LONG_PRESS_MS } from "../cards/inspect/index.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import Prompt from "./Prompt.tsx";
import { baseView, pendingFor, waitingPending } from "../test/fixtures.ts";

afterEach(() => {
  act(() => {
    closeInspect();
  });
  cleanup();
});

const lookup = lookupFromDefs(CATALOG);

const LOT: PendingOption = { key: "mode:core-080", label: "Lot", defId: "core-080", cost: 30 };
const AI_LOT: PendingOption = { key: "mode:classicplus-t-ai-01", label: "AI lot", defId: "classicplus-t-ai-01", cost: 15 };
const BARTER: PendingOption = { key: "instance:h1", label: "Traded", instanceId: "h1", defId: "core-005", cost: -30 };
const LEAVE: PendingOption = { key: "none", label: "Leave" };

function marketView(options: PendingOption[]): PlayerView {
  return baseView({ pending: pendingFor("market", options, { budget: 50 }) });
}

function renderMarket(view: PlayerView) {
  const onAction = vi.fn();
  render(
    <CatalogContext.Provider value={lookup}>
      <Prompt view={view} onAction={onAction} />
    </CatalogContext.Provider>,
  );
  return onAction;
}

function sent(onAction: ReturnType<typeof vi.fn>, selection: unknown[]): void {
  expect(onAction).toHaveBeenCalledTimes(1);
  expect(onAction).toHaveBeenCalledWith({ type: "answer", choiceId: "ch1", selection });
}

describe("R1000 the night market's stall", () => {
  it("R1000 shows the lots with their prices and the purse", () => {
    renderMarket(marketView([LOT, AI_LOT, LEAVE]));
    const modal = screen.getByTestId("prompt-modal");
    expect(modal.getAttribute("data-prompt-kind")).toBe("market");
    expect(screen.getByTestId("market-purse")).toHaveTextContent("¥50");
    expect(screen.getByTestId("prompt-option-mode:core-080")).toHaveTextContent("¥30");
    expect(screen.getByTestId("prompt-option-mode:classicplus-t-ai-01")).toHaveTextContent("¥15");
    expect(screen.getByRole("button", { name: "Lot, 30 yuan" })).toBeInTheDocument();
    // R432: a price is yuan, never a card's "cost N".
    expect(modal.textContent ?? "").not.toMatch(/cost \d/i);
  });

  it("R1000 a lot answers its card, and Leave answers none", () => {
    const onAction = renderMarket(marketView([LOT, LEAVE]));
    fireEvent.click(screen.getByTestId("prompt-option-mode:core-080"));
    sent(onAction, [{ pick: "mode", option: "core-080" }]);
    cleanup();
    const leaving = renderMarket(marketView([LOT, LEAVE]));
    fireEvent.click(screen.getByTestId("prompt-option-none"));
    sent(leaving, [{ pick: "none" }]);
  });

  it("R1001 a barter shows what it adds and answers its instance; with none offered there is no barter section", () => {
    const onAction = renderMarket(marketView([LOT, BARTER, LEAVE]));
    expect(screen.getByTestId("prompt-option-instance:h1")).toHaveTextContent("+¥30");
    expect(screen.getByRole("region", { name: "Barter from your hand" })).toBeInTheDocument();
    fireEvent.click(screen.getByTestId("prompt-option-instance:h1"));
    sent(onAction, [{ pick: "instance", instanceId: "h1" }]);
    cleanup();
    renderMarket(marketView([LOT, LEAVE]));
    expect(screen.queryByRole("region", { name: "Barter from your hand" })).toBeNull();
  });

  it("R1000 with no lot it can buy the stall says so, and Leave stays", () => {
    renderMarket(marketView([BARTER, LEAVE]));
    expect(screen.getByTestId("market-empty")).toHaveTextContent("Nothing on the stall you can buy.");
    expect(screen.getByTestId("prompt-option-none")).toBeInTheDocument();
  });

  it("R1000 the first deal has the focus as it opens, and the arrows walk the deals", () => {
    renderMarket(marketView([LOT, AI_LOT, LEAVE]));
    const first = screen.getByTestId("prompt-option-mode:core-080");
    expect(document.activeElement).toBe(first);
    fireEvent.keyDown(first, { key: "ArrowRight" });
    expect(document.activeElement).toBe(screen.getByTestId("prompt-option-mode:classicplus-t-ai-01"));
    fireEvent.keyDown(document.activeElement ?? first, { key: "ArrowLeft" });
    expect(document.activeElement).toBe(first);
  });

  it("R97 a waiting seat sees no stall", () => {
    renderMarket(baseView({ pending: waitingPending }));
    expect(document.querySelector('[data-prompt-kind="market"]')).toBeNull();
  });
});

// #552 (MN09, mobile): a lot is read before it is bought, as a Discover's card is (Prompt.tsx): a
// long-press opens its card's sheet and buys nothing, and a plain tap still buys it.
describe("#552 a lot opens its card on a long-press, and only a tap buys it", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("#552 a touch long-press on a lot opens the lot's card in the inspect sheet and sends no deal", () => {
    vi.useFakeTimers();
    const onAction = renderMarket(marketView([LOT, LEAVE]));
    const lot = screen.getByTestId("prompt-option-mode:core-080");
    fireEvent.pointerDown(lot, { pointerType: "touch", pointerId: 1, clientX: 10, clientY: 10 });
    act(() => {
      vi.advanceTimersByTime(LONG_PRESS_MS);
    });
    const sheet = screen.getByTestId(INSPECT_SHEET);
    expect(sheet).toHaveTextContent(CATALOG["core-080"]?.name ?? "core-080");
    fireEvent.pointerUp(lot, { pointerType: "touch", pointerId: 1 });
    fireEvent.click(lot);
    expect(onAction).not.toHaveBeenCalled();
  });

  it("#552 a tap on a lot still buys it", () => {
    const onAction = renderMarket(marketView([LOT, LEAVE]));
    fireEvent.click(screen.getByTestId("prompt-option-mode:core-080"));
    sent(onAction, [{ pick: "mode", option: "core-080" }]);
    expect(screen.queryByTestId(INSPECT_SHEET)).toBeNull();
  });
});
