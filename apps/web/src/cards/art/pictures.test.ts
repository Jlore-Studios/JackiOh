// R3015002 (#3015): every level of a variant draws its first level's picture, so no look has a picture of its own
// past Alt art's: a signed level draws its variant's own (R2949000's Signature picture is gone). The faces wearing
// them are fullArtTiers.test.tsx's and variantLevels.test.tsx's; the pixels are e2e's
// cypress/component/variant-ladders.cy.tsx. R2940001: Alt art's picture, seeded apart from the normal one, and the
// Sketch, the normal picture in pencil; the faces wearing them are versionLooks.test.tsx's. R2942001: Pixel's and Comic's, the normal picture restyled; the
// faces wearing them are versionLooks3.test.tsx's. R2943001: Set showcase's, the normal picture in its set's treatment,
// and Out of frame's, the picture's backdrop and subject drawn apart; the faces wearing them are versionLooks4.test.tsx's.
// R2944001: Immersive's, the picture's backdrop hazed as far off, and Prismatic's, the normal picture in chrome under a
// sheen; the faces wearing them are versionLooks5.test.tsx's. R3017002: part 6's, four repaints of a picture of their own
// (Night, Watercolor, Stained-glass window, Tarot card) and two restyles of the normal one (Woodcut, Neon sign); the
// faces wearing them are versionLooks6.test.tsx's.

import { createElement } from "react";
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { SHIPPED_SETS } from "@jackioh/shared";

import { COSMETIC_LEVELS, COSMETIC_OFFERS } from "../../wire/generated/cosmetics.ts";
import { artMotionOf } from "../fullArt.ts";
import { VERSION_ALT_ART_ID, VERSION_FULL_ART_ID, VERSION_FULL_ART_SIGNED_ID, artPictureOf } from "../versions.ts";
import { CardArt, hasArtParts, type CardArtProps } from "./CardArt.tsx";
import {
  SHOWCASE_FAMILIES,
  artSeedOf,
  comicSvg,
  depthSvg,
  isRepaint,
  neonSvg,
  nightSvg,
  pixelSvg,
  prismaticSvg,
  restyledSvg,
  restyleOf,
  showcaseFamilyOf,
  showcaseSvg,
  sketchSvg,
  stainedGlassSvg,
  tarotSvg,
  watercolorSvg,
  woodcutSvg,
  type ArtPicture,
  type ArtRestyle,
  type ShowcaseFamily,
} from "./pictures.ts";

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

