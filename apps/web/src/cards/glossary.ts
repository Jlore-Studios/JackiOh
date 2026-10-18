// The keyword glossary the inspect views print beside a card (docs/polish/6-cards.md, Surface B).
//
// Every `rule` is reviewed data, written once in the words a player reads (v0.3.0, #133). Until
// then the table was built from SPEC's "Rule" column at load time, rewritten by regexes; each line
// below is exactly what that produced, copied once, so no player-visible text changed. The link to
// the spec is structural: each entry names its section (`section`: §6.1 unit keywords, §6.2
// triggers and timing words, §6.3 verbs, §5.2 Radiant) and, where its spec row is spelled
// differently from its id ("Armor X" for Armor), that row's name (`row`). rules.test.ts checks
// that every row of spec/06-keywords.md's three tables is a term here or a rules-only row, and
// never compares a sentence, so a reworded spec row fails nothing. If a spec rule changes what a
// player should read, change the line here by hand.
//
// The lines follow the spec's rules, with three kinds of difference a player is owed:
// - Players' words (v0.1.1, R373): the rules' "library" is the Deck and "sacrifice" is a Tribute;
//   the retired turn-trigger prose reads label-style ("At the start of turn"), and "backrow zone"
//   reads "backrow" (patch v0.2.1, issue #45).
// - No ruling's number: a spec row's citation ("(R384)") is no player's word (R512).
// - A row's own **Ruling** where it overrides its Rule column: Cry's Rule column says "enters the
//   field for the first time", but its ruling in the same §6.2 row says it fires only when the card
//   is played from hand or cast, and never for a copy, a Recruit, a Reborn, a token or a Transform
//   result (§6.3's Summon and Recruit rows agree). RULED_TERMS lists these rows.
//
// And two rows are short reminders rather than the spec's full text (R500, patch v0.2.0): Cry's
// ruling and Tribute's Rule column are long enough to crowd a card's inspect view, so the glossary
// says each in one short line that is still true to the rule (Tribute's no longer mentions R41's
// backrow clause, which R428 removed). SHORT_REMINDERS holds the two lines.
//
// `label` is what the rules-text tokenizer (rules.ts) looks for, case-sensitively, and `aliases`
// are the other spellings cards use.
//
// What bolds (R910000, issue #910): every §6.2 trigger and §6.3 verb, and Radiant, bolds wherever a
// card uses it as that term, in every case and inflection the cards print ("exile it", "exiled",
// "steal it", "Locked", "Tributes", "Buffs and Nerfs"), each such spelling an alias below. A word
// spelled like a term that names something else stays plain, listed in PLAIN_SPELLINGS: the exile
// pile ("your exile", Lizard's Breath's "Exile," pile). A
// §6.1 keyword bolds as the keyword a card has, so "you stack onto this" stays plain. Card types,
// zones, stats, positions, rarities and the §6.3 rows rules.test.ts keeps rules-only (Summon, Draw,
// Discard, Destroy, Damage, Heal, Add to hand, Shuffle into) stay plain, as Hearthstone prints them. rules.test.ts fails on
// a card that prints a trigger or verb that neither bolds nor is a plain spelling.
//
// Patch v0.2.1 (issue #45) retired spelled variants ("Start of your turn", "End of your turn",
// "Start of Game", "Once per Turn", "Cannot be in Defense Position", "Trigger the Cry", "Set a
// hero's health", "End your turn"), keeping only grammatical plurals ("Plague Counters") and
// specific prompts ("Look at your opponent's hand").
//
// Patch v0.2.0 adds the §6 rows its card texts print (R512):
// - §6.1's statuses that are not keyword kinds (StatusTermId): "Can't be in Defense Position" (the
//   catalog's spelling of the spec's "Cannot be in Defense Position", its `row`), "Can't be
//   attacked", "Only Units in this lane can attack this" and Berserk. "Can't attack or be attacked"
//   gets no entry: no card prints it since patch v0.2.3 (it is the rule of a Unit a carrier holds,
//   R446), and matching stays case-sensitive, as "may tribute enemy units" stays plain words.
// - §6.2's Activate, one entry for "Activate", "Activate X" and "Activate ♾️" (the tokenizer takes
//   the count or the ♾️ with the label, as it takes "Armor 2").
// - Balance patch 1 adds §6.3's Bounce (R692), with "Bounced" as its alias.
// - §6.3's Counter, Steal, Unlock, Flicker, Plague Counter (and "Plague Counters"), Redirect, Set health,
//   End the turn, Trigger a Cry and Look at a hand ("Look at your opponent's hand"): each label is
//   the spec's row name and each alias the words cards print. Only capitalised spellings matched, so
//   "steal it" mid-sentence stayed plain, until R910000 gave the lower-case spellings aliases.
// - §6.3's Nerf and Buff, one row and one entry each, since a card prints one word or the other. Patch
//   v0.3.4 (R1320) renamed them from Degrade and Upgrade, the names the engine keeps; a plain stat buff
//   ("+2/+2") stays plain words, since matching is case-sensitive and "buff" is lower-case there.
// - The Meditative set's words (R1384): §6.3's Exile, Hand size (matched in lower case, as #79 prints
//   it) and Mark in a hand (printed "Mark"). Grant tag stays rules vocabulary: #35 says it in plain
//   words, so no card prints the row's name.
//
// Issue #1174 (R1174000) adds §6.1's Frozen, a status that is no keyword kind, and §6.3's Freeze,
// with "freeze" and "freezes" as its aliases. No card prints either yet (M+ #24 Bob, #1185, will).

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
  | "Activate";
