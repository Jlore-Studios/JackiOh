// R2940001 (#2940): the faces of the Alt art, Sketch and Alt full art versions: every card each is offered draws it,
// base and Radiant, apart from its normal face and every other version; Alt full art prints the full-art layout with
// Alt art's picture. R2940002: a foil, a finish and a border layer on each. R2725004 holds under each: a Radiant face
// keeps every Radiant mark, a base face shows none. R3015000 (#3015): Alt art and the Sketch are ladders, each a full
// art, animated and signed above its first level; R3015002: every level draws its first level's picture (the Sketch's
// full art the sketch); R3015005: an older pick of Alt art or the Sketch with Animated wears its first level. The
// pictures themselves are art/pictures.test.ts's; the pixels are e2e's cypress/component/card-versions.cy.tsx.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { SHIPPED_SETS } from "@jackioh/shared";

import { COSMETIC_OFFERS } from "../wire/generated/cosmetics.ts";
import { CardFace } from "./CardFace.tsx";
import { CARD_PICKS_SETTINGS_KEY, resetCosmeticsForTests, writeCardLevel, writeCardPick } from "./cosmetics.ts";
import { faceModel } from "./model.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "./settings.ts";
import { __resetSettingsForTests } from "../settings/store.ts";
import {
  AUGMENT_ANIMATED_ID,
  AUGMENT_SIGNATURE_ID,
  VERSION_ALT_ART_ID,
  VERSION_ALT_FULL_ART_ID,
  VERSION_FULL_ART_ID,
  VERSION_SKETCH_FULL_ART_ID,
  VERSION_SKETCH_ID,
  artPictureOf,
  isFullBleed,
  ladderOf,
  levelOf,
  lookOfLevel,
  offeredOn,
} from "./versions.ts";
import { facePicture } from "../test/artPicture.ts";
import { legacyText } from "../test/faceData.ts";

/** R2940000: #2940's three versions, in the registry's order. */
const NEW_VERSIONS = [VERSION_ALT_ART_ID, VERSION_SKETCH_ID, VERSION_ALT_FULL_ART_ID] as const;
/** Every still level of part 1's and part 2's variants but the normal face (R3015000: the Sketch's full art among them). */
const VERSIONS = [VERSION_FULL_ART_ID, ...NEW_VERSIONS, VERSION_SKETCH_FULL_ART_ID] as const;

/** R3015005: `id`'s pick as a client stored it before #3015, a version and the augments layered on it, read afresh. */
function storeOlderPick(id: string, fullArt: string, augments: readonly string[]): void {
  localStorage.setItem(CARD_PICKS_SETTINGS_KEY, JSON.stringify({ [id]: { fullArt, augments } }));
  resetCosmeticsForTests();
}

function faceIn(id: string, radiant: boolean): HTMLElement {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`no ${id}`);
  const { container } = render(<CardFace face={faceModel({ defId: id, def, radiant })} layout="full" worn />);
  const cf = container.querySelector<HTMLElement>(".cf");
  if (cf === null) throw new Error(`no face for ${id}`);
  return cf;
}

