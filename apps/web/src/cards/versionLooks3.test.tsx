// R2942001 (#2942): the faces of the Extended art, Pixel and Comic versions: every card each is offered draws it,
// base and Radiant, apart from its normal face and every other version; each keeps every part of the normal face.
// Extended art lays the normal picture across the face behind a translucent frame and rules box; Pixel and Comic
// restyle it, Pixel's name set in its pixel face. R2942002: Animated, a foil, a finish and a border layer on each, and
// Signature on none. R2725004 holds under each: a Radiant face keeps every Radiant mark, a base face shows none. The
// pictures themselves are art/pictures.test.ts's, the name face pixelName.test.ts's; the pixels are e2e's
// cypress/component/card-versions.cy.tsx.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { SHIPPED_SETS } from "@jackioh/shared";

import { COSMETIC_OFFERS } from "../wire/generated/cosmetics.ts";
import { CardFace } from "./CardFace.tsx";
import { EXTENDED_ART_BOX_PCT, EXTENDED_ART_FRAME_OPACITY, EXTENDED_ART_PICTURE_PCT, FACE_TEXT_MIN_HEIGHT_PX } from "./constants.ts";
import { resetCosmeticsForTests, writeCardPick, writeCardVersion } from "./cosmetics.ts";
import { faceModel } from "./model.ts";
import {
  PIXEL_GLYPH_COLS,
  PIXEL_GLYPH_GAP,
  PIXEL_GLYPH_ROWS,
  PIXEL_NAME_CELL_MIN_PX,
  PIXEL_NAME_MAX_GLYPHS,
  PIXEL_NAME_SHADOW_OFFSET,
  pixelNameSvg,
  pixelNameUri,
} from "./pixelName.ts";
import { WATERMARK_INK, WATERMARK_OPACITY } from "./setMark.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "./settings.ts";
import { __resetSettingsForTests } from "../settings/store.ts";
import {
  AUGMENT_ANIMATED_ID,
  AUGMENT_SIGNATURE_ID,
  VERSION_ALT_ART_ID,
  VERSION_ALT_FULL_ART_ID,
  VERSION_COMIC_ID,
  VERSION_EXTENDED_ART_ID,
  VERSION_FULL_ART_ID,
  VERSION_PIXEL_ID,
  VERSION_SKETCH_ID,
  artPictureOf,
  augmentsOf,
  isFullBleed,
  lookOf,
  offeredOn,
} from "./versions.ts";
import { legacyText } from "../test/faceData.ts";

/** R2942000: #2942's three versions, in the registry's order. */
const PART_THREE = [VERSION_EXTENDED_ART_ID, VERSION_PIXEL_ID, VERSION_COMIC_ID] as const;
/** Every version a card may wear but its normal face. */
const VERSIONS = [VERSION_FULL_ART_ID, VERSION_ALT_ART_ID, VERSION_SKETCH_ID, VERSION_ALT_FULL_ART_ID, ...PART_THREE] as const;
/** The restyles' filters, by version (art/pictures.ts). */
const FILTER: Readonly<Record<string, string>> = { [VERSION_PIXEL_ID]: "px-pixel", [VERSION_COMIC_ID]: "pop-comic" };

const HERE = dirname(fileURLToPath(import.meta.url));
const cardsCss = readFileSync(join(HERE, "cards.css"), "utf8");
const paletteCss = readFileSync(join(HERE, "..", "palette.css"), "utf8");

function faceIn(id: string, radiant: boolean): HTMLElement {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`no ${id}`);
  const { container } = render(<CardFace face={faceModel({ defId: id, def, radiant })} layout="full" worn />);
  const cf = container.querySelector<HTMLElement>(".cf");
  if (cf === null) throw new Error(`no face for ${id}`);
  return cf;
}

/** The art window's picture, decoded: what its CSS background draws. */
function pictureOf(cf: HTMLElement): string {
  return decodeURIComponent(cf.querySelector<HTMLElement>(".cf-art")?.style.backgroundImage ?? "");
}

