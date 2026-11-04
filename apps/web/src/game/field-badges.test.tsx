// #2932: on a phone held upright a field card's Activate control and keyword marks hang off its
// top-left corner (FieldBadges.tsx), as siblings of the card that DEF never rotates; everywhere else
// they stay on the face. The phone is `stubPhonePortrait`: jsdom has no media queries.

import { CATALOG } from "@jackioh/cards";
import type { ActivationView, PlayerView } from "@jackioh/shared";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

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
});

/** `b1` a unit with an Activate ability and two keywords that draw a mark; `t1` one that has only Taunt (drawn as a frame). */
function view(position: "ATK" | "DEF" = "ATK"): PlayerView {
  return baseView({
    you: emptySide("p1", {
      units: [
        unit("p1", {
          instanceId: "b1",
          defId: "classicplus-076-1",
          position,
          keywords: [{ kind: "Rush" }, { kind: "Can't Defend" }],
          activations: [PING],
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

function renderGame(position: "ATK" | "DEF" = "ATK"): void {
  render(
    <CatalogContext.Provider value={lookup}>
      <Game view={view(position)} legal={[]} onAction={() => undefined} />
    </CatalogContext.Provider>,
  );
}

const el = (id: string): HTMLElement => screen.getByTestId(id);

describe("#2932 a phone's field unit hangs its Activate control and keyword marks off its corner", () => {
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

  it("a unit with nothing to hang (only Taunt, drawn as a frame) gets no stack", () => {
    restore = stubPhonePortrait(true);
    renderGame();
    expect(el("card-t1").parentElement?.querySelector(".field-badges")).toBeNull();
  });

  it("a face-up card in the backrow hangs its Activate control the same way", () => {
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
