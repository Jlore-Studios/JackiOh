// R2118000: Ctrl+K (Cmd+K) opens a command menu on every menu screen, never over a board:
// the common actions, every menu page and the Almanac shelf's cards by name, each a `navigate`
// move, with the focus moving in, staying inside and going back on close.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { setShips } from "@jackioh/shared";

import { INSPECT_CLOSE, INSPECT_DETAIL, closeInspect } from "../cards/index.ts";
import { CatalogContext, lookupFromDefs } from "../game/catalog.ts";
import Game from "../game/Game.tsx";
import { almanacShelf } from "../game/deckbuilder/shelf.ts";
import { resetLocaleForTests, setLocale } from "../i18n/i18n.ts";
import { guardLeaving, paths } from "../net/navigate.ts";
import { baseView } from "../test/fixtures.ts";
import { at } from "../test/stubServer.ts";
import CommandMenu, { commandEntryTestid, commandMenuTestid } from "./CommandMenu.tsx";
import { commandCards, commandMenuOn } from "./commandEntries.ts";

const { App } = await import("../main.tsx");

/** A lazily-imported route chunk, and a pool of every card, can outrun the 1 s default. */
const SLOW = { timeout: 10_000 } as const;

beforeEach(() => {
  at("/");
  // An account read that never answers: a screen that asked the server would sit on it.
  vi.stubGlobal("fetch", vi.fn(() => new Promise<Response>(() => {})));
});

afterEach(() => {
  cleanup();
  closeInspect();
  vi.unstubAllGlobals();
  resetLocaleForTests();
  at("/");
});

/** Opens the menu from the keyboard and returns its input. */
function openMenu(): HTMLElement {
  fireEvent.keyDown(window, { key: "k", ctrlKey: true });
  return screen.getByTestId(commandMenuTestid.input);
}

/** Types into the menu's input the way a player does. */
function typeQuery(input: HTMLElement, value: string): void {
  fireEvent.change(input, { target: { value } });
}

