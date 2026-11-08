// R2655000-R2655003: the full-art tiers (an Epic, an animated Legendary and a signed Mythic on every
// full-art card), the registry as the list of full-art cards, the tokens that wear their parent's full
// art, and the motion's gate and the signature's words. R2715000: the Animated art, the normal face
// with its art moving; R2715001: whose art moves its own layers, as the Legendary's and the Mythic's
// do. The pixels are e2e's cypress/component/full-art-tiers.cy.tsx and animated-art.cy.tsx; this
// file holds the DOM. R3015005 (#3015): each tier's pick wears the level it maps to, owned through the retired full art
// that maps to it (R3015006), else the highest owned level below it; R3015002: a signed level is Full art's own picture,
// moving, with the signature. R3343001 (#3343): a card's Default levels draw its default look, a Mythic's Full art and
// a Legendary's Extended art, so their normal face is that look (`defaultTierOf`).

import { cleanup, render } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { SHIPPED_SETS } from "@jackioh/shared";

import { COSMETICS, COSMETIC_OFFERS } from "../wire/generated/cosmetics.ts";
import { CardFace } from "./CardFace.tsx";
import { COSMETIC_FOIL_ANIMATED_MAX } from "./constants.ts";
import {
  DEFAULT_CARD_COSMETICS,
  readCardPicks,
  resetCosmeticsForTests,
  writeCardFullArt,
  writeCardLevel,
  writeCardPick,
  type CardPicks,
  type OwnedByCard,
} from "./cosmetics.ts";
import {
  FULL_ART_CARDS,
  FULL_ART_TIERS,
  FULL_ART_TOKEN_PARENT,
  FULL_ARTS_OF_CARD,
  FullArtAllowed,
  FullArtOwned,
  FullArtShown,
  NO_FULL_ART,
  OwnCopy,
  fullArtIdOf,
  fullArtPickOf,
  fullArtSource,
  isFullArtCard,
  type FullArtOwnership,
  type FullArtTier,
} from "./fullArt.ts";
import fullArtTokens from "./fullArtTokens.json";
import { TheirCopy, parseCardLoadouts, type SeatLoadout } from "./loadouts.ts";
import { faceModel } from "./model.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "./settings.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { setReducedMotion } from "../test/setup.ts";
import {
  AUGMENT_ANIMATED_ID,
  AUGMENT_SIGNATURE_ID,
  VERSION_ANIMATED_ID,
  VERSION_FULL_ART_ANIMATED_ID,
  VERSION_FULL_ART_ID,
  VERSION_FULL_ART_SIGNED_ID,
  defaultLookOf,
  impliedBy,
  isFullBleed,
  lookOfLevel,
} from "./versions.ts";
import { facePicture } from "../test/artPicture.ts";
import { legacyText } from "../test/faceData.ts";

/** R2655001: the fifteen cards #2655 adds to the six hero portraits' cards. */
const NEW_CARDS = [
  "classicplus-019",
  "core-018",
  "classicplus-012",
  "core-086",
  "meditative-021",
  "classicplus-046",
  "stinkyscape-154",
  "classicplus-062",
  "classic-090",
  "classicplus-048",
  "meditative-087",
  "classicplus-069",
  "traininggrounds-040",
  "classicplus-026",
  "classicplus-075",
] as const;

/** Mr. Vanilla, one of the six, whose three tiers the tests below pick among. */
const VANILLA = "core-008";

const TOKENS = fullArtTokens as Readonly<Record<string, readonly string[]>>;

function idOf(card: string, tier: FullArtTier): string {
  const id = FULL_ARTS_OF_CARD.get(card)?.get(tier);
  if (id === undefined) throw new Error(`no ${tier} full art for ${card}`);
  return id;
}

/**
 * The ownership of `tiers` of each card in `cards`, as `GET /api/cosmetics` reads once mapped (`ownedCardCosmetics`):
 * R3015006, each retired full art with the levels it maps to.
 */
function owning(cards: readonly string[], tiers: readonly FullArtTier[] = FULL_ART_TIERS): OwnedByCard {
  return new Map(cards.map((card) => [card, new Set(tiers.flatMap((tier) => [idOf(card, tier), ...impliedBy(idOf(card, tier))]))]));
}

function faceOf(id: string, chinese = false) {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`no ${id}`);
  return faceModel({ defId: id, def, radiant: false, ...(chinese ? { chinese: true } : {}) });
}

/** The full face of `id` that `wrap` mounts, worn as the viewer's own unless `worn` is false. */
function drawn(id: string, { worn = true, owned = "every", wrap = (face) => face, chinese = false }: {
  worn?: boolean;
  owned?: FullArtOwnership;
  wrap?: (face: ReactElement) => ReactElement;
  chinese?: boolean;
} = {}): HTMLElement {
  const { container } = render(
    <FullArtOwned owned={owned}>{wrap(<CardFace face={faceOf(id, chinese)} layout="full" worn={worn} />)}</FullArtOwned>,
  );
  const cf = container.querySelector<HTMLElement>(".cf");
  if (cf === null) throw new Error(`no face for ${id}`);
  return cf;
}

/**
 * The tier the face wears, or null for its normal face; a full face that prints full art has no words,
 * and R2715000's Animated art keeps them.
 */
function tierOf(cf: HTMLElement): string | null {
  const full = cf.getAttribute("data-full-art") === "true";
  expect(cf.querySelector(".card-type") === null).toBe(full);
  return cf.getAttribute("data-full-art-tier");
}

/**
 * R3343001: the tier `id`'s Default level prints, still or `moving`: a Mythic's, whose Default draws Full art, Full art's
 * (the Epic's, the Legendary's moving); any other card's the normal face's (none, the Animated art moving).
 */
function defaultTierOf(id: string, moving = false): string | null {
  if (isFullBleed(defaultLookOf(fullArtSource(id)))) return moving ? "Legendary" : "Epic";
  return moving ? "Animated" : null;
}

/** The art window's picture, decoded: what its CSS background draws. */
function pictureOf(cf: HTMLElement): string {
  return facePicture(cf);
}

/** A seat's loadout whose cards' own picks are `cards`, every slot its default. */
function loadout(cards: Record<string, CardPicks>): SeatLoadout {
  return { seat: { ...DEFAULT_CARD_COSMETICS }, cards };
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
  setReducedMotion(false);
});

