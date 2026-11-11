// #2932, #3257: on a phone held upright a field card's Activate control and keyword marks stand in the
// top-left corner of its own zone (FieldBadges.tsx), as siblings of the card that DEF never rotates;
// everywhere else they stay on the face. The marks that do not fit above the name plate fold into
// the "+n". The phone is `stubPhonePortrait`: jsdom has no media queries and no layout, so the fold is
// tested against a stubbed one (`stubLayout`); board-layout.cy.tsx measures the real one.

import { CATALOG } from "@jackioh/cards";
import type { ActivationView, PlayerView } from "@jackioh/shared";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { stubPhonePortrait } from "../test/phoneBar.ts";
import { baseView, emptySide, faceUpBackrow, unit } from "../test/fixtures.ts";
import Game from "./Game.tsx";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";

const lookup = lookupFromDefs(CATALOG);

const PING: ActivationView = { ability: "ping", label: "Deal {damage} damage.", usesLeft: 1, usable: true };

let restore: (() => void) | null = null;

afterEach(() => {
  cleanup();
  restore?.();
  restore = null;
  vi.restoreAllMocks();
});

/** `b1` a unit with an Activate ability and two keywords that draw a mark; `t1` one that has only Taunt (drawn as a frame). */
function view(position: "ATK" | "DEF" = "ATK", activate = true): PlayerView {
  return baseView({
    you: emptySide("p1", {
      units: [
        unit("p1", {
          instanceId: "b1",
          defId: "classicplus-076-1",
          position,
          keywords: [{ kind: "Rush" }, { kind: "Can't Defend" }],
          activations: activate ? [PING] : [],
        }),
        unit("p1", { instanceId: "t1", keywords: [{ kind: "Taunt" }] }),
        null,
        null,
        null,
      ],
      backrow: [faceUpBackrow("p1", { instanceId: "fs1", defId: "classic-020", activations: [PING] }), null, null, null, null],
    }),
    opponent: emptySide("p2", { hand: { count: 0 } }),
  });
}

function renderGame(position: "ATK" | "DEF" = "ATK", activate = true): void {
  render(
    <CatalogContext.Provider value={lookup}>
      <Game view={view(position, activate)} legal={[]} onAction={() => undefined} />
    </CatalogContext.Provider>,
  );
}

const el = (id: string): HTMLElement => screen.getByTestId(id);

