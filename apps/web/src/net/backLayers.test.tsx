// R2127000: Back closes the topmost open sheet or dialog before it leaves the screen (`backLayers.ts`); where
// the browser has `CloseWatcher` the top layer holds one instead. jsdom traverses the history in queued
// tasks, so every Back here waits on real timers (`test/history.ts`).

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { useState, type ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { CardListSheet } from "../cards/inspect/CardList.tsx";
import { InspectSheet } from "../cards/inspect/InspectSheet.tsx";
import { INSPECT_LIST_SHEET, INSPECT_SHEET } from "../cards/inspect/testids.ts";
import { faceModel } from "../cards/model.ts";
import { PracticeLeave } from "../practice/PracticeLeave.tsx";
import { practiceTestid } from "../practice/testids.ts";
import { withCatalog } from "../test/board.tsx";
import { pressBack, pressForward } from "../test/history.ts";
import { at, must } from "../test/stubServer.ts";
import { useBackLayer } from "./backLayers.ts";
import { guardLeaving, navigate, usePathname } from "./navigate.ts";

function Overlay({ name, onClose, closable = true }: { name: string; onClose: () => void; closable?: boolean }): ReactElement {
  useBackLayer(onClose, closable);
  return <div data-testid={`overlay-${name}`} />;
}

function Screen(): ReactElement {
  const path = usePathname();
  const [open, setOpen] = useState<readonly string[]>([]);
  return (
    <>
      <p data-testid="screen">{path}</p>
      <button
        type="button"
        onClick={() => {
          setOpen((names) => [...names, `o${String(names.length)}`]);
        }}
      >
        Open
      </button>
      <button
        type="button"
        onClick={() => {
          setOpen((names) => names.slice(0, -1));
        }}
      >
        Close
      </button>
      {open.map((name) => (
        <Overlay
          key={name}
          name={name}
          onClose={() => {
            setOpen((names) => names.filter((other) => other !== name));
          }}
        />
      ))}
    </>
  );
}

function press(name: "Open" | "Close"): void {
  fireEvent.click(screen.getByRole("button", { name }));
}

function shown(): string[] {
  return screen.queryAllByTestId(/^overlay-/).map((element) => element.dataset.testid ?? "");
}

/** What a close request uses of the browser's `CloseWatcher`; it is spent once its close has fired. */
class FakeWatcher {
  static made: FakeWatcher[] = [];
  onclose: ((event: Event) => void) | null = null;
  destroyed = false;
  constructor() {
    FakeWatcher.made.push(this);
  }
  destroy(): void {
    this.destroyed = true;
  }
}

function liveWatchers(): FakeWatcher[] {
  return FakeWatcher.made.filter((watcher) => !watcher.destroyed);
}

/** Android's back gesture: the browser fires the watcher's close, and the watcher is done. */
function closeRequest(watcher: FakeWatcher): void {
  act(() => {
    watcher.onclose?.(new Event("close"));
  });
  watcher.destroyed = true;
}

function face(defId: string) {
  return faceModel({ defId, def: must(CATALOG[defId], defId), radiant: false });
}

/** A focused opener and the sheet it opens, closed through the sheet's own `onClose`. */
function SheetHost({ sheet }: { sheet: (close: () => void) => ReactElement }): ReactElement {
  const [open, setOpen] = useState(false);
  return withCatalog(
    <>
      <button
        type="button"
        onClick={() => {
          setOpen(true);
        }}
      >
        Open
      </button>
      {open
        ? sheet(() => {
            setOpen(false);
          })
        : null}
    </>,
  );
}

beforeEach(() => {
  at("/decks");
  window.history.pushState(null, "", "/play");
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  FakeWatcher.made = [];
});

describe("R2127000 Back closes the topmost sheet or dialog before it leaves the screen", () => {
  it("R2127000 Back closes the open overlay and leaves the address and the screen as they were", async () => {
    render(<Screen />);
    press("Open");
    expect(shown()).toEqual(["overlay-o0"]);

    await pressBack();
    expect(shown()).toEqual([]);
    expect(window.location.pathname).toBe("/play");
    expect(screen.getByTestId("screen")).toHaveTextContent("/play");
  });

  it("R2127000 two stacked overlays take two Backs, topmost first", async () => {
    render(<Screen />);
    press("Open");
    press("Open");
    expect(shown()).toEqual(["overlay-o0", "overlay-o1"]);

    await pressBack();
    expect(shown()).toEqual(["overlay-o0"]);
    expect(window.location.pathname).toBe("/play");
    await pressBack();
    expect(shown()).toEqual([]);
    expect(window.location.pathname).toBe("/play");
    await pressBack();
    expect(window.location.pathname).toBe("/decks");
    expect(screen.getByTestId("screen")).toHaveTextContent("/decks");
  });

  it("R2127000 the fallback leaves history.length unchanged across repeated open and close cycles", async () => {
    render(<Screen />);
    const before = window.history.length;
    press("Open");
    press("Close");
    expect(window.history.length, "the first overlay adds its one entry").toBe(before + 1);
    for (let cycle = 0; cycle < 3; cycle += 1) {
      press("Open");
      press("Close");
    }
    for (let cycle = 0; cycle < 3; cycle += 1) {
      press("Open");
      await pressBack();
      expect(shown()).toEqual([]);
    }
    expect(window.history.length).toBe(before + 1);
    expect(window.location.pathname).toBe("/play");
  });

  it("R2127000 Forward after a Back that closed an overlay reopens nothing", async () => {
    render(<Screen />);
    press("Open");
    await pressBack();
    expect(shown()).toEqual([]);

    await pressForward();
    expect(shown()).toEqual([]);
    expect(window.location.pathname).toBe("/play");
    expect(screen.getByTestId("screen")).toHaveTextContent("/play");
    await pressBack();
    expect(window.location.pathname).toBe("/decks");
  });

  it("R2127000 after an overlay closed another way, one Back still leaves the screen", async () => {
    render(<Screen />);
    press("Open");
    press("Close");

    await pressBack();
    expect(window.location.pathname).toBe("/decks");
    expect(screen.getByTestId("screen")).toHaveTextContent("/decks");
  });

  it("R2127000 a navigate off an overlay's entry takes its place", async () => {
    render(<Screen />);
    press("Open");
    press("Close");
    const length = window.history.length;

    act(() => {
      navigate("/collection");
    });
    expect(window.history.length).toBe(length);
    expect(screen.getByTestId("screen")).toHaveTextContent("/collection");
    await pressBack();
    expect(window.location.pathname).toBe("/play");
    expect(screen.getByTestId("screen")).toHaveTextContent("/play");
  });

  it("R2127000 a replacing navigate off an overlay's entry lets Back skip the address it replaced, and Forward finds the page it made", async () => {
    render(<Screen />);
    press("Open");
    press("Close");

    act(() => {
      navigate("/collection", { replace: true });
    });
    await pressBack();
    expect(window.location.pathname).toBe("/decks");
    expect(screen.getByTestId("screen")).toHaveTextContent("/decks");
    await pressForward();
    expect(window.location.pathname).toBe("/collection");
    expect(screen.getByTestId("screen")).toHaveTextContent("/collection");
  });

  it("R2127000 a move after a replacing one comes Back to the page the replacing move made", async () => {
    render(<Screen />);
    press("Open");
    press("Close");
    act(() => {
      navigate("/collection", { replace: true });
    });
    act(() => {
      navigate("/shop");
    });

    await pressBack();
    expect(window.location.pathname).toBe("/collection");
    expect(screen.getByTestId("screen")).toHaveTextContent("/collection");
    await pressBack();
    expect(window.location.pathname).toBe("/decks");
    expect(screen.getByTestId("screen")).toHaveTextContent("/decks");
  });

  it("R2127000 closing an overlay is never a leave, and a Back with none open still meets the leave guard", async () => {
    render(<Screen />);
    const held: (() => void)[] = [];
    const release = guardLeaving((leave) => {
      held.push(leave);
    });
    try {
      press("Open");
      await pressBack();
      expect(shown()).toEqual([]);
      expect(held).toHaveLength(0);
      expect(window.location.pathname).toBe("/play");

      press("Open");
      press("Close");
      await pressBack();
      expect(held).toHaveLength(1);
      expect(window.location.pathname).toBe("/play");
      expect(screen.getByTestId("screen")).toHaveTextContent("/play");
    } finally {
      release();
    }
  });

  it("R2127000 Back closes nothing while the top layer cannot close, and answers again once it can", async () => {
    const onClose = vi.fn();
    const { rerender } = render(<Overlay name="busy" onClose={onClose} closable={false} />);
    const length = window.history.length;

    await pressBack();
    expect(onClose).not.toHaveBeenCalled();
    expect(shown()).toEqual(["overlay-busy"]);
    expect(window.location.pathname).toBe("/play");
    expect(window.history.length).toBe(length);

    rerender(<Overlay name="busy" onClose={onClose} closable />);
    await pressBack();
    expect(onClose).toHaveBeenCalledTimes(1);
    expect(window.location.pathname).toBe("/play");
  });

  it("R2127000 the CloseWatcher path is used where the browser has it", () => {
    vi.stubGlobal("CloseWatcher", FakeWatcher);
    render(<Screen />);
    const length = window.history.length;
    press("Open");
    press("Open");
    expect(liveWatchers(), "the top layer alone holds a watcher").toHaveLength(1);
    expect(window.history.length, "no history entry").toBe(length);

    const top = must(liveWatchers()[0], "the top layer's watcher");
    closeRequest(top);
    expect(shown()).toEqual(["overlay-o0"]);
    expect(liveWatchers()).toHaveLength(1);
    expect(liveWatchers()[0]).not.toBe(top);

    press("Close");
    expect(shown()).toEqual([]);
    expect(liveWatchers()).toHaveLength(0);
    expect(window.history.length).toBe(length);
  });

  it("R2127000 a close request while the top layer cannot close leaves it on top with a new watcher", () => {
    vi.stubGlobal("CloseWatcher", FakeWatcher);
    const onClose = vi.fn();
    render(<Overlay name="busy" onClose={onClose} closable={false} />);
    closeRequest(must(liveWatchers()[0], "the layer's watcher"));
    expect(onClose).not.toHaveBeenCalled();
    expect(liveWatchers()).toHaveLength(1);
  });

  it("R2127000 while a close watcher is held Escape is left to the layers' own keys, and it is untouched without one", () => {
    render(<Overlay name="plain" onClose={vi.fn()} />);
    expect(fireEvent.keyDown(window, { key: "Escape" }), "no watcher: Escape is the page's").toBe(true);
    cleanup();

    vi.stubGlobal("CloseWatcher", FakeWatcher);
    render(<Overlay name="watched" onClose={vi.fn()} />);
    expect(fireEvent.keyDown(window, { key: "Escape" }), "the watcher's close request is canceled").toBe(false);
    expect(fireEvent.keyDown(window, { key: "Enter" })).toBe(true);
  });

  it("R2127000 Back closes the inspect sheet and the pile sheet, and the focus goes back", async () => {
    render(<SheetHost sheet={(close) => <InspectSheet face={face("core-002")} onClose={close} />} />);
    let opener = screen.getByRole("button", { name: "Open" });
    opener.focus();
    fireEvent.click(opener);
    expect(screen.getByTestId(INSPECT_SHEET)).toBeInTheDocument();
    await pressBack();
    expect(screen.queryByTestId(INSPECT_SHEET)).toBeNull();
    expect(opener).toHaveFocus();
    expect(window.location.pathname).toBe("/play");
    cleanup();

    render(
      <SheetHost
        sheet={(close) => <CardListSheet title="Your graveyard" entries={[{ key: "e1", face: face("core-002") }]} onClose={close} />}
      />,
    );
    opener = screen.getByRole("button", { name: "Open" });
    opener.focus();
    fireEvent.click(opener);
    expect(screen.getByTestId(INSPECT_LIST_SHEET)).toBeInTheDocument();
    await pressBack();
    expect(screen.queryByTestId(INSPECT_LIST_SHEET)).toBeNull();
    expect(opener).toHaveFocus();
    expect(window.location.pathname).toBe("/play");
  });

  it('R2127000 Back on "Leave this game?" means stay', async () => {
    const onStay = vi.fn();
    const onLeave = vi.fn();
    render(<PracticeLeave to="menu" onStay={onStay} onLeave={onLeave} />);
    expect(screen.getByTestId(practiceTestid.leave)).toBeInTheDocument();

    await pressBack();
    expect(onStay).toHaveBeenCalledTimes(1);
    expect(onLeave).not.toHaveBeenCalled();
    expect(window.location.pathname).toBe("/play");
  });
});