describe("R2655001 the registry is the list of full-art cards", () => {
  it("R2655001 each of the fifteen new cards prints full art when owned and picked, and its normal face otherwise", () => {
    for (const id of NEW_CARDS) {
      expect(FULL_ART_CARDS.has(id), id).toBe(true);
      expect(isFullArtCard(id), id).toBe(true);
      // Owned and picked: its Epic, then its Mythic.
      writeCardFullArt(id, true);
      expect(tierOf(drawn(id, { owned: owning([id], ["Epic"]) })), id).toBe("Epic");
      cleanup();
      writeCardFullArt(id, idOf(id, "Mythic"));
      expect(tierOf(drawn(id, { owned: owning([id], ["Mythic"]) })), id).toBe("Mythic");
      cleanup();
      // Not owned, or picked off: its normal face, its words back (R3343001: a Mythic's Default, its Full art).
      expect(tierOf(drawn(id, { owned: NO_FULL_ART })), id).toBe(defaultTierOf(id));
      cleanup();
      writeCardFullArt(id, false);
      expect(tierOf(drawn(id, { owned: owning([id]) })), id).toBe(defaultTierOf(id));
      cleanup();
    }
  });

  it("R2655001 a card with no registry entry never prints full art", () => {
    const others = Object.values(CATALOG).filter((def) => !FULL_ART_CARDS.has(def.id) && !FULL_ART_TOKEN_PARENT.has(def.id));
    expect(others.length).toBeGreaterThan(0);
    for (const def of others) expect(isFullArtCard(def.id), def.id).toBe(false);
    // A pick of `true`, or of another card's full art, written for one changes nothing, every cosmetic owned.
    const samples = (["Unit", "Spell", "Field Spell", "Trap", "Field Trap"] as const).flatMap((type) =>
      others.filter((def) => def.type === type && !def.token).slice(0, 2),
    );
    expect(samples.length).toBeGreaterThanOrEqual(8);
    for (const def of samples) {
      writeCardFullArt(def.id, true);
      expect(tierOf(drawn(def.id)), def.id).toBeNull();
      cleanup();
      writeCardFullArt(def.id, idOf(VANILLA, "Mythic"));
      expect(tierOf(drawn(def.id)), def.id).toBeNull();
      cleanup();
    }
    // The registry's full-art cards are its `fullArt` entries' cards, each with one full art of each tier.
    const cards = new Set(COSMETICS.flatMap((cosmetic) => (cosmetic.kind === "fullArt" ? [cosmetic.card] : [])));
    expect(cards).toEqual(FULL_ART_CARDS);
    for (const card of FULL_ART_CARDS) expect([...(FULL_ARTS_OF_CARD.get(card)?.keys() ?? [])], card).toEqual(FULL_ART_TIERS);
  });
});

describe("R2655000 the Legendary and Mythic tiers", () => {
  it("R2655000 the Legendary face carries the animated class and the Mythic the animation and the card's name as its signature", () => {
    const cases: [FullArtTier, boolean, boolean][] = [
      ["Epic", false, false],
      ["Legendary", true, false],
      ["Mythic", true, true],
    ];
    for (const id of [VANILLA, ...NEW_CARDS]) {
      for (const [tier, animated, signed] of cases) {
        writeCardFullArt(id, fullArtPickOf(id, idOf(id, tier)));
        const cf = drawn(id);
        const where = `${id} ${tier}`;
        expect(tierOf(cf), where).toBe(tier);
        expect(cf.classList.contains("cf-full-art--animated"), where).toBe(animated);
        expect(cf.classList.contains("cf-full-art--signed"), where).toBe(signed);
        expect(cf.querySelector(".cf-art-frame > .cf-full-art-sweep") !== null, where).toBe(animated);
        expect(cf.getAttribute("data-full-art-motion"), where).toBe(animated ? "animated" : null);
        const signature = cf.querySelector(".cf-signature");
        expect(signature !== null, where).toBe(signed);
        if (signed) {
          expect(signature?.getAttribute("aria-hidden"), where).toBe("true");
          expect(signature?.querySelector("svg text.cf-signature-text")?.textContent, where).toBe(CATALOG[id]?.name);
        }
        cleanup();
      }
    }
    // With Animated foil off a moving tier holds still, its classes and its signature kept.
    writeCardSettings({ animatedFoil: false });
    writeCardFullArt(VANILLA, idOf(VANILLA, "Mythic"));
    const still = drawn(VANILLA);
    expect(still.getAttribute("data-full-art-motion")).toBe("static");
    expect(still.classList.contains("cf-full-art--signed")).toBe(true);
    expect(still.querySelector(".cf-signature")).not.toBeNull();
  });

  it("R2655000 a pick names a tier: true its Epic, an id its own card's tier, and nothing else", () => {
    expect(fullArtIdOf(VANILLA, true)).toBe(idOf(VANILLA, "Epic"));
    expect(fullArtPickOf(VANILLA, idOf(VANILLA, "Epic"))).toBe(true);
    for (const tier of FULL_ART_TIERS) {
      expect(fullArtIdOf(VANILLA, fullArtPickOf(VANILLA, idOf(VANILLA, tier))), tier).toBe(idOf(VANILLA, tier));
    }
    expect(fullArtPickOf(VANILLA, idOf(VANILLA, "Mythic"))).toBe(idOf(VANILLA, "Mythic"));
    for (const pick of [false, undefined, null, "", idOf("core-004", "Mythic"), "full-art-from-a-newer-client", "foil-rainbow"]) {
      expect(fullArtIdOf(VANILLA, pick), String(pick)).toBeNull();
    }
    expect(fullArtIdOf("core-002", true)).toBeNull();
  });

  it("R2655000 R3015005 R3015006 an unowned tier's pick wears the highest owned level below it, and with no pick a card wears its Epic, which any rarer tier owns", () => {
    // R3015005: the Mythic picked, only the Epic owned: Full art's first level, the highest owned below its signed one.
    writeCardFullArt(VANILLA, idOf(VANILLA, "Mythic"));
    expect(tierOf(drawn(VANILLA, { owned: owning([VANILLA], ["Epic"]) }))).toBe("Epic");
    cleanup();
    expect(tierOf(drawn(VANILLA, { owned: owning([VANILLA], ["Epic", "Legendary"]) }))).toBe("Legendary");
    cleanup();
    expect(tierOf(drawn(VANILLA, { owned: owning([VANILLA], ["Mythic"]) }))).toBe("Mythic");
    cleanup();
    // The Animated art owns the Default's animated level alone, no level of Full art: no full art.
    expect(tierOf(drawn(VANILLA, { owned: owning([VANILLA], ["Animated"]) }))).toBeNull();
    cleanup();
    // R3015006: the Epic picked, only the Legendary and the Mythic owned: each owns Full art's first level, the Epic.
    writeCardFullArt(VANILLA, true);
    expect(tierOf(drawn(VANILLA, { owned: owning([VANILLA], ["Legendary", "Mythic"]) }))).toBe("Epic");
    cleanup();
    // No pick: the setting prints the Epic where it, or a rarer tier mapping to it, is owned, and nothing otherwise.
    writeCardFullArt(VANILLA, null);
    expect(tierOf(drawn(VANILLA, { owned: owning([VANILLA]) }))).toBe("Epic");
    cleanup();
    expect(tierOf(drawn(VANILLA, { owned: owning([VANILLA], ["Legendary", "Mythic"]) }))).toBe("Epic");
    cleanup();
    expect(tierOf(drawn(VANILLA, { owned: owning([VANILLA], ["Animated"]) }))).toBeNull();
    cleanup();
    // R3015005: another card's tier, owned on that card, names no level of this one, which wears its normal face as the
    // server freezes it.
    writeCardFullArt(VANILLA, idOf("core-004", "Mythic"));
    expect(tierOf(drawn(VANILLA, { owned: owning([VANILLA, "core-004"]) }))).toBeNull();
  });

  it("R2655000 the opponent's copy shows the tier its owner's loadout names", () => {
    const theirs = (cards: Record<string, CardPicks>) => (face: ReactElement) => <TheirCopy loadout={loadout(cards)}>{face}</TheirCopy>;
    // The viewer owns nothing and picks nothing: what prints is the owner's, as the server froze it.
    for (const tier of FULL_ART_TIERS) {
      const pick = fullArtPickOf(VANILLA, idOf(VANILLA, tier));
      expect(tierOf(drawn(VANILLA, { worn: false, owned: NO_FULL_ART, wrap: theirs({ [VANILLA]: { fullArt: pick } }) })), tier).toBe(tier);
      cleanup();
    }
    const signed = drawn(VANILLA, { worn: false, owned: NO_FULL_ART, wrap: theirs({ [VANILLA]: { fullArt: idOf(VANILLA, "Mythic") } }) });
    expect(signed.querySelector(".cf-signature text")?.textContent).toBe(CATALOG[VANILLA]?.name);
    cleanup();
    // No entry for the card, another card's id or an id this client does not know: its normal face.
    for (const fullArt of [undefined, idOf("core-004", "Mythic"), "full-art-from-a-newer-client"]) {
      const cards: Record<string, CardPicks> = fullArt === undefined ? {} : { [VANILLA]: { fullArt } };
      expect(tierOf(drawn(VANILLA, { worn: false, owned: NO_FULL_ART, wrap: theirs(cards) })), String(fullArt)).toBeNull();
      cleanup();
    }
    // The viewer's own pick of the card never dresses their copy.
    writeCardFullArt(VANILLA, idOf(VANILLA, "Mythic"));
    expect(tierOf(drawn(VANILLA, { worn: false, owned: owning([VANILLA]), wrap: theirs({ [VANILLA]: { fullArt: true } }) }))).toBe("Epic");
  });

  it("R2655000 the Full-art cards setting turns every tier off on a card with no pick and on another's copy, and a lesson's board on every copy", () => {
    writeCardSettings({ fullArt: false });
    expect(tierOf(drawn(VANILLA))).toBeNull();
    cleanup();
    const theirs = (face: ReactElement) => <TheirCopy loadout={loadout({ [VANILLA]: { fullArt: idOf(VANILLA, "Mythic") } })}>{face}</TheirCopy>;
    expect(tierOf(drawn(VANILLA, { worn: false, wrap: theirs }))).toBeNull();
    cleanup();
    // R1655000: the card's own pick still prints on its own copy with the setting off.
    writeCardFullArt(VANILLA, idOf(VANILLA, "Legendary"));
    expect(tierOf(drawn(VANILLA))).toBe("Legendary");
    cleanup();
    // R1108000: a lesson's board prints no tier, the setting on or off, a pick or the owner's loadout.
    const lesson = (face: ReactElement) => <FullArtAllowed allowed={false}>{face}</FullArtAllowed>;
    for (const fullArt of [false, true]) {
      writeCardSettings({ fullArt });
      for (const tier of FULL_ART_TIERS) {
        writeCardFullArt(VANILLA, fullArtPickOf(VANILLA, idOf(VANILLA, tier)));
        expect(tierOf(drawn(VANILLA, { wrap: lesson })), `${tier}, setting ${String(fullArt)}`).toBeNull();
        cleanup();
      }
      expect(tierOf(drawn(VANILLA, { worn: false, wrap: (face) => lesson(theirs(face)) }))).toBeNull();
      cleanup();
    }
  });
});

