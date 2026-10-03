// The deck builder's pool grid prints every card's rules text with its keywords in bold (issue #85).
//
// The grid draws each entry as a full `CardFace`, so the text is the face's own (CardFace.test.tsx
// holds the face alone); what this proves is the grid itself, over the whole catalog at once: Core,
// Classic, Classic+ and the tokens the Almanac browses through the same grid (R630). jsdom has no
// layout, so whether the rules box fits is the pixel specs' (e2e/cypress/component/deckbuilder-layout.cy.tsx).

import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { fillParams } from "@jackioh/shared";
import type { CatalogSnapshot } from "@jackioh/validator";

import PoolGrid from "./PoolGrid.tsx";

afterEach(cleanup);

const catalog: CatalogSnapshot = { version: "test", cards: CATALOG };
const IDS = Object.keys(CATALOG);
/** The terms patch v0.2.0 brought (R384–R386, R383). */
const V020_TERMS = ["Animated", "Activate", "Brittle", "Degrade", "Upgrade"] as const;

/** The whole catalog in one read-only grid. */
function renderPool(): Map<string, HTMLElement> {
  const { container } = render(<PoolGrid ids={IDS} catalog={catalog} onInspect={() => undefined} />);
  const faces = new Map<string, HTMLElement>();
  for (const id of IDS) {
    const face = container.querySelector<HTMLElement>(`[data-card="${id}"] .db-card-face .cf`);
    if (face === null) throw new Error(`the pool grid drew no face for ${id}`);
    faces.set(id, face);
  }
  return faces;
}

describe("the deck builder's pool grid prints rules text", () => {
  it("draws every card as a full face whose rules box holds its text, word for word", () => {
    for (const [id, face] of renderPool()) {
      expect(face.getAttribute("data-layout"), id).toBe("full");
      expect(face.querySelector(".card-text .cf-text-base")?.textContent, id).toBe(fillParams(CATALOG[id]!, "base"));
    }
  });

  it("bolds every keyword a card declares, in the text it prints", () => {
    for (const [id, face] of renderPool()) {
      const bold = new Set([...face.querySelectorAll(".card-text strong.cf-term")].map((term) => term.getAttribute("data-term")));
      for (const keyword of CATALOG[id]!.base.keywords) expect(bold.has(keyword.kind), `${id}: ${keyword.kind}`).toBe(true);
    }
  });

  it("bolds each of patch v0.2.0's keywords on at least one card, Classic and Classic+ included", () => {
    const bolded = new Map<string, Set<string>>();
    for (const [id, face] of renderPool()) {
      for (const term of face.querySelectorAll(".card-text strong.cf-term")) {
        const name = term.getAttribute("data-term") ?? "";
        const sets = bolded.get(name) ?? new Set<string>();
        sets.add(CATALOG[id]!.set);
        bolded.set(name, sets);
      }
    }
    for (const name of V020_TERMS) expect(bolded.get(name)?.size ?? 0, `${name} is bold on no card`).toBeGreaterThan(0);
    // The older sets print bold keywords too, not only the cards v0.2.0 added.
    for (const set of ["Core", "Classic", "Classic+"]) {
      expect([...bolded.values()].some((sets) => sets.has(set)), `no bold keyword on a ${set} card`).toBe(true);
    }
  });
});
