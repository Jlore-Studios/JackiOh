// What a face in play prints where the card in play and the card as printed part ways (SPEC §10.10).
//
// A card in the collection is its catalog definition, both faces printed in full. A card in a game
// is the card as the view says it stands (R243), and three cards print something else there:
//
// - #98 Heroic Power rolled one of thirteen powers as it arrived (R43, R151, R352, patch v0.2.1). In
//   play its text is that power alone, read off the view's `power` (a hand card's `CardView.power`, a
//   backrow card's `HeroPowerView`), as the catalog prints it — "Activate: Spend (X): <Title>:
//   <clause>." — with its X; the collection keeps the list of thirteen.
// - A card with the Call to Chaos tag (Core #95, Classic+ #73) reads "???" in play. What it does is
//   rolled when it resolves (§8 #95), and the game keeps it a mystery; the collection prints the real
//   text, so a player building a deck can still read it. The Classic+ Edition's Radiant face reads
//   "!!!", as its designer wrote it (docs/classic-sets.md B7 #73), one mystery for three effects.
// - A unit a Vanilla took the text of (§6.3, R115) prints that its text is gone: the definition the
//   client reads still names the keywords and scripts it no longer has, and the view says so
//   (`UnitView.vanilla`, R243).
//
// Presentation only (CLAUDE.md rule 7): every word here is the card's §8 text or says what the view
// already says. `POWER_WORDS` is keyed by the power's stored name as the view carries it (R103) and
// written from §8 #98's titles and clauses, base and radiant, the way `game/modeText.ts` words a
// "Choose one" option; `inPlay.test.ts` holds it to the engine's own table so the two cannot drift
// apart. A clause may write a declared number (Steady Shot's `{shot}`, R666), which
// `fillPowerParams` fills from the view as every face's text is filled (B3.4 rule 5, R386). A power's
// X is the view's (`HeroPowerView.x`) on the field; a hand card's view names only the power, so its X
// is the engine's own constant for that power (`HERO_POWER_COST`, CLAUDE.md rule 9).

import { HERO_POWER_COST } from "@jackioh/engine/config";
import { fillParams, type Param, type Tag } from "@jackioh/shared";

/** §8 #98, the one card whose text in play is the power it rolled. */
export const HEROIC_POWER_ID = "core-098";

/** §5: the tag whose cards read {@link CONCEALED_TEXT} in play (#95 Call to Chaos). */
export const CONCEALED_TAG: Tag = "Call to Chaos";

/** What a concealed card's rules box reads in play. */
export const CONCEALED_TEXT = "???";

/** Classic+ #73's Radiant face in play (the designer's "!!!", three effects at once). */
export const CONCEALED_TEXT_LOUD = "!!!";

/** The concealed faces that read something other than {@link CONCEALED_TEXT}, by id and face. */
const CONCEALED_OVERRIDES: Readonly<Record<string, { radiant?: string }>> = {
  "classicplus-073": { radiant: CONCEALED_TEXT_LOUD },
};

/** What a concealed card's face reads in play: "???", or its own override. */
export function concealedText(defId: string, radiant: boolean): string {
  const override = CONCEALED_OVERRIDES[defId];
  return (radiant ? override?.radiant : undefined) ?? CONCEALED_TEXT;
}

/** What a Vanilla unit's rules box reads (§6.3 Vanilla: "remove a unit's text"). */
export const VANILLA_TEXT = "Vanilla: its text is gone";

/** R43, R243: the power a #98 Heroic Power rolled, by the stored name the view gives it, and its X. */
export type RolledPower = { name: string; x: number };

/** One power as §8 #98 prints it: its name on each face (Armor Up's Radiant face is Tank Up) and its clause. */
export type PowerWords = { title: string; radiantTitle: string; base: string; radiant: string };

/**
 * §8 #98's thirteen powers (patch v0.2.1), each its title and clause, base and radiant, by the stored
 * name the view gives it (R103, R243), in the engine's roll order. `{shot}` is Steady Shot's declared
 * number (R666), which the view fills.
 */
