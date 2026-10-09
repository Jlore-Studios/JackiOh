// Where a card's text names another card (SPEC §10.10, R279).
//
// The catalog lists, per card, the cards and tokens its text names (`CardDef.refs`, by id), and
// crates/cards/tests/cross/references.rs proves the list against the texts with one naming rule,
// which this module reads the same way: a card is named by its name, or by its name before a
// parenthesis ("Call to Chaos (Core Edition)" is named "Call to Chaos"), alone or with a plural "s",
// standing as whole words — the characters either side are not letters, digits, an apostrophe or a
// hyphen, so "CN-Viral" does not name "CN-Virus".
//
// A name the text calls Radiant ("a Radiant CN-Virus", "five Radiant Rush Tokens") points at that
// card's Radiant face; any other points at its base face ("a base Right-house defender").
// R903 adds one alias: "Coin" names The Coin.
//
// Presentation only (CLAUDE.md rule 7): the faces a reference shows are printed catalog faces,
// public by §5.1.

import type { CardDef } from "@jackioh/shared";

/** A name in a text that points at a card: where it stands, which card, and which face. */
export type RefMatch = { readonly start: number; readonly end: number; readonly id: string; readonly radiant: boolean };

const WORD = /[A-Za-z0-9'-]/;
const PLURAL = "s";
/** The word before a name that makes it point at the Radiant face. */
const RADIANT_WORD = "Radiant ";

/** R903: one named alias — "Coin" names The Coin, which M #27's texts call by the short name. */
const ALIASES: Readonly<Record<string, readonly string[]>> = { "The Coin": ["Coin"] };

/**
 * The names a text may call a card by: its name, and its name before a parenthesis (a Chinese name's
 * full-width one too, "混沌召唤（核心版）", R1301).
 */
export function namesOf(def: Pick<CardDef, "name">): string[] {
  const bare = def.name.replace(/\s*[(（].*[)）]\s*$/, "");
  const out = bare === def.name ? [def.name] : [def.name, bare];
  return [...out, ...(ALIASES[def.name] ?? [])];
}

function isWordChar(char: string | undefined): boolean {
  return char !== undefined && WORD.test(char);
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
    if (!isWordChar(text[at - 1]) && !isWordChar(text[end])) found.push({ start: at, end });
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
