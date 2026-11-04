// #2878: the practice table draws no HUD above the board. The enemy hero's seat names the
// AI and the difficulty, New game and Menu ride the control bar (or the ⋯ menu on phones), the
// modifier chip and "AI is thinking…" ride the board's status row, the outcome merges into the
// board's result chip, and the turn banner stays in the DOM with no visible box.

import { useState } from "react";

import type { PlayerView } from "@jackioh/shared";

import { INSPECT_LIST_SHEET } from "../../../apps/web/src/cards/inspect/testids.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { ModifierList } from "../../../apps/web/src/game/ModifierList.tsx";
import "../../../apps/web/src/practice/practice.css";
import { PracticeResult } from "../../../apps/web/src/practice/PracticeResult.tsx";
import { ThinkIndicator } from "../../../apps/web/src/practice/ThinkIndicator.tsx";
import { practiceTestid } from "../../../apps/web/src/practice/testids.ts";
import { card, fullBoardView } from "../../../apps/web/src/test/fixtures.ts";

/** board-layout.cy.tsx's six `VIEWPORTS` (B28). */
const VIEWPORTS = [
  { label: "desktop", width: 1280, height: 720 },
  { label: "landscape", width: 844, height: 390 },
  { label: "tablet", width: 768, height: 1024 },
  { label: "phone", width: 390, height: 844 },
  { label: "tablet landscape", width: 1024, height: 768 },
  { label: "phone, Safari's visible viewport", width: 390, height: 664 },
] as const;

const HAND_SIZES = [4, 7, 10] as const;

const HAND_CARD_IDS = [
  "core-002", "core-019", "core-055", "core-077", "core-011",
  "core-013", "core-025", "core-054", "core-066", "core-068",
] as const;

/** Fractional-overlap rounding slack. */
const EPSILON = 1;

/** game/phoneBar.ts's query, so the spec branches exactly where the board does. */
const PHONE_BAR_QUERY = "(max-width: 600px), (orientation: landscape) and (max-height: 500px)";

/** The outcome words the one visible outcome may wear. */
const OUTCOME_WORDS = /\b(win|loss|victory|defeat|draw)\b/i;

function isPhoneLayout(win: Window): boolean {
  return win.matchMedia(PHONE_BAR_QUERY).matches;
}

function menuButtons(): React.JSX.Element {
  return (
    <>
      <button
        type="button"
        className="control control-practice-new"
        data-testid={practiceTestid.newGame}
      >
        New game
      </button>
      <button
        type="button"
        className="control control-practice-menu"
        data-testid={practiceTestid.menu}
        aria-label="Main menu"
      >
        Menu
      </button>
    </>
  );
}

function mountMidgame(view: PlayerView, thinking: boolean): void {
  cy.mount(
    <div className="app-shell app-shell--wide practice practice--game">
      <div className="practice-board practice-table" data-difficulty="easy" data-thinking={thinking ? "true" : "false"}>
        <Game
          view={view}
          legal={[]}
          onAction={() => undefined}
          resultForm="chip"
          enemyLabel="AI · Easy"
          controlExtras={menuButtons()}
          matchStatus={
            <>
              <ModifierList view={view} opponentLabel="AI" />
              <ThinkIndicator thinking={thinking} />
            </>
          }
          onOpenResult={() => undefined}
          openResultTestId={practiceTestid.outcome}
        />
      </div>
    </div>,
  );
}

/** A loss with the opponent's hand revealed (R434) and the "won't earn" notice set. */
function gameOverView(): PlayerView {
  const base = fullBoardView();
  return {
    ...base,
    result: { winner: "p2", reason: "hero-death" },
    opponent: { ...base.opponent, hand: [card({ defId: "core-002" }), card({ defId: "core-019" })] },
  };
}

