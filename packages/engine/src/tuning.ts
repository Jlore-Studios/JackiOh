// Degrade and Upgrade's lasting changes to a card (docs/classic-sets.md B3.4, R386): the readers every
// other module uses. A card's `tuning` rides it through every zone (R78 leaves it alone); what writes
// it is `effects/tune.ts`.
//
// The numbered keywords a script reads as a flag rather than as a `Keyword` — Echo (`staticFlags.echo`),
// Activate (`ActivationDecl.uses`), Tribute (`staticFlags.tribute`) — and an X-cost card's X are read
// through `tunedCount` wherever the engine reads them, so a Degrade or Upgrade of that number is felt
// where the number is used. The numbered keywords that are `Keyword`s (Armor, Lucky, Brittle, Spell
// Damage) are tuned in the layers (§10.4) by the same step (`tunedKeywords`, read by `layers.faceOf`).
//
// What each field of a `Tuning` holds:
//   - `attack`, `health`: the stats changes, summed — a delta beside the layer-4 buffs, which moves
//     max health on the field and the face a card will enter with anywhere else (B3.4 rule 6).
//   - `addKeywords`: the keywords an Upgrade added; `removeKeywords`: the printed keyword kinds a
//     Degrade removed (a granted keyword a Degrade removes is simply taken off the instance).
//   - `x`: a step count per numbered keyword or X (`X_KEY`), each step `TUNE_X_STEP`.
//   - `numbers`: a step count per declared number (`CardDef.params` key), each step that number's
//     own (`params.paramStep`), so a card made Radiant keeps its steps on its Radiant numbers.
//   - `set`: a number KY's Constant set outright; the steps after it count from it (`tunedCount`).
//     A declared number's key is a catalog `params` key (camelCase), a keyword's its kind (capitalised),
//     so the two never share a key.
// The cost change is the card's `costMod` (R65), never a field here.

import type { Keyword, KeywordKind, Tuning } from "@jackioh/shared";
import { TUNE_MIN_AMOUNT } from "./config";
import type { CardInstance } from "./state";

/** B3.4: the least a tuned number may come to — "an amount never drops below 1" (`TUNE_MIN_AMOUNT`). */
export const TUNED_FLOOR = TUNE_MIN_AMOUNT;

/** B3.4 rule 3: the tuning key of an X-cost card's X (`xOf`). */
export const X_KEY = "X";

/**
 * B3.4 rule 3: the numbered keywords that are `Keyword`s, which the layers tune (`tunedKeywords`).
 * Armor and Lucky print as keywords on any card; Brittle is also a count once started (`brittle.ts`);
 * Spell Damage is E6's.
 */
export const LAYERED_NUMBERED_KEYWORDS = ["Armor", "Lucky", "Brittle", "Spell Damage"] as const satisfies readonly KeywordKind[];

/**
 * B3.4 rule 3: the numbered keywords a script declares as data rather than as a `Keyword`, read by
 * their owners through `tunedCount` under these keys: Echo (`echo.ts`), Activate (B3.2's
 * `ActivationDecl.uses`), Tribute (`staticFlags.tribute`, where less is better).
 */
export const FLAG_NUMBERED_KEYS = ["Echo", "Activate", "Tribute"] as const;

/**
 * B3.4, R386: the value a card's numbered keyword or X has now — its printed value (or the value KY's
 * Constant set outright, `tuning.set`) moved by the card's tuning steps for `key`. A number the card
 * does not print (0) is never tuned into existence. Never below `min`.
 */
export function tunedCount(
  instance: Pick<CardInstance, "tuning">,
  key: string,
  printed: number,
  min: number = TUNED_FLOOR,
): number {
  if (printed <= 0) return printed;
  const set = instance.tuning?.set?.[key];
  const step = instance.tuning?.x?.[key] ?? 0;
  if (set === undefined && step === 0) return printed;
  return Math.max(min, (set ?? printed) + step);
}

/**
 * B2.7, B3.4: the X an X-cost card counts on the field and as it resolves — the X it was played for
 * moved by its X steps, never below 1 (R348's floor) — and 0 for a card that has no chosen X (a
 * Recruit, a copy, a card outside a play), as R65 reads an X card's cost off the field.
 */
export function xOf(instance: Pick<CardInstance, "x" | "tuning">): number {
  const x = instance.x;
  if (x === undefined) return 0;
  return tunedCount(instance, X_KEY, x);
}

/**
 * §10.4 layer 1 with B3.4 on top: a face's printed keywords as the card's tuning leaves them. The
 * printed kinds a Degrade removed go, a numbered keyword a Degrade, an Upgrade or KY's Constant moved
 * prints its tuned value (several printed entries of one kind — a fused Armor 7 and Armor 3 — are
 * tuned as their sum and print as one), and the keywords an Upgrade added are appended.
 */
