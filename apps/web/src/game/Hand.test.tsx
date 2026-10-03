// A hand row on the board (Hand.tsx): R504, an empty hand keeps its place, and R434's board half,
// the opponent's hand turning face up at the game's end. jsdom lays nothing out, so R504's size is
// proved here by structure and by the stylesheet's rules, and in pixels by
// e2e/cypress/component/board-layout.cy.tsx.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { CATALOG } from "@jackioh/cards";
import type { PlayerView } from "@jackioh/shared";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { __resetSettingsForTests } from "../settings/store.ts";
import { baseView, card, emptySide, fullBoardView } from "../test/fixtures.ts";
import Board from "./Board.tsx";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import Hand, { HAND_EMPTY_TEXT, handEmptyTestid } from "./Hand.tsx";
import { revealTestid } from "./reveal.ts";

const HERE = dirname(fileURLToPath(import.meta.url));

/** board.css without its comments, as a list of `{ selectors, body }` rules (innermost blocks). */
function boardRules(): { selectors: string[]; body: string }[] {
  const css = readFileSync(join(HERE, "board.css"), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
  return [...css.matchAll(/([^{}]+)\{([^{}]*)\}/g)].map((match) => ({
    selectors: (match[1] ?? "").split(",").map((selector) => selector.trim().replace(/\s+/g, " ")),
    body: (match[2] ?? "").replace(/\s+/g, " ").trim(),
  }));
}

afterEach(() => {
  cleanup();
  __resetSettingsForTests();
  try {
    window.localStorage.clear();
  } catch {
    // Storage is optional.
  }
});

describe("R504 an empty hand keeps its place", () => {
  const empty = (): PlayerView =>
    baseView({ you: emptySide("p1", { hand: [] }), opponent: emptySide("p2", { hand: { count: 0 } }) });

  it("R504 either seat's empty hand keeps its element, its count and one card-sized outline", () => {
    render(<Board view={empty()} />);
    for (const side of ["you", "opponent"] as const) {
      const hand = screen.getByTestId(`hand-${side}`);
      expect(hand, side).toHaveAttribute("data-empty", "true");
      expect(hand, side).toHaveAttribute("data-count", "0");
      expect(screen.getByTestId(`hand-count-${side}`)).toHaveTextContent("0");
      const row = hand.querySelector(".hand-cards");
      expect(row?.children, side).toHaveLength(1);
      const outline = within(hand).getByTestId(handEmptyTestid(side));
      expect(outline).toHaveClass("hand-empty");
      expect(outline).toHaveTextContent(HAND_EMPTY_TEXT);
      // An outline, not a card: nothing reads it as one (the effects layer's anchors, a drag, e2e).
      expect(hand.querySelector(".card"), side).toBeNull();
      expect(hand.querySelector(".hand-slot"), side).toBeNull();
    }
  });

  it("R504 the outline goes with the first card and comes back with the last", () => {
    const { rerender } = render(<Hand side="you" hand={[]} />);
    expect(screen.getByTestId(handEmptyTestid("you"))).toBeInTheDocument();
    rerender(<Hand side="you" hand={[card({ defId: "core-002" })]} />);
    expect(screen.queryByTestId(handEmptyTestid("you"))).toBeNull();
    expect(screen.getByTestId("hand-you")).not.toHaveAttribute("data-empty");
    rerender(<Hand side="you" hand={[]} />);
    expect(screen.getByTestId(handEmptyTestid("you"))).toBeInTheDocument();

    rerender(<Hand side="opponent" hand={{ count: 3 }} />);
    expect(screen.queryByTestId(handEmptyTestid("opponent"))).toBeNull();
    expect(screen.getByTestId("hand-opponent").querySelectorAll(".card-back")).toHaveLength(3);
  });

  it("R504 board.css sizes the outline as a card on every layout", () => {
    const rules = boardRules();
    const base = rules.find((rule) => rule.selectors.includes(".hand-empty"));
    expect(base?.body).toContain("flex: 0 0 var(--slot-w)");
    expect(base?.body).toContain("width: var(--slot-w)");
    // Positioned, so R439's heartbeat under the hand stays under it too.
    expect(base?.body).toContain("position: relative");
    // Your hand's outline is your hand card's height.
    const yours = rules.find((rule) => rule.selectors.includes(".hand-you .hand-empty"));
    expect(yours?.body).toBe("height: var(--hand-card-h);");
    const yourCards = rules.filter((rule) => rule.selectors.includes(".hand-you .card-hand") && /(^|; )height:/.test(rule.body));
    for (const rule of yourCards) expect(rule.body).toContain("height: var(--hand-card-h)");

    // Every rule that sizes the opponent's backs (the base and the desktop, tablet and phone-landscape
    // layouts) sizes the outline with them.
    const backs = rules.filter((rule) => rule.selectors.includes(".hand-opponent .card-hand") && /(^|; )height:/.test(rule.body));
    expect(backs.length).toBeGreaterThanOrEqual(4);
    for (const rule of backs) expect(rule.selectors, rule.body).toContain(".hand-opponent .hand-empty");
  });
});

describe("R434 the opponent's hand row turns face up at the game's end", () => {
  const lookup = lookupFromDefs(CATALOG);
  const REVEALED = ["core-002", "core-019", "core-055"];

  function finished(over: Partial<PlayerView["opponent"]> = {}): PlayerView {
    const view = fullBoardView();
    return {
      ...view,
      phase: "over",
      result: { winner: "p1", reason: "hero-death" },
      opponent: {
        ...view.opponent,
        hand: REVEALED.map((defId, index) => card({ instanceId: `r${String(index)}`, defId })),
        ...over,
      },
    };
  }

  function renderBoard(view: PlayerView, onClick = vi.fn()): void {
    render(
      <CatalogContext.Provider value={lookup}>
        <Board view={view} onClick={onClick} />
      </CatalogContext.Provider>,
    );
  }

  it("R434 each back turns over to its face where it lay, and none of them can be played", () => {
    const onClick = vi.fn();
    renderBoard(finished(), onClick);
    const hand = screen.getByTestId("hand-opponent");
    expect(hand).toHaveAttribute("data-revealed", "true");
    expect(hand).toHaveAttribute("data-count", String(REVEALED.length));
    const slots = hand.querySelectorAll(".hand-slot--revealed");
    expect(slots).toHaveLength(REVEALED.length);
    slots.forEach((slot, index) => {
      // Both sides are drawn, the back behind the face, so the flip has something to turn over.
      expect(slot.querySelector(".hand-flip-back .card-back")).not.toBeNull();
      const face = slot.querySelector(".hand-flip-face .card");
      expect(face).toHaveAttribute("data-testid", revealTestid.handCard(`r${String(index)}`));
      expect(face).toHaveAttribute("data-def-id", REVEALED[index]);
      expect((slot as HTMLElement).style.getPropertyValue("--i")).toBe(String(index));
    });
    // Read, never played: no hand-card testids for the opponent, no click reaches the board.
    expect(within(hand).queryAllByTestId(/^hand-card-/)).toHaveLength(0);
    fireEvent.click(screen.getByTestId(revealTestid.handCard("r1")));
    expect(onClick).not.toHaveBeenCalled();
    expect(screen.getByTestId(revealTestid.handCard("r1"))).not.toHaveAttribute("data-legal");
  });

  it("R434 while the game runs the opponent's hand stays backs", () => {
    renderBoard(fullBoardView());
    const hand = screen.getByTestId("hand-opponent");
    expect(hand).not.toHaveAttribute("data-revealed");
    expect(hand.querySelectorAll(".hand-slot--revealed")).toHaveLength(0);
    expect(hand.querySelectorAll(".card-back")).toHaveLength(6);
    expect(hand.querySelector("[data-def-id]")).toBeNull();
  });

  it("R434 the flip is scaled by --anim-scale and stopped under reduced motion, which leaves the face up", () => {
    const rules = boardRules();
    const flip = rules.find((rule) => rule.selectors.includes(".hand-flip") && rule.body.includes("animation:"));
    expect(flip?.body).toContain("hand-reveal calc(560ms * var(--anim-scale))");
    expect(flip?.body).toContain("calc(var(--i, 0) * 90ms * var(--anim-scale))");
    // The keyframes end on the face (the back is turned away behind it).
    const css = readFileSync(join(HERE, "board.css"), "utf8").replace(/\s+/g, " ");
    expect(css).toContain("@keyframes hand-reveal { 0% { transform: rotateY(180deg); } 100% { transform: rotateY(0deg); } }");
    expect(css).toContain("@media (prefers-reduced-motion: reduce) { .hand-flip { animation: none; } }");
    expect(rules.some((rule) => rule.selectors.includes(".hand-flip-back") && rule.body.includes("transform: rotateY(180deg)"))).toBe(true);
  });
});