/** The game-over shell: the route's result dialog over the board's chip, as `/practice` mounts it. */
function GameOverPractice(): React.JSX.Element {
  const [viewingBoard, setViewingBoard] = useState(false);
  const view = gameOverView();
  const result = view.result;
  if (result === null) throw new Error("the game-over fixture has a result");
  return (
    <div className="app-shell app-shell--wide practice practice--game">
      {viewingBoard ? null : (
        <PracticeResult
          result={result}
          viewer={view.viewer}
          difficulty="easy"
          onPlayAgain={() => undefined}
          onChangeSetup={() => undefined}
          onViewBoard={() => setViewingBoard(true)}
        />
      )}
      <div className="practice-board practice-table" data-difficulty="easy" data-thinking="false">
        <Game
          view={view}
          legal={[]}
          onAction={() => undefined}
          resultForm="chip"
          enemyLabel="AI · Easy"
          controlExtras={menuButtons()}
          matchStatus={
            <>
              <ModifierList view={view} opponentLabel="AI" />
              <ThinkIndicator thinking={false} />
            </>
          }
          boardNotice={
            <p className="notice practice-unearned" data-testid={practiceTestid.unearned} role="status">
              <span>This game won&apos;t earn anything.</span>
              <button type="button" aria-label="Dismiss">
                Dismiss
              </button>
            </p>
          }
          onOpenResult={() => setViewingBoard(false)}
          openResultTestId={practiceTestid.outcome}
        />
      </div>
    </div>
  );
}

function mountTable(handSize: number): void {
  const view = fullBoardView();
  const hand = HAND_CARD_IDS.slice(0, handSize).map((defId, index) => card({ defId, cost: index % 5 }));
  mountMidgame({ ...view, you: { ...view.you, hand } }, false);
}

/** B46: a hand card shows its top 24 px, its cost and name (mobile-ux.cy.tsx). */
const HAND_CARD_SHOWING_PX = 24;

function expectHandCardOnScreen(element: Element, label: string, width: number, height: number): void {
  const style = getComputedStyle(element);
  expect(style.display, `${label} is displayed`).to.not.eq("none");
  expect(style.visibility, `${label} is not hidden`).to.not.eq("hidden");
  const box = element.getBoundingClientRect();
  expect(box.width, `${label} has a width`).to.be.greaterThan(0);
  expect(box.height, `${label} has a height`).to.be.greaterThan(0);
  expect(box.left, `${label} starts inside the viewport`).to.be.at.least(-EPSILON);
  expect(box.top, `${label} starts inside the viewport`).to.be.at.least(-EPSILON);
  expect(box.right, `${label} ends inside ${String(width)} px`).to.be.at.most(width + EPSILON);
  expect(box.top + HAND_CARD_SHOWING_PX, `${label} shows its top inside ${String(height)} px`).to.be.at.most(
    height + EPSILON,
  );
}

function expectOnScreen(element: Element, label: string, width: number, height: number): void {
  const style = getComputedStyle(element);
  expect(style.display, `${label} is displayed`).to.not.eq("none");
  expect(style.visibility, `${label} is not hidden`).to.not.eq("hidden");
  const box = element.getBoundingClientRect();
  expect(box.width, `${label} has a width`).to.be.greaterThan(0);
  expect(box.height, `${label} has a height`).to.be.greaterThan(0);
  expect(box.left, `${label} starts inside the viewport`).to.be.at.least(-EPSILON);
  expect(box.top, `${label} starts inside the viewport`).to.be.at.least(-EPSILON);
  expect(box.right, `${label} ends inside ${String(width)} px`).to.be.at.most(width + EPSILON);
  expect(box.bottom, `${label} ends inside ${String(height)} px`).to.be.at.most(height + EPSILON);
}

