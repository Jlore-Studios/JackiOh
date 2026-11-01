// The keyword glossary the inspect views print beside a card (docs/polish/6-cards.md, Surface B).
//
// Every `rule` is reviewed data in a player's words (#133); each entry names its spec `section` (§6.1
// unit keywords, §6.2 triggers, §6.3 verbs, §5.2 Radiant) and, if the spec row is spelled differently,
// its `row`. rules.test.ts checks every row of spec/06-keywords.md's tables is a term here or rules-only,
// never a sentence: a changed spec rule needs a hand edit here. Owed differences: players' words (R373,
// #45), no ruling's number (R512), and a row's own **Ruling** over its Rule column (Cry; RULED_TERMS).
// SHORT_REMINDERS holds two short lines (R500; Tribute leaves out R41's backrow clause, R428 removed).
// `label` is what rules.ts's tokenizer matches, case-sensitively; `aliases` are other spellings.
// What bolds (R910000, #910): every §6.2 trigger, §6.3 verb and Radiant used as that term, each
// inflection an alias; look-alikes stay plain (PLAIN_SPELLINGS), as do types, zones, stats and rules-only
// rows; a §6.1 keyword bolds only as the keyword a card has. "Can't attack or be attacked" (R446) has no entry.

import type { KeywordKind } from "@jackioh/shared";

export type TriggerTermId =
  | "Cry"
  | "Death"
  | "Start of turn"
  | "End of turn"
  | "Start of game"
  | "Once per turn"
  | "Aura"
  | "Combo"
  | "Echo"
  | "Cast on draw"
  | "Quickdraw"
  | "Float"
  | "Activate"
  | "When this awakens"
  | "When launched";
export type VerbTermId =
  | "Discover"
  | "Tribute"
  | "Embiggened"
  | "Recruit"
  | "Fuse"
  | "Transform"
  | "Vanilla"
  | "Lock"
  | "Choose one"
  | "Radiant"
  | "Counter"
  | "Steal"
  | "Unlock"
  | "Flicker"
  | "Bounce"
  | "Nerf"
  | "Buff"
  | "Plague Counter"
  | "Set health"
  | "Redirect"
  | "End the turn"
  | "Trigger a Cry"
  | "Look at a hand"
  | "Allure"
  | "Freeze"
  | "Awaken"
  | "Launch"
  | "Jade Counter"
  | "Exile"
  | "Hand size"
  | "Mark in a hand";
/** §6.1's unit statuses that are not keyword kinds (B5 E35). */
export type StatusTermId =
  | "Can't be attacked"
  | "Only Units in this lane can attack this"
  | "Berserk"
  | "Frozen"
  | "Dormant"
  | "Combat keyword";
export type GlossaryTermId = KeywordKind | StatusTermId | TriggerTermId | VerbTermId;
export type GlossaryEntry = {
  id: GlossaryTermId;
  label: string;
  /** The rule in a player's words: reviewed data (see the header). */
  rule: string;
  section: "§5.2" | "§6.1" | "§6.2" | "§6.3";
  /** The name of its row in the spec's §6 table, where that is not `id` ("Armor X" for Armor). */
  row?: string;
  /** Other spellings matched in rules text. */
  aliases: readonly string[];
};

const NONE: readonly string[] = [];

/**
 * R500: the two glossary rows written as short reminders instead of the spec's full text. Cry states
 * §6.2's ruling (played or cast, never another way onto the field); Tribute states §6.3's cost.
 */
export const SHORT_REMINDERS = {
  Cry: "When you play this card or an effect casts it. Never when it enters play otherwise",
  Tribute: "Playing this also costs X of your Units, which go to the graveyard",
} as const satisfies Readonly<Partial<Record<string, string>>>;

/** The terms whose `rule` is a short reminder (R500), in SHORT_REMINDERS. */
export const SHORT_TERMS: readonly (keyof typeof SHORT_REMINDERS)[] = ["Cry", "Tribute"];

type Extra = { row?: string; aliases?: readonly string[] };

function entry(id: GlossaryTermId, section: GlossaryEntry["section"], rule: string, extra: Extra): GlossaryEntry {
  const built: GlossaryEntry = { id, label: id, rule, section, aliases: extra.aliases ?? NONE };
  if (extra.row !== undefined) built.row = extra.row;
  return built;
}

