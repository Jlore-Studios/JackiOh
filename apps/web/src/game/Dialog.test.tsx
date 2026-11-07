// R3220000: the one dialog (Dialog.tsx). An alertdialog's Cancel and a dialog's ✕ close it, the row
// runs from the safe choice to the act, and Escape, Back (R2127000) and the scrim close it while Tab
// stays inside and the focus goes back to the opener. The board's five confirms draw through it.

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { useRef, useState, type ReactElement } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { Dialog, dialogTestid, type DialogAction } from "./Dialog.tsx";
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
