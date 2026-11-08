// R3332000: the ratchet that ends the shared motifs on the Legendaries and Mythics (#3330). SIGNATURE_PENDING
// lists every Legendary and Mythic, a token by the rarity it prints (B2.5), whose effects table entry
// (fx/cards/<set>.json5) is not yet its signature (`signature: true`, tables.ts), in catalog order. A
// set's signature pass takes each card off as it gives it one; the list only shrinks, and the last pass
// empties it. A new Legendary or Mythic comes with its signature, or joins the list where the catalog puts it.
// R3389001: Classic's pass (#3389) took its eleven off.

import { describe, expect, it } from "vitest";

import { CATALOG } from "../../wire/cards.ts";
import { FX_CARD_TABLE, isSignature } from "./tables.ts";

/** Every Legendary and Mythic still owed a signature, in catalog order, by set. */
const SIGNATURE_PENDING: readonly string[] = [
  // Core
  "core-052", "core-083", "core-085", "core-087", "core-093", "core-095", "core-096", "core-097", "core-098", "core-099", "core-100",
  // Classic+
  "classicplus-012", "classicplus-012-1", "classicplus-012-2", "classicplus-012-3", "classicplus-012-4", "classicplus-012-5", "classicplus-012-6",
  "classicplus-012-7", "classicplus-012-8", "classicplus-013", "classicplus-019", "classicplus-019-1", "classicplus-019-2", "classicplus-019-3",
  "classicplus-019-4", "classicplus-019-5", "classicplus-027", "classicplus-029", "classicplus-035", "classicplus-037", "classicplus-042",
  "classicplus-042-1", "classicplus-043", "classicplus-046", "classicplus-046-1", "classicplus-047", "classicplus-048", "classicplus-065-4",
  "classicplus-065-5", "classicplus-073", "classicplus-073-1", "classicplus-074", "classicplus-075", "classicplus-075-1", "classicplus-078",
  // Meditative
  "meditative-007", "meditative-008", "meditative-011", "meditative-017", "meditative-020", "meditative-026", "meditative-039-4", "meditative-039-5",
  "meditative-040", "meditative-042", "meditative-045", "meditative-045-1", "meditative-047", "meditative-049", "meditative-049-1",
  "meditative-049-2", "meditative-049-3", "meditative-051", "meditative-063", "meditative-065", "meditative-071", "meditative-071-1",
  "meditative-075", "meditative-079", "meditative-080", "meditative-086", "meditative-087", "meditative-095", "meditative-095-1", "meditative-097",
  "meditative-097-8", "meditative-097-9",
  // Stinkyscape
  "stinkyscape-004", "stinkyscape-004-1", "stinkyscape-004-2", "stinkyscape-004-3", "stinkyscape-004-4", "stinkyscape-004-5", "stinkyscape-004-6",
  "stinkyscape-006", "stinkyscape-007", "stinkyscape-016", "stinkyscape-018", "stinkyscape-018-1", "stinkyscape-018-2", "stinkyscape-018-3",
  "stinkyscape-026", "stinkyscape-034", "stinkyscape-035", "stinkyscape-036", "stinkyscape-037", "stinkyscape-039", "stinkyscape-044",
  "stinkyscape-058", "stinkyscape-059", "stinkyscape-060", "stinkyscape-061", "stinkyscape-066", "stinkyscape-070", "stinkyscape-071",
  "stinkyscape-073", "stinkyscape-074", "stinkyscape-080", "stinkyscape-085", "stinkyscape-086", "stinkyscape-087", "stinkyscape-088",
  "stinkyscape-109", "stinkyscape-110", "stinkyscape-111", "stinkyscape-112", "stinkyscape-112-1", "stinkyscape-115", "stinkyscape-116",
  "stinkyscape-137", "stinkyscape-137-1", "stinkyscape-137-2", "stinkyscape-137-3", "stinkyscape-137-4", "stinkyscape-147", "stinkyscape-147-1",
  "stinkyscape-147-2", "stinkyscape-148", "stinkyscape-148-1", "stinkyscape-149", "stinkyscape-150", "stinkyscape-150-1", "stinkyscape-151",
  "stinkyscape-152", "stinkyscape-153", "stinkyscape-154", "stinkyscape-176", "stinkyscape-t-dormant-starship", "stinkyscape-t-shield-layer",
  "stinkyscape-t-propagation-layer", "stinkyscape-t-meltdown-layer", "stinkyscape-t-narcor", "stinkyscape-t-red-narcor",
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
    // Tokens printed Legendary or Mythic are on it too (Classic+'s Losers among them).
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