/** The picture the face draws in its art window, decoded (R3344000: the planes' together, on a face with depth, test/artPicture.ts). */
function pictureOf(cf: HTMLElement): string {
  return facePicture(cf);
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

describe("R2940001 every card offered a new version draws it", () => {
  /** The cards offered any of #2940's versions, by set. */
  const BY_SET = new Map<string, string[]>();
  for (const id of new Set(NEW_VERSIONS.flatMap((version) => COSMETIC_OFFERS[version] ?? []))) {
    const set = CATALOG[id]?.set ?? "";
    BY_SET.set(set, [...(BY_SET.get(set) ?? []), id]);
  }

  it("R2940001 each new version is offered in every set that ships", () => {
    for (const version of NEW_VERSIONS) {
      const sets = new Set((COSMETIC_OFFERS[version] ?? []).map((id) => CATALOG[id]?.set));
      expect([...sets].sort(), version).toEqual([...SHIPPED_SETS].sort());
    }
  });

  /** How many offered cards one test draws (up to ten faces each), so that no test nears the timeout on a loaded runner. */
  const OFFERED_CARDS_PER_TEST = 12;
  for (const [set, ids] of BY_SET) {
    for (let from = 0; from < ids.length; from += OFFERED_CARDS_PER_TEST) {
      const some = ids.slice(from, from + OFFERED_CARDS_PER_TEST);
      it(`R2940001 every ${set} card ${some[0] ?? ""} to ${some.at(-1) ?? ""} draws each version it is offered, base and Radiant, apart from its normal face and every other version`, () => {
        drawsEachVersion(some);
      });
    }
  }

  /** R2940001: each of `ids` draws its normal face and each version it is offered, every face and picture apart. */
  function drawsEachVersion(ids: readonly string[]): void {
    for (const id of ids) {
      for (const radiant of [false, true]) {
        const where = `${id} ${radiant ? "Radiant" : "base"}`;
        writeCardLevel(id, null);
        const normal = faceIn(id, radiant);
        const faces = new Map<string, { markup: string; picture: string }>([["normal", { markup: normal.outerHTML, picture: pictureOf(normal) }]]);
        cleanup();
        for (const version of VERSIONS.filter((each) => offeredOn(each, id))) {
          const at = `${where} ${version}`;
          writeCardLevel(id, version);
          const cf = faceIn(id, radiant);
          const art = cf.querySelector(".cf-art");
          expect(cf.getAttribute("data-version"), at).toBe(version);
          expect(cf.getAttribute("data-level"), at).toBe(version);
          expect(cf.getAttribute("data-full-art"), at).toBe(isFullBleed(version) ? "true" : null);
          // Full bleed has no words; Alt art and the Sketch keep the normal frame's type line and rules box.
          expect(cf.querySelector(".card-type") === null, at).toBe(isFullBleed(version));
          expect(art?.getAttribute("data-art-variant"), at).toBe(radiant ? "radiant" : "base");
          const picture = artPictureOf(version);
          expect(art?.getAttribute("data-art-picture"), at).toBe(picture === "normal" ? null : picture);
          expect(cf.querySelector(".cf-signature"), at).toBeNull();
          const drawn = pictureOf(cf);
          expect(drawn, at).not.toBe("");
          for (const banned of ["NaN", "undefined", "<text", "<title", "href"]) expect(drawn, `${at} ${banned}`).not.toContain(banned);
          faces.set(version, { markup: cf.outerHTML, picture: drawn });
          cleanup();
        }
        // Every face apart from every other.
        expect(new Set([...faces.values()].map((face) => face.markup)).size, where).toBe(faces.size);
        const picture = (key: string): string | undefined => faces.get(key)?.picture;
        const plain = picture("normal");
        // Full art's picture is the normal one; Alt art's and the Sketch's are their own, and each other's neither.
        if (faces.has(VERSION_FULL_ART_ID)) expect(picture(VERSION_FULL_ART_ID), where).toBe(plain);
        for (const version of NEW_VERSIONS) if (faces.has(version)) expect(picture(version), `${where} ${version}`).not.toBe(plain);
        if (faces.has(VERSION_ALT_ART_ID) && faces.has(VERSION_SKETCH_ID)) expect(picture(VERSION_SKETCH_ID), where).not.toBe(picture(VERSION_ALT_ART_ID));
        if (faces.has(VERSION_SKETCH_ID) && faces.has(VERSION_ALT_FULL_ART_ID)) expect(picture(VERSION_SKETCH_ID), where).not.toBe(picture(VERSION_ALT_FULL_ART_ID));
        // Alt full art is Alt art's picture across the whole face; R3015002: the Sketch's full art the sketch.
        if (faces.has(VERSION_ALT_ART_ID) && faces.has(VERSION_ALT_FULL_ART_ID)) expect(picture(VERSION_ALT_FULL_ART_ID), where).toBe(picture(VERSION_ALT_ART_ID));
        if (faces.has(VERSION_SKETCH_ID) && faces.has(VERSION_SKETCH_FULL_ART_ID)) expect(picture(VERSION_SKETCH_FULL_ART_ID), where).toBe(picture(VERSION_SKETCH_ID));
      }
    }
  }
});

describe("R2940002 R3015000 the levels of Alt art and the Sketch", () => {
  /**
   * Cards offered every level of Alt art and the Sketch: The Mother Pancake (a Legendary Unit), Transmogulate (a
   * Legendary Spell) and Fear Mongerer (an Epic Unit, not offered Full art).
   */
  const CARDS = ["classicplus-012", "core-083", "meditative-012"] as const;
  /** R3015000: each level of the two ladders, lowest first. */
  const LEVELS: readonly string[] = [...ladderOf("alt-art"), ...ladderOf("sketch")];

  it("R2940002 R3015002 a foil, a finish and a border layer at every level of Alt art and the Sketch, each its variant's one picture, moving and signed only from its full art up", () => {
    for (const id of CARDS) {
      for (const level of LEVELS) expect(offeredOn(level, id), `${level} on ${id}`).toBe(true);
      for (const level of LEVELS) {
        writeCardLevel(id, level);
        writeCardPick(id, "border", "border-runes");
        writeCardPick(id, "finish", "finish-stained-glass");
        writeCardPick(id, "foil", "foil-galaxy");
        const cf = faceIn(id, false);
        const where = `${id} ${level}`;
        const look = lookOfLevel(level);
        const full = isFullBleed(level);
        expect(cf.getAttribute("data-version"), where).toBe(look.version);
        expect(cf.getAttribute("data-level"), where).toBe(level);
        expect(cf.getAttribute("data-full-art"), where).toBe(full ? "true" : null);
        expect(cf.querySelector(".card-text") === null, where).toBe(full);
        expect(cf.querySelector(".card-name"), where).not.toBeNull();
        // An animated level moves its variant's own picture, Alt art's or the Sketch's, taking a slot, and lays the band
        // on its full art; no level below the full art moves.
        expect(look.animated, where).toBe(full && (levelOf(level)?.rank ?? 0) >= 3);
        expect(cf.getAttribute("data-art-motion"), where).toBe(look.animated ? "animated" : null);
        const art = cf.querySelector(".cf-art");
        expect(art?.getAttribute("data-art-picture"), where).toBe(artPictureOf(level));
        if (look.animated) expect(pictureOf(cf), where).toContain("art-m-");
        if (levelOf(level)?.variant === "sketch") expect(pictureOf(cf), where).toContain('filter="url(#sk-pencil)"');
        expect(cf.classList.contains("cf-full-art--animated"), where).toBe(full && look.animated);
        expect(cf.getAttribute("data-full-art-motion"), where).toBe(full && look.animated ? "animated" : null);
        // R3015002: a signed level signs its variant's own full art, which draws no picture of its own.
        expect(cf.querySelector(".cf-signature") !== null, where).toBe(look.signed);
        expect(cf.classList.contains("cf-full-art--signed"), where).toBe(look.signed);
        expect(cf.getAttribute("data-frame"), where).toBe("border-runes");
        expect(cf.getAttribute("data-finish"), where).toBe("finish-stained-glass");
        expect(cf.getAttribute("data-cosmetic-foil"), where).toBe("foil-galaxy");
        cleanup();
      }
    }
  });

  it("R2940002 R3015005 an older pick of Alt art or the Sketch with Animated wears its first level, still, and Signature signs none of them", () => {
    for (const id of CARDS) {
      for (const version of [VERSION_ALT_ART_ID, VERSION_SKETCH_ID]) {
        for (const augments of [[AUGMENT_ANIMATED_ID], [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID]]) {
          const where = `${id} ${version} ${augments.join("+")}`;
          storeOlderPick(id, version, augments);
          const cf = faceIn(id, false);
          // Its second level is a full art, so the motion of a normal-frame Alt art or Sketch no longer exists: ruled.
          expect(cf.getAttribute("data-level"), where).toBe(version);
          expect(cf.getAttribute("data-version"), where).toBe(version);
          expect(cf.getAttribute("data-full-art"), where).toBeNull();
          expect(cf.getAttribute("data-art-motion"), where).toBeNull();
          expect(pictureOf(cf), where).not.toContain("art-m-");
          expect(cf.querySelector(".cf-signature"), where).toBeNull();
          cleanup();
        }
      }
      // Alt full art with Animated is Alt art's animated level, and with Signature too its signed one.
      storeOlderPick(id, VERSION_ALT_FULL_ART_ID, [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID]);
      const signed = faceIn(id, false);
      expect(signed.getAttribute("data-level"), id).toBe(ladderOf("alt-art").at(-1));
      expect(signed.getAttribute("data-art-motion"), id).toBe("animated");
      expect(signed.querySelector(".cf-signature"), id).not.toBeNull();
      cleanup();
    }
  });

  it("R2940002 a worn border keeps its edge and bezel on a Sketch, and a Radiant face its gold frame", () => {
    // The Sketch's frame, its edge and bezel in cards.css and its window's ring in cosmetics.css: the edge and bezel
    // set only on a base face with no border.
    const here = dirname(fileURLToPath(import.meta.url));
    const css = ["cards.css", "cosmetics.css"].map((name) => readFileSync(join(here, name), "utf8")).join("\n");
    const frames = [...css.matchAll(/([^{}]*\[data-version="version-sketch"\][^{}]*)\{([^}]*)\}/g)].map((match) => ({
      selector: match[1]?.trim() ?? "",
      body: match[2] ?? "",
    }));
    const setting = frames.filter((rule) => /--cf-(bezel|edge)\s*:/.test(rule.body));
    expect(setting.length).toBeGreaterThan(0);
    for (const rule of setting) {
      expect(rule.selector, rule.selector).toContain(':not([data-frame])');
      expect(rule.selector, rule.selector).toContain(':not([data-radiant-face="true"])');
    }
    for (const rule of frames) expect(rule.selector, rule.selector).toContain(':not([data-radiant-face="true"])');
  });
});

