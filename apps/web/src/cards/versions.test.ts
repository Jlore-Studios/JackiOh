// R2725000 (#2725): a card's version and the augments layered on it, read from its pick (versions.ts). R2725001: the
// offers are the server's own (`COSMETIC_OFFERS`); R2725002: a retired full art's pick looks as it did and owns what
// it combines. R2949000: Signature signs only beside the Full art version and Animated; R2949001: Full art's offer is a
// varied share of the cards, and Signature's lies inside Full art's and Animated's. R2940000: #2940's Alt art, Sketch
// and Alt full art, each on a varied set of its own; R2940001: the full bleed and the picture each version draws.
// R2942000: #2942's Extended art, Pixel and Comic, each on a varied set of its own; R2942001: the picture each draws.
// R2943000: #2943's Set showcase and Out of frame, each on a varied set of its own; R2943001: the picture and part each
// draws. R2944000: #2944's Immersive and Prismatic, each on a varied set of its own; R2944001: the picture each draws.

import { describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { COSMETICS, COSMETIC_OFFERS } from "../wire/generated/cosmetics.ts";
import { FULL_ARTS_OF_CARD, FULL_ART_CARDS, tierOfLook } from "./fullArt.ts";
import { ownedCardCosmetics } from "./ownedFullArt.ts";
import {
  AUGMENT_ANIMATED_ID,
  AUGMENT_SIGNATURE_ID,
  NORMAL_LOOK,
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
  artPartOf,
  artPictureOf,
  augmentPicksOf,
  augmentsOf,
  impliedBy,
  isFullBleed,
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
    // R2940000: #2725's three, then #2940's three versions; R2942000: then #2942's three; R2943000: then #2943's two;
    // R2944000: then #2944's two.
    const partOne = [VERSION_FULL_ART_ID, AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID];
    expect(offered).toEqual([
      ...partOne,
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
    ]);
    expect(Object.keys(COSMETIC_OFFERS)).toEqual(offered);
    for (const id of offered) {
      const cards = COSMETIC_OFFERS[id] ?? [];
      expect(isOfferedId(id), id).toBe(true);
      // Real cards, never a token, and not every card.
      expect(cards.length, id).toBeGreaterThan(FULL_ART_CARDS.size);
      for (const card of cards) {
        expect(CATALOG[card]?.token, `${id} on ${card}`).toBe(false);
        expect(offeredOn(id, card)).toBe(true);
      }
      expect(offeredOn(id, "core-002"), `${id} on Speciesist`).toBe(false);
    }
    // Part 1's three on each of the 21 full-art cards, whose retired full arts imply them (R2725002).
    for (const id of partOne) for (const card of FULL_ART_CARDS) expect(offeredOn(id, card), `${id} on ${card}`).toBe(true);
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

  it("R2940000 each new version is offered on its own varied set of cards, never every card and never Full art's", () => {
    const on = (id: string): ReadonlySet<string> => new Set(COSMETIC_OFFERS[id] ?? []);
    const key = (cards: ReadonlySet<string>): string => [...cards].sort().join(" ");
    const collectible = Object.values(CATALOG).filter((def) => !def.token && !def.tags.includes("Token")).length;
    const versions = [VERSION_FULL_ART_ID, VERSION_ALT_ART_ID, VERSION_SKETCH_ID, VERSION_ALT_FULL_ART_ID];
    // No two of the four on the same cards, and none on every card.
    expect(new Set(versions.map((id) => key(on(id)))).size).toBe(versions.length);
    for (const id of versions) expect(on(id).size, id).toBeLessThan(collectible);
    // Alt art the widest of the three; each lands on cards another of them leaves out.
    expect(on(VERSION_ALT_ART_ID).size).toBeGreaterThan(on(VERSION_SKETCH_ID).size);
    expect(on(VERSION_ALT_ART_ID).size).toBeGreaterThan(on(VERSION_ALT_FULL_ART_ID).size);
    for (const id of [VERSION_ALT_ART_ID, VERSION_SKETCH_ID, VERSION_ALT_FULL_ART_ID]) {
      for (const other of versions.filter((version) => version !== id)) {
        expect([...on(id)].some((card) => !on(other).has(card)), `${id} beside ${other}`).toBe(true);
      }
    }
    // R2940001: never Alt art or Alt full art on a Legacy card, whose one picture is its Core card's.
    const legacy = Object.values(CATALOG).filter((def) => def.name.startsWith("Legacy: "));
    expect(legacy.length).toBeGreaterThan(0);
    for (const def of legacy) {
      expect(offeredOn(VERSION_ALT_ART_ID, def.id), def.id).toBe(false);
      expect(offeredOn(VERSION_ALT_FULL_ART_ID, def.id), def.id).toBe(false);
    }
  });

  it("R2940001 each new version's look: its picture, the full bleed of Alt full art, and Signature on none", () => {
    expect([null, VERSION_FULL_ART_ID, VERSION_ALT_ART_ID, VERSION_SKETCH_ID, VERSION_ALT_FULL_ART_ID].map(isFullBleed)).toEqual([false, true, false, false, true]);
    expect([null, VERSION_FULL_ART_ID, VERSION_ALT_ART_ID, VERSION_SKETCH_ID, VERSION_ALT_FULL_ART_ID].map((version) => artPictureOf(version))).toEqual([
      "normal",
      "normal",
      "alt",
      "sketch",
      "alt",
    ]);
    for (const version of [VERSION_ALT_ART_ID, VERSION_SKETCH_ID, VERSION_ALT_FULL_ART_ID]) {
      const both = [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID];
      // R2940002: Animated layers on it; R2949000: Signature fits none of them.
      expect(lookOf(version, both), version).toEqual({ version, animated: true, signed: false });
      expect(signatureFits(version, both), version).toBe(false);
      expect(augmentsOf(lookOf(version, both)), version).toEqual([AUGMENT_ANIMATED_ID]);
      expect(versionPickOf(version), version).toBe(version);
    }
  });

  it("R2942000 each part 3 version is offered on its own varied set, never every card, never another version's", () => {
    const on = (id: string): ReadonlySet<string> => new Set(COSMETIC_OFFERS[id] ?? []);
    const key = (cards: ReadonlySet<string>): string => [...cards].sort().join(" ");
    const collectible = Object.values(CATALOG).filter((def) => !def.token && !def.tags.includes("Token")).length;
    const versions = [
      VERSION_FULL_ART_ID,
      VERSION_ALT_ART_ID,
      VERSION_SKETCH_ID,
      VERSION_ALT_FULL_ART_ID,
      VERSION_EXTENDED_ART_ID,
      VERSION_PIXEL_ID,
      VERSION_COMIC_ID,
    ];
    // No two of the seven on the same cards, and none on every card.
    expect(new Set(versions.map((id) => key(on(id)))).size).toBe(versions.length);
    for (const id of versions) expect(on(id).size, id).toBeLessThan(collectible);
    for (const id of [VERSION_EXTENDED_ART_ID, VERSION_PIXEL_ID, VERSION_COMIC_ID]) {
      // Each holds a card no other version takes, and none is on Speciesist, a Common Unit.
      const others = new Set(versions.filter((version) => version !== id).flatMap((version) => [...on(version)]));
      expect([...on(id)].some((card) => !others.has(card)), id).toBe(true);
      expect(offeredOn(id, "core-002"), `${id} on Speciesist`).toBe(false);
      // Each lands on cards of every type its clauses name and on Legendary and Mythic cards of any type.
      const rarities = new Set([...on(id)].map((card) => CATALOG[card]?.rarity));
      expect(rarities.has("Legendary") && rarities.has("Mythic"), id).toBe(true);
    }
    // Extended art on Spells of every rarity; Pixel and Comic never on a Common card.
    const commons = (id: string): number => [...on(id)].filter((card) => CATALOG[card]?.rarity === "Common").length;
    expect(commons(VERSION_EXTENDED_ART_ID)).toBeGreaterThan(0);
    expect([commons(VERSION_PIXEL_ID), commons(VERSION_COMIC_ID)]).toEqual([0, 0]);
    for (const card of on(VERSION_EXTENDED_ART_ID)) {
      const def = CATALOG[card];
      expect(def?.type === "Spell" || def?.type === "Field Spell" || def?.rarity === "Legendary" || def?.rarity === "Mythic", card).toBe(true);
    }
  });

  it("R2942001 artPictureOf, the full bleed and Signature on none for part 3", () => {
    const partThree = [VERSION_EXTENDED_ART_ID, VERSION_PIXEL_ID, VERSION_COMIC_ID];
    // Extended art draws the normal picture and is no full art (its words stay); Pixel and Comic restyle it.
    expect(partThree.map(isFullBleed)).toEqual([false, false, false]);
    expect(partThree.map((version) => artPictureOf(version))).toEqual(["normal", "pixel", "comic"]);
    for (const version of partThree) {
      const both = [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID];
      // R2942002: Animated layers on it; R2949000: Signature fits none of them.
      expect(lookOf(version, both), version).toEqual({ version, animated: true, signed: false });
      expect(signatureFits(version, both), version).toBe(false);
      expect(augmentsOf(lookOf(version, both)), version).toEqual([AUGMENT_ANIMATED_ID]);
      expect(versionPickOf(version), version).toBe(version);
    }
  });

  it("R2943000 each part 4 version is offered on its own varied set, never every card, never another version's", () => {
    const on = (id: string): ReadonlySet<string> => new Set(COSMETIC_OFFERS[id] ?? []);
    const key = (cards: ReadonlySet<string>): string => [...cards].sort().join(" ");
    const collectible = Object.values(CATALOG).filter((def) => !def.token && !def.tags.includes("Token")).length;
    const versions = [
      VERSION_FULL_ART_ID,
      VERSION_ALT_ART_ID,
      VERSION_SKETCH_ID,
      VERSION_ALT_FULL_ART_ID,
      VERSION_EXTENDED_ART_ID,
      VERSION_PIXEL_ID,
      VERSION_COMIC_ID,
      VERSION_SET_SHOWCASE_ID,
      VERSION_OUT_OF_FRAME_ID,
    ];
    // No two of the nine on the same cards, and none on every card.
    expect(new Set(versions.map((id) => key(on(id)))).size).toBe(versions.length);
    for (const id of versions) expect(on(id).size, id).toBeLessThan(collectible);
    // Set showcase on Legendary and Mythic cards of every type; Out of frame on Rare to Mythic Units, never a Legacy card
    // nor the Starship, whose pictures have no subject of their own to step out.
    const legendary = (card: string): boolean => CATALOG[card]?.rarity === "Legendary" || CATALOG[card]?.rarity === "Mythic";
    for (const card of on(VERSION_SET_SHOWCASE_ID)) expect(legendary(card), card).toBe(true);
    expect(new Set([...on(VERSION_SET_SHOWCASE_ID)].map((card) => CATALOG[card]?.type)).size).toBeGreaterThan(2);
    for (const card of on(VERSION_OUT_OF_FRAME_ID)) {
      expect(CATALOG[card]?.type, card).toBe("Unit");
      expect(CATALOG[card]?.rarity, card).not.toBe("Common");
      expect(card, card).not.toMatch(/^traininggrounds-0(4[1-9]|5\d|60)$/);
      expect(card).not.toBe("stinkyscape-t-dormant-starship");
    }
    for (const id of [VERSION_SET_SHOWCASE_ID, VERSION_OUT_OF_FRAME_ID]) {
      expect(offeredOn(id, "core-002"), `${id} on Speciesist`).toBe(false);
      expect(offeredOn(id, "classic-080"), `${id} on BOOM! Big Max`).toBe(true);
    }
  });

  it("R2943001 artPictureOf, artPartOf and Signature on none for part 4", () => {
    // Set showcase draws its set's own treatment, Classic+ and Meditative+ their parents', and the normal picture
    // where a card has no set of its own; Out of frame draws the normal picture, its subject apart.
    const sets = ["Core", "Classic", "Classic+", "Meditative", "Meditative+", "Stinkyscape", "Training Grounds"];
    expect(sets.map((set) => artPictureOf(VERSION_SET_SHOWCASE_ID, set))).toEqual([
      "showcase-core",
      "showcase-classic",
      "showcase-classic",
      "showcase-meditative",
      "showcase-meditative",
      "showcase-stinkyscape",
      "showcase-training-grounds",
    ]);
    expect([artPictureOf(VERSION_SET_SHOWCASE_ID), artPictureOf(VERSION_SET_SHOWCASE_ID, "Boss")]).toEqual(["normal", "normal"]);
    for (const set of sets) expect(artPictureOf(VERSION_OUT_OF_FRAME_ID, set), set).toBe("normal");
    expect([null, VERSION_FULL_ART_ID, VERSION_COMIC_ID, VERSION_SET_SHOWCASE_ID, VERSION_OUT_OF_FRAME_ID].map((version) => artPartOf(version))).toEqual([
      "whole",
      "whole",
      "whole",
      "whole",
      "subject",
    ]);
    for (const version of [VERSION_SET_SHOWCASE_ID, VERSION_OUT_OF_FRAME_ID]) {
      const both = [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID];
      // Neither is a full art (its words stay); R2943002: Animated layers on it; R2949000: Signature fits neither.
      expect(isFullBleed(version), version).toBe(false);
      expect(lookOf(version, both), version).toEqual({ version, animated: true, signed: false });
      expect(signatureFits(version, both), version).toBe(false);
      expect(augmentsOf(lookOf(version, both)), version).toEqual([AUGMENT_ANIMATED_ID]);
      expect(versionPickOf(version), version).toBe(version);
    }
  });
  it("R2944000 each part 5 version is offered on its own varied set, never every card, never another version's", () => {
    const on = (id: string): ReadonlySet<string> => new Set(COSMETIC_OFFERS[id] ?? []);
    const key = (cards: ReadonlySet<string>): string => [...cards].sort().join(" ");
    const collectible = Object.values(CATALOG).filter((def) => !def.token && !def.tags.includes("Token"));
    const versions = [
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
    ];
    // No two of the eleven on the same cards, and none on every card.
    expect(new Set(versions.map((id) => key(on(id)))).size).toBe(versions.length);
    for (const id of versions) expect(on(id).size, id).toBeLessThan(collectible.length);
    // Both on Legendary and Mythic cards alone, the few marquee cards of each set: Immersive never on the Starship, whose
    // picture has no subject of its own to lift, and Prismatic, the top chase, on every Mythic card.
    const legendary = (card: string): boolean => CATALOG[card]?.rarity === "Legendary" || CATALOG[card]?.rarity === "Mythic";
    for (const id of [VERSION_IMMERSIVE_ID, VERSION_PRISMATIC_ID]) {
      for (const card of on(id)) expect(legendary(card), `${id} on ${card}`).toBe(true);
      expect(new Set([...on(id)].map((card) => CATALOG[card]?.type)).size, id).toBeGreaterThan(2);
    }
    expect(on(VERSION_IMMERSIVE_ID).has("stinkyscape-t-dormant-starship")).toBe(false);
    for (const def of collectible.filter((each) => each.rarity === "Mythic")) expect(on(VERSION_PRISMATIC_ID).has(def.id), def.id).toBe(true);
    for (const id of [VERSION_IMMERSIVE_ID, VERSION_PRISMATIC_ID]) {
      expect(offeredOn(id, "core-002"), `${id} on Speciesist`).toBe(false);
      expect(offeredOn(id, "core-100"), `${id} on Ceaseless Void`).toBe(true);
    }
  });

  it("R2944001 artPictureOf, artPartOf and Signature on none for part 5", () => {
    // Immersive draws its depth (its window the backdrop, CardFace; a tile the whole picture), Prismatic its chrome,
    // whatever the card's set.
    for (const set of [null, "Core", "Classic+", "Stinkyscape", "Training Grounds"]) {
      expect(artPictureOf(VERSION_IMMERSIVE_ID, set), `${String(set)}`).toBe("depth");
      expect(artPictureOf(VERSION_PRISMATIC_ID, set), `${String(set)}`).toBe("prismatic");
    }
    expect([VERSION_IMMERSIVE_ID, VERSION_PRISMATIC_ID].map((version) => artPartOf(version))).toEqual(["whole", "whole"]);
    for (const version of [VERSION_IMMERSIVE_ID, VERSION_PRISMATIC_ID]) {
      const both = [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID];
      // Neither is a full art (its words stay); R2944002: Animated layers on it; R2949000: Signature fits neither.
      expect(isFullBleed(version), version).toBe(false);
      expect(lookOf(version, both), version).toEqual({ version, animated: true, signed: false });
      expect(signatureFits(version, both), version).toBe(false);
      expect(augmentsOf(lookOf(version, both)), version).toEqual([AUGMENT_ANIMATED_ID]);
      expect(versionPickOf(version), version).toBe(version);
    }
  });
});
