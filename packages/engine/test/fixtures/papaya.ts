// Test-only cards for Classic+ #62 KY's Papaya's curve targeting (SPEC §8.7 row 62, R422; B5 E32).
// The fixture Spell runs the engine half of the card exactly as the real script does — `papayaBegin`
// on resolve, `papayaAnswered` on each answer — so the subsystem is proved without `packages/cards`
// (CLAUDE.md: the engine never imports it); the card's own test repeats the key cases.

import type { CardDef } from "@jackioh/shared";
import { registerCatalog, registeredCatalog } from "../../src/catalog";
import type { CardScripts, Script } from "../../src/script";
import { registerScripts, registeredScripts } from "../../src/scripts";
import { PAPAYA_STEP, papayaAnswered, papayaBegin } from "../../src/subsystems/papaya";

function def(id: string, type: CardDef["type"], extra: Partial<CardDef> = {}): CardDef {
  return {
    id: `pp-${id}`,
    index: `pp-${id}`,
    name: `${id} (papaya)`,
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

/** The curve Spell: both faces run one script, the running face deciding the rows (R422). */
export const curve = def("curve", "Spell");
/** A plain 2/2 to stand on cells. */
export const body = def("body", "Unit", {
  base: { attack: 2, health: 2, keywords: [], text: "2/2" },
  radiant: { attack: 4, health: 4, keywords: [], text: "4/4" },
});
/** A Trap with no text, set face-down. */
export const snare = def("snare", "Trap");
/** A Field Spell, face-up in the backrow. */
export const field = def("field", "Field Spell");
/** A unit token, which ceases to exist rather than reach the exile pile (R11). */
export const token = def("token", "Unit", {
  tags: ["Token"],
  rarity: "Token",
  token: true,
  base: { attack: 1, health: 1, keywords: [], text: "1/1" },
  radiant: { attack: 2, health: 2, keywords: [], text: "2/2" },
});
/** The curve with Quickdraw, for a replayable game whose opening hand holds it (R225). */
export const curveQuickdraw = def("curve-qd", "Spell");

const curveScript: Script = {
  cry: () => papayaBegin(),
  resume: { [PAPAYA_STEP]: papayaAnswered },
};

function both(script: Script): CardScripts {
  return { base: script, radiant: script };
}

export const PAPAYA_DEFS: CardDef[] = [curve, body, snare, field, token, curveQuickdraw];

const PAPAYA_SCRIPTS: Record<string, CardScripts> = {
  [curve.id]: both(curveScript),
  [curveQuickdraw.id]: both({ ...curveScript, staticFlags: { quickdraw: true } }),
};

export function registerPapayaFixtures(): void {
  registerCatalog({ ...registeredCatalog(), ...Object.fromEntries(PAPAYA_DEFS.map((card) => [card.id, card])) });
  registerScripts({ ...registeredScripts(), ...PAPAYA_SCRIPTS });
}