describe("R2655002 a listed token wears its parent's full art", () => {
  const LISTED = Object.entries(TOKENS).flatMap(([parent, tokens]) => tokens.map((token) => [token, parent] as const));

  it("R2655002 the table names real tokens of full-art cards, by card id", () => {
    expect(LISTED).toHaveLength(16);
    for (const [token, parent] of LISTED) {
      expect(CATALOG[token]?.token, token).toBe(true);
      expect(FULL_ART_CARDS.has(parent), parent).toBe(true);
      expect(FULL_ART_TOKEN_PARENT.get(token), token).toBe(parent);
      expect(fullArtSource(token), token).toBe(parent);
      // A token is never owned and never in a pack, so the registry names none.
      expect(FULL_ART_CARDS.has(token), token).toBe(false);
    }
    expect(fullArtSource(VANILLA)).toBe(VANILLA);
  });

  it("R2655002 each listed token prints its parent's full art at the parent's tier", () => {
    for (const [token, parent] of LISTED) {
      for (const tier of FULL_ART_TIERS) {
        writeCardFullArt(parent, fullArtPickOf(parent, idOf(parent, tier)));
        const cf = drawn(token, { owned: owning([parent], [tier]) });
        const where = `${token} (${parent}) ${tier}`;
        expect(tierOf(cf), where).toBe(tier);
        expect(cf.classList.contains("cf-full-art--signed"), where).toBe(tier === "Mythic");
        // A token signs its own name, as it prints it in its banner.
        if (tier === "Mythic") expect(cf.querySelector(".cf-signature text")?.textContent, where).toBe(CATALOG[token]?.name);
        cleanup();
      }
      // On the opponent's side it follows the parent's loadout entry, which comes once the parent shows.
      const pick = idOf(parent, "Legendary");
      const theirs = (face: ReactElement) => <TheirCopy loadout={loadout({ [parent]: { fullArt: pick } })}>{face}</TheirCopy>;
      expect(tierOf(drawn(token, { worn: false, owned: NO_FULL_ART, wrap: theirs })), token).toBe("Legendary");
      cleanup();
      const unshown = (face: ReactElement) => <TheirCopy loadout={loadout({})}>{face}</TheirCopy>;
      expect(tierOf(drawn(token, { worn: false, owned: NO_FULL_ART, wrap: unshown })), token).toBeNull();
      cleanup();
    }
  });

  it("R2655002 a token not in the table never prints full art", () => {
    const others = Object.values(CATALOG).filter((def) => def.token && !FULL_ART_TOKEN_PARENT.has(def.id));
    expect(others.length).toBeGreaterThan(0);
    for (const def of others) expect(isFullArtCard(def.id), def.id).toBe(false);
    // Every full-art card picked at its Mythic, every cosmetic owned: an unlisted token keeps its face.
    for (const card of FULL_ART_CARDS) writeCardFullArt(card, idOf(card, "Mythic"));
    for (const def of others.slice(0, 12)) {
      expect(tierOf(drawn(def.id)), def.id).toBeNull();
      cleanup();
    }
  });

  it("R2655002 R3015005 a token whose parent's full art is unowned or off prints its normal face", () => {
    for (const [token, parent] of LISTED) {
      // Unowned: the parent's pick names a tier the viewer owns no level of Full art below (its Animated art owns the
      // Default's animated level alone).
      writeCardFullArt(parent, idOf(parent, "Mythic"));
      expect(tierOf(drawn(token, { owned: owning([parent], ["Animated"]) })), `${token} unowned`).toBeNull();
      cleanup();
      // R3015005: with the Epic and the Legendary owned, it wears the Legendary, the highest owned level below.
      expect(tierOf(drawn(token, { owned: owning([parent], ["Epic", "Legendary"]) })), `${token} Legendary owned`).toBe("Legendary");
      cleanup();
      expect(tierOf(drawn(token, { owned: NO_FULL_ART })), `${token} nothing owned`).toBeNull();
      cleanup();
      // Off: the parent's pick is no full art, every cosmetic owned.
      writeCardFullArt(parent, false);
      expect(tierOf(drawn(token)), `${token} off`).toBeNull();
      cleanup();
      // The setting off and no pick: off too; a lesson's board: off.
      writeCardFullArt(parent, null);
      writeCardSettings({ fullArt: false });
      expect(tierOf(drawn(token)), `${token} setting off`).toBeNull();
      cleanup();
      writeCardSettings({ fullArt: true });
      expect(tierOf(drawn(token, { wrap: (face) => <FullArtAllowed allowed={false}>{face}</FullArtAllowed> })), `${token} lesson`).toBeNull();
      cleanup();
    }
  });
});

