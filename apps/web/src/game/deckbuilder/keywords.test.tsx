// Issue #37, "fix keywords not rendering in the deckbuilder". Every keyword a card's face carries is
// printed in its text (the catalog writes it out, R366) and drawn there as a glossary term in bold
// (`.cf-term`, rules.ts), with a glossary row since patch v0.2.0 added its terms (R512). This holds the
// deck builder's own pool grid, and the Radiant face its detail pages to, to that for every card in
// the catalog, so a keyword the tokenizer does not know cannot go back to plain words there.

import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG, CATALOG_VERSION } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import { CardFace, faceModel } from "../../cards/index.ts";
import PoolGrid from "./PoolGrid.tsx";
import { poolCardId } from "./testids.ts";

afterEach(cleanup);

const DEFS: readonly CardDef[] = Object.values(CATALOG);

/** The glossary terms a rendered face draws, by term id. */
function termsIn(element: Element): Set<string> {
  return new Set([...element.querySelectorAll(".cf-term[data-term]")].map((term) => term.getAttribute("data-term") ?? ""));
}

describe("#37 every keyword renders in the deck builder", () => {
  it("the pool grid's face of every card draws each of its base keywords as a glossary term", () => {
    const keyworded = DEFS.filter((def) => def.base.keywords.length > 0);
    const view = render(
      <PoolGrid ids={keyworded.map((def) => def.id)} catalog={{ version: CATALOG_VERSION, cards: CATALOG }} onInspect={() => undefined} />,
    );
    const missing: string[] = [];
    for (const def of keyworded) {
      const tile = view.getByTestId(poolCardId(def.id));
      const drawn = termsIn(tile);
      for (const keyword of def.base.keywords) if (!drawn.has(keyword.kind)) missing.push(`${def.id} ${keyword.kind}`);
    }
    expect(missing).toEqual([]);
    // The control: the catalog has keywords to draw.
    expect(keyworded.flatMap((def) => def.base.keywords).length).toBeGreaterThan(50);
    // Some 80 full faces in jsdom, as CardDetail.test.tsx's every-card test budgets for its 317.
  }, 30_000);

  it("the Radiant face of every card draws each of its keywords as a glossary term", () => {
    const keyworded = DEFS.filter((def) => def.radiant.keywords.length > 0);
    const view = render(
      <>
        {keyworded.map((def) => (
          <div key={def.id} data-testid={`radiant-${def.id}`}>
            <CardFace face={faceModel({ defId: def.id, def, radiant: true })} layout="full" />
          </div>
        ))}
      </>,
    );
    const missing: string[] = [];
    for (const def of keyworded) {
      const drawn = termsIn(view.getByTestId(`radiant-${def.id}`));
      for (const keyword of def.radiant.keywords) if (!drawn.has(keyword.kind)) missing.push(`${def.id} ${keyword.kind}`);
    }
    expect(missing).toEqual([]);
  }, 30_000);
});
