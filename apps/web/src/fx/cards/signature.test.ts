// R3332000: the ratchet that ends the shared motifs on the Legendaries and Mythics (#3330). SIGNATURE_PENDING
// lists every Legendary and Mythic, a token by the rarity it prints (B2.5), whose effects table entry
// (fx/cards/<set>.json5) is not yet its signature (`signature: true`, tables.ts), in catalog order. A
// set's signature pass takes each card off as it gives it one; the list only shrinks, and the last pass
// empties it. A new Legendary or Mythic comes with its signature, or joins the list where the catalog puts it.
// R3389001: Classic's pass (#3389) took its eleven off. R3399000: Classic+'s pass (#3399) took its thirty-five off.
// R3402001: Core's pass (#3402) took its ten off, leaving Ceaseless Void (#3338). Stinkyscape+'s fifty-nine
// joined it at the set's release (#3596), owed their set's signature pass.

import { describe, expect, it } from "vitest";

import { CATALOG } from "../../wire/cards.ts";
import { FX_CARD_TABLE, isSignature } from "./tables.ts";

/** Every Legendary and Mythic still owed a signature, in catalog order, by set. */
const SIGNATURE_PENDING: readonly string[] = [
  // Core
  "core-100",
  // Stinkyscape
  "stinkyscape-004", "stinkyscape-004-1", "stinkyscape-004-2", "stinkyscape-004-3", "stinkyscape-004-4", "stinkyscape-004-5", "stinkyscape-004-6",
  "stinkyscape-036",
  "stinkyscape-137-1", "stinkyscape-137-2", "stinkyscape-137-3", "stinkyscape-137-4",
  "stinkyscape-154", "stinkyscape-t-shield-layer",
  "stinkyscape-t-propagation-layer", "stinkyscape-t-meltdown-layer",
  "stinkyscape-t-anomaly-found",
  // Stinkyscape+
  "stinkyscapeplus-003", "stinkyscapeplus-004", "stinkyscapeplus-005", "stinkyscapeplus-007", "stinkyscapeplus-010",
  "stinkyscapeplus-033", "stinkyscapeplus-045", "stinkyscapeplus-052", "stinkyscapeplus-056", "stinkyscapeplus-064",
  "stinkyscapeplus-072", "stinkyscapeplus-075", "stinkyscapeplus-075-1", "stinkyscapeplus-077", "stinkyscapeplus-078",
  "stinkyscapeplus-079", "stinkyscapeplus-080", "stinkyscapeplus-081", "stinkyscapeplus-082", "stinkyscapeplus-084",
  "stinkyscapeplus-089", "stinkyscapeplus-090", "stinkyscapeplus-095", "stinkyscapeplus-098", "stinkyscapeplus-106",
  "stinkyscapeplus-114", "stinkyscapeplus-119", "stinkyscapeplus-121", "stinkyscapeplus-121-1", "stinkyscapeplus-123",
  "stinkyscapeplus-125", "stinkyscapeplus-127", "stinkyscapeplus-131", "stinkyscapeplus-132", "stinkyscapeplus-133",
  "stinkyscapeplus-133-5", "stinkyscapeplus-139", "stinkyscapeplus-142", "stinkyscapeplus-142-1", "stinkyscapeplus-142-2",
  "stinkyscapeplus-145", "stinkyscapeplus-148", "stinkyscapeplus-155", "stinkyscapeplus-157", "stinkyscapeplus-160",
  "stinkyscapeplus-t-tears-of-the-kwzki-tree", "stinkyscapeplus-t-supernova", "stinkyscapeplus-t-alnilam",
  "stinkyscapeplus-t-the-enigmata",
  "stinkyscapeplus-t-terminus-starship-ark-of-ashes", "stinkyscapeplus-t-terminus-starship-dying-ember",
  "stinkyscapeplus-t-terminus-starship-heat-death-pilgrim", "stinkyscapeplus-t-terminus-starship-entropys-wake",
  "stinkyscapeplus-t-terminus-starship-red-giant-requiem", "stinkyscapeplus-t-terminus-starship-cold-exodus",
  "stinkyscapeplus-t-terminus-starship-omega-drift", "stinkyscapeplus-t-terminus-starship-farthest-shore",
  "stinkyscapeplus-t-terminus-starship-nightfall-covenant", "stinkyscapeplus-t-terminus-starship-vacuum-psalm",
];

/** Every Legendary and Mythic of the catalog, tokens by the rarity they print, in catalog order. */
function legendsAndMythics(): string[] {
  return Object.keys(CATALOG).filter((id) => {
    const def = CATALOG[id];
    const rarity = def?.printedRarity ?? def?.rarity;
    return rarity === "Legendary" || rarity === "Mythic";
  });
}

describe("R3332000 the signature ratchet", () => {
  it("R3332000 SIGNATURE_PENDING lists every Legendary and Mythic, tokens by printed rarity, without a signature, in catalog order, and none with one", () => {
    const all = legendsAndMythics();
    const owed = all.filter((id) => !isSignature(FX_CARD_TABLE.get(id)));
    const pending = new Set(SIGNATURE_PENDING);
    expect(
      owed.filter((id) => !pending.has(id)),
      "a Legendary or Mythic with no signature: give it one, or list it in SIGNATURE_PENDING",
    ).toEqual([]);
    expect(
      SIGNATURE_PENDING.filter((id) => isSignature(FX_CARD_TABLE.get(id))),
      "a listed card has its signature now: take it off SIGNATURE_PENDING",
    ).toEqual([]);
    expect(
      SIGNATURE_PENDING.filter((id) => !all.includes(id)),
      "a listed card is no Legendary or Mythic of the catalog",
    ).toEqual([]);
    expect(SIGNATURE_PENDING).toEqual(owed);
    // Tokens printed Legendary or Mythic are on it too (Stinkyscape's Drone Battleship layers among them).
    expect(SIGNATURE_PENDING.some((id) => CATALOG[id]?.token === true)).toBe(true);
    // Felinor Fiender's is the first signature (R3332001).
    expect(isSignature(FX_CARD_TABLE.get("core-092"))).toBe(true);
    expect(SIGNATURE_PENDING).not.toContain("core-092");
  });

  it("R3389001 Classic's eleven Legendaries and Mythics are all signatures, so none of its cards is pending", () => {
    const classic = legendsAndMythics().filter((id) => CATALOG[id]?.set === "Classic");
    expect(classic).toEqual(["classic-004", "classic-007", "classic-009", "classic-028", "classic-033", "classic-044", "classic-045", "classic-056", "classic-080", "classic-085", "classic-090"]);
    for (const id of classic) expect(isSignature(FX_CARD_TABLE.get(id)), id).toBe(true);
    expect(SIGNATURE_PENDING.filter((id) => id.startsWith("classic-"))).toEqual([]);
  });

});
