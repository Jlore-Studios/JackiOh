// R90 (#992): a hand card whose listed plays the engine cut at `MAX_CHOICE_COMBINATIONS` carries
// `choicesTruncated: true` in its player's own view, and the board says so on the card: a badge, and
// the note in its hover preview. The words are informational: the combinations not listed stay legal.
// The engine half is proved in crates/engine/tests/rules/play_choices_filters.rs.

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { PlayerView } from "@jackioh/shared";

import { CARD_SETTINGS_DEFAULTS, INSPECT_HOVER, INSPECT_NOTE, closeInspect, writeCardSettings } from "../cards/index.ts";
import Board from "./Board.tsx";
import { CHOICES_TRUNCATED_NOTE } from "./Card.tsx";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import { testid } from "./contract.ts";
import { baseView, card, emptySide } from "../test/fixtures.ts";

afterEach(() => {
  cleanup();
  closeInspect();
  vi.useRealTimers();
  writeCardSettings(CARD_SETTINGS_DEFAULTS);
});

const lookup = lookupFromDefs(CATALOG);
const PAST_ANY_INSPECT_DELAY_MS = 2_000;

function renderBoard(view: PlayerView): HTMLElement {
  const { container } = render(
    <CatalogContext.Provider value={lookup}>
      <Board view={view} />
    </CatalogContext.Provider>,
  );
  return container;
}

function hover(element: HTMLElement): void {
  fireEvent.pointerEnter(element, { pointerType: "mouse" });
  act(() => {
    vi.advanceTimersByTime(PAST_ANY_INSPECT_DELAY_MS);
  });
}

/** Two cards in hand: c1 with its listed plays cut, c2 with none. */
function truncatedView(): PlayerView {
  return baseView({
    you: emptySide("p1", {
      hand: [
        card({ instanceId: "c1", defId: "core-001", choicesTruncated: true }),
        card({ instanceId: "c2", defId: "core-001" }),
      ],
    }),
  });
}

describe("R90 a hand card whose listed plays were cut says so", () => {
  it("R90 the flagged hand card wears the badge with the note as its tooltip and hidden text, and an unflagged one wears none", () => {
    renderBoard(truncatedView());
    const badge = screen.getByTestId(testid.choicesTruncated("c1"));
    expect(screen.getByTestId(testid.handCard("c1")).contains(badge)).toBe(true);
    expect(badge.getAttribute("title")).toBe(CHOICES_TRUNCATED_NOTE);
    expect(badge.textContent).toContain(CHOICES_TRUNCATED_NOTE);
    expect(screen.queryByTestId(testid.choicesTruncated("c2"))).toBeNull();
  });

  it("R90 its hover preview carries the note", () => {
    vi.useFakeTimers();
    renderBoard(truncatedView());
    hover(screen.getByTestId(testid.handCard("c1")));
    expect(within(screen.getByTestId(INSPECT_HOVER)).getByTestId(INSPECT_NOTE).textContent).toBe(CHOICES_TRUNCATED_NOTE);
  });

  it("R90 an unflagged hand card's hover preview has no such note", () => {
    vi.useFakeTimers();
    renderBoard(truncatedView());
    hover(screen.getByTestId(testid.handCard("c2")));
    expect(within(screen.getByTestId(INSPECT_HOVER)).queryByTestId(INSPECT_NOTE)).toBeNull();
  });

  it("R90 the note is informational: it calls no combination illegal", () => {
    expect(CHOICES_TRUNCATED_NOTE).not.toMatch(/illegal|not allowed|can't|cannot/i);
  });
});
