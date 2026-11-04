// R2943001 (#2943): the faces of the Set showcase and Out of frame versions: every card each is offered draws it, base
// and Radiant, apart from its normal face and every other version; each keeps every part of the normal face. Set
// showcase wears its set's own look (`data-showcase`): its picture in its set's treatment and, on a base face, its
// set's edge, bezel, ring and name banner; Out of frame draws its picture's backdrop in a smaller window and its subject
// cut out over the frame, inside the card's rim and under the gem, crest, name, stats and rules box. R2943002:
// Animated, a foil, a finish and a border layer on each, and Signature on none. R2725004 holds under each: a Radiant
// face keeps every Radiant mark, a base face shows none. The pictures themselves are art/pictures.test.ts's; the pixels
// are e2e's cypress/component/card-versions.cy.tsx.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { SHIPPED_SETS } from "@jackioh/shared";

import { COSMETIC_OFFERS } from "../wire/generated/cosmetics.ts";
import { showcaseFamilyOf, type ShowcaseFamily } from "./art/pictures.ts";
import { CardFace } from "./CardFace.tsx";
import { OUT_OF_FRAME_SUBJECT_PCT, OUT_OF_FRAME_SUBJECT_SINK_PCT, OUT_OF_FRAME_WINDOW_SIDE_PCT, OUT_OF_FRAME_WINDOW_TOP_PCT } from "./constants.ts";
import { resetCosmeticsForTests, writeCardPick, writeCardVersion } from "./cosmetics.ts";
import { faceModel } from "./model.ts";
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
  VERSION_OUT_OF_FRAME_ID,
  VERSION_PIXEL_ID,
  VERSION_SET_SHOWCASE_ID,
  VERSION_SKETCH_ID,
  artPictureOf,
  augmentsOf,
  isFullBleed,
  lookOf,
  offeredOn,
} from "./versions.ts";

/** R2943000: #2943's two versions, in the registry's order. */
const PART_FOUR = [VERSION_SET_SHOWCASE_ID, VERSION_OUT_OF_FRAME_ID] as const;
/** Every version a card may wear but its normal face. */
const VERSIONS = [
  VERSION_FULL_ART_ID,
  VERSION_ALT_ART_ID,
  VERSION_SKETCH_ID,
  VERSION_ALT_FULL_ART_ID,
  VERSION_EXTENDED_ART_ID,
  VERSION_PIXEL_ID,
  VERSION_COMIC_ID,
  ...PART_FOUR,
] as const;
/** Each Set showcase family's filter (art/pictures.ts `showcaseSvg`). */
const FILTER: Readonly<Record<ShowcaseFamily, string>> = {
  core: "sc-core",
  classic: "sc-stone",
  meditative: "sc-ink",
  stinkyscape: "sc-space",
  "training-grounds": "sc-wood",
};
const FAMILIES = Object.keys(FILTER) as ShowcaseFamily[];

