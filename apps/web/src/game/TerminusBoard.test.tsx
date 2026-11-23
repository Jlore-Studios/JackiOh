// R3338000: the Terminus cards' set pieces on the board. Each card's moment holds the board for its own
// set piece (`data-set-piece="<key>"`, `--set-piece-hold`, its shape on `data-set-piece-shape`) behind
// the "Tap to skip" hint, and a click skips it, as every set piece (R2785000). Big moments' "short" and
// "off" set it; under Reduce motion nothing holds and the set piece is one still frame.

import { CATALOG } from "@jackioh/cards";
import type { GameEvent, PlayerView } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { FX_CARD_TABLE } from "../fx/cards/tables.ts";
import { galleryCardEvents } from "../fx/motifs/gallery.ts";
import { SET_PIECES, type SetPieceKey } from "../fx/setPieces.ts";
import { resetFxSettingsForTests, setFxSettings } from "../fx/settings.ts";
import { TERMINUS_PLANNERS } from "../fx/terminus/index.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { fullBoardView, withEvents } from "../test/fixtures.ts";
import { SET_PIECE_SHORT_HOLD_MS } from "./animations.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import Game from "./Game.tsx";
import { readRootVar } from "./rootVars.ts";

const lookup = lookupFromDefs(CATALOG);
const START: GameEvent[] = [{ type: "turnStarted", player: "p1", turn: 5 }];
const SKIP_HINT = "set-piece-skip";
/** The longest the entries before a set piece's can take, at speed 1. */
const BEFORE_MAX_MS = 4_000;
/** Every Terminus set piece: the seven signatures and the two Terminus board clears. */
const KEYS: readonly SetPieceKey[] = [...(Object.keys(TERMINUS_PLANNERS) as SetPieceKey[]), "voidFold", "farewell"];

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.useRealTimers();
  window.localStorage.clear();
  __resetSettingsForTests();
  resetFxSettingsForTests();
  document.body.innerHTML = "";
});

function cardOf(key: SetPieceKey): string {
  const found = [...FX_CARD_TABLE].find(([, entry]) => entry.kind === "setPiece" && entry.key === key);
  if (found === undefined) throw new Error(`no card names ${key}`);
  return found[0];
}

/** The board before `key`'s card plays, then after its play arrives, run on until its set piece holds (or `until` says stop). */
function playOn(key: SetPieceKey, until: (game: HTMLElement) => boolean = (game) => game.getAttribute("data-set-piece") === key): HTMLElement {
  const view: PlayerView = fullBoardView();
  const events = galleryCardEvents(cardOf(key), { kind: "setPiece", key }, view, 1);
  const { rerender } = render(
    <CatalogContext.Provider value={lookup}>
      <Game view={withEvents(view, START)} legal={[]} onAction={vi.fn()} />
    </CatalogContext.Provider>,
  );
  rerender(
    <CatalogContext.Provider value={lookup}>
      <Game view={withEvents(view, [...START, ...events])} legal={[]} onAction={vi.fn()} />
    </CatalogContext.Provider>,
  );
  const game = screen.getByTestId("game");
  for (let waited = 0; waited < BEFORE_MAX_MS && !until(game); waited += 10) {
    act(() => {
      vi.advanceTimersByTime(10);
    });
  }
  return game;
}

describe("R3338000 a Terminus card's set piece on the board", () => {
  it.each(KEYS)("R3338000 each card holds as its key: %s", (key) => {
    const game = playOn(key);
    expect(game).toHaveAttribute("data-set-piece", key);
    expect(game).toHaveAttribute("data-set-piece-shape", SET_PIECES[key].shape);
    const hold = SET_PIECES[key].holdMs;
    expect(readRootVar(game, "--set-piece-hold")).toBe(`${String(hold)}ms`);
    expect(screen.getByTestId(SKIP_HINT)).toBeInTheDocument();
    // A click skips it.
    fireEvent.click(game);
    expect(game).not.toHaveAttribute("data-set-piece");
    expect(screen.queryByTestId(SKIP_HINT)).toBeNull();
  });

  // One test a card and a setting: each plays the board through on its own.
  it.each(KEYS)("R3338000 Big moments short holds the short beat: %s", (key) => {
    setFxSettings({ bigMoments: "short" });
    const short = playOn(key);
    expect(short).toHaveAttribute("data-set-piece", key);
    expect(readRootVar(short, "--set-piece-hold")).toBe(`${String(SET_PIECE_SHORT_HOLD_MS)}ms`);
  });

  it.each(KEYS)("R3338000 Big moments off holds nothing: %s", (key) => {
    setFxSettings({ bigMoments: "off" });
    // Played through, the board never holds and never shows the hint.
    let held = false;
    playOn(key, (game) => {
      held ||= game.hasAttribute("data-set-piece") || screen.queryByTestId(SKIP_HINT) !== null;
      return false;
    });
    expect(held).toBe(false);
  });

  it.each(KEYS)("R3338000 reduce motion: no hold, a still [data-fx=setPiece][data-still]: %s", (key) => {
    // jsdom lays nothing out: give every element a box, so the still frame has somewhere to stand.
    vi.spyOn(Element.prototype, "getBoundingClientRect").mockReturnValue({
      x: 300, y: 200, left: 300, top: 200, width: 90, height: 126, right: 390, bottom: 326, toJSON: () => ({}),
    } as DOMRect);
    writeSettings({ reduceMotion: true });
    const game = playOn(key, () => document.querySelector('[data-fx="setPiece"]') !== null);
    expect(game).toHaveAttribute("data-reduced-motion", "true");
    expect(game).not.toHaveAttribute("data-set-piece");
    expect(screen.queryByTestId(SKIP_HINT)).toBeNull();
    const still = document.querySelector<HTMLElement>('[data-fx="setPiece"]');
    expect(still).not.toBeNull();
    expect(still).toHaveAttribute("data-still", "true");
    expect(still).toHaveAttribute("data-shape", SET_PIECES[key].shape);
  });
});
