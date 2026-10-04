// R658 on the board: Classic #87 Plague Chalice's warning on the viewer's own hand card. The engine
// puts `counteredOnPlay: true` on the card's view and the board only draws it (CLAUDE.md rule 7): the
// fixtures below are `PlayerView`s, and the component is the board that renders them.

import { readFileSync, existsSync } from "node:fs";
import { resolve } from "node:path";

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { PlayerView } from "@jackioh/shared";

import { CARD_SETTINGS_DEFAULTS, INSPECT_CLOSE, INSPECT_HOVER, INSPECT_NOTE, INSPECT_SHEET, closeInspect, writeCardSettings } from "../cards/index.ts";
import Board from "./Board.tsx";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import { testid } from "./contract.ts";
import { COUNTERED_NOTE } from "./glow.ts";
import { baseView, card, emptySide } from "../test/fixtures.ts";

afterEach(() => {
  cleanup();
  closeInspect();
  vi.useRealTimers();
  writeCardSettings(CARD_SETTINGS_DEFAULTS);
});

const lookup = lookupFromDefs(CATALOG);
const PAST_ANY_INSPECT_DELAY_MS = 2_000;

/** p1's hand: a (2) the Chalice would counter, and a (1) it would not. */
function warnedView(): PlayerView {
  return baseView({
    you: emptySide("p1", {
      mana: { current: 4, max: 4 },
      hand: [
        card({ instanceId: "doomed", defId: "core-020", cost: 2, counteredOnPlay: true }),
        card({ instanceId: "safe", defId: "core-008", cost: 1 }),
      ],
    }),
  });
}

function renderBoard(view: PlayerView): void {
  render(
    <CatalogContext.Provider value={lookup}>
      <Board view={view} />
    </CatalogContext.Provider>,
  );
}

describe("R658 the Plague Chalice warning on a hand card", () => {
  it("R658 the view's flag draws the warning film and badge, with the words for a screen reader; a card without it has neither", () => {
    renderBoard(warnedView());

    const doomed = screen.getByTestId(testid.handCard("doomed"));
    expect(doomed.getAttribute("data-countered-on-play")).toBe("true");
    const warning = within(doomed).getByTestId(testid.countered("doomed"));
    expect(warning.querySelector(".countered-warning-badge")?.getAttribute("title")).toBe(COUNTERED_NOTE);
    expect(warning.textContent).toBe(COUNTERED_NOTE);

    const safe = screen.getByTestId(testid.handCard("safe"));
    expect(safe.hasAttribute("data-countered-on-play")).toBe(false);
    expect(screen.queryByTestId(testid.countered("safe"))).toBeNull();
  });

  it("R658 the hover preview and the long-press sheet carry the note, so a phone shows it with no hover", () => {
    vi.useFakeTimers();
    renderBoard(warnedView());
    const doomed = screen.getByTestId(testid.handCard("doomed"));

    fireEvent.pointerEnter(doomed, { pointerType: "mouse" });
    act(() => {
      vi.advanceTimersByTime(PAST_ANY_INSPECT_DELAY_MS);
    });
    expect(within(screen.getByTestId(INSPECT_HOVER)).getByTestId(INSPECT_NOTE).textContent).toBe(COUNTERED_NOTE);
    fireEvent.pointerLeave(doomed, { pointerType: "mouse" });

    fireEvent.pointerDown(doomed, { pointerType: "touch", clientX: 10, clientY: 10 });
    act(() => {
      vi.advanceTimersByTime(PAST_ANY_INSPECT_DELAY_MS);
    });
    fireEvent.pointerUp(doomed, { pointerType: "touch", clientX: 10, clientY: 10 });
    expect(within(screen.getByTestId(INSPECT_SHEET)).getByTestId(INSPECT_NOTE).textContent).toBe(COUNTERED_NOTE);
    fireEvent.click(screen.getByTestId(INSPECT_CLOSE));
  });
});

describe("R658 countered.css", () => {
  function sheet(): string {
    for (const candidate of ["src/game/countered.css", "apps/web/src/game/countered.css"]) {
      const path = resolve(process.cwd(), candidate);
      if (existsSync(path)) return readFileSync(path, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
    }
    throw new Error("countered.css not found");
  }

  it("R658 the film takes no pointer event, and its bubbles rise by transform alone", () => {
    const css = sheet();
    expect(css).toMatch(/\.countered-warning \{[^}]*pointer-events: none;/);
    const keyframes = /@keyframes countered-rise \{([\s\S]*?)\n\}/.exec(css)?.[1] ?? "";
    expect(keyframes).toMatch(/transform:/);
    expect(keyframes).not.toMatch(/(?<![-\w])(top|bottom|left|right|background|opacity):/);
  });

  it("R658 under the media query and under <html data-reduce-motion> the bubbles are gone and the tint holds still", () => {
    const css = sheet();
    const media = /@media \(prefers-reduced-motion: reduce\) \{([\s\S]*?)\n\}/.exec(css)?.[1] ?? "";
    for (const part of [".countered-warning::before", ".countered-warning::after"]) {
      expect(media, `${part} under the media query`).toContain(part);
      expect(css, `${part} under the setting`).toContain(`:root[data-reduce-motion="true"] ${part}`);
    }
    expect(media).toMatch(/animation: none/);
    // The tint and the badge are the film itself, which nothing hides.
    expect(css).not.toMatch(/\.countered-warning(-badge)? \{[^}]*display: none/);
  });
});