describe("R3015002 no level's picture of its own", () => {
  it("R3015002 R3017002 artSeedOf seeds a card's normal picture by its id alone, and no level seeds one past Alt art's and part 6's repaints", () => {
    expect(artSeedOf("core-022", "normal")).toBe("core-022");
    // Every level of every variant draws the normal picture, a restyle of it (seeded by the card's id), Alt art's or a
    // part 6 repaint's, each seeded by the card's id and its name.
    const seeds = new Set(Object.keys(COSMETIC_LEVELS).map((id) => artSeedOf("core-022", artPictureOf(id, "Core"))));
    const repaints = ["night", "watercolor", "stained-glass", "tarot"].map((name) => `core-022#${name}`);
    expect([...seeds].sort()).toEqual(["core-022", artSeedOf("core-022", artPictureOf(VERSION_ALT_ART_ID)), ...repaints].sort());
    // No catalog id is another card's picture's seed.
    for (const id of Object.keys(CATALOG)) expect(CATALOG[artSeedOf(id, "alt")], id).toBeUndefined();
  });

  it("R3015002 every card offered Full art's signed level draws its normal picture there, base and Radiant, as at Full art's first level", () => {
    const offered = COSMETIC_OFFERS[VERSION_FULL_ART_SIGNED_ID] ?? [];
    expect(offered.length).toBeGreaterThan(0);
    expect(artPictureOf(VERSION_FULL_ART_SIGNED_ID)).toBe("normal");
    for (const id of offered) {
      for (const radiant of [false, true]) {
        const where = `${id} ${radiant ? "Radiant" : "base"}`;
        const first = stillPicture(id, radiant, artPictureOf(VERSION_FULL_ART_ID));
        const signed = stillPicture(id, radiant, artPictureOf(VERSION_FULL_ART_SIGNED_ID));
        expect(first.background, where).not.toBe("");
        expect(signed.background, where).toBe(first.background);
        expect(first.marked, where).toBeNull();
        expect(signed.marked, where).toBeNull();
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
    part: art?.getAttribute("data-art-part") ?? null,
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
  it("R2940001 R3015002 artSeedOf seeds Alt art apart from the normal picture, and the Sketch by the card's own id", () => {
    expect(artSeedOf("core-022", "alt")).not.toBe("core-022");
    expect(artSeedOf("core-022", "alt")).not.toBe(artSeedOf("core-083", "alt"));
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
        // Three pictures apart (R3015002: and no Signature picture beside them).
        expect(new Set([normal.picture, alt.picture, sketch.picture]).size, where).toBe(3);
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
    // R3015002: a signed level draws its variant's picture, Full art's the normal one, so it keeps a delivered file.
    expect(drawnArt(id, false, artPictureOf(VERSION_FULL_ART_SIGNED_ID), { manifest }).art).toBe("real");
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

describe("R2942001 Pixel's and Comic's pictures", () => {
  it("R2942001 Pixel and Comic restyle the normal picture: seeded by the card's own id, each its own restyle", () => {
    for (const picture of ["pixel", "comic"] as const) expect(artSeedOf("core-022", picture), picture).toBe("core-022");
    expect((["normal", "alt", "sketch", "pixel", "comic"] as const).map(restyleOf)).toEqual([null, null, "sketch", "pixel", "comic"]);
  });

  /** One card of each painter, as R2940001's tests: two procedural cards, a Stinkyscape one and a Training Grounds one. */
  const PAINTED = ["core-008", "core-083", "stinkyscape-154", "traininggrounds-040"] as const;

  it("R2942001 Pixel and Comic draw the normal picture under their filters, apart from every other picture, base and Radiant, on every painter", () => {
    for (const id of PAINTED) {
      for (const radiant of [false, true]) {
        const where = `${id} ${radiant ? "Radiant" : "base"}`;
        const normal = drawnArt(id, radiant, "normal");
        const sketch = drawnArt(id, radiant, "sketch");
        const pixel = drawnArt(id, radiant, "pixel");
        const comic = drawnArt(id, radiant, "comic");
        expect(new Set([normal.picture, sketch.picture, pixel.picture, comic.picture]).size, where).toBe(4);
        expect([pixel.marked, comic.marked], where).toEqual(["pixel", "comic"]);
        // The subject is the normal picture's: its motif and family.
        for (const drawn of [pixel, comic]) expect([drawn.motif, drawn.family], where).toEqual([normal.motif, normal.family]);
        for (const [drawn, filter] of [
          [pixel.picture ?? "", "px-pixel"],
          [comic.picture ?? "", "pop-comic"],
        ] as const) {
          expect(drawn, `${where} ${filter}`).toContain(`filter id="${filter}"`);
          expect(drawn, `${where} ${filter}`).toContain(`filter="url(#${filter})"`);
          expect(drawn, `${where} ${filter}`).toContain(inner(normal.picture ?? ""));
          expect(drawn.match(/<svg[\s>]/g) ?? [], `${where} ${filter}`).toHaveLength(1);
          for (const banned of ["href", "<text", "<title", "NaN", "undefined"]) expect(drawn, `${where} ${filter} ${banned}`).not.toContain(banned);
        }
        // R2725004: a base face's Pixel cuts its colours to the palette, a Radiant one keeps its paint; a Radiant face's
        // painted picture washes back over its Comic, a base face's never.
        expect((pixel.picture ?? "").includes('result="px-palette"'), where).toBe(!radiant);
        expect((comic.picture ?? "").includes('result="pop-wash"'), where).toBe(radiant);
      }
    }
  });

  it("R2942001 a Legacy card's and a Starship's picture are restyled, and a delivered file gives way to the painter", () => {
    const LEGACY = "traininggrounds-044";
    const normal = drawnArt(LEGACY, false, "normal");
    const ship = { starship: { pieces: 2, launched: false } };
    const flying = drawnArt("core-008", true, "normal", ship);
    for (const picture of ["pixel", "comic"] as const) {
      const legacy = drawnArt(LEGACY, false, picture);
      expect(legacy.family, picture).toBe("legacy");
      expect(legacy.marked, picture).toBe(picture);
      expect(legacy.picture, picture).toContain(inner(normal.picture ?? ""));
      const restyledShip = drawnArt("core-008", true, picture, ship);
      expect(restyledShip.family, picture).toBe("starship");
      expect(restyledShip.picture, picture).toContain(inner(flying.picture ?? ""));
      expect(restyledShip.picture, picture).not.toBe(flying.picture);
    }
    const id = "core-008";
    const manifest = { [id]: { base: true } } as const;
    for (const picture of ["pixel", "comic"] as const) {
      const drawn = drawnArt(id, false, picture, { manifest });
      expect(drawn.art, picture).toBe("procedural");
      expect(drawn.picture, picture).toBe(drawnArt(id, false, picture).picture);
    }
  });

  it("R2942002 an animated Pixel or Comic moves its restyled picture", () => {
    const id = "core-008";
    const motion = artMotionOf(id);
    expect(motion).not.toBeNull();
    for (const picture of ["pixel", "comic"] as const) {
      const still = drawnArt(id, false, picture);
      const moving = drawnArt(id, false, picture, { motion });
      expect(moving.picture, picture).toContain("art-m-");
      expect(moving.picture, picture).not.toBe(still.picture);
      expect(moving.marked, picture).toBe(picture);
    }
  });

  it("R2942001 restyledSvg draws each restyle, the same for the same picture, and none takes a seed but the Sketch", () => {
    const picture = '<svg viewBox="0 0 100 100"><rect width="100" height="100" fill="#123456"/></svg>';
    for (const radiant of [false, true]) {
      expect(restyledSvg(picture, "sketch", radiant, "core-008")).toBe(sketchSvg(picture, radiant, "core-008"));
      expect(restyledSvg(picture, "pixel", radiant, "core-008")).toBe(pixelSvg(picture, radiant));
      expect(restyledSvg(picture, "comic", radiant, "core-008")).toBe(comicSvg(picture, radiant));
      expect(restyledSvg(picture, "pixel", radiant, "core-008")).toBe(restyledSvg(picture, "pixel", radiant, "core-083"));
      expect(restyledSvg(picture, "comic", radiant, "core-008")).toBe(restyledSvg(picture, "comic", radiant, "core-083"));
      for (const drawn of [pixelSvg(picture, radiant), comicSvg(picture, radiant)]) {
        expect(drawn).toContain('<rect width="100" height="100" fill="#123456"/>');
        expect(drawn).not.toContain('viewBox="0 0 100 100"><rect');
      }
    }
  });
});

describe("R2943001 Set showcase's and Out of frame's pictures", () => {
  /** Each family's filter (pictures.ts `showcaseSvg`). */
  const FILTERS: Readonly<Record<ShowcaseFamily, string>> = {
    core: "sc-core",
    classic: "sc-stone",
    meditative: "sc-ink",
    stinkyscape: "sc-space",
    "training-grounds": "sc-wood",
  };
  const FAMILIES = Object.keys(FILTERS) as ShowcaseFamily[];

  it("R2943001 every shipped set has a Set showcase family, Classic+ and Meditative+ their parents', and each restyles the normal picture seeded by the card's id", () => {
    for (const set of SHIPPED_SETS) expect(showcaseFamilyOf(set), set).not.toBeNull();
    expect(new Set(SHIPPED_SETS.map((set) => showcaseFamilyOf(set))).size).toBe(FAMILIES.length);
    expect([showcaseFamilyOf("Classic+"), showcaseFamilyOf("Meditative+")]).toEqual([showcaseFamilyOf("Classic"), showcaseFamilyOf("Meditative")]);
    expect([showcaseFamilyOf(null), showcaseFamilyOf("Boss"), showcaseFamilyOf("Boss-X")]).toEqual([null, null, null]);
    expect(new Set(Object.values(SHOWCASE_FAMILIES))).toEqual(new Set(FAMILIES));
    for (const family of FAMILIES) {
      const picture = `showcase-${family}` as const;
      expect(restyleOf(picture), family).toBe(picture);
      expect(artSeedOf("core-022", picture), family).toBe("core-022");
    }
  });

  /** One card of each shipped set: a procedural card of each of Core, Classic, Classic+, Meditative and Meditative+, a Stinkyscape one and a Training Grounds one. */
  const SHOWCASED = ["core-083", "classic-080", "classicplus-019", "meditative-087", "meditativeplus-024", "stinkyscape-154", "traininggrounds-040"] as const;

  it("R2943001 each card's Set showcase draws its normal picture under its set's filter, apart from every other picture, base and Radiant", () => {
    for (const id of SHOWCASED) {
      const family = showcaseFamilyOf(CATALOG[id]?.set ?? null);
      if (family === null) throw new Error(`${id} has no family`);
      const filter = FILTERS[family];
      for (const radiant of [false, true]) {
        const where = `${id} ${radiant ? "Radiant" : "base"}`;
        const normal = drawnArt(id, radiant, "normal");
        const showcase = drawnArt(id, radiant, `showcase-${family}`);
        const others = (["sketch", "pixel", "comic"] as const).map((picture) => drawnArt(id, radiant, picture).picture);
        const drawn = showcase.picture ?? "";
        expect(new Set([normal.picture, drawn, ...others]).size, where).toBe(5);
        expect(showcase.marked, where).toBe(`showcase-${family}`);
        expect([showcase.motif, showcase.family], where).toEqual([normal.motif, normal.family]);
        expect(drawn, where).toContain(`filter id="${filter}"`);
        expect(drawn, where).toContain(`filter="url(#${filter})"`);
        expect(drawn, where).toContain(inner(normal.picture ?? ""));
        expect(drawn.match(/<svg[\s>]/g) ?? [], where).toHaveLength(1);
        for (const banned of ["href", "<text", "<title", "NaN", "undefined"]) expect(drawn, `${where} ${banned}`).not.toContain(banned);
        // R2725004: a Radiant face's painted picture washes back over every family's but Core's, which keeps its colours.
        expect(drawn.includes('result="sc-wash"'), where).toBe(radiant && family !== "core");
      }
    }
  });

  it("R2943001 the five Set showcase treatments of one picture differ from one another, base and Radiant, and showcaseSvg draws each", () => {
    const picture = '<svg viewBox="0 0 100 100"><rect width="100" height="100" fill="#123456"/></svg>';
    for (const radiant of [false, true]) {
      const drawn = FAMILIES.map((family) => showcaseSvg(picture, family, radiant));
      expect(new Set(drawn).size).toBe(FAMILIES.length);
      for (const [at, family] of FAMILIES.entries()) {
        expect(restyledSvg(picture, `showcase-${family}`, radiant, "core-008"), family).toBe(drawn[at]);
        expect(restyledSvg(picture, `showcase-${family}`, radiant, "core-083"), family).toBe(drawn[at]);
        expect(drawn[at], family).toContain('<rect width="100" height="100" fill="#123456"/>');
        expect(drawn[at], family).not.toContain('viewBox="0 0 100 100"><rect');
        for (const other of FAMILIES) if (other !== family) expect(drawn[at], `${family} holds ${other}'s filter`).not.toContain(`id="${FILTERS[other]}"`);
      }
      // On one card too, against its normal, Pixel and Comic pictures.
      const card = FAMILIES.map((family) => drawnArt("core-008", radiant, `showcase-${family}`).picture);
      const plain = (["normal", "pixel", "comic"] as const).map((other) => drawnArt("core-008", radiant, other).picture);
      expect(new Set([...card, ...plain]).size).toBe(FAMILIES.length + plain.length);
    }
  });

  /** One Unit Out of frame is offered on of each painter: two procedural ones, a Stinkyscape one and a Training Grounds one. */
  const CUT_OUT = ["core-052", "classic-080", "stinkyscape-080", "traininggrounds-016"] as const;

  it("R2943001 a painter draws its picture whole, its backdrop with no subject or its subject on nothing, base and Radiant", () => {
    for (const id of CUT_OUT) {
      expect(CATALOG[id]?.type, id).toBe("Unit");
      expect(hasArtParts(id, null), id).toBe(true);
      for (const radiant of [false, true]) {
        const where = `${id} ${radiant ? "Radiant" : "base"}`;
        const plain = drawnArt(id, radiant, "normal");
        const whole = drawnArt(id, radiant, "normal", { part: "whole" });
        const backdrop = drawnArt(id, radiant, "normal", { part: "backdrop" });
        const subject = drawnArt(id, radiant, "normal", { part: "subject" });
        // "whole" is the picture, byte for byte, and marks no part.
        expect(whole, where).toEqual(plain);
        expect(plain.part, where).toBeNull();
        expect([backdrop.part, subject.part], where).toEqual(["backdrop", "subject"]);
        const [all, back, figure] = [plain.picture ?? "", backdrop.picture ?? "", subject.picture ?? ""];
        expect(new Set([all, back, figure]).size, where).toBe(3);
        for (const drawn of [back, figure]) {
          expect(inner(drawn), where).not.toBe("");
          expect(drawn.match(/<svg[\s>]/g) ?? [], where).toHaveLength(1);
          for (const banned of ["href", "<text", "<title", "NaN", "undefined"]) expect(drawn, `${where} ${banned}`).not.toContain(banned);
        }
        // The subject stands on nothing: no sky, no vignette; the backdrop keeps both.
        expect(figure, where).not.toContain("url(#v)");
        expect(figure, where).not.toContain("url(#s)");
        expect(back, where).toContain("url(#v)");
        // Each part draws less than the whole picture.
        for (const drawn of [back, figure]) expect(inner(drawn).length, where).toBeLessThan(inner(all).length);
        // Its subject and family are the normal picture's.
        expect([subject.motif, subject.family], where).toEqual([plain.motif, plain.family]);
      }
    }
  });

  it("R2943002 an animated part moves: the subject's own layer and the backdrop's", () => {
    for (const id of CUT_OUT) {
      const motion = artMotionOf(id);
      expect(motion, id).not.toBeNull();
      const subject = drawnArt(id, false, "normal", { part: "subject", motion }).picture ?? "";
      const backdrop = drawnArt(id, false, "normal", { part: "backdrop", motion }).picture ?? "";
      expect(subject, id).toContain("art-m-subject");
      expect(backdrop, id).toContain("art-m-");
      expect(backdrop, id).not.toContain('class="art-m-subject"');
      expect(subject, id).not.toBe(drawnArt(id, false, "normal", { part: "subject" }).picture);
    }
  });

  it("R2943001 a delivered file gives way to the painter's parts, and a Legacy card and a Starship in play draw their one picture whole", () => {
    const id = "core-052";
    const manifest = { [id]: { base: true } } as const;
    expect(drawnArt(id, false, "normal", { manifest }).art).toBe("real");
    for (const part of ["backdrop", "subject"] as const) {
      const drawn = drawnArt(id, false, "normal", { manifest, part });
      expect(drawn.art, part).toBe("procedural");
      expect(drawn.picture, part).toBe(drawnArt(id, false, "normal", { part }).picture);
    }
    const LEGACY = "traininggrounds-044";
    const ship = { starship: { pieces: 2, launched: false } };
    expect([hasArtParts(LEGACY, null), hasArtParts("core-008", ship.starship)]).toEqual([false, false]);
    for (const part of ["backdrop", "subject"] as const) {
      expect(drawnArt(LEGACY, false, "normal", { part }), part).toEqual(drawnArt(LEGACY, false, "normal"));
      expect(drawnArt("core-008", false, "normal", { ...ship, part }), part).toEqual(drawnArt("core-008", false, "normal", ship));
    }
  });
});

describe("R2944001 Immersive's and Prismatic's pictures", () => {
  const BANNED = ["href", "<text", "<title", "NaN", "undefined"] as const;

  it("R2944001 both restyle the normal picture, seeded by the card's id, and restyledSvg draws each", () => {
    const picture = '<svg viewBox="0 0 100 100"><rect width="100" height="100" fill="#123456"/></svg>';
    for (const restyle of ["depth", "prismatic"] as const) {
      expect(restyleOf(restyle), restyle).toBe(restyle);
      expect(artSeedOf("core-022", restyle), restyle).toBe("core-022");
    }
    for (const radiant of [false, true]) {
      const depth = depthSvg(picture, radiant);
      const prismatic = prismaticSvg(picture, radiant);
      expect(restyledSvg(picture, "depth", radiant, "core-008")).toBe(depth);
      expect(restyledSvg(picture, "prismatic", radiant, "core-008")).toBe(prismatic);
      expect(depth).not.toBe(prismatic);
      for (const drawn of [depth, prismatic]) {
        expect(drawn).toContain('<rect width="100" height="100" fill="#123456"/>');
        expect(drawn).not.toContain('viewBox="0 0 100 100"><rect');
      }
      expect(depth).not.toContain('id="pr-chrome"');
      expect(prismatic).not.toContain('id="dp-haze"');
    }
  });

  /** Two Mythic Units (a procedural one and a Stinkyscape one), two Mythic Spells (a procedural one and a Training Grounds one) and a Legacy card, whose painter draws one picture. */
  const CHROMED = ["core-097", "stinkyscape-006", "traininggrounds-040", "traininggrounds-044"] as const;

  it("R2944001 each card's Prismatic draws its normal picture in chrome under a sheen, apart from every other picture, base and Radiant", () => {
    for (const id of CHROMED) {
      for (const radiant of [false, true]) {
        const where = `${id} ${radiant ? "Radiant" : "base"}`;
        const normal = drawnArt(id, radiant, "normal");
        const prismatic = drawnArt(id, radiant, "prismatic");
        const others = (["sketch", "pixel", "comic"] as const).map((picture) => drawnArt(id, radiant, picture).picture);
        const drawn = prismatic.picture ?? "";
        expect(new Set([normal.picture, drawn, ...others]).size, where).toBe(5);
        expect(prismatic.marked, where).toBe("prismatic");
        expect([prismatic.motif, prismatic.family], where).toEqual([normal.motif, normal.family]);
        expect(drawn, where).toContain('filter id="pr-chrome"');
        expect(drawn, where).toContain('filter="url(#pr-chrome)"');
        expect(drawn, where).toContain('fill="url(#pr-sheen)"');
        expect(drawn, where).toContain(inner(normal.picture ?? ""));
        expect(drawn.match(/<svg[\s>]/g) ?? [], where).toHaveLength(1);
        for (const banned of BANNED) expect(drawn, `${where} ${banned}`).not.toContain(banned);
        // R2725004: a Radiant face's painted picture washes back over the chrome.
        expect(drawn.includes('result="pr-wash"'), where).toBe(radiant);
      }
    }
  });

  /** Cards of each painter Immersive lifts the subject of: two procedural ones (a Spell, a Unit), a Stinkyscape one and a Training Grounds one. */
  const HAZED = ["core-097", "core-100", "stinkyscape-006", "traininggrounds-040"] as const;

  it("R2944001 each card's Immersive window draws its backdrop hazed as far off, apart from the plain backdrop and the whole picture, base and Radiant", () => {
    for (const id of HAZED) {
      expect(hasArtParts(id, null), id).toBe(true);
      for (const radiant of [false, true]) {
        const where = `${id} ${radiant ? "Radiant" : "base"}`;
        const whole = drawnArt(id, radiant, "normal");
        const backdrop = drawnArt(id, radiant, "normal", { part: "backdrop" });
        const subject = drawnArt(id, radiant, "normal", { part: "subject" });
        const far = drawnArt(id, radiant, "depth", { part: "backdrop" });
        const drawn = far.picture ?? "";
        expect(new Set([whole.picture, backdrop.picture, subject.picture, drawn]).size, where).toBe(4);
        expect([far.marked, far.part], where).toEqual(["depth", "backdrop"]);
        expect([far.motif, far.family], where).toEqual([whole.motif, whole.family]);
        expect(drawn, where).toContain('filter id="dp-haze"');
        expect(drawn, where).toContain('filter="url(#dp-haze)"');
        // The backdrop alone under the haze: the subject is the near layer's (CardFace).
        expect(drawn, where).toContain(inner(backdrop.picture ?? ""));
        expect(drawn, where).not.toContain(inner(whole.picture ?? ""));
        expect(drawn.match(/<svg[\s>]/g) ?? [], where).toHaveLength(1);
        for (const banned of BANNED) expect(drawn, `${where} ${banned}`).not.toContain(banned);
        expect(drawn.includes('result="dp-wash"'), where).toBe(radiant);
        // The whole picture hazed, as a tile draws it, is a picture of its own too.
        const tile = drawnArt(id, radiant, "depth").picture ?? "";
        expect(tile, where).toContain(inner(whole.picture ?? ""));
        expect(new Set([tile, drawn, whole.picture]).size, where).toBe(3);
      }
    }
  });

  it("R2944002 an animated far layer moves, and so does its near subject", () => {
    for (const id of HAZED) {
      const motion = artMotionOf(id);
      expect(motion, id).not.toBeNull();
      const far = drawnArt(id, false, "depth", { part: "backdrop", motion }).picture ?? "";
      const near = drawnArt(id, false, "normal", { part: "subject", motion }).picture ?? "";
      expect(far, id).toContain("art-m-");
      expect(far, id).toContain('filter="url(#dp-haze)"');
      expect(far, id).not.toBe(drawnArt(id, false, "depth", { part: "backdrop" }).picture);
      expect(near, id).toContain("art-m-subject");
      const chrome = drawnArt(id, false, "prismatic", { motion }).picture ?? "";
      expect(chrome, id).toContain("art-m-");
      expect(chrome, id).toContain('fill="url(#pr-sheen)"');
    }
  });
});

describe("R3017002 part 6's pictures", () => {
  const BANNED = ["href", "<text", "<title", "NaN", "undefined"] as const;
  /** Each of part 6's pictures and its filter (pictures.ts). */
  const FILTERS: Readonly<Record<"night" | "watercolor" | "stained-glass" | "tarot" | "woodcut" | "neon", string>> = {
    night: "nt-night",
    watercolor: "wc-paint",
    "stained-glass": "sg-glass",
    tarot: "tr-tarot",
    woodcut: "wd-print",
    neon: "ne-neon",
  };
  const PART_SIX = Object.keys(FILTERS) as (keyof typeof FILTERS)[];
  const REPAINTS = ["night", "watercolor", "stained-glass", "tarot"] as const;
  const RESTYLES = ["woodcut", "neon"] as const;
  /** What each filter names a Radiant face's wash. */
  const WASHES: Readonly<Record<keyof typeof FILTERS, string>> = {
    night: "nt-wash",
    watercolor: "wc-wash",
    "stained-glass": "sg-wash",
    tarot: "tr-wash",
    woodcut: "wd-wash",
    neon: "ne-wash",
  };

  it("R3017002 the four repaints seed a picture of their own by the card's id and name, and Woodcut and Neon sign restyle the normal one", () => {
    for (const picture of PART_SIX) expect(restyleOf(picture), picture).toBe(picture);
    for (const picture of REPAINTS) {
      expect(isRepaint(picture), picture).toBe(true);
      expect(artSeedOf("core-022", picture), picture).toBe(`core-022#${picture}`);
      expect(artSeedOf("core-022", picture), picture).not.toBe(artSeedOf("core-022", "alt"));
    }
    for (const picture of [...RESTYLES, "normal", "alt", "sketch", "pixel"] as const) expect(isRepaint(picture), picture).toBe(false);
    for (const picture of RESTYLES) expect(artSeedOf("core-022", picture), picture).toBe("core-022");
    // No catalog id is another card's picture's seed, and no two pictures of one card share one.
    for (const id of Object.keys(CATALOG)) for (const picture of REPAINTS) expect(CATALOG[artSeedOf(id, picture)], `${id} ${picture}`).toBeUndefined();
    expect(new Set(REPAINTS.map((picture) => artSeedOf("core-022", picture))).size).toBe(REPAINTS.length);
  });

  it("R3017002 restyledSvg draws each of part 6's filters, the same for the same picture whatever the card, each apart", () => {
    const picture = '<svg viewBox="0 0 100 100"><rect width="100" height="100" fill="#123456"/></svg>';
    const draw: Readonly<Record<keyof typeof FILTERS, (svg: string, radiant: boolean) => string>> = {
      night: nightSvg,
      watercolor: watercolorSvg,
      "stained-glass": stainedGlassSvg,
      tarot: tarotSvg,
      woodcut: woodcutSvg,
      neon: neonSvg,
    };
    for (const radiant of [false, true]) {
      const drawn = PART_SIX.map((each) => draw[each](picture, radiant));
      expect(new Set(drawn).size).toBe(PART_SIX.length);
      for (const [at, each] of PART_SIX.entries()) {
        const one = drawn[at] ?? "";
        expect(restyledSvg(picture, each, radiant, "core-008"), each).toBe(one);
        expect(restyledSvg(picture, each, radiant, "core-083"), each).toBe(one);
        expect(one, each).toContain(`filter id="${FILTERS[each]}"`);
        expect(one, each).toContain(`filter="url(#${FILTERS[each]})"`);
        expect(one, each).toContain('<rect width="100" height="100" fill="#123456"/>');
        expect(one, each).not.toContain('viewBox="0 0 100 100"><rect');
        expect(one.match(/<svg[\s>]/g) ?? [], each).toHaveLength(1);
        for (const banned of BANNED) expect(one, `${each} ${banned}`).not.toContain(banned);
        for (const other of PART_SIX) if (other !== each) expect(one, `${each} holds ${other}'s filter`).not.toContain(`id="${FILTERS[other]}"`);
        // R2725004: a Radiant face's painted picture washes back over it; a base face's never.
        expect(one.includes(`result="${WASHES[each]}"`), each).toBe(radiant);
      }
    }
  });

  /** One card of each painter, as R2940001's tests: two procedural cards, a Stinkyscape one and a Training Grounds one. */
  const PAINTED = ["core-008", "core-083", "stinkyscape-154", "traininggrounds-040"] as const;

  it("R3017002 each of part 6's pictures is its own, apart from the normal one, Alt art's and every other, base and Radiant, on every painter", () => {
    for (const id of PAINTED) {
      for (const radiant of [false, true]) {
        const where = `${id} ${radiant ? "Radiant" : "base"}`;
        const normal = drawnArt(id, radiant, "normal");
        const alt = drawnArt(id, radiant, "alt");
        const drawn = PART_SIX.map((name) => ({ name, art: drawnArt(id, radiant, name) }));
        expect(new Set([normal.picture, alt.picture, ...drawn.map((each) => each.art.picture)]).size, where).toBe(2 + PART_SIX.length);
        for (const { name, art } of drawn) {
          const at = `${where} ${name}`;
          const svg = art.picture ?? "";
          expect(art.marked, at).toBe(name);
          // The subject is the normal picture's: what the card's name, tags and type pick.
          expect([art.motif, art.family], at).toEqual([normal.motif, normal.family]);
          expect(svg, at).toContain(`filter id="${FILTERS[name]}"`);
          expect(svg.match(/<svg[\s>]/g) ?? [], at).toHaveLength(1);
          for (const banned of BANNED) expect(svg, `${at} ${banned}`).not.toContain(banned);
          // A repaint draws a picture of its own, Alt art's no more than the normal one; a restyle the normal one.
          if ((RESTYLES as readonly string[]).includes(name)) expect(svg, at).toContain(inner(normal.picture ?? ""));
          else {
            expect(svg, at).not.toContain(inner(normal.picture ?? ""));
            expect(svg, at).not.toContain(inner(alt.picture ?? ""));
          }
          expect(svg.includes(`result="${WASHES[name]}"`), at).toBe(radiant);
        }
      }
    }
  });

  it("R3017002 a Legacy card and a Starship in play keep their one picture under each, and a delivered file gives way to the painter", () => {
    // Legacy: Duplicating Felinors wears its Core card's picture (R2724000); a Starship in play draws its pieces.
    const LEGACY = "traininggrounds-044";
    const ship = { starship: { pieces: 2, launched: false } };
    const normal = drawnArt(LEGACY, false, "normal");
    const flying = drawnArt("core-008", true, "normal", ship);
    for (const picture of PART_SIX) {
      const legacy = drawnArt(LEGACY, false, picture);
      expect(legacy.family, picture).toBe("legacy");
      expect(legacy.marked, picture).toBe(picture);
      expect(legacy.picture, picture).toContain(inner(normal.picture ?? ""));
      expect(legacy.picture, picture).toContain(`filter id="${FILTERS[picture]}"`);
      expect(legacy.picture, picture).not.toBe(normal.picture);
      const restyledShip = drawnArt("core-008", true, picture, ship);
      expect(restyledShip.family, picture).toBe("starship");
      expect(restyledShip.picture, picture).toContain(inner(flying.picture ?? ""));
      expect(restyledShip.picture, picture).not.toBe(flying.picture);
    }
    const id = "core-008";
    const manifest = { [id]: { base: true } } as const;
    expect(drawnArt(id, false, "normal", { manifest }).art).toBe("real");
    for (const picture of PART_SIX) {
      const drawn = drawnArt(id, false, picture, { manifest });
      expect(drawn.art, picture).toBe("procedural");
      expect(drawn.picture, picture).toBe(drawnArt(id, false, picture).picture);
    }
  });

  it("R3017002 an animated level moves each of part 6's pictures under its filter", () => {
    // Mr. Vanilla's own motion and Dice of Doom's, a Stinkyscape card's (two of the 21).
    for (const id of ["core-008", "stinkyscape-154"]) {
      const motion = artMotionOf(id);
      expect(motion, id).not.toBeNull();
      for (const picture of PART_SIX) {
        const still = drawnArt(id, false, picture);
        const moving = drawnArt(id, false, picture, { motion });
        expect(moving.picture, `${id} ${picture}`).toContain("art-m-");
        expect(moving.picture, `${id} ${picture}`).toContain(`filter="url(#${FILTERS[picture]})"`);
        expect(moving.picture, `${id} ${picture}`).not.toBe(still.picture);
        expect(moving.marked, `${id} ${picture}`).toBe(picture);
      }
    }
  });

  it("R3017002 artPictureOf gives each of part 6's levels its variant's picture", () => {
    const levels = Object.keys(COSMETIC_LEVELS).filter((id) => PART_SIX.some((picture) => COSMETIC_LEVELS[id]?.variant === picture));
    expect(levels).toHaveLength(2 * PART_SIX.length);
    for (const id of levels) expect(artPictureOf(id, "Core"), id).toBe(COSMETIC_LEVELS[id]?.variant as ArtRestyle);
  });
});
