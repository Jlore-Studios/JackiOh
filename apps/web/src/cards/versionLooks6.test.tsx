// R3017002 (#3017): the faces of part 6's variants, Night, Neon sign, Watercolor, Stained-glass window, Woodcut and Tarot
// card: every card each is offered draws it, base and Radiant, apart from its normal face and every other version, in
// the normal frame, every part of the normal face kept. R3017000: each is a ladder of two levels, its art then
// animated, never a full art nor signed: both draw its one picture, the animated one moving it; a foil, a finish and a
// border layer on each; R3015005: an older pick of one with Animated wears its animated level. R2725004 holds under
// each: a Radiant face keeps every Radiant mark, a base face shows none. Each pull request draws a sample of the cards,
// two of each set for each variant, a Legacy card and the Starship; the daily run draws every other offered card. The
// pictures themselves are art/pictures.test.ts's; the pixels are e2e's cypress/component/card-versions.cy.tsx.

import { cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { COSMETIC_OFFERS } from "../wire/generated/cosmetics.ts";
import { CardFace } from "./CardFace.tsx";
import { CARD_PICKS_SETTINGS_KEY, resetCosmeticsForTests, writeCardLevel, writeCardPick } from "./cosmetics.ts";
import { faceModel } from "./model.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "./settings.ts";
import { __resetSettingsForTests } from "../settings/store.ts";
import { daily } from "../test/daily.ts";
import { legacyText } from "../test/faceData.ts";
import {
  AUGMENT_ANIMATED_ID,
  AUGMENT_SIGNATURE_ID,
  VERSION_ALT_ART_ID,
  VERSION_ALT_FULL_ART_ID,
  VERSION_COMIC_ID,
  VERSION_EXTENDED_ART_ID,
  VERSION_FULL_ART_ID,
  VERSION_IMMERSIVE_ID,
  VERSION_NEON_ID,
  VERSION_NIGHT_ID,
  VERSION_OUT_OF_FRAME_ID,
  VERSION_PIXEL_ID,
  VERSION_PRISMATIC_ID,
  VERSION_SET_SHOWCASE_ID,
  VERSION_SKETCH_ID,
  VERSION_STAINED_GLASS_ID,
  VERSION_TAROT_ID,
  VERSION_WATERCOLOR_ID,
  VERSION_WOODCUT_ID,
  artPictureOf,
  isFullBleed,
  offeredOn,
} from "./versions.ts";

/** R3015005: `id`'s pick as a client stored it before #3015, a version and the augments layered on it, read afresh. */
function storeOlderPick(id: string, fullArt: string, augments: readonly string[]): void {
  localStorage.setItem(CARD_PICKS_SETTINGS_KEY, JSON.stringify({ [id]: { fullArt, augments } }));
  resetCosmeticsForTests();
}

/** R3017000: #3017's six variants' first levels, in the registry's order. */
const PART_SIX = [VERSION_NIGHT_ID, VERSION_NEON_ID, VERSION_WATERCOLOR_ID, VERSION_STAINED_GLASS_ID, VERSION_WOODCUT_ID, VERSION_TAROT_ID] as const;
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
  VERSION_IMMERSIVE_ID,
  VERSION_PRISMATIC_ID,
  ...PART_SIX,
] as const;
/** R3017002: each part 6 variant's filter and what it names a Radiant face's wash (art/pictures.ts). */
const FILTERS: Readonly<Record<(typeof PART_SIX)[number], { filter: string; wash: string }>> = {
  [VERSION_NIGHT_ID]: { filter: "nt-night", wash: "nt-wash" },
  [VERSION_NEON_ID]: { filter: "ne-neon", wash: "ne-wash" },
  [VERSION_WATERCOLOR_ID]: { filter: "wc-paint", wash: "wc-wash" },
  [VERSION_STAINED_GLASS_ID]: { filter: "sg-glass", wash: "sg-wash" },
  [VERSION_WOODCUT_ID]: { filter: "wd-print", wash: "wd-wash" },
  [VERSION_TAROT_ID]: { filter: "tr-tarot", wash: "tr-wash" },
};
const isPartSix = (version: string): version is (typeof PART_SIX)[number] => (PART_SIX as readonly string[]).includes(version);

/** Legacy: Duplicating Felinors, whose painter draws its Core card's one picture (R2724000), and the Dormant Starship. */
const LEGACY = "traininggrounds-044";
const STARSHIP = "stinkyscape-t-dormant-starship";
/** A procedural Common Unit (Speciesist) offered Neon sign, Watercolor and Woodcut, and a procedural Mythic Spell (Scratch Pad) the other three. */
const UNIT = "core-002";
const SPELL = "meditativeplus-026";

function faceIn(id: string, radiant: boolean): HTMLElement {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`no ${id}`);
  const { container } = render(<CardFace face={faceModel({ defId: id, def, radiant })} layout="full" worn />);
  const cf = container.querySelector<HTMLElement>(".cf");
  if (cf === null) throw new Error(`no face for ${id}`);
  return cf;
}

