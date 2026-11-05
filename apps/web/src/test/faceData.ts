// The keyword glossary as the face model's crate holds it (`crates/shared/data/glossary.json`), and the
// tokenizer as the face model answers it (`richWords`), for the tests that name a term's label or rule or
// check where a text's terms stand. The web keeps no copy of either: a face's terms and their rules come
// from the face model (jackioh-shared), and a test reads the same file and asks the same module.

import glossaryJson from "../../../../crates/shared/data/glossary.json";
import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import { faceModel, richWords, wordDiff } from "../cards/model.ts";
import type { Span } from "../wire/generated/Span.ts";

/** Every term of the glossary (`glossary.json`): a term added there is added here, and `rules.test.ts` says when one is missing. */
export type GlossaryTermId =
  | "Taunt"
  | "Rush"
  | "Charge"
  | "First Strike"
  | "Poisonous"
  | "Lifesteal"
  | "Reborn"
  | "Divine Shield"
  | "Trample"
  | "Cleave"
  | "Pierce"
  | "Indestructible"
  | "Immutable"
  | "Stack"
  | "Can't attack"
  | "Armor"
  | "Lucky"
  | "Animated"
  | "Animated on your turn"
  | "Brittle"
  | "Spell Damage"
  | "Immune to Spells"
  | "Windfury"
  | "Temporary"
  | "Deft"
  | "Untributable"
  | "Equip"
  | "Ranged"
  | "Flying"
  | "Cloaked"
  | "Crater"
  | "Flashback"
  | "Tradable"
  | "Starship Piece"
  | "Starship"
  | "Can't Defend"
  | "Can't be attacked"
  | "Only Units in this lane can attack this"
  | "Berserk"
  | "Frozen"
  | "Dormant"
  | "Combat keyword"
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
  | "When launched"
  | "Discover"
  | "Tribute"
  | "Embiggened"
  | "Recruit"
  | "Fuse"
  | "Transform"
  | "Vanilla"
  | "Lock"
  | "Choose one"
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
  | "Mark in a hand"
  | "Radiant";
export type GlossaryEntry = {
  readonly id: GlossaryTermId;
  readonly label: string;
  readonly rule: string;
  readonly section: "§5.2" | "§6.1" | "§6.2" | "§6.3";
  readonly row?: string;
  readonly aliases: readonly string[];
};
export type TriggerTermId = GlossaryTermId;
export type VerbTermId = GlossaryTermId;
export type StatusTermId = GlossaryTermId;

/** Every entry in the glossary's order. */
export const GLOSSARY_ROWS = glossaryJson.entries as readonly GlossaryEntry[];

/** The glossary by term id. */
export const GLOSSARY = Object.fromEntries(GLOSSARY_ROWS.map((row) => [row.id, row])) as Readonly<Record<GlossaryTermId, GlossaryEntry>>;

/** The spellings that hold a term's word but name something else, so they print plain (R910000). */
export const PLAIN_SPELLINGS: readonly string[] = glossaryJson.plain;

/** The terms whose rule states their spec row's ruling instead of its Rule column. */
export const RULED_TERMS = glossaryJson.ruled as readonly GlossaryTermId[];

/** R500: the two glossary rows written as short reminders. */
export const SHORT_REMINDERS: { readonly Cry: string; readonly Tribute: string } = glossaryJson.shortReminders;

/** The terms whose `rule` is a short reminder (R500). */
export const SHORT_TERMS = glossaryJson.shortTerms as readonly (keyof typeof SHORT_REMINDERS)[];

export type RulesToken = { kind: "text"; text: string } | { kind: "term"; text: string; term: GlossaryTermId };

function plain(spans: readonly Span[]): string {
  return spans
    .map((span) => {
      switch (span.kind) {
        case "text":
        case "number":
        case "value":
          return span.text;
        default:
          return plain(span.children);
      }
    })
    .join("");
}

/** `text` as tokens the way the module splits it: plain words and the glossary terms it puts in bold. */
export function tokenizeRules(text: string): RulesToken[] {
  return richWords(text).spans.map((span): RulesToken => (span.kind === "keyword" ? { kind: "term", text: plain([span]), term: span.entry.id as GlossaryTermId } : { kind: "text", text: plain([span]) }));
}