describe("the practice table keeps the hand, the modifiers and the counters on screen", () => {
  for (const viewport of VIEWPORTS) {
    for (const handSize of HAND_SIZES) {
      const where = `${viewport.label} ${String(viewport.width)}x${String(viewport.height)}`;

      it(`${where}, ${String(handSize)} cards in hand`, () => {
        cy.viewport(viewport.width, viewport.height);
        mountTable(handSize);

        cy.get('[data-testid="board"]').should("be.visible");
        cy.get(".hand-you .hand-cards .card").should("have.length", handSize);

        cy.document().should((doc) => {
          const handCards = [...doc.querySelectorAll(".hand-you .hand-cards .card")];
          expect(handCards, "the hand is drawn").to.have.length(handSize);
          handCards.forEach((element, index) => {
            expectHandCardOnScreen(element, `hand card ${String(index + 1)} of ${String(handSize)}`, viewport.width, viewport.height);
          });
          for (let index = 0; index + 1 < handCards.length; index += 1) {
            const here = (handCards[index] as Element).getBoundingClientRect();
            const next = (handCards[index + 1] as Element).getBoundingClientRect();
            expect(next.left - here.left, `hand card ${String(index + 1)} shows a clickable edge`).to.be.at.least(12);
          }

          for (const side of ["you", "opponent"] as const) {
            const badges = doc.querySelector(`[data-testid="modifiers-${side}"]`);
            expect(badges, `modifiers-${side} is in the DOM`).to.not.eq(null);
            if (badges !== null) expectOnScreen(badges, `modifiers-${side}`, viewport.width, viewport.height);
          }

          const counters = [...doc.querySelectorAll(".field .card-unit .counter")];
          expect(counters, "the fixture's plague and grade counters on units").to.have.length(3);
          counters.forEach((element) => {
            expectOnScreen(element, `counter ${element.getAttribute("data-counter") ?? ""}`, viewport.width, viewport.height);
          });

          const buried = [...doc.querySelectorAll(".field .card-unit .buried-badge")];
          expect(buried, "the fixture's Stack pile").to.have.length(1);
          buried.forEach((element) => {
            expectOnScreen(element, "the Stack's buried count", viewport.width, viewport.height);
          });
        });
      });
    }
  }
});

