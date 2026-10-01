// B3.4 rule 5, R386: a card's tunable numbers are written `{key}` (or `{key|singular|plural}`) in the
// catalog's face texts, and the client fills every one in before a player reads it: from the face's
// printed values in the collection, from the numbers the view says the card has now in play. A raw
// placeholder must never reach the screen, so every catalog face goes through each text path the
// client draws with, and none of them may still hold a "{".

import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { paramPlaceholders, type CardDef, type CardView } from "@jackioh/shared";

import { lookupFromDefs } from "../game/catalog.ts";
import { liveFace } from "../game/faces.ts";
import { searchMatches } from "../game/deckbuilder/filters.ts";
import { CardFace } from "./CardFace.tsx";
import { CardDetail } from "./inspect/CardDetail.tsx";
import { faceModel } from "./model.ts";
import { CardDefsProvider } from "./refContext.tsx";

afterEach(cleanup);

const DEFS: readonly CardDef[] = Object.values(CATALOG);
const FACES = ["base", "radiant"] as const;

/** The cards whose texts write a placeholder on either face. */
const TUNABLE: readonly CardDef[] = DEFS.filter((def) =>
  FACES.some((face) => paramPlaceholders(def[face].text).length > 0),
);

/** Each of a card's declared numbers moved one past its printed value, as a Degrade or an Upgrade might. */
function moved(def: CardDef, face: (typeof FACES)[number]): Record<string, number> {
  return Object.fromEntries((def.params ?? []).map((param) => [param.key, param[face] + 1]));
}

function inHand(def: CardDef, radiant: boolean, params?: Record<string, number>): CardView {
  return {
    instanceId: `h-${def.id}`,
    defId: def.id,
    radiant,
    cost: typeof def.cost === "number" ? def.cost : 0,
    ...(params === undefined ? {} : { params }),
  };
}

describe("R386 no placeholder reaches the screen", () => {
  it("R386 the catalog writes placeholders on many faces, so the checks below have something to catch", () => {
    expect(TUNABLE.length).toBeGreaterThan(50);
    expect(TUNABLE.some((def) => FACES.some((face) => paramPlaceholders(def[face].text).some((p) => p.one !== undefined))), "a plural placeholder").toBe(true);
  });

  it("R386 every face the collection prints fills its placeholders, both faces", () => {
    for (const def of DEFS) {
      for (const face of FACES) {
        const model = faceModel({ defId: def.id, def, radiant: face === "radiant" });
        expect(model.text.full, `${def.id} ${face}`).not.toContain("{");
      }
    }
  });

  it("R386 every catalog card face, drawn whole, shows no brace", () => {
    for (const def of DEFS) {
      for (const radiant of [false, true]) {
        const { container, unmount } = render(
          <CardDefsProvider defs={CATALOG}>
            <CardFace face={faceModel({ defId: def.id, def, radiant })} layout="full" />
          </CardDefsProvider>,
        );
        expect(container.textContent, `${def.id} radiant=${String(radiant)}`).not.toContain("{");
        unmount();
      }
    }
  }, 60_000);

  it("R386 the collection's detail view of every tunable card shows no brace", () => {
    for (const def of TUNABLE) {
      const { unmount } = render(
        <CardDefsProvider defs={CATALOG}>
          <CardDetail def={def} onClose={() => undefined} />
        </CardDefsProvider>,
      );
      expect(document.body.textContent, def.id).not.toContain("{");
      unmount();
    }
  }, 60_000);

  it("R386 in play a card's text reads the numbers the view gives it, singular and plural agreeing", () => {
    for (const def of TUNABLE) {
      for (const face of FACES) {
        const values = moved(def, face);
        const info = lookupFromDefs(CATALOG)(def.id, face === "radiant");
        if (info === undefined) throw new Error(`${def.id} should be in the catalog`);
        expect(info.text, `${def.id} ${face} lookup text`).not.toContain("{");
        const model = liveFace(info, inHand(def, face === "radiant", values));
        expect(model.text.full, `${def.id} ${face}`).not.toContain("{");
        for (const placeholder of paramPlaceholders(def[face].text)) {
          const value = values[placeholder.key];
          if (value === undefined) continue;
          const word = placeholder.one === undefined ? "" : ` ${value === 1 ? placeholder.one : (placeholder.many ?? "")}`;
          expect(model.text.full, `${def.id} ${face} ${placeholder.key}`).toContain(`${String(value)}${word}`);
        }
        // The collection's words stay beside a face whose numbers moved, for the inspect overlays.
        if (Object.keys(values).length > 0 && model.text.full !== faceModel({ defId: def.id, def, radiant: face === "radiant" }).text.full) {
          expect(model.printed?.full, `${def.id} ${face} printed`).not.toContain("{");
        }
      }
    }
  });

  it("R386 the deck builder's search finds a card by its filled-in words and never by a brace", () => {
    for (const def of DEFS) expect(searchMatches(def, "{"), def.id).toBe(false);
  });
});
