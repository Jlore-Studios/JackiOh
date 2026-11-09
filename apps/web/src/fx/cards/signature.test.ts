// R3332000: the ratchet that ends the shared motifs on the Legendaries and Mythics (#3330). SIGNATURE_PENDING
// lists every Legendary and Mythic, a token by the rarity it prints (B2.5), whose effects table entry
// (fx/cards/<set>.json5) is not yet its signature (`signature: true`, tables.ts), in catalog order. A
// set's signature pass takes each card off as it gives it one; the list only shrinks, and the last pass
// empties it. A new Legendary or Mythic comes with its signature, or joins the list where the catalog puts it.
// R3389001: Classic's pass (#3389) took its eleven off. R3399000: Classic+'s pass (#3399) took its thirty-five off.
// R3402001: Core's pass (#3402) took its ten off, leaving Ceaseless Void (#3338).

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