describe("#2878 mid-game: no HUD above the board", () => {
  for (const viewport of VIEWPORTS) {
    const where = `${viewport.label} ${viewport.width}x${viewport.height}`;

    it(`draws nothing but gameplay above the board at ${where}`, () => {
      cy.viewport(viewport.width, viewport.height);
      mountMidgame(fullBoardView(), false);

      // No practice-hud element is rendered.
      cy.get('[data-testid="practice-hud"]').should("not.exist");
      cy.get('[data-testid="board"]').should("be.visible");

      cy.document().should((doc) => {
        const shell = doc.querySelector(".app-shell");
        expect(shell, "the shell is mounted").to.not.eq(null);
        if (shell === null) return;
        const shellBox = shell.getBoundingClientRect();
        const shellStyle = getComputedStyle(shell);
        const contentTop = shellBox.top + parseFloat(shellStyle.paddingTop);
        const board = doc.querySelector('[data-testid="board"]');
        expect(board, "the board is mounted").to.not.eq(null);
        if (board === null) return;
        const boardTop = board.getBoundingClientRect().top;

        // No visible element sits between the top of the shell and the board's top: every
        // in-flow element strictly above the board is at most a pixel tall (the visually-hidden
        // turn banner, the screen reader's live regions). Fixed layers (the effects, the drag
        // layer) and the hidden match menu are not in the flow.
        const between: string[] = [];
        doc.querySelectorAll("body *").forEach((candidate) => {
          const el = candidate as HTMLElement;
          if (candidate === doc.documentElement || candidate === doc.body || candidate === shell) return;
          const style = getComputedStyle(candidate);
          if (style.display === "none" || style.visibility === "hidden") return;
          if (style.position === "fixed") return;
          // Ancestors of the board span past it; only what ends above it counts.
          const box = candidate.getBoundingClientRect();
          if (box.width <= 1 || box.height <= 1) return;
          if (box.top >= contentTop - EPSILON && box.bottom <= boardTop + EPSILON) {
            between.push(`${el.tagName.toLowerCase()}${el.dataset.testid ?? ""}`);
          }
        });
        expect(between, `nothing visible sits above the board at ${where}`).to.deep.eq([]);

        // The opponent's hero starts right under the shell's safe-area padding.
        const hero = doc.querySelector('[data-testid="hero-opponent"]');
        expect(hero, "hero-opponent is mounted").to.not.eq(null);
        if (hero === null) return;
        expect(
          hero.getBoundingClientRect().top - contentTop,
          `hero-opponent starts within 24px of the shell's content top at ${where}`,
        ).to.be.at.most(24);

        // The turn banner stays in the DOM as the live region with no visible box.
        const banner = doc.querySelector('[data-testid="turn-banner"]');
        expect(banner, "the turn banner stays in the DOM").to.not.eq(null);
        if (banner === null) return;
        expect(banner.getAttribute("role")).to.eq("status");
        const bannerBox = banner.getBoundingClientRect();
        expect(bannerBox.width, "the turn banner has no visible box").to.be.at.most(1);
        expect(bannerBox.height, "the turn banner has no visible box").to.be.at.most(1);
      });

      // The enemy hero's seat names the AI and the difficulty, not OPPONENT.
      cy.get('[data-testid="hero-opponent"]').should("contain.text", "AI · Easy");
      cy.get('[data-testid="hero-you"]').should("contain.text", "You");

      // The modifier chip rides the control bar's status row.
      cy.get('[data-testid="modifier-list"]')
        .should("be.visible")
        .and(($chip) => {
          expect($chip.closest(".control-bar").length, "the modifier chip is in the control bar").to.be.greaterThan(0);
        });

      // B28: the document fits the viewport.
      cy.document().should((doc) => {
        expect(doc.documentElement.scrollWidth, `the document fits ${where}`).to.be.at.most(viewport.width);
        expect(doc.documentElement.scrollHeight, `the document fits ${where} vertically`).to.be.at.most(viewport.height);
      });
    });

    it(`homes New game and Menu at ${where}`, () => {
      cy.viewport(viewport.width, viewport.height);
      mountMidgame(fullBoardView(), false);
      cy.get('[data-testid="board"]').should("be.visible");

      cy.window().then((win) => {
        if (isPhoneLayout(win)) {
          // On the phone layouts New game and Menu are rows of the ⋯ match menu.
          cy.get('[data-testid="match-menu"]').should("be.visible").click();
          cy.get('[data-testid="match-menu-panel"]').should("be.visible");
          cy.get('[data-testid="practice-new-game"]')
            .should("be.visible")
            .and(($button) => {
              expect($button.closest('[data-testid="match-menu-panel"]').length, "New game is a menu row").to.be.greaterThan(0);
            });
          cy.get('[data-testid="practice-menu"]').should("be.visible");
        } else {
          // On desktop and tablet New game and Menu sit in the control bar beside Concede.
          cy.get('[data-testid="practice-new-game"]')
            .should("be.visible")
            .and(($button) => {
              expect($button.closest(".control-bar").length, "New game is in the control bar").to.be.greaterThan(0);
            });
          cy.get('[data-testid="practice-menu"]')
            .should("be.visible")
            .and(($button) => {
              expect($button.closest(".control-bar").length, "Menu is in the control bar").to.be.greaterThan(0);
            });
        }
      });
    });
  }
});

describe("#2878 mid-game thinking rides the control bar", () => {
  for (const viewport of [
    { label: "desktop", width: 1280, height: 720 },
    { label: "phone", width: 390, height: 844 },
  ] as const) {
    const where = `${viewport.label} ${viewport.width}x${viewport.height}`;

    it(`shows the thinking pill in the control bar and the ring on the enemy hero at ${where}`, () => {
      cy.viewport(viewport.width, viewport.height);
      mountMidgame(fullBoardView(), true);

      cy.get('[data-testid="practice-thinking"]')
        .should("be.visible")
        .and("have.attr", "role", "status")
        .and(($pill) => {
          expect($pill.closest(".control-bar").length, "the thinking pill is in the control bar").to.be.greaterThan(0);
        });
      cy.get(".practice-table").should("have.attr", "data-thinking", "true");
      cy.get('[data-testid="hero-opponent"]').should(($hero) => {
        const name = getComputedStyle($hero.get(0)).animationName;
        expect(name, "the enemy hero wears the thinking ring").to.contain("practice-hero-think");
      });
    });
  }
});

