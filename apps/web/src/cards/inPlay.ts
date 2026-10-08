// What a face in play prints where the card in play and the card as printed part ways (SPEC §10.10).
//
// A card in the collection is its catalog definition, both faces printed in full. A card in a game
// is the card as the view says it stands (R243), and three cards print something else there:
//
// - #98 Heroic Power rolled one of thirteen powers as it arrived (R43, R151, R752). In play its text is
//   that power alone, read off the view's `power` (a hand card's `CardView.power`, a backrow card's
//   `HeroPowerView`), with its name and its X; the collection keeps the list of thirteen.
// - A card with the Call to Chaos tag (Core #95, Classic+ #73) reads "???" in play. What it does is
//   rolled when it resolves (§8 #95), and the game keeps it a mystery; the collection prints the real
//   text, so a player building a deck can still read it. The Classic+ Edition's Radiant face reads
//   "!!!", as its designer wrote it (docs/classic-sets.md B7 #73), one mystery for three effects.
// - A unit a Vanilla took the text of (§6.3, R115) prints that its text is gone: the definition the
//   client reads still names the keywords and scripts it no longer has, and the view says so
//   (`UnitView.vanilla`, R243).
//
// Presentation only (CLAUDE.md rule 7): every word here is the card's §8 text or says what the view
// already says. `POWER_WORDS` is keyed by the power's name as the view carries it and written from
// §8 #98's clauses, base and radiant, the way `game/modeText.ts` words a "Choose one" option;
// `inPlay.test.ts` holds it to the engine's own table so the two cannot drift apart.

import type { Tag } from "@jackioh/shared";

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
    radiant: "Deal {shot} damage to the enemy hero. Buff this permanently by +2 damage.",
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
 * A Heroic Power's text in play (R752): its keyword line, then the one power it rolled — its name,
 * then on a line of its own its Activate and X and its words, R366's layout — with the card's
 * numbers (`{shot}`, `{ping}`, …) filled in from `values`. Only the rolled power shows, so the other twelve add no
 * clutter. Null for a name this table does not know, which leaves the printed text in place.
 */
export function powerText(
  power: RolledPower,
  radiant: boolean,
  keywordLine: string,
  values: Readonly<Record<string, number>> = {},
): string | null {
  const entry = POWER_WORDS[power.name];
  if (entry === undefined) return null;
  const words = (radiant ? entry.radiant : entry.base).replace(/\{(\w+)\}/g, (whole, key: string) =>
    values[key] === undefined ? whole : String(values[key]),
  );
  const clause = `${radiant ? entry.radiantTitle : entry.title}\nActivate: Spend (${String(entry.x)}): ${words}`;
  return keywordLine === "" ? clause : `${keywordLine}\n${clause}`;
}

/** Whether a card with these tags reads {@link CONCEALED_TEXT} in play. */
export function concealedInPlay(tags: readonly Tag[]): boolean {
  return tags.includes(CONCEALED_TAG);
}