/** Distinct terms of `text`, in order of first appearance. */
export function termsIn(text: string): GlossaryTermId[] {
  const seen: GlossaryTermId[] = [];
  for (const token of tokenizeRules(text)) {
    if (token.kind === "term" && !seen.includes(token.term)) seen.push(token.term);
  }
  return seen;
}

/** A stretch of a face's text, by UTF-16 offsets: `start` inclusive, `end` exclusive. */
export type TextRange = { readonly start: number; readonly end: number };
export type TunedRange = TextRange & { key: string; printed: number; way: "better" | "worse" };

/**
 * A face's text in the shape the web read before the face model moved to Rust: the words, the stretches of them a
 * Radiant face adds (`marks`) and the numbers the view moved (`tuned`), recovered from the spans. A span the
 * model puts between words (a formula value and the space before it) is not part of `full`, so it is skipped.
 */
export function legacyText(text: { full: string; spans: readonly Span[] }): { full: string; marks: TextRange[]; tuned?: TunedRange[] } {
  const marks: TextRange[] = [];
  const tuned: TunedRange[] = [];
  let at = 0;
  const walk = (spans: readonly Span[], inMark: boolean): void => {
    spans.forEach((span, index) => {
      const next = spans[index + 1];
      const leaf = (words: string): void => {
        if (words === "") return;
        if (inMark) {
          const last = marks[marks.length - 1];
          if (last !== undefined && last.end === at) marks[marks.length - 1] = { start: last.start, end: at + words.length };
          else marks.push({ start: at, end: at + words.length });
        }
        at += words.length;
      };
      switch (span.kind) {
        case "text":
          // A value is set after a space the model put there, at the end of the words before it.
          leaf(next?.kind === "value" && span.text.endsWith(" ") ? span.text.slice(0, -1) : span.text);
          break;
        case "number":
          tuned.push({ start: at, end: at + span.text.length, key: span.key, printed: span.printed, way: span.way });
          leaf(span.text);
          break;
        case "value":
          break;
        case "mark":
          walk(span.children, true);
          break;
        default:
          walk(span.children, inMark);
      }
    });
  };
  walk(text.spans, false);
  if (at !== text.full.length) throw new Error(`the spans say ${String(at)} characters of a text of ${String(text.full.length)}: ${text.full}`);
  return { full: text.full, marks, ...(tuned.length === 0 ? {} : { tuned }) };
}

/** The marked words themselves, for tests and accessible summaries. */
export function markedText(text: string, marks: readonly TextRange[]): string[] {
  return marks.map((range) => text.slice(range.start, range.end));
}

/** R277: the stretches of `radiant` that `base` does not have, as the face model's diff answers. */
export function radiantMarks(base: string, radiant: string): TextRange[] {
  return wordDiff(base, radiant).added;
}

export type RefMatch = { readonly start: number; readonly end: number; readonly id: string; readonly radiant: boolean };

/**
 * The names in `text` that point at one of `refs`, as the face model finds them (R279): a card whose text it is
 * and whose `refs` they are is asked for its face, and the `card` spans of that are read back.
 */
export function findRefs(text: string, refs: readonly CardDef[]): RefMatch[] {
  const base = CATALOG["core-001"];
  if (base === undefined) throw new Error("the catalog has no core-001");
  const host: CardDef = {
    ...base,
    id: "t-ref-host",
    refs: refs.map((def) => def.id),
    base: { ...base.base, text },
    radiant: { ...base.radiant, text },
  };
  const found: RefMatch[] = [];
  let at = 0;
  const walk = (spans: readonly Span[], card: { id: string; radiant: boolean } | null): void => {
    for (const span of spans) {
      if (span.kind === "card") {
        const start = at;
        walk(span.children, { id: span.defId, radiant: span.radiant });
        found.push({ start, end: at, id: span.defId, radiant: span.radiant });
      } else if (span.kind === "keyword" || span.kind === "mark") {
        walk(span.children, card);
      } else if (span.kind !== "value") {
        at += span.text.length;
      }
    }
  };
  walk(faceModel({ defId: host.id, def: host, radiant: false }).text.spans, null);
  return found;
}
