// BUILD M4-T1 and M9-T1 acceptance: catalog.json is the diff against SPEC §8 and the brief.
//
// PROVENANCE OF THE FIXTURE TABLES BELOW. Each table is a transcription of a document, not of
// catalog.json, so that a card whose data drifts from its source fails here. Each was produced
// mechanically from its document's own lines and checked in as a literal; the test never reads a
// document at run time (a test that re-parsed one would pass while it and the catalog drifted
// together, and would put file I/O in a package that must stay pure — CLAUDE.md rule 4).
//
//   CORE           — SPEC §8.1–§8.5, the 105 rows of the catalog table, columns "#", "Name",
//                    "Rarity", "Cost", "Type, tags" and "Stats" (`A/B → C/D` is base → radiant;
//                    `A/B` alone means the radiant form keeps those stats; an empty cell means the
//                    card has no stats), and SPEC §7's six named tokens (`T-rush`, `T-sheep`,
//                    `T-felinor`, `T-bread`, `T-coin`, the one §2.1's setup deals, R244, and
//                    `T-ghoul`, a card of its own in patch v0.1.1, R353), whose `rarity: "Token"` and
//                    Token tag follow §8's in-catalog token rows and §7's token rules. The costs of
//                    #16 Hit Job (3), #17 Flood, #34 Collateral Damage, #43 Big Felinor, #49 Snom
//                    Bunny Mind Control, #88 Twisting Nether (4) and #65 Masochism Mask (1) are
//                    patch v0.2.0's (issue #40).
//   CLASSIC        — docs/classic-sets.md B6, each entry's header line (`classic-NNN` · (cost)
//                    type, tags · rarity · stats base → Radiant), with the decisions B9 and the
//                    v0.2.0 plan adopted: #55 is Book of Wildfire, #72 Grand Counterspell (R381),
//                    #86 Genn's Radiant 42/42.
//   CLASSIC_PLUS   — docs/classic-sets.md B7's header lines and token bullets (B2.3's numbering;
//                    the Losers typed Unit, Otherworldly Removal's three and the Grapes typed Spell
//                    by their sections' own sentences, the Grapes tagged Fruit, every token tagged
//                    Token and rated "Token", B2.5) and B8's table of the ten AI generated cards
//                    (tagged AI). Buff Billy's "3X/3X → 7X/7X" prints 0/0 (its `xStats` carry X).
//   radiant faces  — every entry has a Radiant face of its own (§5.2, R276), the Ghoul Token's
//                    being R349's fallback.
//   rarity         — SPEC §8's rarity paragraph for Core (32/40/16/7/5) and B2.5's table for
//                    Classic (35/26/18/10/1) and Classic+ (13/24/25/13/3), as patch v0.2.17
//                    (issue #44) left them; B2.1's totals, 268 cards and 50 tokens in 318
//                    entries (Glitch, issue #170, the fiftieth).
//   tag vocabulary — SPEC §5/§6 tags as BUILD M4-T1 lists them, R278's Jlockeed, B2.4's Book,
//                    Pancake and AI, and the mechanics patch's Plague (every card that uses
//                    Plague Counters).
//
// Not asserted here: `keywords`, `text`, `params`, `refs` and `loc` (the per-card tests prove the
// behaviour `keywords`, `text` and `params` describe, references.test.ts proves `refs`, loc.test.ts
// `loc`), and id shape (`packages/cards/scripts/validate-catalog.ts` owns the schema).

import { describe, expect, it } from "vitest";
import { fillParams, type CardCost, type CardDef, type CardFace, type CardType, type Rarity, type SetName, type Tag } from "@jackioh/shared";
import { GLITCH_DEF_ID } from "@jackioh/engine";
import { CATALOG } from "../src/catalog-data";

/** `[attack, health]`, or `null` for a §8 Stats cell that is empty (a card with no stats). */
type StatPair = readonly [number | null, number | null];

type SpecRow = {
  readonly index: string;
  readonly name: string;
  readonly cost: CardCost;
  readonly type: CardType;
  readonly tags: readonly Tag[];
  readonly rarity: Rarity;
  readonly base: StatPair;
  readonly radiant: StatPair;
};

