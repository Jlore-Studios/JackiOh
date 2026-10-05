// The keyword glossary the inspect views print beside a card (docs/polish/6-cards.md, Surface B).
//
// Every `rule` is SPEC's own "Rule" column, copied verbatim: §6.1 for unit keywords, §6.2 for
// triggers and timing words, §6.3 for verbs. §5.2 has no Rule column, so Radiant's line is §6.3's
// "Make Radiant" rule followed by §5.2's sentence about a card in hand or library. If SPEC changes
// a rule, this file is the bug (CLAUDE.md: SPEC wins).
//
// One exception: where a row's own **Ruling** overrides its Rule column, the glossary states the
// ruling, because a player reads this as what the card does. Cry's Rule column says "enters the
// field for the first time", but its ruling in the same §6.2 row says it fires only when the card is
// played from hand or cast, and never for a copy, a Recruit, a Reborn, a token or a Transform
// result (§6.3's Summon and Recruit rows agree). RULED_TERMS lists these rows for rules.test.ts.
//
// And two rows are short reminders rather than SPEC's full text (R500, patch v0.2.0): Cry's ruling and
// Tribute's Rule column are long enough to crowd a card's inspect view, so the glossary says each in
// one short line that is still true to the rule (Tribute's no longer mentions R41's backrow clause,
// which R428 removed). SHORT_REMINDERS holds the two lines, and rules.test.ts pins them.
//
// `label` is what the rules-text tokenizer (rules.ts) looks for, case-sensitively, and `aliases`
// are the other spellings cards use. Patch v0.2.4 (issue #45) retired spelled variants ("Start of
// your turn", "End of your turn", "Start of Game", "Once per Turn", "Cannot be in Defense Position",
// "Trigger the Cry", "Set a hero's health", "End your turn"), keeping only grammatical plurals
// ("Plague Counters") and specific prompts ("Look at your opponent's hand").
//
// Patch v0.2.0 adds the §6 rows its card texts print (R512):
// - §6.1's statuses that are not keyword kinds (StatusTermId): "Can't be in Defense Position" (the
//   catalog's spelling of SPEC's "Cannot be in Defense Position", which stays an alias), "Can't be
//   attacked", "Only Units in this lane can attack this" and Berserk. "Can't attack or be attacked"
//   gets no row: no card prints it since patch v0.2.10 (it is the rule of a Unit a carrier holds,
//   R446), and matching stays case-sensitive, as "may tribute enemy units" stays plain words.
// - §6.2's Activate, one row for "Activate", "Activate X" and "Activate ♾️" (the tokenizer takes
//   the count or the ♾️ with the label, as it takes "Armor 2").
// - §6.3's Counter, Steal, Unlock, Flicker, Plague Counter (and "Plague Counters"), Redirect, Set health,
//   End the turn, Trigger a Cry and Look at a hand ("Look at your opponent's hand"): each label is
//   SPEC's row name and each alias the words cards print. Only capitalised spellings match, so "steal
//   it" mid-sentence stays plain.
// - §6.3's one row "Degrade / Upgrade" is two terms, since a card prints one word or the other: its
//   Rule column "Weaken / strengthen a card: …" pairs the words before and after the slash, and each
//   term's rule is its own word (capitalised) with the shared rest of the sentence: "Weaken a card:
//   one change per application" and "Strengthen a card: one change per application".
//
// A rule a SPEC row cites a ruling in ("(R384)") is printed without the citation: a ruling's number is
// not a player's word (`inPlayerWords`).
//
// Players read two of SPEC's words differently (v0.1.1, R373): the rules' "library" is the Deck and
// its "sacrifice" is a Tribute. The rules below are still SPEC's text, copied verbatim, and every one
// is put into players' words by `inPlayerWords` as the table is built, so a SPEC edit still lands
// here unchanged and nothing a player reads says "library" or "sacrifice".

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
  | "Degrade"
  | "Upgrade"
  | "Plague Counter"
  | "Set health"
  | "Redirect"
  | "End the turn"
  | "Trigger a Cry"
  | "Look at a hand";
/** §6.1's unit statuses that are not keyword kinds (patch v0.2.0, B5 E35). */
export type StatusTermId =
  | "Can't be in Defense Position"
  | "Can't be attacked"
  | "Only Units in this lane can attack this"
  | "Berserk";
export type GlossaryTermId = KeywordKind | StatusTermId | TriggerTermId | VerbTermId;
export type GlossaryEntry = {
  id: GlossaryTermId;
  label: string;
  /** SPEC's "Rule" column, copied verbatim (§6.1 keywords, §6.2 triggers, §6.3 verbs, §5.2 Radiant). */
  rule: string;
  section: "§5.2" | "§6.1" | "§6.2" | "§6.3";
  /** Other spellings matched in rules text. */
  aliases: readonly string[];
};

const NONE: readonly string[] = [];

/**
 * R500: the two glossary rows written as short reminders instead of SPEC's full text. Cry states §6.2's
 * ruling (played or cast, never another way onto the field); Tribute states §6.3's cost.
 */
