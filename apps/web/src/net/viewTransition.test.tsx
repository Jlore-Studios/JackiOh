// R2117000: a move between two menu screens cross-fades in the browser's view transition (`showMove`),
// the render flushed inside it so the browser captures the new screen.

import { act, cleanup, render, screen } from "@testing-library/react";
import { useEffect, type ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { setFxSettings, resetFxSettingsForTests } from "../fx/settings.ts";
import { testid } from "../game/contract.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { setReducedMotion } from "../test/setup.ts";
import { at } from "../test/stubServer.ts";
import { guardLeaving, navigate, paths, usePathname, useSearch } from "./navigate.ts";

const releases: (() => void)[] = [];

/** Registers a guard that keeps the moves it is handed. */
function guard(): (() => void)[] {
  const held: (() => void)[] = [];
  releases.push(
    guardLeaving((leave) => {
      held.push(leave);
    }),
  );
  return held;
}

async function traverse(step: () => void): Promise<void> {
  await act(async () => {
    const popped = new Promise((resolve) => {
      window.addEventListener("popstate", resolve, { once: true });
    });
    step();
    await popped;
  });
}

const drawnInside: string[] = [];
/** What the next transition's `ready` does: resolve, or reject as a transition a later one skipped. */
let ready = (): Promise<void> => Promise.resolve();
const start = vi.fn((update: () => void) => {
  update();
  drawnInside.push(document.querySelector('[data-testid="shown"]')?.textContent ?? "");
  return { ready: ready(), finished: Promise.resolve(), updateCallbackDone: Promise.resolve() };
});
function Screen(): ReactElement {
  const path = usePathname();
  const search = useSearch();
  return <p data-testid="shown">{path + search}</p>;
}

/** A gate as `Gated` in main.tsx is one: drawn on `from`, its effect moves on to `to`. */
function Redirect({ from, to }: { from: string; to: string }): null {
  const path = usePathname();
  useEffect(() => {
    if (path === from) navigate(to, { replace: true });
  }, [path, from, to]);
  return null;
}

beforeEach(() => {
  ready = () => Promise.resolve();
  start.mockClear();
  drawnInside.length = 0;
  Object.defineProperty(document, "startViewTransition", { configurable: true, writable: true, value: start });
  at(paths.decks);
});

afterEach(() => {
  for (const release of releases.splice(0)) release();
  cleanup();
  document.body.replaceChildren();
  Reflect.deleteProperty(document, "startViewTransition");
  setReducedMotion(false);
  __resetSettingsForTests();
  resetFxSettingsForTests();
  window.localStorage.clear();
});

describe("R2117000 a move between two menu screens cross-fades in a view transition", () => {
  it("R2117000 a navigate from one menu screen to another starts one view transition, and the new screen is drawn inside it", () => {
    render(<Screen />);
    act(() => {
      navigate(paths.collection);
    });
    expect(start).toHaveBeenCalledTimes(1);
    expect(drawnInside).toEqual([paths.collection]);
    expect(window.location.pathname).toBe(paths.collection);
  });

  it("R2117000 the browser's Back and Forward between two menu screens each start one view transition", async () => {
    render(<Screen />);
    act(() => {
      navigate(paths.collection);
    });
    start.mockClear();
    drawnInside.length = 0;

    await traverse(() => window.history.back());
    expect(start).toHaveBeenCalledTimes(1);
    expect(drawnInside).toEqual([paths.decks]);

    await traverse(() => window.history.forward());
    expect(start).toHaveBeenCalledTimes(2);
    expect(drawnInside).toEqual([paths.decks, paths.collection]);
  });

  it("R2117000 no move into or out of a board route starts a view transition", async () => {
    at(paths.play);
    render(<Screen />);
    const boards = [
      paths.match("m-1"),
      paths.watch("m-1"),
      paths.duos("m-1"),
      paths.practice,
      paths.hotseat,
      paths.duosDev,
    ];
    for (const board of boards) {
      act(() => {
        navigate(board);
      });
      expect(window.location.pathname).toBe(board);
      expect(screen.getByTestId("shown").textContent).toBe(board);
      act(() => {
        navigate(paths.play);
      });
      expect(window.location.pathname).toBe(paths.play);
      expect(screen.getByTestId("shown").textContent).toBe(paths.play);
    }
    act(() => {
      navigate(paths.match("m-1"));
    });
    await traverse(() => window.history.back());
    expect(window.location.pathname).toBe(paths.play);
    await traverse(() => window.history.forward());
    expect(window.location.pathname).toBe(paths.match("m-1"));
    expect(start).not.toHaveBeenCalled();
  });

  it("R2117000 a move off a board on screen, as a replay playing in place, starts no view transition", () => {
    at(paths.replays);
    render(
      <>
        <Screen />
        <div data-testid={testid.board} />
      </>,
    );
    act(() => {
      navigate(paths.play);
    });
    expect(start).not.toHaveBeenCalled();
    expect(screen.getByTestId("shown").textContent).toBe(paths.play);
  });

  it("R2117000 under Reduce motion, the system's, the settings panel's or the effects setting's, a move starts no view transition and still happens", () => {
    render(<Screen />);

    setReducedMotion(true);
    act(() => {
      navigate(paths.collection);
    });
    expect(screen.getByTestId("shown").textContent).toBe(paths.collection);
    setReducedMotion(false);

    writeSettings({ reduceMotion: true });
    act(() => {
      navigate(paths.decks);
    });
    expect(screen.getByTestId("shown").textContent).toBe(paths.decks);
    writeSettings({ reduceMotion: false });

    setFxSettings({ motion: "reduce" });
    act(() => {
      navigate(paths.collection);
    });
    expect(screen.getByTestId("shown").textContent).toBe(paths.collection);
    setFxSettings({ motion: "system" });

    expect(start).not.toHaveBeenCalled();
  });

  it("R2117000 where the browser has no view transitions a move happens at once", () => {
    render(<Screen />);
    Object.defineProperty(document, "startViewTransition", { configurable: true, writable: true, value: undefined });
    act(() => {
      navigate(paths.collection);
    });
    expect(screen.getByTestId("shown").textContent).toBe(paths.collection);
    expect(start).not.toHaveBeenCalled();
  });

  it("R2117000 a move the leave guard holds starts no view transition until it is let go", async () => {
    at(paths.creator);
    render(<Screen />);
    const held = guard();
    let went: boolean | undefined;
    act(() => {
      went = navigate(paths.decks);
    });
    expect(went).toBe(false);
    expect(start).not.toHaveBeenCalled();
    expect(window.location.pathname).toBe(paths.creator);
    act(() => {
      held[0]?.();
    });
    expect(start).toHaveBeenCalledTimes(1);
    expect(window.location.pathname).toBe(paths.decks);

    for (const release of releases.splice(0)) release();
    act(() => {
      navigate(paths.creator);
    });
    const held2 = guard();
    await traverse(() => window.history.back());
    expect(start).toHaveBeenCalledTimes(2);
    expect(window.location.pathname).toBe(paths.creator);
    await traverse(() => {
      held2[0]?.();
    });
    expect(start).toHaveBeenCalledTimes(3);
    expect(drawnInside[drawnInside.length - 1]).toBe(paths.decks);
  });

  it("R2117000 a gate's redirect made while the new screen is drawn joins its view transition rather than starting a second (spec 14 B37)", () => {
    at(paths.landing);
    render(
      <>
        <Screen />
        <Redirect from={paths.collection} to={paths.login} />
      </>,
    );
    act(() => {
      navigate(paths.collection);
    });
    expect(start).toHaveBeenCalledTimes(1);
    expect(drawnInside).toEqual([paths.login]);
    expect(window.location.pathname).toBe(paths.login);
  });

  it("R2117000 a transition a later one skips rejects its ready, and that is no error", async () => {
    render(<Screen />);
    ready = () => Promise.reject(new DOMException("Transition was skipped", "AbortError"));
    act(() => {
      navigate(paths.collection);
    });
    // An unhandled rejection fails the run; let it surface before the test ends.
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(start).toHaveBeenCalledTimes(1);
    expect(screen.getByTestId("shown").textContent).toBe(paths.collection);
  });

  it("R2117000 a change of the query alone stays on its screen and starts no view transition", () => {
    at(paths.almanac);
    render(<Screen />);
    act(() => {
      navigate(paths.almanacCard("core-001"));
    });
    expect(start).not.toHaveBeenCalled();
    expect(screen.getByTestId("shown").textContent).toBe(`${paths.almanac}?card=core-001`);
  });
});