/** The art window: the art frame's own picture. */
function windowOf(cf: HTMLElement): HTMLElement | null {
  return cf.querySelector<HTMLElement>(".cf-art-frame > .cf-art:not(.cf-art--cutout)");
}

/** The art window's picture: what its CSS background draws, decoded. */
function pictureOf(cf: HTMLElement): string {
  return decodeURIComponent(windowOf(cf)?.style.backgroundImage ?? "");
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

/** R3017002: each of `ids` draws its normal face and each version it is offered, every face apart. */
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
        const art = windowOf(cf);
        expect(cf.getAttribute("data-version"), at).toBe(version);
        expect(cf.getAttribute("data-full-art"), at).toBe(isFullBleed(version) ? "true" : null);
        expect(art?.getAttribute("data-art-variant"), at).toBe(radiant ? "radiant" : "base");
        expect(cf.querySelector(".cf-signature"), at).toBeNull();
        const drawn = pictureOf(cf);
        expect(drawn, at).not.toBe("");
        for (const banned of ["NaN", "undefined", "<text", "<title", "href"]) expect(drawn, `${at} ${banned}`).not.toContain(banned);
        if (isPartSix(version)) {
          // In the normal frame, every part of the normal face kept: the cost gem, the name, the type line and the rules box.
          for (const part of [".cost-gem", ".card-name", ".card-type", ".card-text"]) expect(cf.querySelector(part), `${at} ${part}`).not.toBeNull();
          expect(art?.getAttribute("data-art-picture"), at).toBe(artPictureOf(version));
          expect(art?.getAttribute("data-art-part"), at).toBeNull();
          const { filter, wash } = FILTERS[version];
          expect(drawn, at).toContain(`filter="url(#${filter})"`);
          expect(drawn.includes(`result="${wash}"`), at).toBe(radiant);
        }
        faces.set(version, { markup: cf.outerHTML, picture: drawn });
        cleanup();
      }
      // Every face apart from every other, and each part 6 variant's picture its own.
      expect(new Set([...faces.values()].map((face) => face.markup)).size, where).toBe(faces.size);
      for (const version of PART_SIX) {
        const own = faces.get(version)?.picture;
        if (own === undefined) continue;
        for (const [other, face] of faces) if (other !== version) expect(own, `${where} ${version} beside ${other}`).not.toBe(face.picture);
      }
    }
  }
}

describe("R3017002 every card offered a part 6 variant draws it", () => {
  /** The cards offered any of #3017's variants, by set. */
  const BY_SET = new Map<string, string[]>();
  for (const id of new Set(PART_SIX.flatMap((version) => COSMETIC_OFFERS[version] ?? []))) {
    const set = CATALOG[id]?.set ?? "";
    BY_SET.set(set, [...(BY_SET.get(set) ?? []), id]);
  }
  /** Each pull request's sample: the first two cards of each set offered each variant, a Legacy card and the Starship. */
  const SAMPLE = [
    ...new Set([
      ...PART_SIX.flatMap((version) => [...BY_SET.values()].flatMap((ids) => ids.filter((id) => offeredOn(version, id)).slice(0, 2))),
      LEGACY,
      STARSHIP,
    ]),
  ];
  const REST = [...BY_SET.values()].flat().filter((id) => !SAMPLE.includes(id));
  /** How many offered cards one test draws (up to 34 faces each), so that no test nears the timeout on a loaded runner. */
  const OFFERED_CARDS_PER_TEST = 8;

  it("R3017002 the sample holds every variant in every set, a Legacy card and the Starship, and the rest is every other offered card", () => {
    expect(BY_SET.size).toBeGreaterThan(5);
    for (const version of PART_SIX) for (const ids of BY_SET.values()) expect(ids.filter((id) => offeredOn(version, id) && SAMPLE.includes(id)).length, version).toBeGreaterThan(0);
    for (const id of [LEGACY, STARSHIP]) expect(PART_SIX.some((version) => offeredOn(version, id)), id).toBe(true);
    expect(new Set([...SAMPLE, ...REST]).size).toBe([...BY_SET.values()].flat().length);
  });

  for (let from = 0; from < SAMPLE.length; from += OFFERED_CARDS_PER_TEST) {
    const some = SAMPLE.slice(from, from + OFFERED_CARDS_PER_TEST);
    it(`R3017002 sampled cards ${some[0] ?? ""} to ${some.at(-1) ?? ""} draw each version they are offered, base and Radiant, apart from their normal face and every other version`, () => {
      drawsEachVersion(some);
    });
  }
  for (let from = 0; from < REST.length; from += OFFERED_CARDS_PER_TEST) {
    const some = REST.slice(from, from + OFFERED_CARDS_PER_TEST);
    daily(`R3017002 cards ${some[0] ?? ""} to ${some.at(-1) ?? ""} draw each version they are offered, base and Radiant, apart from their normal face and every other version`, () => {
      drawsEachVersion(some);
    });
  }
});