export const SHORT_REMINDERS = {
  Cry: "When you play this card or an effect casts it. Never when it enters play otherwise",
  Tribute: "Playing this also costs X of your Units, which go to the graveyard",
} as const satisfies Readonly<Partial<Record<string, string>>>;

/** The terms whose `rule` is a short reminder (R500), in SHORT_REMINDERS. */
export const SHORT_TERMS: readonly (keyof typeof SHORT_REMINDERS)[] = ["Cry", "Tribute"];

/**
 * R373: SPEC's rules vocabulary in the words a player reads — the rules' library is the Deck, and
 * to sacrifice is to tribute. Patch v0.2.4 (issue #45) adds the retired turn-trigger prose ("At the
 * start of your turn" reads "At the start of turn"). Whole words only, keeping a leading capital.
 */
const PLAYER_WORDS: readonly (readonly [RegExp, string])[] = [
  [/\blibraries\b/g, "decks"],
  [/\bLibraries\b/g, "Decks"],
  [/\blibrary\b/g, "deck"],
  [/\bLibrary\b/g, "Deck"],
  [/\bsacrific(e|es|ed|ing)\b/g, "tribut$1"],
  [/\bSacrific(e|es|ed|ing)\b/g, "Tribut$1"],
  [/\bbackrow zone\b/g, "backrow"],
  [/\bBackrow zone\b/g, "Backrow"],
  // Patch v0.2.4 (issue #45): the retired turn-trigger prose reads label-style.
  [/\bAt the start of your turn\b/g, "At the start of turn"],
  [/\bAt the end of your turn\b/g, "At the end of turn"],
  [/\bat the start of your turn\b/g, "at the start of turn"],
  [/\bat the end of your turn\b/g, "at the end of turn"],
];

/** A SPEC row's citation of its ruling, " (R384)" or " (R41, R428)": no player's word. */
const RULING_CITATION = / \(R\d+(?:, R\d+)*\)/g;

/**
 * R373: a rule as SPEC writes it, in the words a player reads — the Deck and Tribute for the rules'
 * library and sacrifice, and no ruling's number (R512).
 */
export function inPlayerWords(rule: string): string {
  return PLAYER_WORDS.reduce((text, [pattern, word]) => text.replace(pattern, word), rule.replace(RULING_CITATION, ""));
}

function keyword(id: KeywordKind, rule: string): GlossaryEntry {
  return { id, label: id, rule: inPlayerWords(rule), section: "§6.1", aliases: NONE };
}

function trigger(id: TriggerTermId, rule: string, aliases: readonly string[] = NONE): GlossaryEntry {
  return { id, label: id, rule: inPlayerWords(rule), section: "§6.2", aliases };
}

function verb(id: VerbTermId, rule: string, aliases: readonly string[] = NONE): GlossaryEntry {
  return { id, label: id, rule: inPlayerWords(rule), section: "§6.3", aliases };
}

function status(id: StatusTermId, rule: string, aliases: readonly string[] = NONE): GlossaryEntry {
  return { id, label: id, rule: inPlayerWords(rule), section: "§6.1", aliases };
}

/**
 * R512: §6.3's "Degrade / Upgrade" row as two rules. "Weaken / strengthen a card: one change per
 * application" pairs "Weaken" with Degrade and "strengthen" with Upgrade, and each takes the rest of
 * the sentence: "Weaken a card: …", "Strengthen a card: …". rules.test.ts splits SPEC's row the same
 * way at test time.
 */
export function splitPairedRule(rule: string): [string, string] {
  const slash = rule.indexOf(" / ");
  if (slash < 0) return [rule, rule];
  const first = rule.slice(0, slash);
  const after = rule.slice(slash + " / ".length);
  const space = after.indexOf(" ");
  const second = space < 0 ? after : after.slice(0, space);
  const rest = space < 0 ? "" : after.slice(space);
  const capital = (word: string): string => word.charAt(0).toUpperCase() + word.slice(1);
  return [`${capital(first)}${rest}`, `${capital(second)}${rest}`];
}

