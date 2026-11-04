// R2949000: a look with a picture of its own (pictures.ts): the Signature look draws its card's painter seeded
// apart from the card's normal picture, base and Radiant, on every card Signature is offered on. The faces
// wearing it are fullArtTiers.test.tsx's; the pixels are e2e's cypress/component/card-versions.cy.tsx.
// R2940001: Alt art's picture, seeded apart as Signature's is, and the Sketch, the normal picture in pencil; the
// faces wearing them are versionLooks.test.tsx's. R2942001: Pixel's and Comic's, the normal picture restyled; the
// faces wearing them are versionLooks3.test.tsx's. R2943001: Set showcase's, the normal picture in its set's treatment,
// and Out of frame's, the picture's backdrop and subject drawn apart; the faces wearing them are versionLooks4.test.tsx's.

import { createElement } from "react";
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { SHIPPED_SETS } from "@jackioh/shared";

import { COSMETIC_OFFERS } from "../../wire/generated/cosmetics.ts";
import { artMotionOf } from "../fullArt.ts";
import { AUGMENT_SIGNATURE_ID } from "../versions.ts";
import { CardArt, hasArtParts, type CardArtProps } from "./CardArt.tsx";
import {
  SHOWCASE_FAMILIES,
  artSeedOf,
  comicSvg,
  pixelSvg,
  restyledSvg,
  restyleOf,
  showcaseFamilyOf,
  showcaseSvg,
  sketchSvg,
  type ArtPicture,
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

describe("R2942001 Pixel's and Comic's pictures", () => {
  it("R2942001 Pixel and Comic restyle the normal picture: seeded by the card's own id, each its own restyle", () => {
    for (const picture of ["pixel", "comic"] as const) expect(artSeedOf("core-022", picture), picture).toBe("core-022");
    expect((["normal", "signature", "alt", "sketch", "pixel", "comic"] as const).map(restyleOf)).toEqual([null, null, null, "sketch", "pixel", "comic"]);
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
    expect([showcaseFamilyOf(null), showcaseFamilyOf("Boss"), showcaseFamilyOf("Custom")]).toEqual([null, null, null]);
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
