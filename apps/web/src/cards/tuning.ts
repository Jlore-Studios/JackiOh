// B3.4, R386; R1320: face marks present Degrade and Upgrade as Nerf and Buff while verdict data keeps
// the engine's words. Compare `tuning` and `params` with the public catalog only (SPEC §10.8;
// CLAUDE.md rule 7). Stats and keywords use their player-facing direction; numbered keywords and X are
// greater-is-better except Tribute (§6.3). Declared numbers use `better` (B3.4 rule 5); constants set
// outright are neither better nor worse. `filledText` stays identical to `fillParams`.

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

/** One change in a player's words: "+2 Attack", "Gained Rush", "Damage 2 → 3". */
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

export function tuningSummary(tuning: FaceTuning): string {
  return `${VERDICT_WORD[tuning.verdict]}: ${tuning.changes.map(changeWords).join("; ")}`;
}

/** A moved number's tooltip: "Better than printed: 2" (the number bare, since "(2)" reads as a cost, R432). */
export function tunedRangeWords(range: Pick<TunedRange, "printed" | "way">): string {
  return `${range.way === "better" ? "Better" : "Worse"} than printed: ${String(range.printed)}`;
}

/**
 * A face's text with its `{key}`s filled in, as `fillParams` fills it, and where each number the view
 * moved off its printed value stands in the result (the digits alone). `values` is `CardView.params`.
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