/** §6.3 "Degrade / Upgrade", copied verbatim; the glossary splits it (splitPairedRule). */
const DEGRADE_UPGRADE_RULE = "Weaken / strengthen a card: one change per application (R386)";
const [DEGRADE_RULE, UPGRADE_RULE] = splitPairedRule(DEGRADE_UPGRADE_RULE);

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
  Indestructible: keyword("Indestructible", "Can't be destroyed or damaged; can be exiled or sacrificed"),
  Immutable: keyword("Immutable", "Text can't be changed or transformed"),
  Stack: keyword("Stack", "May be played onto an occupied zone"),
  "Can't attack": keyword("Can't attack", "Cannot declare attacks"),
  Armor: keyword("Armor", "Reduce each damage instance by X"),
  Lucky: keyword("Lucky", "Repeat a luck-based roll X extra times, keep the best"),
  // Patch v0.2.0's keywords (docs/classic-sets.md B3.1, B3.3, B5 E6 and E35; R383, R385).
  Animated: keyword(
    "Animated",
    "A Field Spell or Trap that becomes a Unit in an open unit zone when it fires or enters the field",
  ),
  "Animated on your turn": keyword(
    "Animated on your turn",
    "A Unit on your turn; back in its backrow zone on your opponent's",
  ),
  Brittle: keyword(
    "Brittle",
    "At the start of your turn, after a full turn cycle, the count drops by 1; at 0 the card is destroyed",
  ),
  "Spell Damage": keyword("Spell Damage", "Your Spells deal X more damage per hit"),
  "Immune to Spells": keyword("Immune to Spells", "Spells can't target it or affect it"),
  // R636, R637: the keyword rules patch (SPEC §6.1).
  Windfury: keyword("Windfury", "Can attack twice each turn"),
  Temporary: keyword("Temporary", "Discarded from its owner's hand at the end of their turn"),
  // Patch v0.2.11 (issue #181, R49): Deft, #45 Deft Duelist's keyword.
  Deft: keyword("Deft", "Can attack and switch position in the same turn"),
  // §6.1's statuses that are not keyword kinds (patch v0.2.0, B5 E35; R512, see the header).
  "Can't be in Defense Position": status("Can't be in Defense Position", "Never switches to Defense"),
  "Can't be attacked": status("Can't be attacked", "No attack may target it"),
  "Only Units in this lane can attack this": status(
    "Only Units in this lane can attack this",
    "An attack on it is legal only from the enemy unit zone of its own lane",
  ),
  Berserk: status("Berserk", "While Berserk, at the start and end of its controller's turn this attacks its own hero"),

  // §6.2 Triggers and timing words
  // §6.2's ruling, not its Rule column, and short (R500; see the header).
  Cry: trigger("Cry", SHORT_REMINDERS.Cry),
  Death: trigger("Death", "When sent from the field to the GY"),
  "Start of turn": trigger("Start of turn", "Controller's turn start, before the draw"),
  "End of turn": trigger("End of turn", "Controller's turn end, before cleanup"),
  "Start of game": trigger("Start of game", "After mulligan, before turn 1"),
  "Once per turn": trigger("Once per turn", "Activated ability limit"),
  Aura: trigger("Aura", "Effect while in play"),
  Combo: trigger("Combo", "Extra effect if X or more cards were played earlier this turn"),
  Echo: trigger("Echo", "Recast this card X more times"),
  "Cast on draw": trigger("Cast on draw", "Plays itself on draw, then draw again"),
  Quickdraw: trigger("Quickdraw", "Starts in your opening hand instead of a draw"),
  // Patch v0.2.0 (B3.2, R384): "Activate", "Activate 2", "Activate ♾️" are this one term.
  Activate: trigger(
    "Activate",
    'Once per turn on your turn, click the card to do an effect; "Activate X" up to X times per turn, "Activate ♾️" any number of times (R384)',
  ),

  // §6.3 Actions and verbs
  Discover: verb("Discover", "Choose 1 of 3 options"),
  // Short (R500; see the header).
  Tribute: verb("Tribute", SHORT_REMINDERS.Tribute),
  Embiggen: verb("Embiggen", "Two prices, bigger effect for the bigger one"),
  Recruit: verb("Recruit", "Summon from library, scanning top down"),
  Fuse: verb("Fuse", "Combine effects, stats and cost, cost capped at 4"),
  Transform: verb("Transform", "Replace a card with another in place"),
  Vanilla: verb("Vanilla", "Remove a unit's text"),
  Lock: verb("Lock", "Zone can't be summoned into"),
  "Choose one": verb("Choose one", "Modal effect"),
  // Patch v0.2.0's verbs (R512, see the header).
  Counter: verb("Counter", "Cancel a card being played or cast"),
  Steal: verb("Steal", "Take control"),
  Unlock: verb("Unlock", "A Locked zone accepts summons again"),
  Flicker: verb("Flicker", "The card leaves the field then re-enters the same zone at once"),
  Degrade: verb("Degrade", DEGRADE_RULE),
  Upgrade: verb("Upgrade", UPGRADE_RULE),
  "Plague Counter": verb("Plague Counter", "Counter on a permanent, any number, reset on leaving the field", ["Plague Counters"]),
  "Set health": verb("Set health", "A hero's health becomes N"),
  Redirect: verb("Redirect", "A hit, a chosen target or an attack moves to another"),
  "End the turn": verb("End the turn", "The turn ends from an effect"),
  "Trigger a Cry": verb("Trigger a Cry", "Run a unit's Cry again"),
  "Look at a hand": verb("Look at a hand", "See the opponent's hand in a prompt", ["Look at your opponent's hand"]),

  // §5.2 Radiant
  Radiant: {
    id: "Radiant",
    label: "Radiant",
    rule: inPlayerWords("Upgrade a card. In hand or library: cost unchanged, stats and text swap to the radiant form."),
    section: "§5.2",
    aliases: NONE,
  },
};

/** The terms whose `rule` states their SPEC row's ruling instead of its Rule column (see the header). */
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
};