export const POWER_WORDS: Readonly<Record<string, PowerWords>> = {
  recruit: {
    title: "Expedition Map",
    radiantTitle: "Expedition Map",
    base: "Recruit a permanent",
    radiant: "Recruit a permanent. Make it Radiant",
  },
  draw: {
    title: "Life Tap",
    radiantTitle: "Life Tap",
    base: "Draw 1. Take 2 damage",
    radiant: "Draw 1 from each player's deck",
  },
  ping: {
    title: "Ping",
    radiantTitle: "Ping",
    base: "Pierce. Deal 1 damage",
    radiant: "Pierce. Deal 1 damage. If this kills a Unit, summon a Ghoul Token with its stats",
  },
  burn: {
    title: "Steady Shot",
    radiantTitle: "Steady Shot",
    base: "Deal {shot} damage to the enemy hero",
    radiant: "Deal {shot} damage to the enemy hero. Upgrade this permanently by +2 damage",
  },
  rush: { title: "Ranching", radiantTitle: "Ranching", base: "Summon a Rush Token", radiant: "Summon a Radiant Rush Token" },
  felinor: { title: "Cat Cafe", radiantTitle: "Cat Cafe", base: "Summon a Felinor Token", radiant: "Summon a random Felinor" },
  discover: { title: "Witness Value", radiantTitle: "Witness Value", base: "Discover a Unit", radiant: "Discover a Radiant Unit" },
  stitching: {
    title: "Stitching",
    radiantTitle: "Stitching",
    base: "Discover two Units that cost (2) or less. Fuse them and add the result to your hand",
    radiant: "Discover two Radiant Units that cost (2) or less. Fuse them and add the result to your hand",
  },
  armor: {
    title: "Armor Up",
    radiantTitle: "Tank Up",
    base: "Your hero gains 2 Armor until your next turn",
    radiant: "Your hero gains 4 Armor. Refresh this power",
  },
  insect: {
    title: "Die Insect",
    radiantTitle: "Die Insect",
    base: "Deal 8 damage to a random enemy",
    radiant: "Lucky 1. Deal 8 damage to a random enemy",
  },
  brainstorm: {
    title: "KY Brainstorm",
    radiantTitle: "KY Brainstorm",
    base: "Add a random KY card to your hand. Reduce the cost of all Spells in your hand by (1)",
    radiant: "Add a random Radiant KY card to your hand. Reduce the cost of all Spells in your hand by (1)",
  },
  pluck: {
    title: "Pluck",
    radiantTitle: "Pluck",
    base: "Add a random Fruit to your hand. It costs (0)",
    radiant: "Add a random Radiant Fruit to your hand. It costs (0)",
  },
  terminus: {
    title: "Terminus Tricks",
    radiantTitle: "Terminus Tricks",
    base: "Discover a Trap to summon",
    radiant: "Discover a Radiant Trap to summon",
  },
};

/** The table's entry for a stored name, or null for a name it does not know (never a prototype key). */
export function powerWordsOf(name: string): PowerWords | null {
  return Object.hasOwn(POWER_WORDS, name) ? (POWER_WORDS[name] ?? null) : null;
}

/**
 * R43: the X a power's "Activate: Spend (X)" pays, by its stored name, for a hand card whose view
 * names the power but not its X (the card itself costs (0)); null for a name the engine does not have.
 */
export function powerX(name: string): number | null {
  const costs: Readonly<Record<string, number>> = HERO_POWER_COST;
  return Object.hasOwn(costs, name) ? (costs[name] ?? null) : null;
}

/** The power's name as a face prints it (Armor Up's Radiant face is Tank Up), or null for a name the table does not know. */
export function powerTitle(name: string, radiant: boolean): string | null {
  const words = powerWordsOf(name);
  if (words === null) return null;
  return radiant ? words.radiantTitle : words.title;
}

/** The power's clause on a face, its `{key}`s unfilled, or null for a name the table does not know. */
export function powerClause(name: string, radiant: boolean): string | null {
  const words = powerWordsOf(name);
  if (words === null) return null;
  return radiant ? words.radiant : words.base;
}

/**
 * The rolled power as the catalog prints each one, as an ability (§8 #98): "Activate: Spend (X):
 * <Title>: <clause>." Its `{key}`s are left for `fillPowerParams`. Null for a name the table does not
 * know.
 */
export function powerLine(power: RolledPower, radiant: boolean): string | null {
  const title = powerTitle(power.name, radiant);
  const clause = powerClause(power.name, radiant);
  if (title === null || clause === null) return null;
  return `Activate: Spend (${String(power.x)}): ${title}: ${clause}.`;
}

/**
 * A Heroic Power's text in play: its keyword line, then on a line of its own the one power it
 * rolled (`powerLine`), as R366 lays text out. Null for a name this table does not know, which
 * leaves the printed text in place rather than inventing one.
 */
export function powerText(power: RolledPower, radiant: boolean, keywordLine: string): string | null {
  const line = powerLine(power, radiant);
  if (line === null) return null;
  return keywordLine === "" ? line : `${keywordLine}\n${line}`;
}

/**
 * A power's words with their `{key}`s filled (B3.4 rule 5, R386, R666) exactly as `fillParams` fills
 * a face: the view's numbers as they stand (`values`), else the card's printed ones (`params`). With
 * no declaration to hand (no catalog yet) the view's numbers fill alone, so Steady Shot's `{shot}`
 * never shows raw.
 */
export function fillPowerParams(
  text: string,
  radiant: boolean,
  params: readonly Param[] | undefined,
  values?: Readonly<Record<string, number>>,
): string {
  const declared: Param[] =
    params === undefined
      ? Object.entries(values ?? {}).map(([key, value]) => ({ key, base: value, radiant: value, better: "up" }))
      : [...params];
  const face = { keywords: [], text };
  return fillParams({ params: declared, base: face, radiant: face }, radiant ? "radiant" : "base", values);
}

/** Whether a card with these tags reads {@link CONCEALED_TEXT} in play. */
export function concealedInPlay(tags: readonly Tag[]): boolean {
  return tags.includes(CONCEALED_TAG);
}
