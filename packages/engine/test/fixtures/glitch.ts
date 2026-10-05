// Fixture cards for the Glitch Easter egg (SPEC §7, R661–R664). The engine never imports
// `packages/cards` (CLAUDE.md), so the two System cards and Glitch are written here under their real
// ids — the ids `SYSTEM_CARD_DEF_IDS` and `GLITCH_DEF_ID` name — with the smallest scripts that have
// their shapes, beside the generators that make cards from a pool. The real cards' tests cover the
// same cases again.
//
// Indexes are 4901 upward and the generators' ids `glitchfx-*`, so nothing collides with another
// fixture file's.

import type { CardDef, CardFace, CardType } from "@jackioh/shared";
import { registerCatalog, registeredCatalog } from "../../src/catalog";
import { GLITCH_DEF_ID, SYSTEM_CARD_DEF_IDS } from "../../src/config";
import { addRandomFromCatalog, glitchOutcome, shuffleRandomFromCatalog, summonRandom, transformRandom } from "../../src/effects";
import type { CardScripts, Script } from "../../src/script";
import { registerScripts, registeredScripts } from "../../src/scripts";

let nextIndex = 4900;

function face(text: string, extra: Partial<CardFace> = {}): CardFace {
  return { keywords: [], text, ...extra };
}

function def(id: string, type: CardType, extra: Partial<CardDef> = {}): CardDef {
  nextIndex += 1;
  return {
    id,
    index: String(nextIndex),
    name: `Glitch fixture ${id}`,
    set: "Classic",
    type,
    tags: [],
    rarity: "Common",
    token: false,
    cost: 0,
    base: face(`${id} base`),
    radiant: face(`${id} radiant`),
    ...extra,
  };
}

function both(script: Script): CardScripts {
  return { base: script, radiant: script };
}

const [SYSTEM_A = "", SYSTEM_B = ""] = SYSTEM_CARD_DEF_IDS;

/** C #18 and C #25 stand-ins: a System card is counted by its id alone, so neither needs a text. */
export const SYSTEM_18 = SYSTEM_A;
export const SYSTEM_25 = SYSTEM_B;

/** Adds `GLITCHFX_ADDED` random Units to its caster's hand. */
export const ADDER = "glitchfx-adder";
export const GLITCHFX_ADDED = 3;
/** Summons a random Unit for its caster. */
export const SUMMONER = "glitchfx-summoner";
/** Shuffles a random Unit into its caster's deck. */
export const SHUFFLER = "glitchfx-shuffler";
/** Transforms its caster's first hand card into a random Unit. */
export const TRANSFORMER = "glitchfx-transformer";
/** A plain (0) Spell that does nothing, to set beside Glitch under a price rule or a ban. */
export const PLAIN = "glitchfx-plain";
/** A Unit whose aura makes the opponent's cards cost (3) more and bans every one of their plays. */
export const WARDEN = "glitchfx-warden";
/** The surcharge the warden lays on the opponent's cards. */
export const WARDEN_SURCHARGE = 3;

const DEFS: CardDef[] = [
  def(SYSTEM_18, "Spell", { cost: 1 }),
  def(SYSTEM_25, "Spell"),
  def(GLITCH_DEF_ID, "Spell", { index: "T-Glitch", tags: ["Token"], rarity: "Token", token: true, hidden: true }),
  def(ADDER, "Spell"),
  def(SUMMONER, "Spell"),
  def(SHUFFLER, "Spell"),
  def(TRANSFORMER, "Spell"),
  def(PLAIN, "Spell"),
  def(WARDEN, "Unit", { base: face("warden", { attack: 1, health: 5 }), radiant: face("warden", { attack: 2, health: 10 }) }),
];

const UNITS = { query: { type: "Unit" as const } };

const SCRIPTS: Record<string, CardScripts> = {
  [SYSTEM_18]: both({}),
  [SYSTEM_25]: both({}),
  [GLITCH_DEF_ID]: both({ staticFlags: { alwaysPlayable: true }, cry: () => [glitchOutcome()] }),
  [ADDER]: both({ cry: () => [addRandomFromCatalog({ ...UNITS, count: GLITCHFX_ADDED })] }),
  [SUMMONER]: both({ cry: () => [summonRandom(UNITS)] }),
  [SHUFFLER]: both({ cry: () => [shuffleRandomFromCatalog({ ...UNITS, count: 1 })] }),
  [TRANSFORMER]: both({
    cry: (ctx) => {
      const first = ctx.state.players[ctx.controller].hand[0];
      return first === undefined ? [] : [transformRandom({ instanceId: first.id, ...UNITS })];
    },
  }),
  [PLAIN]: both({}),
  [WARDEN]: both({
    costAura: () => [
      { whose: "opponents", amount: WARDEN_SURCHARGE },
      { whose: "opponents", ban: true },
    ],
  }),
};

/** Register this file's fixtures on top of whatever is registered. */
export function registerGlitch(): void {
  registerCatalog({ ...registeredCatalog(), ...Object.fromEntries(DEFS.map((entry) => [entry.id, entry])) });
  registerScripts({ ...registeredScripts(), ...SCRIPTS });
}
