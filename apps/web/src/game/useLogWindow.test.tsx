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
});
