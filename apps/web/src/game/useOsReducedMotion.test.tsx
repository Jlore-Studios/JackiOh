// #258: the OS reduced-motion preference is followed live (useOsReducedMotion.ts): the hook re-reads
// the media query on its `change` event, and Game.tsx rebuilds its animation queue when it flips,
// exactly as it does for the panel's own "Reduce motion".

import type { GameEvent } from "@jackioh/shared";
import { act, cleanup, render, renderHook, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ANIMATIONS, REDUCED_MOTION_QUERY } from "./animations.ts";
import Game from "./Game.tsx";
import { useOsReducedMotion } from "./useOsReducedMotion.ts";
import { __resetSettingsForTests } from "../settings/store.ts";
import { baseView, withEvents } from "../test/fixtures.ts";

type Listener = () => void;

/** A `matchMedia` whose reduced-motion answer the test flips, firing `change` as a browser does. */
function controllableQuery(): { flip: (reduce: boolean) => void; listeners: Set<Listener> } {
  let reduce = false;
  const listeners = new Set<Listener>();
  vi.spyOn(window, "matchMedia").mockImplementation(
    (query: string) =>
      ({
        media: query,
        get matches() {
          return reduce && query === REDUCED_MOTION_QUERY;
        },
        onchange: null,
        addListener: () => {},
        removeListener: () => {},
        addEventListener: (_type: string, fn: Listener) => {
          listeners.add(fn);
        },
        removeEventListener: (_type: string, fn: Listener) => {
          listeners.delete(fn);
        },
        dispatchEvent: () => false,
      }) as unknown as MediaQueryList,
  );
  return {
    listeners,
    flip(next: boolean) {
      reduce = next;
      for (const fn of [...listeners]) fn();
    },
  };
}

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.useRealTimers();
  window.localStorage.clear();
  __resetSettingsForTests();
});

describe("#258 the OS reduced-motion setting, live", () => {
  it("the hook follows the query's change event and unsubscribes on unmount", () => {
    const query = controllableQuery();
    const { result, unmount } = renderHook(() => useOsReducedMotion());
    expect(result.current).toBe(false);
    act(() => {
      query.flip(true);
    });
    expect(result.current).toBe(true);
    act(() => {
      query.flip(false);
    });
    expect(result.current).toBe(false);
    unmount();
    expect(query.listeners.size).toBe(0);
  });

  it("turning it on mid-animation rebuilds the queue, which drains at once; turning it off brings motion back", () => {
    const query = controllableQuery();
    const START: GameEvent[] = [{ type: "turnStarted", player: "p1", turn: 3 }];
    const hit: GameEvent = { type: "damage", sourceId: null, targetId: "hero-p1", amount: 3, combat: false };
    const again: GameEvent = { type: "damage", sourceId: null, targetId: "hero-p1", amount: 2, combat: false };
    const { rerender } = render(<Game view={withEvents(baseView(), START)} legal={[]} onAction={vi.fn()} />);
    rerender(<Game view={withEvents(baseView(), [...START, hit])} legal={[]} onAction={vi.fn()} />);
    expect(screen.getByTestId("animation-queue")).toHaveAttribute("data-animating", "damage");

    act(() => {
      query.flip(true);
    });
    expect(screen.queryByTestId("animation-queue"), "reduced motion: the in-flight hit is cut").toBeNull();

    act(() => {
      query.flip(false);
    });
    rerender(<Game view={withEvents(baseView(), [...START, hit, again])} legal={[]} onAction={vi.fn()} />);
    expect(screen.getByTestId("animation-queue")).toHaveAttribute("data-animating", "damage");
    act(() => {
      vi.advanceTimersByTime(ANIMATIONS.damage.durationMs);
    });
    expect(screen.queryByTestId("animation-queue")).toBeNull();
  });
});
