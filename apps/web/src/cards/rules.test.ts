// Polish 6, slice B: the rules-text tokenizer and the glossary, tested pure (docs/polish/6-cards.md,
// Surface B `glossary.ts` / `rules.ts`, "Glossary terms", "Tokenizer", behaviours B10–B11).
//
// The rendered half of B10 (`RulesText` → `strong.cf-term[data-term]`) is in CardFace.test.tsx,
// through the face that uses it.
//
// B11 says each keyword carries "SPEC §6.1's rule text", and the Surface says the rule is "SPEC's
// 'Rule' column, copied verbatim". So the SPEC tables are read here at test time, rather than
// retyped: a transcription in this file would be a second copy that could drift from both.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { KEYWORD_KINDS, fillParams, type KeywordKind } from "@jackioh/shared";

import {
  GLOSSARY,
  KEYWORD_MARK,
  RULED_TERMS,
  SHORT_REMINDERS,
  SHORT_TERMS,
  inPlayerWords,
  splitPairedRule,
  type GlossaryTermId,
  type StatusTermId,
  type TriggerTermId,
  type VerbTermId,
} from "./glossary.ts";
import { termsIn, tokenizeRules, type RulesToken } from "./rules.ts";

/* -------------------------------------------------------------------------------------- helpers */

const SPEC = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "../../../../SPEC.md"), "utf8");

/** The `Rule` column of one SPEC §6 table, keyed by the table's first column. */
function ruleColumn(section: "6.1" | "6.2" | "6.3"): Map<string, string> {
  const lines = SPEC.split("\n");
  const start = lines.findIndex((line) => line.startsWith(`### ${section} `));
  if (start < 0) throw new Error(`SPEC.md has no §${section} heading`);
  const rules = new Map<string, string>();
  for (const line of lines.slice(start + 1)) {
    if (line.startsWith("#")) break;
    if (!line.startsWith("|")) continue;
    const [term, rule] = line
      .split("|")
      .slice(1)
      .map((cell) => cell.trim());
    if (term === undefined || rule === undefined) continue;
    if (/^-+$/.test(term) || rule === "Rule") continue;
    rules.set(term, rule);
  }
  return rules;
}

/** The Cry line the glossary printed before R500 shortened it: §6.2's ruling in full. */
const CRY_RULING_BEFORE_R500 =
  "When you play this card from your hand, or it is cast (Cast on draw, Echo, Call to Chaos). Not when it is summoned, copied, Recruited, Reborn or Transformed into";

/** R500: a short reminder is one line of at most this many words. */
const SHORT_REMINDER_MAX_WORDS = 16;

function isShort(id: GlossaryTermId): id is (typeof SHORT_TERMS)[number] {
  return (SHORT_TERMS as readonly string[]).includes(id);
}

/** Where a glossary id's SPEC row spells it differently (a trailing X, a capital). */
const SPEC_ROW_NAME: Readonly<Partial<Record<GlossaryTermId, string>>> = {
  Armor: "Armor X",
  Lucky: "Lucky X",
  Brittle: "Brittle X",
  "Spell Damage": "Spell Damage X",
  "Start of game": "Start of Game",
  "Once per turn": "Once per Turn",
  Combo: "Combo X",
  Echo: "Echo X",
  Tribute: "Tribute X",
  // Patch v0.2.0 (R512): the catalog's spelling of SPEC's row, and the rows that name several forms.
  "Can't be in Defense Position": "Cannot be in Defense Position",
  Activate: "Activate / Activate X / Activate ♾️",
  Degrade: "Degrade / Upgrade",
  Upgrade: "Degrade / Upgrade",
};

/** R512: the one SPEC row the glossary splits in two, and which half of it each term takes. */
const PAIRED_HALF: Readonly<Partial<Record<GlossaryTermId, 0 | 1>>> = { Degrade: 0, Upgrade: 1 };

function specRule(section: "6.1" | "6.2" | "6.3", id: GlossaryTermId): string {
  const name = SPEC_ROW_NAME[id] ?? id;
  const rule = ruleColumn(section).get(name);
  if (rule === undefined) throw new Error(`SPEC §${section} has no row "${name}"`);
  const half = PAIRED_HALF[id];
  return half === undefined ? rule : splitPairedRule(rule)[half];
}

