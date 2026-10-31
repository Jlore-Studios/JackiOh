// Where a card's text names another card (SPEC §10.10, R279). The catalog lists the cards and tokens
// a card's text names (`CardDef.refs`), and crates/cards/tests/cross/references.rs proves the list
// with one naming rule, which this module reads the same way. Presentation only (CLAUDE.md rule 7):
// the faces a reference shows are printed catalog faces, public by §5.1.

import type { CardDef } from "@jackioh/shared";

export type RefMatch = { readonly start: number; readonly end: number; readonly id: string; readonly radiant: boolean };

const WORD = /[A-Za-z0-9'-]/;
const PLURAL = "s";
const RADIANT_WORD = "Radiant ";

/** R903: one named alias — "Coin" names The Coin, which M #27's texts call by the short name. */
const ALIASES: Readonly<Record<string, readonly string[]>> = { "The Coin": ["Coin"] };

/** A card's names in a text: its name, and its name before a parenthesis (full-width in Chinese, R1301). */
export function namesOf(def: Pick<CardDef, "name">): string[] {
  const bare = def.name.replace(/\s*[(（].*[)）]\s*$/, "");
  const out = bare === def.name ? [def.name] : [def.name, bare];
  return [...out, ...(ALIASES[def.name] ?? [])];
}

function isWordChar(char: string | undefined): boolean {
  return char !== undefined && WORD.test(char);
}

/**
 * Rules phrases that hold a card's name without naming it (R961): "Jade Counter" holds "Jade"
 * (Meditative #39.2), which names nothing inside it; "Fortify Mind" (M #22.1) holds "Fortify"
 * (Training Grounds #12) and still names itself (R2198004).
 */
const RULES_PHRASES: readonly string[] = ["Jade Counter", "Fortify Mind"];

/**
 * Whether the match at [start, end) lies inside a rules phrase, which reads as the phrase's word. A
 * match that is the whole phrase is not inside it (R2198004).
 */
function insideRulesPhrase(text: string, start: number, end: number): boolean {
  return RULES_PHRASES.some((phrase) => {
    let from = 0;
    for (;;) {
      const at = text.indexOf(phrase, from);
      if (at < 0) return false;
      if (at <= start && end <= at + phrase.length && !(at === start && end === at + phrase.length)) return true;
      from = at + 1;
    }
  });
}

/** Every whole-word occurrence of `name` (plural included) in `text`, as [start, end) pairs. */
function occurrences(text: string, name: string): { start: number; end: number }[] {
  const found: { start: number; end: number }[] = [];
  let from = 0;
  for (;;) {
    const at = text.indexOf(name, from);
    if (at < 0) return found;
    let end = at + name.length;
    if (text[end] === PLURAL) end += 1;
    if (!isWordChar(text[at - 1]) && !isWordChar(text[end]) && !insideRulesPhrase(text, at, end)) {
      found.push({ start: at, end });
    }
    from = at + 1;
  }
}

/**
 * The names in `text` that point at one of `refs`, in text order and never overlapping: where two
 * names start at the same place the longer wins, and a name inside another name's stretch is not
 * a name of its own. `radiantWord` is the word before a name that points it at the Radiant face: a
 * Chinese text (R1301) writes "光辉" right before the name, with no space.
 */
export function findRefs(text: string, refs: readonly CardDef[], radiantWord: string = RADIANT_WORD): RefMatch[] {
  const candidates: RefMatch[] = [];
  for (const def of refs) {
    for (const name of namesOf(def)) {
      for (const hit of occurrences(text, name)) {
        const radiant = text.slice(Math.max(0, hit.start - radiantWord.length), hit.start) === radiantWord;
        candidates.push({ start: hit.start, end: hit.end, id: def.id, radiant });
      }
    }
  }
  candidates.sort((a, b) => a.start - b.start || b.end - a.end);
  const kept: RefMatch[] = [];
  let reach = -1;
  for (const candidate of candidates) {
    if (candidate.start < reach) continue;
    kept.push(candidate);
    reach = candidate.end;
  }
  return kept;
}