const HERE = dirname(fileURLToPath(import.meta.url));
const cardsCss = readFileSync(join(HERE, "cards.css"), "utf8");
const paletteCss = readFileSync(join(HERE, "..", "palette.css"), "utf8");
/** palette.css's `--pal-*` colours by name. */
const PALETTE = new Map([...paletteCss.matchAll(/(--pal-[a-z0-9-]+):\s*(#[0-9a-fA-F]{6})/g)].map((match) => [match[1] ?? "", match[2] ?? ""]));

function faceIn(id: string, radiant: boolean): HTMLElement {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`no ${id}`);
  const { container } = render(<CardFace face={faceModel({ defId: id, def, radiant })} layout="full" worn />);
  const cf = container.querySelector<HTMLElement>(".cf");
  if (cf === null) throw new Error(`no face for ${id}`);
  return cf;
}

/** A picture's CSS background, decoded. */
function drawnBy(art: HTMLElement | null | undefined): string {
  return decodeURIComponent(art?.style.backgroundImage ?? "");
}

/** The art window's picture: what its CSS background draws (the backdrop on an Out of frame face). */
function pictureOf(cf: HTMLElement): string {
  return drawnBy(cf.querySelector<HTMLElement>(".cf-art:not(.cf-art--cutout)"));
}

/** The Out of frame face's cut-out subject, or null. */
function cutoutOf(cf: HTMLElement): HTMLElement | null {
  return cf.querySelector<HTMLElement>(".cf-art--cutout");
}

/** Every rule of `css` whose selector matches `pattern`: its selector (comments dropped) and declarations. */
function rulesMatching(css: string, pattern: RegExp): { selector: string; body: string }[] {
  return [...css.matchAll(/([^{}]*)\{([^{}]*)\}/g)]
    .map((match) => ({ selector: (match[1] ?? "").replace(/\/\*[^]*?\*\//g, "").trim(), body: match[2] ?? "" }))
    .filter((rule) => pattern.test(rule.selector));
}

/** The declarations of the one rule of `css` whose selector is exactly `selector`. */
function bodyOf(css: string, selector: string): string {
  const bodies = rulesMatching(css, /./).filter((rule) => rule.selector === selector);
  expect(bodies, selector).toHaveLength(1);
  return bodies[0]?.body ?? "";
}

/** The number before `unit` that `property` is set to in `body`. */
function lengthOf(body: string, property: string, unit: string): number {
  const value = new RegExp(`(?:^|[;\\s])${property}:\\s*(-?[\\d.]+)${unit}\\s*;`).exec(body)?.[1];
  expect(value, `${property} in ${unit}`).toBeDefined();
  return Number(value);
}

/** The z-index the one rule `selector` gives. */
function zIndexOf(selector: string): number {
  return Number(/z-index:\s*(-?\d+);/.exec(bodyOf(cardsCss, selector))?.[1]);
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

describe("R2943001 every card offered a part 4 version draws it", () => {
  /** The cards offered either of #2943's versions, by set. */
  const BY_SET = new Map<string, string[]>();
  for (const id of new Set(PART_FOUR.flatMap((version) => COSMETIC_OFFERS[version] ?? []))) {
    const set = CATALOG[id]?.set ?? "";
    BY_SET.set(set, [...(BY_SET.get(set) ?? []), id]);
  }

  it("R2943001 each part 4 version is offered in every set that ships", () => {
    for (const version of PART_FOUR) {
      const sets = new Set((COSMETIC_OFFERS[version] ?? []).map((id) => CATALOG[id]?.set));
      expect([...sets].sort(), version).toEqual([...SHIPPED_SETS].sort());
    }
  });

  /** How many offered cards one test draws (up to twenty faces each), so that no test nears the timeout on a loaded runner. */
  const OFFERED_CARDS_PER_TEST = 8;
  for (const [set, ids] of BY_SET) {
    for (let from = 0; from < ids.length; from += OFFERED_CARDS_PER_TEST) {
      const some = ids.slice(from, from + OFFERED_CARDS_PER_TEST);
      it(`R2943001 every ${set} card ${some[0] ?? ""} to ${some.at(-1) ?? ""} draws each version it is offered, base and Radiant, apart from its normal face and every other version`, () => {
        drawsEachVersion(some);
      });
    }
  }

  /** R2943001: each of `ids` draws its normal face and each version it is offered, every face apart. */
  function drawsEachVersion(ids: readonly string[]): void {
    for (const id of ids) {
      const set = CATALOG[id]?.set ?? null;
      for (const radiant of [false, true]) {
        const where = `${id} ${radiant ? "Radiant" : "base"}`;
        writeCardVersion(id, false, []);
        const normal = faceIn(id, radiant);
        expect(normal.getAttribute("data-showcase"), where).toBeNull();
        expect(cutoutOf(normal), where).toBeNull();
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
          expect(cf.querySelector(".cf-signature"), at).toBeNull();
          const drawn = pictureOf(cf);
          expect(drawn, at).not.toBe("");
          for (const banned of ["NaN", "undefined", "<text", "<title", "href"]) expect(drawn, `${at} ${banned}`).not.toContain(banned);
          if ((PART_FOUR as readonly string[]).includes(version)) {
            // Every part of the normal face is kept: the cost gem, the name, the type line and the rules box.
            for (const part of [".cost-gem", ".card-name", ".card-type", ".card-text"]) expect(cf.querySelector(part), `${at} ${part}`).not.toBeNull();
            expect(cf.getAttribute("data-full-art"), at).toBeNull();
          }
          if (version === VERSION_SET_SHOWCASE_ID) {
            // Its set's own look, its picture under its set's filter.
            const family = showcaseFamilyOf(set);
            expect(family, at).not.toBeNull();
            expect(cf.getAttribute("data-showcase"), at).toBe(family);
            expect(art?.getAttribute("data-art-picture"), at).toBe(artPictureOf(version, set));
            expect(drawn, at).toContain(`filter="url(#${FILTER[family ?? "core"]})"`);
            expect(cutoutOf(cf), at).toBeNull();
          } else {
            expect(cf.getAttribute("data-showcase"), at).toBeNull();
          }
          if (version === VERSION_OUT_OF_FRAME_ID) {
            // The window draws the backdrop, and the subject is cut out over it inside the art frame.
            const cutout = cutoutOf(cf);
            expect(cutout?.parentElement?.classList.contains("cf-art-frame"), at).toBe(true);
            expect(cutout?.getAttribute("data-art-part"), at).toBe("subject");
            expect(cutout?.getAttribute("data-art-variant"), at).toBe(radiant ? "radiant" : "base");
            expect(cutout?.getAttribute("aria-hidden"), at).toBe("true");
            expect(art?.getAttribute("data-art-part"), at).toBe("backdrop");
            const subject = drawnBy(cutout);
            expect(subject, at).not.toBe("");
            expect(subject, at).not.toBe(drawn);
            for (const banned of ["NaN", "undefined", "<text", "<title", "href"]) expect(subject, `${at} ${banned}`).not.toContain(banned);
          } else {
            expect(cutoutOf(cf), at).toBeNull();
          }
          faces.set(version, { markup: cf.outerHTML, picture: drawn });
          cleanup();
        }
        // Every face apart from every other, and each part 4 version's picture its own.
        expect(new Set([...faces.values()].map((face) => face.markup)).size, where).toBe(faces.size);
        for (const version of PART_FOUR) {
          const own = faces.get(version)?.picture;
          if (own === undefined) continue;
          for (const [other, face] of faces) if (other !== version) expect(own, `${where} ${version} beside ${other}`).not.toBe(face.picture);
        }
      }
    }
  }
});

describe("R2943001 the cut-out subject's place", () => {
  it("R2943001 the cutout stays inside the rim, under the gem, crest, name, stats and rules box", () => {
    const id = "classic-080";
    expect(offeredOn(VERSION_OUT_OF_FRAME_ID, id)).toBe(true);
    writeCardVersion(id, VERSION_OUT_OF_FRAME_ID, []);
    const cf = faceIn(id, false);
    const frame = cf.querySelector(".cf-art-frame");
    const cutout = cutoutOf(cf);
    // Inside the art frame, after its window, inside the clipped face (`.cf-scale`), before the rules box.
    expect(cutout?.parentElement).toBe(frame);
    expect(frame?.parentElement?.classList.contains("cf-scale")).toBe(true);
    expect(frame?.lastElementChild).toBe(cutout);
    const text = cf.querySelector(".card-text");
    expect(text !== null && frame !== null && (frame.compareDocumentPosition(text) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0).toBe(true);
    // The art frame lies under the crest, name and type line, and under the gems and stats; the rules box, at its
    // level, follows it.
    const frameZ = zIndexOf(".cf .cf-art-frame");
    for (const over of [".cf .cf-crest", ".cf .card-name", ".cf .cost-gem", ".cf .cf-gem", ".cf .cf-stats"]) expect(zIndexOf(over), over).toBeGreaterThan(frameZ);
    expect(zIndexOf(".cf .card-text")).toBe(frameZ);
    expect(bodyOf(cardsCss, ".cf-scale")).toContain("overflow: hidden;");
    // The window and the cutout as constants.ts has them.
    const opening = bodyOf(cardsCss, '.cf[data-version="version-out-of-frame"] .cf-art-frame > .cf-art:not(.cf-art--cutout)');
    expect(opening).toContain(`inset: ${String(OUT_OF_FRAME_WINDOW_TOP_PCT)}% ${String(OUT_OF_FRAME_WINDOW_SIDE_PCT)}% 0;`);
    const square = bodyOf(cardsCss, '.cf[data-version="version-out-of-frame"] .cf-art-frame > .cf-art.cf-art--cutout');
    for (const declaration of [
      "position: absolute;",
      `bottom: -${String(OUT_OF_FRAME_SUBJECT_SINK_PCT)}%;`,
      `height: ${String(OUT_OF_FRAME_SUBJECT_PCT)}%;`,
      "aspect-ratio: 1;",
      "background-color: transparent;",
      "box-shadow: none;",
    ]) {
      expect(square, declaration).toContain(declaration);
    }
    // On every art frame a Unit's face draws, full size and small, the square rises above the window and never above
    // the face's top, is wider than the window and narrower than the frame, and its foot, sunk below the frame's, lies
    // under the name banner (a cqh is the face's height, a cqw its width: the face is `.cf-scale`, its card's aspect
    // less its inset).
    const card = bodyOf(cardsCss, ".cf");
    const aspect = /aspect-ratio: (\d+) \/ (\d+);/.exec(card);
    const inset = /inset: ([\d.]+)% ([\d.]+)%;/.exec(bodyOf(cardsCss, ".cf-scale"));
    const cqhInCqw = (1 - (2 * Number(inset?.[1])) / 100) / ((Number(aspect?.[1]) / Number(aspect?.[2])) * (1 - (2 * Number(inset?.[2])) / 100));
    expect(cqhInCqw).toBeGreaterThan(1);
    const small = cardsCss.slice(cardsCss.indexOf("@container cardface (max-height: 149px) {"));
    const frames = [
      [bodyOf(cardsCss, '.cf[data-card-type="Unit"] .cf-art-frame'), bodyOf(cardsCss, ".cf .card-name")],
      [
        bodyOf(small, '.cf:not([data-layout="minion"]) .cf-art-frame,\n  .cf:not([data-layout="minion"])[data-card-type] .cf-art-frame'),
        bodyOf(small, '.cf:not([data-layout="minion"]) .card-name,\n  .cf:not([data-layout="minion"])[data-layout] .card-name'),
      ],
    ] as const;
    for (const [body, banner] of frames) {
      const top = lengthOf(body, "top", "cqh");
      const height = lengthOf(body, "height", "cqh");
      const width = 100 - lengthOf(body, "left", "cqw") - lengthOf(body, "right", "cqw");
      const side = (height * OUT_OF_FRAME_SUBJECT_PCT) / 100;
      const foot = top + height + (height * OUT_OF_FRAME_SUBJECT_SINK_PCT) / 100;
      expect(foot - side, body).toBeGreaterThanOrEqual(0);
      expect(foot - side, body).toBeLessThan(top + (height * OUT_OF_FRAME_WINDOW_TOP_PCT) / 100);
      expect(side * cqhInCqw, body).toBeGreaterThan((width * (100 - 2 * OUT_OF_FRAME_WINDOW_SIDE_PCT)) / 100);
      expect(side * cqhInCqw, body).toBeLessThanOrEqual(width);
      expect(foot, body).toBeGreaterThan(lengthOf(banner, "top", "cqh"));
      expect(foot, body).toBeLessThanOrEqual(lengthOf(banner, "top", "cqh") + lengthOf(banner, "height", "cqh"));
    }
  });
});

describe("R2943001 Set showcase's name banners", () => {
  /** sRGB relative luminance of `hex`. */
  function luminance(hex: string): number {
    const channel = (at: number): number => {
      const c = parseInt(hex.slice(at, at + 2), 16) / 255;
      return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
    };
    return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
  }

  /** WCAG 2.2 AA's contrast for text (1.4.3). */
  const AA_TEXT_CONTRAST = 4.5;

  it("R2943001 each Set showcase name banner keeps its name at AA contrast", () => {
    for (const family of FAMILIES) {
      const body = bodyOf(cardsCss, `.cf[data-showcase="${family}"]:not([data-radiant-face="true"]) .card-name`);
      const background = PALETTE.get(/(?:^|[;\s])background:\s*var\((--pal-[a-z0-9-]+)\);/.exec(body)?.[1] ?? "");
      const ink = PALETTE.get(/(?:^|[;\s])color:\s*var\((--pal-[a-z0-9-]+)\);/.exec(body)?.[1] ?? "");
      expect(background, family).toMatch(/^#[0-9a-f]{6}$/i);
      expect(ink, family).toMatch(/^#[0-9a-f]{6}$/i);
      const [light, dark] = [luminance(background ?? ""), luminance(ink ?? "")].sort((a, b) => b - a);
      expect(((light ?? 0) + 0.05) / ((dark ?? 0) + 0.05), family).toBeGreaterThanOrEqual(AA_TEXT_CONTRAST);
      // The banner's shape and lettering are the set's on either face; its colours a base face's alone.
      expect(rulesMatching(cardsCss, new RegExp(`^\\.cf\\[data-showcase="${family}"\\] \\.card-name$`)), family).toHaveLength(1);
    }
    // Nothing sets the type's panel colour or a size of its own: the name's fitting stands.
    for (const rule of rulesMatching(cardsCss, /\[data-showcase=|\[data-version="version-out-of-frame"\]/)) {
      expect(rule.body, rule.selector).not.toMatch(/--cf-frame\s*:|--cf-type\s*:|font-size\s*:/);
    }
  });
});

describe("R2943002 the augments on part 4's versions", () => {
  /** Two Units offered both part 4 versions and Animated (Galaxy, Stinkyscape's painter; BOOM! Big Max) and a Spell offered Set showcase and Animated (Transmogulate). */
  const CARDS = ["stinkyscape-080", "classic-080", "core-083"] as const;
  const AUGMENTS: readonly (readonly string[])[] = [[], [AUGMENT_ANIMATED_ID], [AUGMENT_SIGNATURE_ID], [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID]];

  it("R2943002 Animated, a foil, a finish and a border layer on Set showcase and Out of frame, and Signature on none", () => {
    for (const id of CARDS) {
      expect(offeredOn(AUGMENT_ANIMATED_ID, id), id).toBe(true);
      const versions = PART_FOUR.filter((version) => offeredOn(version, id));
      expect(versions, id).toEqual(id === "core-083" ? [VERSION_SET_SHOWCASE_ID] : [...PART_FOUR]);
      for (const version of versions) {
        for (const augments of AUGMENTS) {
          writeCardVersion(id, version, augments);
          writeCardPick(id, "border", "border-runes");
          writeCardPick(id, "finish", "finish-stained-glass");
          writeCardPick(id, "foil", "foil-galaxy");
          const cf = faceIn(id, false);
          const where = `${id} ${version} ${augments.join("+")}`;
          const animated = augments.includes(AUGMENT_ANIMATED_ID);
          // R2949000: Signature fits neither; the store keeps none.
          expect(augmentsOf(lookOf(version, augments)).includes(AUGMENT_SIGNATURE_ID), where).toBe(false);
          expect(cf.getAttribute("data-version"), where).toBe(version);
          expect(cf.getAttribute("data-full-art"), where).toBeNull();
          expect(cf.getAttribute("data-augments"), where).toBe(animated ? AUGMENT_ANIMATED_ID : null);
          expect(cf.querySelector(".card-text"), where).not.toBeNull();
          expect(cf.querySelector(".card-name"), where).not.toBeNull();
          // Animated moves the version's own picture, taking a slot: Out of frame's window and its cutout both.
          expect(cf.getAttribute("data-art-motion"), where).toBe(animated ? "animated" : null);
          if (animated) expect(pictureOf(cf), where).toContain("art-m-");
          const cutout = cutoutOf(cf);
          expect(cutout !== null, where).toBe(version === VERSION_OUT_OF_FRAME_ID);
          if (cutout !== null && animated) expect(drawnBy(cutout), where).toContain("art-m-subject");
          expect(cf.classList.contains("cf-full-art--animated"), where).toBe(false);
          expect(cf.querySelector(".cf-signature"), where).toBeNull();
          expect(cf.classList.contains("cf-full-art--signed"), where).toBe(false);
          expect(cf.getAttribute("data-frame"), where).toBe("border-runes");
          expect(cf.getAttribute("data-finish"), where).toBe("finish-stained-glass");
          expect(cf.getAttribute("data-cosmetic-foil"), where).toBe("foil-galaxy");
          cleanup();
        }
      }
    }
  });

  it("R2943002 a worn border keeps its edge and bezel on Set showcase, and Out of frame sets neither", () => {
    for (const family of FAMILIES) {
      const frames = rulesMatching(cardsCss, new RegExp(`\\[data-showcase="${family}"\\]`));
      const setting = frames.filter((rule) => /--cf-(bezel|edge)\s*:/.test(rule.body));
      expect(setting, family).toHaveLength(1);
      for (const rule of setting) {
        expect(rule.selector, rule.selector).toContain(":not([data-frame])");
        expect(rule.selector, rule.selector).toContain(':not([data-radiant-face="true"])');
      }
      // The window's ring takes the bezel, a border's where one is worn, and only on a base face.
      const rings = frames.filter((rule) => rule.selector.endsWith(".cf-art-frame > .cf-art"));
      expect(rings, family).toHaveLength(1);
      for (const rule of rings) {
        expect(rule.selector, rule.selector).toContain(':not([data-radiant-face="true"])');
        expect(rule.body, rule.selector).toContain("var(--cf-bezel)");
      }
    }
    // Out of frame draws no frame of its own: a border's and a Radiant face's stand, and the cutout takes no foil over
    // its empty corners.
    const out = rulesMatching(cardsCss, /\[data-version="version-out-of-frame"\]/);
    expect(out.length).toBeGreaterThan(2);
    for (const rule of out) expect(rule.body, rule.selector).not.toMatch(/--cf-(bezel|edge)\s*:/);
    expect(out.some((rule) => rule.selector.includes(".cf-art--cutout::after") && rule.body.includes("content: none;"))).toBe(true);
  });
});

describe("R2943001 Radiant under part 4's looks", () => {
  /** Two Units offered both part 4 versions and Animated, and a Spell offered Set showcase and Animated. */
  const CARDS = ["stinkyscape-080", "classic-080", "core-083"] as const;

  it('R2943001 a Radiant face keeps data-radiant-face, data-art-variant="radiant", R277 marks and gold stats under every part 4 look; a base face has none', () => {
    for (const id of CARDS) {
      const def = CATALOG[id];
      if (def === undefined) throw new Error(`no ${id}`);
      const marked = faceModel({ defId: id, def, radiant: true }).text.marks.length > 0;
      const family = showcaseFamilyOf(def.set);
      for (const version of PART_FOUR.filter((each) => offeredOn(each, id))) {
        for (const augments of [[], [AUGMENT_ANIMATED_ID]]) {
          writeCardVersion(id, version, augments);
          for (const radiant of [false, true]) {
            const cf = faceIn(id, radiant);
            const where = `${id} ${radiant ? "Radiant" : "base"} ${version} ${augments.join("+")}`;
            // The gold-foil frame, and the art's Radiant paint: washed back over a Set showcase but Core's, which keeps
            // its colours, and on the cutout's subject too.
            expect(cf.getAttribute("data-radiant-face"), where).toBe(radiant ? "true" : null);
            for (const art of cf.querySelectorAll(".cf-art")) expect(art.getAttribute("data-art-variant"), where).toBe(radiant ? "radiant" : "base");
            const drawn = pictureOf(cf);
            expect(drawn.includes('result="sc-wash"'), where).toBe(radiant && version === VERSION_SET_SHOWCASE_ID && family !== "core");
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
    }
  });
});
