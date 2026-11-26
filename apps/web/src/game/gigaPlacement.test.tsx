// #4995: a GIGA card's placement highlight lights all four zones it will take, not only its
// anchor. Driven through `<Game/>` as a player drives it; every action expected is a body
// `legal` lists (CLAUDE.md rule 7). The scaffold (fake layout, render, click-to-select) is
// copied from `PromptLane.test.tsx`; the press/move/release drag scaffold from
// `src/test/ux/drag-layer.test.tsx`.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { CATALOG } from "@jackioh/cards";
import type { ActionBody, PlayerId, PlayerView, Row } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { __resetSettingsForTests } from "../settings/index.ts";
import { baseView, card, emptySide } from "../test/fixtures.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import { DRAG_THRESHOLD_PX } from "./drag/model.ts";
import Game from "./Game.tsx";
import { gigaOblongTestid } from "./GigaPlacement.tsx";
import { LANE_PICK_OUTSET_PX } from "./LanePick.tsx";

/** The stylesheet's text, read off the file as `duos-ux.test.tsx` reads its own. */
function gigaCss(): string {
  return readFileSync(join(dirname(fileURLToPath(import.meta.url)), "gigaPlacement.css"), "utf8");
}

const lookup = lookupFromDefs(CATALOG);

const el = (id: string): HTMLElement => screen.getByTestId(id);

const playAt = (row: Row, lane: number): ActionBody => ({ type: "play", instanceId: "g1", zone: { row, lane } });

/** A GIGA unit in hand at cost 3, on an empty board. `viewer` p2 is the same board from the second seat. */
function gigaView(over: { viewer?: PlayerId } = {}): PlayerView {
  const me = over.viewer ?? "p1";
  const them: PlayerId = me === "p1" ? "p2" : "p1";
  return baseView({
    viewer: me,
    active: me,
    you: emptySide(me, {
      hand: [card({ instanceId: "g1", defId: "core-008", cost: 3, keywords: [{ kind: "GIGA" }] })],
    }),
    opponent: emptySide(them, { hand: { count: 3 } }),
  });
}

function renderGame(legal: readonly ActionBody[], view: PlayerView) {
  const onAction = vi.fn();
  const utils = render(
    <CatalogContext.Provider value={lookup}>
      <Game view={view} legal={legal} onAction={onAction} />
    </CatalogContext.Provider>,
  );
  return { ...utils, onAction };
}

// The rows of the board, top to bottom, as Board.tsx stacks them.
const ROWS = ["opponent-backrow", "opponent-units", "you-units", "you-backrow"];
const ZONE_WIDTH = 80;
const ZONE_HEIGHT = 50;
const LANE_PITCH = 100;
const ROW_PITCH = 60;

function rect(left: number, top: number, width: number, height: number): DOMRect {
  return { left, top, width, height, right: left + width, bottom: top + height, x: left, y: top, toJSON: () => ({}) };
}

/** jsdom has no layout: lane n's zones stand at x = 100 n, each row 60 below the last. */
function fakeLayout(): void {
  vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(function (this: Element) {
    const match = /^zone-(you|opponent)-(units|backrow)-(\d+)$/.exec(this.getAttribute("data-testid") ?? "");
    if (match === null) return rect(0, 0, 0, 0);
    const row = ROWS.indexOf(`${match[1] ?? ""}-${match[2] ?? ""}`);
    return rect(LANE_PITCH * Number(match[3]), ROW_PITCH * row, ZONE_WIDTH, ZONE_HEIGHT);
  });
}

/**
 * The oblong's box over the 2×2 block at lane `lane` on your side, pushed out by the oblong's
 * outset. Every footprint spans your units row to your backrow and lanes L to L+1, whichever row
 * the anchor stands in.
 */
function blockBox(lane: number) {
  const out = LANE_PICK_OUTSET_PX;
  return {
    left: `${LANE_PITCH * lane - out}px`,
    top: `${ROW_PITCH * 2 - out}px`,
    width: `${ZONE_WIDTH + LANE_PITCH + 2 * out}px`,
    height: `${ZONE_HEIGHT + ROW_PITCH + 2 * out}px`,
  };
}

