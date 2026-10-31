// Issue #2121 (R2121000): a card or pile sheet closes on a touch swipe down, exactly as Close
// does; a prompt and the mulligan never do. Gestures are pointer events.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { useState, type ReactElement, type ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { CatalogContext, lookupFromDefs } from "../../game/catalog.ts";
import Prompt from "../../game/Prompt.tsx";
import { __resetSettingsForTests, writeSettings } from "../../settings/store.ts";
import { baseView, card, emptySide, pendingFor } from "../../test/fixtures.ts";
import { setReducedMotion } from "../../test/setup.ts";
import { must } from "../../test/stubServer.ts";
import { faceModel } from "../model.ts";
import { CardListSheet } from "./CardList.tsx";
import { SHEET_DISMISS_DISTANCE, SHEET_FLICK_WINDOW_MS } from "./constants.ts";
import { FaceDownSheet } from "./FaceDown.tsx";
import { InspectSheet } from "./InspectSheet.tsx";
import {
  INSPECT_FACE,
  INSPECT_FACE_DOWN,
  INSPECT_LIST_CARD,
  INSPECT_LIST_DETAIL,
  INSPECT_LIST_SHEET,
  INSPECT_SCRIM,
  INSPECT_SHEET,
} from "./testids.ts";

const lookup = lookupFromDefs(CATALOG);

function face(defId: string) {
  return faceModel({ defId, def: must(CATALOG[defId], defId), radiant: false });
}

const FACE_A = "core-002";
const FACE_B = "core-019";

/** An opener button plus the sheet while open, closing through the sheet's own `onClose`. */
function Host({ onClose, children }: { onClose: () => void; children: (close: () => void) => ReactNode }): ReactElement {
  const [open, setOpen] = useState(false);
  const close = (): void => {
    onClose();
    setOpen(false);
  };
  return (
    <CatalogContext.Provider value={lookup}>
      <button type="button" onClick={() => setOpen(true)}>
        Open
      </button>
      {open ? children(close) : null}
    </CatalogContext.Provider>
  );
}

const START_Y = 100;
const FAR = START_Y + 600 * SHEET_DISMISS_DISTANCE + 50;
const SHORT = START_Y + 75;

function touch(y: number, x = 100, pointerType = "touch") {
  return { pointerType, pointerId: 1, isPrimary: true, button: 0, clientX: x, clientY: y };
}

/** jsdom has no layout: every sheet is 600 high before the press. */
function size(sheet: HTMLElement): void {
  Object.defineProperty(sheet, "offsetHeight", { configurable: true, value: 600 });
}

function advance(ms: number): void {
  act(() => {
    vi.advanceTimersByTime(ms);
  });
}

/** Focus the opener, then click it: the sheet's close has a focus to give back. */
function openSheet(): HTMLElement {
  const opener = screen.getByRole("button", { name: "Open" });
  opener.focus();
  fireEvent.click(opener);
  return opener;
}

function press(target: Element, y: number, x = 100, pointerType = "touch"): void {
  fireEvent.pointerDown(target, touch(y, x, pointerType));
}

function drag(target: Element, y: number, x = 100, pointerType = "touch"): void {
  fireEvent.pointerMove(target, touch(y, x, pointerType));
}

function lift(target: Element, y: number, x = 100, pointerType = "touch"): void {
  fireEvent.pointerUp(target, touch(y, x, pointerType));
}

/** A slow drag: hold, move, stop, lift, so only the distance can close the sheet. */
function slowDragTo(target: Element, y: number, x = 100): void {
  advance(1000);
  drag(target, y, x);
  advance(SHEET_FLICK_WINDOW_MS + 1);
  lift(target, y, x);
}

function pileEntries() {
  return [
    { key: "e1", face: face(FACE_A) },
    { key: "e2", face: face(FACE_B) },
  ];
}

function sheetOf(testid: string): HTMLElement {
  const sheet = screen.getByTestId(testid);
  size(sheet);
  return sheet;
}

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  setReducedMotion(false);
  __resetSettingsForTests();
  window.localStorage.clear();
});

