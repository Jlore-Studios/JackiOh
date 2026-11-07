// R3220000: the one dialog (Dialog.tsx). An alertdialog's Cancel and a dialog's ✕ close it, the row
// runs from the safe choice to the act, and Escape, Back (R2127000) and the scrim close it while Tab
// stays inside and the focus goes back to the opener. The board's five confirms draw through it.
// R3221000: the menus' callers keep their testids and focus, a sheet marks its scrim, and the keys go to
// the top layer alone.

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { useRef, useState, type ReactElement } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { Dialog, DialogClose, dialogTestid, type DialogAction } from "./Dialog.tsx";
import { CATALOG } from "@jackioh/cards";

import { CardDetail, closeInspect } from "../cards/inspect/index.ts";
import { useModalOverlay } from "../cards/inspect/store.ts";
import { INSPECT_DETAIL } from "../cards/inspect/testids.ts";
import { pressBack } from "../test/history.ts";
import { readSheet } from "../test/setup.ts";
import { at } from "../test/stubServer.ts";

afterEach(cleanup);

function press(key: string, options: { shiftKey?: boolean } = {}): void {
  fireEvent.keyDown(document.activeElement ?? document.body, { key, ...options });
}

function action(testid: string, kind: DialogAction["kind"], onClick = vi.fn()): DialogAction {
  return { label: testid, kind, onClick, testid };
}

function rowOf(dialog: HTMLElement): HTMLElement[] {
  const row = dialog.querySelector<HTMLElement>(".dialog__actions");
  if (row === null) throw new Error("no row");
  return [...row.querySelectorAll<HTMLElement>("button")];
}

function Alert({ onClose = vi.fn() }: { onClose?: () => void }): ReactElement {
  return (
    <Dialog
      role="alertdialog"
      testid="alert"
      title="Leave it?"
      description="It ends here."
      onClose={onClose}
      cancel={{ label: "Stay", kind: "primary", testid: "stay" }}
      actions={[action("leave", "danger")]}
    />
  );
}

/** A dialog opened by a button, closed through its own state, as a caller mounts one. */
function Opened({ moveFocus = false }: { moveFocus?: boolean }): ReactElement {
  const [open, setOpen] = useState(false);
  const elsewhere = useRef<HTMLButtonElement>(null);
  return (
    <>
      <button type="button" data-testid="opener" onClick={() => setOpen(true)}>
        Open
      </button>
      <button type="button" data-testid="elsewhere" ref={elsewhere}>
        Elsewhere
      </button>
      {open ? (
        <Dialog
          role="dialog"
          testid="opened"
          title="Opened"
          onClose={() => {
            setOpen(false);
            if (moveFocus) elsewhere.current?.focus();
          }}
          actions={[action("one", "primary"), action("two", "secondary")]}
        />
      ) : null}
    </>
  );
}

function open(): HTMLElement {
  const opener = screen.getByTestId("opener");
  opener.focus();
  fireEvent.click(opener);
  return opener;
}