describe("#2932 a phone's field unit stands its Activate control and keyword marks in its zone's corner", () => {
  it("the zone holds a `.field-badges` stack beside the card, and the card's face holds neither", () => {
    restore = stubPhonePortrait(true);
    renderGame();
    const card = el("card-b1");
    const zone = card.parentElement;
    expect(zone?.classList.contains("zone")).toBe(true);
    const badges = zone?.querySelector(":scope > .field-badges");
    expect(badges, "the zone holds the stack").not.toBeNull();
    expect(badges?.parentElement, "a sibling of the card, not inside it").toBe(zone);
    expect(card.contains(badges ?? null)).toBe(false);
    expect(badges?.querySelector(".activate-control"), "the Activate control is in the stack").not.toBeNull();
    expect([...(badges?.querySelectorAll(".keyword-icon") ?? [])].map((mark) => mark.getAttribute("data-keyword"))).toEqual([
      "Rush",
      "Can't Defend",
    ]);
    expect(card.querySelector(".activate-controls, .activate-control"), "none on the face").toBeNull();
    expect(card.querySelector(".keywords, .keyword-icon"), "no marks on the face").toBeNull();
  });

  it("R3339000 the stack's marks and its +n are images with accessible names, as the face's are", () => {
    restore = stubPhonePortrait(true);
    stubLayout("30.6px");
    renderGame();
    const more = document.querySelector<HTMLElement>(".field-badges .cf-kw-more");
    expect(more?.getAttribute("role")).toBe("img");
    expect(more?.getAttribute("aria-label")).toContain("Rush");
    expect(more?.getAttribute("aria-label")).toContain("Can't Defend");
    cleanup();
    stubLayout("60px");
    renderGame();
    const marks = [...document.querySelectorAll(".field-badges .keyword-icon")];
    expect(marks).toHaveLength(2);
    for (const mark of marks) {
      expect(mark.getAttribute("role")).toBe("img");
      expect(mark.getAttribute("aria-label")).toBeTruthy();
    }
  });

  it("the control still reports its press as the card's own Activate action", () => {
    restore = stubPhonePortrait(true);
    renderGame();
    const control = document.querySelector<HTMLElement>('.field-badges [data-testid="activate-b1"]');
    expect(control, "the control keeps its test id").not.toBeNull();
    expect(control?.getAttribute("data-activate-for")).toBe("b1");
  });

  it("a DEF unit's stack has no transform of its own while the card keeps its turn", () => {
    restore = stubPhonePortrait(true);
    renderGame("DEF");
    const card = el("card-b1");
    expect(card.style.transform).toBe("rotate(90deg) scale(var(--def-scale, 1))");
    const badges = card.parentElement?.querySelector<HTMLElement>(":scope > .field-badges");
    expect(badges, "the stack is there in DEF").not.toBeNull();
    expect(badges?.style.transform ?? "").toBe("");
    expect(badges?.closest('[data-position="DEF"]'), "no DEF card above it to turn it").toBeNull();
  });

  it("a unit with no mark to draw (only Taunt, drawn as a frame) gets no stack", () => {
    restore = stubPhonePortrait(true);
    renderGame();
    expect(el("card-t1").parentElement?.querySelector(".field-badges")).toBeNull();
  });

  it("a face-up card in the backrow stands its Activate control the same way", () => {
    restore = stubPhonePortrait(true);
    renderGame();
    const card = el("card-fs1");
    const badges = card.parentElement?.querySelector(":scope > .field-badges");
    expect(badges?.querySelector(".activate-control")).not.toBeNull();
    expect(card.querySelector(".activate-controls")).toBeNull();
  });

  it("off a phone portrait layout the control and the marks stay on the face and there is no stack", () => {
    restore = stubPhonePortrait(false);
    renderGame();
    const card = el("card-b1");
    expect(card.parentElement?.querySelector(".field-badges")).toBeNull();
    expect(card.querySelector(".activate-controls .activate-control")).not.toBeNull();
    expect(card.querySelectorAll(".keywords .keyword-icon")).toHaveLength(2);
    expect(el("card-fs1").querySelector(".activate-controls .activate-control")).not.toBeNull();
  });

  it("a card with no `onField` (a hand card) keeps both on its face even on a phone", () => {
    restore = stubPhonePortrait(true);
    const base = view();
    const hand = [{ instanceId: "h1", defId: "core-002", radiant: false, cost: 1 }];
    render(
      <CatalogContext.Provider value={lookup}>
        <Game view={{ ...base, you: { ...base.you, hand } }} legal={[]} onAction={() => undefined} />
      </CatalogContext.Provider>,
    );
    expect(screen.getByTestId("hand-card-h1").parentElement?.querySelector(".field-badges")).toBeNull();
  });
});

/**
 * jsdom has no layout: `maxHeight` is the stack's room above the name plate (board.css), and the control is 16px tall, a
 * mark 12px, and the gap 2px, as on a phone. The stack's marks are laid out one under the other.
 */
function stubLayout(maxHeight: string): void {
  const marks = (element: Element): Element[] => [...(element.closest(".field-badges")?.querySelectorAll(".activate-control, .keyword, .cf-kw-more") ?? [])];
  const heightOf = (element: Element): number => (element.matches(".activate-control") ? 16 : element.matches(".keyword, .cf-kw-more") ? 12 : 0);
  const computed = window.getComputedStyle.bind(window);
  vi.spyOn(window, "getComputedStyle").mockImplementation((element: Element, pseudo?: string | null) =>
    element.classList.contains("field-badges") ? ({ maxHeight } as CSSStyleDeclaration) : computed(element, pseudo),
  );
  vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(function (this: HTMLElement) {
    return heightOf(this);
  });
  vi.spyOn(HTMLElement.prototype, "offsetTop", "get").mockImplementation(function (this: HTMLElement) {
    const before = marks(this).indexOf(this);
    return marks(this)
      .slice(0, Math.max(before, 0))
      .reduce((top, mark) => top + heightOf(mark) + 2, 0);
  });
}

