// v0.1.1 on the board (SPEC §10.10): a face-down trap's cost on its back (R370), your own face-down
// trap marked as one the other player cannot see (R371), and #93 Combo-Index's grade by its letter
// (R372). Each reads the view and nothing else (CLAUDE.md rule 7): the fixtures below are
// `PlayerView`s, and the component is the board that renders them.

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { PlayerView } from "@jackioh/shared";

import {
  CARD_SETTINGS_DEFAULTS,
  INSPECT_CLOSE,
  INSPECT_FACE,
  INSPECT_FACE_DOWN,
  INSPECT_FACE_DOWN_COST,
  INSPECT_HOVER,
  INSPECT_NOTE,
  INSPECT_SHEET,
  UNREVEALED_NOTE,
  closeInspect,
  writeCardSettings,
} from "../cards/index.ts";
import Board from "./Board.tsx";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import { testid } from "./contract.ts";
import { baseView, emptySide, faceUpBackrow } from "../test/fixtures.ts";

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

/** The back in one backrow zone. */
function backIn(side: "you" | "opponent", lane: number): HTMLElement {
  const zone = screen.getByTestId(testid.zone(side, "backrow", lane));
  const back = zone.querySelector<HTMLElement>(".card-back");
  if (back === null) throw new Error(`no back in ${side}'s backrow lane ${String(lane)}`);
  return back;
}

function hover(element: HTMLElement): void {
  fireEvent.pointerEnter(element, { pointerType: "mouse" });
  act(() => {
    vi.advanceTimersByTime(PAST_ANY_INSPECT_DELAY_MS);
  });
}

function longPress(element: HTMLElement): void {
  fireEvent.pointerDown(element, { pointerType: "touch", clientX: 10, clientY: 10 });
  act(() => {
    vi.advanceTimersByTime(PAST_ANY_INSPECT_DELAY_MS);
  });
  fireEvent.pointerUp(element, { pointerType: "touch", clientX: 10, clientY: 10 });
}

/** A board with face-down cards on both seats (one of yours the opponent stole, R33), with and without a cost. */
function backsView(): PlayerView {
  return baseView({
    you: emptySide("p1", {
      backrow: [
        faceUpBackrow("p1", { instanceId: "own-trap", defId: "core-041", type: "Trap", cost: 1, unrevealed: true }),
        faceUpBackrow("p1", { instanceId: "own-field", defId: "core-020", type: "Field Spell", cost: 2 }),
        faceUpBackrow("p1", { instanceId: "fired", defId: "core-061", type: "Field Trap", cost: 3 }),
        null,
        null,
      ],
    }),
    opponent: emptySide("p2", {
      backrow: [{ faceDown: true, cost: 2 }, { faceDown: true, cost: 0 }, { faceDown: true }, null, null],
    }),
  });
}

describe("R370 a face-down trap shows its cost", () => {
  it("R370 a back in the backrow wears the view's cost as a gem and says it in its label, naming nothing", () => {
    renderBoard(backsView());
    const names = Object.values(CATALOG).map((def) => def.name);
    for (const [lane, cost] of [
      [1, 2],
      [2, 0],
    ] as const) {
      const back = backIn("opponent", lane);
      expect(back.getAttribute("data-face-down")).toBe("true");
      expect(back.getAttribute("data-facedown-cost")).toBe(String(cost));
      expect(back.querySelector(".facedown-cost")?.textContent).toBe(String(cost));
      expect(back.getAttribute("aria-label")).toBe(`Face-down trap, (${String(cost)}) Cost`);
      // Still a back: no name, no definition, no instance id, no face.
      expect(back.textContent).toBe(String(cost));
      expect(back.hasAttribute("data-def-id")).toBe(false);
      expect(back.hasAttribute("data-testid")).toBe(false);
      expect(back.querySelector(".cf, .card-name")).toBeNull();
      for (const name of names) expect(back.getAttribute("aria-label")?.includes(name)).toBe(false);
    }
  });

  it("R370 a view that gives no cost draws the back as before, with no gem", () => {
    renderBoard(backsView());
    const back = backIn("opponent", 3);
    expect(back.querySelector(".facedown-cost")).toBeNull();
    expect(back.hasAttribute("data-facedown-cost")).toBe(false);
    expect(back.getAttribute("aria-label")).toBe("Face-down trap");
    expect(back.textContent).toBe("");
  });

  it("R370 R432 a resting mouse opens the back's preview: a face-down trap, (2) Cost, and who can see it", () => {
    vi.useFakeTimers();
    renderBoard(backsView());
    hover(backIn("opponent", 1));
    const preview = screen.getByTestId(INSPECT_FACE_DOWN);
    expect(preview.getAttribute("data-mode")).toBe("hover");
    expect(preview.getAttribute("aria-hidden")).toBe("true");
    expect(preview.textContent).toContain("Face-down trap");
    expect(within(preview).getByTestId(INSPECT_FACE_DOWN_COST).textContent).toBe("(2) Cost");
    expect(preview.textContent).toContain("Only the player who set it can see what it is.");
    // A back has no face to show large.
    expect(screen.queryByTestId(INSPECT_HOVER)).toBeNull();
    expect(screen.queryByTestId(INSPECT_FACE)).toBeNull();
  });

  it("R370 a long-press opens the same as a dialog with Close, and a back with no cost states none", () => {
    vi.useFakeTimers();
    renderBoard(backsView());
    longPress(backIn("opponent", 3));
    const sheet = screen.getByTestId(INSPECT_FACE_DOWN);
    expect(sheet.getAttribute("role")).toBe("dialog");
    expect(sheet.getAttribute("aria-label")).toBe("Face-down trap");
    expect(within(sheet).queryByTestId(INSPECT_FACE_DOWN_COST)).toBeNull();
    fireEvent.click(screen.getByTestId(INSPECT_CLOSE));
    expect(screen.queryByTestId(INSPECT_FACE_DOWN)).toBeNull();
    expect(screen.queryByTestId(INSPECT_SHEET)).toBeNull();
  });
});