const TRIGGERS: readonly TriggerTermId[] = [
  "Cry",
  "Death",
  "Start of turn",
  "End of turn",
  "Start of game",
  "Once per turn",
  "Aura",
  "Combo",
  "Echo",
  "Cast on draw",
  "Quickdraw",
  "Activate",
];

/** R512: §6.1's statuses that are not keyword kinds, which patch v0.2.0's cards print. */
const STATUSES: readonly StatusTermId[] = [
  "Can't be in Defense Position",
  "Can't be attacked",
  "Only Units in this lane can attack this",
  "Berserk",
];

const VERBS_6_3: readonly VerbTermId[] = [
  "Discover",
  "Tribute",
  "Embiggen",
  "Recruit",
  "Fuse",
  "Transform",
  "Vanilla",
  "Lock",
  "Choose one",
  "Counter",
  "Steal",
  "Unlock",
  "Flicker",
  "Degrade",
  "Upgrade",
  "Plague Counter",
  "Set health",
  "Redirect",
  "End the turn",
  "Trigger a Cry",
  "Look at a hand",
];

/**
 * "Moved unchanged out of Card.tsx": TA RU CH FS PO LS RB DS TR CL ND IM ST NA AR LK, R346's PI, and
 * patch v0.2.0's AN AT BR SD IS, then R636's WF and R637's TE, then patch v0.2.11's DE.
 */