describe("R2655003 the signature's words", () => {
  it("R2655003 a Mythic face signs its English name, on a Chinese face too", () => {
    for (const id of [VANILLA, "core-004", "classicplus-019"]) {
      writeCardFullArt(id, idOf(id, "Mythic"));
      const english = CATALOG[id]?.name;
      const cf = drawn(id, { chinese: true });
      const printed = cf.querySelector(".card-name")?.textContent;
      // The banner prints the Chinese name, and the signature the English one.
      expect(printed, id).not.toBe(english);
      expect(cf.querySelector(".cf-signature text")?.textContent, id).toBe(english);
      cleanup();
    }
  });
});

describe("R2715000 the Animated art", () => {
  it("R2715000 R3015006 an owned Animated pick prints the normal face with its art moving, an unowned one its still face", () => {
    for (const id of FULL_ART_CARDS) {
      const animated = idOf(id, "Animated");
      expect(animated, id).toBe(`animated-${idOf(id, "Epic").slice("full-art-".length)}`);
      writeCardFullArt(id, fullArtPickOf(id, animated));
      const cf = drawn(id, { owned: owning([id], ["Animated"]) });
      // Its normal face: the frame, its type line and rules box, no full art, nothing signed or swept. R3343001: the
      // Default's animated level draws its default look moving: a Legendary's Extended art, its picture past its window,
      // and a Mythic's Full art, swept as an animated full art is.
      const shown = defaultLookOf(id);
      const full = isFullBleed(shown);
      expect(tierOf(cf), id).toBe(defaultTierOf(id, true));
      expect(cf.getAttribute("data-full-art"), id).toBe(full ? "true" : null);
      expect(cf.querySelector(".card-text") === null, id).toBe(full);
      expect(cf.querySelector(".cf-art.cf-art--full") !== null, id).toBe(shown !== null);
      expect(cf.classList.contains("cf-full-art--animated"), id).toBe(full);
      expect(cf.querySelector(".cf-signature"), id).toBeNull();
      expect(cf.querySelector(".cf-full-art-sweep") !== null, id).toBe(full);
      // Its art moving in its window.
      expect(cf.getAttribute("data-art-motion"), id).toBe("animated");
      expect(cf.querySelector('.cf-art[data-art-moving="true"]'), id).not.toBeNull();
      expect(pictureOf(cf), id).toContain("art-m-particle");
      cleanup();
      // Unowned, only its still Epic owned: the normal face, still.
      const still = drawn(id, { owned: owning([id], ["Epic"]) });
      expect(tierOf(still), id).toBe(defaultTierOf(id));
      expect(still.getAttribute("data-art-motion"), id).toBeNull();
      expect(still.querySelector('.cf-art[data-art-moving="true"]'), id).toBeNull();
      expect(pictureOf(still), id).not.toContain("art-m-");
      cleanup();
      // R3015006: a Legendary or a Mythic owns the Default's animated level, so the pick moves with either.
      for (const tier of ["Legendary", "Mythic"] as const) {
        const moving = drawn(id, { owned: owning([id], [tier]) });
        expect(tierOf(moving), `${id} ${tier}`).toBe(defaultTierOf(id, true));
        expect(moving.getAttribute("data-art-motion"), `${id} ${tier}`).toBe("animated");
        cleanup();
      }
    }
  });

  it("R2715000 the opponent's, a watcher's and a replay's copy wear the Animated tier their owner's loadout names", () => {
    // Every other seat's copy, on a board, a watched game or a replay, reads its owner's frozen loadout.
    const theirs = (cards: Record<string, CardPicks>) => (face: ReactElement) => <TheirCopy loadout={loadout(cards)}>{face}</TheirCopy>;
    for (const id of [VANILLA, "classicplus-046", "stinkyscape-154"]) {
      const cf = drawn(id, { worn: false, owned: NO_FULL_ART, wrap: theirs({ [id]: { fullArt: idOf(id, "Animated") } }) });
      // R3343001: Dice of Doom, a Mythic, moves its Default's Full art.
      expect(tierOf(cf), id).toBe(defaultTierOf(id, true));
      expect(cf.getAttribute("data-art-motion"), id).toBe("animated");
      cleanup();
    }
    // The viewer's Full-art cards setting off turns it off there, as it does every tier of another's copy.
    writeCardSettings({ fullArt: false });
    const off = drawn(VANILLA, { worn: false, owned: NO_FULL_ART, wrap: theirs({ [VANILLA]: { fullArt: idOf(VANILLA, "Animated") } }) });
    expect(tierOf(off)).toBeNull();
    expect(off.getAttribute("data-art-motion")).toBeNull();
  });

  it("R2715000 a stored Animated pick survives the store and true still names the still Epic", () => {
    const animated = idOf(VANILLA, "Animated");
    const epic = idOf(VANILLA, "Epic");
    expect(fullArtPickOf(VANILLA, animated)).toBe(animated);
    expect(fullArtPickOf(VANILLA, epic)).toBe(true);
    expect(fullArtIdOf(VANILLA, true)).toBe(epic);
    writeCardFullArt(VANILLA, animated);
    // Read back after the page reloads, from what the device stored.
    resetCosmeticsForTests();
    expect(readCardPicks(VANILLA).fullArt).toBe(animated);
    expect(tierOf(drawn(VANILLA))).toBe("Animated");
    cleanup();
    // `true` on the account and the device still wears the still Epic, never the Animated art.
    writeCardFullArt(VANILLA, true);
    expect(tierOf(drawn(VANILLA))).toBe("Epic");
  });
});