/** Every rule of cards.css whose selector names `version`: its selector and declarations. */
function rulesOf(version: string): { selector: string; body: string }[] {
  return [...cardsCss.matchAll(new RegExp(`([^{}]*\\[data-version="${version}"\\][^{}]*)\\{([^}]*)\\}`, "g"))].map((match) => ({
    selector: match[1]?.trim() ?? "",
    body: match[2] ?? "",
  }));
}

beforeEach(() => {
  localStorage.clear();
  resetCosmeticsForTests();
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
});

afterEach(() => {
  cleanup();
  resetCosmeticsForTests();
  localStorage.clear();
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
  __resetSettingsForTests();
});

describe("R2942001 every card offered a part 3 version draws it", () => {
  /** The cards offered any of #2942's versions, by set. */
  const BY_SET = new Map<string, string[]>();
  for (const id of new Set(PART_THREE.flatMap((version) => COSMETIC_OFFERS[version] ?? []))) {
    const set = CATALOG[id]?.set ?? "";
    BY_SET.set(set, [...(BY_SET.get(set) ?? []), id]);
  }

  it("R2942001 each part 3 version is offered in every set that ships", () => {
    for (const version of PART_THREE) {
      const sets = new Set((COSMETIC_OFFERS[version] ?? []).map((id) => CATALOG[id]?.set));
      expect([...sets].sort(), version).toEqual([...SHIPPED_SETS].sort());
    }
  });

  /** How many offered cards one test draws (up to sixteen faces each), so that no test nears the timeout on a loaded runner. */
  const OFFERED_CARDS_PER_TEST = 8;
  for (const [set, ids] of BY_SET) {
    for (let from = 0; from < ids.length; from += OFFERED_CARDS_PER_TEST) {
      const some = ids.slice(from, from + OFFERED_CARDS_PER_TEST);
      it(`R2942001 every ${set} card ${some[0] ?? ""} to ${some.at(-1) ?? ""} draws each version it is offered, base and Radiant, apart from its normal face and every other version`, () => {
        drawsEachVersion(some);
      });
    }
  }

  /** R2942001: each of `ids` draws its normal face and each version it is offered, every face apart. */
  function drawsEachVersion(ids: readonly string[]): void {
    for (const id of ids) {
      for (const radiant of [false, true]) {
        const where = `${id} ${radiant ? "Radiant" : "base"}`;
        writeCardVersion(id, false, []);
        const normal = faceIn(id, radiant);
        const faces = new Map<string, { markup: string; picture: string }>([["normal", { markup: normal.outerHTML, picture: pictureOf(normal) }]]);
        cleanup();
        for (const version of VERSIONS.filter((each) => offeredOn(each, id))) {
          const at = `${where} ${version}`;
          writeCardVersion(id, version, []);
          const cf = faceIn(id, radiant);
          const art = cf.querySelector(".cf-art");
          expect(cf.getAttribute("data-version"), at).toBe(version);
          expect(cf.getAttribute("data-full-art"), at).toBe(isFullBleed(version) ? "true" : null);
          expect(art?.getAttribute("data-art-variant"), at).toBe(radiant ? "radiant" : "base");
          const picture = artPictureOf(version);
          expect(art?.getAttribute("data-art-picture"), at).toBe(picture === "normal" ? null : picture);
          expect(cf.querySelector(".cf-signature"), at).toBeNull();
          const drawn = pictureOf(cf);
          expect(drawn, at).not.toBe("");
          for (const banned of ["NaN", "undefined", "<text", "<title", "href"]) expect(drawn, `${at} ${banned}`).not.toContain(banned);
          if ((PART_THREE as readonly string[]).includes(version)) {
            // Every part of the normal face is kept: the name, the type line and the rules box.
            for (const part of [".cost-gem", ".card-name", ".card-type", ".card-text"]) expect(cf.querySelector(part), `${at} ${part}`).not.toBeNull();
            // Extended art lays the art across the face; Pixel and Comic keep the window's shape.
            expect(art?.classList.contains("cf-art--full"), at).toBe(version === VERSION_EXTENDED_ART_ID);
            const filter = FILTER[version];
            if (filter !== undefined) expect(drawn, at).toContain(`filter="url(#${filter})"`);
            // Pixel sets the name in its pixel face wherever the face has every character of it.
            const name = cf.querySelector<HTMLElement>(".card-name");
            const pixelled = version === VERSION_PIXEL_ID && pixelNameUri(name?.textContent ?? "") !== null;
            expect(name?.getAttribute("data-pixel-name"), at).toBe(pixelled ? "true" : null);
            expect(name?.style.getPropertyValue("--cf-pixel-name") !== "", at).toBe(pixelled);
          }
          faces.set(version, { markup: cf.outerHTML, picture: drawn });
          cleanup();
        }
        // Every face apart from every other.
        expect(new Set([...faces.values()].map((face) => face.markup)).size, where).toBe(faces.size);
        const pictureFor = (key: string): string | undefined => faces.get(key)?.picture;
        const plain = pictureFor("normal");
        // Extended art's picture is the normal one, past its window; Pixel's and Comic's are their own, and no other
        // version's.
        if (faces.has(VERSION_EXTENDED_ART_ID)) expect(pictureFor(VERSION_EXTENDED_ART_ID), where).toBe(plain);
        for (const version of [VERSION_PIXEL_ID, VERSION_COMIC_ID]) {
          if (!faces.has(version)) continue;
          for (const [other, face] of faces) if (other !== version) expect(pictureFor(version), `${where} ${version} beside ${other}`).not.toBe(face.picture);
        }
      }
    }
  }
});