export function tunedKeywords(keywords: readonly Keyword[], instance: Pick<CardInstance, "tuning">): Keyword[] {
  const tuning = instance.tuning;
  if (tuning === undefined) return [...keywords];
  const removed = new Set<KeywordKind>(tuning.removeKeywords ?? []);
  let out: Keyword[] = keywords.filter((keyword) => !removed.has(keyword.kind));

  for (const kind of LAYERED_NUMBERED_KEYWORDS) {
    if (tuning.set?.[kind] === undefined && (tuning.x?.[kind] ?? 0) === 0) continue;
    const printed = numberedSum(out, kind);
    if (printed === null) continue;
    const first = out.findIndex((keyword) => keyword.kind === kind);
    const value = tunedCount(instance, kind, printed);
    out = out.filter((keyword, at) => keyword.kind !== kind || at === first);
    out[first] = { kind, n: value };
  }

  for (const keyword of tuning.addKeywords ?? []) {
    const stacks = keyword.kind === "Armor" || keyword.kind === "Lucky";
    if (stacks || !out.some((held) => held.kind === keyword.kind)) out.push(keyword);
  }
  return out;
}

/** The sum of a numbered keyword's entries in a list, or null when the list has none of that kind. */
export function numberedSum(keywords: readonly Keyword[], kind: KeywordKind): number | null {
  let found = false;
  let sum = 0;
  for (const keyword of keywords) {
    if (keyword.kind !== kind || !("n" in keyword)) continue;
    found = true;
    sum += keyword.n;
  }
  return found ? sum : null;
}

/** B3.4: whether the card carries any tuning at all. */
export function isTuned(instance: Pick<CardInstance, "tuning">): boolean {
  const tuning: Tuning | undefined = instance.tuning;
  if (tuning === undefined) return false;
  return (
    (tuning.attack ?? 0) !== 0 ||
    (tuning.health ?? 0) !== 0 ||
    (tuning.addKeywords?.length ?? 0) > 0 ||
    (tuning.removeKeywords?.length ?? 0) > 0 ||
    Object.values(tuning.x ?? {}).some((step) => step !== 0) ||
    Object.values(tuning.numbers ?? {}).some((step) => step !== 0) ||
    Object.keys(tuning.set ?? {}).length > 0
  );
}

/** The card's tuning record, made on first write. */
export function tuningOf(instance: CardInstance): Tuning {
  instance.tuning ??= {};
  return instance.tuning;
}

/** Add `delta` to a step count, dropping a count that comes back to 0 so an untuned card stores nothing. */
export function addStep(record: Record<string, number> | undefined, key: string, delta: number): Record<string, number> {
  const out = { ...(record ?? {}) };
  const next = (out[key] ?? 0) + delta;
  if (next === 0) delete out[key];
  else out[key] = next;
  return out;
}

/**
 * Drop every empty field, and the record itself once nothing is left, so a card whose changes have
 * cancelled out stores and hashes exactly as a card never tuned (§9.3).
 */
export function tidyTuning(instance: Pick<CardInstance, "tuning">): void {
  const tuning = instance.tuning;
  if (tuning === undefined) return;
  if ((tuning.attack ?? 0) === 0) delete tuning.attack;
  if ((tuning.health ?? 0) === 0) delete tuning.health;
  if ((tuning.addKeywords?.length ?? 0) === 0) delete tuning.addKeywords;
  if ((tuning.removeKeywords?.length ?? 0) === 0) delete tuning.removeKeywords;
  for (const field of ["x", "numbers", "set"] as const) {
    const record = tuning[field];
    if (record !== undefined && Object.keys(record).length === 0) delete tuning[field];
  }
  if (Object.keys(tuning).length === 0) delete instance.tuning;
}

/** A deep copy, so a copy's tuning shares nothing with its source's (R57, B3.4 rule 4). */
export function copyTuning(tuning: Tuning | undefined): Tuning | undefined {
  return tuning === undefined ? undefined : (JSON.parse(JSON.stringify(tuning)) as Tuning);
}

/**
 * R102, B3.4 rule 4: a Fuse sums its ingredients' tuning — the stats and every step count add up,
 * the keyword changes unite, and a number KY's Constant set is kept from the first ingredient that
 * set it. Undefined when no ingredient carries any.
 */
export function sumTunings(tunings: readonly (Tuning | undefined)[]): Tuning | undefined {
  const holder: Pick<CardInstance, "tuning"> = { tuning: {} };
  const out = holder.tuning ?? {};
  for (const tuning of tunings) {
    if (tuning === undefined) continue;
    out.attack = (out.attack ?? 0) + (tuning.attack ?? 0);
    out.health = (out.health ?? 0) + (tuning.health ?? 0);
    for (const keyword of tuning.addKeywords ?? []) {
      const stacks = keyword.kind === "Armor" || keyword.kind === "Lucky";
      if (stacks || !(out.addKeywords ?? []).some((held) => held.kind === keyword.kind)) {
        out.addKeywords = [...(out.addKeywords ?? []), keyword];
      }
    }
    for (const kind of tuning.removeKeywords ?? []) {
      if (!(out.removeKeywords ?? []).includes(kind)) out.removeKeywords = [...(out.removeKeywords ?? []), kind];
    }
    for (const [key, step] of Object.entries(tuning.x ?? {})) out.x = addStep(out.x, key, step);
    for (const [key, step] of Object.entries(tuning.numbers ?? {})) out.numbers = addStep(out.numbers, key, step);
    for (const [key, value] of Object.entries(tuning.set ?? {})) {
      if (out.set?.[key] === undefined) out.set = { ...(out.set ?? {}), [key]: value };
    }
  }
  tidyTuning(holder);
  return holder.tuning;
}