describe("R3220000 the dialog convention", () => {
  it("R3220000 an alertdialog is modal, named by its h2 title, described by its line, with no ✕", () => {
    render(<Alert />);
    const dialog = screen.getByTestId("alert");
    expect(dialog).toHaveAttribute("role", "alertdialog");
    expect(dialog).toHaveAttribute("aria-modal", "true");
    expect(dialog).toHaveAccessibleName("Leave it?");
    expect(dialog).toHaveAccessibleDescription("It ends here.");
    expect(within(dialog).getByRole("heading", { name: "Leave it?" }).tagName).toBe("H2");
    expect(screen.queryByTestId(dialogTestid.close)).toBeNull();
    expect(within(dialog).getAllByRole("button").map((button) => button.dataset.testid)).toEqual(["stay", "leave"]);
  });

  it("R3220000 a dialog's ✕ is named Close, sits last, and closes it", () => {
    const onClose = vi.fn();
    render(<Dialog role="dialog" testid="plain" title="Plain" onClose={onClose} actions={[action("go", "primary")]} />);
    const dialog = screen.getByTestId("plain");
    expect(dialog).toHaveAttribute("role", "dialog");
    expect(dialog).not.toHaveAttribute("aria-describedby");
    const close = within(dialog).getByRole("button", { name: "Close" });
    expect(close).toHaveAttribute("data-testid", dialogTestid.close);
    expect(dialog.lastElementChild).toBe(close);
    fireEvent.click(close);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("R3220000 the row runs from the safe choice to the act", () => {
    const onCancel = vi.fn();
    render(
      <Dialog
        role="alertdialog"
        testid="three"
        title="Three"
        onClose={onCancel}
        cancel={{ label: "Keep", kind: "primary", testid: "keep" }}
        actions={[action("gone", "danger"), action("save", "secondary")]}
      />,
    );
    const row = rowOf(screen.getByTestId("three"));
    expect(row.map((button) => button.dataset.testid)).toEqual(["keep", "save", "gone"]);
    expect(row.map((button) => button.className)).toEqual(["button-primary", "button-secondary", "button-danger"]);
    expect(row[0]?.parentElement).toHaveAttribute("data-count", "3");
    expect(row[0]).toHaveFocus();
    fireEvent.click(screen.getByTestId("keep"));
    expect(onCancel).toHaveBeenCalledTimes(1);
    cleanup();

    render(<Dialog role="dialog" testid="two" title="Two" onClose={vi.fn()} actions={[action("first", "primary"), action("second", "secondary")]} />);
    const kept = rowOf(screen.getByTestId("two"));
    expect(kept.map((button) => button.dataset.testid)).toEqual(["first", "second"]);
    expect(kept[0]?.parentElement).toHaveAttribute("data-count", "2");
    expect(kept[0]).toHaveFocus();
  });

  it("R3220000 Escape closes it wherever the focus is", () => {
    const onClose = vi.fn();
    render(<Alert onClose={onClose} />);
    press("Escape");
    fireEvent.keyDown(window, { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(2);
  });

  it("R3220000 a click on the scrim closes it, one inside does not", () => {
    const onClose = vi.fn();
    render(<Alert onClose={onClose} />);
    const dialog = screen.getByTestId("alert");
    const scrim = dialog.parentElement;
    if (scrim === null) throw new Error("no scrim");
    expect(scrim).toHaveClass("dialog-scrim");
    fireEvent.click(dialog);
    fireEvent.click(within(dialog).getByText("It ends here."));
    expect(onClose).not.toHaveBeenCalled();
    fireEvent.click(scrim);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("R3220000 Tab stays inside and the focus goes back to the opener, unless the caller moved it", () => {
    render(<Opened />);
    const opener = open();
    const [one, two] = rowOf(screen.getByTestId("opened"));
    const close = screen.getByTestId(dialogTestid.close);
    expect(one).toHaveFocus();
    press("Tab");
    expect(two).toHaveFocus();
    press("Tab");
    expect(close).toHaveFocus();
    press("Tab");
    expect(one).toHaveFocus();
    press("Tab", { shiftKey: true });
    expect(close).toHaveFocus();
    // A click on the page can leave the focus on the body; Tab still lands inside.
    for (const [shiftKey, lands] of [
      [false, one],
      [true, close],
    ] as const) {
      (document.activeElement as HTMLElement | null)?.blur();
      fireEvent.keyDown(document.body, { key: "Tab", shiftKey });
      expect(lands).toHaveFocus();
    }
    act(() => {
      press("Escape");
    });
    expect(screen.queryByTestId("opened")).toBeNull();
    expect(opener).toHaveFocus();
    cleanup();

    render(<Opened moveFocus />);
    open();
    fireEvent.click(screen.getByTestId(dialogTestid.close));
    expect(screen.queryByTestId("opened")).toBeNull();
    expect(screen.getByTestId("elsewhere"), "the caller's move stands").toHaveFocus();
  });

  it("R3220000 Back closes it and keeps the address", async () => {
    at("/decks");
    window.history.pushState(null, "", "/practice");
    render(<Opened />);
    const opener = open();
    expect(screen.getByTestId("opened")).toBeInTheDocument();
    await pressBack();
    expect(screen.queryByTestId("opened")).toBeNull();
    expect(opener).toHaveFocus();
    expect(window.location.pathname).toBe("/practice");
  });

  it("R3220000 one scrim at --z-dialog, and the five keep no Escape or scrim of their own", () => {
    const sheet = readSheet("src/game/dialog.css").replace(/\/\*[\s\S]*?\*\//g, " ");
    const scrim = /\.dialog-scrim\s*\{([^}]*)\}/.exec(sheet)?.[1] ?? "";
    expect(scrim).toMatch(/z-index:\s*var\(--z-dialog\);/);
    expect(scrim).toMatch(/background:\s*color-mix\(in srgb, var\(--pal-wood-950\) 70%, transparent\);/);
    expect(readSheet("src/surfaces.css")).toMatch(/^\s*--z-dialog: 72;$/m);
    for (const source of [
      "src/game/ConfirmConcede.tsx",
      "src/game/ConfirmLeaveMatch.tsx",
      "src/practice/PracticeLeave.tsx",
      "src/dungeon/fight/ConfirmLeaveFight.tsx",
      "src/practice/PracticeResult.tsx",
    ]) {
      const text = readSheet(source);
      expect(text, source).toContain("<Dialog");
      expect(text, source).not.toMatch(/"Escape"/);
      expect(text, source).not.toMatch(/[\w-]+-scrim/);
      expect(text, source).not.toMatch(/\buse(?:BackLayer|ModalKeys|Effect|LayoutEffect)\b|\.focus\(/);
    }
    expect(readSheet("src/game/notices.css")).not.toMatch(/concede-(?:scrim|dialog)/);
    expect(readSheet("src/practice/practice.css")).not.toMatch(/practice-leave|practice-result-scrim/);
  });
});

/** A dialog with a button that opens a second one over it, as the item preview opens its Looks. */
function Nested({ onOuter = vi.fn(), onInner = vi.fn() }: { onOuter?: () => void; onInner?: () => void }): ReactElement {
  const [inner, setInner] = useState(false);
  return (
    <Dialog role="dialog" testid="outer" title="Outer" onClose={onOuter} actions={[]}>
      <button type="button" data-testid="open-inner" onClick={() => setInner(true)}>
        More
      </button>
      {inner ? (
        <Dialog
          role="alertdialog"
          testid="inner"
          title="Inner"
          onClose={() => {
            onInner();
            setInner(false);
          }}
          cancel={{ label: "Back", kind: "secondary", testid: "inner-cancel" }}
          actions={[action("inner-go", "primary")]}
        />
      ) : null}
    </Dialog>
  );
}

/** The inspect overlay's hook (CardDetail.tsx), opened over a dialog as the card view's Card text opens it. */
function Overlay({ onClose }: { onClose: () => void }): ReactElement {
  const close = useRef<HTMLButtonElement>(null);
  const modal = useModalOverlay(onClose, close);
  return (
    <div role="dialog" aria-modal="true" aria-label="Card" data-testid="overlay">
      <DialogClose buttonRef={close} testid="overlay-close" {...modal.dismissProps} />
    </div>
  );
}

describe("R3221000 the menus' dialogs on the one Dialog", () => {
  it("R3221000 a caller keeps its testids, id and focus", () => {
    function Caller(): ReactElement {
      const input = useRef<HTMLInputElement>(null);
      return (
        <Dialog
          role="dialog"
          id="panel-id"
          testid="caller"
          scrimTestid="caller-scrim"
          closeTestid="caller-close"
          closeLabel="Close the caller"
          focus={input}
          title={<span data-testid="named">Named</span>}
          onClose={vi.fn()}
          actions={[action("go", "primary")]}
        >
          <input ref={input} data-testid="caller-input" aria-label="Words" />
        </Dialog>
      );
    }
    render(<Caller />);
    const dialog = screen.getByTestId("caller");
    expect(dialog).toHaveAttribute("id", "panel-id");
    expect(dialog.parentElement).toBe(screen.getByTestId("caller-scrim"));
    expect(dialog).toHaveAccessibleName("Named");
    expect(within(dialog).getByTestId("named").closest("h2")).toHaveClass("dialog__title");
    const close = screen.getByTestId("caller-close");
    expect(close).toHaveAccessibleName("Close the caller");
    expect(close.textContent).toBe("✕");
    expect(screen.queryByTestId(dialogTestid.close)).toBeNull();
    expect(screen.getByTestId("caller-input"), "the caller's focus, not the first answer").toHaveFocus();
  });

  it("R3221000 an answer waits while disabled or busy", () => {
    const onSend = vi.fn();
    const onClose = vi.fn();
    const { rerender } = render(
      <Dialog
        role="alertdialog"
        testid="wait"
        title="Wait"
        onClose={onClose}
        cancel={{ label: "Cancel", kind: "secondary", testid: "wait-cancel", disabled: true }}
        actions={[{ label: "Send", kind: "primary", onClick: onSend, testid: "wait-send", disabled: true, busy: true }]}
      />,
    );
    const send = screen.getByTestId("wait-send");
    expect(send).toBeDisabled();
    expect(send).toHaveAttribute("aria-busy", "true");
    expect(screen.getByTestId("wait-cancel")).toBeDisabled();
    fireEvent.click(send);
    fireEvent.click(screen.getByTestId("wait-cancel"));
    expect(onSend).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
    rerender(
      <Dialog
        role="alertdialog"
        testid="wait"
        title="Wait"
        onClose={onClose}
        cancel={{ label: "Cancel", kind: "secondary", testid: "wait-cancel" }}
        actions={[{ label: "Send", kind: "primary", onClick: onSend, testid: "wait-send" }]}
      />,
    );
    expect(send).toBeEnabled();
    expect(send).not.toHaveAttribute("aria-busy");
    fireEvent.click(send);
    expect(onSend).toHaveBeenCalledTimes(1);
  });

  it("R3221000 a sheet marks its scrim, which sits at the foot of an upright phone and down a side one's edge", () => {
    render(<Dialog role="dialog" sheet testid="sheet" title="Sheet" onClose={vi.fn()} actions={[]} />);
    expect(screen.getByTestId("sheet").parentElement).toHaveAttribute("data-sheet", "");
    cleanup();
    render(<Dialog role="dialog" testid="plain" title="Plain" onClose={vi.fn()} actions={[]} />);
    expect(screen.getByTestId("plain").parentElement).not.toHaveAttribute("data-sheet");
    const sheet = readSheet("src/game/dialog.css").replace(/\/\*[\s\S]*?\*\//g, " ");
    const upright = /@media \(max-width: 600px\) \{([\s\S]*?)\n\}/.exec(sheet)?.[1] ?? "";
    expect(upright).toMatch(/\.dialog-scrim\[data-sheet\] > \.dialog \{[^}]*margin: auto 0 0;/);
    const side = /@media \(orientation: landscape\) and \(max-height: 500px\) \{([\s\S]*?)\n\}/.exec(sheet)?.[1] ?? "";
    expect(side).toMatch(/\.dialog-scrim\[data-sheet\] > \.dialog \{[^}]*height: 100%;[^}]*margin: 0 0 0 auto;/);
  });

  it("R3221000 a scrim under a tavern shell covers the window, not the shell's capped column", () => {
    // tavern.css last, as main.tsx loads it after the sheets the components import: the worst order.
    const styles = ["src/game/dialog.css", "src/auth/tavern.css"].map((path) => {
      const style = document.createElement("style");
      style.textContent = readSheet(path).replace(/^@import [^;]+;$/gm, "");
      document.head.append(style);
      return style;
    });
    try {
      for (const shell of ["app-shell tavern", "tavern app-shell dev-layer", "app-shell app-shell--wide tavern one-screen"]) {
        const { unmount } = render(
          <div className={shell}>
            <Dialog role="dialog" testid="capped" title="Capped" onClose={vi.fn()} actions={[]} />
          </div>,
        );
        const scrim = getComputedStyle(screen.getByTestId("capped").parentElement as HTMLElement);
        expect(scrim.getPropertyValue("max-width"), shell).toBe("none");
        expect(scrim.getPropertyValue("width"), shell).toBe("auto");
        expect([scrim.getPropertyValue("margin-left"), scrim.getPropertyValue("margin-right")], shell).toEqual(["0px", "0px"]);
        unmount();
      }
    } finally {
      for (const style of styles) style.remove();
    }
  });

  it("R3221000 Escape and Tab belong to the top dialog, so a dialog over another closes first", async () => {
    const onOuter = vi.fn();
    const onInner = vi.fn();
    render(<Nested onOuter={onOuter} onInner={onInner} />);
    fireEvent.click(screen.getByTestId("open-inner"));
    const inner = screen.getByTestId("inner");
    const cancel = screen.getByTestId("inner-cancel");
    const go = screen.getByTestId("inner-go");
    expect(cancel).toHaveFocus();
    press("Tab");
    expect(go).toHaveFocus();
    press("Tab");
    expect(cancel, "Tab wraps inside the inner dialog").toHaveFocus();
    press("Tab", { shiftKey: true });
    expect(inner.contains(document.activeElement)).toBe(true);
    act(() => {
      press("Escape");
    });
    expect(onInner).toHaveBeenCalledTimes(1);
    expect(onOuter).not.toHaveBeenCalled();
    expect(screen.queryByTestId("inner")).toBeNull();
    expect(screen.getByTestId("outer")).toBeInTheDocument();
    await act(async () => {
      await Promise.resolve();
    });
    press("Escape");
    expect(onOuter).toHaveBeenCalledTimes(1);
  });

  it("R3221000 Back closes the top dialog alone", async () => {
    at("/collection");
    const onOuter = vi.fn();
    render(<Nested onOuter={onOuter} />);
    fireEvent.click(screen.getByTestId("open-inner"));
    await pressBack();
    expect(screen.queryByTestId("inner")).toBeNull();
    expect(onOuter).not.toHaveBeenCalled();
    expect(screen.getByTestId("outer")).toBeInTheDocument();
  });

  it("R3221000 the inspect overlay opened over a dialog takes Escape and Tab first", () => {
    function Over(): ReactElement {
      const [overlay, setOverlay] = useState(false);
      return (
        <Dialog role="dialog" testid="under" title="Under" onClose={onUnder} actions={[]}>
          <button type="button" data-testid="card-text" onClick={() => setOverlay(true)}>
            Card text
          </button>
          {overlay ? <Overlay onClose={() => setOverlay(false)} /> : null}
        </Dialog>
      );
    }
    const onUnder = vi.fn();
    render(<Over />);
    const opener = screen.getByTestId("card-text");
    opener.focus();
    fireEvent.click(opener);
    const close = screen.getByTestId("overlay-close");
    expect(close).toHaveFocus();
    expect(close).toHaveAccessibleName("Close");
    press("Tab");
    expect(close, "Tab stays in the overlay").toHaveFocus();
    act(() => {
      press("Escape");
    });
    expect(screen.queryByTestId("overlay")).toBeNull();
    expect(onUnder).not.toHaveBeenCalled();
    expect(opener, "the focus goes back to Card text").toHaveFocus();
  });

  it("R3221000 a dialog that opens over a card's detail draws over it and takes its keys", () => {
    // A result that ends the game while a card's detail is open: the board's dialogs are not portalled,
    // so at an equal layer the detail, portalled after them, would draw over the dialog that holds the keys.
    const found = CATALOG["core-043"];
    if (found === undefined) throw new Error("expected core-043 in the catalog");
    const def = found;
    const onResult = vi.fn();
    function Board(): ReactElement {
      const [result, setResult] = useState(false);
      const [detail, setDetail] = useState(true);
      return (
        <>
          <button type="button" data-testid="game-over" onClick={() => setResult(true)}>
            Game over
          </button>
          {detail ? <CardDetail def={def} onClose={() => setDetail(false)} /> : null}
          {result ? (
            <Dialog
              role="dialog"
              testid="result"
              title="Victory"
              onClose={() => {
                onResult();
                setResult(false);
              }}
              actions={[action("view", "primary")]}
            />
          ) : null}
        </>
      );
    }
    const styles = ["src/surfaces.css", "src/cards/inspect/inspect.css"].map((path) => {
      const style = document.createElement("style");
      style.textContent = readSheet(path).replace(/^@import [^;]+;$/gm, "");
      document.head.append(style);
      return style;
    });
    try {
      render(<Board />);
      const layer = screen.getByTestId(INSPECT_DETAIL).closest<HTMLElement>(".inspect-layer");
      if (layer === null) throw new Error("no inspect layer");
      expect(layer, "alone, the detail is the top layer").not.toHaveAttribute("data-under");
      expect(getComputedStyle(layer).getPropertyValue("z-index")).toBe("var(--z-dialog)");
      fireEvent.click(screen.getByTestId("game-over"));
      expect(screen.getByTestId("view")).toHaveFocus();
      expect(layer, "under the result").toHaveAttribute("data-under", "");
      expect(getComputedStyle(layer).getPropertyValue("z-index")).toBe("var(--z-toast)");
      const tokens = readSheet("src/surfaces.css");
      const layerOf = (name: string): number => Number(new RegExp(`--z-${name}:\\s*(\\d+);`).exec(tokens)?.[1]);
      expect(layerOf("toast"), "under every dialog").toBeLessThan(layerOf("dialog"));
      act(() => {
        press("Escape");
      });
      expect(onResult, "Escape answers the result the player sees").toHaveBeenCalledTimes(1);
      expect(screen.getByTestId(INSPECT_DETAIL), "and leaves the detail under it").toBeInTheDocument();
      expect(layer, "the detail is on top again").not.toHaveAttribute("data-under");
    } finally {
      act(() => {
        closeInspect();
      });
      for (const style of styles) style.remove();
    }
  });

  it("R3221000 Escape over an open tooltip leaves the dialog open", () => {
    const onClose = vi.fn();
    render(
      <Dialog role="dialog" testid="tip" title="Tip" onClose={onClose} actions={[]}>
        <span data-tip-open="true">Rush</span>
      </Dialog>,
    );
    press("Escape");
    expect(onClose).not.toHaveBeenCalled();
    cleanup();
    render(
      <Dialog role="dialog" testid="tip" title="Tip" onClose={onClose} actions={[]}>
        <span data-tip-open="false">Rush</span>
      </Dialog>,
    );
    press("Escape");
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("R3221000 a press that began in the panel and ends over the scrim keeps it open", () => {
    const onClose = vi.fn();
    render(
      <Dialog role="dialog" testid="chat" title="Chat" onClose={onClose} actions={[]}>
        <p>A line to select</p>
        <input aria-label="Draft" />
      </Dialog>,
    );
    const dialog = screen.getByTestId("chat");
    const scrim = dialog.parentElement;
    if (scrim === null) throw new Error("no scrim");
    // A selection dragged out of the text or the input: the browser clicks the common ancestor.
    for (const start of [within(dialog).getByText("A line to select"), within(dialog).getByLabelText("Draft")]) {
      fireEvent.pointerDown(start);
      fireEvent.click(scrim);
      expect(onClose).not.toHaveBeenCalled();
    }
    // A press that begins and ends on the scrim still closes it, as does a click with no press before it.
    fireEvent.pointerDown(scrim);
    fireEvent.click(scrim);
    expect(onClose).toHaveBeenCalledTimes(1);
    fireEvent.pointerDown(dialog);
    fireEvent.click(dialog);
    fireEvent.click(scrim);
    expect(onClose).toHaveBeenCalledTimes(2);
  });

  it("R3221000 Tab skips a hidden panel and a control out of the order", () => {
    render(
      <Dialog role="dialog" testid="tabs" title="Tabs" onClose={vi.fn()} actions={[action("shown", "primary")]}>
        <button type="button" tabIndex={-1} data-testid="roving">
          Other tab
        </button>
        <div hidden>
          <button type="button" data-testid="hidden">
            Hidden
          </button>
        </div>
        <textarea aria-label="Note" data-testid="note" />
      </Dialog>,
    );
    const shown = screen.getByTestId("shown");
    const note = screen.getByTestId("note");
    const close = screen.getByTestId(dialogTestid.close);
    shown.focus();
    press("Tab", { shiftKey: true });
    expect(note).toHaveFocus();
    press("Tab", { shiftKey: true });
    expect(close).toHaveFocus();
    press("Tab", { shiftKey: true });
    expect(shown).toHaveFocus();
  });
});