function keyword(id: KeywordKind, rule: string, extra: Extra = {}): GlossaryEntry {
  return entry(id, "§6.1", rule, extra);
}

function status(id: StatusTermId, rule: string, extra: Extra = {}): GlossaryEntry {
  return entry(id, "§6.1", rule, extra);
}

function trigger(id: TriggerTermId, rule: string, extra: Extra = {}): GlossaryEntry {
  return entry(id, "§6.2", rule, extra);
}

function verb(id: VerbTermId, rule: string, extra: Extra = {}): GlossaryEntry {
  return entry(id, "§6.3", rule, extra);
}

export const GLOSSARY: Readonly<Record<GlossaryTermId, GlossaryEntry>> = {
  // §6.1 Unit keywords
  Taunt: keyword("Taunt", "Enemies must attack Taunt units first"),
  Rush: keyword("Rush", "May attack units, not heroes, on summon turn"),
  Charge: keyword("Charge", "May attack units and heroes on summon turn"),
  "First Strike": keyword("First Strike", "Deals damage before non-First-Strike units"),
  Poisonous: keyword("Poisonous", "Destroys any unit it damages"),
  Lifesteal: keyword("Lifesteal", "Damage dealt heals your hero"),
  Reborn: keyword("Reborn", "First death: return at 1 health without Reborn"),
  "Divine Shield": keyword("Divine Shield", "Negate the first damage instance, then lose it"),
  Trample: keyword("Trample", "Excess damage to a unit hits its hero; a unit tramples only when it attacks"),
  Cleave: keyword("Cleave", "Also damages units adjacent to the target"),
  Pierce: keyword("Pierce", "Its damage ignores Armor"),
  // R2179000: Indestructible is also never in Defense Position and never has Taunt (R347).
  Indestructible: keyword(
    "Indestructible",
    "Can't be destroyed or damaged, can't be in Defense Position and never has Taunt; can be exiled or tributed",
  ),
  Immutable: keyword(
    "Immutable",
    "Text can't be changed or transformed. While in play, Spells, Field Spells and Field Traps can't target it",
  ),
  Stack: keyword("Stack", "May be played onto an occupied zone"),
  "Can't attack": keyword("Can't attack", "Cannot declare attacks"),
  Armor: keyword("Armor", "Reduce each damage instance by X", { row: "Armor X" }),
  // R1438, R1440: a coin flip is a luck-based roll, and Lucky given to a card adds to its own.
  Lucky: keyword(
    "Lucky",
    "Repeat a luck-based roll X extra times, keep the best; a coin flip is one, heads its better side unless the card says otherwise; Lucky given to it adds to X",
    { row: "Lucky X" },
  ),
  // docs/classic-sets.md B3.1, B3.3, B5 E6 and E35; R383, R385.
  Animated: keyword(
    "Animated",
    "A Field Spell or Trap with stats that becomes a Unit in an open unit zone when it fires or enters the field, or as it gains stats",
  ),
  "Animated on your turn": keyword("Animated on your turn", "A Unit on your turn; back in its backrow on your opponent's"),
  Brittle: keyword(
    "Brittle",
    "At the start of turn, after a full turn cycle, the count drops by 1; at 0 the card is destroyed",
    { row: "Brittle X" },
  ),
  "Spell Damage": keyword("Spell Damage", "Your Spells deal X more damage per hit", { row: "Spell Damage X" }),
  "Immune to Spells": keyword("Immune to Spells", "Spells can't target it or affect it"),
  // R636, R637: the keyword rules patch (§6.1).
  Windfury: keyword("Windfury", "Can attack twice each turn"),
  // R2179001: Temporary X counts down at each end of its owner's turn; a plain Temporary is Temporary 1.
  Temporary: keyword(
    "Temporary",
    "At the end of its owner's turn the count drops by 1; at 0 the card is discarded from their hand. Temporary alone is Temporary 1",
    { row: "Temporary X" },
  ),
  // #181, R49: Deft, #45 Deft Duelist's keyword.
  Deft: keyword("Deft", "Can attack and switch position in the same turn"),
  // R1220: Untributable, Meditative #80 Aluneth's keyword.
  Untributable: keyword("Untributable", "Can't be Tributed, by any cost or effect"),
  // #532, R1086: Equip, M #65 and M #67's keyword.
  Equip: keyword("Equip", "May be played onto one of your Units, fusing into it"),
  // #783: five keywords no card prints yet (R783000, R783002, R783004, R783010, R783012).
  Ranged: keyword("Ranged", "Takes no damage back when it attacks, unless the defender is Ranged"),
  Flying: keyword("Flying", "Only Units with Flying or Ranged can attack it, unless it is in Defense Position. It may be played into your backrow as well as into a unit zone"),
  Cloaked: keyword("Cloaked", "Can be played face-down as a Trap. Activate: Reveal"),
  Crater: keyword("Crater", "Locks its zone when it arrives as a Unit, or when it is revealed as a Trap"),
  Flashback: keyword("Flashback", "Can be played from your graveyard once, at its cost"),
  // #1838, R1838000: Tradable, Meditative #53 and #98's keyword, Stinkyscape's too.
  Tradable: keyword(
    "Tradable",
    "In your main phase, put it from your hand into your deck at a random position and draw a card, at no mana. One Trade per turn",
  ),
  // #1852 (R1852000–R1852007): Stinkyscape's two keywords, and When launched and Launch below.
  "Starship Piece": keyword(
    "Starship Piece",
    "When it dies, a copy of it fuses into your Dormant Starship, which is summoned first if you have none",
  ),
  Starship: keyword("Starship", "A launched Starship, built from the Starship Pieces fused into it"),
  // Issue #2657 (R2657000): the successor to the status "Cannot be in Defense Position".
  "Can't Defend": keyword("Can't Defend", "Never in Defense Position"),
  // §6.1's statuses that are not keyword kinds (patch v0.2.0, B5 E35; R512, see the header).
  "Can't be attacked": status("Can't be attacked", "No attack may target it"),
  "Only Units in this lane can attack this": status(
    "Only Units in this lane can attack this",
    "An attack on it is legal only from the enemy unit zone of its own lane",
  ),
  Berserk: status("Berserk", "While Berserk, at the start and end of its controller's turn this attacks its own hero"),
  // R1174000 (#1174): a status set by Freeze (§6.3); R2246000: it stops an Activate too.
  Frozen: status(
    "Frozen",
    "Can't attack, switch position or use its Activate until it thaws: it loses its next chance to attack or switch, and thaws at the end of the turn it could have done it",
  ),
  // R1839000 (#1839): Stinkyscape's status, which no effect reaches until it awakens. A card under a
  // Stack pile's top is "buried" to a player, never Dormant (§3.2).
  Dormant: status(
    "Dormant",
    "On the field and filling its zone, but out of play until it awakens: it can't attack, be attacked or be targeted, no effect reaches it, and its auras and triggers are off",
  ),
  // R781005: the pool's name, printed lower-case ("random combat keywords"); Armor 1 and Spell Damage
  // +1 are out of it (R1025000), and Charge and Indestructible (#1615, R1615000).
  "Combat keyword": status(
    "Combat keyword",
    "One of the keywords Units fight with: Taunt, Rush, First Strike, Flying, Ranged, Poisonous, Reborn, Divine Shield, Windfury, Cleave, Deft, Lifesteal, Pierce, Trample",
    { aliases: ["combat keywords", "combat keyword"] },
  ),

  // §6.2 Triggers and timing words
  // §6.2's ruling, not its Rule column, and short (R500; see the header).
  Cry: trigger("Cry", SHORT_REMINDERS.Cry),
  Death: trigger("Death", "When sent from the field to the GY"),
  // Issue #814: the lowercase second half of a compound trigger ("Cry and start of turn",
  // "Start of turn and end of turn") is the same term, and bolds as one. R366 keeps the catalog's
  // spelling, so the aliases live here, not there.
  "Start of turn": trigger("Start of turn", "Controller's turn start, before the draw", {
    aliases: ["start of turn"],
  }),
  "End of turn": trigger("End of turn", "Controller's turn end, before cleanup", {
    aliases: ["end of turn"],
  }),
  "Start of game": trigger("Start of game", "After mulligan, before turn 1", { row: "Start of Game" }),
  "Once per turn": trigger("Once per turn", "Activated ability limit", { row: "Once per Turn" }),
  Aura: trigger("Aura", "Effect while in play"),
  Combo: trigger("Combo", "Extra effect if X or more cards were played earlier this turn", { row: "Combo X" }),
  Echo: trigger("Echo", "Recast this card X more times", { row: "Echo X" }),
  "Cast on draw": trigger("Cast on draw", "Plays itself on draw, then draw again"),
  Quickdraw: trigger("Quickdraw", "Starts in your opening hand instead of a draw"),
  // #1563 (R1563000): "Floats", "the Float" and "Float reduces cost" all bold as this term.
  Float: trigger(
    "Float",
    'The mana a player leaves unspent when their turn ends. "Float reduces cost": whenever you Float, this costs that much less',
    { aliases: ["Floats", "Floated"] },
  ),
  // B3.2, R384: "Activate", "Activate 2", "Activate ♾️" are this one term.
  Activate: trigger(
    "Activate",
    'Once per turn on your turn, click the card to do an effect; "Activate X" up to X times per turn, "Activate ♾️" any number of times',
    // #2521: "Activated", ZolarKeth's "A University can be Activated at most once each turn" (R2521003).
    { row: "Activate / Activate X / Activate ♾️", aliases: ["Activated"] },
  ),
  // R1839002: a Dormant card's own trigger.
  "When this awakens": trigger("When this awakens", "When this Dormant card awakens"),
  // R1852003: a Starship Piece's trigger, which runs once it is part of a Starship that launches.
  "When launched": trigger("When launched", "When the Starship this is fused into launches, after its Cries"),

  // §6.3 Actions and verbs
  Discover: verb("Discover", "Choose 1 of 3 options"),
  // Short (R500; see the header).
  Tribute: verb("Tribute", SHORT_REMINDERS.Tribute, { row: "Tribute X", aliases: ["Tributes"] }),
  // R1175000: an embiggen card's text labels its Embiggened version "Embiggened (N):", so the entry is
  // the word it prints, on §6.3's Embiggen row, and shows beside every embiggen card.
  Embiggened: verb(
    "Embiggened",
    "An embiggen card has two prices. Its normal version costs the cheaper one; its Embiggened version costs more and does more.",
    { row: "Embiggen" },
  ),
  Recruit: verb("Recruit", "Summon from deck, scanning top down"),
  // R910000, R1857000: the lower-case and past spellings the cards print mid-sentence.
  Fuse: verb("Fuse", "Combine effects, stats and cost, cost capped at 4", { aliases: ["fuse", "fused", "fuses"] }),
  Transform: verb("Transform", "Replace a card with another in place", { aliases: ["transform", "transforms"] }),
  Vanilla: verb("Vanilla", "Remove a unit's text and return it to its printed stats"),
  Lock: verb("Lock", "Zone can't be played into", { aliases: ["Locked"] }),
  "Choose one": verb("Choose one", "Modal effect"),
  // The §6.3 rows card texts print (R512).
  Counter: verb("Counter", "Cancel a card being played or cast"),
  Steal: verb("Steal", "Take control", { aliases: ["steal"] }),
  Unlock: verb("Unlock", "A Locked zone accepts plays again"),
  Flicker: verb("Flicker", "The card leaves the field then re-enters the same zone at once"),
  // R692: the printed word for a permanent's return from the field to its controller's hand (R746, R747).
  Bounce: verb("Bounce", "Return to controller's hand", { aliases: ["Bounced", "bounced"] }),
  // R386, R512, R1320: two rows of §6.3, one word each (the engine's Degrade and Upgrade).
  Nerf: verb("Nerf", "Weaken a card: one change per application", { aliases: ["Nerfs"] }),
  Buff: verb("Buff", "Strengthen a card: one change per application", { aliases: ["Buffs"] }),
  "Plague Counter": verb("Plague Counter", "Counter on a permanent, any number, reset on leaving the field", {
    aliases: ["Plague Counters"],
  }),
  "Set health": verb("Set health", "A hero's health becomes N"),
  Redirect: verb("Redirect", "A hit, a chosen target or an attack moves to another", { aliases: ["redirected"] }),
  "End the turn": verb("End the turn", "The turn ends from an effect", { aliases: ["end the turn"] }),
  "Trigger a Cry": verb("Trigger a Cry", "Run a unit's Cry again"),
  "Look at a hand": verb("Look at a hand", "See the opponent's hand in a prompt", {
    aliases: ["Look at your opponent's hand"],
  }),
  Allure: verb(
    "Allure",
    "Steal the marked enemy permanents at the start of your next turn; one you have no room for is destroyed",
    { aliases: ["Allured"] },
  ),
  // R1174000, R910000: the verb that sets Frozen (M+ #24 Bob prints it, #1185), bold in the spellings a card prints.
  Freeze: verb("Freeze", "The Unit loses its next attack or position switch, and thaws at the end of the turn it could have made one", {
    aliases: ["freeze", "freezes"],
  }),
  // R1839002: the one verb that reaches a Dormant card.
  Awaken: verb("Awaken", "A Dormant card awakens now"),
  // R1852003: a Dormant Starship's awakening, which fires every Cry fused into it.
  Launch: verb("Launch", "Your Dormant Starship awakens: every Cry fused into it fires, then every When launched", {
    aliases: ["launch", "launches"],
  }),
  "Jade Counter": verb("Jade Counter", "A public count each player has, that only rises"),
  // R1384: §6.3's words the Meditative cards print; "hand size" matches as #79 prints it. Grant tag
  // stays rules vocabulary: #35 says it in plain words.
  Exile: verb("Exile", "Send to the exile pile, from anywhere; it triggers no Death", {
    aliases: ["exile", "exiles", "exiled"],
  }),
  "Hand size": verb("Hand size", "How many cards a hand holds, set for the rest of the game", { aliases: ["hand size"] }),
  "Mark in a hand": verb("Mark in a hand", "Mark cards in a hand; a later effect acts on the ones still there", {
    aliases: ["Mark", "marked"],
  }),

  // §5.2 Radiant: §6.3's "Make Radiant" rule, then §5.2's sentence about a card in hand or deck.
  Radiant: entry(
    "Radiant",
    "§5.2",
    "Raise a card to its Radiant form. In hand or deck: cost unchanged, stats and text swap to the radiant form.",
    {},
  ),
};