describe("R2715001 the moving art", () => {
  it("R2715001 the Legendary and Mythic move their art's own layers, the Epic none", () => {
    for (const tier of ["Epic", "Legendary", "Mythic"] as const) {
      writeCardFullArt(VANILLA, fullArtPickOf(VANILLA, idOf(VANILLA, tier)));
      const cf = drawn(VANILLA);
      const moves = tier !== "Epic";
      expect(cf.getAttribute("data-art-motion"), tier).toBe(moves ? "animated" : null);
      expect(cf.querySelector('.cf-art[data-art-moving="true"]') !== null, tier).toBe(moves);
      expect(pictureOf(cf).includes("art-m-subject"), tier).toBe(moves);
      cleanup();
    }
  });

  it("R2715001 Reduce motion, the system's reduced motion and Animated foil off draw every moving tier still", () => {
    const stills: [string, () => void][] = [
      ["the Reduce motion setting", () => writeSettings({ reduceMotion: true })],
      ["the system's reduced motion", () => setReducedMotion(true)],
      ["Animated foil off", () => writeCardSettings({ animatedFoil: false })],
    ];
    for (const [why, still] of stills) {
      still();
      for (const tier of ["Animated", "Legendary", "Mythic"] as const) {
        writeCardFullArt(VANILLA, fullArtPickOf(VANILLA, idOf(VANILLA, tier)));
        const cf = drawn(VANILLA);
        const where = `${tier} under ${why}`;
        expect(tierOf(cf), where).toBe(tier);
        expect(cf.getAttribute("data-art-motion"), where).toBe("static");
        expect(cf.querySelector('.cf-art[data-art-moving="true"]'), where).toBeNull();
        expect(cf.querySelector<HTMLElement>(".cf-art")?.style.backgroundImage, where).not.toContain("%3Cstyle");
        // The Mythic's signature stands whole.
        if (tier === "Mythic") expect(cf.querySelector(".cf-signature text")?.textContent, where).toBe(CATALOG[VANILLA]?.name);
        cleanup();
      }
      __resetSettingsForTests();
      setReducedMotion(false);
      writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
    }
  });

  it("R2715001 past COSMETIC_FOIL_ANIMATED_MAX moving faces the rest hold still", () => {
    writeCardFullArt(VANILLA, idOf(VANILLA, "Animated"));
    const face = faceOf(VANILLA);
    const { container } = render(
      <FullArtOwned owned="every">
        {Array.from({ length: COSMETIC_FOIL_ANIMATED_MAX + 1 }, (_, index) => (
          <CardFace key={index} face={face} layout="full" worn />
        ))}
      </FullArtOwned>,
    );
    const motions = [...container.querySelectorAll<HTMLElement>(".cf")].map((cf) => cf.getAttribute("data-art-motion"));
    expect(motions).toEqual([...Array<string>(COSMETIC_FOIL_ANIMATED_MAX).fill("animated"), "static"]);
    expect(container.querySelectorAll('.cf-art[data-art-moving="true"]')).toHaveLength(COSMETIC_FOIL_ANIMATED_MAX);
  });

  it("R2715001 a lazy face never moves its art", () => {
    for (const tier of ["Animated", "Legendary", "Mythic"] as const) {
      writeCardFullArt(VANILLA, fullArtPickOf(VANILLA, idOf(VANILLA, tier)));
      const { container } = render(<CardFace face={faceOf(VANILLA)} layout="full" worn lazyArt />);
      expect(container.querySelector(".cf")?.getAttribute("data-art-motion"), tier).toBe("static");
      expect(container.querySelector('.cf-art[data-art-moving="true"]'), tier).toBeNull();
      cleanup();
    }
  });
});