const focus = (id: string): void => {
  act(() => {
    el(id).focus();
  });
};

// The drag scaffold, copied from `src/test/ux/drag-layer.test.tsx`: jsdom's
// `elementsFromPoint` is stubbed; presses target elements and moves/releases target `window`.
const POINTER = 1;
const START = { x: 200, y: 400 };

let under: Element[] = [];

function over(...stack: Element[]): void {
  under = stack;
}

function press(target: Element): void {
  fireEvent.pointerDown(target, { pointerId: POINTER, button: 0, clientX: START.x, clientY: START.y });
}

function move(x: number, y: number): void {
  fireEvent.pointerMove(window, { pointerId: POINTER, clientX: x, clientY: y });
}

function release(x = START.x, y = START.y): void {
  fireEvent.pointerUp(window, { pointerId: POINTER, button: 0, clientX: x, clientY: y });
}

beforeEach(() => {
  __resetSettingsForTests();
  fakeLayout();
  under = [];
  document.elementsFromPoint = (() => under) as Document["elementsFromPoint"];
});

afterEach(() => {
  cleanup();
  delete (document as Partial<Document>).elementsFromPoint;
  document.documentElement.removeAttribute("data-dragging");
  localStorage.clear();
  vi.restoreAllMocks();
  __resetSettingsForTests();
});