describe("R2940001 Radiant under the new versions", () => {
  /** Two Units and a Spell, each offered every level of Alt art and the Sketch. */
  const CARDS = ["classicplus-012", "core-083", "meditative-012"] as const;
  /** R3015000: every level of Alt art and the Sketch. */
  const LEVELS: readonly string[] = [...ladderOf("alt-art"), ...ladderOf("sketch")];

  it('R2940001 R3015002 a Radiant face keeps data-radiant-face, data-art-variant="radiant", R277 marks and gold stats at every new level; a base face has none', () => {
    for (const id of CARDS) {
      const def = CATALOG[id];
      if (def === undefined) throw new Error(`no ${id}`);
      const marked = legacyText(faceModel({ defId: id, def, radiant: true }).text).marks.length > 0;
      for (const level of LEVELS) {
        writeCardLevel(id, level);
        for (const radiant of [false, true]) {
          const cf = faceIn(id, radiant);
          const where = `${id} ${radiant ? "Radiant" : "base"} ${level}`;
          // The gold-foil frame, and the art's Radiant paint (washed back over a Sketch at every level, art/pictures.test.ts).
          expect(cf.getAttribute("data-radiant-face"), where).toBe(radiant ? "true" : null);
          expect(cf.querySelector(".cf-art")?.getAttribute("data-art-variant"), where).toBe(radiant ? "radiant" : "base");
          expect(pictureOf(cf).includes('result="sk-wash"'), where).toBe(radiant && levelOf(level)?.variant === "sketch");
          // R277's marks wherever the words show: never on a base face or full bleed, on a Radiant face that differs.
          const marks = cf.querySelectorAll('.cf-mark[data-mark="radiant"]').length;
          if (!radiant || isFullBleed(level)) expect(marks, where).toBe(0);
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