describe("R2725000 R3015000 a card's level on its face", () => {
  /**
   * R2725001: three Units and a Spell offered every level of Full art and the Default's animated one: "Miss" Mrow and
   * API Key Fishing, two of the 21, and Carnivorous Cube and Corpse Eater (Epic Units), which have no retired full art.
   * R3343001: each below Legendary, so its Default is the normal face (a Mythic's draws Full art, a Legendary's
   * Extended art; cards/defaultLooks.test.tsx).
   */
  const CARDS = ["core-086", "meditative-021", "core-022", "core-089"] as const;
  /** R3015000: every level of part 1's two ladders: the normal face, the Default's animated level, and Full art's three. */
  const LEVELS: readonly (string | null)[] = [null, VERSION_ANIMATED_ID, VERSION_FULL_ART_ID, VERSION_FULL_ART_ANIMATED_ID, VERSION_FULL_ART_SIGNED_ID];

  function faceIn(id: string, radiant: boolean): HTMLElement {
    const def = CATALOG[id];
    if (def === undefined) throw new Error(`no ${id}`);
    const { container } = render(<CardFace face={faceModel({ defId: id, def, radiant })} layout="full" worn />);
    const cf = container.querySelector<HTMLElement>(".cf");
    if (cf === null) throw new Error(`no face for ${id}`);
    return cf;
  }

  it("R2725003 R3015002 each level of the Default and Full art: animated moves the normal face and Full art, and Full art's signed level signs its own picture", () => {
    for (const id of CARDS) {
      const pictures = new Map<string | null, string>();
      for (const level of LEVELS) {
        writeCardLevel(id, level);
        // A border and a finish wear on every level, and a foil on these cards, none of which wears a rarity foil.
        writeCardPick(id, "border", "border-runes");
        writeCardPick(id, "finish", "finish-stained-glass");
        writeCardPick(id, "foil", "foil-galaxy");
        const cf = faceIn(id, false);
        const where = `${id} ${String(level)}`;
        const full = isFullBleed(level);
        const { animated, signed } = lookOfLevel(level);
        expect(cf.getAttribute("data-full-art"), where).toBe(full ? "true" : null);
        expect(cf.getAttribute("data-version"), where).toBe(full ? VERSION_FULL_ART_ID : null);
        expect(cf.getAttribute("data-level"), where).toBe(level);
        // R3015000: the augments are retired; a face names its one level.
        expect(cf.hasAttribute("data-augments"), where).toBe(false);
        // Full art has no words; the normal face keeps its type line and rules box at both its levels.
        expect(cf.querySelector(".card-text") === null, where).toBe(full);
        expect(cf.querySelector(".card-name"), where).not.toBeNull();
        // An animated level moves the art's own layers, taking a slot; on Full art it also lays the band.
        expect(cf.getAttribute("data-art-motion"), where).toBe(animated ? "animated" : null);
        if (animated) expect(pictureOf(cf), where).toContain("art-m-");
        expect(cf.classList.contains("cf-full-art--animated"), where).toBe(full && animated);
        // R3015002: the signed level signs over Full art's own picture, moving, never in the art window, and draws no
        // picture of its own.
        const signature = cf.querySelector(".cf-signature");
        expect(signature !== null, where).toBe(signed);
        expect(cf.classList.contains("cf-full-art--signed"), where).toBe(signed);
        expect(cf.querySelector(".cf-art-frame > .cf-signature"), where).toBeNull();
        expect(cf.querySelector(".cf-art")?.getAttribute("data-art-picture"), where).toBeNull();
        expect(cf.getAttribute("data-full-art-motion"), where).toBe(full && animated ? "animated" : null);
        expect(cf.getAttribute("data-frame"), where).toBe("border-runes");
        expect(cf.getAttribute("data-finish"), where).toBe("finish-stained-glass");
        expect(cf.getAttribute("data-cosmetic-foil"), where).toBe("foil-galaxy");
        pictures.set(level, pictureOf(cf));
        cleanup();
      }
      // R3015002: one picture at every level: the still ones draw the same, and every moving one the same with its motion.
      expect(pictures.get(VERSION_FULL_ART_ID), id).toBe(pictures.get(null));
      expect(pictures.get(VERSION_FULL_ART_SIGNED_ID), id).toBe(pictures.get(VERSION_FULL_ART_ANIMATED_ID));
      expect(pictures.get(VERSION_FULL_ART_ANIMATED_ID), id).toBe(pictures.get(VERSION_ANIMATED_ID));
      expect(pictures.get(VERSION_FULL_ART_SIGNED_ID), id).not.toBe(pictures.get(null));
    }
    // Still under reduced motion and with Animated foil off: the art holds, its slot given back.
    writeCardLevel("core-022", VERSION_FULL_ART_SIGNED_ID);
    setReducedMotion(true);
    const reduced = faceIn("core-022", false);
    expect(reduced.getAttribute("data-art-motion")).toBe("static");
    expect(pictureOf(reduced)).not.toContain("art-m-");
    cleanup();
    setReducedMotion(false);
    writeCardSettings({ animatedFoil: false });
    const off = faceIn("core-022", false);
    expect(off.getAttribute("data-art-motion")).toBe("static");
    expect(off.getAttribute("data-full-art-motion")).toBe("static");
  });

  it("R2725000 R3015005 a shown pick wears the level it names and none of the store's, an older shape's augments stepping it up", () => {
    // A Collection edit of a retired Mythic stores Full art's signed level; the Stats drill-down's art buttons
    // (R2756000) then show a pick alone, as one's own copy, with every cosmetic counted as owned.
    writeCardLevel(VANILLA, VERSION_FULL_ART_SIGNED_ID);
    expect(readCardPicks(VANILLA).fullArt).toBe(VERSION_FULL_ART_SIGNED_ID);
    expect(readCardPicks(VANILLA).augments).toBeUndefined();
    function shown(pick: boolean | string, augments?: readonly string[]): HTMLElement {
      const def = CATALOG[VANILLA];
      if (def === undefined) throw new Error(`no ${VANILLA}`);
      const { container } = render(
        <OwnCopy own>
          <FullArtShown defId={VANILLA} pick={pick} {...(augments === undefined ? {} : { augments })}>
            <CardFace face={faceModel({ defId: VANILLA, def, radiant: false })} layout="full" />
          </FullArtShown>
        </OwnCopy>,
      );
      const cf = container.querySelector<HTMLElement>(".cf");
      if (cf === null) throw new Error(`no face for ${VANILLA}`);
      return cf;
    }
    const expectations: readonly (readonly [boolean | string, readonly string[] | undefined, string | null, string | null])[] = [
      [false, undefined, null, null],
      [true, undefined, "Epic", VERSION_FULL_ART_ID],
      [idOf(VANILLA, "Legendary"), undefined, "Legendary", VERSION_FULL_ART_ANIMATED_ID],
      [VERSION_ANIMATED_ID, undefined, "Animated", VERSION_ANIMATED_ID],
      [VERSION_FULL_ART_SIGNED_ID, undefined, "Mythic", VERSION_FULL_ART_SIGNED_ID],
      // R2949000: never signed on the normal face.
      [false, [AUGMENT_SIGNATURE_ID], null, null],
      [true, [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID], "Mythic", VERSION_FULL_ART_SIGNED_ID],
    ];
    for (const [pick, augments, tier, level] of expectations) {
      const cf = shown(pick, augments);
      const where = `${String(pick)} ${augments?.join("+") ?? "(none given)"}`;
      expect(cf.getAttribute("data-full-art-tier"), where).toBe(tier);
      expect(cf.getAttribute("data-level"), where).toBe(level);
      expect(cf.querySelector(".cf-signature") !== null, where).toBe(level === VERSION_FULL_ART_SIGNED_ID);
      cleanup();
    }
  });

  it("R2725000 R2725001 R3015005 the other seat's copy wears the level its owner's frozen loadout names, an older frame's shape mapped, and nothing its card is not offered", () => {
    // The cards' entries as the server froze and sent them (a `cardLoadouts` frame, R1834003), read as the board
    // reads them: Transmogulate's as frozen since #3015, one level's id; Carnivorous Cube's as a replay frozen before
    // it, the Full art version and both augments, which name the same level. Speciesist (core-002, a Common) is offered
    // no level of Full art, so a frame naming one there (a newer client's, or a forged one) draws its normal face.
    const OFFERED = "core-022";
    const LEVELLED = "core-083";
    const UNOFFERED = "core-002";
    for (const id of ladderOfFullArt()) {
      expect(COSMETIC_OFFERS[id], id).toContain(OFFERED);
      expect(COSMETIC_OFFERS[id], id).toContain(LEVELLED);
      expect(COSMETIC_OFFERS[id], id).not.toContain(UNOFFERED);
    }
    const frame = parseCardLoadouts({
      seat: "p2",
      cards: [
        { cardId: OFFERED, fullArt: VERSION_FULL_ART_ID, augments: [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID, "augment-from-a-newer-client"] },
        { cardId: LEVELLED, fullArt: VERSION_FULL_ART_SIGNED_ID },
        { cardId: UNOFFERED, fullArt: VERSION_FULL_ART_SIGNED_ID },
      ],
    });
    if (frame === null) throw new Error("the frame did not parse");
    const theirs = (face: ReactElement) => <TheirCopy loadout={loadout(frame.cards)}>{face}</TheirCopy>;
    // The viewer owns nothing and picks nothing of their own: what prints is the owner's.
    for (const id of [OFFERED, LEVELLED]) {
      writeCardLevel(id, null);
      const worn = drawn(id, { worn: false, owned: NO_FULL_ART, wrap: theirs });
      expect(worn.getAttribute("data-full-art"), id).toBe("true");
      expect(worn.getAttribute("data-version"), id).toBe(VERSION_FULL_ART_ID);
      expect(worn.getAttribute("data-level"), id).toBe(VERSION_FULL_ART_SIGNED_ID);
      expect(worn.getAttribute("data-full-art-tier"), id).toBe("Mythic");
      expect(worn.getAttribute("data-art-motion"), id).toBe("animated");
      expect(worn.querySelector(".cf-signature text")?.textContent, id).toBe(CATALOG[id]?.name);
      cleanup();
    }
    const refused = drawn(UNOFFERED, { worn: false, owned: NO_FULL_ART, wrap: theirs });
    expect(refused.getAttribute("data-full-art")).toBeNull();
    expect(refused.getAttribute("data-version")).toBeNull();
    expect(refused.getAttribute("data-level")).toBeNull();
    expect(refused.querySelector(".cf-signature")).toBeNull();
    cleanup();
    // The Full-art cards setting off: the other seat's copy prints its normal face, every level off.
    writeCardSettings({ fullArt: false });
    const off = drawn(OFFERED, { worn: false, owned: NO_FULL_ART, wrap: theirs });
    expect(off.getAttribute("data-full-art")).toBeNull();
    expect(off.getAttribute("data-version")).toBeNull();
    expect(off.getAttribute("data-level")).toBeNull();
    expect(off.querySelector(".cf-signature")).toBeNull();
  });

  /** R3015000: Full art's three levels, lowest first. */
  function ladderOfFullArt(): readonly string[] {
    return [VERSION_FULL_ART_ID, VERSION_FULL_ART_ANIMATED_ID, VERSION_FULL_ART_SIGNED_ID];
  }

  /** R2725001: the cards offered Full art and the Default's animated level (R3015000), and each offered card by its set. */
  const OFFERED_FULL_ART = COSMETIC_OFFERS[VERSION_FULL_ART_ID] ?? [];
  const OFFERED_ANIMATED = new Set(COSMETIC_OFFERS[VERSION_ANIMATED_ID] ?? []);
  const OFFERED_BY_SET = new Map<string, string[]>();
  for (const id of new Set([...OFFERED_FULL_ART, ...OFFERED_ANIMATED])) {
    const set = CATALOG[id]?.set ?? "";
    OFFERED_BY_SET.set(set, [...(OFFERED_BY_SET.get(set) ?? []), id]);
  }

  it("R2725001 R3015000 Full art and the Default's animated level are offered in every set that ships", () => {
    expect(OFFERED_FULL_ART.length).toBeGreaterThan(FULL_ART_CARDS.size);
    expect([...OFFERED_BY_SET.keys()].sort()).toEqual([...SHIPPED_SETS].sort());
  });

  /** How many offered cards one test draws (six faces each), so that no test nears the timeout on a loaded runner. */
  const OFFERED_CARDS_PER_TEST = 12;
  for (const [set, ids] of OFFERED_BY_SET) {
    for (let from = 0; from < ids.length; from += OFFERED_CARDS_PER_TEST) {
      const some = ids.slice(from, from + OFFERED_CARDS_PER_TEST);
      it(`R2725001 R3015000 every ${set} card offered Full art or the Default's animated level, ${some[0] ?? ""} to ${some.at(-1) ?? ""}, draws it, base and Radiant, apart from its normal face`, () => {
        drawsEveryOfferedLook(some);
      });
    }
  }

  /** R2725001: each of `ids` draws its normal face, and Full art's first level and the Default's animated one where offered. */
  function drawsEveryOfferedLook(ids: readonly string[]): void {
    const fullArt = OFFERED_FULL_ART;
    const animated = OFFERED_ANIMATED;
    for (const id of ids) {
      for (const radiant of [false, true]) {
        const where = `${id} ${radiant ? "Radiant" : "base"}`;
        writeCardLevel(id, null);
        const normal = faceIn(id, radiant);
        const normalMarkup = normal.outerHTML;
        const stillPicture = pictureOf(normal);
        // R3343001: but a Mythic's Default draws its Full art.
        expect(normal.getAttribute("data-full-art"), where).toBe(isFullBleed(defaultLookOf(id)) ? "true" : null);
        cleanup();
        if (fullArt.includes(id)) {
          writeCardLevel(id, VERSION_FULL_ART_ID);
          const full = faceIn(id, radiant);
          expect(full.getAttribute("data-full-art"), where).toBe("true");
          expect(full.getAttribute("data-version"), where).toBe(VERSION_FULL_ART_ID);
          expect(full.outerHTML, where).not.toBe(normalMarkup);
          cleanup();
        }
        if (animated.has(id)) {
          writeCardLevel(id, VERSION_ANIMATED_ID);
          const moving = faceIn(id, radiant);
          expect(moving.getAttribute("data-art-motion"), where).toBe("animated");
          expect(pictureOf(moving), where).toContain("art-m-");
          expect(pictureOf(moving), where).not.toBe(stillPicture);
          cleanup();
        }
      }
    }
  }

  /** R3015000: the cards offered Full art's signed level, by set: those offered Full art, but a Legacy card. */
  const OFFERED_SIGNED = COSMETIC_OFFERS[VERSION_FULL_ART_SIGNED_ID] ?? [];
  const SIGNED_BY_SET = new Map<string, string[]>();
  for (const id of OFFERED_SIGNED) {
    const set = CATALOG[id]?.set ?? "";
    SIGNED_BY_SET.set(set, [...(SIGNED_BY_SET.get(set) ?? []), id]);
  }

  it("R2949000 R3015000 Full art's signed level is offered in every set that ships, only on cards offered Full art and its animated level", () => {
    expect([...SIGNED_BY_SET.keys()].sort()).toEqual([...SHIPPED_SETS].sort());
    const fullArtAnimated = new Set(COSMETIC_OFFERS[VERSION_FULL_ART_ANIMATED_ID] ?? []);
    // R3343000: Full art's first level counts where it is the card's default look, a Mythic's, offered nowhere there.
    const fullArtThere = (id: string): boolean => OFFERED_FULL_ART.includes(id) || defaultLookOf(id) === VERSION_FULL_ART_ID;
    for (const id of OFFERED_SIGNED) expect(fullArtThere(id) && fullArtAnimated.has(id) && OFFERED_ANIMATED.has(id), id).toBe(true);
  });

  /** How many signed cards one test draws (ten faces each), so that no test nears the timeout on a loaded runner. */
  const SIGNED_CARDS_PER_TEST = 7;
  for (const [set, ids] of SIGNED_BY_SET) {
    for (let from = 0; from < ids.length; from += SIGNED_CARDS_PER_TEST) {
      const some = ids.slice(from, from + SIGNED_CARDS_PER_TEST);
      it(`R3015002 every ${set} card offered Full art's signed level, ${some[0] ?? ""} to ${some.at(-1) ?? ""}, draws it, base and Radiant, as its animated full art with the signature`, () => {
        drawsTheSignedLevel(some);
      });
    }
  }

  /**
   * R3015002 (replacing R2949000's Signature look): each of `ids` draws Full art's signed level as its animated full art,
   * the same picture moving the same way, with the signature, and no picture of its own.
   */
  function drawsTheSignedLevel(ids: readonly string[]): void {
    const below: readonly (string | null)[] = [null, VERSION_ANIMATED_ID, VERSION_FULL_ART_ID, VERSION_FULL_ART_ANIMATED_ID];
    for (const id of ids) {
      for (const radiant of [false, true]) {
        const where = `${id} ${radiant ? "Radiant" : "base"}`;
        const pictures = below.map((level) => {
          writeCardLevel(id, level);
          const cf = faceIn(id, radiant);
          expect(cf.querySelector(".cf-signature"), `${where} ${String(level)}`).toBeNull();
          expect(cf.querySelector(".cf-art")?.getAttribute("data-art-picture"), `${where} ${String(level)}`).toBeNull();
          const picture = pictureOf(cf);
          cleanup();
          return picture;
        });
        writeCardLevel(id, VERSION_FULL_ART_SIGNED_ID);
        const signed = faceIn(id, radiant);
        expect(signed.getAttribute("data-full-art-tier"), where).toBe("Mythic");
        expect(signed.getAttribute("data-level"), where).toBe(VERSION_FULL_ART_SIGNED_ID);
        expect(signed.querySelector(".cf-signature"), where).not.toBeNull();
        const art = signed.querySelector(".cf-art");
        expect(art?.getAttribute("data-art-picture"), where).toBeNull();
        // R2725004: the Radiant marks under it.
        expect(art?.getAttribute("data-art-variant"), where).toBe(radiant ? "radiant" : "base");
        expect(signed.getAttribute("data-radiant-face"), where).toBe(radiant ? "true" : null);
        const picture = pictureOf(signed);
        expect(picture, where).toContain("art-m-");
        // The animated full art's picture and motion, which the Default's animated level moves the same way.
        expect(picture, where).toBe(pictures[3]);
        expect(picture, where).toBe(pictures[1]);
        expect(picture, where).not.toBe(pictures[0]);
        expect(picture, where).not.toBe(pictures[2]);
        cleanup();
      }
    }
  }

  it("R2949000 R3015005 no face signs below Full art's signed level, the normal face included", () => {
    const unfit: readonly (readonly [false | string, readonly string[]])[] = [
      [false, [AUGMENT_SIGNATURE_ID]],
      [false, [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID]],
      [VERSION_FULL_ART_ID, [AUGMENT_SIGNATURE_ID]],
      [VERSION_ANIMATED_ID, [AUGMENT_SIGNATURE_ID]],
      [VERSION_FULL_ART_ANIMATED_ID, []],
    ];
    const unsigned = (cf: HTMLElement, where: string): void => {
      expect(cf.querySelector(".cf-signature"), where).toBeNull();
      expect(cf.classList.contains("cf-full-art--signed"), where).toBe(false);
      expect(cf.getAttribute("data-level"), where).not.toBe(VERSION_FULL_ART_SIGNED_ID);
      expect(cf.querySelector(".cf-art")?.getAttribute("data-art-picture"), where).toBeNull();
    };
    for (const id of CARDS) {
      const def = CATALOG[id];
      if (def === undefined) throw new Error(`no ${id}`);
      for (const [version, augments] of unfit) {
        const where = `${id} ${String(version)} ${augments.join("+")}`;
        // A shown pick, which the store's writes never touch (the Collection's preview, R2756000's art buttons).
        const { container } = render(
          <OwnCopy own>
            <FullArtShown defId={id} pick={version} augments={augments}>
              <CardFace face={faceModel({ defId: id, def, radiant: false })} layout="full" />
            </FullArtShown>
          </OwnCopy>,
        );
        const shown = container.querySelector<HTMLElement>(".cf");
        if (shown === null) throw new Error(`no face for ${where}`);
        unsigned(shown, `${where} shown`);
        cleanup();
        // The other seat's copy, under a frame naming it (a newer client's, or a forged one).
        const frame = parseCardLoadouts({ seat: "p2", cards: [{ cardId: id, fullArt: version, augments: [...augments] }] });
        if (frame === null) throw new Error("the frame did not parse");
        const theirs = drawn(id, { worn: false, owned: NO_FULL_ART, wrap: (face) => <TheirCopy loadout={loadout(frame.cards)}>{face}</TheirCopy> });
        unsigned(theirs, `${where} theirs`);
        cleanup();
      }
    }
  });

  it("R2949000 R3015002 R3015005 a retired Mythic's pick wears Full art's signed level: the Legendary's own picture, moving, signed", () => {
    writeCardFullArt(VANILLA, idOf(VANILLA, "Mythic"));
    const mythic = drawn(VANILLA);
    expect(mythic.getAttribute("data-full-art-tier")).toBe("Mythic");
    expect(mythic.getAttribute("data-level")).toBe(VERSION_FULL_ART_SIGNED_ID);
    expect(mythic.querySelector(".cf-signature")).not.toBeNull();
    expect(mythic.querySelector(".cf-art")?.getAttribute("data-art-picture")).toBeNull();
    const signed = pictureOf(mythic);
    cleanup();
    // The Legendary, Full art's animated level, draws the same picture moving the same way: nothing is redrawn.
    writeCardFullArt(VANILLA, idOf(VANILLA, "Legendary"));
    const legendary = drawn(VANILLA);
    expect(legendary.getAttribute("data-level")).toBe(VERSION_FULL_ART_ANIMATED_ID);
    expect(legendary.querySelector(".cf-art")?.getAttribute("data-art-picture")).toBeNull();
    expect(pictureOf(legendary)).toBe(signed);
  });

  it('R2725004 a Radiant face keeps data-radiant-face and data-art-variant="radiant" at every level; a base face has neither', () => {
    for (const id of CARDS) {
      const def = CATALOG[id];
      if (def === undefined) throw new Error(`no ${id}`);
      const marked = legacyText(faceModel({ defId: id, def, radiant: true }).text).marks.length > 0;
      for (const level of LEVELS) {
        writeCardLevel(id, level);
        for (const radiant of [false, true]) {
          const cf = faceIn(id, radiant);
          const where = `${id} ${radiant ? "Radiant" : "base"} ${String(level)}`;
          // The gold-foil frame, and the art's Radiant paint.
          expect(cf.getAttribute("data-radiant-face"), where).toBe(radiant ? "true" : null);
          expect(cf.querySelector(".cf-art")?.getAttribute("data-art-variant"), where).toBe(radiant ? "radiant" : "base");
          // R277's marks wherever the words show: never on a base face, on a Radiant normal face that differs, at
          // either of the Default's levels.
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
