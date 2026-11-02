// What a face in play prints where the card in play and the card as printed part ways (SPEC §10.10).
// The collection prints the catalog definition, both faces in full; a game prints the card as the view
// says it stands (R243), and three cards differ there:
// - #98 Heroic Power prints only the power it rolled as it arrived (R43, R151, R752), read off the
//   view's `power` (a hand card's `CardView.power`, a backrow card's `HeroPowerView`), name and X.
// - A Call to Chaos card (Core #95, Classic+ #73, Meditative #95) reads "???" in play: what it does is
//   rolled when it resolves (§8 #95); the collection prints the real text for deckbuilding. The Classic+
//   and Meditative Radiant faces read "!!!" for their three effects (docs/classic-sets.md B7 #73).
// - A unit a Vanilla took the text of (§6.3, R115) prints that its text is gone: the definition still
//   names the keywords and scripts it lost, and the view says so (`UnitView.vanilla`, R243).
//
// Presentation only (CLAUDE.md rule 7). `POWER_WORDS` is keyed by the power's name in the view and
// written from §8 #98's clauses; `inPlay.test.ts` holds it to the engine's own table.

import type { Tag } from "@jackioh/shared";

import { t } from "../i18n/i18n.ts";

export const HEROIC_POWER_ID = "core-098";

/** §5: the tag whose cards read {@link CONCEALED_TEXT} in play (#95 Call to Chaos). */
export const CONCEALED_TAG: Tag = "Call to Chaos";

export const CONCEALED_TEXT = "???";

export const CONCEALED_TEXT_LOUD = "!!!";

const CONCEALED_OVERRIDES: Readonly<Record<string, { radiant?: string }>> = {
  "classicplus-073": { radiant: CONCEALED_TEXT_LOUD },
  "meditative-095": { radiant: CONCEALED_TEXT_LOUD },
};

export function concealedText(defId: string, radiant: boolean): string {
  const override = CONCEALED_OVERRIDES[defId];
  return (radiant ? override?.radiant : undefined) ?? CONCEALED_TEXT;
}

/** What a Vanilla unit's rules box reads (§6.3 Vanilla: "remove a unit's text"). */
export const VANILLA_TEXT = "Vanilla: its text is gone";

/** R43, R243: the power a #98 Heroic Power rolled, by the name the view gives it (R103). */
export type RolledPower = { name: string };

type PowerWords = { x: number; title: string; radiantTitle: string; base: string; radiant: string };

/**
 * §8 #98's thirteen powers (R752), by the power's name in the view (R243): each one's X, its name on
 * each face (Armor Up is Tank Up on the Radiant one, R757) and its words, base and Radiant.
 */
export const POWER_WORDS: Readonly<Record<string, PowerWords>> = {
  recruit: { x: 3, title: "Expedition Map", radiantTitle: "Expedition Map", base: "Recruit a permanent.", radiant: "Recruit a permanent. Make it Radiant." },
  draw: { x: 1, title: "Life Tap", radiantTitle: "Life Tap", base: "Draw {tapDraw}. Take {tapDamage} damage.", radiant: "Draw {tapDraw} from each player's deck." },
  ping: {
    x: 1,
    title: "Ping",
    radiantTitle: "Ping",
    base: "Pierce. Deal {ping} damage.",
    radiant: "Pierce. Deal {ping} damage. If this kills a Unit, summon a Ghoul Token with its stats.",
  },
  burn: {
    x: 1,
    title: "Steady Shot",
    radiantTitle: "Steady Shot",
    base: "Deal {shot} damage to the enemy hero.",
    radiant: "Deal {shot} damage to the enemy hero. This permanently deals 2 more damage.",
  },
  rush: { x: 2, title: "Ranching", radiantTitle: "Ranching", base: "Summon a Rush Token.", radiant: "Summon a Radiant Rush Token." },
  felinor: { x: 1, title: "Cat Cafe", radiantTitle: "Cat Cafe", base: "Summon a Felinor Token.", radiant: "Summon a random Felinor." },
  discover: { x: 2, title: "Witness Value", radiantTitle: "Witness Value", base: "Discover a Unit.", radiant: "Discover a Radiant Unit." },
  stitching: {
    x: 2,
    title: "Stitching",
    radiantTitle: "Stitching",
    base: "Discover two ({stitchCost}) Cost or less Units. Fuse them.",
    radiant: "Discover two Radiant ({stitchCost}) Cost or less Units. Fuse them.",
  },
  armor: {
    x: 1,
    title: "Armor Up",
    radiantTitle: "Tank Up",
    base: "Your hero gains {armor} Armor until your next turn.",
    radiant: "Your hero gains {armor} Armor, then this power refreshes.",
  },
  insect: { x: 2, title: "Die Insect", radiantTitle: "Die Insect", base: "Deal {insect} damage to a random enemy.", radiant: "Lucky 1. Deal {insect} damage to a random enemy." },
  brainstorm: {
    x: 2,
    title: "KY Brainstorm",
    radiantTitle: "KY Brainstorm",
    base: "Add a random KY card to your hand. Reduce the cost of all Spells in your hand by ({discount}).",
    radiant: "Add a Radiant KY card to your hand. Reduce the cost of all Spells in your hand by ({discount}).",
  },
  pluck: {
    x: 2,
    title: "Pluck",
    radiantTitle: "Pluck",
    base: "Add a random Fruit to your hand. It costs ({fruitCost}).",
    radiant: "Add a random Radiant Fruit to your hand. It costs ({fruitCost}).",
  },
  tricks: { x: 3, title: "Terminus Tricks", radiantTitle: "Terminus Tricks", base: "Discover a Trap to summon.", radiant: "Discover a Radiant Trap to summon." },
};

/** The power's name on a face (R752): what the hero panel and the card in play call it. */
export function powerTitle(power: RolledPower, radiant: boolean): string {
  const words = POWER_WORDS[power.name];
  return words === undefined ? power.name : radiant ? words.radiantTitle : words.title;
}

/**
 * A Heroic Power's text in play (R752, R366's layout): its keyword line, the rolled power's name, then on
 * its own line its Activate, X and words, `{…}` numbers filled from `values`. Null for a name this table
 * does not know, which leaves the printed text in place. `translate` (R711000): name and words in the
 * reader's language; the English is what the glossary's terms are found in, so it is never translated.
 */
export function powerText(
  power: RolledPower,
  radiant: boolean,
  keywordLine: string,
  values: Readonly<Record<string, number>> = {},
  translate = false,
): string | null {
  const entry = POWER_WORDS[power.name];
  if (entry === undefined) return null;
  const say = (english: string): string => (translate ? t(english) : english);
  const words = say(radiant ? entry.radiant : entry.base).replace(/\{(\w+)\}/g, (whole, key: string) =>
    values[key] === undefined ? whole : String(values[key]),
  );
  const title = say(radiant ? entry.radiantTitle : entry.title);
  const line = translate
    ? t("Activate: Spend ({x}): {words}", { x: entry.x, words })
    : `Activate: Spend (${String(entry.x)}): ${words}`;
  const clause = `${title}\n${line}`;
  return keywordLine === "" ? clause : `${keywordLine}\n${clause}`;
}

export function concealedInPlay(tags: readonly Tag[]): boolean {
  return tags.includes(CONCEALED_TAG);
}
