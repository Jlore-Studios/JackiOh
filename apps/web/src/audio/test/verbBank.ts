// #5190: the spell verbs' effects in card-audio.json5 (its head comment holds the table), as data for
// the tests. The card parts (#5187) name these effects from their cards; voice-lines.test.ts keeps its own
// UNUSED_VERB_EFFECTS list of those not yet named, checked against this table. Test support only: the client
// reads the bank.

import type { SfxElement } from "../types.ts";

export type VerbRow = { windUp: string; release: string; tail: string };

export const DAMAGE_ELEMENTS: readonly SfxElement[] = [
  "fire", "frost", "lightning", "arcane", "shadow", "poison", "holy", "physical",
];

/** The verbs other than damage, which has a row per element. */
export const SPELL_VERBS = [
  "destroy", "heal", "draw", "discard", "summon", "transform", "bounce", "steal", "buff", "debuff",
  "lock", "armor", "mana", "shuffle", "counter", "reveal",
] as const;

const OTHER_ROWS: Record<(typeof SPELL_VERBS)[number], VerbRow> = {
  destroy: { windUp: "dreadWind", release: "shatterRelease", tail: "hushTail" },
  heal: { windUp: "calmWind", release: "healRelease", tail: "sparkleTail" },
  draw: { windUp: "calmWind", release: "drawRelease", tail: "sparkleTail" },
  discard: { windUp: "dreadWind", release: "discardRelease", tail: "hushTail" },
  summon: { windUp: "surgeWind", release: "summonRelease", tail: "sparkleTail" },
  transform: { windUp: "twistWind", release: "morphRelease", tail: "ringTail" },
  bounce: { windUp: "twistWind", release: "bounceRelease", tail: "ringTail" },
  steal: { windUp: "dreadWind", release: "stealRelease", tail: "hushTail" },
  buff: { windUp: "surgeWind", release: "buffRelease", tail: "sparkleTail" },
  debuff: { windUp: "dreadWind", release: "debuffRelease", tail: "hushTail" },
  lock: { windUp: "snapWind", release: "lockRelease", tail: "ringTail" },
  armor: { windUp: "surgeWind", release: "armorRelease", tail: "ringTail" },
  mana: { windUp: "calmWind", release: "manaRelease", tail: "sparkleTail" },
  shuffle: { windUp: "twistWind", release: "shuffleRelease", tail: "ringTail" },
  counter: { windUp: "snapWind", release: "counterRelease", tail: "ringTail" },
  reveal: { windUp: "calmWind", release: "revealRelease", tail: "sparkleTail" },
};

/** Every row by key: `damage.<element>` and each verb of SPELL_VERBS. */
export const VERB_ROWS: Record<string, VerbRow> = {
  ...Object.fromEntries(
    DAMAGE_ELEMENTS.map((e): [string, VerbRow] => [
      `damage.${e}`,
      { windUp: `${e}Wind`, release: `${e}Release`, tail: `${e}Tail` },
    ]),
  ),
  ...OTHER_ROWS,
};

/** The name of every bank effect a row uses. */
export const VERB_BANK_EFFECTS: ReadonlySet<string> = new Set(
  Object.values(VERB_ROWS).flatMap((row) => [row.windUp, row.release, row.tail]),
);
