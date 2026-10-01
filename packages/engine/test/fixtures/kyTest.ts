// Test-only cards for Classic+ #42 KY's Test's question bank (`subsystems/kyTest.ts`, R420, R580): a
// fixture KY's Test running `kyTestScript` over a fixture bank, and one card for each reward pool, so
// the engine is proved without `packages/cards` (CLAUDE.md). The named rewards are registered under
// the real ids `config.KY_TEST_REWARDS` names (The Coin, KY's Gift), as stand-ins.

import type { CardDef, CardType, Rarity, Tag } from "@jackioh/shared";
import { registerCatalog, registeredCatalog } from "../../src/catalog";
import type { CardScripts } from "../../src/script";
import { registerScripts, registeredScripts } from "../../src/scripts";
import { kyTestScript, type KyTestProblem } from "../../src/subsystems/kyTest";

let nextIndex = 3300;
function def(id: string, type: CardType, extra: Partial<CardDef> = {}): CardDef {
  nextIndex += 1;
  return {
    id,
    index: String(nextIndex),
    name: id,
    set: "Core",
    type,
    tags: [],
    rarity: "Common",
    token: false,
    cost: 1,
    base: { keywords: [], text: id },
    radiant: { keywords: [], text: id },
    ...extra,
  };
}

function pooled(id: string, tags: Tag[], cost: number, rarity: Rarity = "Common"): CardDef {
  return def(id, "Spell", { tags, cost, rarity });
}

/** Two Medium problems and one Hard one: enough to tell a bank draw from a fixed pick. */
export const FIXTURE_BANK: readonly KyTestProblem[] = [
  { id: "m1", difficulty: "Medium", statement: "∫₀¹ 2x dx = ?", options: ["1", "2", "1/2", "0"], answer: "1" },
  { id: "m2", difficulty: "Medium", statement: "∫₀¹∫₀¹ 1 dy dx = ?", options: ["1", "2", "0", "1/2"], answer: "1" },
  { id: "h1", difficulty: "Hard", statement: "det [[2, 0], [0, 3]] = ?", options: ["6", "5", "1", "0"], answer: "6" },
];

export const kyTest = def("kt-test", "Spell", { tags: ["KY"], rarity: "Legendary" });
export const coin = def("core-t-coin", "Spell", { tags: ["Token"], rarity: "Token", token: true, cost: 0 });
export const gift = def("classicplus-042-1", "Field Spell", { tags: ["KY", "Token"], rarity: "Token", token: true, cost: 4 });
export const book = pooled("kt-book", ["Book"], 1);
export const kyTwo = pooled("kt-ky-two", ["KY"], 2);
export const legend = pooled("kt-legend", [], 3, "Legendary");
export const four = pooled("kt-four", [], 4);
/** The fixture KY's Test with Quickdraw (§6.2), so a replayable game's opening hand holds it; in no reward pool. */
export const kyTestQd = def("kt-test-qd", "Spell");

const DEFS = [kyTest, kyTestQd, coin, gift, book, kyTwo, legend, four];
const script = kyTestScript(FIXTURE_BANK);
const quick = { ...script, staticFlags: { quickdraw: true } };
const SCRIPTS: Record<string, CardScripts> = {
  [kyTest.id]: { base: script, radiant: script },
  [kyTestQd.id]: { base: quick, radiant: quick },
};

export function registerKyTestFixtures(): void {
  registerCatalog({ ...registeredCatalog(), ...Object.fromEntries(DEFS.map((card) => [card.id, card])) });
  registerScripts({ ...registeredScripts(), ...SCRIPTS });
}
