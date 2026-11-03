// R2949000: a look with a picture of its own (pictures.ts): the Signature look draws its card's painter seeded
// apart from the card's normal picture, base and Radiant, on every card Signature is offered on. The faces
// wearing it are fullArtTiers.test.tsx's; the pixels are e2e's cypress/component/card-versions.cy.tsx.

import { createElement } from "react";
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { COSMETIC_OFFERS } from "../../wire/generated/cosmetics.ts";
import { AUGMENT_SIGNATURE_ID } from "../versions.ts";
import { CardArt } from "./CardArt.tsx";
import { artSeedOf, type ArtPicture } from "./pictures.ts";

afterEach(cleanup);

/** The still picture `id`'s full face draws for `picture`: its art's CSS background, and what it marks. */
function stillPicture(id: string, radiant: boolean, picture: ArtPicture): { background: string; marked: string | null } {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`the catalog has no ${id}`);
  const { container } = render(createElement(CardArt, { defId: id, radiant, tags: def.tags, type: def.type, shape: "full", name: def.name, picture }));
  const art = container.querySelector<HTMLElement>(".cf-art");
  const drawn = { background: art?.style.backgroundImage ?? "", marked: art?.getAttribute("data-art-picture") ?? null };
  cleanup();
  return drawn;
}

describe("R2949000 a look's picture of its own", () => {
  it("R2949000 artSeedOf seeds a card's own picture by its id alone", () => {
    expect(artSeedOf("core-022", "normal")).toBe("core-022");
    expect(artSeedOf("core-022", "signature")).not.toBe("core-022");
    expect(artSeedOf("core-022", "signature")).not.toBe(artSeedOf("core-083", "signature"));
    // No catalog id is another card's picture's seed.
    for (const id of Object.keys(CATALOG)) expect(CATALOG[artSeedOf(id, "signature")], id).toBeUndefined();
  });

  it("R2949000 every card offered Signature draws a still picture of its own, base and Radiant", () => {
    const offered = COSMETIC_OFFERS[AUGMENT_SIGNATURE_ID] ?? [];
    expect(offered.length).toBeGreaterThan(0);
    for (const id of offered) {
      for (const radiant of [false, true]) {
        const where = `${id} ${radiant ? "Radiant" : "base"}`;
        const normal = stillPicture(id, radiant, "normal");
        const signature = stillPicture(id, radiant, "signature");
        expect(normal.background, where).not.toBe("");
        expect(signature.background, where).not.toBe("");
        expect(signature.background, where).not.toBe(normal.background);
        expect(normal.marked, where).toBeNull();
        expect(signature.marked, where).toBe("signature");
      }
    }
  });
});
