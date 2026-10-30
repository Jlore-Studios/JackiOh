// R2521009: a Resource's face prints what it sells for on the face shown, "Credits: <min>–<max>",
// read off its definition's `resource` (R1840000) and nothing else (CLAUDE.md rule 7): the base face
// its `base` prices, the Radiant face its `radiant` ones. A card that is no Resource prints no such
// line. Every face is CardFace's, so the hand, the inspect views, the Almanac and the deck builder
// all print it.

import { cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import { CardFace } from "./CardFace.tsx";
import { flushFits } from "./fit.ts";
import { creditsLine, faceModel } from "./model.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "./settings.ts";
import { resetLocaleForTests, setLocale } from "../i18n/i18n.ts";

/** Stinkyscape's Korrelite, a Common Resource: base 2–5, Radiant 4–10. */
const KORRELITE = "stinkyscape-t-korrelite";
/** Core #2, a Unit and no Resource. */
const NO_RESOURCE = "core-002";

beforeEach(() => {
  // R1017001: a full-art face prints none of its words; these read them.
  writeCardSettings({ fullArt: false });
});

afterEach(() => {
  cleanup();
  writeCardSettings(CARD_SETTINGS_DEFAULTS);
  resetLocaleForTests();
});

function def(id: string): CardDef {
  const found = CATALOG[id];
  if (found === undefined) throw new Error(`the catalog has no ${id}`);
  return found;
}

/** The Credits line a card's full face prints on one face, or null when it prints none. */
function printedCredits(id: string, radiant: boolean): string | null {
  const { container } = render(<CardFace face={faceModel({ defId: id, def: def(id), radiant })} />);
  flushFits();
  const line = container.querySelector(".card-text .cf-text-credits");
  const text = line === null ? null : line.textContent;
  cleanup();
  return text;
}

describe("R2521009 a Resource's face prints its Credits", () => {
  it("R2521009 Korrelite's base face prints Credits: 2–5 and its Radiant face Credits: 4–10, its definition's prices", () => {
    const korrelite = def(KORRELITE);
    expect(korrelite.resource?.base).toMatchObject({ sellMin: 2, sellMax: 5 });
    expect(korrelite.resource?.radiant).toMatchObject({ sellMin: 4, sellMax: 10 });
    expect(faceModel({ defId: KORRELITE, def: korrelite, radiant: false }).credits).toEqual({ min: 2, max: 5 });
    expect(faceModel({ defId: KORRELITE, def: korrelite, radiant: true }).credits).toEqual({ min: 4, max: 10 });
    expect(printedCredits(KORRELITE, false)).toBe("Credits: 2–5");
    expect(printedCredits(KORRELITE, true)).toBe("Credits: 4–10");
  });

  it("R2521009 the line rides the face in a game too, a Resource in its owner's hand", () => {
    const korrelite = def(KORRELITE);
    const inHand = faceModel({ defId: KORRELITE, def: korrelite, radiant: false, liveCost: 0, inPlay: {} });
    expect(inHand.inPlay).toBe(true);
    expect(creditsLine(inHand)).toBe("Credits: 2–5");
  });

  it("R2521009 a card that is no Resource prints no Credits line on either face", () => {
    const card = def(NO_RESOURCE);
    expect(card.resource).toBeUndefined();
    for (const radiant of [false, true]) {
      const face = faceModel({ defId: NO_RESOURCE, def: card, radiant });
      expect(face.credits).toBeNull();
      expect(creditsLine(face)).toBe("");
      expect(printedCredits(NO_RESOURCE, radiant)).toBeNull();
    }
    // A card nobody can name has no definition, so no prices either.
    expect(faceModel({ defId: "core-999", radiant: false }).credits).toBeNull();
  });

  it("R2521009 every catalog card's faces print its own prices when it has some, and none when it has not", () => {
    for (const card of Object.values(CATALOG)) {
      for (const radiant of [false, true]) {
        const price = card.resource === undefined ? undefined : radiant ? card.resource.radiant : card.resource.base;
        const line = creditsLine(faceModel({ defId: card.id, def: card, radiant }));
        expect(line, `${card.id} ${radiant ? "radiant" : "base"}`).toBe(
          price === undefined ? "" : `Credits: ${String(price.sellMin)}–${String(price.sellMax)}`,
        );
      }
    }
  });

  it("R2521009 a Chinese reader reads the line in Chinese, with the same numbers", async () => {
    await setLocale("zh-CN");
    expect(creditsLine(faceModel({ defId: KORRELITE, def: def(KORRELITE), radiant: true }))).toBe("信用点：4–10");
  });
});