describe("#4995 a GIGA card's placement highlight lights all four zones it will take", () => {
  it("#4995 selecting a GIGA unit lights all four zones of each offered placement, p1 and p2", () => {
    for (const viewer of ["p1", "p2"] as const) {
      const legal = [playAt("units", 1), playAt("units", 3)];
      renderGame(legal, gigaView({ viewer }));
      fireEvent.click(el("hand-card-g1"));

      for (const lane of [1, 3]) {
        const style = el(gigaOblongTestid("units", lane)).style;
        expect({ left: style.left, top: style.top, width: style.width, height: style.height }).toEqual(blockBox(lane));
      }

      // The anchors keep their green placement glow; the covered zones stay dark and unclickable.
      expect(el("zone-you-units-1")).toHaveAttribute("data-glow", "ready");
      expect(el("zone-you-units-3")).toHaveAttribute("data-glow", "ready");
      for (const id of ["zone-you-units-2", "zone-you-units-4", "zone-you-backrow-1", "zone-you-backrow-2"]) {
        expect(el(id)).not.toHaveAttribute("data-glow");
        expect(el(id)).toHaveAttribute("data-legal", "false");
      }
      cleanup();
    }
  });

  it("#4995 a GIGA field spell lights its backrow footprint", () => {
    // Two anchors: a single fully-determined play sends on the click (`settle`), so only a
    // build still picking its zone keeps the selection the oblongs read.
    const legal = [playAt("backrow", 1), playAt("backrow", 2)];
    renderGame(legal, gigaView());
    fireEvent.click(el("hand-card-g1"));

    const style = el(gigaOblongTestid("backrow", 1)).style;
    expect({ left: style.left, top: style.top, width: style.width, height: style.height }).toEqual(blockBox(1));
    expect(el("zone-you-backrow-1")).toHaveAttribute("data-glow", "ready");
    expect(el("zone-you-backrow-2")).toHaveAttribute("data-glow", "ready");
    for (const id of ["zone-you-units-1", "zone-you-units-2", "zone-you-units-3", "zone-you-backrow-3"]) {
      expect(el(id)).not.toHaveAttribute("data-glow");
    }
  });

  it("#4995 two adjacent anchors stay readable: hover and focus isolate one block", () => {
    renderGame([playAt("units", 1), playAt("units", 2)], gigaView());
    fireEvent.click(el("hand-card-g1"));
    expect(el(gigaOblongTestid("units", 1))).toBeInTheDocument();
    expect(el(gigaOblongTestid("units", 2))).toBeInTheDocument();
    expect(el(gigaOblongTestid("units", 1))).not.toHaveAttribute("data-active");

    fireEvent.pointerOver(el("zone-you-units-2"));
    expect(el(gigaOblongTestid("units", 2))).toHaveAttribute("data-active", "true");
    expect(el(gigaOblongTestid("units", 1))).toHaveAttribute("data-dim", "true");
    expect(el(gigaOblongTestid("units", 2))).not.toHaveAttribute("data-dim");

    // The pointer is elsewhere now, so the keyboard focus isolates the other block instead.
    fireEvent.pointerOver(el("zone-you-units-5"));
    focus("zone-you-units-1");
    expect(el(gigaOblongTestid("units", 1))).toHaveAttribute("data-active", "true");
    expect(el(gigaOblongTestid("units", 2))).toHaveAttribute("data-dim", "true");
  });

  it("#4995 a non-GIGA card's placement glow is unchanged", () => {
    const view = baseView({
      you: emptySide("p1", { hand: [card({ instanceId: "h1", defId: "core-008", cost: 3 })] }),
      opponent: emptySide("p2", { hand: { count: 3 } }),
    });
    const legal: ActionBody[] = [
      { type: "play", instanceId: "h1", zone: { row: "units", lane: 3 } },
      { type: "play", instanceId: "h1", zone: { row: "units", lane: 4 } },
    ];
    renderGame(legal, view);
    fireEvent.click(el("hand-card-h1"));

    expect(screen.queryByTestId("giga-placement")).toBeNull();
    expect(document.querySelector(".giga-placement-oblong")).toBeNull();
    expect(el("zone-you-units-3")).toHaveAttribute("data-glow", "ready");
    expect(el("zone-you-units-4")).toHaveAttribute("data-glow", "ready");
  });

  it("#4995 the anchor takes the click and the oblong takes none", () => {
    const legal = [playAt("units", 1), playAt("units", 3)];
    const { onAction } = renderGame(legal, gigaView());
    fireEvent.click(el("hand-card-g1"));

    const oblong = el(gigaOblongTestid("units", 1));
    fireEvent.click(el("zone-you-units-1"));
    expect(onAction).toHaveBeenCalledTimes(1);
    expect(onAction).toHaveBeenCalledWith(legal[0]);

    expect(oblong).not.toHaveAttribute("tabindex");
    expect(oblong.getAttribute("role")).not.toBe("button");
    expect(gigaCss()).toContain("pointer-events: none");
  });

  it("#4995 dragging a GIGA card over an anchor pads its whole 2x2 block", () => {
    const legal = [playAt("units", 1), playAt("units", 3)];
    const { onAction } = renderGame(legal, gigaView());

    over(el("hand-card-g1"));
    press(el("hand-card-g1"));
    move(START.x, START.y - DRAG_THRESHOLD_PX);
    expect(el("drag-layer")).toHaveAttribute("data-kind", "play");

    over(el("zone-you-units-1"));
    move(120, 150);
    const reticle = el("drag-reticle");
    expect(reticle).toHaveAttribute("data-target", "zone-you-units-1");
    expect(reticle).toHaveAttribute("data-shape", "pad");
    // The four-zone union, with no outset: lanes 1-2 (x 100-280), your units to backrow (y 120-230).
    expect(reticle.style.width).toBe("180px");
    expect(reticle.style.height).toBe("110px");

    release(120, 150);
    expect(onAction).toHaveBeenCalledTimes(1);
    expect(onAction).toHaveBeenCalledWith(legal[0]);
  });

  it("#4995 a printed GIGA keyword counts with no live override", () => {
    // S+ #106 Spectre prints GIGA on both faces; the hand card carries no live `keywords` field.
    const view = baseView({
      you: emptySide("p1", {
        hand: [card({ instanceId: "g1", defId: "stinkyscapeplus-106", radiant: false, cost: 12 })],
      }),
      opponent: emptySide("p2", { hand: { count: 3 } }),
    });
    const hand = view.you.hand;
    expect(Array.isArray(hand) && hand[0] !== undefined && !("keywords" in hand[0])).toBe(true);

    // Two anchors, as above: a single fully-determined play would send on the click.
    renderGame([playAt("units", 1), playAt("units", 3)], view);
    fireEvent.click(el("hand-card-g1"));
    const style = el(gigaOblongTestid("units", 1)).style;
    expect({ left: style.left, top: style.top, width: style.width, height: style.height }).toEqual(blockBox(1));
  });
});
