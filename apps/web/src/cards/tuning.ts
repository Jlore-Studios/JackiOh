// Degrade and Upgrade on a face (docs/classic-sets.md B3.4, R386), which players read as Nerf and Buff
// (R1320): what the view says changed on a card, read for the marks a face draws and the words the
// inspect overlays print. The verdicts keep the engine's words as data (`data-tuned="upgraded"`); the
// words a player reads are `VERDICT_WORD`'s.
//
// The view carries two things about a tuned card (CardView, SPEC §10.8): its declared numbers as they
// stand now (`params`, which fill the face's `{key}`s) and the record of what Degrade, Upgrade and KY's
// Constant changed (`tuning`: a stat delta, keywords added and removed, a step per numbered keyword or
// X, a value set outright). This module compares those with the public catalog and nothing else
// (CLAUDE.md rule 7), and names each change better or worse for the card's controller:
//
// - a stat: up is better;
// - a keyword: one added is better, one removed is worse (a Buff adds, a Nerf removes, §6.3);
// - a numbered keyword or X (`tuning.x`): more is better, except Tribute, where less is (§6.3);
// - a declared number: its live value against the face's printed one, the way its `better` says
//   (`CardDef.params`, B3.4 rule 5);
// - a number set outright that is not a declared number (KY's Constant on a numbered keyword) is
//   neither: it is "set to N".
//
// The face marks each by shape as well as colour (cardstate.css): a number that moved sits in a dotted
// box with ▲ (better) or ▼ (worse) after it, a tuned stat wears the same glyph as a pip, an added
// keyword is a "+" chip and a removed one a struck "−" chip, and the card as a whole carries a mark
// that says Buffed (▲, every change better), Nerfed (▼, every change worse) or Tuned (◆, mixed).
// The cost change is the card's live cost (`CardView.cost`), which the gem already tones (model.ts).
//
// `filledText` fills a face's text exactly as `fillParams` does and records where each number that
// moved stands in the result, so the rules text can box it; a test holds the two to the same string.

import {
  PARAM_PLACEHOLDER,
  keywordKey,
  type CardDef,
  type Keyword,
  type KeywordKind,
  type Param,
  type Tuning,
} from "@jackioh/shared";

import type { TextRange } from "./radiantDiff.ts";

/** Which way a change went for the card's controller. */
export type TuneWay = "better" | "worse";

/** One change the view says a card carries, as the client words and marks it. */
export type TuneChange =
  | { kind: "stat"; stat: "attack" | "health"; delta: number; way: TuneWay }
  | { kind: "keyword"; keyword: string; added: boolean; way: TuneWay }
  | { kind: "count"; key: string; delta: number; way: TuneWay }
  | { kind: "number"; key: string; printed: number; value: number; way: TuneWay }
  | { kind: "set"; key: string; value: number };

/** The card as a whole: every change better, every change worse, or a mix (or a number set outright). */
export type TuneVerdict = "upgraded" | "degraded" | "tuned";

export type FaceTuning = {
  verdict: TuneVerdict;
  changes: readonly TuneChange[];
  /** The stat pips: which way each stat was tuned; absent for a stat no change moved. */
  attack?: TuneWay;
  health?: TuneWay;
  /** Keywords a Buff added, which the face prints as "+" chips. */
  added: readonly Keyword[];
  /** Keyword kinds a Nerf removed, which the face prints as struck chips. */
  removed: readonly KeywordKind[];
};

/** A number in a face's text that the view says moved off its printed value. */
export type TunedRange = TextRange & { key: string; printed: number; value: number; way: TuneWay };

/** §6.3: a numbered keyword or X is better higher, except these, which are better lower. */
export const LESS_IS_BETTER: readonly string[] = ["Tribute"];

/** The glyph each way and each verdict is drawn with, so a mark never rests on colour alone. */
export const WAY_GLYPH: Readonly<Record<TuneWay, string>> = { better: "▲", worse: "▼" };
export const VERDICT_GLYPH: Readonly<Record<TuneVerdict, string>> = { upgraded: "▲", degraded: "▼", tuned: "◆" };

/** The word each verdict is printed with (the ribbon, the mark's title): Buffed and Nerfed (R1320). */
export const VERDICT_WORD: Readonly<Record<TuneVerdict, string>> = {
  upgraded: "Buffed",
  degraded: "Nerfed",
  tuned: "Tuned",
};

/** The minus sign the catalog prints ("−3/−3"), not a hyphen. */
export const MINUS = "\u2212";

function wayOf(delta: number, lessIsBetter: boolean): TuneWay {
  return delta > 0 !== lessIsBetter ? "better" : "worse";
}

function paramsOf(def: CardDef | undefined): readonly Param[] {
  return def?.params ?? [];
}

/**
 * B3.4, R386: what changed on a card, from its view's `tuning` and `params` against its printed face.
 * Null when nothing did: no tuning, and every declared number at its printed value.
 */