describe("R2121000 a sheet closes on a swipe down", () => {
  it("R2121000 a drag past SHEET_DISMISS_DISTANCE closes the sheet and gives focus back", () => {
    const onClose = vi.fn();
    render(
      <Host onClose={onClose}>
        {(close) => <InspectSheet face={face(FACE_A)} onClose={close} />}
      </Host>,
    );
    const opener = openSheet();
    const sheet = sheetOf(INSPECT_SHEET);
    const faceEl = screen.getByTestId(INSPECT_FACE);
    press(faceEl, START_Y);
    slowDragTo(faceEl, FAR);
    expect(screen.queryByTestId(INSPECT_SHEET)).toBeNull();
    expect(onClose).toHaveBeenCalledTimes(1);
    expect(document.activeElement).toBe(opener);
    expect(sheet.isConnected).toBe(false);
    cleanup();

    const onListClose = vi.fn();
    render(
      <Host onClose={onListClose}>
        {(close) => <CardListSheet title="Your graveyard" entries={pileEntries()} onClose={close} />}
      </Host>,
    );
    const listOpener = openSheet();
    sheetOf(INSPECT_LIST_SHEET);
    const tile = screen.getAllByTestId(INSPECT_LIST_CARD)[0];
    if (tile === undefined) throw new Error("no pile tile");
    press(tile, START_Y);
    slowDragTo(tile, FAR);
    expect(screen.queryByTestId(INSPECT_LIST_SHEET)).toBeNull();
    expect(onListClose).toHaveBeenCalledTimes(1);
    expect(document.activeElement).toBe(listOpener);

    openSheet();
    sheetOf(INSPECT_LIST_SHEET);
    const head = document.querySelector<HTMLElement>(".inspect-list-sheet .inspect-list-head");
    if (head === null) throw new Error("no pile header");
    press(head, START_Y);
    slowDragTo(head, FAR);
    expect(screen.queryByTestId(INSPECT_LIST_SHEET)).toBeNull();
    expect(onListClose).toHaveBeenCalledTimes(2);
    cleanup();

    const onFacedownClose = vi.fn();
    render(
      <Host onClose={onFacedownClose}>
        {(close) => <FaceDownSheet cost={2} onClose={close} />}
      </Host>,
    );
    const facedownOpener = openSheet();
    const facedown = sheetOf(INSPECT_FACE_DOWN);
    press(facedown, START_Y);
    slowDragTo(facedown, FAR);
    expect(screen.queryByTestId(INSPECT_FACE_DOWN)).toBeNull();
    expect(onFacedownClose).toHaveBeenCalledTimes(1);
    expect(document.activeElement).toBe(facedownOpener);
  });

  it("R2121000 the sheet follows the finger and the scrim fades; a short slow drag springs back", () => {
    const onClose = vi.fn();
    render(
      <Host onClose={onClose}>
        {(close) => <InspectSheet face={face(FACE_A)} onClose={close} />}
      </Host>,
    );
    openSheet();
    const sheet = sheetOf(INSPECT_SHEET);
    const scrim = screen.getByTestId(INSPECT_SCRIM);
    const faceEl = screen.getByTestId(INSPECT_FACE);
    press(faceEl, START_Y);
    advance(1000);
    drag(faceEl, SHORT);
    expect(sheet.style.getPropertyValue("--sheet-drag")).toBe("75px");
    expect(sheet.dataset.swipe).toBe("dragging");
    expect(scrim.style.opacity).toBe(String(1 - 75 / 600));
    advance(SHEET_FLICK_WINDOW_MS + 1);
    lift(faceEl, SHORT);
    expect(sheet.style.getPropertyValue("--sheet-drag")).toBe("");
    expect(sheet.dataset.swipe).toBeUndefined();
    expect(scrim.style.opacity).toBe("");
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByTestId(INSPECT_SHEET)).toBeInTheDocument();
  });

  it("R2121000 a fast flick closes it", () => {
    const onClose = vi.fn();
    render(
      <Host onClose={onClose}>
        {(close) => <InspectSheet face={face(FACE_A)} onClose={close} />}
      </Host>,
    );
    openSheet();
    sheetOf(INSPECT_SHEET);
    const faceEl = screen.getByTestId(INSPECT_FACE);
    press(faceEl, START_Y);
    advance(10);
    drag(faceEl, START_Y + 60);
    lift(faceEl, START_Y + 60);
    expect(onClose).toHaveBeenCalledTimes(1);
    expect(screen.queryByTestId(INSPECT_SHEET)).toBeNull();
  });

  it("R2121000 up or sideways does nothing", () => {
    const onClose = vi.fn();
    render(
      <Host onClose={onClose}>
        {(close) => <InspectSheet face={face(FACE_A)} onClose={close} />}
      </Host>,
    );
    openSheet();
    const sheet = sheetOf(INSPECT_SHEET);
    const faceEl = screen.getByTestId(INSPECT_FACE);
    press(faceEl, START_Y);
    drag(faceEl, START_Y - 80);
    drag(faceEl, FAR);
    lift(faceEl, FAR);
    expect(sheet.style.getPropertyValue("--sheet-drag")).toBe("");
    expect(onClose).not.toHaveBeenCalled();
    press(faceEl, START_Y);
    drag(faceEl, START_Y + 20, 180);
    lift(faceEl, START_Y + 20, 180);
    expect(sheet.style.getPropertyValue("--sheet-drag")).toBe("");
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByTestId(INSPECT_SHEET)).toBeInTheDocument();
  });

  it("R2121000 while the content is scrolled down a drag scrolls it instead; the grip still drags", () => {
    const onClose = vi.fn();
    render(
      <Host onClose={onClose}>
        {(close) => <InspectSheet face={face(FACE_A)} onClose={close} />}
      </Host>,
    );
    openSheet();
    const sheet = sheetOf(INSPECT_SHEET);
    sheet.scrollTop = 120;
    const faceEl = screen.getByTestId(INSPECT_FACE);
    press(faceEl, START_Y);
    slowDragTo(faceEl, FAR);
    expect(sheet.style.getPropertyValue("--sheet-drag")).toBe("");
    expect(onClose).not.toHaveBeenCalled();
    const scrolled = new Event("touchmove", { bubbles: true, cancelable: true });
    faceEl.dispatchEvent(scrolled);
    expect(scrolled.defaultPrevented).toBe(false);

    const grip = sheet.querySelector<HTMLElement>("[data-sheet-handle]");
    if (grip === null) throw new Error("no sheet grip");
    press(grip, START_Y);
    advance(1000);
    drag(grip, FAR);
    expect(sheet.style.getPropertyValue("--sheet-drag")).toBe("200px");
    const held = new Event("touchmove", { bubbles: true, cancelable: true });
    faceEl.dispatchEvent(held);
    expect(held.defaultPrevented).toBe(true);
    advance(SHEET_FLICK_WINDOW_MS + 1);
    lift(grip, FAR);
    expect(screen.queryByTestId(INSPECT_SHEET)).toBeNull();
    expect(onClose).toHaveBeenCalledTimes(1);
    cleanup();

    const onListClose = vi.fn();
    render(
      <Host onClose={onListClose}>
        {(close) => <CardListSheet title="Your graveyard" entries={pileEntries()} onClose={close} />}
      </Host>,
    );
    openSheet();
    sheetOf(INSPECT_LIST_SHEET);
    const grid = document.querySelector<HTMLElement>(".inspect-list-grid--sheet");
    if (grid === null) throw new Error("no pile grid");
    grid.scrollTop = 120;
    const tile = screen.getAllByTestId(INSPECT_LIST_CARD)[0];
    if (tile === undefined) throw new Error("no pile tile");
    press(tile, START_Y);
    slowDragTo(tile, FAR);
    expect(onListClose).not.toHaveBeenCalled();
    const tileMove = new Event("touchmove", { bubbles: true, cancelable: true });
    tile.dispatchEvent(tileMove);
    expect(tileMove.defaultPrevented).toBe(false);

    const head = document.querySelector<HTMLElement>(".inspect-list-sheet .inspect-list-head");
    if (head === null) throw new Error("no pile header");
    press(head, START_Y);
    advance(1000);
    drag(head, FAR);
    const headMove = new Event("touchmove", { bubbles: true, cancelable: true });
    tile.dispatchEvent(headMove);
    expect(headMove.defaultPrevented).toBe(true);
    advance(SHEET_FLICK_WINDOW_MS + 1);
    lift(head, FAR);
    expect(screen.queryByTestId(INSPECT_LIST_SHEET)).toBeNull();
    expect(onListClose).toHaveBeenCalledTimes(1);
  });

  it("R2121000 a mouse never drags a sheet", () => {
    const onClose = vi.fn();
    render(
      <Host onClose={onClose}>
        {(close) => <InspectSheet face={face(FACE_A)} onClose={close} />}
      </Host>,
    );
    openSheet();
    const sheet = sheetOf(INSPECT_SHEET);
    const faceEl = screen.getByTestId(INSPECT_FACE);
    press(faceEl, START_Y, 100, "mouse");
    advance(1000);
    drag(faceEl, FAR, 100, "mouse");
    expect(sheet.style.getPropertyValue("--sheet-drag")).toBe("");
    advance(SHEET_FLICK_WINDOW_MS + 1);
    lift(faceEl, FAR, 100, "mouse");
    expect(sheet.style.getPropertyValue("--sheet-drag")).toBe("");
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByTestId(INSPECT_SHEET)).toBeInTheDocument();
  });

  it("R2121000 the lift after a drag that springs back clicks nothing", () => {
    render(
      <Host onClose={() => undefined}>
        {(close) => <CardListSheet title="Your graveyard" entries={pileEntries()} onClose={close} />}
      </Host>,
    );
    openSheet();
    sheetOf(INSPECT_LIST_SHEET);
    const tile = screen.getAllByTestId(INSPECT_LIST_CARD)[0];
    if (tile === undefined) throw new Error("no pile tile");
    press(tile, START_Y);
    slowDragTo(tile, SHORT);
    expect(screen.queryByTestId(INSPECT_LIST_SHEET)).toBeInTheDocument();
    expect(screen.queryByTestId(INSPECT_LIST_DETAIL)).toBeNull();
    fireEvent.click(tile, { detail: 1 });
    expect(screen.queryByTestId(INSPECT_LIST_DETAIL)).toBeNull();
    fireEvent.click(tile, { detail: 1 });
    expect(screen.getByTestId(INSPECT_LIST_DETAIL)).toBeInTheDocument();
  });

  it("R2121000 under Reduce motion nothing follows, and a swipe past the threshold closes at once", () => {
    setReducedMotion(true);
    const onClose = vi.fn();
    render(
      <Host onClose={onClose}>
        {(close) => <InspectSheet face={face(FACE_A)} onClose={close} />}
      </Host>,
    );
    openSheet();
    const sheet = sheetOf(INSPECT_SHEET);
    const scrim = screen.getByTestId(INSPECT_SCRIM);
    const faceEl = screen.getByTestId(INSPECT_FACE);
    press(faceEl, START_Y);
    advance(1000);
    drag(faceEl, FAR);
    expect(sheet.style.getPropertyValue("--sheet-drag")).toBe("");
    expect(scrim.style.opacity).toBe("");
    advance(SHEET_FLICK_WINDOW_MS + 1);
    lift(faceEl, FAR);
    expect(onClose).toHaveBeenCalledTimes(1);

    openSheet();
    const again = sheetOf(INSPECT_SHEET);
    const faceAgain = screen.getByTestId(INSPECT_FACE);
    press(faceAgain, START_Y);
    slowDragTo(faceAgain, SHORT);
    expect(onClose).toHaveBeenCalledTimes(1);
    expect(again.style.getPropertyValue("--sheet-drag")).toBe("");
    expect(screen.getByTestId(INSPECT_SHEET)).toBeInTheDocument();
    cleanup();

    setReducedMotion(false);
    writeSettings({ reduceMotion: true });
    const onPanelClose = vi.fn();
    render(
      <Host onClose={onPanelClose}>
        {(close) => <InspectSheet face={face(FACE_A)} onClose={close} />}
      </Host>,
    );
    openSheet();
    const panelSheet = sheetOf(INSPECT_SHEET);
    const panelScrim = screen.getByTestId(INSPECT_SCRIM);
    const panelFace = screen.getByTestId(INSPECT_FACE);
    press(panelFace, START_Y);
    advance(1000);
    drag(panelFace, FAR);
    expect(panelSheet.style.getPropertyValue("--sheet-drag")).toBe("");
    expect(panelScrim.style.opacity).toBe("");
    advance(SHEET_FLICK_WINDOW_MS + 1);
    lift(panelFace, FAR);
    expect(onPanelClose).toHaveBeenCalledTimes(1);

    openSheet();
    sheetOf(INSPECT_SHEET);
    const shortFace = screen.getByTestId(INSPECT_FACE);
    press(shortFace, START_Y);
    slowDragTo(shortFace, SHORT);
    expect(onPanelClose).toHaveBeenCalledTimes(1);
    expect(screen.getByTestId(INSPECT_SHEET)).toBeInTheDocument();
  });

  it("R2121000 a Prompt and the mulligan can't be swiped away", () => {
    const onAction = vi.fn();
    render(
      <Prompt
        view={baseView({
          pending: pendingFor("discover", [
            { key: "mode:core-043", label: "Flood", defId: "core-043" },
            { key: "mode:core-055", label: "Archivist", defId: "core-055" },
            { key: "mode:core-066", label: "Lava Golem", defId: "core-066" },
          ]),
        })}
        onAction={onAction}
      />,
    );
    const modal = screen.getByTestId("prompt-modal");
    press(modal, START_Y);
    slowDragTo(modal, FAR);
    expect(screen.getByTestId("prompt-modal")).toBeInTheDocument();
    expect(onAction).not.toHaveBeenCalled();
    expect(modal.querySelector("[data-sheet-handle]")).toBeNull();
    expect(modal.style.getPropertyValue("--sheet-drag")).toBe("");
    cleanup();

    render(
      <Prompt
        view={baseView({
          you: emptySide("p1", {
            hand: [
              card({ instanceId: "h1", defId: FACE_A }),
              card({ instanceId: "h2", defId: FACE_B }),
            ],
          }),
          pending: pendingFor(
            "mulligan",
            [
              { key: "h1", label: "One", instanceId: "h1" },
              { key: "h2", label: "Two", instanceId: "h2" },
            ],
            { min: 0, max: 2 },
          ),
        })}
        onAction={onAction}
      />,
    );
    const mulligan = screen.getByTestId("prompt-modal");
    expect(mulligan.getAttribute("data-prompt-kind")).toBe("mulligan");
    press(mulligan, START_Y);
    slowDragTo(mulligan, FAR);
    expect(screen.getByTestId("prompt-modal")).toBeInTheDocument();
    expect(onAction).not.toHaveBeenCalled();
    expect(mulligan.querySelector("[data-sheet-handle]")).toBeNull();
    expect(mulligan.style.getPropertyValue("--sheet-drag")).toBe("");
  });

  it("R2121000 the sheets stay inside the safe-area insets and pan only off a handle", () => {
    const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "inspect.css"), "utf8").replace(
      /\/\*[\s\S]*?\*\//g,
      "",
    );
    const sheet = /\n\.inspect-sheet \{([^}]*)\}/.exec(css)?.[1];
    if (sheet === undefined) throw new Error("no top-level .inspect-sheet");
    expect(sheet).toContain("safe-area-inset-top");
    expect(sheet).toContain("--sheet-drag");
    const list = /\n\.inspect-list-sheet \{([^}]*)\}/.exec(css)?.[1];
    if (list === undefined) throw new Error("no top-level .inspect-list-sheet");
    for (const inset of [
      "safe-area-inset-top",
      "safe-area-inset-right",
      "safe-area-inset-bottom",
      "safe-area-inset-left",
    ]) {
      expect(list).toContain(inset);
    }
    expect(list).toContain("--sheet-drag");
    const handle = /\[data-sheet-handle\] \{([^}]*)\}/.exec(css)?.[1];
    if (handle === undefined) throw new Error("no [data-sheet-handle] block");
    expect(handle).toContain("touch-action: none");
  });
});
