// #4614: the hook that keeps the mounted lines as the log scrolls and resizes. A real layout
// answers nothing here, so the reads of the page are the viewport this file hands
// logWindow.ts's `readLogViewport`, and the rows keep their estimate tall.

import { cleanup, fireEvent, render } from "@testing-library/react";
import { useRef, type ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useLogWindow } from "./useLogWindow.ts";

/** What the page says of the log's layout: null where it has none, as jsdom has none. */
const layout = vi.hoisted(() => ({
  viewport: null as { scrollTop: number; clientHeight: number } | null,
}));

vi.mock("./logWindow.ts", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./logWindow.ts")>()),
  readLogViewport: () => layout.viewport,
}));

/** 100 lines; the hook's window decides which draw. */
const KEYS = Array.from({ length: 100 }, (_, index) => `p1-${String(index)}`);
/** The phone popover's 400 px show as many estimate-tall rows as the overscan doubles. */
const VIEWPORT = { scrollTop: 0, clientHeight: 400 };

function harness(keys: readonly string[], held: string | null): ReactElement {
  return <Harness keys={keys} held={held} />;
}

function Harness({ keys, held }: { keys: readonly string[]; held: string | null }): ReactElement {
  const ref = useRef<HTMLOListElement>(null);
  const mounted = useLogWindow(ref, keys, held);
  const shown =
    mounted === null || mounted === "all" ? keys : keys.slice(mounted.first, mounted.last + 1);
  return (
    <ol ref={ref} data-testid="log">
      {shown.map((key) => (
        <li key={key} data-log-key={key} />
      ))}
    </ol>
  );
}

function mountedKeys(container: HTMLElement): string[] {
  return [...container.querySelectorAll("li[data-log-key]")].map((li) => li.getAttribute("data-log-key") ?? "");
}

beforeEach(() => {
  layout.viewport = null;
});

afterEach(() => {
  cleanup();
});

describe("#4614 the hook that keeps the mounted lines", () => {
  it("mounts every line where the page has no layout to read", () => {
    const { container } = render(harness(KEYS, null));
    expect(mountedKeys(container)).toEqual(KEYS);
  });

  it("mounts the rows near the view, and moves the window on scroll", () => {
    layout.viewport = { ...VIEWPORT };
    const { container, getByTestId } = render(harness(KEYS, null));
    const first = mountedKeys(container);
    expect(first.length).toBeLessThan(KEYS.length);
    expect(first[0]).toBe(KEYS[0]);
    expect(mountedKeys(container)).not.toContain(KEYS[KEYS.length - 1]);

    layout.viewport = { scrollTop: 1200, clientHeight: 400 };
    fireEvent.scroll(getByTestId("log"));
    const moved = mountedKeys(container);
    expect(moved.length).toBeLessThan(KEYS.length);
    expect(moved[0]).not.toBe(KEYS[0]);
    expect(mountedKeys(container)).not.toContain(KEYS[0]);
    expect(moved).toContain(KEYS[50]);
  });

  it("keeps the held line mounted past the window, and drops it after blur", () => {
    layout.viewport = { scrollTop: 1200, clientHeight: 400 };
    const { container, rerender } = render(harness(KEYS, KEYS[0] ?? null));
    expect(mountedKeys(container)).toContain(KEYS[0]);

    rerender(harness(KEYS, null));
    expect(mountedKeys(container)).not.toContain(KEYS[0]);
  });

  it("#4614 settles when measured heights jitter across passes, instead of cascading", () => {
    layout.viewport = { scrollTop: 0, clientHeight: 400 };
    // Subpixel layout dust: every measure reads a different uniform height, as
    // fractional scroll offsets and rasterization shift rects between passes in a
    // real browser. A layout effect that re-measures on every window it places
    // cascades into React's maximum update depth under sustained arrivals (the e2e
    // smoke crash on this change's first pass).
    let reads = 0;
    const rect = vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(function (this: Element) {
      reads += 1;
      return { height: Math.floor(reads / 40) % 2 === 0 ? 25 : 23 } as DOMRect;
    });
    let renders = 0;
    const counting = (keys: readonly string[], held: string | null): ReactElement => {
      renders += 1;
      return harness(keys, held);
    };
    try {
      const { container, getByTestId } = render(counting(KEYS, null));
      renders = 0;
      for (let top = 0; top <= 2000; top += 400) {
        layout.viewport = { scrollTop: top, clientHeight: 400 };
        fireEvent.scroll(getByTestId("log"));
      }
      // Settled, not cascaded: a bounded render count and a bounded window.
      expect(renders).toBeLessThan(60);
      const mounted = mountedKeys(container);
      expect(mounted.length).toBeLessThan(KEYS.length);
      expect(mounted.length).toBeGreaterThan(0);
    } finally {
      rect.mockRestore();
    }
  });
});
