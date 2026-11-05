// R2944001 (#2944): the faces of the Immersive and Prismatic versions: every card each is offered draws it, base and
// Radiant, apart from its normal face and every other version; each keeps every part of the normal face. Immersive draws
// its window's backdrop hazed as far off and its subject on a near layer over it, the window's own box and shape;
// Prismatic draws its picture in chrome under a sheen and, on a base face, a rainbow edge, a chrome bezel and a chrome
// banner. R2944002: Animated, a foil, a finish and a border layer on each, and Signature on none; Immersive's near layer
// follows the pointer and the tilt and breathes while it holds a motion slot, and holds still under reduced motion, with
// Animated foil off and on a lazy face. R2725004 holds under each: a Radiant face keeps every Radiant mark, a base face
// shows none. The pictures themselves are art/pictures.test.ts's; the pixels are e2e's
// cypress/component/card-versions.cy.tsx.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { SHIPPED_SETS } from "@jackioh/shared";

import { COSMETIC_OFFERS } from "../wire/generated/cosmetics.ts";
import { PRISMATIC_SHEEN } from "./art/pictures.ts";
import { CardFace } from "./CardFace.tsx";
import {
  COSMETIC_FOIL_ANIMATED_MAX,
  IMMERSIVE_BREATH_S,
  IMMERSIVE_BREATH_SCALE,
  IMMERSIVE_SHIFT_PCT,
  IMMERSIVE_TILT_RANGE_DEG,
  IMMERSIVE_TILT_REST_DEG,
} from "./constants.ts";
import { resetCosmeticsForTests, writeCardPick, writeCardVersion } from "./cosmetics.ts";
import { IMMERSIVE_X_PROPERTY, IMMERSIVE_Y_PROPERTY, pointerShift, tiltShift } from "./immersive.ts";
import { faceModel } from "./model.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "./settings.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { setReducedMotion } from "../test/setup.ts";
import {
  AUGMENT_ANIMATED_ID,
  AUGMENT_SIGNATURE_ID,
  VERSION_ALT_ART_ID,
  VERSION_ALT_FULL_ART_ID,
  VERSION_COMIC_ID,
  VERSION_EXTENDED_ART_ID,
  VERSION_FULL_ART_ID,
  VERSION_IMMERSIVE_ID,
  VERSION_OUT_OF_FRAME_ID,
  VERSION_PIXEL_ID,
  VERSION_PRISMATIC_ID,
  VERSION_SET_SHOWCASE_ID,
  VERSION_SKETCH_ID,
  augmentsOf,
  isFullBleed,
  lookOf,
  offeredOn,
} from "./versions.ts";

/** R2944000: #2944's two versions, in the registry's order. */
const PART_FIVE = [VERSION_IMMERSIVE_ID, VERSION_PRISMATIC_ID] as const;
/** Every version a card may wear but its normal face. */
const VERSIONS = [
  VERSION_FULL_ART_ID,
  VERSION_ALT_ART_ID,
  VERSION_SKETCH_ID,
  VERSION_ALT_FULL_ART_ID,
  VERSION_EXTENDED_ART_ID,
  VERSION_PIXEL_ID,
  VERSION_COMIC_ID,
  VERSION_SET_SHOWCASE_ID,
  VERSION_OUT_OF_FRAME_ID,
  ...PART_FIVE,
] as const;
/** A Mythic Unit offered both part 5 versions, Animated and Signature (Ceaseless Void). */
const BOTH = "core-100";

