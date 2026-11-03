// R2739000: the seat waiting on the other's card picker sees one card back per option, laid out as the
// chooser's bare picker, when its view counts them (`optionCount`), and today's notice when it does not.
// R2739001: the back the opponent's pointer is on, as last relayed, is outlined; the chooser's own engine
// card picker sends its hovered option once per change, and null when the pointer or focus leaves or the
// picker closes. A mulligan, a replay's picker and a play's own picker send nothing.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { ActionBody, PendingView, PlayerView } from "@jackioh/shared";

import { DEFAULT_CARD_COSMETICS, cosmeticsOf } from "../cards/cosmetics.ts";
import type { Interaction } from "./actions.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import type { ChoiceHover } from "./choiceHover.ts";
import Prompt from "./Prompt.tsx";
import { WearerProvider } from "./wearer.ts";
import { baseView, card, emptySide, pendingFor } from "../test/fixtures.ts";

afterEach(cleanup);

const lookup = lookupFromDefs(CATALOG);

const WAITING_NAME = "Your opponent is choosing…";

/** The other seat (p2) holds a prompt, a card picker the view counts `optionCount` options of when it is given. */
function theirs(optionCount?: number): PendingView {
  return { forYou: false, pendingFor: "p2", ...(optionCount === undefined ? {} : { optionCount }) };
}

function viewWith(pending: PendingView): PlayerView {
  return baseView({
    you: emptySide("p1", {
      hand: [card({ instanceId: "h1", defId: "core-002" }), card({ instanceId: "h2", defId: "core-019" }), card({ instanceId: "h3", defId: "core-004" })],
    }),
    opponent: emptySide("p2", { hand: { count: 3 } }),
    pending,
  });
}

function renderPrompt(
  view: PlayerView,
  opts: { choiceHover?: ChoiceHover; readOnly?: boolean; interaction?: Interaction; legal?: ActionBody[] } = {},
) {
  return render(
    <CatalogContext.Provider value={lookup}>
      <Prompt
        view={view}
        onAction={vi.fn()}
        {...(opts.choiceHover === undefined ? {} : { choiceHover: opts.choiceHover })}
        {...(opts.readOnly === true ? { readOnly: true } : {})}
        {...(opts.interaction === undefined ? {} : { interaction: opts.interaction })}
        {...(opts.legal === undefined ? {} : { legal: opts.legal })}
      />
    </CatalogContext.Provider>,
  );
}

function backs(): HTMLElement[] {
  return [...screen.getByTestId("opponent-choice").querySelectorAll<HTMLElement>("[data-testid^='opponent-choice-']")];
}

function hoveredBacks(): string[] {
  return backs()
    .filter((back) => back.hasAttribute("data-hovered"))
    .map((back) => back.getAttribute("data-testid") ?? "");
}

function option(key: string): HTMLElement {
  return screen.getByTestId(`prompt-option-${key}`);
}

const DISCOVER = pendingFor("discover", [
  { key: "mode:core-043", label: "Flood", defId: "core-043" },
  { key: "mode:core-055", label: "Archivist", defId: "core-055" },
  { key: "mode:core-066", label: "Lava Golem", defId: "core-066" },
]);