/**
 * R910000: spellings that hold a term's word but name something else, so the tokenizer prints them
 * plain: the exile pile, which reads like Exile ("your exile", "from exile", Lizard's Breath's "Exile,"; #2179).
 */
export const PLAIN_SPELLINGS: readonly string[] = [
  "your exile",
  "their exile",
  "from exile",
  "graveyard or exile",
  "Exile, deal",
];

/** The terms whose `rule` states their spec row's ruling instead of its Rule column (see the header). */
export const RULED_TERMS: readonly GlossaryTermId[] = ["Cry"];

/** Two-letter marks rather than glyphs, so a snapshot is stable and a screen reader gets the name. */
export const KEYWORD_MARK: Readonly<Record<KeywordKind, string>> = {
  Taunt: "TA",
  Rush: "RU",
  Charge: "CH",
  "First Strike": "FS",
  Poisonous: "PO",
  Lifesteal: "LS",
  Reborn: "RB",
  "Divine Shield": "DS",
  Trample: "TR",
  Cleave: "CL",
  Pierce: "PI",
  Indestructible: "ND",
  Immutable: "IM",
  Stack: "ST",
  "Can't attack": "NA",
  Armor: "AR",
  Lucky: "LK",
  Animated: "AN",
  "Animated on your turn": "AT",
  Brittle: "BR",
  "Spell Damage": "SD",
  "Immune to Spells": "IS",
  Windfury: "WF",
  Temporary: "TE",
  Deft: "DE",
  Untributable: "UT",
  Equip: "EQ",
  Ranged: "RG",
  Flying: "FL",
  Cloaked: "CK",
  Crater: "CR",
  Flashback: "FB",
  Tradable: "TD",
  "Starship Piece": "SP",
  Starship: "SS",
  "Can't Defend": "CD",
};
