// #258: two board cues on the card root (Card.tsx), mounted through the real Board as Card.test.tsx
// does: the "can't act yet" cue on a Unit with no action left (spent.ts, spent.css), and keyboard
// inspect (I, the context-menu key, Shift+F10) on every hand and field card.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { PlayerView, UnitView } from "@jackioh/shared";

import { INSPECT_FACE_DOWN, INSPECT_SHEET, closeInspect } from "../cards/index.ts";
import Board from "./Board.tsx";
import { SPENT_NOTE } from "./Card.tsx";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import { testid, type BoardProps, type ClickTarget } from "./contract.ts";
import { actingSeat, isSpent } from "./spent.ts";
import { fullBoardView, pendingFor } from "../test/fixtures.ts";

afterEach(() => {
  cleanup();
  closeInspect();
  vi.useRealTimers();
});

const lookup = lookupFromDefs(CATALOG);

function renderBoard(view: PlayerView, props: Omit<BoardProps, "view"> = {}): void {
  render(
    <CatalogContext.Provider value={lookup}>
      <Board view={view} {...props} />
    </CatalogContext.Provider>,
  );
}

function units(view: PlayerView, side: "you" | "opponent"): UnitView[] {
  return view[side].units.filter((u): u is UnitView => u !== null);
}

function root(u: UnitView): HTMLElement {
  return screen.getByTestId(testid.card(u.instanceId));
}

/** The fixture with the viewer's lane-1 unit spent too, and every enemy unit unable to act. */
function spentView(over: Partial<PlayerView> = {}): PlayerView {
  const view = fullBoardView(over);
  const mine = view.you.units.map((u, i) => (u !== null && i === 0 ? { ...u, canAct: false } : u));
  const theirs = view.opponent.units.map((u) => (u === null ? u : { ...u, canAct: false }));
  return { ...view, you: { ...view.you, units: mine }, opponent: { ...view.opponent, units: theirs } };
}

describe("#258 the can't-act-yet cue", () => {
  it("only the acting player's units with canAct false wear it, as data-spent and a Zz badge that says it in words", () => {
    const view = spentView();
    renderBoard(view);
    for (const u of units(view, "you")) {
      const el = root(u);
      expect(el.getAttribute("data-can-act")).toBe(u.canAct ? "true" : "false");
      if (u.canAct) {
        expect(el.hasAttribute("data-spent")).toBe(false);
        expect(el.querySelector(".spent-badge")).toBeNull();
      } else {
        expect(el.getAttribute("data-spent")).toBe("true");
        const badge = el.querySelector(".spent-badge");
        expect(badge).not.toBeNull();
        expect(badge?.textContent).toContain("Zz");
        expect(badge?.textContent).toContain(SPENT_NOTE);
        expect(badge?.closest(".cf")).toBeNull();
      }
    }
    // The opponent's units cannot act on your turn; that is not "spent", so nothing is drawn.
    for (const u of units(view, "opponent")) {
      expect(root(u).hasAttribute("data-spent")).toBe(false);
      expect(root(u).querySelector(".spent-badge")).toBeNull();
    }
  });

  it("on the opponent's turn it is their units that wear it, and none of yours", () => {
    const view = spentView({ active: "p2" });
    renderBoard(view);
    for (const u of units(view, "opponent")) expect(root(u).getAttribute("data-spent")).toBe("true");
    for (const u of units(view, "you")) expect(root(u).hasAttribute("data-spent")).toBe(false);
  });

  it("outside the main phase, under an open prompt or after the result, no unit wears it", () => {
    const pending = pendingFor("chooseTarget", []);
    expect(actingSeat({ ...fullBoardView(), pending })).toBeNull();
    expect(actingSeat({ ...fullBoardView(), phase: "mulligan" })).toBeNull();
    expect(actingSeat({ ...fullBoardView(), result: { winner: "p1", reason: "hero-death" } })).toBeNull();
    expect(actingSeat(fullBoardView())).toBe("p1");
    renderBoard(spentView({ pending }));
    expect(document.querySelector("[data-spent]")).toBeNull();
    expect(document.querySelector(".spent-badge")).toBeNull();
  });

  it("is drawn state, never permission: a spent unit the highlight lists is still clickable", () => {
    const view = spentView();
    const spent = units(view, "you").find((u) => !u.canAct);
    if (spent === undefined) throw new Error("the fixture has no spent unit");
    expect(isSpent(spent, "p1")).toBe(true);
    const onClick = vi.fn<(target: ClickTarget) => void>();
    renderBoard(view, { highlight: { legal: new Set([testid.card(spent.instanceId)]), selected: new Set() }, onClick });
    fireEvent.click(root(spent));
    expect(onClick).toHaveBeenCalledTimes(1);
  });
});