describe("R2739000 the other seat's card picker, face down", () => {
  it("R2739000 the other seat sees one back per option for a Discover and a Choose one", () => {
    for (const count of [3, 2]) {
      renderPrompt(viewWith(theirs(count)));
      const dialog = screen.getByRole("dialog", { name: WAITING_NAME });
      expect(dialog).toBe(screen.getByTestId("prompt-modal"));
      // Prompt.tsx's waiting contract: nothing that waits on a prompt of the viewer's takes it for one.
      expect(dialog).toHaveAttribute("data-prompt-waiting", "waiting");
      expect(dialog).toHaveAttribute("data-pending-for", "p2");
      expect(dialog).not.toHaveAttribute("data-prompt-kind");
      expect(dialog).toHaveAttribute("data-prompt-layout", "cards");
      expect(dialog).toHaveAttribute("data-bare", "true");
      expect(backs().map((back) => back.getAttribute("data-testid"))).toEqual(
        Array.from({ length: count }, (_, at) => `opponent-choice-${at}`),
      );
      for (const back of backs()) expect(back.querySelectorAll(".cf-back")).toHaveLength(1);
      // A count and nothing else: no words, no option, no control.
      expect(dialog.textContent).toBe("");
      expect(screen.queryAllByRole("button")).toEqual([]);
      expect(document.querySelectorAll("[data-testid^='prompt-option-']")).toHaveLength(0);
      expect(hoveredBacks()).toEqual([]);
      cleanup();
    }
  });

  it("R2739000 the backs are the back the opponent's hand wears, their loadout's when they froze one", () => {
    const theirBack = cosmeticsOf("cardBack").find((cosmetic) => !cosmetic.default)?.id;
    if (theirBack === undefined) throw new Error("no card back but the default");
    const seat = (cardBack: string) => ({ seat: { ...DEFAULT_CARD_COSMETICS, cardBack }, cards: {} });
    render(
      <WearerProvider seat="p1" viewer="p1" loadouts={{ p1: seat(DEFAULT_CARD_COSMETICS.cardBack), p2: seat(theirBack) }}>
        <Prompt view={viewWith(theirs(3))} onAction={vi.fn()} />
      </WearerProvider>,
    );
    const worn = backs().map((back) => back.querySelector(".cf-back")?.getAttribute("data-card-back"));
    expect(worn).toEqual([theirBack, theirBack, theirBack]);
  });

  it("R2739000 without a count it is today's notice", () => {
    renderPrompt(viewWith(theirs()), { choiceHover: { emit: vi.fn(), opponent: 1 } });
    const modal = screen.getByTestId("prompt-modal");
    expect(modal).toHaveClass("prompt-waiting");
    expect(modal).toHaveAttribute("data-prompt-waiting", "waiting");
    expect(modal).toHaveAttribute("data-pending-for", "p2");
    expect(modal).not.toHaveAttribute("data-prompt-layout");
    expect(screen.getByRole("status")).toHaveTextContent(WAITING_NAME);
    expect(screen.queryByTestId("opponent-choice")).toBeNull();
    expect(document.querySelector(".cf-back")).toBeNull();
  });

  // jsdom lays nothing out, so the sheet is read: the back is laid over its box (`.cf-back`, `inset: 0`)
  // and grows nothing, so without a shape of its own a phone's option box (--card-h tall) drew it squat.
  it("R2739000 a back keeps a card's 5:7 at every width", () => {
    const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "prompt.css"), "utf8");
    const start = css.indexOf(".prompt-card.prompt-card--back {");
    expect(start).toBeGreaterThan(-1);
    expect(css.slice(start, css.indexOf("}", start))).toContain("aspect-ratio: 5 / 7;");
  });

  // The phone docks every picker's sheet to the bottom edge (`place-items: end stretch`); the backs stay in
  // the middle of the screen, clear of the viewer's hand and control bar.
  it("R2739000 the backs float in the middle of a phone's screen, not in the bottom sheet", () => {
    const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "prompt.css"), "utf8");
    const phone = "@media (max-width: 600px), (orientation: landscape) and (max-height: 500px) {\n  .prompt-scrim:has(> .prompt-opponent-choice) {";
    const start = css.indexOf(phone);
    expect(start).toBeGreaterThan(-1);
    expect(css.slice(start, css.indexOf("}", start))).toContain("place-items: center stretch;");
  });
});

