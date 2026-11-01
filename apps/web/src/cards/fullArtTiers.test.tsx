// R2655000-R2655003: the full-art tiers (an Epic, an animated Legendary and a signed Mythic on every
// full-art card), the registry as the list of full-art cards, the tokens that wear their parent's full
// art, and the motion's gate and the signature's words. R2715000: the Animated art, the normal face
// with its art moving; R2715001: whose art moves its own layers, as the Legendary's and the Mythic's
// do. The pixels are e2e's cypress/component/full-art-tiers.cy.tsx and animated-art.cy.tsx; this
// file holds the DOM.

import { cleanup, render } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { COSMETICS } from "../wire/generated/cosmetics.ts";
import { CardFace } from "./CardFace.tsx";
import { COSMETIC_FOIL_ANIMATED_MAX } from "./constants.ts";
import {
  DEFAULT_CARD_COSMETICS,
  readCardPicks,
  resetCosmeticsForTests,
  writeCardFullArt,
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
  NO_FULL_ART,
  fullArtIdOf,
  fullArtPickOf,
  fullArtSource,
  fullArtTierOf,
  isFullArtCard,
  type FullArtOwnership,
  type FullArtTier,
} from "./fullArt.ts";
import fullArtTokens from "./fullArtTokens.json";
import { TheirCopy, type SeatLoadout } from "./loadouts.ts";
import { faceModel } from "./model.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "./settings.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { setReducedMotion } from "../test/setup.ts";

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

/** The ownership of `tiers` of each card in `cards`, as `GET /api/cosmetics` reads once mapped. */
function owning(cards: readonly string[], tiers: readonly FullArtTier[] = FULL_ART_TIERS): OwnedByCard {
  return new Map(cards.map((card) => [card, new Set(tiers.map((tier) => idOf(card, tier)))]));
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

/** The art window's picture, decoded: what its CSS background draws. */
function pictureOf(cf: HTMLElement): string {
  return decodeURIComponent(cf.querySelector<HTMLElement>(".cf-art")?.style.backgroundImage ?? "");
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
      // Not owned, or picked off: its normal face, its words back.
      expect(tierOf(drawn(id, { owned: NO_FULL_ART })), id).toBeNull();
      cleanup();
      writeCardFullArt(id, false);
      expect(tierOf(drawn(id, { owned: owning([id]) })), id).toBeNull();
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
      expect(fullArtTierOf(VANILLA, fullArtPickOf(VANILLA, idOf(VANILLA, tier))), tier).toBe(tier);
    }
    expect(fullArtPickOf(VANILLA, idOf(VANILLA, "Mythic"))).toBe(idOf(VANILLA, "Mythic"));
    for (const pick of [false, undefined, null, "", idOf("core-004", "Mythic"), "full-art-from-a-newer-client", "foil-rainbow"]) {
      expect(fullArtIdOf(VANILLA, pick), String(pick)).toBeNull();
      expect(fullArtTierOf(VANILLA, pick), String(pick)).toBeNull();
    }
    expect(fullArtIdOf("core-002", true)).toBeNull();
  });

  it("R2655000 an unowned tier's pick prints no full art, and with no pick a card wears its Epic, never a rarer tier it owns", () => {
    // The Mythic picked, only the Epic owned: no full art, not the Epic in its place.
    writeCardFullArt(VANILLA, idOf(VANILLA, "Mythic"));
    expect(tierOf(drawn(VANILLA, { owned: owning([VANILLA], ["Epic"]) }))).toBeNull();
    cleanup();
    expect(tierOf(drawn(VANILLA, { owned: owning([VANILLA], ["Mythic"]) }))).toBe("Mythic");
    cleanup();
    // The Epic picked, only the Mythic owned: none.
    writeCardFullArt(VANILLA, true);
    expect(tierOf(drawn(VANILLA, { owned: owning([VANILLA], ["Legendary", "Mythic"]) }))).toBeNull();
    cleanup();
    // No pick: the setting prints the Epic where it is owned, and nothing where only rarer tiers are.
    writeCardFullArt(VANILLA, null);
    expect(tierOf(drawn(VANILLA, { owned: owning([VANILLA]) }))).toBe("Epic");
    cleanup();
    expect(tierOf(drawn(VANILLA, { owned: owning([VANILLA], ["Legendary", "Mythic"]) }))).toBeNull();
    cleanup();
    // Another card's tier, owned on that card, is no pick of this one.
    writeCardFullArt(VANILLA, idOf("core-004", "Mythic"));
    expect(tierOf(drawn(VANILLA, { owned: owning([VANILLA, "core-004"]) }))).toBe("Epic");
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

  it("R2655002 a token whose parent's full art is unowned or off prints its normal face", () => {
    for (const [token, parent] of LISTED) {
      // Unowned: the parent's pick names a tier the viewer does not own.
      writeCardFullArt(parent, idOf(parent, "Mythic"));
      expect(tierOf(drawn(token, { owned: owning([parent], ["Epic", "Legendary"]) })), `${token} unowned`).toBeNull();
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
  it("R2715000 an owned Animated pick prints the normal face with its art moving, an unowned one its still face", () => {
    for (const id of FULL_ART_CARDS) {
      const animated = idOf(id, "Animated");
      expect(animated, id).toBe(`animated-${idOf(id, "Epic").slice("full-art-".length)}`);
      writeCardFullArt(id, fullArtPickOf(id, animated));
      const cf = drawn(id, { owned: owning([id], ["Animated"]) });
      // Its normal face: the frame, its type line and rules box, no full art, nothing signed or swept.
      expect(tierOf(cf), id).toBe("Animated");
      expect(cf.getAttribute("data-full-art"), id).toBeNull();
      expect(cf.querySelector(".card-text"), id).not.toBeNull();
      expect(cf.querySelector(".cf-art.cf-art--full"), id).toBeNull();
      expect(cf.classList.contains("cf-full-art--animated"), id).toBe(false);
      expect(cf.querySelector(".cf-signature, .cf-full-art-sweep"), id).toBeNull();
      // Its art moving in its window.
      expect(cf.getAttribute("data-art-motion"), id).toBe("animated");
      expect(cf.querySelector('.cf-art[data-art-moving="true"]'), id).not.toBeNull();
      expect(pictureOf(cf), id).toContain("art-m-particle");
      cleanup();
      // Unowned, only its other tiers owned: the normal face, still.
      const still = drawn(id, { owned: owning([id], ["Epic", "Legendary", "Mythic"]) });
      expect(tierOf(still), id).toBeNull();
      expect(still.getAttribute("data-art-motion"), id).toBeNull();
      expect(still.querySelector('.cf-art[data-art-moving="true"]'), id).toBeNull();
      expect(pictureOf(still), id).not.toContain("art-m-");
      cleanup();
    }
  });

  it("R2715000 the opponent's, a watcher's and a replay's copy wear the Animated tier their owner's loadout names", () => {
    // Every other seat's copy, on a board, a watched game or a replay, reads its owner's frozen loadout.
    const theirs = (cards: Record<string, CardPicks>) => (face: ReactElement) => <TheirCopy loadout={loadout(cards)}>{face}</TheirCopy>;
    for (const id of [VANILLA, "classicplus-046", "stinkyscape-154"]) {
      const cf = drawn(id, { worn: false, owned: NO_FULL_ART, wrap: theirs({ [id]: { fullArt: idOf(id, "Animated") } }) });
      expect(tierOf(cf), id).toBe("Animated");
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
    expect(fullArtTierOf(VANILLA, animated)).toBe("Animated");
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