const CORE: readonly SpecRow[] = [
  { index: "1", name: "Big D-fender", cost: 2, type: "Unit", tags: ["Human"], rarity: "Common", base: [0, 7], radiant: [0, 14] },
  { index: "2", name: "Bigot", cost: 2, type: "Unit", tags: ["Human"], rarity: "Common", base: [6, 1], radiant: [12, 2] },
  { index: "3", name: "Right-house defender", cost: 1, type: "Unit", tags: ["Human"], rarity: "Common", base: [1, 1], radiant: [2, 2] },
  { index: "4", name: "Gary the Gambler", cost: 1, type: "Unit", tags: ["Human"], rarity: "Common", base: [1, 1], radiant: [2, 2] },
  { index: "5", name: "Stockpile", cost: 1, type: "Spell", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "6", name: "Mana Well", cost: 3, type: "Field Spell", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "7", name: "Jewelosco Scarab", cost: 1, type: "Unit", tags: [], rarity: "Rare", base: [1, 1], radiant: [2, 2] },
  { index: "8", name: "Mr. Vanilla", cost: 1, type: "Unit", tags: ["Human"], rarity: "Common", base: [4, 4], radiant: [12, 12] },
  { index: "9", name: "Moths to the Flame", cost: 2, type: "Unit", tags: [], rarity: "Rare", base: [1, 14], radiant: [2, 28] },
  { index: "10", name: "Rapid Replenish", cost: 0, type: "Spell", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "11", name: "Tempo Timmy", cost: 1, type: "Unit", tags: ["Human"], rarity: "Common", base: [3, 3], radiant: [6, 6] },
  { index: "12", name: "Duplicating Felinors", cost: 2, type: "Unit", tags: ["Felinor"], rarity: "Rare", base: [3, 4], radiant: [6, 9] },
  { index: "13", name: "Jlockeed Shredder-10", cost: 3, type: "Unit", tags: ["Jlockeed"], rarity: "Common", base: [8, 10], radiant: [16, 20] },
  { index: "14", name: "Jlockeed's Weapons", cost: 4, type: "Field Spell", tags: ["Jlockeed"], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "15", name: "Me and Mr Token", cost: 1, type: "Unit", tags: ["Human"], rarity: "Common", base: [1, 1], radiant: [2, 2] },
  { index: "16", name: "Hit Job", cost: 3, type: "Spell", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "17", name: "Flood", cost: 4, type: "Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "18", name: "Bread and Butter", cost: 1, type: "Field Trap", tags: [], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "19", name: "Midrange Menace", cost: 3, type: "Unit", tags: [], rarity: "Common", base: [9, 9], radiant: [18, 18] },
  { index: "20", name: "Pointmaster", cost: 2, type: "Unit", tags: ["Human"], rarity: "Common", base: [7, 1], radiant: [14, 2] },
  { index: "21", name: "Hinder", cost: 0, type: "Spell", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "22", name: "Carnivorous Cube", cost: 3, type: "Unit", tags: [], rarity: "Epic", base: [4, 6], radiant: [8, 12] },
  { index: "23", name: "Reoccurring Dream", cost: 1, type: "Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "24", name: "Efficiency Dividend", cost: "X", type: "Spell", tags: [], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "25", name: "4-mana 7/7", cost: 4, type: "Unit", tags: [], rarity: "Common", base: [7, 7], radiant: [14, 14] },
  { index: "26", name: "Glowy Jelly Bean", cost: 3, type: "Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "27", name: "Blood Ridden Glowy Jelly Bean", cost: 1, type: "Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "28", name: "Knockoff Temu Glowy Jelly Bean", cost: 2, type: "Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "29", name: "GIGA Glowy Jelly Bean", cost: 6, type: "Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "30", name: "Archivist", cost: 2, type: "Unit", tags: ["Human"], rarity: "Rare", base: [4, 5], radiant: [8, 10] },
  { index: "31", name: "KY's Math Equation", cost: 1, type: "Spell", tags: ["KY"], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "32", name: "Prem Panther", cost: 2, type: "Unit", tags: [], rarity: "Rare", base: [5, 4], radiant: [10, 8] },
  { index: "33", name: "Unstable Clone Machine", cost: 2, type: "Field Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "34", name: "Collateral Damage", cost: 4, type: "Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "35", name: "Lunar Eclipse", cost: 1, type: "Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "36", name: "Magic Jammed", cost: 1, type: "Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "37", name: "Gravedigger", cost: 2, type: "Unit", tags: [], rarity: "Rare", base: [4, 5], radiant: [8, 10] },
  { index: "38", name: "Quickstriker", cost: 3, type: "Field Spell", tags: [], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "39", name: "Recycling Initiative", cost: 0, type: "Spell", tags: [], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "40", name: "Echoes of the Forgotten", cost: 2, type: "Field Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "41", name: "Sheepish", cost: 1, type: "Trap", tags: [], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "42", name: "Eugenics", cost: 2, type: "Spell", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "43", name: "Big Felinor", cost: 4, type: "Unit", tags: ["Felinor"], rarity: "Rare", base: [3, 10], radiant: [6, 20] },
  { index: "44", name: "True Strike", cost: 1, type: "Spell", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "45", name: "Deft Duelist", cost: 2, type: "Unit", tags: ["Human"], rarity: "Rare", base: [4, 3], radiant: [8, 6] },
  { index: "46", name: "Suppressive Aura", cost: { base: 2, embiggen: 4 }, type: "Field Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "47", name: "Fig of Life", cost: 3, type: "Spell", tags: ["Fruit"], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "48", name: "5pek Controller", cost: 0, type: "Spell", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "49", name: "Snom Bunny Mind Control", cost: 4, type: "Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "50", name: "K-Pop Fanatic", cost: 1, type: "Unit", tags: ["Human"], rarity: "Epic", base: [1, 1], radiant: [2, 2] },
  { index: "51", name: "KY's Private Tutor", cost: 1, type: "Spell", tags: ["KY"], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "51.1", name: "KY's Empty Notebook", cost: 1, type: "Spell", tags: ["KY", "Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "52", name: "Silly Silas", cost: 3, type: "Unit", tags: ["Human"], rarity: "Legendary", base: [4, 4], radiant: [8, 8] },
  { index: "53", name: "Reno", cost: 3, type: "Unit", tags: ["Human"], rarity: "Common", base: [4, 6], radiant: [8, 12] },
  { index: "54", name: "Straaza", cost: 4, type: "Unit", tags: [], rarity: "Common", base: [8, 8], radiant: [16, 16] },
  { index: "55", name: "Lava Golem", cost: 3, type: "Unit", tags: [], rarity: "Rare", base: [10, 5], radiant: [20, 10] },
  { index: "56", name: "Jilliax", cost: 2, type: "Unit", tags: [], rarity: "Common", base: [3, 2], radiant: [6, 4] },
  { index: "57", name: "Conjure KY", cost: 2, type: "Spell", tags: ["KY"], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "58", name: "Rush Token Farm", cost: 2, type: "Field Spell", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "59", name: "Unbiased Immigration", cost: { base: 2, embiggen: 4 }, type: "Field Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "60", name: "Bear Honeypot", cost: 1, type: "Trap", tags: [], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "61", name: "Prejudiced Postdoc", cost: 2, type: "Unit", tags: ["Human"], rarity: "Rare", base: [2, 4], radiant: [4, 8] },
  { index: "62", name: "Friend of Felinors", cost: 1, type: "Spell", tags: ["Felinor"], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "63", name: "Plastic Surgery", cost: 1, type: "Spell", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "64", name: "Gifted Program", cost: 2, type: "Field Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "65", name: "Masochism Mask", cost: 1, type: "Field Spell", tags: ["Quickdraw"], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "65.1", name: "Spikey Pillow", cost: 1, type: "Unit", tags: ["Token"], rarity: "Token", base: [0, 2], radiant: [0, 4] },
  { index: "66", name: "The Rock", cost: 4, type: "Unit", tags: ["Human"], rarity: "Common", base: [10, 10], radiant: [20, 20] },
  { index: "67", name: "Zoomerbin Oomen", cost: 1, type: "Unit", tags: ["Human"], rarity: "Rare", base: [1, 2], radiant: [2, 4] },
  { index: "68", name: "Twisted Sorcerer", cost: 2, type: "Unit", tags: [], rarity: "Common", base: [5, 5], radiant: [10, 10] },
  { index: "69", name: "Call to Arms", cost: 2, type: "Spell", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "70", name: "Spiteful Stab", cost: 3, type: "Spell", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "71", name: "Intern Stimmy", cost: 1, type: "Field Trap", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "72", name: "Reminisce", cost: 1, type: "Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "73", name: "Anti-oneshot Armor", cost: 2, type: "Field Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "74", name: "Adaptive UI", cost: "X", type: "Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "75", name: "Infinite Reserves", cost: 0, type: "Field Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "76", name: "Field of Dreams", cost: 3, type: "Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "77", name: "Professor Curvature", cost: 2, type: "Unit", tags: ["Human"], rarity: "Rare", base: [3, 3], radiant: [6, 6] },
  { index: "78", name: "/fullsend", cost: 4, type: "Spell", tags: [], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "79", name: "Twinspell", cost: 2, type: "Field Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "80", name: "Zao Gao", cost: 2, type: "Spell", tags: ["CN"], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "81", name: "Radiant Saintess", cost: 1, type: "Unit", tags: ["Human"], rarity: "Epic", base: [2, 2], radiant: [4, 4] },
  { index: "82", name: "KY's Trial", cost: 1, type: "Spell", tags: ["KY"], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "83", name: "Transmogulate", cost: 2, type: "Spell", tags: [], rarity: "Legendary", base: [null, null], radiant: [null, null] },
  { index: "84", name: "Going Long", cost: { base: 2, embiggen: 4 }, type: "Field Spell", tags: ["Quickdraw"], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "85", name: "Unlicensed Experimentation", cost: 2, type: "Trap", tags: [], rarity: "Legendary", base: [null, null], radiant: [null, null] },
  { index: "86", name: "\"Miss\" Mrow", cost: 1, type: "Unit", tags: ["Felinor"], rarity: "Epic", base: [1, 1], radiant: [2, 2] },
  { index: "87", name: "Pocket Chaos", cost: 4, type: "Spell", tags: [], rarity: "Legendary", base: [null, null], radiant: [null, null] },
  { index: "88", name: "Twisting Nether", cost: 4, type: "Spell", tags: [], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "89", name: "Corpse Eater", cost: 4, type: "Unit", tags: [], rarity: "Epic", base: [2, 2], radiant: [6, 6] },
  { index: "90", name: "CN-Viral Injection", cost: 2, type: "Spell", tags: ["CN"], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "90.1", name: "CN-Virus", cost: 1, type: "Spell", tags: ["CN", "Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "91", name: "Fed Fauci", cost: 2, type: "Unit", tags: ["Human", "Plague"], rarity: "Rare", base: [1, 6], radiant: [2, 12] },
  { index: "92", name: "Felinor Fiender", cost: 2, type: "Unit", tags: ["Human"], rarity: "Legendary", base: [5, 7], radiant: [10, 14] },
  { index: "93", name: "Combo-Index", cost: 2, type: "Field Spell", tags: [], rarity: "Legendary", base: [null, null], radiant: [null, null] },
  { index: "93.1", name: "Combo-Fodder", cost: 0, type: "Spell", tags: ["Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "94", name: "Genn's Greed", cost: 4, type: "Spell", tags: [], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "95", name: "Call to Chaos (Core Edition)", cost: 4, type: "Spell", tags: ["Call to Chaos"], rarity: "Legendary", base: [null, null], radiant: [null, null] },
  { index: "95.1", name: "Chaos Golem", cost: 4, type: "Unit", tags: ["Token"], rarity: "Token", base: [10, 10], radiant: [20, 20] },
  { index: "96", name: "My Pawn", cost: 1, type: "Trap", tags: [], rarity: "Mythic", base: [null, null], radiant: [null, null] },
  { index: "97", name: "Zephyrs", cost: 0, type: "Spell", tags: [], rarity: "Mythic", base: [null, null], radiant: [null, null] },
  { index: "98", name: "Heroic Power", cost: "X", type: "Field Spell", tags: ["Quickdraw"], rarity: "Mythic", base: [null, null], radiant: [null, null] },
  { index: "99", name: "Craft a Card", cost: 4, type: "Spell", tags: [], rarity: "Mythic", base: [null, null], radiant: [null, null] },
  { index: "100", name: "Ceaseless Void", cost: 100, type: "Unit", tags: [], rarity: "Mythic", base: [10, 10], radiant: [20, 20] },
  { index: "T-rush", name: "Rush Token", cost: 1, type: "Unit", tags: ["Token"], rarity: "Token", base: [3, 3], radiant: [6, 6] },
  { index: "T-sheep", name: "Sheep Token", cost: 1, type: "Unit", tags: ["Token"], rarity: "Token", base: [1, 1], radiant: [2, 2] },
  { index: "T-felinor", name: "Felinor Token", cost: 1, type: "Unit", tags: ["Felinor", "Token"], rarity: "Token", base: [1, 1], radiant: [2, 2] },
  { index: "T-bread", name: "Bread Token", cost: 0, type: "Unit", tags: ["Token"], rarity: "Token", base: [0, 0], radiant: [0, 0] },
  { index: "T-coin", name: "The Coin", cost: 0, type: "Spell", tags: ["Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "T-ghoul", name: "Ghoul Token", cost: 0, type: "Unit", tags: ["Token"], rarity: "Token", base: [0, 0], radiant: [0, 0] },
];

const CLASSIC: readonly SpecRow[] = [
  { index: "1", name: "Curse of the Forgotten Classic", cost: 1, type: "Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "2", name: "The Trickster", cost: 1, type: "Unit", tags: ["Human"], rarity: "Common", base: [2, 1], radiant: [4, 2] },
  { index: "3", name: "Book of Heal", cost: 1, type: "Spell", tags: ["Book"], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "4", name: "Palantir", cost: 1, type: "Field Spell", tags: ["Jlockeed"], rarity: "Legendary", base: [null, null], radiant: [null, null] },
  { index: "5", name: "Tesla", cost: 2, type: "Field Trap", tags: [], rarity: "Epic", base: [1, 4], radiant: [2, 8] },
  { index: "6", name: "Cloaked Toe Cracker", cost: 2, type: "Unit", tags: ["Human"], rarity: "Common", base: [3, 4], radiant: [6, 8] },
  { index: "7", name: "InfiniScepter", cost: 1, type: "Field Spell", tags: [], rarity: "Legendary", base: [null, null], radiant: [null, null] },
  { index: "8", name: "Pickle", cost: 1, type: "Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "9", name: "Income Tax", cost: 2, type: "Trap", tags: [], rarity: "Legendary", base: [null, null], radiant: [null, null] },
  { index: "10", name: "Exile", cost: 1, type: "Trap", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "11", name: "Mind Melt", cost: 1, type: "Spell", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "12", name: "Book of Blood", cost: 1, type: "Spell", tags: ["Book"], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "13", name: "Boots on the Ground", cost: 1, type: "Unit", tags: ["Human"], rarity: "Common", base: [2, 1], radiant: [4, 2] },
  { index: "14", name: "Shadowstep", cost: 2, type: "Trap", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "15", name: "Nose Hunter", cost: 1, type: "Unit", tags: ["Human"], rarity: "Common", base: [3, 1], radiant: [6, 2] },
  { index: "16", name: "Book of Flame", cost: 1, type: "Spell", tags: ["Book"], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "17", name: "Counterspell", cost: 2, type: "Trap", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "18", name: "Glitch in the System", cost: 3, type: "Spell", tags: [], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "19", name: "Lizard's Breath", cost: 1, type: "Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "20", name: "The Power to Punish", cost: 2, type: "Field Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "21", name: "Turtinator", cost: 2, type: "Unit", tags: [], rarity: "Common", base: [5, 4], radiant: [10, 8] },
  { index: "22", name: "Mid Runner", cost: 1, type: "Unit", tags: ["Human"], rarity: "Common", base: [2, 1], radiant: [4, 2] },
  { index: "23", name: "Devil's Pact", cost: 2, type: "Field Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "24", name: "Book of Knowledge", cost: 1, type: "Spell", tags: ["Book"], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "25", name: "Lag in the System", cost: 0, type: "Spell", tags: [], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "26", name: "Rapid Draw", cost: 0, type: "Spell", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "27", name: "Pestilent Slime", cost: 0, type: "Unit", tags: ["Plague"], rarity: "Common", base: [1, 1], radiant: [2, 2] },
  { index: "28", name: "Second Wind", cost: 0, type: "Field Spell", tags: [], rarity: "Legendary", base: [null, null], radiant: [null, null] },
  { index: "29", name: "Book of Vital Kill", cost: 1, type: "Spell", tags: ["Book"], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "30", name: "Recycle", cost: 1, type: "Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "31", name: "Cookie Guild", cost: 2, type: "Unit", tags: ["Human"], rarity: "Common", base: [2, 4], radiant: [4, 8] },
  { index: "32", name: "Felinor Feelings", cost: 0, type: "Spell", tags: ["Felinor"], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "33", name: "Joro", cost: 0, type: "Unit", tags: [], rarity: "Legendary", base: [1, 1], radiant: [2, 2] },
  { index: "34", name: "Ancient Acquisition", cost: 1, type: "Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "35", name: "Prep", cost: 0, type: "Spell", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "36", name: "Burn", cost: 0, type: "Spell", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "37", name: "Last Hurrah", cost: 1, type: "Spell", tags: [], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "38", name: "Jackiestan Auctioneer", cost: 2, type: "Field Trap", tags: ["Human"], rarity: "Rare", base: [4, 4], radiant: [8, 8] },
  { index: "39", name: "Outbreak", cost: 1, type: "Spell", tags: ["Plague"], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "40", name: "MC Tech", cost: 1, type: "Unit", tags: [], rarity: "Rare", base: [3, 3], radiant: [6, 6] },
  { index: "41", name: "State of the Game", cost: 1, type: "Unit", tags: [], rarity: "Common", base: [3, 3], radiant: [6, 6] },
  { index: "42", name: "Transmutable Toxins", cost: 2, type: "Field Spell", tags: ["Plague"], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "43", name: "Plague Nuke", cost: 4, type: "Spell", tags: ["Plague"], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "44", name: "Back from the GY", cost: 4, type: "Spell", tags: [], rarity: "Legendary", base: [null, null], radiant: [null, null] },
  { index: "45", name: "Nature Titan", cost: 2, type: "Unit", tags: [], rarity: "Legendary", base: [6, 6], radiant: [12, 12] },
  { index: "46", name: "Divine Favor", cost: 1, type: "Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "47", name: "Recurring Felinor", cost: 2, type: "Unit", tags: ["Felinor"], rarity: "Rare", base: [3, 2], radiant: [6, 4] },
  { index: "48", name: "Hired Shrimp", cost: 2, type: "Unit", tags: [], rarity: "Common", base: [4, 3], radiant: [8, 6] },
  { index: "49", name: "Anti-Greed Machine", cost: 3, type: "Unit", tags: [], rarity: "Common", base: [9, 9], radiant: [18, 18] },
  { index: "50", name: "Voidwalker", cost: 2, type: "Unit", tags: [], rarity: "Rare", base: [6, 3], radiant: [12, 6] },
  { index: "51", name: "Back Breaker", cost: 1, type: "Unit", tags: [], rarity: "Common", base: [3, 2], radiant: [6, 4] },
  { index: "52", name: "Final Gambit", cost: 2, type: "Trap", tags: [], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "53", name: "Plague Crawler", cost: 1, type: "Unit", tags: ["Plague"], rarity: "Common", base: [2, 2], radiant: [4, 4] },
  { index: "54", name: "Rewind", cost: 1, type: "Spell", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "55", name: "Book of Wildfire", cost: 1, type: "Spell", tags: ["Book"], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "56", name: "Spell Tyrant", cost: 4, type: "Unit", tags: [], rarity: "Legendary", base: [5, 5], radiant: [10, 10] },
  { index: "57", name: "Echo", cost: 1, type: "Spell", tags: [], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "58", name: "Common Resources", cost: 2, type: "Field Spell", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "59", name: "Plague Doctor", cost: 1, type: "Unit", tags: ["Human", "Plague"], rarity: "Common", base: [2, 3], radiant: [4, 6] },
  { index: "60", name: "Pile On", cost: 5, type: "Spell", tags: [], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "61", name: "Plague Bringer Goliath", cost: 3, type: "Unit", tags: ["Plague"], rarity: "Rare", base: [7, 7], radiant: [14, 14] },
  { index: "62", name: "Living Bomb", cost: 1, type: "Field Spell", tags: ["Plague"], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "63", name: "Crop Dusting", cost: 2, type: "Trap", tags: ["Plague"], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "64", name: "Malzahar's Recycler", cost: 2, type: "Field Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "65", name: "Ace in the Hole", cost: 2, type: "Trap", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "66", name: "EU Striker", cost: 2, type: "Unit", tags: ["Human"], rarity: "Common", base: [5, 4], radiant: [10, 8] },
  { index: "67", name: "Felinor Feeler", cost: 1, type: "Unit", tags: ["Human"], rarity: "Common", base: [2, 4], radiant: [4, 8] },
  { index: "68", name: "Small Card Lobbyist", cost: 4, type: "Unit", tags: [], rarity: "Common", base: [11, 13], radiant: [22, 26] },
  { index: "69", name: "Plague Charger", cost: 2, type: "Unit", tags: ["Plague"], rarity: "Rare", base: [4, 2], radiant: [8, 4] },
  { index: "70", name: "Book of Plague", cost: 1, type: "Spell", tags: ["Book", "Plague"], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "71", name: "Lane Eater", cost: 3, type: "Unit", tags: [], rarity: "Common", base: [4, 4], radiant: [8, 8] },
  { index: "72", name: "Grand Counterspell", cost: 2, type: "Trap", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "73", name: "Nurse Cleaver", cost: 2, type: "Unit", tags: [], rarity: "Common", base: [3, 6], radiant: [6, 12] },
  { index: "74", name: "Corpse Plantation", cost: 2, type: "Field Spell", tags: ["Plague"], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "75", name: "Argusland", cost: 3, type: "Field Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "76", name: "Plague Bringer", cost: 2, type: "Unit", tags: ["Plague"], rarity: "Rare", base: [4, 4], radiant: [8, 8] },
  { index: "77", name: "Anti-Magic Monkey", cost: 2, type: "Unit", tags: [], rarity: "Common", base: [5, 5], radiant: [10, 10] },
  { index: "78", name: "Mutate Spell", cost: 1, type: "Field Spell", tags: ["Plague"], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "79", name: "Risky Die", cost: 1, type: "Spell", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "80", name: "BOOM! Big Max", cost: 4, type: "Unit", tags: [], rarity: "Legendary", base: [13, 8], radiant: [26, 16] },
  { index: "81", name: "The Power to Thrive", cost: 2, type: "Field Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "82", name: "Sheeople", cost: 1, type: "Unit", tags: [], rarity: "Common", base: [1, 1], radiant: [2, 2] },
  { index: "83", name: "Flame Lance", cost: 3, type: "Spell", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "84", name: "Lockdown", cost: 2, type: "Field Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "85", name: "King Wagtoggle", cost: 4, type: "Unit", tags: [], rarity: "Legendary", base: [5, 5], radiant: [10, 10] },
  { index: "86", name: "Genn", cost: 4, type: "Unit", tags: [], rarity: "Common", base: [14, 14], radiant: [42, 42] },
  { index: "87", name: "Plague Chalice", cost: "X", type: "Field Spell", tags: ["Plague"], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "88", name: "Siphon Squad", cost: 2, type: "Field Trap", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "89", name: "Paul Allen's Ghost", cost: 2, type: "Unit", tags: [], rarity: "Rare", base: [5, 6], radiant: [10, 12] },
  { index: "90", name: "In Too Deep", cost: 1, type: "Field Spell", tags: ["Quickdraw"], rarity: "Mythic", base: [null, null], radiant: [null, null] },
  // Issue #170, R674: Glitch, the hidden token R673's roll makes, blank on both faces.
  { index: "T-glitch", name: "Glitch", cost: 0, type: "Spell", tags: ["Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
];

const CLASSIC_PLUS: readonly SpecRow[] = [
  { index: "1", name: "Doom Shroom", cost: 3, type: "Trap", tags: [], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "2", name: "Groom Shroom", cost: 3, type: "Trap", tags: ["Felinor"], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "3", name: "Second Amendment Snake", cost: 2, type: "Unit", tags: ["Plague"], rarity: "Rare", base: [1, 6], radiant: [2, 12] },
  { index: "4", name: "Juhan Biggest Bat", cost: 3, type: "Unit", tags: ["CN"], rarity: "Common", base: [9, 6], radiant: [18, 12] },
  { index: "5", name: "Guy Att", cost: 2, type: "Unit", tags: ["Human"], rarity: "Common", base: [6, 8], radiant: [12, 16] },
  { index: "6", name: "Wrong-House Attacker", cost: 1, type: "Unit", tags: ["Human"], rarity: "Common", base: [1, 1], radiant: [2, 2] },
  { index: "7", name: "The House", cost: 3, type: "Field Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "8", name: "Withering Storm", cost: 2, type: "Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "9", name: "Silence", cost: 0, type: "Spell", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "10", name: "New Wraps", cost: 0, type: "Spell", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "11", name: "Anime Armor", cost: 2, type: "Unit", tags: [], rarity: "Rare", base: [4, 4], radiant: [8, 8] },
  { index: "12", name: "The Mother Pancake", cost: 3, type: "Unit", tags: ["Pancake"], rarity: "Legendary", base: [8, 8], radiant: [16, 16] },
  { index: "12.1", name: "Devour", cost: 0, type: "Spell", tags: ["Pancake", "Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "12.2", name: "Death Boil", cost: 1, type: "Spell", tags: ["Pancake", "Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "12.3", name: "Fluffy Grip", cost: 1, type: "Spell", tags: ["Pancake", "Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "12.4", name: "Powder Spray", cost: 1, type: "Spell", tags: ["Pancake", "Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "12.5", name: "Anti-Waffle Shell", cost: 1, type: "Field Spell", tags: ["Pancake", "Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "12.6", name: "Frozen Wastes", cost: 2, type: "Spell", tags: ["Pancake", "Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "12.7", name: "Legion of the Hungry", cost: 2, type: "Field Spell", tags: ["Pancake", "Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "12.8", name: "Frostspatula", cost: 2, type: "Field Spell", tags: ["Pancake", "Token"], rarity: "Token", base: [10, 3], radiant: [20, 6] },
  { index: "13", name: "Mommy Barker", cost: 1, type: "Unit", tags: ["Human", "Pancake"], rarity: "Legendary", base: [2, 2], radiant: [4, 4] },
  { index: "14", name: "Forever&", cost: 1, type: "Spell", tags: [], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "15", name: "Conjure Rush Token", cost: 1, type: "Spell", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "16", name: "Conjure Rush Token+", cost: 2, type: "Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "17", name: "Conjure Rush Token++", cost: 4, type: "Spell", tags: [], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "18", name: "Gullible Treatler", cost: 2, type: "Unit", tags: ["Human"], rarity: "Common", base: [8, 9], radiant: [16, 18] },
  { index: "19", name: "League of Losers", cost: 4, type: "Spell", tags: [], rarity: "Legendary", base: [null, null], radiant: [null, null] },
  { index: "19.1", name: "Top Loser", cost: 2, type: "Unit", tags: ["Token"], rarity: "Token", base: [5, 5], radiant: [10, 10] },
  { index: "19.2", name: "Jungle Loser", cost: 2, type: "Unit", tags: ["Token"], rarity: "Token", base: [5, 5], radiant: [10, 10] },
  { index: "19.3", name: "Mid Loser", cost: 2, type: "Unit", tags: ["Token"], rarity: "Token", base: [5, 5], radiant: [10, 10] },
  { index: "19.4", name: "Support Loser", cost: 2, type: "Unit", tags: ["Token"], rarity: "Token", base: [0, 5], radiant: [0, 10] },
  { index: "19.5", name: "Bot Loser", cost: 2, type: "Unit", tags: ["Token"], rarity: "Token", base: [5, 5], radiant: [10, 10] },
  { index: "20", name: "Mushroom Power", cost: 1, type: "Unit", tags: [], rarity: "Common", base: [2, 2], radiant: [4, 4] },
  { index: "21", name: "Whirlwind", cost: 0, type: "Spell", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "22", name: "Blood Moon", cost: 1, type: "Trap", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "23", name: "Dropshipping", cost: 1, type: "Spell", tags: ["CN"], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "24", name: "Crushing Walls", cost: 3, type: "Spell", tags: [], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "25", name: "Soul Shot", cost: 2, type: "Spell", tags: [], rarity: "Common", base: [null, null], radiant: [null, null] },
  { index: "26", name: "Tommy Tempo", cost: 3, type: "Unit", tags: ["Human"], rarity: "Common", base: [9, 9], radiant: [18, 18] },
  { index: "27", name: "Zephrys Zealotism", cost: 4, type: "Spell", tags: [], rarity: "Mythic", base: [null, null], radiant: [null, null] },
  { index: "28", name: "Nuestro hogar, nuestras tumbas", cost: 2, type: "Unit", tags: [], rarity: "Common", base: [3, 4], radiant: [6, 8] },
  { index: "29", name: "Portal to the Past", cost: 3, type: "Spell", tags: [], rarity: "Mythic", base: [null, null], radiant: [null, null] },
  { index: "30", name: "Felinor Fuser", cost: 4, type: "Unit", tags: ["Felinor"], rarity: "Epic", base: [3, 3], radiant: [6, 6] },
  { index: "31", name: "Fusion Lab", cost: 2, type: "Field Spell", tags: [], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "32", name: "Otherworldly Removal", cost: 2, type: "Spell", tags: [], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "32.1", name: "Execute", cost: 1, type: "Spell", tags: ["Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "32.2", name: "Brawl", cost: 2, type: "Spell", tags: ["Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "32.3", name: "Blade Storm", cost: 1, type: "Spell", tags: ["Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "33", name: "Ivory Tower", cost: 2, type: "Field Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "34", name: "Memory Leak", cost: 3, type: "Field Spell", tags: [], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "35", name: "Rollback", cost: 4, type: "Spell", tags: [], rarity: "Legendary", base: [null, null], radiant: [null, null] },
  { index: "36", name: "Conjure Bones", cost: {base: 2, embiggen: 4}, type: "Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "36.1", name: "Bone Storm", cost: 1, type: "Spell", tags: ["Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "37", name: "Wardrum", cost: 5, type: "Unit", tags: ["Quickdraw"], rarity: "Legendary", base: [5, 5], radiant: [10, 10] },
  { index: "38", name: "Solarius", cost: 2, type: "Unit", tags: [], rarity: "Epic", base: [3, 2], radiant: [6, 4] },
  { index: "38.1", name: "Solarius-Prime", cost: 4, type: "Unit", tags: ["Token"], rarity: "Token", base: [9, 5], radiant: [18, 10] },
  { index: "39", name: "Book Worm", cost: 1, type: "Unit", tags: [], rarity: "Common", base: [1, 4], radiant: [2, 8] },
  { index: "40", name: "Appropriations", cost: "X", type: "Spell", tags: [], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "41", name: "KY's Constant", cost: 1, type: "Spell", tags: ["KY"], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "42", name: "KY's Test", cost: 1, type: "Spell", tags: ["KY"], rarity: "Legendary", base: [null, null], radiant: [null, null] },
  { index: "42.1", name: "KY's Gift", cost: 4, type: "Field Spell", tags: ["KY", "Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "43", name: "AI Slop", cost: 4, type: "Spell", tags: [], rarity: "Legendary", base: [null, null], radiant: [null, null] },
  { index: "44", name: "Simplicity Audit", cost: 2, type: "Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "45", name: "Complexity Audit", cost: 2, type: "Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "46", name: "Felinor Flagbearer", cost: 2, type: "Unit", tags: ["Felinor"], rarity: "Legendary", base: [4, 4], radiant: [8, 8] },
  { index: "46.1", name: "Felinor Flagbearer Prime", cost: 2, type: "Unit", tags: ["Felinor", "Token"], rarity: "Token", base: [5, 5], radiant: [10, 10] },
  { index: "47", name: "Jogg's Box", cost: 4, type: "Spell", tags: [], rarity: "Legendary", base: [null, null], radiant: [null, null] },
  { index: "48", name: "Jlockheed's Lobbyist", cost: 1, type: "Unit", tags: ["Jlockeed"], rarity: "Legendary", base: [0, 3], radiant: [0, 6] },
  { index: "49", name: "Jay Fungus", cost: 2, type: "Unit", tags: [], rarity: "Rare", base: [3, 6], radiant: [6, 12] },
  { index: "50", name: "Adaptive Growth", cost: 1, type: "Spell", tags: [], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "51", name: "Jlockheed's J15 Fighter", cost: 3, type: "Unit", tags: ["Jlockeed"], rarity: "Epic", base: [7, 2], radiant: [14, 4] },
  { index: "52", name: "Jlockheed's Permanent Defense Contract", cost: 2, type: "Spell", tags: ["Jlockeed"], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "53", name: "Book of Tokens", cost: 1, type: "Spell", tags: ["Book"], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "54", name: "Book of Books", cost: 1, type: "Spell", tags: ["Book"], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "55", name: "Book of Greed", cost: 1, type: "Spell", tags: ["Book"], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "56", name: "Book of Pain", cost: 1, type: "Spell", tags: ["Book"], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "57", name: "Book of Stats", cost: 1, type: "Spell", tags: ["Book"], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "58", name: "Fruit Basket", cost: 1, type: "Spell", tags: ["Fruit"], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "59", name: "All Purpose Apple", cost: 1, type: "Spell", tags: ["Fruit"], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "60", name: "Doctors Orders", cost: 1, type: "Field Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "61", name: "Bauble Bubble", cost: 1, type: "Field Spell", tags: ["Fruit"], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "62", name: "KY's Papaya", cost: 1, type: "Spell", tags: ["Fruit", "KY"], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "63", name: "Fruit Tree", cost: 2, type: "Field Spell", tags: ["Fruit"], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "64", name: "Mulch Muncher", cost: 10, type: "Unit", tags: [], rarity: "Rare", base: [9, 9], radiant: [18, 18] },
  { index: "65", name: "Two Grapes", cost: 1, type: "Spell", tags: ["Fruit"], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "65.1", name: "Rotten Grape", cost: 1, type: "Spell", tags: ["Fruit", "Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "65.2", name: "Normal Grape", cost: 1, type: "Spell", tags: ["Fruit", "Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "65.3", name: "Large Grape", cost: 3, type: "Spell", tags: ["Fruit", "Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "65.4", name: "Golden Grape", cost: 1, type: "Spell", tags: ["Fruit", "Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "65.5", name: "Mythic Grape", cost: 0, type: "Spell", tags: ["Fruit", "Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "66", name: "Vine of Grapes", cost: 3, type: "Spell", tags: ["Fruit"], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "67", name: "Pear", cost: 2, type: "Spell", tags: ["Fruit"], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "68", name: "Organic Produce", cost: 4, type: "Field Spell", tags: ["Fruit"], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "69", name: "Buff Billy", cost: "X", type: "Unit", tags: ["Human"], rarity: "Rare", base: [0, 0], radiant: [0, 0] },
  { index: "70", name: "Chaos Machine", cost: 2, type: "Field Spell", tags: [], rarity: "Rare", base: [null, null], radiant: [null, null] },
  { index: "71", name: "Book of Buff", cost: 1, type: "Spell", tags: ["Book"], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "72", name: "Book of Nerf", cost: 1, type: "Spell", tags: ["Book"], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "73", name: "Call to Chaos (Classic+ Edition)", cost: 4, type: "Spell", tags: ["Call to Chaos"], rarity: "Legendary", base: [null, null], radiant: [null, null] },
  { index: "73.1", name: "Classic Golem", cost: 4, type: "Unit", tags: ["Token"], rarity: "Token", base: [10, 10], radiant: [20, 20] },
  { index: "74", name: "Twice Forward One Step Backwards", cost: 2, type: "Field Trap", tags: [], rarity: "Mythic", base: [null, null], radiant: [null, null] },
  { index: "75", name: "J-lease J-Jungle EX-plorer", cost: 2, type: "Unit", tags: [], rarity: "Legendary", base: [5, 5], radiant: [10, 10] },
  { index: "75.1", name: "J-lease J-Jungle EX-plorer Pack", cost: 2, type: "Spell", tags: ["Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "76", name: "Brother Lar", cost: 1, type: "Unit", tags: ["CN", "Human"], rarity: "Rare", base: [1, 1], radiant: [2, 2] },
  { index: "76.1", name: "Brother Ping", cost: 2, type: "Unit", tags: ["CN", "Human", "Token"], rarity: "Token", base: [4, 4], radiant: [8, 8] },
  { index: "77", name: "Anti-Softlock", cost: 2, type: "Spell", tags: [], rarity: "Epic", base: [null, null], radiant: [null, null] },
  { index: "78", name: "Claude's Datacenter", cost: 2, type: "Field Spell", tags: [], rarity: "Legendary", base: [null, null], radiant: [null, null] },
  { index: "T-AI-1", name: "Helpful Assistant", cost: 1, type: "Unit", tags: ["AI", "Token"], rarity: "Token", base: [1, 3], radiant: [2, 6] },
  { index: "T-AI-2", name: "Scaling Law", cost: 2, type: "Unit", tags: ["AI", "Token"], rarity: "Token", base: [2, 2], radiant: [4, 4] },
  { index: "T-AI-3", name: "Hallucination", cost: 0, type: "Spell", tags: ["AI", "Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "T-AI-4", name: "Chain of Thought", cost: 1, type: "Spell", tags: ["AI", "Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "T-AI-5", name: "Autocomplete", cost: 0, type: "Spell", tags: ["AI", "Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "T-AI-6", name: "Datacenter Fire", cost: 2, type: "Spell", tags: ["AI", "Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "T-AI-7", name: "Alignment Tax", cost: 1, type: "Spell", tags: ["AI", "Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "T-AI-8", name: "Rate Limit", cost: 1, type: "Trap", tags: ["AI", "Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "T-AI-9", name: "Refusal", cost: 1, type: "Trap", tags: ["AI", "Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
  { index: "T-AI-10", name: "Fine-Tuning", cost: 2, type: "Field Spell", tags: ["AI", "Token"], rarity: "Token", base: [null, null], radiant: [null, null] },
];

/** Each set's fixture, in catalog order (B2.2). */
const FIXTURES: readonly (readonly [SetName, readonly SpecRow[]])[] = [
  ["Core", CORE],
  ["Classic", CLASSIC],
  ["Classic+", CLASSIC_PLUS],
];

/** BUILD M4-T1 and B2.4, plus the mechanics patch's Plague: the only tags any entry may carry. */
const ALLOWED_TAGS: readonly string[] = [
  "Human",
  "Felinor",
  "KY",
  "CN",
  "Fruit",
  "Call to Chaos",
  "Quickdraw",
  "Jlockeed",
  "Book",
  "Pancake",
  "AI",
  "Plague",
  "Token",
];

/** SPEC §8's and B2.5's distributions. Tokens carry rarity "Token" and are counted apart. */
const RARITY_COUNTS: Readonly<Record<string, Readonly<Record<string, number>>>> = {
  Core: { Common: 32, Rare: 40, Epic: 16, Legendary: 7, Mythic: 5 },
  Classic: { Common: 35, Rare: 26, Epic: 18, Legendary: 10, Mythic: 1 },
  "Classic+": { Common: 13, Rare: 24, Epic: 25, Legendary: 13, Mythic: 3 },
};

/** B2.1: cards and tokens per set. */
const SET_SIZES: Readonly<Record<string, { cards: number; tokens: number }>> = {
  Core: { cards: 100, tokens: 11 },
  Classic: { cards: 90, tokens: 1 },
  "Classic+": { cards: 78, tokens: 38 },
};

/** B2.1's totals, as SET_SIZES sums them: the only count of the whole catalog this file keeps. */
const TOTAL_CARDS = Object.values(SET_SIZES).reduce((n, size) => n + size.cards, 0);
const TOTAL_TOKENS = Object.values(SET_SIZES).reduce((n, size) => n + size.tokens, 0);

/** A set's SET_SIZES row as one number (cards + tokens), for the fixture-row counts below. */
const setSize = (set: string): number => {
  const size = SET_SIZES[set];
  return (size?.cards ?? 0) + (size?.tokens ?? 0);
};

const ENTRIES: readonly CardDef[] = Object.values(CATALOG);

/** An index is unique only within its set (B2.2), so entries are found by `set` and `index`. */
const BY_KEY = new Map<string, CardDef>();
const keyOf = (set: string, index: string): string => `${set} #${index}`;
for (const entry of ENTRIES) {
  const key = keyOf(entry.set, entry.index);
  if (BY_KEY.has(key)) {
    throw new Error(`catalog has two entries with ${key}`);
  }
  BY_KEY.set(key, entry);
}

function entryFor(set: SetName, index: string): CardDef {
  const entry = BY_KEY.get(keyOf(set, index));
  if (entry === undefined) {
    throw new Error(`${keyOf(set, index)} is in the fixture but missing from catalog.json`);
  }
  return entry;
}

/** A face's stat as the fixture spells it: a number, or `null` when the card has no stats. */
function statOf(face: CardFace, field: "attack" | "health"): number | null {
  const value = face[field];
  return value === undefined ? null : value;
}

const show = (value: unknown): string =>
  typeof value === "string" ? value : JSON.stringify(value);

/** "core-043 rarity: expected Rare, got Epic" — the label every failure in this file carries. */
const label = (entry: CardDef, field: string, expected: unknown, actual: unknown): string =>
  `${entry.id} (#${entry.index}) ${field}: expected ${show(expected)}, got ${show(actual)}`;

describe("catalog membership (BUILD M4-T1, B2.1)", () => {
  it("holds B2.1's cards and tokens, every entry, across Core, Classic and Classic+", () => {
    const cards = ENTRIES.filter((entry) => entry.token === false);
    const tokens = ENTRIES.filter((entry) => entry.token === true);
    expect(cards.length, "entries with token: false").toBe(TOTAL_CARDS);
    expect(tokens.length, "entries with token: true").toBe(TOTAL_TOKENS);
    expect(ENTRIES.length, "catalog entries").toBe(TOTAL_CARDS + TOTAL_TOKENS);
    for (const [set, size] of Object.entries(SET_SIZES)) {
      const inSet = ENTRIES.filter((entry) => entry.set === set);
      expect(inSet.filter((entry) => !entry.token).length, `${set} cards`).toBe(size.cards);
      expect(inSet.filter((entry) => entry.token).length, `${set} tokens`).toBe(size.tokens);
    }
  });

  it("has each set's indices 1-N present exactly once, none of them a token", () => {
    const missing: string[] = [];
    for (const [set, size] of Object.entries(SET_SIZES)) {
      for (let n = 1; n <= size.cards; n += 1) {
        const entry = BY_KEY.get(keyOf(set, String(n)));
        if (entry === undefined) {
          missing.push(`${set} #${n} missing`);
        } else if (entry.token !== false) {
          missing.push(`${set} #${n} (${entry.id}) is flagged token: true`);
        }
      }
      // BY_KEY is built with a duplicate guard, so one entry per index is already proved.
      const plain = ENTRIES.filter((entry) => entry.set === set && /^\d+$/.test(entry.index));
      expect(plain.length, `${set} plain numeric indices`).toBe(size.cards);
    }
    expect(missing, "indices").toEqual([]);
  });

  it("has Core's five card-defined tokens, the four shared tokens, The Coin and the Ghoul Token", () => {
    const expected = ["51.1", "65.1", "90.1", "93.1", "95.1", "T-rush", "T-sheep", "T-felinor", "T-bread", "T-coin", "T-ghoul"];
    for (const index of expected) {
      const entry = BY_KEY.get(keyOf("Core", index));
      expect(entry?.index, `token index ${index}`).toBe(index);
      expect(entry?.token, `token index ${index} token flag`).toBe(true);
    }
    const flagged = ENTRIES.filter((entry) => entry.set === "Core" && entry.token === true)
      .map((entry) => entry.index)
      .sort();
    expect(flagged, "Core entries flagged token: true").toEqual([...expected].sort());
  });

  it("has Classic+'s card-defined tokens and the ten AI generated cards (B2.1, B2.3, B8)", () => {
    const tokens = CLASSIC_PLUS.filter((row) => row.rarity === "Token").map((row) => row.index);
    expect(tokens.length).toBe(SET_SIZES["Classic+"]?.tokens);
    expect(tokens.filter((index) => index.startsWith("T-AI-")).length).toBe(10);
    const flagged = ENTRIES.filter((entry) => entry.set === "Classic+" && entry.token).map((entry) => entry.index);
    expect([...flagged].sort()).toEqual([...tokens].sort());
  });

  it("holds no entry its set's fixture does not list", () => {
    const known = new Set(FIXTURES.flatMap(([set, rows]) => rows.map((row) => keyOf(set, row.index))));
    const extra = ENTRIES.filter((entry) => !known.has(keyOf(entry.set, entry.index))).map(
      (entry) => `${entry.id} (${keyOf(entry.set, entry.index)})`,
    );
    expect(extra, "catalog entries with no fixture row").toEqual([]);
    for (const [set, rows] of FIXTURES) {
      expect(rows.length, `${set} fixture rows`).toBe(setSize(set));
    }
  });
});

for (const [set, rows] of FIXTURES) {
  describe(`every ${set} entry equals its fixture row (BUILD M4-T1, M9-T1)`, () => {
    for (const row of rows) {
      it(`#${row.index} ${row.name}`, () => {
        const entry = entryFor(set, row.index);
        const problems: string[] = [];

        if (entry.name !== row.name) problems.push(label(entry, "name", row.name, entry.name));
        if (JSON.stringify(entry.cost) !== JSON.stringify(row.cost)) {
          problems.push(label(entry, "cost", row.cost, entry.cost));
        }
        if (entry.type !== row.type) problems.push(label(entry, "type", row.type, entry.type));
        if (entry.rarity !== row.rarity) {
          problems.push(label(entry, "rarity", row.rarity, entry.rarity));
        }

        // A cell's tag order and the catalog's may differ without either being wrong, so the
        // assertion is on the set.
        const expectedTags = [...row.tags].sort();
        const actualTags = [...entry.tags].sort();
        if (JSON.stringify(expectedTags) !== JSON.stringify(actualTags)) {
          problems.push(label(entry, "tags (as a set)", expectedTags, actualTags));
        }

        const stats: [string, CardFace, StatPair][] = [
          ["base", entry.base, row.base],
          ["radiant", entry.radiant, row.radiant],
        ];
        for (const [face, actual, expected] of stats) {
          if (statOf(actual, "attack") !== expected[0]) {
            problems.push(label(entry, `${face}.attack`, expected[0], statOf(actual, "attack")));
          }
          if (statOf(actual, "health") !== expected[1]) {
            problems.push(label(entry, `${face}.health`, expected[1], statOf(actual, "health")));
          }
        }

        expect(problems.join("\n"), `${set} #${row.index} ${row.name} vs its fixture`).toBe("");
      });
    }
  });
}

describe("rarity distribution (SPEC §8, B2.5, BUILD M4-T1)", () => {
  it("is Core 32/40/16/7/5, Classic 35/26/18/10/1 and Classic+ 13/24/25/13/3 (Common/Rare/Epic/Legendary/Mythic)", () => {
    for (const [set, expected] of Object.entries(RARITY_COUNTS)) {
      const counted: Record<string, number> = {};
      for (const entry of ENTRIES.filter((e) => e.set === set && e.token === false)) {
        counted[entry.rarity] = (counted[entry.rarity] ?? 0) + 1;
      }
      expect(counted, `${set} rarity counts over its non-token cards`).toEqual({ ...expected });
    }
  });

  it("gives every token rarity Token and no card rarity Token", () => {
    const wrong = ENTRIES.filter((entry) => (entry.rarity === "Token") !== entry.token).map((entry) =>
      label(entry, "rarity", entry.token ? "Token" : "not Token", entry.rarity),
    );
    expect(wrong, "token rarity").toEqual([]);
  });

  it("R739 every family shares one rarity (issue #44)", () => {
    const byId = new Map(ENTRIES.map((entry) => [entry.id, entry]));
    const rarityOf = (id: string): string => {
      const entry = byId.get(id);
      if (entry === undefined) throw new Error(`R739 family member ${id} is not in the catalog`);
      return entry.rarity;
    };
    // [family, expected rarity, member ids]. The Jlockeed family is Core's two: the Classic+
    // Jlockheed cards spell it differently and differ in type and effect shape, so the
    // template does not reach them (R739). The House is no Right-house/Wrong-House card.
    const families: readonly (readonly [string, string, readonly string[]])[] = [
      ["Book of ___", "Epic", ["classic-003", "classic-012", "classic-016", "classic-024", "classic-029", "classic-055", "classic-070", "classicplus-053", "classicplus-054", "classicplus-055", "classicplus-056", "classicplus-057", "classicplus-071", "classicplus-072"]],
      ["Call to Chaos", "Legendary", ["core-095", "classicplus-073"]],
      ["___ Glowy Jelly Bean", "Rare", ["core-026", "core-027", "core-028", "core-029"]],
      ["The Power to ___", "Rare", ["classic-020", "classic-081"]],
      ["___ in the System", "Epic", ["classic-018", "classic-025"]],
      ["___ of the Forgotten", "Rare", ["core-040", "classic-001"]],
      ["Rapid ___", "Common", ["core-010", "classic-026"]],
      ["___ Shroom", "Epic", ["classicplus-001", "classicplus-002"]],
      ["Plague Bringer", "Rare", ["classic-061", "classic-076"]],
      ["Jlockeed ___", "Common", ["core-013", "core-014"]],
      ["Right-house defender / Wrong-House Attacker", "Common", ["core-003", "classicplus-006"]],
    ];
    for (const [family, rarity, members] of families) {
      for (const id of members) {
        expect(rarityOf(id), `R739 ${family} member ${id}`).toBe(rarity);
      }
    }
  });

  it("R739 a bigger version of an effect is never a lower rarity (Pile On over Call to Arms)", () => {
    const rank = ["Common", "Rare", "Epic", "Legendary", "Mythic"];
    const pileOn = CATALOG["classic-060"];
    const callToArms = CATALOG["core-069"];
    expect(rank.indexOf(pileOn?.rarity ?? "")).toBeGreaterThanOrEqual(rank.indexOf(callToArms?.rarity ?? ""));
  });

  it("R739 one name names one card", () => {
    const seen = new Map<string, string>();
    const dupes: string[] = [];
    for (const entry of ENTRIES.filter((entry) => entry.token === false)) {
      const first = seen.get(entry.name);
      if (first === undefined) seen.set(entry.name, entry.id);
      else dupes.push(`${entry.name} (${first}, ${entry.id})`);
    }
    expect(dupes, "cards sharing a name").toEqual([]);
  });

  it("B2.5 prints the designer's rarity on the Classic+ tokens he rated, never on a card or an AI generated card", () => {
    const printed = Object.fromEntries(
      ENTRIES.filter((entry) => entry.printedRarity !== undefined).map((entry) => [entry.index, entry.printedRarity]),
    );
    const legendary = [...CLASSIC_PLUS.filter((row) => /^(12|19)\./.test(row.index)).map((row) => row.index), "42.1", "46.1", "73.1", "75.1"];
    const expected: Record<string, string> = {
      ...Object.fromEntries(legendary.map((index) => [index, "Legendary"])),
      "32.1": "Epic",
      "32.2": "Epic",
      "32.3": "Epic",
      "38.1": "Epic",
      "36.1": "Rare",
      "76.1": "Rare",
      "65.1": "Common",
      "65.2": "Common",
      "65.3": "Rare",
      "65.4": "Legendary",
      "65.5": "Mythic",
    };
    expect(printed).toEqual(expected);
    expect(ENTRIES.filter((entry) => entry.printedRarity !== undefined).every((entry) => entry.set === "Classic+" && entry.token)).toBe(true);
  });
});

describe("faces that change more than text (B2.7, E40)", () => {
  it("gives Blood Moon's Radiant face a type of its own, a Field Trap, and no other face one", () => {
    const typed = ENTRIES.flatMap((entry) =>
      (["base", "radiant"] as const).flatMap((face) => (entry[face].type === undefined ? [] : [`${entry.id} ${face} ${entry[face].type}`])),
    );
    expect(typed).toEqual(["classicplus-022 radiant Field Trap"]);
    expect(CATALOG["classicplus-022"]?.type).toBe("Trap");
  });

  it("prints Buff Billy's [3X/3X] and [7X/7X] as X multiples on a 0/0 face, and no other face has them", () => {
    const withX = ENTRIES.filter((entry) => entry.base.xStats !== undefined || entry.radiant.xStats !== undefined).map((entry) => entry.id);
    expect(withX).toEqual(["classicplus-069"]);
    const billy = CATALOG["classicplus-069"];
    expect(billy?.cost).toBe("X");
    expect(billy?.base.xStats).toEqual({ attack: 3, health: 3 });
    expect(billy?.radiant.xStats).toEqual({ attack: 7, health: 7 });
  });
});


describe("names (R381, B2.8)", () => {
  it("R381 gives no two cards one name: Classic #55 is Book of Wildfire and #72 Grand Counterspell", () => {
    const names = new Map<string, string[]>();
    for (const entry of ENTRIES) names.set(entry.name, [...(names.get(entry.name) ?? []), entry.id]);
    expect([...names].filter(([, ids]) => ids.length > 1)).toEqual([]);
    expect(CATALOG["classic-016"]?.name).toBe("Book of Flame");
    expect(CATALOG["classic-055"]?.name).toBe("Book of Wildfire");
    expect(CATALOG["classic-017"]?.name).toBe("Counterspell");
    expect(CATALOG["classic-072"]?.name).toBe("Grand Counterspell");
    // Book of Wildfire is its own card: the designer's text, not a copy of Book of Flame's.
    expect(CATALOG["classic-055"]?.rarity).toBe("Epic");
    expect(CATALOG["classic-055"]?.tags).toEqual(["Book"]);
  });

  it("R381 keeps the names that are rules words — Exile, Burn, Echo, Recycle — as the designer named them", () => {
    expect(["classic-010", "classic-036", "classic-057", "classic-030"].map((id) => CATALOG[id]?.name)).toEqual([
      "Exile",
      "Burn",
      "Echo",
      "Recycle",
    ]);
  });
});

describe("tag vocabulary (BUILD M4-T1, B2.4)", () => {
  it("uses only Human, Felinor, KY, CN, Fruit, Call to Chaos, Quickdraw, Jlockeed, Book, Pancake, AI, Plague and Token", () => {
    const wrong: string[] = [];
    for (const entry of ENTRIES) {
      for (const tag of entry.tags) {
        if (!ALLOWED_TAGS.includes(tag)) {
          wrong.push(label(entry, "tags", `one of ${ALLOWED_TAGS.join(", ")}`, tag));
        }
      }
    }
    expect(wrong, "tags outside the allowed vocabulary").toEqual([]);
  });
});

describe("the Jlockeed tag (SPEC §5, §8, R278, B2.4)", () => {
  it("R278 tags Core #13 and #14, the three Classic+ Jlockheed cards, #48, #51 and #52, and Classic #4 Palantir, and no other entry", () => {
    const tagged = ENTRIES.filter((entry) => entry.tags.includes("Jlockeed")).map((entry) => entry.id);
    expect(tagged.sort(), "entries tagged Jlockeed").toEqual([
      "classic-004",
      "classicplus-048",
      "classicplus-051",
      "classicplus-052",
      "core-013",
      "core-014",
    ]);
    // The tag follows the name: every entry whose name says Jlockeed, or the Classic+ spelling
    // Jlockheed, carries it — one faction under two spellings (B2.4) — plus Classic #4 Palantir,
    // which the balance patch inducted without renaming it.
    const named = ENTRIES.filter((entry) => /Jlockh?eed/.test(entry.name)).map((entry) => entry.id);
    expect(named.sort(), "entries whose name names Jlockeed or Jlockheed").toEqual(
      tagged.filter((id) => id !== "classic-004").sort(),
    );
  });
});

describe("every entry has a radiant face of its own (SPEC §5.2, R276)", () => {
  it("gives no entry a radiant face identical to its base face — the five that had none included", () => {
    // R349: a card that prints no Radiant form has the fallback as its Radiant face, which doubles
    // its stats — for an X/X token, the X it is summoned with (`layers.wornStatsOverride`) — so it
    // is the one entry whose printed faces may read alike. It is marked as such, and only a Unit.
    // A face's text is read with its own `params` values filled in (B3.4 rule 5): "Heal a target
    // {heal}." prints 9 on one face and 18 on the other.
    const printed = (entry: CardDef, face: "base" | "radiant"): string =>
      JSON.stringify({ ...entry[face], text: fillParams(entry, face) });
    // R674: Glitch is blank on both faces, the one entry exempt; its client face corrupts whatever it shows.
    const same = ENTRIES.filter(
      (entry) =>
        entry.radiantFallback !== true && entry.id !== GLITCH_DEF_ID && printed(entry, "radiant") === printed(entry, "base"),
    ).map((entry) => `${entry.id} (#${entry.index}) radiant is a copy of base`);
    expect(same, "entries whose radiant face is identical to base").toEqual([]);
    expect(ENTRIES.filter((entry) => entry.radiantFallback === true).map((entry) => entry.id)).toEqual(["core-t-ghoul"]);
  });
});

describe("no face prints Taunt beside Indestructible (SPEC §6.1, R347)", () => {
  // R347 keeps Taunt off an Indestructible unit whatever prints it, so a face printing both would
  // print a Taunt it never has. Patch v0.1.1 dropped Indestructible from the two faces that did, #55
  // and #56 radiant (issue #27), and none may print both again.
  it("R347 no Unit face prints both", () => {
    const both = ENTRIES.filter((entry) => entry.type === "Unit").flatMap((entry) =>
      (["base", "radiant"] as const).flatMap((face) => {
        const kinds = entry[face].keywords.map((keyword) => keyword.kind);
        return kinds.includes("Taunt") && kinds.includes("Indestructible") ? [`${entry.id} ${face}`] : [];
      }),
    );
    expect(both).toEqual([]);
  });
});