describe("R2942001 Extended art's rules box", () => {
  /** sRGB relative luminance of `hex`, laid at `alpha` over `over` when given, as R2718000's test reads it. */
  function luminance(hex: string, over?: string, alpha = 1): number {
    const channel = (at: number): number => {
      const own = parseInt(hex.slice(at, at + 2), 16);
      const under = over === undefined ? own : parseInt(over.slice(at, at + 2), 16);
      const c = (alpha * own + (1 - alpha) * under) / 255;
      return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
    };
    return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
  }

  function contrast(a: number, b: number): number {
    return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
  }

  /** `top` at `alpha` over `under`, as a `#rrggbb`. */
  function over(top: string, under: string, alpha: number): string {
    const mixed = [1, 3, 5].map((at) => Math.round(alpha * parseInt(top.slice(at, at + 2), 16) + (1 - alpha) * parseInt(under.slice(at, at + 2), 16)));
    return `#${mixed.map((channel) => channel.toString(16).padStart(2, "0")).join("")}`;
  }

  it("R2942001 Extended art's rules box keeps the box's contrast floor over the darkest art, its watermark on it", () => {
    // The base face's box: the parchment at EXTENDED_ART_BOX_PCT over the picture; a Radiant face keeps its gold box.
    const boxes = rulesOf(VERSION_EXTENDED_ART_ID).filter((rule) => rule.selector.endsWith(".card-text"));
    expect(boxes).toHaveLength(1);
    const [box] = boxes;
    expect(box?.selector).toContain(':not([data-radiant-face="true"])');
    const mix = /background: color-mix\(in srgb, var\(--pal-parchment\) (\d+)%, transparent\);/.exec(box?.body ?? "");
    expect(Number(mix?.[1])).toBe(EXTENDED_ART_BOX_PCT);
    const parchment = /--pal-parchment:\s*(#[0-9a-f]{6})/i.exec(paletteCss)?.[1] ?? "";
    expect(parchment).toMatch(/^#[0-9a-f]{6}$/i);
    // Over black, the darkest art, the box is no darker than the normal box's edge (R2718000's floor), and every ink
    // reads over it under the watermark at least as well as over that edge.
    const edge = "#c9b48a";
    expect(cardsCss).toContain(`radial-gradient(120% 90% at 50% 40%, #f6ecd4, #e3d2ac 70%, ${edge})`);
    const darkest = over(parchment, "#000000", EXTENDED_ART_BOX_PCT / 100);
    expect(luminance(darkest)).toBeGreaterThanOrEqual(luminance(edge));
    const marked = luminance(WATERMARK_INK, darkest, WATERMARK_OPACITY);
    for (const ink of ["#2b2116", "#6b4200", "#1d4f8a", "#1d5a1f", "#13692a", "#a3120f"]) {
      expect(contrast(luminance(ink), marked), ink).toBeGreaterThanOrEqual(contrast(luminance(ink), luminance(edge)));
    }
  });

  it("R2942001 Extended art's frame is the panel over the art at EXTENDED_ART_FRAME_OPACITY at its rim, and the art fills the face at every size", () => {
    // The picture, a square EXTENDED_ART_PICTURE_PCT of the face's height on the face's foot: its middle a quarter of
    // the way down, where the normal window's is, and past every edge of the face.
    const picture = rulesOf(VERSION_EXTENDED_ART_ID).find((rule) => rule.selector.endsWith(".cf-art-frame > .cf-art"));
    for (const declaration of ["position: absolute;", "bottom: 0;", `height: ${String(EXTENDED_ART_PICTURE_PCT)}%;`, "aspect-ratio: 1;"]) {
      expect(picture?.body, declaration).toContain(declaration);
    }
    expect(100 - EXTENDED_ART_PICTURE_PCT / 2).toBeLessThan(30);
    expect(EXTENDED_ART_PICTURE_PCT).toBeGreaterThan(100);
    const frame = rulesOf(VERSION_EXTENDED_ART_ID).find((rule) => rule.selector.endsWith(".cf-art-frame::after"));
    expect(frame?.body).toContain("background: inherit;");
    expect(frame?.body).toContain(`opacity: ${String(EXTENDED_ART_FRAME_OPACITY)};`);
    expect(frame?.body).toContain("mask-image:");
    const filling = rulesOf(VERSION_EXTENDED_ART_ID).filter((rule) => rule.selector.endsWith(".cf-art-frame") && rule.body.includes("inset: 0;"));
    // The full face, and a small face's in its container query.
    expect(filling.length).toBeGreaterThanOrEqual(2);
  });
});

describe("R2942001 Pixel's name face stays readable", () => {
  /** The declarations of the one rule of `css` whose selector is exactly `selector`. */
  function bodyOf(css: string, selector: string): string {
    const bodies = [...css.matchAll(/([^{}]*)\{([^}]*)\}/g)].filter((match) => match[1]?.replace(/\/\*[^]*?\*\//g, "").trim() === selector);
    expect(bodies, selector).toHaveLength(1);
    return bodies[0]?.[2] ?? "";
  }

  /** The number before `unit` that `property` is set to in `body`, the first of a shorthand's. */
  function lengthOf(body: string, property: string, unit: string, at = 0): number {
    const value = new RegExp(`(?:^|[;\\s])${property}:([^;]*);`).exec(body)?.[1] ?? "";
    const lengths = [...value.matchAll(new RegExp(`(-?[\\d.]+)${unit}`, "g"))].map((match) => Number(match[1]));
    expect(lengths[at], `${property} in ${unit}`).toBeDefined();
    return lengths[at] ?? Number.NaN;
  }

  const FULL = '.cf[data-version="version-pixel"] .card-name[data-pixel-name="true"]';
  const SMALL = '.cf[data-version="version-pixel"]:not([data-layout="minion"]) .card-name[data-pixel-name="true"]';

  /**
   * The smallest full face's pixel-name box, in CSS pixels: the face (`.cf-scale`, the `cardface` container) is
   * FACE_TEXT_MIN_HEIGHT_PX tall and as wide as its card's aspect and inset make it; the box is the name banner less the
   * pixel face's padding.
   */
  function smallestBox(): { width: number; height: number } {
    const card = bodyOf(cardsCss, ".cf");
    const aspect = /aspect-ratio: (\d+) \/ (\d+);/.exec(card);
    const inset = /inset: ([\d.]+)% ([\d.]+)%;/.exec(bodyOf(cardsCss, ".cf-scale"));
    const height = FACE_TEXT_MIN_HEIGHT_PX;
    const width = (height / (1 - (2 * Number(inset?.[1])) / 100)) * (Number(aspect?.[1]) / Number(aspect?.[2])) * (1 - (2 * Number(inset?.[2])) / 100);
    const banner = bodyOf(cardsCss, ".cf .card-name");
    const pixel = bodyOf(cardsCss, FULL);
    const across = 100 - lengthOf(banner, "left", "cqw") - lengthOf(banner, "right", "cqw") - 2 * lengthOf(pixel, "padding", "cqw");
    const tall = lengthOf(banner, "height", "cqh") - 2 * lengthOf(pixel, "padding", "cqh");
    expect(width).toBeGreaterThan(height / 2);
    return { width: (width * across) / 100, height: (height * tall) / 100 };
  }

  /** A pixel name's cell, in CSS pixels, its bitmap `cols` by `rows` cells drawn `contain` in `box`. */
  function cellIn(box: { width: number; height: number }, cols: number, rows: number): number {
    return Math.min(box.width / cols, box.height / rows);
  }

  it("R2942001 on the smallest full face every pixel name keeps its cells at PIXEL_NAME_CELL_MIN_PX, and PIXEL_NAME_MAX_GLYPHS is the most that does", () => {
    const box = smallestBox();
    const rows = PIXEL_GLYPH_ROWS + PIXEL_NAME_SHADOW_OFFSET;
    const colsOf = (glyphs: number): number => glyphs * (PIXEL_GLYPH_COLS + PIXEL_GLYPH_GAP) - PIXEL_GLYPH_GAP + PIXEL_NAME_SHADOW_OFFSET;
    expect(cellIn(box, colsOf(PIXEL_NAME_MAX_GLYPHS), rows)).toBeGreaterThanOrEqual(PIXEL_NAME_CELL_MIN_PX);
    expect(cellIn(box, colsOf(PIXEL_NAME_MAX_GLYPHS + 1), rows)).toBeLessThan(PIXEL_NAME_CELL_MIN_PX);
    // Every name Pixel is offered on, the longest first, draws within the floor or prints in the normal face.
    const names = [...new Set((COSMETIC_OFFERS[VERSION_PIXEL_ID] ?? []).map((id) => CATALOG[id]?.name ?? ""))].sort((a, b) => b.length - a.length);
    expect(names[0]?.length ?? 0).toBeGreaterThan(PIXEL_NAME_MAX_GLYPHS);
    let drawn = 0;
    for (const name of names) {
      const svg = pixelNameSvg(name);
      expect(svg === null, name).toBe([...name].length > PIXEL_NAME_MAX_GLYPHS || pixelNameUri(name) === null);
      if (svg === null) continue;
      drawn += 1;
      const view = /viewBox="0 0 (\d+) (\d+)"/.exec(svg);
      expect(cellIn(box, Number(view?.[1]), Number(view?.[2])), name).toBeGreaterThanOrEqual(PIXEL_NAME_CELL_MIN_PX);
    }
    expect(drawn).toBeGreaterThan(names.length / 2);
  });

  it("R2942001 a name longer than PIXEL_NAME_MAX_GLYPHS prints in the normal face on a Pixel card", () => {
    // Call to Chaos (Training-Grounds Edition), Pixel's longest name, 40 characters.
    const id = "traininggrounds-039";
    expect(offeredOn(VERSION_PIXEL_ID, id)).toBe(true);
    writeCardVersion(id, VERSION_PIXEL_ID, []);
    for (const radiant of [false, true]) {
      const name = faceIn(id, radiant).querySelector<HTMLElement>(".card-name");
      expect(name?.textContent).toBe(CATALOG[id]?.name);
      expect(name?.getAttribute("data-pixel-name")).toBeNull();
      expect(name?.style.getPropertyValue("--cf-pixel-name")).toBe("");
      cleanup();
    }
  });

  it("R2942001 a small face prints a Pixel name in the normal face, on its two lines", () => {
    // Below FACE_TEXT_MIN_HEIGHT_PX, where a hand card's name wraps onto two lines, one line of bitmap cells would fall
    // under a pixel: the name's words show again, white with the normal name's outline, on Pixel's flat banner.
    const opening = `@container cardface (max-height: ${String(FACE_TEXT_MIN_HEIGHT_PX - 1)}px) {`;
    const from = cardsCss.indexOf(opening);
    expect(from).toBeGreaterThan(cardsCss.indexOf(FULL));
    const small = cardsCss.slice(from + opening.length, cardsCss.indexOf("\n}\n", from));
    const body = bodyOf(small, SMALL);
    expect(body).toContain("color: var(--pal-white);");
    expect(body).toContain("text-shadow: var(--cf-outline);");
    expect(body).toContain("background: var(--cf-frame);");
    expect(body).not.toContain("--cf-pixel-name");
    // The normal name's own ink and outline, the white palette.css's `--pal-white`.
    const banner = bodyOf(cardsCss, ".cf .card-name");
    expect(banner).toContain("color: #fff;");
    expect(banner).toContain("text-shadow: var(--cf-outline);");
    expect(paletteCss).toMatch(/--pal-white:\s*#ffffff;/);
  });
});

describe("R2942002 the augments on part 3's versions", () => {
  /**
   * Cards offered all three part 3 versions and Animated: Spell Tyrant (a Legendary Unit), CN Flea Market (a
   * Legendary Spell) and Galaxy (a Legendary Unit of Stinkyscape's painter).
   */
  const CARDS = ["classic-056", "meditative-042", "stinkyscape-080"] as const;
  /** Each part 3 version with no augment, either and both. */
  const LOOKS: readonly (readonly [string, readonly string[]])[] = PART_THREE.flatMap((version) =>
    [[], [AUGMENT_ANIMATED_ID], [AUGMENT_SIGNATURE_ID], [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID]].map((augments) => [version, augments] as const),
  );

  it("R2942002 Animated, a foil, a finish and a border layer on Extended art, Pixel and Comic, and Signature on none", () => {
    for (const id of CARDS) {
      for (const version of PART_THREE) expect(offeredOn(version, id), `${version} on ${id}`).toBe(true);
      expect(offeredOn(AUGMENT_ANIMATED_ID, id), id).toBe(true);
      for (const [version, augments] of LOOKS) {
        writeCardVersion(id, version, augments);
        writeCardPick(id, "border", "border-runes");
        writeCardPick(id, "finish", "finish-stained-glass");
        writeCardPick(id, "foil", "foil-galaxy");
        const cf = faceIn(id, false);
        const where = `${id} ${version} ${augments.join("+")}`;
        const animated = augments.includes(AUGMENT_ANIMATED_ID);
        // R2949000: Signature fits none of them; the store keeps none.
        expect(augmentsOf(lookOf(version, augments)).includes(AUGMENT_SIGNATURE_ID), where).toBe(false);
        expect(cf.getAttribute("data-version"), where).toBe(version);
        expect(cf.getAttribute("data-full-art"), where).toBeNull();
        expect(cf.getAttribute("data-augments"), where).toBe(animated ? AUGMENT_ANIMATED_ID : null);
        expect(cf.querySelector(".card-text"), where).not.toBeNull();
        expect(cf.querySelector(".card-name"), where).not.toBeNull();
        // Animated moves the version's own picture, taking a slot; no part 3 version lays the full-art band.
        expect(cf.getAttribute("data-art-motion"), where).toBe(animated ? "animated" : null);
        const picture = artPictureOf(version);
        expect(cf.querySelector(".cf-art")?.getAttribute("data-art-picture"), where).toBe(picture === "normal" ? null : picture);
        if (animated) expect(pictureOf(cf), where).toContain("art-m-");
        const filter = FILTER[version];
        if (filter !== undefined) expect(pictureOf(cf), where).toContain(`filter="url(#${filter})"`);
        expect(cf.classList.contains("cf-full-art--animated"), where).toBe(false);
        expect(cf.querySelector(".cf-signature"), where).toBeNull();
        expect(cf.classList.contains("cf-full-art--signed"), where).toBe(false);
        expect(cf.getAttribute("data-frame"), where).toBe("border-runes");
        expect(cf.getAttribute("data-finish"), where).toBe("finish-stained-glass");
        expect(cf.getAttribute("data-cosmetic-foil"), where).toBe("foil-galaxy");
        cleanup();
      }
    }
  });

  it("R2942002 a worn border keeps its edge and bezel on Pixel and Comic, and a Radiant face its gold frame", () => {
    for (const version of [VERSION_PIXEL_ID, VERSION_COMIC_ID]) {
      const frames = rulesOf(version);
      const setting = frames.filter((rule) => /--cf-(bezel|edge)\s*:/.test(rule.body));
      expect(setting.length, version).toBeGreaterThan(0);
      for (const rule of setting) {
        expect(rule.selector, rule.selector).toContain(":not([data-frame])");
        expect(rule.selector, rule.selector).toContain(':not([data-radiant-face="true"])');
      }
      // The window's ring takes the bezel, a border's where one is worn, and only on a base face.
      const rings = frames.filter((rule) => rule.selector.endsWith(".cf-art-frame > .cf-art"));
      expect(rings.length, version).toBeGreaterThan(0);
      for (const rule of rings) {
        expect(rule.selector, rule.selector).toContain(':not([data-radiant-face="true"])');
        expect(rule.body, rule.selector).toContain("var(--cf-bezel)");
      }
    }
    // Extended art sets no edge or bezel: a border's and a Radiant face's own frame stand; its rules box turns
    // translucent only on a base face.
    for (const rule of rulesOf(VERSION_EXTENDED_ART_ID)) {
      expect(rule.body, rule.selector).not.toMatch(/--cf-(bezel|edge)\s*:/);
      if (rule.selector.endsWith(".card-text")) expect(rule.selector).toContain(':not([data-radiant-face="true"])');
    }
  });
});

describe("R2942001 Radiant under part 3's looks", () => {
  /** Two Units and a Spell, each offered all three part 3 versions and Animated. */
  const CARDS = ["classic-056", "meditative-042", "stinkyscape-080"] as const;
  const LOOKS: readonly (readonly [string, readonly string[]])[] = PART_THREE.flatMap((version) =>
    [[], [AUGMENT_ANIMATED_ID]].map((augments) => [version, augments] as const),
  );

  it('R2942001 a Radiant face keeps data-radiant-face, data-art-variant="radiant", R277 marks and gold stats under every part 3 look; a base face has none', () => {
    for (const id of CARDS) {
      const def = CATALOG[id];
      if (def === undefined) throw new Error(`no ${id}`);
      const marked = legacyText(faceModel({ defId: id, def, radiant: true }).text).marks.length > 0;
      for (const [version, augments] of LOOKS) {
        writeCardVersion(id, version, augments);
        for (const radiant of [false, true]) {
          const cf = faceIn(id, radiant);
          const where = `${id} ${radiant ? "Radiant" : "base"} ${version} ${augments.join("+")}`;
          // The gold-foil frame, and the art's Radiant paint: kept uncut under Pixel, washed back over a Comic.
          expect(cf.getAttribute("data-radiant-face"), where).toBe(radiant ? "true" : null);
          expect(cf.querySelector(".cf-art")?.getAttribute("data-art-variant"), where).toBe(radiant ? "radiant" : "base");
          const drawn = pictureOf(cf);
          expect(drawn.includes('result="px-palette"'), where).toBe(!radiant && version === VERSION_PIXEL_ID);
          expect(drawn.includes('result="pop-wash"'), where).toBe(radiant && version === VERSION_COMIC_ID);
          // R277's marks wherever the words show: never on a base face, on a Radiant face that differs.
          const marks = cf.querySelectorAll('.cf-mark[data-mark="radiant"]').length;
          if (!radiant) expect(marks, where).toBe(0);
          else expect(marks > 0, where).toBe(marked);
          // A Unit's gold stats: what its Radiant face raises.
          if (def.type === "Unit") {
            const grew = (def.radiant.attack ?? 0) > (def.base.attack ?? 0);
            expect(cf.querySelector(".cf-atk")?.getAttribute("data-grew"), where).toBe(radiant && grew ? "true" : null);
          }
          cleanup();
        }
      }
    }
  });
});