describe("#2878 game over: one outcome, in the board's chip", () => {
  for (const viewport of VIEWPORTS) {
    const where = `${viewport.label} ${viewport.width}x${viewport.height}`;

    it(`merges the outcome into the chip and clears the controls at ${where}`, () => {
      cy.viewport(viewport.width, viewport.height);
      cy.mount(<GameOverPractice />);

      // The route's result dialog opens first; View the board folds it to the chip.
      cy.get('[data-testid="practice-result"]').should("be.visible");
      cy.get('[data-testid="practice-view-board"]').click();
      cy.get('[data-testid="practice-result"]').should("not.exist");
      cy.get('[data-testid="result-overlay"]').should("be.visible").and("have.attr", "data-form", "chip");

      // The enemy hero's seat still names the AI.
      cy.get('[data-testid="hero-opponent"]').should("contain.text", "AI · Easy");

      // B28: the document fits the viewport.
      cy.document().should((doc) => {
        expect(doc.documentElement.scrollWidth, `the document fits ${where}`).to.be.at.most(viewport.width);
        expect(doc.documentElement.scrollHeight, `the document fits ${where} vertically`).to.be.at.most(viewport.height);
      });

      // Exactly one visible outcome word on screen, the chip's own.
      cy.document().should((doc) => {
        const words: string[] = [];
        doc.querySelectorAll("*").forEach((child) => {
          const el = child as HTMLElement;
          const style = getComputedStyle(child);
          if (style.display === "none" || style.visibility === "hidden") return;
          if (child.closest("[hidden]") !== null) return;
          const text = child.textContent ?? "";
          if (!OUTCOME_WORDS.test(text)) return;
          // Only the chip's own word element counts: ancestors (the chip, the game, the body)
          // contain it too.
          const direct = [...child.childNodes].some(
            (node) => node.nodeType === 3 && OUTCOME_WORDS.test(node.textContent ?? ""),
          );
          if (!direct) return;
          const own =
            el.tagName.toLowerCase() === "strong" &&
            el.classList.contains("result-overlay__word") &&
            child.closest('[data-testid="result-overlay"]') !== null;
          words.push(`${el.tagName.toLowerCase()}.${[...el.classList].join(".")}:${text.trim().slice(0, 24)}`);
          if (!own) words.push("NOT-THE-CHIP");
        });
        expect(words, `one visible outcome word at ${where}`).to.have.length(1);
      });

      // The chip overlaps no visible control in the control bar.
      cy.document().should((doc) => {
        const chip = doc.querySelector('[data-testid="result-overlay"]');
        expect(chip, "the chip is mounted").to.not.eq(null);
        if (chip === null) return;
        const chipBox = chip.getBoundingClientRect();
        expect(chipBox.width, "the chip has a box").to.be.greaterThan(0);
        const clashes: string[] = [];
        doc.querySelectorAll(".control-bar button, .control-bar [role='button'], .control-bar a").forEach((control) => {
          const style = getComputedStyle(control);
          if (style.display === "none" || style.visibility === "hidden") return;
          if (control.closest("[hidden]") !== null) return;
          const box = control.getBoundingClientRect();
          if (box.width === 0 || box.height === 0) return;
          const overlaps =
            box.left < chipBox.right - EPSILON &&
            box.right > chipBox.left + EPSILON &&
            box.top < chipBox.bottom - EPSILON &&
            box.bottom > chipBox.top + EPSILON;
          if (overlaps) {
            const el = control as HTMLElement;
            clashes.push(`${el.tagName.toLowerCase()}${el.dataset.testid ?? ""}`);
          }
        });
        expect(clashes, `the chip clears the control bar at ${where}`).to.deep.eq([]);
      });

      // Their hand opens R434's list, and the outcome button reopens the result dialog.
      cy.get('[data-testid="result-their-hand-open"]').should("be.visible").click();
      cy.get(`[data-testid="${INSPECT_LIST_SHEET}"]`).should("be.visible");
      cy.get("body").type("{esc}");
      cy.get('[data-testid="practice-outcome"]').should("be.visible").click();
      cy.get('[data-testid="practice-result"]').should("be.visible");
    });
  }
});