describe("#3257 the marks that do not fit above the name plate fold into the +n", () => {
  const keywordsOf = (): string[] => [...document.querySelectorAll(".field-badges .keyword-icon")].map((mark) => mark.getAttribute("data-keyword") ?? "");
  const more = (): HTMLElement | null => document.querySelector<HTMLElement>(".field-badges .cf-kw-more");

  it("every mark is drawn when the stack has the room", () => {
    restore = stubPhonePortrait(true);
    stubLayout("60px");
    renderGame();
    expect(keywordsOf()).toEqual(["Rush", "Can't Defend"]);
    expect(more(), "nothing folded").toBeNull();
  });

  it("a stack with room for the control and one mark only draws the +n", () => {
    restore = stubPhonePortrait(true);
    stubLayout("30.6px");
    renderGame();
    expect(keywordsOf(), "no named mark: the +n alone took the row under the control").toEqual([]);
    expect(more()?.textContent).toBe("+2");
    expect(more()?.getAttribute("title")).toContain("Rush");
    expect(document.querySelector(".field-badges .activate-control"), "the control stays").not.toBeNull();
  });

  it("a stack with no room for a second item keeps the Activate control and the +n, and draws no named mark", () => {
    restore = stubPhonePortrait(true);
    stubLayout("10px");
    renderGame();
    expect(keywordsOf()).toEqual([]);
    expect(more()?.textContent, "the +n is never dropped while a mark is to be drawn").toBe("+2");
    expect(more()?.getAttribute("title")).toContain("Rush");
    expect(document.querySelector(".field-badges .activate-control"), "the control stays").not.toBeNull();
  });
});

describe("#3315 a DEF unit's keywords never vanish from the board", () => {
  // The unit `b1`'s own stack: the face-up backrow card `fs1` has a control stack of its own.
  const stack = (): HTMLElement | null => el("card-b1").parentElement?.querySelector<HTMLElement>(":scope > .field-badges") ?? null;
  const keywordsOf = (): string[] => [...(stack()?.querySelectorAll(".keyword-icon") ?? [])].map((mark) => mark.getAttribute("data-keyword") ?? "");
  const more = (): HTMLElement | null => stack()?.querySelector<HTMLElement>(".cf-kw-more") ?? null;

  it("a DEF unit with Activate, over a band shorter than the control, keeps the control and a +n for both marks", () => {
    restore = stubPhonePortrait(true);
    stubLayout("3.7px");
    renderGame("DEF");
    expect(keywordsOf(), "no named mark fits").toEqual([]);
    expect(stack()?.querySelector(".activate-control"), "the control stays").not.toBeNull();
    expect(more()?.textContent).toBe("+2");
    expect(more()?.getAttribute("title")).toContain("Rush");
    expect(more()?.getAttribute("title")).toContain("Can't Defend");
  });

  it("a DEF unit with no Activate, over a band shorter than one mark, keeps the +n for both marks", () => {
    restore = stubPhonePortrait(true);
    stubLayout("3.7px");
    renderGame("DEF", false);
    expect(stack(), "the stack is still there").not.toBeNull();
    expect(stack()?.querySelector(".activate-control"), "no ability, no control").toBeNull();
    expect(keywordsOf()).toEqual([]);
    expect(more()?.textContent).toBe("+2");
    expect(more()?.getAttribute("title")).toContain("Rush");
    expect(more()?.getAttribute("title")).toContain("Can't Defend");
  });
});
