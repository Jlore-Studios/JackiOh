// A pool card's face is drawn the first time the browser un-skips its holder (PoolGrid's PoolFace):
// a skipped face holds nothing, since every style read inside a skipped face lays the page out.
// jsdom has no `content-visibility`, so the event's handler is stubbed in before PoolGrid loads.

import type { CatalogSnapshot } from "@jackioh/validator";
import { act, cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { must } from "../../test/stubServer.ts";

const CATALOG_HERE: CatalogSnapshot = { version: "test", cards: CATALOG };
const CARD = "core-002";
const HANDLER = "oncontentvisibilityautostatechange";

function report(holder: Element, skipped: boolean): void {
  act(() => {
    holder.dispatchEvent(Object.assign(new Event("contentvisibilityautostatechange"), { skipped }));
  });
}

afterEach(() => {
  cleanup();
  Reflect.deleteProperty(HTMLElement.prototype, HANDLER);
  vi.resetModules();
});

describe("PoolGrid's faces", () => {
  it("draws a face once the browser un-skips its holder, and keeps it", async () => {
    Object.defineProperty(HTMLElement.prototype, HANDLER, { value: null, configurable: true, writable: true });
    vi.resetModules();
    const { default: PoolGrid } = await import("./PoolGrid.tsx");
    const { container } = render(<PoolGrid ids={[CARD]} catalog={CATALOG_HERE} onInspect={() => {}} />);
    const holder = must(container.querySelector(".db-card-face"), "the face's holder");

    expect(holder.querySelector(".cf"), "a skipped face holds nothing").toBeNull();
    report(holder, true);
    expect(holder.querySelector(".cf"), "still skipped").toBeNull();
    report(holder, false);
    expect(holder.querySelector(".cf"), "near the screen, it is drawn").not.toBeNull();
    report(holder, true);
    expect(holder.querySelector(".cf"), "skipped again, it stays").not.toBeNull();
  });

  it("draws every face at once where the browser reports no skips", async () => {
    const { default: PoolGrid } = await import("./PoolGrid.tsx");
    const { container } = render(<PoolGrid ids={[CARD]} catalog={CATALOG_HERE} onInspect={() => {}} />);
    expect(container.querySelector(".db-card-face .cf")).not.toBeNull();
  });
});
