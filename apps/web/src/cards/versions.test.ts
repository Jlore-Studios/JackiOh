// R2725000 (#2725): a card's version and the augments layered on it, read from its pick (versions.ts). R2725001: the
// offers are the server's own (`COSMETIC_OFFERS`); R2725002: a retired full art's pick looks as it did and owns what
// it combines. R2949000: Signature signs only beside the Full art version and Animated; R2949001: Full art's offer is a
// varied share of the cards, and Signature's lies inside Full art's and Animated's.

import { describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { COSMETICS, COSMETIC_OFFERS } from "../wire/generated/cosmetics.ts";
import { FULL_ARTS_OF_CARD, FULL_ART_CARDS, tierOfLook } from "./fullArt.ts";
import { ownedCardCosmetics } from "./ownedFullArt.ts";
import {
  AUGMENT_ANIMATED_ID,
  AUGMENT_SIGNATURE_ID,
  NORMAL_LOOK,
  VERSION_FULL_ART_ID,
  augmentPicksOf,
  augmentsOf,
  impliedBy,
  isOfferedId,
  lookOf,
  offeredOn,
  signatureFits,
  versionPickOf,
} from "./versions.ts";

const VANILLA = "core-008";

/** Mr. Vanilla's retired full art of `tier`. */
function retired(tier: "Epic" | "Legendary" | "Mythic" | "Animated"): string {
  const id = FULL_ARTS_OF_CARD.get(VANILLA)?.get(tier);
  if (id === undefined) throw new Error(`Mr. Vanilla has no ${tier} full art`);
  return id;
}

describe("R2725000 a card's version and augments", () => {
  it("R2725002 legacy picks look as before", () => {
    // Each retired full art's pick, and `true`, prints the tier it printed before #2725.
    expect(tierOfLook(lookOf(true))).toBe("Epic");
    for (const tier of ["Epic", "Legendary", "Mythic", "Animated"] as const) {
      expect(tierOfLook(lookOf(retired(tier))), tier).toBe(tier);
    }
    expect(lookOf(retired("Mythic"))).toEqual({ version: VERSION_FULL_ART_ID, animated: true, signed: true });
    expect(lookOf(retired("Animated"))).toEqual({ version: null, animated: true, signed: false });
    // No pick, `false` and an id this client does not know are the normal face.
    for (const pick of [undefined, null, false, "full-art-from-a-newer-client", "version-from-a-newer-client"]) {
      expect(lookOf(pick), String(pick)).toEqual(NORMAL_LOOK);
    }
    // The augments layer on any version, a retired one's too, and an unknown one adds nothing.
    expect(lookOf(VERSION_FULL_ART_ID, [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID])).toEqual({
      version: VERSION_FULL_ART_ID,
      animated: true,
      signed: true,
    });
    expect(tierOfLook(lookOf(VERSION_FULL_ART_ID, [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID]))).toBe("Mythic");
    // R2949000: but Signature only beside the Full art version and Animated.
    expect(lookOf(false, [AUGMENT_SIGNATURE_ID])).toEqual(NORMAL_LOOK);
    expect(tierOfLook(lookOf(false, [AUGMENT_SIGNATURE_ID]))).toBeNull();
    expect(lookOf(retired("Epic"), [AUGMENT_ANIMATED_ID])).toEqual(lookOf(retired("Legendary")));
    expect(lookOf(false, ["augment-from-a-newer-client"])).toBe(NORMAL_LOOK);
    expect(augmentsOf(lookOf(retired("Mythic")))).toEqual([AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID]);

    // Rewriting a retired pick keeps its look: the version it wears and the augments it combines.
    expect(versionPickOf(retired("Mythic"))).toBe(VERSION_FULL_ART_ID);
    expect(augmentPicksOf(retired("Mythic"))).toEqual([AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID]);
    expect(versionPickOf(retired("Animated"))).toBe(false);
    expect(augmentPicksOf(retired("Animated"), ["augment-from-a-newer-client"])).toEqual([AUGMENT_ANIMATED_ID, "augment-from-a-newer-client"]);
    expect(versionPickOf(true)).toBe(true);
    expect(versionPickOf(undefined)).toBeNull();
    expect(versionPickOf("full-art-from-a-newer-client")).toBe("full-art-from-a-newer-client");
  });

  it("R2949000 lookOf signs only beside the Full art version and Animated", () => {
    const SIG = AUGMENT_SIGNATURE_ID;
    const ANIM = AUGMENT_ANIMATED_ID;
    // The version, `true` or a retired full art implying it, with Animated listed or implied.
    for (const [fullArt, augments] of [
      [VERSION_FULL_ART_ID, [ANIM, SIG]],
      [true, [ANIM]],
      [retired("Legendary"), []],
      [retired("Mythic"), []],
      [retired("Epic"), [ANIM]],
    ] as const) {
      expect(signatureFits(fullArt, augments), `${String(fullArt)} ${augments.join(" ")}`).toBe(true);
      expect(lookOf(fullArt, [...augments, SIG]).signed, `${String(fullArt)} ${augments.join(" ")}`).toBe(true);
    }
    // Without Animated, without the version, or a version this client does not know.
    for (const [fullArt, augments] of [
      [VERSION_FULL_ART_ID, [SIG]],
      [retired("Epic"), []],
      [retired("Animated"), [ANIM]],
      [false, [ANIM, SIG]],
      [undefined, [ANIM, SIG]],
      [null, [ANIM, SIG]],
      ["version-from-a-newer-client", [ANIM, SIG]],
    ] as const) {
      expect(signatureFits(fullArt, augments), `${String(fullArt)} ${augments.join(" ")}`).toBe(false);
      expect(lookOf(fullArt, [...augments, SIG]).signed, `${String(fullArt)} ${augments.join(" ")}`).toBe(false);
    }
    // Animated still layers where Signature cannot, and a retired Mythic signs on its own.
    expect(lookOf(false, [ANIM, SIG])).toEqual({ version: null, animated: true, signed: false });
    expect(lookOf(VERSION_FULL_ART_ID, [SIG])).toEqual({ version: VERSION_FULL_ART_ID, animated: false, signed: false });
    expect(lookOf(retired("Mythic"))).toEqual({ version: VERSION_FULL_ART_ID, animated: true, signed: true });
    expect(lookOf(retired("Animated"), [SIG])).toEqual({ version: null, animated: true, signed: false });
    expect(tierOfLook(lookOf(VERSION_FULL_ART_ID, [ANIM, SIG]))).toBe("Mythic");
  });

  it("R2725002 a retired full art owns the version and augments it combines, on its own card alone", () => {
    expect(impliedBy(retired("Epic"))).toEqual([VERSION_FULL_ART_ID]);
    expect(impliedBy(retired("Legendary"))).toEqual([VERSION_FULL_ART_ID, AUGMENT_ANIMATED_ID]);
    expect(impliedBy(retired("Mythic"))).toEqual([VERSION_FULL_ART_ID, AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID]);
    expect(impliedBy(retired("Animated"))).toEqual([AUGMENT_ANIMATED_ID]);
    expect(impliedBy("foil-galaxy")).toEqual([]);
    expect(impliedBy(VERSION_FULL_ART_ID)).toEqual([]);
    const owned = ownedCardCosmetics({
      cards: [
        { cardId: VANILLA, cosmeticId: retired("Legendary") },
        // A version on a card it is not offered, and another card's full art: neither counts.
        { cardId: "core-002", cosmeticId: VERSION_FULL_ART_ID },
        { cardId: "core-002", cosmeticId: retired("Epic") },
      ],
      account: [],
    });
    expect([...(owned.get(VANILLA) ?? [])].sort()).toEqual([AUGMENT_ANIMATED_ID, retired("Legendary"), VERSION_FULL_ART_ID].sort());
    expect(owned.get("core-002")).toBeUndefined();
  });

  it("R2725001 offers match COSMETIC_OFFERS", () => {
    const offered = COSMETICS.filter((cosmetic) => cosmetic.kind === "version" || cosmetic.kind === "augment").map((cosmetic) => cosmetic.id);
    expect(offered).toEqual([VERSION_FULL_ART_ID, AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID]);
    expect(Object.keys(COSMETIC_OFFERS)).toEqual(offered);
    for (const id of offered) {
      const cards = COSMETIC_OFFERS[id] ?? [];
      expect(isOfferedId(id), id).toBe(true);
      // Real cards, never a token, the 21 full-art cards among them, and not every card.
      expect(cards.length, id).toBeGreaterThan(FULL_ART_CARDS.size);
      for (const card of cards) {
        expect(CATALOG[card]?.token, `${id} on ${card}`).toBe(false);
        expect(offeredOn(id, card)).toBe(true);
      }
      for (const card of FULL_ART_CARDS) expect(offeredOn(id, card), `${id} on ${card}`).toBe(true);
      expect(offeredOn(id, "core-002"), `${id} on Speciesist`).toBe(false);
    }
    // No other id has an offer.
    expect(isOfferedId("foil-galaxy")).toBe(false);
    expect(isOfferedId(retired("Epic"))).toBe(false);
    expect(offeredOn("foil-galaxy", VANILLA)).toBe(false);
  });

  it("R2949001 Full art's offer is a varied share holding the 21, and Signature's lies inside Full art's and Animated's", () => {
    const on = (id: string): ReadonlySet<string> => new Set(COSMETIC_OFFERS[id] ?? []);
    const fullArt = on(VERSION_FULL_ART_ID);
    const animated = on(AUGMENT_ANIMATED_ID);
    const signature = on(AUGMENT_SIGNATURE_ID);
    // Full art is offered on fewer cards than Animated, and Animated on cards Full art is not: the looks differ by card.
    expect(fullArt.size).toBeLessThan(animated.size);
    expect([...animated].some((card) => !fullArt.has(card))).toBe(true);
    // The 21 with retired full arts are always among them.
    for (const card of FULL_ART_CARDS) expect(fullArt.has(card), card).toBe(true);
    // Signature exactly where both are offered.
    expect([...signature].sort()).toEqual([...fullArt].filter((card) => animated.has(card)).sort());
  });
});
