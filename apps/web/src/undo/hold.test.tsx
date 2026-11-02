// R2119000: the undo hold and its toast, on a manual clock. Each screen's own tests check what its
// request and its Undo do (the workshop, the creator, Play together).

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactElement } from "react";
import { afterEach, describe, expect, it } from "vitest";

import { manualClock, type ManualClock } from "../game/deckbuilder/testkit.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { setReducedMotion } from "../test/setup.ts";
import { UNDO_WINDOW_MS, useUndoHold, type HeldDeletion, type UndoTimer } from "./hold.ts";
import { UNDO_TOAST, UNDO_TOAST_UNDO, UndoToast } from "./UndoToast.tsx";

/** Two deletions to hold and a "now" that sends the held one at once, each logging what it does. */
function Harness({ timer, log }: { timer: UndoTimer; log: string[] }): ReactElement {
  const { held, hold, undo, sendNow } = useUndoHold(timer);
  const deletion = (name: string): HeldDeletion => ({
    words: () => `${name} deleted`,
    send: (keepalive) => {
      log.push(`send ${name} ${String(keepalive)}`);
    },
    restore: () => {
      log.push(`restore ${name}`);
    },
  });
  return (
    <div>
      <button type="button" onClick={() => hold(deletion("first"))}>
        first
      </button>
      <button type="button" onClick={() => hold(deletion("second"))}>
        second
      </button>
      <button type="button" onClick={sendNow}>
        now
      </button>
      <UndoToast held={held} onUndo={undo} />
    </div>
  );
}

function mount(): { log: string[]; clock: ManualClock; tick: (ms: number) => Promise<void>; unmount: () => void } {
  const log: string[] = [];
  const clock = manualClock();
  const { unmount } = render(<Harness timer={clock} log={log} />);
  return {
    log,
    clock,
    tick: async (ms) => {
      await act(async () => {
        await clock.advance(ms);
      });
    },
    unmount,
  };
}

afterEach(() => {
  cleanup();
  setReducedMotion(false);
  window.localStorage.clear();
  __resetSettingsForTests();
});

describe("R2119000 the undo hold", () => {
  it("R2119000 sends nothing until UNDO_WINDOW_MS has passed, then sends once", async () => {
    const { log, tick } = mount();
    fireEvent.click(screen.getByText("first"));
    expect(screen.getByTestId(UNDO_TOAST)).toHaveTextContent("first deleted · Undo");
    await tick(UNDO_WINDOW_MS - 1);
    expect(log).toEqual([]);
    await tick(1);
    expect(log).toEqual(["send first false"]);
    expect(screen.queryByTestId(UNDO_TOAST)).toBeNull();
    await tick(UNDO_WINDOW_MS * 2);
    expect(log).toEqual(["send first false"]);
  });

  it("R2119000 Undo puts the item back and sends nothing, then or later", async () => {
    const { log, clock, tick } = mount();
    fireEvent.click(screen.getByText("first"));
    fireEvent.click(screen.getByTestId(UNDO_TOAST_UNDO));
    expect(log).toEqual(["restore first"]);
    expect(screen.queryByTestId(UNDO_TOAST)).toBeNull();
    expect(clock.pending(), "the window's timer is gone").toBe(0);
    await tick(UNDO_WINDOW_MS * 2);
    fireEvent.click(screen.getByText("now"));
    expect(log).toEqual(["restore first"]);
  });

  it("R2119000 a second deletion sends the first at once and takes the toast", async () => {
    const { log, tick } = mount();
    fireEvent.click(screen.getByText("first"));
    await tick(UNDO_WINDOW_MS / 2);
    fireEvent.click(screen.getByText("second"));
    expect(log).toEqual(["send first false"]);
    expect(screen.getAllByTestId(UNDO_TOAST)).toHaveLength(1);
    expect(screen.getByTestId(UNDO_TOAST)).toHaveTextContent("second deleted · Undo");
    await tick(UNDO_WINDOW_MS - 1);
    expect(log, "the second has a window of its own").toEqual(["send first false"]);
    await tick(1);
    expect(log).toEqual(["send first false", "send second false"]);
  });

  it("R2119000 pagehide, or the page going hidden, sends it at once with keepalive", async () => {
    const { log, tick } = mount();
    fireEvent.click(screen.getByText("first"));
    act(() => {
      window.dispatchEvent(new Event("pagehide"));
    });
    expect(log).toEqual(["send first true"]);
    expect(screen.queryByTestId(UNDO_TOAST)).toBeNull();

    fireEvent.click(screen.getByText("second"));
    const visibility = Object.getOwnPropertyDescriptor(document, "visibilityState");
    Object.defineProperty(document, "visibilityState", { configurable: true, get: () => "hidden" });
    try {
      act(() => {
        document.dispatchEvent(new Event("visibilitychange"));
      });
    } finally {
      if (visibility === undefined) Reflect.deleteProperty(document, "visibilityState");
      else Object.defineProperty(document, "visibilityState", visibility);
    }
    expect(log).toEqual(["send first true", "send second true"]);
    await tick(UNDO_WINDOW_MS * 2);
    expect(log, "nothing is sent twice").toEqual(["send first true", "send second true"]);
  });

  it("R2119000 leaving the screen sends it", () => {
    const { log, clock, unmount } = mount();
    fireEvent.click(screen.getByText("first"));
    unmount();
    expect(log).toEqual(["send first false"]);
    expect(clock.pending()).toBe(0);
  });

  it("R2119000 the toast is a status that takes no focus, and its Undo is reached and pressed by keyboard", async () => {
    const { log } = mount();
    const user = userEvent.setup();
    const first = screen.getByText("first");
    first.focus();
    await user.click(first);
    const region = screen.getByRole("status");
    expect(region).toContainElement(screen.getByTestId(UNDO_TOAST));
    expect(first, "the toast leaves the focus where it was").toHaveFocus();

    screen.getByText("now").focus();
    await user.tab();
    expect(screen.getByTestId(UNDO_TOAST_UNDO)).toHaveFocus();
    await user.keyboard("{Enter}");
    expect(log).toEqual(["restore first"]);
  });

  it("R2119000 under Reduce motion the toast appears without sliding, inside the safe-area insets", () => {
    const first = mount();
    fireEvent.click(screen.getByText("first"));
    expect(screen.getByTestId(UNDO_TOAST)).toHaveAttribute("data-motion", "full");
    first.unmount();

    // The OS preference.
    setReducedMotion(true);
    const second = mount();
    fireEvent.click(screen.getByText("first"));
    expect(screen.getByTestId(UNDO_TOAST)).toHaveAttribute("data-motion", "reduced");
    second.unmount();

    // The settings panel's switch.
    setReducedMotion(false);
    __resetSettingsForTests();
    writeSettings({ reduceMotion: true });
    mount();
    fireEvent.click(screen.getByText("first"));
    expect(screen.getByTestId(UNDO_TOAST)).toHaveAttribute("data-motion", "reduced");

    const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "undo.css"), "utf8").replace(/\s+/g, " ");
    expect(css).toContain('.undo-toast[data-motion="reduced"] { animation: none; }');
    expect(css).toContain("@media (prefers-reduced-motion: reduce) { .undo-toast { animation: none; } }");
    for (const side of ["left", "right", "bottom"]) expect(css).toContain(`env(safe-area-inset-${side})`);
  });
});