export function faceTuning(
  def: CardDef | undefined,
  radiant: boolean,
  tuning: Tuning | undefined,
  values: Readonly<Record<string, number>> | undefined,
): FaceTuning | null {
  const changes: TuneChange[] = [];
  const out: { attack?: TuneWay; health?: TuneWay } = {};

  for (const stat of ["attack", "health"] as const) {
    const delta = tuning?.[stat] ?? 0;
    if (delta === 0) continue;
    const way = wayOf(delta, false);
    out[stat] = way;
    changes.push({ kind: "stat", stat, delta, way });
  }

  const added = tuning?.addKeywords ?? [];
  const removed = tuning?.removeKeywords ?? [];
  for (const keyword of added) changes.push({ kind: "keyword", keyword: keywordKey(keyword), added: true, way: "better" });
  for (const kind of removed) changes.push({ kind: "keyword", keyword: kind, added: false, way: "worse" });

  for (const [key, delta] of Object.entries(tuning?.x ?? {})) {
    if (delta === 0) continue;
    changes.push({ kind: "count", key, delta, way: wayOf(delta, LESS_IS_BETTER.includes(key)) });
  }

  const declared = paramsOf(def);
  for (const param of declared) {
    const value = values?.[param.key];
    const printed = radiant ? param.radiant : param.base;
    if (value === undefined || value === printed) continue;
    changes.push({ kind: "number", key: param.key, printed, value, way: wayOf(value - printed, param.better === "down") });
  }

  // KY's Constant's "to N" on something that is not a declared number (those are read above).
  for (const [key, value] of Object.entries(tuning?.set ?? {})) {
    if (declared.some((param) => param.key === key)) continue;
    changes.push({ kind: "set", key, value });
  }

  if (changes.length === 0) return null;
  return { verdict: verdictOf(changes), changes, ...out, added, removed };
}

function verdictOf(changes: readonly TuneChange[]): TuneVerdict {
  const ways = changes.map((change) => (change.kind === "set" ? null : change.way));
  if (ways.every((way) => way === "better")) return "upgraded";
  if (ways.every((way) => way === "worse")) return "degraded";
  return "tuned";
}

/** A declared number's key in words: "drawLimit" is "Draw limit". */
export function keyWords(key: string): string {
  const spaced = key.replace(/([a-z0-9])([A-Z])/g, "$1 $2").toLowerCase();
  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}

function signed(delta: number): string {
  return delta > 0 ? `+${String(delta)}` : `${MINUS}${String(Math.abs(delta))}`;
}

/** One change in a player's words: "+2 Attack", "Gained Rush", "Lost Taunt", "Tribute −1", "Damage 2 → 3". */
export function changeWords(change: TuneChange): string {
  switch (change.kind) {
    case "stat":
      return `${signed(change.delta)} ${change.stat === "attack" ? "Attack" : "Health"}`;
    case "keyword":
      return change.added ? `Gained ${change.keyword}` : `Lost ${change.keyword}`;
    case "count":
      return `${change.key} ${signed(change.delta)}`;
    case "number":
      return `${keyWords(change.key)} ${String(change.printed)} → ${String(change.value)}`;
    case "set":
      return `${keyWords(change.key)} set to ${String(change.value)}`;
  }
}

/** The card's mark as one line: "Buffed: +2 Attack; Gained Rush". */
export function tuningSummary(tuning: FaceTuning): string {
  return `${VERDICT_WORD[tuning.verdict]}: ${tuning.changes.map(changeWords).join("; ")}`;
}

/** A moved number's tooltip: "Better than printed: 2" (the number bare, since "(2)" reads as a cost, R432). */
export function tunedRangeWords(range: Pick<TunedRange, "printed" | "way">): string {
  return `${range.way === "better" ? "Better" : "Worse"} than printed: ${String(range.printed)}`;
}

/**
 * A face's text with its `{key}`s filled in, as `fillParams` fills it, and where each number the view
 * moved off its printed value stands in the result (the digits alone, not the words that agree with
 * it). `values` are the view's (`CardView.params`); none, or a number at its printed value, marks
 * nothing.
 */
export function filledText(
  def: Pick<CardDef, "params" | "base" | "radiant">,
  face: "base" | "radiant",
  values?: Readonly<Record<string, number>>,
): { text: string; tuned: TunedRange[] } {
  const text = def[face].text;
  const params = def.params;
  if (params === undefined || params.length === 0) return { text, tuned: [] };
  const tuned: TunedRange[] = [];
  let out = "";
  let from = 0;
  for (const match of text.matchAll(PARAM_PLACEHOLDER)) {
    const at = match.index;
    const whole = match[0];
    out += text.slice(from, at);
    from = at + whole.length;
    const key = match[1] ?? "";
    const param = params.find((candidate) => candidate.key === key);
    if (param === undefined) {
      out += whole;
      continue;
    }
    const printed = param[face];
    const value = values?.[key] ?? printed;
    const digits = String(value);
    if (value !== printed) {
      tuned.push({
        start: out.length,
        end: out.length + digits.length,
        key,
        printed,
        value,
        way: wayOf(value - printed, param.better === "down"),
      });
    }
    const one = match[2];
    const many = match[3];
    out += one === undefined || many === undefined ? digits : `${digits} ${value === 1 ? one : many}`;
  }
  out += text.slice(from);
  return { text: out, tuned };
}
