// R2949000: a look with a picture of its own (pictures.ts): the Signature look draws its card's painter seeded
// apart from the card's normal picture, base and Radiant, on every card Signature is offered on. The faces
// wearing it are fullArtTiers.test.tsx's; the pixels are e2e's cypress/component/card-versions.cy.tsx.
// R2940001: Alt art's picture, seeded apart as Signature's is, and the Sketch, the normal picture in pencil; the
// faces wearing them are versionLooks.test.tsx's.

import { createElement } from "react";
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { COSMETIC_OFFERS } from "../../wire/generated/cosmetics.ts";
import { artMotionOf } from "../fullArt.ts";
import { AUGMENT_SIGNATURE_ID } from "../versions.ts";
import { CardArt, type CardArtProps } from "./CardArt.tsx";
import { artSeedOf, sketchSvg, type ArtPicture } from "./pictures.ts";

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

/** What `id`'s art window draws for `picture`: its CSS background decoded, and what it marks. */
function drawnArt(id: string, radiant: boolean, picture: ArtPicture, more: Partial<CardArtProps> = {}): Record<string, string | null> {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`the catalog has no ${id}`);
  const { container } = render(createElement(CardArt, { defId: id, radiant, tags: def.tags, type: def.type, shape: "full", name: def.name, picture, ...more }));
  const art = container.querySelector<HTMLElement>(".cf-art");
  const drawn = {
    picture: decodeURIComponent(art?.style.backgroundImage ?? ""),
    art: art?.getAttribute("data-art") ?? null,
    marked: art?.getAttribute("data-art-picture") ?? null,
    motif: art?.getAttribute("data-art-motif") ?? null,
    family: art?.getAttribute("data-art-family") ?? null,
  };
  cleanup();
  return drawn;
}

/** The markup inside a picture's one `<svg>`, its root unwrapped. */
function inner(picture: string): string {
  const svg = picture.slice(picture.indexOf("<svg"), picture.lastIndexOf("</svg>") + "</svg>".length);
  return svg.replace(/^<svg[^>]*>/, "").replace(/<\/svg>$/, "");
}

