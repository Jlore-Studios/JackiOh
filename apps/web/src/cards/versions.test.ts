// R2725000 (#2725): a card's look, read from its pick (versions.ts). R2725001: the offers are the server's own
// (`COSMETIC_OFFERS`); R2725002: a retired full art's pick looks as it did and owns what it combines. R2949001: Full
// art's offer is a varied share of the cards. R2940000: #2940's Alt art, Sketch and Alt full art, each on a varied set
// of its own; R2940001: the full bleed and the picture each version draws. R2942000: #2942's Extended art, Pixel and
// Comic, each on a varied set of its own; R2942001: the picture each draws. R2943000: #2943's Set showcase and Out of
// frame, each on a varied set of its own; R2943001: the picture and part each draws. R2944000: #2944's Immersive and
// Prismatic, each on a varied set of its own; R2944001: the picture each draws.
// R3015000 (#3015): each look is a variant, a ladder of levels, every card's Default first; the Animated and Signature
// augments are retired. R3015005: a card's pick names one level, an older pick's shape mapped as the server maps it
// (`levelOfPick`, the server's `level_of_pick`), worn at the highest owned and offered at or below it (`wornLevel`).
// R3015006: a retired entry owns the levels it maps to (`impliedBy`). R3017000 (#3017): six two-level variants, Night,
// Neon sign, Watercolor, Stained-glass window, Woodcut and Tarot card; R3017001: three pairs, each splitting every
// card between its two, so every card has at least `VARIANT_MIN_PER_CARD` variants besides the Default; R3017002: the
// picture each draws.

import { describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { SHIPPED_SETS } from "@jackioh/shared";

import { COSMETICS, COSMETIC_LEVELS, COSMETIC_OFFERS } from "../wire/generated/cosmetics.ts";
import { FULL_ARTS_OF_CARD, FULL_ART_CARDS, tierOfLook } from "./fullArt.ts";
import { ownedCardCosmetics } from "./ownedFullArt.ts";
import { aiConstants } from "../wasm/index.ts";
import {
  AUGMENT_ANIMATED_ID,
  AUGMENT_SIGNATURE_ID,
  DEFAULT_VARIANT,
  NORMAL_LOOK,
  VARIANTS,
  VERSION_ALT_ART_ID,
  VERSION_ALT_FULL_ART_ANIMATED_ID,
  VERSION_ALT_FULL_ART_ID,
  VERSION_ALT_FULL_ART_SIGNED_ID,
  VERSION_ANIMATED_ID,
  VERSION_COMIC_ID,
  VERSION_EXTENDED_ART_ID,
  VERSION_FULL_ART_ANIMATED_ID,
  VERSION_FULL_ART_ID,
  VERSION_FULL_ART_SIGNED_ID,
  VERSION_IMMERSIVE_ID,
  VERSION_NEON_ID,
  VERSION_NIGHT_ID,
  VERSION_OUT_OF_FRAME_ID,
  VERSION_PIXEL_ID,
  VERSION_PRISMATIC_ID,
  VERSION_SET_SHOWCASE_ID,
  VERSION_SKETCH_FULL_ART_ANIMATED_ID,
  VERSION_SKETCH_FULL_ART_ID,
  VERSION_SKETCH_FULL_ART_SIGNED_ID,
  VERSION_SKETCH_ID,
  VERSION_STAINED_GLASS_ID,
  VERSION_TAROT_ID,
  VERSION_WATERCOLOR_ID,
  VERSION_WOODCUT_ID,
  artPartOf,
  artPictureOf,
  impliedBy,
  isFullBleed,
  defaultLookOf,
  isNextLevel,
  isOfferedId,
  ladderOf,
  levelOf,
  levelOfPick,
  levelsBelow,
  levelsOn,
  lookOfLevel,
  lookOn,
  offeredOn,
  variantsOn,
  wornLevel,
  type CardLook,
} from "./versions.ts";

const VANILLA = "core-008";
const ANIM = AUGMENT_ANIMATED_ID;
const SIG = AUGMENT_SIGNATURE_ID;

/** Mr. Vanilla's retired full art of `tier`. */
function retired(tier: "Epic" | "Legendary" | "Mythic" | "Animated"): string {
  const id = FULL_ARTS_OF_CARD.get(VANILLA)?.get(tier);
  if (id === undefined) throw new Error(`Mr. Vanilla has no ${tier} full art`);
  return id;
}

/** R3015005: the look a pick of `fullArt` and `augments` on Mr. Vanilla names, owned or not. */
function lookOfPick(fullArt: boolean | string | null | undefined, augments: readonly string[] = []): CardLook {
  return lookOfLevel(levelOfPick(VANILLA, fullArt, augments));
}

/** The seven one-art variants of #2942–#2944, whose ladders are art then animated (R3015000). */
const ONE_ART = [VERSION_EXTENDED_ART_ID, VERSION_PIXEL_ID, VERSION_COMIC_ID, VERSION_SET_SHOWCASE_ID, VERSION_OUT_OF_FRAME_ID, VERSION_IMMERSIVE_ID, VERSION_PRISMATIC_ID];

/** R3017000: #3017's six variants' first levels, in the registry's order, each ladder art then animated. */
const PART_SIX = [VERSION_NIGHT_ID, VERSION_NEON_ID, VERSION_WATERCOLOR_ID, VERSION_STAINED_GLASS_ID, VERSION_WOODCUT_ID, VERSION_TAROT_ID];

/** R3017001: #3017's pairs, each splitting every card between its two. */
const PAIRS = [
  [VERSION_NIGHT_ID, VERSION_NEON_ID],
  [VERSION_WATERCOLOR_ID, VERSION_STAINED_GLASS_ID],
  [VERSION_WOODCUT_ID, VERSION_TAROT_ID],
] as const;

const COLLECTIBLE = Object.values(CATALOG).filter((def) => !def.token && !def.tags.includes("Token"));
const isLegacy = (card: string): boolean => CATALOG[card]?.tags.includes("Legacy") === true;

describe("R2725000 a card's look", () => {
  it("R3015000 every variant is a ladder of levels, ordered, signed only above an animated full art, every card with its Default", () => {
    expect(VARIANTS).toEqual([
      DEFAULT_VARIANT,
      "full-art",
      "alt-art",
      "sketch",
      "extended-art",
      "pixel",
      "comic",
      "set-showcase",
      "out-of-frame",
      "immersive",
      "prismatic",
      "night",
      "neon",
      "watercolor",
      "stained-glass",
      "woodcut",
      "tarot",
    ]);
    expect(ladderOf(DEFAULT_VARIANT)).toEqual([VERSION_ANIMATED_ID]);
    expect(ladderOf("full-art")).toEqual([VERSION_FULL_ART_ID, VERSION_FULL_ART_ANIMATED_ID, VERSION_FULL_ART_SIGNED_ID]);
    expect(ladderOf("alt-art")).toEqual([VERSION_ALT_ART_ID, VERSION_ALT_FULL_ART_ID, VERSION_ALT_FULL_ART_ANIMATED_ID, VERSION_ALT_FULL_ART_SIGNED_ID]);
    expect(ladderOf("sketch")).toEqual([VERSION_SKETCH_ID, VERSION_SKETCH_FULL_ART_ID, VERSION_SKETCH_FULL_ART_ANIMATED_ID, VERSION_SKETCH_FULL_ART_SIGNED_ID]);
    for (const first of [...ONE_ART, ...PART_SIX]) {
      const variant = levelOf(first)?.variant ?? "";
      // No signed level: art, then animated; no full-art level but (R3343001) Prismatic's, its one art level.
      expect(ladderOf(variant), variant).toEqual([first, `${first}-animated`]);
      expect(ladderOf(variant).some(isFullBleed), variant).toBe(first === VERSION_PRISMATIC_ID);
    }
    expect(ladderOf("variant-from-a-newer-client")).toEqual([]);
    // Every version entry is a level, and every level a version entry.
    expect(Object.keys(COSMETIC_LEVELS).sort()).toEqual(COSMETICS.filter((cosmetic) => cosmetic.kind === "version").map((cosmetic) => cosmetic.id).sort());
    expect(levelOf(retired("Epic"))).toBeNull();
    expect(levelOf(ANIM)).toBeNull();

    for (const variant of VARIANTS) {
      const ladder = ladderOf(variant);
      // Ranks gapless from 1 (from 2 for the Default, whose first level is the normal face), each level the ones below
      // it plus one more thing: an art level, a full-art one, then animated, then signed.
      const from = variant === DEFAULT_VARIANT ? 2 : 1;
      expect(ladder.map((id) => levelOf(id)?.rank), variant).toEqual(ladder.map((_, index) => from + index));
      expect(ladder.map((id) => levelOf(id)?.adds).join(" "), variant).toMatch(variant === DEFAULT_VARIANT ? /^animated$/ : /^(art )?(fullArt )?animated( signed)?$/);
      ladder.forEach((id, index) => {
        expect(levelsBelow(id), id).toEqual(ladder.slice(0, index));
        const look = lookOfLevel(id);
        expect(look.level, id).toBe(id);
        expect(look.animated, id).toBe(ladder.slice(0, index + 1).some((each) => levelOf(each)?.adds === "animated"));
        expect(look.signed, id).toBe(levelOf(id)?.adds === "signed");
        // Signed only where the level below is an animated full art.
        if (levelOf(id)?.adds === "signed") {
          const below = ladder[index - 1] ?? null;
          expect(below === null ? null : levelOf(below)?.adds, id).toBe("animated");
          expect(isFullBleed(below), id).toBe(true);
        }
        // Each level is offered on its first level's cards (the Default's every card), a moving one never on a Legacy card.
        // R3343000: where the first is the card's default look, offered nowhere there, the levels above it share one offer.
        const first = ladder[0] ?? id;
        const moves = look.animated || look.signed;
        for (const def of COLLECTIBLE) {
          if (variant !== DEFAULT_VARIANT && defaultLookOf(def.id) === first) {
            expect(offeredOn(first, def.id), `${first} on ${def.id}`).toBe(false);
            if (index > 0) expect(offeredOn(id, def.id), `${id} on ${def.id}`).toBe(offeredOn(ladder[1] ?? id, def.id));
            continue;
          }
          const base = variant === DEFAULT_VARIANT || offeredOn(first, def.id);
          expect(offeredOn(id, def.id), `${id} on ${def.id}`).toBe(base && !(moves && isLegacy(def.id)));
        }
      });
    }

    // Every card has the Default variant (its first level the normal face), and each other whose first level it is offered
    // (R3343002: or, where that is its default look, a level above it).
    const legacy = COLLECTIBLE.filter((def) => isLegacy(def.id));
    expect(legacy.length).toBeGreaterThan(0);
    for (const def of COLLECTIBLE) {
      const variants = variantsOn(def.id);
      expect(variants[0], def.id).toBe(DEFAULT_VARIANT);
      for (const variant of VARIANTS.slice(1)) {
        const first = ladderOf(variant)[0] ?? "";
        const listed = defaultLookOf(def.id) === first ? levelsOn(def.id, variant).length > 0 : offeredOn(first, def.id);
        expect(variants.includes(variant), `${variant} on ${def.id}`).toBe(listed);
      }
    }
    // A Legacy card's picture never moves (R2724000): its ladders stop before animated, so its Default has no level past
    // the normal face, and no level of any variant it is offered moves or signs.
    for (const def of legacy) {
      expect(levelsOn(def.id, DEFAULT_VARIANT), def.id).toEqual([]);
      for (const variant of variantsOn(def.id)) {
        for (const id of levelsOn(def.id, variant)) {
          expect(lookOfLevel(id).animated || lookOfLevel(id).signed, `${id} on ${def.id}`).toBe(false);
        }
      }
    }
    expect(levelsOn("core-002", DEFAULT_VARIANT)).toEqual([VERSION_ANIMATED_ID]);
    // R3015003, R3015004: with nothing owned, only a variant's first level is next; owning it makes the next one next.
    const none = (): boolean => false;
    expect(isNextLevel(VERSION_SKETCH_ID, VANILLA, none)).toBe(true);
    expect(isNextLevel(VERSION_ANIMATED_ID, VANILLA, none)).toBe(true);
    expect(isNextLevel(VERSION_SKETCH_FULL_ART_ID, VANILLA, none)).toBe(false);
    expect(isNextLevel(VERSION_SKETCH_FULL_ART_ID, VANILLA, (_, id) => id === VERSION_SKETCH_ID)).toBe(true);
    expect(isNextLevel(VERSION_SKETCH_FULL_ART_SIGNED_ID, VANILLA, (_, id) => id === VERSION_SKETCH_ID)).toBe(false);
    expect(isNextLevel("foil-galaxy", VANILLA, none)).toBe(true);
    // The retired augments have no offer and no level.
    for (const id of [ANIM, SIG]) {
      expect(isOfferedId(id), id).toBe(false);
      expect(offeredOn(id, VANILLA), id).toBe(false);
    }
  });

  it("R3015005 levelOfPick maps every old pick as the server does", () => {
    // No pick, `false` and an id this client does not know: no level, the normal face.
    for (const pick of [undefined, null, false, "full-art-from-a-newer-client", "version-from-a-newer-client"]) {
      expect(levelOfPick(VANILLA, pick), String(pick)).toBeNull();
    }
    // `true`: Full art's first level on a full-art card (its still Epic, R2655000), nothing on any other.
    expect(levelOfPick(VANILLA, true)).toBe(VERSION_FULL_ART_ID);
    expect(levelOfPick("core-002", true)).toBeNull();
    // A level by its id, itself, on any card (the wear path asks the offer).
    for (const id of Object.keys(COSMETIC_LEVELS)) expect(levelOfPick(VANILLA, id), id).toBe(id);
    // Each retired full art of the card: the highest level it maps to, a non-Default one first.
    expect(levelOfPick(VANILLA, retired("Epic"))).toBe(VERSION_FULL_ART_ID);
    expect(levelOfPick(VANILLA, retired("Legendary"))).toBe(VERSION_FULL_ART_ANIMATED_ID);
    expect(levelOfPick(VANILLA, retired("Mythic"))).toBe(VERSION_FULL_ART_SIGNED_ID);
    expect(levelOfPick(VANILLA, retired("Animated"))).toBe(VERSION_ANIMATED_ID);
    // Another card's retired full art names nothing on this one.
    expect(levelOfPick("core-002", retired("Mythic"))).toBeNull();
    // The old augments step up one rank where the next level adds them: Animated, then Signature.
    expect(levelOfPick(VANILLA, VERSION_FULL_ART_ID, [ANIM])).toBe(VERSION_FULL_ART_ANIMATED_ID);
    // R2949000's Signature look (Full art, Animated and Signature): Full art's third level.
    expect(levelOfPick(VANILLA, VERSION_FULL_ART_ID, [ANIM, SIG])).toBe(VERSION_FULL_ART_SIGNED_ID);
    expect(levelOfPick(VANILLA, true, [ANIM, SIG])).toBe(VERSION_FULL_ART_SIGNED_ID);
    expect(levelOfPick(VANILLA, retired("Epic"), [ANIM])).toBe(VERSION_FULL_ART_ANIMATED_ID);
    expect(levelOfPick(VANILLA, retired("Epic"), [ANIM, SIG])).toBe(VERSION_FULL_ART_SIGNED_ID);
    expect(levelOfPick(VANILLA, retired("Legendary"), [SIG])).toBe(VERSION_FULL_ART_SIGNED_ID);
    // Animated on the normal face: the Default's second level, whatever else it lists.
    expect(levelOfPick(VANILLA, false, [ANIM])).toBe(VERSION_ANIMATED_ID);
    expect(levelOfPick(VANILLA, undefined, [ANIM, SIG])).toBe(VERSION_ANIMATED_ID);
    expect(levelOfPick(VANILLA, "version-from-a-newer-client", [ANIM])).toBe(VERSION_ANIMATED_ID);
    expect(levelOfPick(VANILLA, retired("Animated"), [ANIM, SIG])).toBe(VERSION_ANIMATED_ID);
    // Alt art or the Sketch with Animated stays at its first level (its second is a full art: ruled); Alt full art with
    // Animated is Alt art's third, with Signature too its fourth.
    expect(levelOfPick(VANILLA, VERSION_ALT_ART_ID, [ANIM])).toBe(VERSION_ALT_ART_ID);
    expect(levelOfPick(VANILLA, VERSION_SKETCH_ID, [ANIM])).toBe(VERSION_SKETCH_ID);
    expect(levelOfPick(VANILLA, VERSION_SKETCH_ID, [ANIM, SIG])).toBe(VERSION_SKETCH_ID);
    expect(levelOfPick(VANILLA, VERSION_ALT_FULL_ART_ID, [ANIM])).toBe(VERSION_ALT_FULL_ART_ANIMATED_ID);
    expect(levelOfPick(VANILLA, VERSION_ALT_FULL_ART_ID, [ANIM, SIG])).toBe(VERSION_ALT_FULL_ART_SIGNED_ID);
    // A one-art version with Animated is its second level; Signature has no level to step to on it.
    for (const first of ONE_ART) {
      expect(levelOfPick(VANILLA, first, [ANIM]), first).toBe(`${first}-animated`);
      expect(levelOfPick(VANILLA, first, [ANIM, SIG]), first).toBe(`${first}-animated`);
      expect(levelOfPick(VANILLA, first, [SIG]), first).toBe(first);
    }
    expect(levelOfPick(VANILLA, VERSION_PIXEL_ID, [ANIM])).toBe(`${VERSION_PIXEL_ID}-animated`);
    // Signature alone steps nowhere; an augment this client does not know is passed over.
    expect(levelOfPick(VANILLA, VERSION_FULL_ART_ID, [SIG])).toBe(VERSION_FULL_ART_ID);
    expect(levelOfPick(VANILLA, false, [SIG])).toBeNull();
    expect(levelOfPick(VANILLA, false, ["augment-from-a-newer-client"])).toBeNull();
    expect(levelOfPick(VANILLA, VERSION_FULL_ART_ID, ["augment-from-a-newer-client", ANIM])).toBe(VERSION_FULL_ART_ANIMATED_ID);

    // `wornLevel` wears the level where it may, else the highest below it that it may, else none.
    const owning =
      (...ids: string[]) =>
      (id: string): boolean =>
        ids.includes(id);
    expect(wornLevel(VERSION_FULL_ART_SIGNED_ID, owning(VERSION_FULL_ART_ID, VERSION_FULL_ART_ANIMATED_ID, VERSION_FULL_ART_SIGNED_ID))).toBe(VERSION_FULL_ART_SIGNED_ID);
    expect(wornLevel(VERSION_FULL_ART_SIGNED_ID, owning(VERSION_FULL_ART_ID, VERSION_FULL_ART_ANIMATED_ID))).toBe(VERSION_FULL_ART_ANIMATED_ID);
    expect(wornLevel(VERSION_FULL_ART_SIGNED_ID, owning(VERSION_FULL_ART_ID))).toBe(VERSION_FULL_ART_ID);
    expect(wornLevel(VERSION_FULL_ART_SIGNED_ID, owning(VERSION_ANIMATED_ID))).toBeNull();
    expect(wornLevel(VERSION_ANIMATED_ID, owning())).toBeNull();
    expect(wornLevel(null, owning(VERSION_FULL_ART_ID))).toBeNull();
  });

  it("R2725002 R3015005 legacy picks look as before", () => {
    // Each retired full art's pick, and `true`, prints the tier it printed before #2725, through the level it maps to.
    expect(tierOfLook(lookOfPick(true))).toBe("Epic");
    for (const tier of ["Epic", "Legendary", "Mythic", "Animated"] as const) {
      expect(tierOfLook(lookOfPick(retired(tier))), tier).toBe(tier);
    }
    expect(lookOfPick(retired("Mythic"))).toEqual({ level: VERSION_FULL_ART_SIGNED_ID, version: VERSION_FULL_ART_ID, animated: true, signed: true });
    expect(lookOfPick(retired("Animated"))).toEqual({ level: VERSION_ANIMATED_ID, version: null, animated: true, signed: false });
    // No pick, `false` and an id this client does not know are the normal face.
    for (const pick of [undefined, null, false, "full-art-from-a-newer-client", "version-from-a-newer-client"]) {
      expect(lookOfPick(pick), String(pick)).toEqual(NORMAL_LOOK);
    }
    expect(lookOfLevel(null)).toBe(NORMAL_LOOK);
    expect(lookOfLevel("version-from-a-newer-client")).toBe(NORMAL_LOOK);
    // The augments of an older pick step up its level, a retired one's too, and an unknown one adds nothing.
    expect(lookOfPick(VERSION_FULL_ART_ID, [ANIM, SIG])).toEqual({ level: VERSION_FULL_ART_SIGNED_ID, version: VERSION_FULL_ART_ID, animated: true, signed: true });
    expect(tierOfLook(lookOfPick(VERSION_FULL_ART_ID, [ANIM, SIG]))).toBe("Mythic");
    expect(lookOfPick(false, [SIG])).toEqual(NORMAL_LOOK);
    expect(tierOfLook(lookOfPick(false, [SIG]))).toBeNull();
    expect(lookOfPick(retired("Epic"), [ANIM])).toEqual(lookOfPick(retired("Legendary")));
    expect(lookOfPick(false, ["augment-from-a-newer-client"])).toBe(NORMAL_LOOK);
  });

  it("R2949000 R3015005 a pick signs only at Full art's signed level, which an older Signature pick reaches beside Full art and Animated alone", () => {
    // The level, or the Full art version, `true` or a retired full art mapping to it, with Animated listed or implied.
    for (const [fullArt, augments] of [
      [VERSION_FULL_ART_SIGNED_ID, []],
      [VERSION_FULL_ART_ID, [ANIM, SIG]],
      [true, [ANIM, SIG]],
      [retired("Legendary"), [SIG]],
      [retired("Mythic"), []],
      [retired("Epic"), [ANIM, SIG]],
    ] as const) {
      expect(lookOfPick(fullArt, augments).signed, `${String(fullArt)} ${augments.join(" ")}`).toBe(true);
    }
    // Without Animated, without the version, or a version this client does not know.
    for (const [fullArt, augments] of [
      [VERSION_FULL_ART_ID, [SIG]],
      [retired("Epic"), [SIG]],
      [retired("Animated"), [ANIM, SIG]],
      [false, [ANIM, SIG]],
      [undefined, [ANIM, SIG]],
      [null, [ANIM, SIG]],
      ["version-from-a-newer-client", [ANIM, SIG]],
    ] as const) {
      expect(lookOfPick(fullArt, augments).signed, `${String(fullArt)} ${augments.join(" ")}`).toBe(false);
    }
    // Animated still moves where Signature cannot, and a retired Mythic signs on its own.
    expect(lookOfPick(false, [ANIM, SIG])).toEqual({ level: VERSION_ANIMATED_ID, version: null, animated: true, signed: false });
    expect(lookOfPick(VERSION_FULL_ART_ID, [SIG])).toEqual({ level: VERSION_FULL_ART_ID, version: VERSION_FULL_ART_ID, animated: false, signed: false });
    expect(lookOfPick(retired("Mythic"))).toEqual({ level: VERSION_FULL_ART_SIGNED_ID, version: VERSION_FULL_ART_ID, animated: true, signed: true });
    expect(lookOfPick(retired("Animated"), [SIG])).toEqual({ level: VERSION_ANIMATED_ID, version: null, animated: true, signed: false });
    expect(tierOfLook(lookOfLevel(VERSION_FULL_ART_SIGNED_ID))).toBe("Mythic");
  });

  it("R2725002 R3015006 a retired entry owns the levels it maps to, on its own card alone", () => {
    expect(impliedBy(retired("Epic"))).toEqual([VERSION_FULL_ART_ID]);
    expect(impliedBy(retired("Legendary"))).toEqual([VERSION_FULL_ART_ID, VERSION_FULL_ART_ANIMATED_ID, VERSION_ANIMATED_ID]);
    expect(impliedBy(retired("Mythic"))).toEqual([VERSION_FULL_ART_ID, VERSION_FULL_ART_ANIMATED_ID, VERSION_FULL_ART_SIGNED_ID, VERSION_ANIMATED_ID]);
    expect(impliedBy(retired("Animated"))).toEqual([VERSION_ANIMATED_ID]);
    expect(impliedBy(ANIM)).toEqual([VERSION_ANIMATED_ID]);
    // Signature alone owns no level.
    expect(impliedBy(SIG)).toEqual([]);
    expect(impliedBy("foil-galaxy")).toEqual([]);
    expect(impliedBy(VERSION_FULL_ART_ID)).toEqual([]);
    const owned = ownedCardCosmetics({
      cards: [
        { cardId: VANILLA, cosmeticId: retired("Legendary") },
        // A level on a card it is not offered, and another card's full art: neither counts.
        { cardId: "core-002", cosmeticId: VERSION_FULL_ART_ID },
        { cardId: "core-002", cosmeticId: retired("Epic") },
      ],
      account: [],
    });
    expect([...(owned.get(VANILLA) ?? [])].sort()).toEqual([retired("Legendary"), VERSION_FULL_ART_ID, VERSION_FULL_ART_ANIMATED_ID, VERSION_ANIMATED_ID].sort());
    expect(owned.get("core-002")).toBeUndefined();
  });

  it("R2725001 R3015000 offers match COSMETIC_OFFERS", () => {
    const offered = COSMETICS.filter((cosmetic) => cosmetic.kind === "version" || cosmetic.kind === "augment").map((cosmetic) => cosmetic.id);
    // R2940000: #2725's three, then #2940's three versions; R2942000: then #2942's three; R2943000: then #2943's two;
    // R2944000: then #2944's two; R3015000: then #3015's fifteen levels; R3017000: then #3017's twelve. The two augments
    // are retired: no offer.
    expect(offered).toEqual([
      VERSION_FULL_ART_ID,
      ANIM,
      SIG,
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
      VERSION_ANIMATED_ID,
      VERSION_FULL_ART_ANIMATED_ID,
      VERSION_FULL_ART_SIGNED_ID,
      VERSION_ALT_FULL_ART_ANIMATED_ID,
      VERSION_ALT_FULL_ART_SIGNED_ID,
      VERSION_SKETCH_FULL_ART_ID,
      VERSION_SKETCH_FULL_ART_ANIMATED_ID,
      VERSION_SKETCH_FULL_ART_SIGNED_ID,
      ...ONE_ART.map((first) => `${first}-animated`),
      ...PART_SIX.flatMap((id) => [id, `${id}-animated`]),
    ]);
    const levels = offered.filter((id) => id !== ANIM && id !== SIG);
    expect(Object.keys(COSMETIC_OFFERS)).toEqual(levels);
    for (const id of levels) {
      const cards = COSMETIC_OFFERS[id] ?? [];
      expect(isOfferedId(id), id).toBe(true);
      // Real cards, never a token, and not every card.
      expect(cards.length, id).toBeGreaterThan(FULL_ART_CARDS.size);
      expect(cards.length, id).toBeLessThan(COLLECTIBLE.length);
      for (const card of cards) {
        expect(CATALOG[card]?.token, `${id} on ${card}`).toBe(false);
        expect(offeredOn(id, card)).toBe(true);
      }
      // Every level but the Default's animated one, which every card but a Legacy one is offered, passes Speciesist by,
      // and the part 6 ones of each pair its side of the split lands on (R3017001).
      const speciesist = [VERSION_ANIMATED_ID, ...[VERSION_NEON_ID, VERSION_WATERCOLOR_ID, VERSION_WOODCUT_ID].flatMap((first) => [first, `${first}-animated`])];
      expect(offeredOn(id, "core-002"), `${id} on Speciesist`).toBe(speciesist.includes(id));
    }
    // Full art's levels and the Default's animated one on each of the 21 full-art cards, whose retired full arts map to
    // them (R3015006), but Full art's first on a Mythic one, whose Default draws it (R3343000).
    for (const id of [...ladderOf("full-art"), VERSION_ANIMATED_ID]) {
      for (const card of FULL_ART_CARDS) expect(offeredOn(id, card), `${id} on ${card}`).toBe(id !== defaultLookOf(card));
    }
    // No other id has an offer.
    expect(isOfferedId("foil-galaxy")).toBe(false);
    expect(isOfferedId(retired("Epic"))).toBe(false);
    expect(offeredOn("foil-galaxy", VANILLA)).toBe(false);
  });

  it("R2949001 R3015000 Full art's offer is a varied share holding the 21, and its moving levels take all of it but its Legacy cards", () => {
    const on = (id: string): ReadonlySet<string> => new Set(COSMETIC_OFFERS[id] ?? []);
    const fullArt = on(VERSION_FULL_ART_ID);
    const animated = on(VERSION_ANIMATED_ID);
    // Full art is offered on fewer cards than the Default's animated level, and that on cards Full art is not: the
    // looks differ by card.
    expect(fullArt.size).toBeLessThan(animated.size);
    expect([...animated].some((card) => !fullArt.has(card))).toBe(true);
    // The 21 with retired full arts are always among them, but a Mythic one, whose Default draws it (R3343000).
    for (const card of FULL_ART_CARDS) expect(fullArt.has(card) || defaultLookOf(card) === VERSION_FULL_ART_ID, card).toBe(true);
    // Full art's animated and signed levels exactly where Full art is, but on a Legacy card, whose picture never moves,
    // and (R3343000) on the Mythics its share takes too, where it is the Default's.
    expect([...fullArt].some(isLegacy)).toBe(true);
    const moving = [...on(VERSION_FULL_ART_ANIMATED_ID)].sort();
    for (const card of fullArt) expect(moving.includes(card), card).toBe(!isLegacy(card));
    for (const card of moving.filter((each) => !fullArt.has(each))) expect(defaultLookOf(card), card).toBe(VERSION_FULL_ART_ID);
    expect(moving.some((card) => !fullArt.has(card))).toBe(true);
    expect([...on(VERSION_FULL_ART_SIGNED_ID)].sort()).toEqual(moving);
  });

  it("R2940000 R3015000 each new variant is offered on its own varied set of cards, never every card and never Full art's", () => {
    const on = (id: string): ReadonlySet<string> => new Set(COSMETIC_OFFERS[id] ?? []);
    const key = (cards: ReadonlySet<string>): string => [...cards].sort().join(" ");
    const collectible = COLLECTIBLE.length;
    // R3015000: Alt full art is Alt art's second level, so the two share one offer, Alt art's, which took Alt full art's.
    expect(key(on(VERSION_ALT_FULL_ART_ID))).toBe(key(on(VERSION_ALT_ART_ID)));
    const variants = [VERSION_FULL_ART_ID, VERSION_ALT_ART_ID, VERSION_SKETCH_ID];
    // No two of the three on the same cards, and none on every card.
    expect(new Set(variants.map((id) => key(on(id)))).size).toBe(variants.length);
    for (const id of variants) expect(on(id).size, id).toBeLessThan(collectible);
    // Alt art the widest of the three; each lands on cards another of them leaves out.
    expect(on(VERSION_ALT_ART_ID).size).toBeGreaterThan(on(VERSION_SKETCH_ID).size);
    expect(on(VERSION_ALT_ART_ID).size).toBeGreaterThan(on(VERSION_FULL_ART_ID).size);
    for (const id of [VERSION_ALT_ART_ID, VERSION_SKETCH_ID]) {
      for (const other of variants.filter((variant) => variant !== id)) {
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

  it("R2940001 R3015002 each part 2 level's look: its variant's picture at every level, the full bleed from its full-art level up", () => {
    expect([null, VERSION_FULL_ART_ID, VERSION_ALT_ART_ID, VERSION_SKETCH_ID, VERSION_ALT_FULL_ART_ID, VERSION_SKETCH_FULL_ART_ID].map(isFullBleed)).toEqual([
      false,
      true,
      false,
      false,
      true,
      true,
    ]);
    expect([null, VERSION_FULL_ART_ID, VERSION_ALT_ART_ID, VERSION_SKETCH_ID, VERSION_ALT_FULL_ART_ID].map((version) => artPictureOf(version))).toEqual([
      "normal",
      "normal",
      "alt",
      "sketch",
      "alt",
    ]);
    // R3015002: one picture per variant: every level of Full art the normal one, of Alt art Alt art's, of the Sketch the
    // sketch; each full-art level and every level above it the full bleed.
    const pictures = { "full-art": "normal", "alt-art": "alt", sketch: "sketch" } as const;
    for (const [variant, picture] of Object.entries(pictures)) {
      const ladder = ladderOf(variant);
      for (const id of ladder) {
        expect(artPictureOf(id), id).toBe(picture);
        expect(isFullBleed(id), id).toBe(variant === "full-art" || (levelOf(id)?.rank ?? 0) >= 2);
      }
    }
    // R2940002: an animated level moves its picture; R3015000: a signed level only above an animated full art.
    for (const version of [VERSION_ALT_ART_ID, VERSION_SKETCH_ID, VERSION_ALT_FULL_ART_ID]) {
      expect(lookOfLevel(version), version).toEqual({ level: version, version, animated: false, signed: false });
    }
    expect(lookOfLevel(VERSION_ALT_FULL_ART_SIGNED_ID)).toEqual({ level: VERSION_ALT_FULL_ART_SIGNED_ID, version: VERSION_ALT_FULL_ART_ID, animated: true, signed: true });
    expect(lookOfLevel(VERSION_SKETCH_FULL_ART_ANIMATED_ID)).toEqual({
      level: VERSION_SKETCH_FULL_ART_ANIMATED_ID,
      version: VERSION_SKETCH_FULL_ART_ID,
      animated: true,
      signed: false,
    });
  });

  it("R2942000 each part 3 version is offered on its own varied set, never every card, never another version's", () => {
    const on = (id: string): ReadonlySet<string> => new Set(COSMETIC_OFFERS[id] ?? []);
    const key = (cards: ReadonlySet<string>): string => [...cards].sort().join(" ");
    const collectible = COLLECTIBLE.length;
    // R3015000: Alt full art shares Alt art's offer, so the variants are compared by their first levels.
    const versions = [VERSION_FULL_ART_ID, VERSION_ALT_ART_ID, VERSION_SKETCH_ID, VERSION_EXTENDED_ART_ID, VERSION_PIXEL_ID, VERSION_COMIC_ID];
    // No two of the six on the same cards, and none on every card.
    expect(new Set(versions.map((id) => key(on(id)))).size).toBe(versions.length);
    for (const id of versions) expect(on(id).size, id).toBeLessThan(collectible);
    for (const id of [VERSION_EXTENDED_ART_ID, VERSION_PIXEL_ID, VERSION_COMIC_ID]) {
      // Each holds a card no other version takes, and none is on Speciesist, a Common Unit.
      const others = new Set(versions.filter((version) => version !== id).flatMap((version) => [...on(version)]));
      expect([...on(id)].some((card) => !others.has(card)), id).toBe(true);
      expect(offeredOn(id, "core-002"), `${id} on Speciesist`).toBe(false);
      // Each lands on cards of every type its clauses name and on Legendary and Mythic cards of any type, but Extended art
      // on no Legendary, whose Default draws it (R3343000).
      const rarities = new Set([...on(id)].map((card) => CATALOG[card]?.rarity));
      expect(rarities.has("Mythic"), id).toBe(true);
      expect(rarities.has("Legendary"), id).toBe(id !== VERSION_EXTENDED_ART_ID);
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

  it("R2942001 R3015002 artPictureOf and the full bleed for part 3, at both levels", () => {
    const partThree = [VERSION_EXTENDED_ART_ID, VERSION_PIXEL_ID, VERSION_COMIC_ID];
    // Extended art draws the normal picture and is no full art (its words stay); Pixel and Comic restyle it.
    expect(partThree.map(isFullBleed)).toEqual([false, false, false]);
    expect(partThree.map((version) => artPictureOf(version))).toEqual(["normal", "pixel", "comic"]);
    for (const version of partThree) {
      const animated = `${version}-animated`;
      // R2942002, R3015000: its second level moves it, the same picture, and no level signs it.
      expect(artPictureOf(animated), animated).toBe(artPictureOf(version));
      expect(isFullBleed(animated), animated).toBe(false);
      expect(lookOfLevel(animated), version).toEqual({ level: animated, version, animated: true, signed: false });
    }
  });

  it("R2943000 each part 4 version is offered on its own varied set, never every card, never another version's", () => {
    const on = (id: string): ReadonlySet<string> => new Set(COSMETIC_OFFERS[id] ?? []);
    const key = (cards: ReadonlySet<string>): string => [...cards].sort().join(" ");
    const collectible = COLLECTIBLE.length;
    const versions = [
      VERSION_FULL_ART_ID,
      VERSION_ALT_ART_ID,
      VERSION_SKETCH_ID,
      VERSION_EXTENDED_ART_ID,
      VERSION_PIXEL_ID,
      VERSION_COMIC_ID,
      VERSION_SET_SHOWCASE_ID,
      VERSION_OUT_OF_FRAME_ID,
    ];
    // No two of the eight on the same cards (R3015000: Alt full art is Alt art's), and none on every card.
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

  it("R2943001 R3015002 artPictureOf and artPartOf for part 4, at both levels", () => {
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
    for (const set of sets) {
      expect(artPictureOf(VERSION_OUT_OF_FRAME_ID, set), set).toBe("normal");
      // R3015002: the animated level draws the same picture.
      expect(artPictureOf(`${VERSION_SET_SHOWCASE_ID}-animated`, set), set).toBe(artPictureOf(VERSION_SET_SHOWCASE_ID, set));
    }
    expect([null, VERSION_FULL_ART_ID, VERSION_COMIC_ID, VERSION_SET_SHOWCASE_ID, VERSION_OUT_OF_FRAME_ID, `${VERSION_OUT_OF_FRAME_ID}-animated`].map((version) => artPartOf(version))).toEqual([
      "whole",
      "whole",
      "whole",
      "whole",
      "subject",
      "subject",
    ]);
    for (const version of [VERSION_SET_SHOWCASE_ID, VERSION_OUT_OF_FRAME_ID]) {
      const animated = `${version}-animated`;
      // Neither is a full art (its words stay); R2943002, R3015000: its second level moves it, and no level signs it.
      expect(isFullBleed(version), version).toBe(false);
      expect(isFullBleed(animated), animated).toBe(false);
      expect(lookOfLevel(animated), version).toEqual({ level: animated, version, animated: true, signed: false });
    }
  });

  it("R2944000 each part 5 version is offered on its own varied set, never every card, never another version's", () => {
    const on = (id: string): ReadonlySet<string> => new Set(COSMETIC_OFFERS[id] ?? []);
    const key = (cards: ReadonlySet<string>): string => [...cards].sort().join(" ");
    const versions = [
      VERSION_FULL_ART_ID,
      VERSION_ALT_ART_ID,
      VERSION_SKETCH_ID,
      VERSION_EXTENDED_ART_ID,
      VERSION_PIXEL_ID,
      VERSION_COMIC_ID,
      VERSION_SET_SHOWCASE_ID,
      VERSION_OUT_OF_FRAME_ID,
      VERSION_IMMERSIVE_ID,
      VERSION_PRISMATIC_ID,
    ];
    // No two of the ten on the same cards (R3015000: Alt full art is Alt art's), and none on every card.
    expect(new Set(versions.map((id) => key(on(id)))).size).toBe(versions.length);
    for (const id of versions) expect(on(id).size, id).toBeLessThan(COLLECTIBLE.length);
    // Both on Legendary and Mythic cards alone, the few marquee cards of each set: Immersive never on the Starship, whose
    // picture has no subject of its own to lift, and Prismatic, the top chase, on every Mythic card.
    const legendary = (card: string): boolean => CATALOG[card]?.rarity === "Legendary" || CATALOG[card]?.rarity === "Mythic";
    for (const id of [VERSION_IMMERSIVE_ID, VERSION_PRISMATIC_ID]) {
      for (const card of on(id)) expect(legendary(card), `${id} on ${card}`).toBe(true);
      expect(new Set([...on(id)].map((card) => CATALOG[card]?.type)).size, id).toBeGreaterThan(2);
    }
    expect(on(VERSION_IMMERSIVE_ID).has("stinkyscape-t-dormant-starship")).toBe(false);
    for (const def of COLLECTIBLE.filter((each) => each.rarity === "Mythic")) expect(on(VERSION_PRISMATIC_ID).has(def.id), def.id).toBe(true);
    for (const id of [VERSION_IMMERSIVE_ID, VERSION_PRISMATIC_ID]) {
      expect(offeredOn(id, "core-002"), `${id} on Speciesist`).toBe(false);
      expect(offeredOn(id, "core-100"), `${id} on Ceaseless Void`).toBe(true);
    }
  });

  it("R2944001 R3015002 artPictureOf and artPartOf for part 5, at both levels", () => {
    // Immersive draws its depth (its window the backdrop, CardFace; a tile the whole picture), Prismatic its chrome,
    // whatever the card's set.
    for (const set of [null, "Core", "Classic+", "Stinkyscape", "Training Grounds"]) {
      expect(artPictureOf(VERSION_IMMERSIVE_ID, set), `${String(set)}`).toBe("depth");
      expect(artPictureOf(VERSION_PRISMATIC_ID, set), `${String(set)}`).toBe("prismatic");
      expect(artPictureOf(`${VERSION_IMMERSIVE_ID}-animated`, set), `${String(set)}`).toBe("depth");
      expect(artPictureOf(`${VERSION_PRISMATIC_ID}-animated`, set), `${String(set)}`).toBe("prismatic");
    }
    expect([VERSION_IMMERSIVE_ID, VERSION_PRISMATIC_ID].map((version) => artPartOf(version))).toEqual(["whole", "whole"]);
    for (const version of [VERSION_IMMERSIVE_ID, VERSION_PRISMATIC_ID]) {
      const animated = `${version}-animated`;
      // Immersive is no full art (its words stay), and (R3343001) Prismatic is one at both levels; R2944002, R3015000:
      // its second level moves it, and no level signs it.
      expect(isFullBleed(version), version).toBe(version === VERSION_PRISMATIC_ID);
      expect(isFullBleed(animated), animated).toBe(version === VERSION_PRISMATIC_ID);
      expect(lookOfLevel(animated), version).toEqual({ level: animated, version, animated: true, signed: false });
    }
  });

  it("R3017000 R3017002 artPictureOf for part 6, at both levels: four repaints and two restyles, none a full art nor signed", () => {
    const pictures = ["night", "neon", "watercolor", "stained-glass", "woodcut", "tarot"];
    for (const set of [null, "Core", "Classic+", "Stinkyscape", "Training Grounds"]) {
      for (const [at, first] of PART_SIX.entries()) {
        expect(artPictureOf(first, set), `${first} ${String(set)}`).toBe(pictures[at]);
        expect(artPictureOf(`${first}-animated`, set), `${first} ${String(set)}`).toBe(pictures[at]);
      }
    }
    for (const first of PART_SIX) {
      const animated = `${first}-animated`;
      expect(artPartOf(first), first).toBe("whole");
      expect([isFullBleed(first), isFullBleed(animated)], first).toEqual([false, false]);
      expect(lookOfLevel(first), first).toEqual({ level: first, version: first, animated: false, signed: false });
      expect(lookOfLevel(animated), first).toEqual({ level: animated, version: first, animated: true, signed: false });
      expect(levelsBelow(animated), first).toEqual([first]);
    }
  });

  it("R3017001 each pair splits every collectible card and each part 6 variant reaches every shipped set", () => {
    const { VARIANT_MIN_PER_CARD } = aiConstants() as unknown as { VARIANT_MIN_PER_CARD: number };
    expect(VARIANT_MIN_PER_CARD).toBeGreaterThan(0);
    const on = (id: string): ReadonlySet<string> => new Set(COSMETIC_OFFERS[id] ?? []);
    const key = (cards: ReadonlySet<string>): string => [...cards].sort().join(" ");
    // Every collectible card, a Legacy one too, is offered exactly one of each pair's first levels, so at least the floor
    // of variants besides its Default; a Legacy one none of their animated levels.
    expect(COLLECTIBLE.some((def) => isLegacy(def.id))).toBe(true);
    for (const def of COLLECTIBLE) {
      for (const [a, b] of PAIRS) expect(offeredOn(a, def.id) !== offeredOn(b, def.id), `${def.id}: ${a} or ${b}`).toBe(true);
      expect(variantsOn(def.id).length - 1, def.id).toBeGreaterThanOrEqual(VARIANT_MIN_PER_CARD);
      for (const first of PART_SIX) {
        const levels = levelsOn(def.id, levelOf(first)?.variant ?? "");
        expect(levels, `${first} on ${def.id}`).toEqual(!offeredOn(first, def.id) ? [] : isLegacy(def.id) ? [first] : [first, `${first}-animated`]);
      }
    }
    // Each reaches every set that ships, and no two variants but the Default share their cards, nor does any take them all.
    for (const first of PART_SIX) {
      const sets = new Set([...on(first)].map((card) => CATALOG[card]?.set));
      expect([...sets].sort(), first).toEqual([...SHIPPED_SETS].sort());
    }
    const firsts = VARIANTS.filter((variant) => variant !== DEFAULT_VARIANT).map((variant) => ladderOf(variant)[0] ?? "");
    expect(firsts).toHaveLength(16);
    expect(new Set(firsts.map((first) => key(on(first)))).size).toBe(firsts.length);
    for (const first of firsts) expect(on(first).size, first).toBeLessThan(COLLECTIBLE.length);
    // Speciesist and Scratch Pad between them are offered all six.
    expect(PART_SIX.filter((first) => offeredOn(first, "core-002"))).toEqual([VERSION_NEON_ID, VERSION_WATERCOLOR_ID, VERSION_WOODCUT_ID]);
    expect(PART_SIX.filter((first) => offeredOn(first, "meditativeplus-026"))).toEqual([VERSION_NIGHT_ID, VERSION_STAINED_GLASS_ID, VERSION_TAROT_ID]);
  });
});

/** R3343000: the rarity a card's face prints: a token's printed one, any other card's own. */
function faceRarityOf(id: string): string | null {
  const def = CATALOG[id];
  if (def === undefined) return null;
  return def.rarity === "Token" ? (def.printedRarity ?? null) : def.rarity;
}

describe("R3343000 default looks", () => {
  it("R3343000 a Mythic's Default draws Full art and a Legendary's Extended art, a token by its printed rarity, as the server exports it", () => {
    const counts = { full: 0, extended: 0, tokens: 0 };
    for (const id of Object.keys(CATALOG)) {
      const rarity = faceRarityOf(id);
      const expected = rarity === "Mythic" ? VERSION_FULL_ART_ID : rarity === "Legendary" ? VERSION_EXTENDED_ART_ID : null;
      expect(defaultLookOf(id), id).toBe(expected);
      if (expected === VERSION_FULL_ART_ID) counts.full += 1;
      if (expected === VERSION_EXTENDED_ART_ID) counts.extended += 1;
      if (expected !== null && CATALOG[id]?.rarity === "Token") counts.tokens += 1;
    }
    expect(counts.full).toBeGreaterThan(0);
    expect(counts.extended).toBeGreaterThan(0);
    expect(counts.tokens).toBeGreaterThan(0);
    expect(defaultLookOf("meditative-049-1")).toBe(VERSION_FULL_ART_ID);
    expect(defaultLookOf("classicplus-073-1")).toBe(VERSION_EXTENDED_ART_ID);
    expect(defaultLookOf("core-999")).toBeNull();
    // The registry marks the two first levels, and nothing else.
    const marked = Object.entries(COSMETIC_LEVELS).filter(([, level]) => (level.defaultOn ?? []).length > 0);
    expect(marked).toEqual([
      [VERSION_FULL_ART_ID, { variant: "full-art", rank: 1, adds: "fullArt", defaultOn: ["Mythic"] }],
      [VERSION_EXTENDED_ART_ID, { variant: "extended-art", rank: 1, adds: "art", defaultOn: ["Legendary"] }],
    ]);
  });

  it("R3343000 a default look is offered nowhere on its cards, and the level above it is next with nothing owned", () => {
    const nothing = (): boolean => false;
    for (const def of COLLECTIBLE) {
      const shown = defaultLookOf(def.id);
      if (shown === null) continue;
      expect(offeredOn(shown, def.id), `${shown} on ${def.id}`).toBe(false);
      const above = ladderOf(levelOf(shown)?.variant ?? "")[1];
      if (above === undefined) throw new Error(`${shown} has no level above it`);
      expect(isNextLevel(above, def.id, nothing), `${above} on ${def.id}`).toBe(true);
      // Its variant is listed wherever a level above it is offered.
      const variant = levelOf(shown)?.variant ?? "";
      expect(variantsOn(def.id).includes(variant), `${variant} on ${def.id}`).toBe(levelsOn(def.id, variant).length > 0);
      expect(levelsOn(def.id, variant).includes(shown), `${variant} on ${def.id}`).toBe(false);
    }
    // Ceaseless Void, a Mythic: Full art starts at its animated level; its signed one still needs that.
    expect(levelsOn("core-100", "full-art")).toEqual([VERSION_FULL_ART_ANIMATED_ID, VERSION_FULL_ART_SIGNED_ID]);
    expect(isNextLevel(VERSION_FULL_ART_SIGNED_ID, "core-100", nothing)).toBe(false);
    // Silly Silas, a Legendary: Extended art is its animated level alone.
    expect(levelsOn("core-052", "extended-art")).toEqual([`${VERSION_EXTENDED_ART_ID}-animated`]);
    // Below Legendary nothing changes: Full art's animated level still needs its first.
    expect(isNextLevel(VERSION_FULL_ART_ANIMATED_ID, "core-008", nothing)).toBe(false);
    // Full art stays offered on a Legendary its share takes, Extended art on a Mythic.
    expect(offeredOn(VERSION_FULL_ART_ID, "classicplus-012")).toBe(true);
    expect(offeredOn(VERSION_EXTENDED_ART_ID, "meditativeplus-026")).toBe(true);
  });

  it("R3343001 lookOn: the Default draws the default look, still and animated, and the setting keeps a Mythic's normal face when off", () => {
    // Ceaseless Void, a Mythic.
    expect(lookOn("core-100", null, true)).toEqual({ level: null, version: VERSION_FULL_ART_ID, animated: false, signed: false });
    expect(lookOn("core-100", VERSION_ANIMATED_ID, true)).toEqual({ level: VERSION_ANIMATED_ID, version: VERSION_FULL_ART_ID, animated: true, signed: false });
    expect(lookOn("core-100", null, false)).toEqual(NORMAL_LOOK);
    expect(lookOn("core-100", VERSION_ANIMATED_ID, false)).toEqual(lookOfLevel(VERSION_ANIMATED_ID));
    // A level of its own wins: Prismatic is Prismatic.
    expect(lookOn("core-100", VERSION_PRISMATIC_ID, true)).toEqual(lookOfLevel(VERSION_PRISMATIC_ID));
    // Silly Silas, a Legendary, keeps its Extended art whatever the setting.
    for (const fullArtCards of [true, false]) {
      expect(lookOn("core-052", null, fullArtCards)).toEqual({ level: null, version: VERSION_EXTENDED_ART_ID, animated: false, signed: false });
      expect(lookOn("core-052", VERSION_ANIMATED_ID, fullArtCards)).toEqual({ level: VERSION_ANIMATED_ID, version: VERSION_EXTENDED_ART_ID, animated: true, signed: false });
    }
    // Below Legendary the Default is the normal face.
    for (const def of COLLECTIBLE) {
      if (defaultLookOf(def.id) !== null) continue;
      expect(lookOn(def.id, null, true), def.id).toEqual(NORMAL_LOOK);
      expect(lookOn(def.id, VERSION_ANIMATED_ID, true), def.id).toEqual(lookOfLevel(VERSION_ANIMATED_ID));
    }
  });

  it("R3343001 Prismatic is full bleed at both levels", () => {
    const animated = `${VERSION_PRISMATIC_ID}-animated`;
    expect(levelOf(VERSION_PRISMATIC_ID)).toEqual({ variant: "prismatic", rank: 1, adds: "fullArt" });
    expect(isFullBleed(VERSION_PRISMATIC_ID)).toBe(true);
    expect(isFullBleed(animated)).toBe(true);
    expect(artPictureOf(VERSION_PRISMATIC_ID)).toBe("prismatic");
    expect(artPictureOf(animated)).toBe("prismatic");
    expect(lookOfLevel(animated)).toEqual({ level: animated, version: VERSION_PRISMATIC_ID, animated: true, signed: false });
  });
});