const MARKS: Readonly<Record<KeywordKind, string>> = {
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

type Term = { text: string; term: GlossaryTermId };

function termsOf(text: string): Term[] {
  return tokenizeRules(text).flatMap((token: RulesToken) =>
    token.kind === "term" ? [{ text: token.text, term: token.term }] : [],
  );
}

function joined(text: string): string {
  return tokenizeRules(text)
    .map((token) => token.text)
    .join("");
}

const ALL_TEXTS: readonly { where: string; text: string }[] = Object.values(CATALOG).flatMap((card) => [
  { where: `${card.id} base`, text: card.base.text },
  { where: `${card.id} radiant`, text: card.radiant.text },
]);

/* ----------------------------------------------------------------------------------------- B10 */

describe("B10: tokenizeRules and termsIn", () => {
  it("B10 'Cry: deal 2' is the term 'Cry:' followed by plain text", () => {
    expect(termsOf("Cry: deal 2")).toEqual([{ text: "Cry:", term: "Cry" }]);
    expect(joined("Cry: deal 2")).toBe("Cry: deal 2");
  });

  it("R636 R637 Windfury and Temporary are terms in a card's text, each with the glossary entry the SPEC row gives", () => {
    expect(termsOf("Windfury. Temporary")).toEqual([
      { text: "Windfury", term: "Windfury" },
      { text: "Temporary", term: "Temporary" },
    ]);
    expect([GLOSSARY.Windfury, GLOSSARY.Temporary].map((entry) => [entry.section, entry.rule])).toEqual([
      ["§6.1", "Can attack twice each turn"],
      ["§6.1", "Discarded from its owner's hand at the end of their turn"],
    ]);
  });

  it("R49 Deft is a term in a card's text, with the glossary entry the SPEC row gives", () => {
    expect(termsOf("Charge, Deft")).toEqual([
      { text: "Charge", term: "Charge" },
      { text: "Deft", term: "Deft" },
    ]);
    expect([GLOSSARY.Deft].map((entry) => [entry.section, entry.rule])).toEqual([
      ["§6.1", "Can attack and switch position in the same turn"],
    ]);
  });

  it("B10 'Armor 2' is one term", () => {
    expect(termsOf("Armor 2")).toEqual([{ text: "Armor 2", term: "Armor" }]);
    expect(joined("Armor 2")).toBe("Armor 2");
  });

  it("B10 'Start of turn:' is the Start of turn term", () => {
    expect(termsOf("Start of turn: gain 1 mana")).toEqual([{ text: "Start of turn:", term: "Start of turn" }]);
  });

  it("B10 'End of turn:' is the End of turn term", () => {
    expect(termsOf("End of turn: heal to full")).toEqual([{ text: "End of turn:", term: "End of turn" }]);
    expect(termsIn(played("classic-065"))[0]).toBe("End of turn");
  });

  it("B10 'Start of game' and 'Once per turn' match their exact labels", () => {
    expect(termsOf("Start of game: gain a power")).toEqual([{ text: "Start of game:", term: "Start of game" }]);
    expect(termsOf('each "Once per turn, spend X"')).toEqual([{ text: "Once per turn", term: "Once per turn" }]);
  });

  it("B10 matching is case-sensitive: 'taunt', 'cry:' and 'choose one' stay text", () => {
    for (const text of ["taunt", "cry: deal 2", "choose one: a or b", "may tribute enemy units", "DIVINE SHIELD"]) {
      expect(termsOf(text), text).toEqual([]);
      expect(joined(text), text).toBe(text);
    }
  });

  it("B10 a term glued to a letter or digit on either side stays text: 'Locked', 'Rushing', 'Rush2', 'unTaunt'", () => {
    for (const text of ["Locked", "Rushing", "Rush2", "unTaunt", "XRush", "Crying", "Deathless", "Tauntx"]) {
      expect(termsOf(text), text).toEqual([]);
      expect(joined(text), text).toBe(text);
    }
  });

  it("B10 punctuation and spaces are word boundaries: 'Lock its zone', '(Taunt)', 'Rush.'", () => {
    expect(termsOf("Destroy target backrow card; Lock its zone")).toEqual([{ text: "Lock", term: "Lock" }]);
    expect(termsOf("(Taunt)")).toEqual([{ text: "Taunt", term: "Taunt" }]);
    expect(termsOf("Rush.")).toEqual([{ text: "Rush", term: "Rush" }]);
    expect(joined("Rush.")).toBe("Rush.");
  });

  it("B10 multi-word terms are one token: Divine Shield, First Strike, Can't attack, Cast on draw, Choose one", () => {
    expect(termsOf("Rush, Taunt, Lifesteal, Divine Shield")).toEqual([
      { text: "Rush", term: "Rush" },
      { text: "Taunt", term: "Taunt" },
      { text: "Lifesteal", term: "Lifesteal" },
      { text: "Divine Shield", term: "Divine Shield" },
    ]);
    expect(termsOf("First Strike")).toEqual([{ text: "First Strike", term: "First Strike" }]);
    expect(termsOf("Can't attack. Death: steal all enemy units")).toEqual([
      { text: "Can't attack", term: "Can't attack" },
      { text: "Death:", term: "Death" },
    ]);
    expect(termsOf("Cast on draw: take 1 damage")).toEqual([{ text: "Cast on draw:", term: "Cast on draw" }]);
    expect(termsOf("Choose one: bounce all units")).toEqual([{ text: "Choose one:", term: "Choose one" }]);
  });

  it("B10 the match extends over ' <digits|X>' and then ':': 'Combo 2:', 'Combo X:', 'Lucky 1', 'Tribute 3'", () => {
    expect(termsOf("Combo 2: draw 1")).toEqual([{ text: "Combo 2:", term: "Combo" }]);
    expect(termsOf("Combo X: deal X damage")).toEqual([{ text: "Combo X:", term: "Combo" }]);
    expect(termsOf("Lucky 1 (two rolls, keep the success)")).toEqual([{ text: "Lucky 1", term: "Lucky" }]);
    expect(termsOf("Armor X")).toEqual([{ text: "Armor X", term: "Armor" }]);
    expect(termsOf("Armor 12")).toEqual([{ text: "Armor 12", term: "Armor" }]);
    expect(termsOf("Tribute 3, Armor 3, Taunt; may tribute enemy units")).toEqual([
      { text: "Tribute 3", term: "Tribute" },
      { text: "Armor 3", term: "Armor" },
      { text: "Taunt", term: "Taunt" },
    ]);
  });

  it("B10 anything but a space then digits or X is not absorbed: 'Echo +1', 'Choose 2'", () => {
    expect(termsOf("The next Spell you play gains Echo +1")).toEqual([{ text: "Echo", term: "Echo" }]);
    // "Choose" alone is not a term; only "Choose one" is.
    expect(termsOf("Choose 2")).toEqual([]);
  });

  it("B10 tokens are lossless for every catalog text, and every term is a glossary id at word boundaries", () => {
    const ids = new Set<string>(Object.keys(GLOSSARY));
    for (const { where, text } of ALL_TEXTS) {
      const tokens = tokenizeRules(text);
      expect(tokens.map((token) => token.text).join(""), where).toBe(text);

      let offset = 0;
      for (const token of tokens) {
        if (token.kind === "term") {
          expect(ids.has(token.term), `${where}: ${token.term}`).toBe(true);
          const entry = GLOSSARY[token.term];
          const spellings = [entry.label, ...entry.aliases];
          expect(
            spellings.some((spelling) => token.text.startsWith(spelling)),
            `${where}: "${token.text}" starts with a spelling of ${token.term}`,
          ).toBe(true);
          const before = text.charAt(offset - 1);
          const after = text.charAt(offset + token.text.length);
          expect(/[A-Za-z0-9]/.test(before), `${where}: "${token.text}" has a letter before it`).toBe(false);
          expect(/[A-Za-z0-9]/.test(after), `${where}: "${token.text}" has a letter after it`).toBe(false);
        }
        offset += token.text.length;
      }
    }
  });

  it("B10 real card texts: Cry, Death, Combo, Cast on draw, Radiant and keywords are marked", () => {
    expect(termsIn("Cry: destroy target enemy non-Human unit")).toEqual(["Cry"]);
    expect(termsIn("Reborn; Death: all your other units become Radiant")).toEqual(["Reborn", "Death", "Radiant"]);
    expect(termsIn("Combo 3: draw 3; otherwise nothing")).toEqual(["Combo"]);
    expect(termsIn("Cast on draw: the opponent's next mana refresh is 1 lower")).toEqual(["Cast on draw"]);
    expect(termsIn("Deal 2 damage to a target, Lifesteal")).toEqual(["Lifesteal"]);
    expect(termsIn("Recruit 3 Units costing 1 or less")).toEqual(["Recruit"]);
    expect(termsIn("When the opponent plays a Unit: Transform it into a Sheep Token")).toEqual(["Transform"]);
  });

  it("B10 termsIn lists distinct terms in order of first appearance", () => {
    expect(termsIn("Tribute 3, Armor 3, Taunt; may tribute enemy units")).toEqual(["Tribute", "Armor", "Taunt"]);
    expect(termsIn("Rush; Rush, Rush.")).toEqual(["Rush"]);
    expect(termsIn("Start of your turn: x. Start of turn: y. End of turn: z")).toEqual(["Start of turn", "End of turn"]);
    expect(termsIn("Armor 1; Taunt; Armor 2")).toEqual(["Armor", "Taunt"]);
  });

  it("B10 empty or term-free text gives no terms", () => {
    expect(tokenizeRules("").filter((token) => token.kind === "term")).toEqual([]);
    expect(joined("")).toBe("");
    expect(termsIn("")).toEqual([]);
    expect(termsIn("taunt, Locked, choose one, Rushing")).toEqual([]);
    expect(termsIn("   ")).toEqual([]);
  });
});

/* ----------------------------------------------------------------------------------------- B11 */

describe("B11: GLOSSARY and KEYWORD_MARK", () => {
  it("B11 the SPEC tables this file reads are really there (a control for the checks below)", () => {
    expect(ruleColumn("6.1").size).toBeGreaterThanOrEqual(KEYWORD_KINDS.length);
    expect(ruleColumn("6.2").size).toBeGreaterThanOrEqual(TRIGGERS.length);
    expect(ruleColumn("6.3").size).toBeGreaterThanOrEqual(VERBS_6_3.length);
  });

  it("B11 R373 one entry per KEYWORD_KINDS kind, carrying SPEC §6.1's rule text verbatim, in players' words", () => {
    for (const kind of KEYWORD_KINDS) {
      const entry = GLOSSARY[kind];
      expect(entry, kind).toBeDefined();
      expect(entry.id, kind).toBe(kind);
      expect(entry.section, kind).toBe("§6.1");
      expect(entry.rule, kind).toBe(inPlayerWords(specRule("6.1", kind)));
    }
  });

  it("B11 KEYWORD_MARK keeps the two-letter marks Card.tsx used, for exactly the 18 kinds", () => {
    expect(KEYWORD_MARK).toEqual(MARKS);
  });

  it("B11 KEYWORD_MARK has a distinct two-letter mark for every keyword kind, patch v0.2.0's included", () => {
    expect(Object.keys(KEYWORD_MARK).sort()).toEqual([...KEYWORD_KINDS].sort());
    const marks = KEYWORD_KINDS.map((kind) => KEYWORD_MARK[kind]);
    for (const [index, mark] of marks.entries()) expect(mark, KEYWORD_KINDS[index]).toMatch(/^[A-Z]{2}$/);
    expect(new Set(marks).size).toBe(marks.length);
  });

  it("B11 every §6.2 term, with SPEC §6.2's rule text (or, for a ruled term, not the overridden Rule column)", () => {
    for (const id of TRIGGERS) {
      const entry = GLOSSARY[id];
      expect(entry, id).toBeDefined();
      expect(entry.id, id).toBe(id);
      expect(entry.section, id).toBe("§6.2");
      if (isShort(id)) expect(entry.rule, id).toBe(SHORT_REMINDERS[id]);
      else if (RULED_TERMS.includes(id)) expect(entry.rule, id).not.toBe(inPlayerWords(specRule("6.2", id)));
      else expect(entry.rule, id).toBe(inPlayerWords(specRule("6.2", id)));
    }
  });

  it("B11 R500 Cry states §6.2's ruling, short: played or cast, never onto the field another way", () => {
    // The ruling this pins, read from SPEC so a change there fails here.
    const row = SPEC.split("\n").find((line) => line.startsWith("| Cry |"));
    expect(row).toBeDefined();
    expect(row).toContain("fires only when played from hand (or by Cast on draw / Echo / Call to Chaos casting)");
    expect(row).toContain("Copies, Recruit, Reborn, tokens and Transform results do not fire it");

    const rule = GLOSSARY.Cry.rule;
    expect(rule).toBe(SHORT_REMINDERS.Cry);
    expect(rule).not.toContain("enters the field");
    expect(rule).toMatch(/play this card/);
    expect(rule).toMatch(/casts it/);
    expect(rule).toMatch(/^[^.]*\. Never /);
    expect(RULED_TERMS).toEqual(["Cry"]);
  });

  it("R500 Cry and Tribute are short reminders, each shorter than the SPEC text it replaces and a single line", () => {
    expect([...SHORT_TERMS]).toEqual(["Cry", "Tribute"]);
    expect(GLOSSARY.Tribute.rule).toBe(SHORT_REMINDERS.Tribute);
    expect(GLOSSARY.Tribute.section).toBe("§6.3");
    expect(SHORT_REMINDERS.Cry.length).toBeLessThan(CRY_RULING_BEFORE_R500.length);
    expect(SHORT_REMINDERS.Tribute.length).toBeLessThan(inPlayerWords(specRule("6.3", "Tribute")).length);
    for (const id of SHORT_TERMS) {
      const words = SHORT_REMINDERS[id].split(/\s+/).length;
      expect(words, id).toBeLessThanOrEqual(SHORT_REMINDER_MAX_WORDS);
      expect(SHORT_REMINDERS[id], id).not.toMatch(/\n|\(R\d+\)|library|sacrific/i);
    }
    // Tribute's reminder is Units only (R428: Carnivorous Cube no longer eats the backrow).
    expect(SHORT_REMINDERS.Tribute).toContain("Units");
    expect(SHORT_REMINDERS.Tribute).not.toMatch(/permanent|backrow/i);
  });

  it("B11 every §6.3 term, with SPEC §6.3's rule text, and Radiant under §5.2 with a non-empty rule", () => {
    for (const id of VERBS_6_3) {
      const entry = GLOSSARY[id];
      expect(entry, id).toBeDefined();
      expect(entry.id, id).toBe(id);
      expect(entry.section, id).toBe("§6.3");
      if (isShort(id)) expect(entry.rule, id).toBe(SHORT_REMINDERS[id]);
      else expect(entry.rule, id).toBe(inPlayerWords(specRule("6.3", id)));
    }
    const radiant = GLOSSARY.Radiant;
    expect(radiant.id).toBe("Radiant");
    expect(radiant.section).toBe("§5.2");
    expect(radiant.rule.trim()).not.toBe("");
  });

  it("R373 the glossary says deck for the rules' library and tribute for sacrifice, and never the old words", () => {
    expect(inPlayerWords("Summon from library, scanning top down")).toBe("Summon from deck, scanning top down");
    expect(inPlayerWords("Library order; swap libraries")).toBe("Deck order; swap decks");
    expect(inPlayerWords("can be exiled or sacrificed; Sacrifice X; it sacrifices; sacrificing")).toBe(
      "can be exiled or tributed; Tribute X; it tributes; tributing",
    );
    // Whole words only: nothing inside another word moves.
    expect(inPlayerWords("librarian, sacrificial")).toBe("librarian, sacrificial");
    expect(GLOSSARY.Recruit.rule).toBe("Summon from deck, scanning top down");
    expect(GLOSSARY.Radiant.rule).toContain("In hand or deck");
    expect(GLOSSARY.Indestructible.rule).toContain("tributed");
    // R500: Tribute's reminder is short and says Units, not the rules' "sacrifice".
    expect(GLOSSARY.Tribute.rule).toBe("Playing this also costs X of your Units, which go to the graveyard");
    // Patch v0.2.4: backrow zone becomes backrow in players' words.
    expect(inPlayerWords("steps from its backrow zone into a unit zone")).toBe("steps from its backrow into a unit zone");
    // Patch v0.2.4: retired turn-trigger prose reads label-style in players' words.
    expect(inPlayerWords("At the start of your turn, the count drops")).toBe("At the start of turn, the count drops");
    expect(inPlayerWords("it heals at the end of your turn")).toBe("it heals at the end of turn");
    for (const entry of Object.values(GLOSSARY)) {
      expect(entry.rule, entry.id).not.toMatch(/\blibrar(y|ies)\b|\bsacrific|\bbounce\b|\bbackrow zone\b/i);
      // Patch v0.2.4 (issue #45): no glossary rule uses any other word the vocabulary table retired.
      expect(entry.rule, entry.id).not.toMatch(/\b(at the (start|end)( and end)? of your turn|(start|end) of your turn)\b/i);
      expect(entry.rule, entry.id).not.toMatch(/\bEnd your turn\b/);
      expect(entry.rule, entry.id).not.toMatch(/\bStart of Game\b/);
      expect(entry.rule, entry.id).not.toMatch(/\bOnce per Turn\b/);
      expect(entry.rule, entry.id).not.toMatch(/\bthat costs \(/i);
      expect(entry.rule, entry.id).not.toMatch(/\bCost \(/);
      expect(entry.rule, entry.id).not.toMatch(/\bcosting \(/i);
      expect(entry.rule, entry.id).not.toMatch(/\bTrigger the Cry\b/i);
      expect(entry.rule, entry.id).not.toMatch(/\bSet a hero's health\b/i);
      expect(entry.rule, entry.id).not.toMatch(/\bCannot be in Defense Position\b/i);
      expect(entry.rule, entry.id).not.toMatch(/\breturn\b[^.\n]*\bto your hand\b/i);
    }
  });

  it("R512 every §6.1 status a v0.2.0 card prints, with SPEC §6.1's rule text in players' words", () => {
    for (const id of STATUSES) {
      const entry = GLOSSARY[id];
      expect(entry, id).toBeDefined();
      expect(entry.id, id).toBe(id);
      expect(entry.section, id).toBe("§6.1");
      expect(entry.rule, id).toBe(inPlayerWords(specRule("6.1", id)));
    }
    // The catalog writes "Can't"; aliases are empty (patch v0.2.4, issue #45).
    expect(GLOSSARY["Can't be in Defense Position"].aliases).toEqual([]);
  });

  it("R512 SPEC's one row \"Degrade / Upgrade\" is two terms, each its own word with the row's shared rest", () => {
    const row = ruleColumn("6.3").get("Degrade / Upgrade");
    expect(row).toBeDefined();
    expect(splitPairedRule("Weaken / strengthen a card: one change per application (R386)")).toEqual([
      "Weaken a card: one change per application (R386)",
      "Strengthen a card: one change per application (R386)",
    ]);
    expect(GLOSSARY.Degrade.rule).toBe("Weaken a card: one change per application");
    expect(GLOSSARY.Upgrade.rule).toBe("Strengthen a card: one change per application");
    expect(GLOSSARY.Degrade.section).toBe("§6.3");
    expect(GLOSSARY.Upgrade.section).toBe("§6.3");
    // A rule with no pair is itself, twice.
    expect(splitPairedRule("Take control")).toEqual(["Take control", "Take control"]);
  });

  it("R512 a ruling's number is no player's word: every rule is printed without SPEC's citations", () => {
    expect(inPlayerWords("Once per turn (R384)")).toBe("Once per turn");
    expect(inPlayerWords("one of your other Units (R41, R428)")).toBe("one of your other Units");
    expect(inPlayerWords("A rule (with a note) stays")).toBe("A rule (with a note) stays");
    for (const entry of Object.values(GLOSSARY)) expect(entry.rule, entry.id).not.toMatch(/\(R\d+/);
    expect(GLOSSARY.Activate.rule).toBe(
      'Once per turn on your turn, click the card to do an effect; "Activate X" up to X times per turn, "Activate ♾️" any number of times',
    );
  });

  it("B11 the glossary is exactly those terms, each with a label, a rule and an alias list", () => {
    const expected = [...KEYWORD_KINDS, ...STATUSES, ...TRIGGERS, ...VERBS_6_3, "Radiant"].sort();
    expect(Object.keys(GLOSSARY).sort()).toEqual(expected);
    for (const [key, entry] of Object.entries(GLOSSARY)) {
      expect(entry.id, key).toBe(key);
      expect(entry.label.trim(), key).not.toBe("");
      expect(entry.rule.trim(), key).not.toBe("");
      expect(Array.isArray(entry.aliases), key).toBe(true);
    }
  });

  it("B11 the glossary's aliases for retired variants are empty", () => {
    for (const [key, entry] of Object.entries(GLOSSARY)) {
      if (key === "Plague Counter") {
        expect(entry.aliases, key).toEqual(["Plague Counters"]);
      } else if (key === "Look at a hand") {
        expect(entry.aliases, key).toEqual(["Look at your opponent's hand"]);
      } else {
        expect(entry.aliases, key).toEqual([]);
      }
    }
  });
});

/* ------------------------------------------------------------------------------- patch v0.2.0 */

/** A catalog face's text as a player reads it: its `{key}`s filled with the printed numbers. */
function played(id: string, face: "base" | "radiant" = "base"): string {
  const card = CATALOG[id];
  if (card === undefined) throw new Error(`the catalog has no ${id}`);
  return fillParams(card, face);
}

/** Every catalog face's text as a player reads it. */
const PLAYED_TEXTS: readonly { where: string; text: string }[] = Object.values(CATALOG).flatMap((card) => [
  { where: `${card.id} base`, text: fillParams(card, "base") },
  { where: `${card.id} radiant`, text: fillParams(card, "radiant") },
]);

describe("R512: the tokenizer finds patch v0.2.0's terms in the catalog's own texts", () => {
  it("R384 Activate, Activate 2 and Activate ♾️ are one term each, with the colon; a trap's \"Activates when\" is not", () => {
    expect(termsOf(played("classic-007"))).toContainEqual({ text: "Activate:", term: "Activate" });
    expect(termsOf(played("classicplus-076-1", "radiant"))).toContainEqual({ text: "Activate 2:", term: "Activate" });
    expect(termsOf(played("classic-021"))).toContainEqual({ text: "Activate ♾️:", term: "Activate" });
    expect(termsOf("Activate ♾: deal 1")).toEqual([{ text: "Activate ♾:", term: "Activate" }]);
    expect(termsIn(played("classic-010"))).not.toContain("Activate");
    expect(termsIn("Once this has activated: draw 1")).not.toContain("Activate");
  });

  it("R512 each new verb is found where a card prints it", () => {
    const cases: readonly (readonly [GlossaryTermId, string, "base" | "radiant", string])[] = [
      ["Counter", "classic-017", "base", "Counter"],
      ["Steal", "classic-032", "base", "Steal"],
      ["Unlock", "classicplus-077", "base", "Unlock"],
      ["Flicker", "classic-014", "radiant", "Flicker"],
      ["Degrade", "classicplus-008", "base", "Degrade"],
      ["Upgrade", "classicplus-071", "base", "Upgrade"],
      ["Plague Counter", "classic-039", "base", "Plague Counters"],
      ["Set health", "classic-029", "base", "Set health"],
      ["Redirect", "classic-052", "base", "Redirect"],
      ["End the turn", "classicplus-026", "base", "End the turn"],
      ["Trigger a Cry", "classic-054", "base", "Trigger a Cry"],
      ["Look at a hand", "classic-011", "base", "Look at your opponent's hand"],
    ];
    for (const [term, id, face, spelling] of cases) {
      // As for any term, a number right after it is taken with it ("Degrade 4 random cards").
      const found = termsOf(played(id, face)).filter((entry) => entry.term === term);
      expect(found.length, `${id} ${face}`).toBeGreaterThan(0);
      expect(found.some((entry) => entry.text === spelling || entry.text.startsWith(`${spelling} `)), `${id} ${face}: ${spelling}`).toBe(true);
    }
  });

  it("R512 each new status, and the v0.2.0 keyword kinds, are found where a card prints them", () => {
    expect(termsIn(played("classicplus-051"))).toEqual(expect.arrayContaining(["Can't be in Defense Position", "Can't be attacked"]));
    expect(termsIn(played("classicplus-019-1"))).toContain("Only Units in this lane can attack this");
    expect(termsIn(played("classicplus-019-1", "radiant"))).toContain("Immune to Spells");
    expect(termsOf(played("classicplus-019-5"))).toContainEqual({ text: "Berserk:", term: "Berserk" });
    expect(termsIn(played("classicplus-019-2"))).toContain("Berserk");
    expect(termsOf(played("classicplus-074"))).toContainEqual({ text: "Brittle 4", term: "Brittle" });
    expect(termsIn(played("classicplus-038"))).toContain("Spell Damage");
    expect(termsIn(played("classic-005"))).toContain("Animated");
    expect(termsIn(played("classicplus-012-8"))).toContain("Animated on your turn");
    // Aliases are empty: SPEC's "Cannot" is plain words without the alias.
    expect(termsOf("Cannot be in Defense Position.")).toEqual([]);
  });

  it("R512 matching stays case-sensitive: a lower-case \"steal it\" or \"can't attack or be attacked\" is plain words", () => {
    expect(termsIn("you may Tribute this to steal it")).toEqual(["Tribute"]);
    expect(termsIn("That Unit can't attack or be attacked")).toEqual([]);
    expect(termsIn("This can't go Berserk.")).toEqual(["Berserk"]);
  });

  it("R512 every new term has at least one catalog text that prints it", () => {
    const fresh: readonly GlossaryTermId[] = [
      ...STATUSES,
      "Activate",
      "Counter",
      "Steal",
      "Unlock",
      "Flicker",
      "Degrade",
      "Upgrade",
      "Plague Counter",
      "Set health",
      "Redirect",
      "End the turn",
      "Trigger a Cry",
      "Look at a hand",
    ];
    const found = new Set(PLAYED_TEXTS.flatMap(({ text }) => termsIn(text)));
    for (const term of fresh) expect(found.has(term), term).toBe(true);
  });

  it("R512 tokens stay lossless over every text a player reads, numbers filled in", () => {
    for (const { where, text } of PLAYED_TEXTS) {
      expect(joined(text), where).toBe(text);
    }
  });
});
