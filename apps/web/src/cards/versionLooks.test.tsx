// R2940001 (#2940): the faces of the Alt art, Sketch and Alt full art versions: every card each is offered draws it,
// base and Radiant, apart from its normal face and every other version; Alt full art prints the full-art layout with
// Alt art's picture. R2940002: Animated, a foil, a finish and a border layer on each, and Signature on none. R2725004
// holds under each: a Radiant face keeps every Radiant mark, a base face shows none. The pictures themselves are
// art/pictures.test.ts's; the pixels are e2e's cypress/component/card-versions.cy.tsx.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { SHIPPED_SETS } from "@jackioh/shared";

import { COSMETIC_OFFERS } from "../wire/generated/cosmetics.ts";
import { CardFace } from "./CardFace.tsx";
import { resetCosmeticsForTests, writeCardPick, writeCardVersion } from "./cosmetics.ts";
import { faceModel } from "./model.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "./settings.ts";
import { __resetSettingsForTests } from "../settings/store.ts";
import {
  AUGMENT_ANIMATED_ID,
  AUGMENT_SIGNATURE_ID,
  VERSION_ALT_ART_ID,
  VERSION_ALT_FULL_ART_ID,
  VERSION_FULL_ART_ID,
  VERSION_SKETCH_ID,
  artPictureOf,
  augmentsOf,
  isFullBleed,
  lookOf,
  offeredOn,
} from "./versions.ts";
import { legacyText } from "../test/faceData.ts";

/** R2940000: #2940's three versions, in the registry's order. */
const NEW_VERSIONS = [VERSION_ALT_ART_ID, VERSION_SKETCH_ID, VERSION_ALT_FULL_ART_ID] as const;
/** Every version a card may wear but its normal face. */
const VERSIONS = [VERSION_FULL_ART_ID, ...NEW_VERSIONS] as const;

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
        // Alt full art is Alt art's picture across the whole face.
        if (faces.has(VERSION_ALT_ART_ID) && faces.has(VERSION_ALT_FULL_ART_ID)) expect(picture(VERSION_ALT_FULL_ART_ID), where).toBe(picture(VERSION_ALT_ART_ID));
      }
    }
  }
});

describe("R2940002 the augments on the new versions", () => {
  /**
   * Cards offered all three new versions and Animated: The Mother Pancake (a Legendary Unit), Transmogulate (a
   * Legendary Spell) and Fear Mongerer (an Epic Unit, not offered Full art).
   */
  const CARDS = ["classicplus-012", "core-083", "meditative-012"] as const;
  /** Each new version with no augment, either and both. */
  const LOOKS: readonly (readonly [string, readonly string[]])[] = NEW_VERSIONS.flatMap((version) =>
    [[], [AUGMENT_ANIMATED_ID], [AUGMENT_SIGNATURE_ID], [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID]].map((augments) => [version, augments] as const),
  );

  it("R2940002 Animated, a foil, a finish and a border layer on Alt art, Sketch and Alt full art, and Signature on none", () => {
    for (const id of CARDS) {
      for (const version of NEW_VERSIONS) expect(offeredOn(version, id), `${version} on ${id}`).toBe(true);
      expect(offeredOn(AUGMENT_ANIMATED_ID, id), id).toBe(true);
      for (const [version, augments] of LOOKS) {
        writeCardVersion(id, version, augments);
        writeCardPick(id, "border", "border-runes");
        writeCardPick(id, "finish", "finish-stained-glass");
        writeCardPick(id, "foil", "foil-galaxy");
        const cf = faceIn(id, false);
        const where = `${id} ${version} ${augments.join("+")}`;
        const full = isFullBleed(version);
        const animated = augments.includes(AUGMENT_ANIMATED_ID);
        // R2949000: Signature fits none of them; the store keeps none.
        const worn = augmentsOf(lookOf(version, augments));
        expect(worn.includes(AUGMENT_SIGNATURE_ID), where).toBe(false);
        expect(cf.getAttribute("data-version"), where).toBe(version);
        expect(cf.getAttribute("data-full-art"), where).toBe(full ? "true" : null);
        expect(cf.getAttribute("data-augments"), where).toBe(animated ? AUGMENT_ANIMATED_ID : null);
        expect(cf.querySelector(".card-text") === null, where).toBe(full);
        expect(cf.querySelector(".card-name"), where).not.toBeNull();
        // Animated moves the version's own picture, Alt art's or the Sketch's, taking a slot; on full bleed it lays the band.
        expect(cf.getAttribute("data-art-motion"), where).toBe(animated ? "animated" : null);
        const art = cf.querySelector(".cf-art");
        expect(art?.getAttribute("data-art-picture"), where).toBe(artPictureOf(version));
        if (animated) expect(pictureOf(cf), where).toContain("art-m-");
        if (version === VERSION_SKETCH_ID) expect(pictureOf(cf), where).toContain('filter="url(#sk-pencil)"');
        expect(cf.classList.contains("cf-full-art--animated"), where).toBe(full && animated);
        expect(cf.getAttribute("data-full-art-motion"), where).toBe(full && animated ? "animated" : null);
        expect(cf.querySelector(".cf-signature"), where).toBeNull();
        expect(cf.classList.contains("cf-full-art--signed"), where).toBe(false);
        expect(cf.getAttribute("data-frame"), where).toBe("border-runes");
        expect(cf.getAttribute("data-finish"), where).toBe("finish-stained-glass");
        expect(cf.getAttribute("data-cosmetic-foil"), where).toBe("foil-galaxy");
        cleanup();
      }
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
  /** Two Units and a Spell, each offered all three new versions and Animated. */
  const CARDS = ["classicplus-012", "core-083", "meditative-012"] as const;
  const LOOKS: readonly (readonly [string, readonly string[]])[] = NEW_VERSIONS.flatMap((version) =>
    [[], [AUGMENT_ANIMATED_ID]].map((augments) => [version, augments] as const),
  );

  it('R2940001 a Radiant face keeps data-radiant-face, data-art-variant="radiant", R277 marks and gold stats under every new look; a base face has none', () => {
    for (const id of CARDS) {
      const def = CATALOG[id];
      if (def === undefined) throw new Error(`no ${id}`);
      const marked = legacyText(faceModel({ defId: id, def, radiant: true }).text).marks.length > 0;
      for (const [version, augments] of LOOKS) {
        writeCardVersion(id, version, augments);
        for (const radiant of [false, true]) {
          const cf = faceIn(id, radiant);
          const where = `${id} ${radiant ? "Radiant" : "base"} ${version} ${augments.join("+")}`;
          // The gold-foil frame, and the art's Radiant paint (washed back over a Sketch, art/pictures.test.ts).
          expect(cf.getAttribute("data-radiant-face"), where).toBe(radiant ? "true" : null);
          expect(cf.querySelector(".cf-art")?.getAttribute("data-art-variant"), where).toBe(radiant ? "radiant" : "base");
          expect(pictureOf(cf).includes('result="sk-wash"'), where).toBe(radiant && version === VERSION_SKETCH_ID);
          // R277's marks wherever the words show: never on a base face or full bleed, on a Radiant face that differs.
          const marks = cf.querySelectorAll('.cf-mark[data-mark="radiant"]').length;
          if (!radiant || isFullBleed(version)) expect(marks, where).toBe(0);
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