describe("R371 your own face-down trap says the opponent can't see it", () => {
  it("R371 the view's unrevealed mark draws the Face down tag and the veil, and a public card has neither", () => {
    renderBoard(backsView());
    const own = screen.getByTestId(testid.card("own-trap"));
    expect(own.getAttribute("data-unrevealed")).toBe("true");
    // You read your own trap: its face is drawn, name and all.
    expect(own.getAttribute("data-def-id")).toBe("core-041");
    expect(own.querySelector(".card-name")?.textContent).toBe(CATALOG["core-041"]?.name);
    const tag = screen.getByTestId(testid.unrevealed("own-trap"));
    expect(tag.getAttribute("title")).toBe(UNREVEALED_NOTE);
    expect(tag.textContent).toContain("Face down");
    expect(tag.textContent).toContain("your opponent can't see this card");
    // The struck-through eye is part of the tag, so the mark is a shape as well as a word.
    expect(tag.querySelector("img.cf-icon--eyeOff")).not.toBeNull();

    for (const id of ["own-field", "fired"]) {
      const root = screen.getByTestId(testid.card(id));
      expect(root.hasAttribute("data-unrevealed"), id).toBe(false);
      expect(screen.queryByTestId(testid.unrevealed(id)), id).toBeNull();
    }
  });

  it("R371 its hover preview and its sheet carry the note over the face", () => {
    vi.useFakeTimers();
    renderBoard(backsView());
    const own = screen.getByTestId(testid.card("own-trap"));
    hover(own);
    expect(within(screen.getByTestId(INSPECT_HOVER)).getByTestId(INSPECT_NOTE).textContent).toBe(UNREVEALED_NOTE);
    fireEvent.pointerLeave(own, { pointerType: "mouse" });
    longPress(own);
    expect(within(screen.getByTestId(INSPECT_SHEET)).getByTestId(INSPECT_NOTE).textContent).toBe(UNREVEALED_NOTE);
    fireEvent.click(screen.getByTestId(INSPECT_CLOSE));

    hover(screen.getByTestId(testid.card("own-field")));
    expect(within(screen.getByTestId(INSPECT_HOVER)).queryByTestId(INSPECT_NOTE)).toBeNull();
  });
});

describe("R372 Combo-Index shows its grade by its letter", () => {
  function comboView(counters: { grade?: number; gradeLetter?: string }, withPreview: boolean): PlayerView {
    return baseView({
      you: emptySide("p1", {
        backrow: [
          faceUpBackrow("p1", {
            instanceId: "combo",
            defId: "core-093",
            type: "Field Spell",
            cost: 2,
            counters,
            ...(withPreview
              ? {
                  preview: [
                    { label: "Grade", value: 3, display: "C" },
                    { label: "N = the grades from E to the current one", value: 3 },
                  ],
                }
              : {}),
          }),
          null,
          null,
          null,
          null,
        ],
      }),
    });
  }

  it("R372 the grade badge prints the letter the view names, and the number only when it names none", () => {
    renderBoard(comboView({ grade: 3, gradeLetter: "C" }, false));
    const badge = screen.getByTestId(testid.card("combo")).querySelector<HTMLElement>('[data-counter="grade"]');
    expect(badge?.textContent).toBe("C");
    expect(badge?.getAttribute("data-grade")).toBe("3");
    expect(badge?.getAttribute("data-grade-letter")).toBe("C");
    expect(badge?.getAttribute("title")).toBe("Grade C");
    cleanup();

    renderBoard(comboView({ grade: 3 }, false));
    const bare = screen.getByTestId(testid.card("combo")).querySelector<HTMLElement>('[data-counter="grade"]');
    expect(bare?.textContent).toBe("3");
  });

  it("R372 its face in play prints 'Grade {C}' and N's value after the words they belong to", () => {
    vi.useFakeTimers();
    renderBoard(comboView({ grade: 3, gradeLetter: "C" }, true));
    hover(screen.getByTestId(testid.card("combo")));
    const face = screen.getByTestId(INSPECT_FACE);
    const values = [...face.querySelectorAll<HTMLElement>(".cf-value")];
    expect(values.map((value) => value.textContent)).toEqual(["{C}", "{3}"]);
    expect(values.map((value) => value.getAttribute("data-label"))).toEqual(["Grade", "N = the grades from E to the current one"]);
    expect(face.textContent).toContain("Grade {C} (starts at E).");
  });
});