describe("R3017000 R3017002 the levels of part 6's variants", () => {
  /** A Unit and a Spell between them offered all six, and the Legacy card, whose ladders stop before animated. */
  const CARDS = [UNIT, SPELL] as const;

  it("R3017000 R3017002 a foil, a finish and a border layer at both levels of each part 6 variant, the animated one moving its one picture, and none signs", () => {
    expect([...new Set(CARDS.flatMap((id) => PART_SIX.filter((version) => offeredOn(version, id))))].sort()).toEqual([...PART_SIX].sort());
    for (const id of CARDS) {
      for (const version of PART_SIX.filter((each) => offeredOn(each, id))) {
        let still = "";
        for (const level of [version, `${version}-animated`]) {
          expect(offeredOn(level, id), `${level} on ${id}`).toBe(true);
          writeCardLevel(id, level);
          writeCardPick(id, "border", "border-runes");
          writeCardPick(id, "finish", "finish-stained-glass");
          writeCardPick(id, "foil", "foil-galaxy");
          const cf = faceIn(id, false);
          const where = `${id} ${level}`;
          const animated = level !== version;
          expect(cf.getAttribute("data-version"), where).toBe(version);
          expect(cf.getAttribute("data-level"), where).toBe(level);
          expect(cf.getAttribute("data-full-art"), where).toBeNull();
          expect(cf.querySelector(".card-text"), where).not.toBeNull();
          expect(cf.querySelector(".card-name"), where).not.toBeNull();
          // The animated level moves the variant's own picture, under the same filter, taking a slot.
          expect(cf.getAttribute("data-art-motion"), where).toBe(animated ? "animated" : null);
          const drawn = pictureOf(cf);
          expect(drawn, where).toContain(`filter="url(#${FILTERS[version].filter})"`);
          if (animated) {
            expect(drawn, where).toContain("art-m-");
            expect(drawn, where).not.toBe(still);
          } else {
            expect(drawn, where).not.toContain("art-m-");
            still = drawn;
          }
          expect(cf.classList.contains("cf-full-art--animated"), where).toBe(false);
          expect(cf.querySelector(".cf-signature"), where).toBeNull();
          expect(cf.classList.contains("cf-full-art--signed"), where).toBe(false);
          expect(cf.getAttribute("data-frame"), where).toBe("border-runes");
          expect(cf.getAttribute("data-finish"), where).toBe("finish-stained-glass");
          expect(cf.getAttribute("data-cosmetic-foil"), where).toBe("foil-galaxy");
          cleanup();
        }
        // R3015005: an older pick of the version with Animated, and Signature too, wears its animated level, unsigned.
        storeOlderPick(id, version, [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID]);
        const older = faceIn(id, false);
        expect(older.getAttribute("data-level"), `${id} ${version}`).toBe(`${version}-animated`);
        expect(older.getAttribute("data-art-motion"), `${id} ${version}`).toBe("animated");
        expect(older.querySelector(".cf-signature"), `${id} ${version}`).toBeNull();
        cleanup();
      }
    }
    // A Legacy card wears its part 6 art levels still, and is offered none of their animated ones (R2724000).
    const legacy = PART_SIX.filter((version) => offeredOn(version, LEGACY));
    expect(legacy).toHaveLength(3);
    for (const version of legacy) {
      expect(offeredOn(`${version}-animated`, LEGACY), version).toBe(false);
      writeCardLevel(LEGACY, version);
      const cf = faceIn(LEGACY, false);
      expect(cf.getAttribute("data-version"), version).toBe(version);
      expect(cf.getAttribute("data-art-motion"), version).toBeNull();
      expect(pictureOf(cf), version).toContain(`filter="url(#${FILTERS[version].filter})"`);
      cleanup();
    }
  });
});

describe("R3017002 Radiant under part 6's looks", () => {
  /** A Unit and a Spell between them offered both levels of all six part 6 variants. */
  const CARDS = [UNIT, SPELL] as const;

  it('R3017002 a Radiant face keeps data-radiant-face, data-art-variant="radiant", R277 marks and gold stats at every part 6 level; a base face has none', () => {
    for (const id of CARDS) {
      const def = CATALOG[id];
      if (def === undefined) throw new Error(`no ${id}`);
      const marked = legacyText(faceModel({ defId: id, def, radiant: true }).text).marks.length > 0;
      for (const version of PART_SIX.filter((each) => offeredOn(each, id))) {
        for (const level of [version, `${version}-animated`]) {
          writeCardLevel(id, level);
          for (const radiant of [false, true]) {
            const cf = faceIn(id, radiant);
            const where = `${id} ${radiant ? "Radiant" : "base"} ${level}`;
            // The gold-foil frame, and the art's Radiant paint washed back over the treatment.
            expect(cf.getAttribute("data-radiant-face"), where).toBe(radiant ? "true" : null);
            const arts = cf.querySelectorAll(".cf-art");
            expect(arts.length, where).toBe(1);
            for (const art of arts) expect(art.getAttribute("data-art-variant"), where).toBe(radiant ? "radiant" : "base");
            expect(pictureOf(cf).includes(`result="${FILTERS[version].wash}"`), where).toBe(radiant);
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