describe("R2739001 the hovered option, both ways", () => {
  it("R2739001 a relayed index outlines exactly that back and null clears it", () => {
    const emit = vi.fn();
    const view = viewWith(theirs(3));
    const { rerender } = renderPrompt(view, { choiceHover: { emit, opponent: 1 } });
    expect(hoveredBacks()).toEqual(["opponent-choice-1"]);

    const again = (opponent: number | null) =>
      rerender(
        <CatalogContext.Provider value={lookup}>
          <Prompt view={view} onAction={vi.fn()} choiceHover={{ emit, opponent }} />
        </CatalogContext.Provider>,
      );
    again(2);
    expect(hoveredBacks()).toEqual(["opponent-choice-2"]);
    again(null);
    expect(hoveredBacks()).toEqual([]);
    // An index past the backs outlines none (the server drops one anyway).
    again(3);
    expect(hoveredBacks()).toEqual([]);
    // The watching seat's backs send nothing.
    expect(emit).not.toHaveBeenCalled();
  });

  it("R2739001 the chooser's hover and focus emit once per change", () => {
    const emit = vi.fn();
    const { unmount } = renderPrompt(viewWith(DISCOVER), { choiceHover: { emit, opponent: null } });
    expect(emit).not.toHaveBeenCalled();

    // A repeated hover emits once, and so does the pointer moving within a card.
    fireEvent.pointerOver(option("mode:core-055"));
    fireEvent.pointerOver(option("mode:core-055"));
    expect(emit.mock.calls).toEqual([[1]]);
    fireEvent.pointerOver(option("mode:core-043"));
    expect(emit.mock.calls).toEqual([[1], [0]]);

    // The pointer leaving the cards emits null.
    fireEvent.pointerLeave(option("mode:core-043"));
    expect(emit.mock.calls).toEqual([[1], [0], [null]]);

    // Focus moving between the cards emits the new one only; focus leaving them emits null.
    fireEvent.focus(option("mode:core-066"));
    fireEvent.blur(option("mode:core-066"), { relatedTarget: option("mode:core-043") });
    fireEvent.focus(option("mode:core-043"));
    fireEvent.blur(option("mode:core-043"), { relatedTarget: null });
    expect(emit.mock.calls).toEqual([[1], [0], [null], [2], [0], [null]]);

    // Folding the picker to look at the board, then closing it, each clear a hovered card.
    fireEvent.pointerOver(option("mode:core-066"));
    fireEvent.click(screen.getByTestId("prompt-peek"));
    expect(emit.mock.calls.slice(6)).toEqual([[2], [null]]);
    fireEvent.click(screen.getByTestId("prompt-peek"));
    fireEvent.pointerOver(option("mode:core-055"));
    unmount();
    expect(emit.mock.calls.slice(8)).toEqual([[1], [null]]);
  });

  it("R2739001 an answer's index is its place among the cards, and Stop is no card", () => {
    const emit = vi.fn();
    renderPrompt(
      viewWith(
        pendingFor(
          "answer",
          [...["A", "B", "C"].map((letter) => ({ key: `mode:${letter}`, label: letter })), { key: "none", label: "Stop" }],
          { prompt: "Which?" },
        ),
      ),
      { choiceHover: { emit, opponent: null } },
    );
    // An answer takes the focus as it opens (B5 E18), which is the keyboard's place on the first card.
    expect(emit.mock.calls).toEqual([[0]]);
    fireEvent.pointerOver(option("mode:C"));
    fireEvent.pointerOver(option("none"));
    expect(emit.mock.calls).toEqual([[0], [2]]);
  });

  it("R2739001 a mulligan, a replay's picker and a play's own picker emit nothing", () => {
    const emit = vi.fn();
    const choiceHover = { emit, opponent: null };

    renderPrompt(
      viewWith(
        pendingFor(
          "mulligan",
          ["h1", "h2", "h3"].map((id) => ({ key: id, label: id, instanceId: id })),
          { min: 0, max: 3, prompt: "Keep which cards?" },
        ),
      ),
      { choiceHover },
    );
    fireEvent.pointerOver(option("h2"));
    fireEvent.focus(option("h3"));
    cleanup();

    // R2211000: a replay's picker opens folded; unfolded, its cards still send nothing.
    renderPrompt(viewWith(DISCOVER), { choiceHover, readOnly: true });
    fireEvent.click(screen.getByTestId("prompt-peek"));
    fireEvent.pointerOver(option("mode:core-055"));
    fireEvent.focus(option("mode:core-043"));
    cleanup();

    // R81: a play's Choose one opens no prompt of the sender's, so the server has none to relay.
    const modes: ActionBody[] = ["health", "board", "library"].map((mode) => ({ type: "play", instanceId: "h1", modes: [mode] }));
    const view = baseView({
      you: emptySide("p1", { hand: [card({ instanceId: "h1", defId: "core-087" })] }),
      opponent: emptySide("p2", { hand: { count: 3 } }),
    });
    renderPrompt(view, { choiceHover, interaction: { stage: "playing", instanceId: "h1", candidates: modes, picked: {} }, legal: modes });
    expect(screen.getByTestId("prompt-modal")).toHaveAttribute("data-prompt-layout", "cards");
    fireEvent.pointerOver(option("board"));
    fireEvent.focus(option("health"));
    cleanup();

    expect(emit).not.toHaveBeenCalled();
  });
});
