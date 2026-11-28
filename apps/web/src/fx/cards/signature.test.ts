// R3332000: the ratchet that ends the shared motifs on the Legendaries and Mythics (#3330). SIGNATURE_PENDING
// lists every Legendary and Mythic, a token by the rarity it prints (B2.5), whose effects table entry
// (fx/cards/<set>.json5) is not yet its signature (`signature: true`, tables.ts), in catalog order. A
// set's signature pass takes each card off as it gives it one; the list only shrinks, and the last pass
// empties it. A new Legendary or Mythic comes with its signature, or joins the list where the catalog puts it.
// R3389001: Classic's pass (#3389) took its eleven off. R3399000: Classic+'s pass (#3399) took its thirty-five off.
// R3402001: Core's pass (#3402) took its ten off, leaving Ceaseless Void (#3338). Stinkyscape+'s fifty-nine
// joined it at the set's release (#3596), owed their set's signature pass. The Infinite Assault's two joined
// it with the set's first part (#4577), owed theirs. The foundation (#4602) added the set's other
// forty-two, owed theirs too. R4596000: the set's look and sound (#4596) took its twenty Commanders and Mythics off.
// R5264500: the held Felinors vs Humans set's sixty-three joined it with its effects table (#5264), owed theirs.

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
  "stinkyscapeplus-003", "stinkyscapeplus-004", "stinkyscapeplus-004-1", "stinkyscapeplus-005", "stinkyscapeplus-007",
  "stinkyscapeplus-010", "stinkyscapeplus-033", "stinkyscapeplus-045", "stinkyscapeplus-052", "stinkyscapeplus-056", "stinkyscapeplus-064",
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
  // Felinors vs Humans (R5264500: its look's first part, owed the set's signature pass)
  "felinorsvshumans-002", "felinorsvshumans-003", "felinorsvshumans-004", "felinorsvshumans-016",
  "felinorsvshumans-016-1", "felinorsvshumans-016-2", "felinorsvshumans-031", "felinorsvshumans-034",
  "felinorsvshumans-037", "felinorsvshumans-039", "felinorsvshumans-040", "felinorsvshumans-046",
  "felinorsvshumans-050", "felinorsvshumans-060", "felinorsvshumans-060-1", "felinorsvshumans-060-2",
  "felinorsvshumans-070", "felinorsvshumans-073", "felinorsvshumans-074", "felinorsvshumans-074-1",
  "felinorsvshumans-075", "felinorsvshumans-078", "felinorsvshumans-078-1", "felinorsvshumans-079",
  "felinorsvshumans-082", "felinorsvshumans-091", "felinorsvshumans-092", "felinorsvshumans-098",
  "felinorsvshumans-098-1", "felinorsvshumans-098-2", "felinorsvshumans-099", "felinorsvshumans-100",
  "felinorsvshumans-100-1", "felinorsvshumans-100-2", "felinorsvshumans-107", "felinorsvshumans-109",
  "felinorsvshumans-110", "felinorsvshumans-112", "felinorsvshumans-112-1", "felinorsvshumans-112-2",
  "felinorsvshumans-112-3", "felinorsvshumans-112-4", "felinorsvshumans-112-5", "felinorsvshumans-112-6",
  "felinorsvshumans-112-7", "felinorsvshumans-112-8", "felinorsvshumans-112-9", "felinorsvshumans-112-10",
  "felinorsvshumans-112-11", "felinorsvshumans-112-12", "felinorsvshumans-112-13", "felinorsvshumans-114",
  "felinorsvshumans-115", "felinorsvshumans-147", "felinorsvshumans-167", "felinorsvshumans-173",
  "felinorsvshumans-185", "felinorsvshumans-188", "felinorsvshumans-189", "felinorsvshumans-191",
  "felinorsvshumans-192", "felinorsvshumans-194", "felinorsvshumans-196",
  // Infinite Assault
  "infiniteassault-023", "infiniteassault-024", "infiniteassault-046", "infiniteassault-047", "infiniteassault-070", "infiniteassault-071",
  "infiniteassault-094", "infiniteassault-095", "infiniteassault-118", "infiniteassault-119", "infiniteassault-142", "infiniteassault-143",
  "infiniteassault-166", "infiniteassault-167", "infiniteassault-190", "infiniteassault-191", "infiniteassault-214", "infiniteassault-215",
  "infiniteassault-230", "infiniteassault-232", "infiniteassault-234", "infiniteassault-236", "infiniteassault-238", "infiniteassault-240",
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