describe("#258 keyboard inspect on the board", () => {
  it("I, the context-menu key and Shift+F10 open the inspect sheet on a field unit, legal or not", () => {
    const view = fullBoardView();
    renderBoard(view);
    const enemy = units(view, "opponent")[0];
    if (enemy === undefined) throw new Error("no enemy unit");
    const el = root(enemy);
    // Focusable so a keyboard can reach it, though nothing makes it legal.
    expect(el.getAttribute("tabindex")).toBe("0");
    expect(el.getAttribute("aria-keyshortcuts")).toBe("I");
    for (const key of [{ key: "i" }, { key: "I" }, { key: "ContextMenu" }, { key: "F10", shiftKey: true }]) {
      expect(screen.queryByTestId(INSPECT_SHEET)).toBeNull();
      act(() => {
        fireEvent.keyDown(el, key);
      });
      expect(screen.getByTestId(INSPECT_SHEET), JSON.stringify(key)).toBeInTheDocument();
      act(() => {
        closeInspect();
      });
    }
    // F10 without Shift, and other letters, open nothing.
    fireEvent.keyDown(el, { key: "F10" });
    fireEvent.keyDown(el, { key: "x" });
    expect(screen.queryByTestId(INSPECT_SHEET)).toBeNull();
  });

  it("a hand card opens it by key, and Enter still plays only a legal card", () => {
    const view = fullBoardView();
    const first = view.you.hand[0];
    const second = view.you.hand[1];
    if (first === undefined || second === undefined) throw new Error("no hand cards");
    const onClick = vi.fn<(target: ClickTarget) => void>();
    renderBoard(view, { highlight: { legal: new Set([testid.handCard(first.instanceId)]), selected: new Set() }, onClick });
    const legal = screen.getByTestId(testid.handCard(first.instanceId));
    const illegal = screen.getByTestId(testid.handCard(second.instanceId));
    expect(illegal.getAttribute("tabindex")).toBe("0");
    fireEvent.keyDown(illegal, { key: "Enter" });
    expect(onClick).not.toHaveBeenCalled();
    fireEvent.keyDown(legal, { key: "Enter" });
    expect(onClick).toHaveBeenCalledTimes(1);
    act(() => {
      fireEvent.keyDown(legal, { key: "i" });
    });
    expect(screen.getByTestId(INSPECT_SHEET)).toBeInTheDocument();
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it("a face-down trap in the backrow opens what the view says of it; the opponent's hand of backs takes no focus", () => {
    const view = fullBoardView();
    renderBoard(view);
    const back = document.querySelector<HTMLElement>(".card-facedown");
    if (back === null) throw new Error("no face-down backrow card");
    expect(back.getAttribute("tabindex")).toBe("0");
    act(() => {
      fireEvent.keyDown(back, { key: "i" });
    });
    expect(screen.getByTestId(INSPECT_FACE_DOWN)).toBeInTheDocument();
    const hand = screen.getByTestId("hand-opponent");
    expect(hand.querySelector(".card-back[tabindex]")).toBeNull();
  });
});