export type VerbTermId =
  | "Discover"
  | "Tribute"
  | "Embiggen"
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
  | "Jade Counter"
  | "Exile"
  | "Hand size"
  | "Mark in a hand";
/** §6.1's unit statuses that are not keyword kinds (patch v0.2.0, B5 E35). */
export type StatusTermId =
  | "Can't be in Defense Position"
  | "Can't be attacked"
  | "Only Units in this lane can attack this"
  | "Berserk"
  | "Frozen"
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
  Trample: keyword("Trample", "Excess damage hits the hero"),
  Cleave: keyword("Cleave", "Also damages units adjacent to the target"),
  Pierce: keyword("Pierce", "Its damage ignores Armor"),
  Indestructible: keyword("Indestructible", "Can't be destroyed or damaged; can be exiled or tributed"),
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
  // Patch v0.2.0's keywords (docs/classic-sets.md B3.1, B3.3, B5 E6 and E35; R383, R385).
  Animated: keyword(
    "Animated",
    "A Field Spell or Trap that becomes a Unit in an open unit zone when it fires or enters the field",
  ),
  "Animated on your turn": keyword("Animated on your turn", "A Unit on your turn; back in its backrow on your opponent's"),
  Brittle: keyword(
    "Brittle",
    "At the start of turn, after a full turn cycle, the count drops by 1; at 0 the card is destroyed",
    { row: "Brittle X" },
  ),
  "Spell Damage": keyword("Spell Damage", "Your Spells deal X more damage per hit", { row: "Spell Damage X" }),
  "Immune to Spells": keyword("Immune to Spells", "Spells can't target it or affect it"),
  "Immune to tribal tag based hate": keyword(
    "Immune to tribal tag based hate",
    "Harmful effects that pick cards by a tribal tag can't target or affect it",
  ),
  // R636, R637: the keyword rules patch (§6.1).
  Windfury: keyword("Windfury", "Can attack twice each turn"),
  Temporary: keyword("Temporary", "Discarded from its owner's hand at the end of their turn"),
  // Patch v0.2.4 (issue #181, R49): Deft, #45 Deft Duelist's keyword.
  Deft: keyword("Deft", "Can attack and switch position in the same turn"),
  // R1220: Untributable, Meditative #80 Aluneth's keyword.
  Untributable: keyword("Untributable", "Can't be Tributed, by any cost or effect"),
  // Meditative batch 16 (issue #532, R1086): Equip, M #65 and M #67's keyword.
  Equip: keyword("Equip", "May be played onto one of your Units, fusing into it"),
  // #783: five keywords no card prints yet (R783000, R783002, R783004, R783010, R783012).
  Ranged: keyword("Ranged", "Takes no damage back when it attacks, unless the defender is Ranged"),
  Flying: keyword("Flying", "Only Units with Flying or Ranged can attack it, unless it is in Defense Position. It may be played into your backrow as well as into a unit zone"),
  Cloaked: keyword("Cloaked", "Can be played face-down as a Trap. Activate: Reveal"),
  Crater: keyword("Crater", "Locks its zone when it arrives as a Unit, or when it is revealed as a Trap"),
  Flashback: keyword("Flashback", "Can be played from your graveyard once, at its cost"),
  // §6.1's statuses that are not keyword kinds (patch v0.2.0, B5 E35; R512, see the header).
  "Can't be in Defense Position": status("Can't be in Defense Position", "Never switches to Defense", {
    row: "Cannot be in Defense Position",
  }),
  "Can't be attacked": status("Can't be attacked", "No attack may target it"),
  "Only Units in this lane can attack this": status(
    "Only Units in this lane can attack this",
    "An attack on it is legal only from the enemy unit zone of its own lane",
  ),
  Berserk: status("Berserk", "While Berserk, at the start and end of its controller's turn this attacks its own hero"),
  // R1174000: a status set by Freeze (§6.3).
  Frozen: status(
    "Frozen",
    "Can't attack or switch position until it thaws: it loses its next chance to do either, and thaws at the end of the turn it could have done it",
  ),
  // Patch v0.3.7 (R781005): the pool's name, printed lower-case ("random combat keywords"); patch
  // v0.3.Y (R1025000) took Armor 1 and Spell Damage +1 out of it.
  "Combat keyword": status(
    "Combat keyword",
    "One of the keywords Units fight with: Taunt, Rush, Charge, First Strike, Flying, Ranged, Poisonous, Reborn, Divine Shield, Windfury, Cleave, Indestructible, Deft, Lifesteal, Pierce, Trample",
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
  // Patch v0.2.0 (B3.2, R384): "Activate", "Activate 2", "Activate ♾️" are this one term.
  Activate: trigger(
    "Activate",
    'Once per turn on your turn, click the card to do an effect; "Activate X" up to X times per turn, "Activate ♾️" any number of times',
    { row: "Activate / Activate X / Activate ♾️", aliases: ["activate"] },
  ),

  // §6.3 Actions and verbs
  Discover: verb("Discover", "Choose 1 of 3 options"),
  // Short (R500; see the header).
  Tribute: verb("Tribute", SHORT_REMINDERS.Tribute, { row: "Tribute X", aliases: ["Tributes"] }),
  Embiggen: verb("Embiggen", "Two prices, bigger effect for the bigger one"),
  Recruit: verb("Recruit", "Summon from deck, scanning top down"),
  // R910000: the lower-case and past spellings the cards print mid-sentence.
  Fuse: verb("Fuse", "Combine effects, stats and cost, cost capped at 4", { aliases: ["fuse", "fused"] }),
  Transform: verb("Transform", "Replace a card with another in place", { aliases: ["transform", "transforms"] }),
  Vanilla: verb("Vanilla", "Remove a unit's text and return it to its printed stats"),
  Lock: verb("Lock", "Zone can't be played into", { aliases: ["Locked"] }),
  "Choose one": verb("Choose one", "Modal effect"),
  // Patch v0.2.0's verbs (R512, see the header).
  Counter: verb("Counter", "Cancel a card being played or cast"),
  Steal: verb("Steal", "Take control", { aliases: ["steal"] }),
  Unlock: verb("Unlock", "A Locked zone accepts plays again"),
  Flicker: verb("Flicker", "The card leaves the field then re-enters the same zone at once"),
  // Balance patch 1 (R692): the printed word for a permanent's return from the field to its controller's hand (R746, R747).
  Bounce: verb("Bounce", "Return to controller's hand", { aliases: ["Bounced", "bounced"] }),
  // R386, R512, R1320: two rows of §6.3, one word each (the engine's Degrade and Upgrade).
  Nerf: verb("Nerf", "Weaken a card: one change per application", { aliases: ["Nerfs"] }),
  Buff: verb("Buff", "Strengthen a card: one change per application", { aliases: ["Buffs"] }),
  "Plague Counter": verb("Plague Counter", "Counter on a permanent, any number, reset on leaving the field", {
    aliases: ["Plague Counters"],
  }),
  "Set health": verb("Set health", "A hero's health becomes N"),
  Redirect: verb("Redirect", "A hit, a chosen target or an attack moves to another", { aliases: ["redirected"] }),
  "End the turn": verb("End the turn", "The turn ends from an effect"),
  "Trigger a Cry": verb("Trigger a Cry", "Run a unit's Cry again"),
  "Look at a hand": verb("Look at a hand", "See the opponent's hand in a prompt", {
    aliases: ["Look at your opponent's hand"],
  }),
  Allure: verb(
    "Allure",
    "Steal the marked enemy permanents at the start of your next turn; one you have no room for is destroyed",
  ),
  // R1174000, R910000: the verb that sets Frozen, bold in the spellings a card prints.
  Freeze: verb("Freeze", "The Unit loses its next attack or position switch, and thaws at the end of the turn it could have made one", {
    aliases: ["freeze", "freezes"],
  }),
  "Jade Counter": verb("Jade Counter", "A public count each player has, that only rises"),
  // R1384: §6.3's words the Meditative cards print; "hand size" matches as #79 prints it.
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
 * plain: the exile pile, which reads like Exile ("your exile", "from exile", Lizard's Breath's
 * "Exile," among its piles). Corpse Plantation's "each counter" and "1 counter" went with its old
 * text (patch v0.3.10), which now names its Plague Counters in full.
 */
export const PLAIN_SPELLINGS: readonly string[] = [
  "your exile",
  "their exile",
  "from exile",
  "graveyard and exile",
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
  "Immune to tribal tag based hate": "IT",
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
};