const HERE = dirname(fileURLToPath(import.meta.url));
const cardsCss = readFileSync(join(HERE, "cards.css"), "utf8");
const paletteCss = readFileSync(join(HERE, "..", "palette.css"), "utf8");
/** palette.css's `--pal-*` colours by name. */
const PALETTE = new Map([...paletteCss.matchAll(/(--pal-[a-z0-9-]+):\s*(#[0-9a-fA-F]{6})/g)].map((match) => [match[1] ?? "", match[2] ?? ""]));

function faceIn(id: string, radiant: boolean, lazyArt = false): HTMLElement {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`no ${id}`);
  const { container } = render(<CardFace face={faceModel({ defId: id, def, radiant })} layout="full" worn lazyArt={lazyArt} />);
  const cf = container.querySelector<HTMLElement>(".cf");
  if (cf === null) throw new Error(`no face for ${id}`);
  return cf;
}

/** A picture's CSS background, decoded. */
function drawnBy(art: HTMLElement | null | undefined): string {
  return decodeURIComponent(art?.style.backgroundImage ?? "");
}

/** The art window: the art frame's own picture (the backdrop on an Immersive face). */
function windowOf(cf: HTMLElement): HTMLElement | null {
  return cf.querySelector<HTMLElement>(".cf-art-frame > .cf-art:not(.cf-art--cutout)");
}

/** The art window's picture: what its CSS background draws. */
function pictureOf(cf: HTMLElement): string {
  return drawnBy(windowOf(cf));
}

/** The Immersive face's near layer, or null. */
function nearOf(cf: HTMLElement): HTMLElement | null {
  return cf.querySelector<HTMLElement>(".cf-art-frame > .cf-immersive > .cf-art--near");
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

/** The motion gate's opening, a media query no reduced motion passes. */
const GATE_OPEN = "@media (prefers-reduced-motion: no-preference) {";

/** `css` split into what lies inside its motion gates and what lies outside them. */
function gated(css: string): { inside: string; outside: string } {
  let inside = "";
  let outside = "";
  let from = 0;
  for (let at = css.indexOf(GATE_OPEN); at >= 0; at = css.indexOf(GATE_OPEN, from)) {
    outside += css.slice(from, at);
    let depth = 0;
    let end = at + GATE_OPEN.length - 1;
    for (; end < css.length; end += 1) {
      if (css[end] === "{") depth += 1;
      if (css[end] === "}") depth -= 1;
      if (depth === 0) break;
    }
    inside += css.slice(at + GATE_OPEN.length, end);
    from = end + 1;
  }
  return { inside, outside: outside + css.slice(from) };
}

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
  setReducedMotion(false);
});

describe("R2944001 every card offered a part 5 version draws it", () => {
  /** The cards offered either of #2944's versions, by set. */
  const BY_SET = new Map<string, string[]>();
  for (const id of new Set(PART_FIVE.flatMap((version) => COSMETIC_OFFERS[version] ?? []))) {
    const set = CATALOG[id]?.set ?? "";
    BY_SET.set(set, [...(BY_SET.get(set) ?? []), id]);
  }

  it("R2944001 each part 5 version is offered in every set that ships", () => {
    for (const version of PART_FIVE) {
      const sets = new Set((COSMETIC_OFFERS[version] ?? []).map((id) => CATALOG[id]?.set));
      expect([...sets].sort(), version).toEqual([...SHIPPED_SETS].sort());
    }
  });

  /** How many offered cards one test draws (up to 24 faces each), so that no test nears the timeout on a loaded runner. */
  const OFFERED_CARDS_PER_TEST = 8;
  for (const [set, ids] of BY_SET) {
    for (let from = 0; from < ids.length; from += OFFERED_CARDS_PER_TEST) {
      const some = ids.slice(from, from + OFFERED_CARDS_PER_TEST);
      it(`R2944001 every ${set} card ${some[0] ?? ""} to ${some.at(-1) ?? ""} draws each version it is offered, base and Radiant, apart from its normal face and every other version`, () => {
        drawsEachVersion(some);
      });
    }
  }

  /** R2944001: each of `ids` draws its normal face and each version it is offered, every face apart. */
  function drawsEachVersion(ids: readonly string[]): void {
    for (const id of ids) {
      for (const radiant of [false, true]) {
        const where = `${id} ${radiant ? "Radiant" : "base"}`;
        writeCardVersion(id, false, []);
        const normal = faceIn(id, radiant);
        expect(normal.querySelector(".cf-immersive"), where).toBeNull();
        const faces = new Map<string, { markup: string; picture: string }>([["normal", { markup: normal.outerHTML, picture: pictureOf(normal) }]]);
        cleanup();
        for (const version of VERSIONS.filter((each) => offeredOn(each, id))) {
          const at = `${where} ${version}`;
          writeCardVersion(id, version, []);
          const cf = faceIn(id, radiant);
          const art = windowOf(cf);
          expect(cf.getAttribute("data-version"), at).toBe(version);
          expect(cf.getAttribute("data-full-art"), at).toBe(isFullBleed(version) ? "true" : null);
          expect(art?.getAttribute("data-art-variant"), at).toBe(radiant ? "radiant" : "base");
          expect(cf.querySelector(".cf-signature"), at).toBeNull();
          const drawn = pictureOf(cf);
          expect(drawn, at).not.toBe("");
          for (const banned of ["NaN", "undefined", "<text", "<title", "href"]) expect(drawn, `${at} ${banned}`).not.toContain(banned);
          if ((PART_FIVE as readonly string[]).includes(version)) {
            // Every part of the normal face is kept: the cost gem, the name, the type line and the rules box.
            for (const part of [".cost-gem", ".card-name", ".card-type", ".card-text"]) expect(cf.querySelector(part), `${at} ${part}`).not.toBeNull();
            expect(cf.getAttribute("data-full-art"), at).toBeNull();
          }
          if (version === VERSION_IMMERSIVE_ID) {
            // The window draws the backdrop hazed, and the near layer the subject over it, in the window's own shape.
            expect(art?.getAttribute("data-art-picture"), at).toBe("depth");
            expect(art?.getAttribute("data-art-part"), at).toBe("backdrop");
            expect(drawn, at).toContain('filter="url(#dp-haze)"');
            const near = nearOf(cf);
            const layer = near?.parentElement;
            const shape = [...(art?.classList ?? [])].find((name) => name.startsWith("cf-art--"));
            expect(shape, at).toBeDefined();
            expect(layer?.classList.contains(shape ?? ""), at).toBe(true);
            expect(layer?.getAttribute("aria-hidden"), at).toBe("true");
            expect(near?.getAttribute("data-art-part"), at).toBe("subject");
            expect(near?.getAttribute("data-art-picture"), at).toBeNull();
            expect(near?.getAttribute("data-art-variant"), at).toBe(radiant ? "radiant" : "base");
            const subject = drawnBy(near);
            expect(subject, at).not.toBe("");
            expect(subject, at).not.toBe(drawn);
            for (const banned of ["NaN", "undefined", "<text", "<title", "href"]) expect(subject, `${at} ${banned}`).not.toContain(banned);
            expect(cf.getAttribute("data-immersive-motion"), at).toBe("animated");
          } else {
            expect(cf.querySelector(".cf-immersive"), at).toBeNull();
            expect(cf.getAttribute("data-immersive-motion"), at).toBeNull();
          }
          if (version === VERSION_PRISMATIC_ID) {
            expect(art?.getAttribute("data-art-picture"), at).toBe("prismatic");
            expect(art?.getAttribute("data-art-part"), at).toBeNull();
            expect(drawn, at).toContain('filter="url(#pr-chrome)"');
            expect(drawn, at).toContain('fill="url(#pr-sheen)"');
          }
          faces.set(version, { markup: cf.outerHTML, picture: drawn });
          cleanup();
        }
        // Every face apart from every other, and each part 5 version's picture its own.
        expect(new Set([...faces.values()].map((face) => face.markup)).size, where).toBe(faces.size);
        for (const version of PART_FIVE) {
          const own = faces.get(version)?.picture;
          if (own === undefined) continue;
          for (const [other, face] of faces) if (other !== version) expect(own, `${where} ${version} beside ${other}`).not.toBe(face.picture);
        }
      }
    }
  }
});

describe("R2944002 Immersive's depth", () => {
  it("R2944002 Immersive's depth takes a motion slot and holds still with Animated foil off, under Reduce motion and on a lazy face", () => {
    expect(offeredOn(VERSION_IMMERSIVE_ID, BOTH) && offeredOn(AUGMENT_ANIMATED_ID, BOTH)).toBe(true);
    const def = CATALOG[BOTH];
    if (def === undefined) throw new Error(`no ${BOTH}`);
    const face = faceModel({ defId: BOTH, def, radiant: false });
    const grid = (): HTMLElement[] => {
      const { container } = render(
        <>
          {Array.from({ length: COSMETIC_FOIL_ANIMATED_MAX + 1 }, (_, index) => (
            <CardFace key={index} face={face} layout="full" worn />
          ))}
        </>,
      );
      return [...container.querySelectorAll<HTMLElement>(".cf")];
    };
    const split = [...Array<string>(COSMETIC_FOIL_ANIMATED_MAX).fill("animated"), "static"];
    // Past the slots the rest hold still, Immersive alone or with Animated, one slot a face: its depth and its art's
    // own motion move together.
    writeCardVersion(BOTH, VERSION_IMMERSIVE_ID, []);
    let faces = grid();
    expect(faces.map((cf) => cf.getAttribute("data-immersive-motion"))).toEqual(split);
    expect(faces.map((cf) => cf.getAttribute("data-art-motion"))).toEqual(faces.map(() => null));
    // Without Animated the art's own layers stay still: only the near layer moves, by cards.css.
    for (const cf of faces) expect(drawnBy(nearOf(cf)), "Immersive alone").not.toContain("art-m-");
    cleanup();
    writeCardVersion(BOTH, VERSION_IMMERSIVE_ID, [AUGMENT_ANIMATED_ID]);
    faces = grid();
    expect(faces.map((cf) => cf.getAttribute("data-immersive-motion"))).toEqual(split);
    expect(faces.map((cf) => cf.getAttribute("data-art-motion"))).toEqual(split);
    expect(faces.map((cf) => drawnBy(nearOf(cf)).includes("art-m-subject"))).toEqual(split.map((motion) => motion === "animated"));
    cleanup();

    // Each still: Animated foil off, the Reduce motion setting, the system's reduced motion and a lazy face.
    const stills: readonly (readonly [string, () => void])[] = [
      ["Animated foil off", () => writeCardSettings({ animatedFoil: false })],
      ["the Reduce motion setting", () => writeSettings({ reduceMotion: true })],
      ["the system's reduced motion", () => setReducedMotion(true)],
    ];
    for (const [why, still] of stills) {
      still();
      const cf = faceIn(BOTH, false);
      expect(cf.getAttribute("data-immersive-motion"), why).toBe("static");
      expect(cf.getAttribute("data-art-motion"), why).toBe("static");
      expect(nearOf(cf), why).not.toBeNull();
      cleanup();
      __resetSettingsForTests();
      setReducedMotion(false);
      writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
    }
    const lazy = faceIn(BOTH, false, true);
    expect(lazy.getAttribute("data-immersive-motion"), "a lazy face").toBe("static");
    expect(nearOf(lazy), "a lazy face").not.toBeNull();
  });

  it("R2944002 the near layer follows the pointer and the tilt only while it moves", () => {
    writeCardVersion(BOTH, VERSION_IMMERSIVE_ID, []);
    const cf = faceIn(BOTH, false);
    expect(cf.getAttribute("data-immersive-motion")).toBe("animated");
    const shift = (): [string, string] => [cf.style.getPropertyValue(IMMERSIVE_X_PROPERTY), cf.style.getPropertyValue(IMMERSIVE_Y_PROPERTY)];
    expect(shift()).toEqual(["", ""]);
    // The pointer at the viewport's top right edge: the far shift right and up.
    window.dispatchEvent(new MouseEvent("pointermove", { clientX: window.innerWidth, clientY: 0 }));
    expect(shift()).toEqual(["1.000", "-1.000"]);
    // A tilt half its range to the right, held at its rest forward.
    window.dispatchEvent(Object.assign(new Event("deviceorientation"), { gamma: IMMERSIVE_TILT_RANGE_DEG / 2, beta: IMMERSIVE_TILT_REST_DEG }));
    expect(shift()).toEqual(["0.500", "0.000"]);
    // A device that reports no tilt changes nothing.
    window.dispatchEvent(Object.assign(new Event("deviceorientation"), { gamma: null, beta: null }));
    expect(shift()).toEqual(["0.500", "0.000"]);
    // Unmounted, it clears its shift and listens no more.
    cleanup();
    expect(shift()).toEqual(["", ""]);
    window.dispatchEvent(new MouseEvent("pointermove", { clientX: 0, clientY: 0 }));
    expect(shift()).toEqual(["", ""]);
    // A still face never listens.
    writeSettings({ reduceMotion: true });
    const still = faceIn(BOTH, false);
    expect(still.getAttribute("data-immersive-motion")).toBe("static");
    window.dispatchEvent(new MouseEvent("pointermove", { clientX: window.innerWidth, clientY: 0 }));
    expect(still.style.getPropertyValue(IMMERSIVE_X_PROPERTY)).toBe("");

    // The shifts themselves: held to -1 to 1, none along a side of no length or for no tilt.
    expect(pointerShift(50, 25, 100, 100)).toEqual({ x: 0, y: -0.5 });
    expect(pointerShift(500, -500, 100, 100)).toEqual({ x: 1, y: -1 });
    expect(pointerShift(30, 30, 0, 0)).toEqual({ x: 0, y: 0 });
    expect(tiltShift(null, IMMERSIVE_TILT_REST_DEG)).toBeNull();
    expect(tiltShift(0, null)).toBeNull();
    expect(tiltShift(-IMMERSIVE_TILT_RANGE_DEG * 3, IMMERSIVE_TILT_REST_DEG + IMMERSIVE_TILT_RANGE_DEG)).toEqual({ x: -1, y: 1 });
    expect(tiltShift(Number.NaN, IMMERSIVE_TILT_REST_DEG)).toEqual({ x: 0, y: 0 });
  });

  it("R2944002 cards.css mirrors the shift and breath, inside the motion gate alone", () => {
    const { inside, outside } = gated(cardsCss);
    const moving = bodyOf(inside, ':root:not([data-reduce-motion="true"]) .cf[data-immersive-motion="animated"] .cf-immersive > .cf-art');
    const pct = String(IMMERSIVE_SHIFT_PCT);
    expect(moving).toContain(`translate: calc(var(${IMMERSIVE_X_PROPERTY}, 0) * ${pct}%) calc(var(${IMMERSIVE_Y_PROPERTY}, 0) * ${pct}%);`);
    expect(moving).toContain(`animation: cf-immersive-breathe ${String(IMMERSIVE_BREATH_S)}s ease-in-out infinite alternate;`);
    const breath = /@keyframes cf-immersive-breathe\s*\{\s*from\s*\{\s*scale:\s*1;\s*\}\s*to\s*\{\s*scale:\s*([\d.]+);\s*\}\s*\}/.exec(cardsCss);
    expect(Number(breath?.[1])).toBe(IMMERSIVE_BREATH_SCALE);
    // Outside the gate the near layer never moves, and nothing outside Immersive's own rules names its layer.
    const still = rulesMatching(outside, /cf-immersive/);
    expect(still.length).toBeGreaterThan(3);
    for (const rule of still) {
      expect(rule.body, rule.selector).not.toMatch(/(?:^|[;\s])(animation|translate|transition|transform)\s*:/);
      expect(rule.selector, rule.selector).toMatch(/^\.cf\[data-version="version-immersive"\] /);
    }
    // The near layer takes no ground, ring, foil or pointer, and its box is the window's.
    expect(bodyOf(cardsCss, '.cf[data-version="version-immersive"] .cf-art-frame > .cf-immersive')).toMatch(/inset: 0;[^]*overflow: hidden;[^]*pointer-events: none;/);
    expect(bodyOf(cardsCss, '.cf[data-version="version-immersive"] .cf-immersive > .cf-art')).toMatch(/inset: 0;[^]*background-color: transparent;[^]*box-shadow: none;/);
    expect(bodyOf(cardsCss, '.cf[data-version="version-immersive"] .cf-immersive > .cf-art::before,\n.cf[data-version="version-immersive"] .cf-immersive > .cf-art::after')).toContain("content: none;");
  });
});

describe("R2944002 the augments on part 5's versions", () => {
  /** A Stinkyscape Mythic Unit (Tylannt), a procedural Mythic Unit (Ceaseless Void) and a procedural Mythic Spell (Zephrys). */
  const CARDS = ["stinkyscape-006", "core-100", "core-097"] as const;
  const AUGMENTS: readonly (readonly string[])[] = [[], [AUGMENT_ANIMATED_ID], [AUGMENT_SIGNATURE_ID], [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID]];

  it("R2944002 Animated, a foil, a finish and a border layer on Immersive and Prismatic, and Signature on none", () => {
    for (const id of CARDS) {
      expect(offeredOn(AUGMENT_ANIMATED_ID, id), id).toBe(true);
      const versions = PART_FIVE.filter((version) => offeredOn(version, id));
      expect(versions, id).toEqual([...PART_FIVE]);
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
          // Animated moves the version's own picture, taking a slot: on Immersive the hazed backdrop and the near
          // subject both, in the one slot its depth holds.
          expect(cf.getAttribute("data-art-motion"), where).toBe(animated ? "animated" : null);
          if (animated) expect(pictureOf(cf), where).toContain("art-m-");
          const near = nearOf(cf);
          expect(near !== null, where).toBe(version === VERSION_IMMERSIVE_ID);
          expect(cf.getAttribute("data-immersive-motion"), where).toBe(version === VERSION_IMMERSIVE_ID ? "animated" : null);
          if (near !== null && animated) expect(drawnBy(near), where).toContain("art-m-subject");
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

  it("R2944001 Prismatic keeps a worn border's edge and bezel and its banner at AA contrast, and Immersive sets neither", () => {
    const prismatic = rulesMatching(cardsCss, /\[data-version="version-prismatic"\]/);
    const setting = prismatic.filter((rule) => /--cf-(bezel|edge)\s*:/.test(rule.body));
    expect(setting).toHaveLength(1);
    for (const rule of setting) {
      expect(rule.selector, rule.selector).toContain(":not([data-frame])");
      expect(rule.selector, rule.selector).toContain(':not([data-radiant-face="true"])');
      // A rainbow edge of the sheen's own colours, there and back, and a chrome bezel.
      const edge = /--cf-edge:\s*conic-gradient\(([^;]*)\);/.exec(rule.body)?.[1] ?? "";
      const stops = [...edge.matchAll(/var\((--pal-[a-z0-9-]+)\)/g)].map((match) => PALETTE.get(match[1] ?? ""));
      expect(stops).toEqual([...PRISMATIC_SHEEN]);
      expect(PALETTE.get(/--cf-bezel:\s*var\((--pal-[a-z0-9-]+)\);/.exec(rule.body)?.[1] ?? "")).toMatch(/^#[0-9a-f]{6}$/i);
    }
    // The chrome banner on a base face alone, its name at AA contrast against every stop of its chrome.
    const banner = bodyOf(cardsCss, '.cf[data-version="version-prismatic"]:not([data-radiant-face="true"]) .card-name');
    const backdrop = /(?:^|[;\s])background:\s*([^;]*);/.exec(banner)?.[1] ?? "";
    const stops = [...backdrop.matchAll(/var\((--pal-[a-z0-9-]+)\)/g)].map((match) => PALETTE.get(match[1] ?? "") ?? "");
    const ink = PALETTE.get(/(?:^|[;\s])color:\s*var\((--pal-[a-z0-9-]+)\);/.exec(banner)?.[1] ?? "") ?? "";
    expect(stops.length).toBeGreaterThan(1);
    expect(ink).toMatch(/^#[0-9a-f]{6}$/i);
    for (const stop of stops) {
      expect(stop).toMatch(/^#[0-9a-f]{6}$/i);
      const [light, dark] = [luminance(stop), luminance(ink)].sort((a, b) => b - a);
      expect(((light ?? 0) + 0.05) / ((dark ?? 0) + 0.05), stop).toBeGreaterThanOrEqual(AA_TEXT_CONTRAST);
    }
    // Nothing sets the type's panel colour or a size of its own: the name's fitting stands.
    for (const rule of rulesMatching(cardsCss, /\[data-version="version-(immersive|prismatic)"\]/)) {
      expect(rule.body, rule.selector).not.toMatch(/--cf-frame\s*:|--cf-type\s*:|font-size\s*:/);
    }
    // Immersive draws no frame of its own: a border's and a Radiant face's stand.
    const immersive = rulesMatching(cardsCss, /\[data-version="version-immersive"\]/);
    expect(immersive.length).toBeGreaterThan(3);
    for (const rule of immersive) expect(rule.body, rule.selector).not.toMatch(/--cf-(bezel|edge)\s*:/);
  });
});

describe("R2944001 Radiant under part 5's looks", () => {
  /** Two Mythic Units and a Mythic Spell offered both part 5 versions and Animated. */
  const CARDS = ["stinkyscape-006", "core-100", "core-097"] as const;

  it('R2944001 a Radiant face keeps data-radiant-face, data-art-variant="radiant", R277 marks and gold stats under every part 5 look; a base face has none', () => {
    for (const id of CARDS) {
      const def = CATALOG[id];
      if (def === undefined) throw new Error(`no ${id}`);
      const marked = faceModel({ defId: id, def, radiant: true }).text.marks.length > 0;
      for (const version of PART_FIVE.filter((each) => offeredOn(each, id))) {
        for (const augments of [[], [AUGMENT_ANIMATED_ID]]) {
          writeCardVersion(id, version, augments);
          for (const radiant of [false, true]) {
            const cf = faceIn(id, radiant);
            const where = `${id} ${radiant ? "Radiant" : "base"} ${version} ${augments.join("+")}`;
            // The gold-foil frame, and the art's Radiant paint: washed back over the haze and the chrome, and on the
            // near layer's subject, drawn as it is.
            expect(cf.getAttribute("data-radiant-face"), where).toBe(radiant ? "true" : null);
            const arts = cf.querySelectorAll(".cf-art");
            expect(arts.length, where).toBe(version === VERSION_IMMERSIVE_ID ? 2 : 1);
            for (const art of arts) expect(art.getAttribute("data-art-variant"), where).toBe(radiant ? "radiant" : "base");
            const drawn = pictureOf(cf);
            expect(drawn.includes('result="dp-wash"'), where).toBe(radiant && version === VERSION_IMMERSIVE_ID);
            expect(drawn.includes('result="pr-wash"'), where).toBe(radiant && version === VERSION_PRISMATIC_ID);
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