describe("R2118000 the command menu", () => {
  it("R2118000 Ctrl+K and Meta+K open it on a menu screen, from a text field too, and a plain K never does", () => {
    render(<App />);
    expect(screen.queryByTestId(commandMenuTestid.dialog)).toBeNull();
    fireEvent.keyDown(window, { key: "k" });
    expect(screen.queryByTestId(commandMenuTestid.dialog)).toBeNull();

    const input = openMenu();
    expect(screen.getByTestId(commandMenuTestid.dialog)).toBeInTheDocument();
    expect(input).toHaveFocus();
    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });
    expect(screen.queryByTestId(commandMenuTestid.dialog)).toBeNull();

    fireEvent.keyDown(window, { key: "k", metaKey: true });
    expect(screen.getByTestId(commandMenuTestid.input)).toBeInTheDocument();
    cleanup();

    render(
      <>
        <App />
        <input data-testid="command-menu-field" />
      </>,
    );
    const field = screen.getByTestId("command-menu-field");
    field.focus();
    fireEvent.keyDown(field, { key: "k" });
    expect(screen.queryByTestId(commandMenuTestid.dialog)).toBeNull();
    fireEvent.keyDown(field, { key: "k", ctrlKey: true });
    expect(screen.getByTestId(commandMenuTestid.input)).toBeInTheDocument();
  });

  it("R2118000 is not mounted on a board route", () => {
    for (const path of [paths.match("m1"), paths.series("s1"), paths.watch("m1"), paths.duos("m1"), paths.hotseat, paths.duosDev]) {
      expect(commandMenuOn(path), path).toBe(false);
      render(<CommandMenu path={path} />);
      fireEvent.keyDown(window, { key: "k", ctrlKey: true });
      expect(screen.queryByTestId(commandMenuTestid.dialog), path).toBeNull();
      cleanup();
    }
    for (const path of ["/", "/practice", "/replays", "/almanac", "/nowhere"]) {
      expect(commandMenuOn(path), path).toBe(true);
    }
  });

  it("R2118000 opens nothing while a board is up on a menu path", () => {
    const board = (
      <CatalogContext.Provider value={lookupFromDefs(CATALOG)}>
        <Game view={baseView()} legal={[]} onAction={() => undefined} />
      </CatalogContext.Provider>
    );
    const { rerender } = render(
      <>
        <CommandMenu path="/practice" />
        {board}
      </>,
    );
    fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    expect(screen.queryByTestId(commandMenuTestid.dialog)).toBeNull();

    rerender(<CommandMenu path="/practice" />);
    fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    expect(screen.getByTestId(commandMenuTestid.input)).toBeInTheDocument();
  });

  it("R2118000 opens nothing over another modal dialog", () => {
    render(
      <>
        <CommandMenu path="/" />
        <div role="dialog" aria-modal="true" />
      </>,
    );
    fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    expect(screen.queryByTestId(commandMenuTestid.dialog)).toBeNull();
  });

  it("R2118000 lists the actions and pages, filtered by their words in the reader's language", async () => {
    render(<CommandMenu path="/" />);
    const input = openMenu();
    const listed = (): (string | null)[] => screen.getAllByRole("option").map((node) => node.getAttribute("data-testid"));
    expect(listed()).toEqual(
      [
        "action-new-deck",
        "action-practice",
        "action-play",
        "page-home",
        "page-decks",
        "page-collection",
        "page-creator",
        "page-shop",
        "page-purchases",
        "page-almanac",
        "page-patch-notes",
        "page-stats",
        "page-leaderboard",
        "page-replays",
        "page-together",
        "page-account",
        "page-privacy",
        "page-terms",
        "page-accessibility",
        "page-copyright",
      ].map((id) => commandEntryTestid(id)),
    );

    typeQuery(input, "pat no");
    expect(screen.getByTestId(commandEntryTestid("page-patch-notes"))).toBeInTheDocument();
    expect(screen.queryByTestId(commandEntryTestid("page-decks"))).toBeNull();

    typeQuery(input, "DECK");
    expect(screen.getByTestId(commandEntryTestid("action-new-deck"))).toBeInTheDocument();
    expect(screen.getByTestId(commandEntryTestid("page-decks"))).toBeInTheDocument();
    expect(screen.queryByTestId(commandEntryTestid("page-shop"))).toBeNull();

    typeQuery(input, "zzzzqqq");
    expect(screen.getByTestId(commandMenuTestid.empty)).toBeInTheDocument();
    const stayed = window.location.pathname + window.location.search;
    fireEvent.keyDown(input, { key: "Enter" });
    expect(window.location.pathname + window.location.search).toBe(stayed);
    expect(screen.getByTestId(commandMenuTestid.empty)).toBeInTheDocument();

    await act(async () => {
      await setLocale("zh-CN");
    });
    typeQuery(input, "卡组");
    expect(screen.getByTestId(commandEntryTestid("page-decks"))).toBeInTheDocument();
    typeQuery(input, "decks");
    expect(screen.queryByTestId(commandEntryTestid("page-decks"))).toBeNull();
  });

  it("R2118000 arrow keys move the selection, wrapping, and Enter runs it through navigate()", () => {
    render(<CommandMenu path="/" />);
    const input = openMenu();
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "ArrowDown" });
    expect(screen.getByTestId(commandEntryTestid("action-play"))).toHaveAttribute("aria-selected", "true");
    fireEvent.keyDown(input, { key: "ArrowUp" });
    fireEvent.keyDown(input, { key: "ArrowUp" });
    fireEvent.keyDown(input, { key: "ArrowUp" });
    expect(screen.getByTestId(commandEntryTestid("page-copyright"))).toHaveAttribute("aria-selected", "true");
    fireEvent.keyDown(input, { key: "Enter" });
    expect(window.location.pathname).toBe("/copyright");
    expect(screen.queryByTestId(commandMenuTestid.dialog)).toBeNull();
  });

  it("R2118000 the leave guard still holds a move it makes", () => {
    at("/creator");
    render(<App />);
    const held: (() => void)[] = [];
    const release = guardLeaving((leave) => {
      held.push(leave);
    });
    try {
      const input = openMenu();
      typeQuery(input, "Decks");
      fireEvent.keyDown(input, { key: "Enter" });
      expect(window.location.pathname).toBe("/creator");
      expect(held).toHaveLength(1);
      act(() => {
        held[0]?.();
      });
      expect(window.location.pathname).toBe("/decks");
    } finally {
      release();
    }
  });

  it("R2118000 the card search lists only the shelf, never an unshipped card", () => {
    expect(commandCards(CATALOG).map((entry) => entry.id)).toEqual(almanacShelf(CATALOG).map((id) => `card-${id}`));
    const held = Object.values(CATALOG).find(
      (def) => !setShips(def.set) && Object.values(CATALOG).filter((other) => other.name === def.name).length === 1,
    );
    if (held !== undefined) {
      render(<CommandMenu path="/" />);
      const input = openMenu();
      typeQuery(input, held.name);
      expect(screen.queryByTestId(commandEntryTestid(`card-${held.id}`))).toBeNull();
    }
  });

  it("R2118000 a card's result opens its inspect detail", async () => {
    at("/");
    render(<App />);
    const core = CATALOG["core-001"];
    if (core === undefined) throw new Error("the catalog has no core-001");
    const input = openMenu();
    typeQuery(input, core.name);
    fireEvent.click(screen.getByTestId(commandEntryTestid("card-core-001")));
    expect(window.location.pathname + window.location.search).toBe(paths.almanacCard("core-001"));
    const detail = await screen.findByTestId(INSPECT_DETAIL, undefined, SLOW);
    expect(detail).toHaveAttribute("data-card", "core-001");

    // While the detail is open it is a modal, so Ctrl+K opens nothing.
    fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    expect(screen.queryByTestId(commandMenuTestid.input)).toBeNull();
    fireEvent.click(screen.getByTestId(INSPECT_CLOSE));
    expect(screen.queryByTestId(INSPECT_DETAIL)).toBeNull();

    const rush = CATALOG["core-t-rush"];
    if (rush === undefined) throw new Error("the catalog has no core-t-rush");
    const again = openMenu();
    typeQuery(again, rush.name);
    fireEvent.click(screen.getByTestId(commandEntryTestid("card-core-t-rush")));
    const token = await screen.findByTestId(INSPECT_DETAIL, undefined, SLOW);
    expect(token).toHaveAttribute("data-card", "core-t-rush");
  });

  it("R2118000 Escape and a click outside close it and give the focus back", () => {
    render(
      <>
        <button type="button" data-testid="command-menu-opener">
          opener
        </button>
        <CommandMenu path="/" />
      </>,
    );
    const opener = screen.getByTestId("command-menu-opener");
    opener.focus();
    openMenu();
    expect(screen.getByTestId(commandMenuTestid.input)).toHaveFocus();
    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });
    expect(screen.queryByTestId(commandMenuTestid.dialog)).toBeNull();
    expect(opener).toHaveFocus();

    opener.focus();
    openMenu();
    fireEvent.click(screen.getByTestId(commandMenuTestid.scrim));
    expect(screen.queryByTestId(commandMenuTestid.dialog)).toBeNull();
    expect(opener).toHaveFocus();
  });

  it("R2118000 Tab and Shift+Tab stay inside it", () => {
    render(<CommandMenu path="/" />);
    const input = openMenu();
    expect(input).toHaveFocus();
    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Tab" });
    expect(screen.getByTestId(commandMenuTestid.close)).toHaveFocus();
    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Tab" });
    expect(screen.getByTestId(commandMenuTestid.input)).toHaveFocus();
    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Tab", shiftKey: true });
    expect(screen.getByTestId(commandMenuTestid.close)).toHaveFocus();
  });
});