describe("R2940001 Alt art's picture and the Sketch", () => {
  it("R2940001 artSeedOf seeds Alt art apart from the normal and Signature pictures, and the Sketch by the card's own id", () => {
    expect(artSeedOf("core-022", "alt")).not.toBe("core-022");
    expect(artSeedOf("core-022", "alt")).not.toBe(artSeedOf("core-022", "signature"));
    expect(artSeedOf("core-022", "sketch")).toBe("core-022");
    for (const id of Object.keys(CATALOG)) expect(CATALOG[artSeedOf(id, "alt")], id).toBeUndefined();
  });

  /**
   * One card of each painter: Mr. Vanilla (a procedural Unit), Transmogulate (a procedural Spell), Dice of Doom
   * (Stinkyscape) and Goetzmann Game Design (Training Grounds).
   */
  const PAINTED = ["core-008", "core-083", "stinkyscape-154", "traininggrounds-040"] as const;

  it("R2940001 Alt art keeps the subject in a new scene and the Sketch pencils the normal picture, base and Radiant, on every painter", () => {
    for (const id of PAINTED) {
      for (const radiant of [false, true]) {
        const where = `${id} ${radiant ? "Radiant" : "base"}`;
        const normal = drawnArt(id, radiant, "normal");
        const alt = drawnArt(id, radiant, "alt");
        const sketch = drawnArt(id, radiant, "sketch");
        const signature = drawnArt(id, radiant, "signature");
        // Three pictures apart, and Alt art's none of the Signature look's.
        expect(new Set([normal.picture, alt.picture, sketch.picture, signature.picture]).size, where).toBe(4);
        // Alt art keeps what the card's name, tags and type pick: its motif and its family.
        expect([alt.motif, alt.family], where).toEqual([normal.motif, normal.family]);
        expect([normal.marked, alt.marked, sketch.marked], where).toEqual([null, "alt", "sketch"]);
        // The Sketch draws the normal picture itself, under its pencil filter, in one <svg> with no reference out.
        const pencil = sketch.picture ?? "";
        expect(pencil, where).toContain('filter id="sk-pencil"');
        expect(pencil, where).toContain('filter="url(#sk-pencil)"');
        expect(pencil, where).toContain(inner(normal.picture ?? ""));
        expect(pencil.match(/<svg[\s>]/g) ?? [], where).toHaveLength(1);
        for (const banned of ["href", "<text", "<title", "NaN", "undefined"]) expect(pencil, `${where} ${banned}`).not.toContain(banned);
        // R2725004: a Radiant face's painted picture washes back over its Sketch; a base face's never.
        expect(pencil.includes('result="sk-wash"'), where).toBe(radiant);
      }
    }
  });

  it("R2940001 a Legacy card keeps its one picture for Alt art, and the Sketch pencils it and a Starship's", () => {
    // Legacy: Duplicating Felinors wears its Core card's picture (R2724000), which no Alt art offer takes.
    const LEGACY = "traininggrounds-044";
    const normal = drawnArt(LEGACY, false, "normal");
    const alt = drawnArt(LEGACY, false, "alt");
    expect(alt.picture).toBe(normal.picture);
    expect(alt.marked).toBeNull();
    const sketch = drawnArt(LEGACY, false, "sketch");
    expect(sketch.family).toBe("legacy");
    expect(sketch.marked).toBe("sketch");
    expect(sketch.picture).toContain(inner(normal.picture ?? ""));
    expect(sketch.picture).toContain('filter id="sk-pencil"');
    // A Starship in play draws its pieces; its Sketch pencils them.
    const ship = { starship: { pieces: 2, launched: false } };
    const flying = drawnArt("core-008", true, "normal", ship);
    const pencilled = drawnArt("core-008", true, "sketch", ship);
    expect(pencilled.family).toBe("starship");
    expect(pencilled.picture).toContain(inner(flying.picture ?? ""));
    expect(pencilled.picture).toContain('result="sk-wash"');
  });

  it("R2940001 a delivered file is the normal picture's: Alt art and the Sketch draw the painter's", () => {
    const id = "core-008";
    const manifest = { [id]: { base: true } } as const;
    expect(drawnArt(id, false, "normal", { manifest }).art).toBe("real");
    // R2949000: the Signature look keeps a delivered file, as it did.
    expect(drawnArt(id, false, "signature", { manifest }).art).toBe("real");
    for (const picture of ["alt", "sketch"] as const) {
      const drawn = drawnArt(id, false, picture, { manifest });
      expect(drawn.art, picture).toBe("procedural");
      expect(drawn.picture, picture).toBe(drawnArt(id, false, picture).picture);
    }
  });

  it("R2940002 an animated Sketch moves its pencilled picture, and an animated Alt art its own", () => {
    // Mr. Vanilla's own motion (one of the 21).
    const id = "core-008";
    const motion = artMotionOf(id);
    expect(motion).not.toBeNull();
    for (const picture of ["sketch", "alt"] as const) {
      const still = drawnArt(id, false, picture);
      const moving = drawnArt(id, false, picture, { motion });
      expect(moving.picture, picture).toContain("art-m-");
      expect(moving.picture, picture).not.toBe(still.picture);
      expect(moving.marked, picture).toBe(picture);
    }
    expect(drawnArt(id, false, "sketch", { motion }).picture).toContain('filter id="sk-pencil"');
  });

  it("R2940001 sketchSvg seeds its strokes by the card and draws the same for the same card", () => {
    const picture = '<svg viewBox="0 0 100 100"><rect width="100" height="100" fill="#123456"/></svg>';
    expect(sketchSvg(picture, false, "core-008")).toBe(sketchSvg(picture, false, "core-008"));
    expect(sketchSvg(picture, false, "core-008")).not.toBe(sketchSvg(picture, false, "core-083"));
    expect(sketchSvg(picture, false, "core-008")).toContain('<rect width="100" height="100" fill="#123456"/>');
    expect(sketchSvg(picture, false, "core-008")).not.toContain('viewBox="0 0 100 100"><rect');
  });
});
