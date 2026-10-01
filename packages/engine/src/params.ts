// Declared numbers (docs/classic-sets.md B3.4 rule 5, R386): the numbers on a card that Degrade,
// Upgrade and KY's Constant may move, beyond cost, stats and numbered keywords.
//
// A card declares them in the catalog (`CardDef.params`, one `Param` per number, with its printed
// value on each face) and writes them into its texts as `{key}`. A card script never writes the
// literal: it reads `param(ctx, key)`, which is the number as it stands on the instance now — its
// face's printed value moved by the instance's tuning (`tuning.numbers`, a step count, and
// `tuning.set`, KY's Constant's "to 3") and held inside the number's bounds. `viewFor` carries the
// same values on the card's view (`CardView.params`, `paramsView`) for the client to fill the `{key}`s
// with, and a `preview` hook (R280) reads `paramValue`, so the text, the view and the resolution read
// one function and cannot disagree.
//
// A fused card (R77, R102) runs each ingredient's text, and each text reads its own declaration: the
// running part's ingredient definition (`work.PART_KEY`'s path) names which. The fused card's own
// declared numbers are its ingredients' — the first declaration of each key — and its tuning is one
// record, so a step on "damage" moves every ingredient's "damage".

import type { Param } from "@jackioh/shared";
import { defOf, fusedIdParts } from "./catalog";
import { TUNE_MIN_AMOUNT, PARAM_DEFAULT_STEP } from "./config";
import type { CardInstance, GameState } from "./state";
import { addStep, tidyTuning, tuningOf } from "./tuning";
import { partPathOf } from "./work";

/**
 * B3.4 rule 5: the numbers a definition declares. A catalog card's are its `params`; a fused
 * definition's (R77, R102) are its ingredients', the first declaration of each key kept. Empty for a
 * card that declares none.
 */
export function paramsOf(state: GameState, defId: string): readonly Param[] {
  const def = defOf(state, defId);
  if (def.params !== undefined) return def.params;
  const parts = fusedIdParts(defId);
  if (parts === null) return [];
  const out: Param[] = [];
  for (const part of parts) {
    for (const param of paramsOf(state, part)) {
      if (!out.some((held) => held.key === param.key)) out.push(param);
    }
  }
  return out;
}

/** The declaration of one key, or undefined when the definition declares no such number. */
export function paramDeclOf(state: GameState, defId: string, key: string): Param | undefined {
  return paramsOf(state, defId).find((param) => param.key === key);
}

/**
 * B3.4 rule 5: how far one Degrade or Upgrade moves a declared number — the `step` it declares, else
 * 1 for a number up to 5, 2 for 6 to 12, and a quarter of it above that (rounded), all read off the
 * face's printed value.
 */
export function paramStep(param: Param, printed: number): number {
  if (param.step !== undefined) return Math.max(1, Math.trunc(param.step));
  const size = Math.abs(printed);
  if (size <= PARAM_DEFAULT_STEP.small.upTo) return PARAM_DEFAULT_STEP.small.step;
  if (size <= PARAM_DEFAULT_STEP.medium.upTo) return PARAM_DEFAULT_STEP.medium.step;
  return Math.max(1, Math.round(size / PARAM_DEFAULT_STEP.largeDivisor));
}

/**
 * B3.4 rule 5, R386: a declared number's floor — its `min`, else "an amount never drops below 1",
 * or its printed value where that is lower (a number printed at 0 is never taken below it).
 */
function paramMin(param: Param, printed: number): number {
  return param.min ?? Math.min(TUNE_MIN_AMOUNT, printed);
}

function paramMax(param: Param): number {
  return param.max ?? Number.POSITIVE_INFINITY;
}

/** A declared number's value on a face, as `tuning` moves it: the one formula every reader shares. */
function valueWith(param: Param, radiant: boolean, tuning: Pick<CardInstance, "tuning">["tuning"], steps?: number): number {
  const printed = radiant ? param.radiant : param.base;
  const set = tuning?.set?.[param.key];
  const count = steps ?? tuning?.numbers?.[param.key] ?? 0;
  if (set === undefined && count === 0) return printed;
  const raw = (set ?? printed) + count * paramStep(param, printed);
  return Math.min(paramMax(param), Math.max(paramMin(param, printed), raw));
}

/**
 * B3.4 rule 5, R386: the value of the declared number `key` on `instance` now — pure, for `viewFor`
 * (`CardView.params`) and for a `preview` hook (R280). `defId` names the definition whose declaration
 * to read when it is not the instance's own (an ingredient of a fused card, `param`); `radiant` the
 * face, the instance's own by default. Throws for a key the definition does not declare: a card text
 * that reads a number it never declared is a catalog error, not a 0.
 */
export function paramValue(
  state: GameState,
  instance: Pick<CardInstance, "defId" | "radiant" | "tuning"> | null,
  key: string,
  options: { defId?: string; radiant?: boolean } = {},
): number {
  const defId = options.defId ?? instance?.defId;
  if (defId === undefined) throw new Error(`param "${key}": no card to read it on (B3.4 rule 5)`);
  const param = paramDeclOf(state, defId, key);
  if (param === undefined) throw new Error(`${defId} declares no number "${key}" (B3.4 rule 5, CardDef.params)`);
  return valueWith(param, options.radiant ?? instance?.radiant ?? false, instance?.tuning);
}

/**
 * B3.4 rule 5: what a card script reads in place of a literal — `param(ctx, "damage")` on a card
 * whose text says "Deal {damage} damage". The number is the running card's (`ctx.self`), on the face
 * that is running (`ctx.radiant`), read off the declaration of the text that is running: on a fused
 * card, the running ingredient's (`work.PART_KEY`). A script whose card has ceased to exist (R127,
 * `ctx.self` null) reads the printed value of its definition (`ctx.defId`).
 *
 * B5 E14, R546: the text that is running is named by `ctx.defId` where the run set it — a copier
 * (Classic #57 Echo) running a copied Spell's text reads that Spell's declared numbers, on its own
 * instance — and by the instance otherwise.
 */
export function param(
  ctx: {
    state: GameState;
    self: CardInstance | null;
    radiant: boolean;
    data?: Record<string, unknown>;
    defId?: string;
  },
  key: string,
): number {
  let defId = ctx.defId ?? ctx.self?.defId;
  if (defId === undefined) throw new Error(`param "${key}": no card to read it on (B3.4 rule 5)`);
  const path = ctx.data === undefined ? null : partPathOf(ctx.data);
  for (const index of path ?? []) {
    const parts = fusedIdParts(defId);
    const part = parts?.[index];
    if (part === undefined) break;
    defId = part;
  }
  return paramValue(ctx.state, ctx.self, key, { defId, radiant: ctx.radiant });
}

/**
 * B3.4 rule 5, R386: every declared number on the card as it stands, by key, for `CardView.params`.
 * Null for a card that declares none, so its view carries no key.
 */
export function paramsView(state: GameState, instance: CardInstance): Record<string, number> | null {
  const params = paramsOf(state, instance.defId);
  if (params.length === 0) return null;
  const out: Record<string, number> = {};
  for (const param of params) out[param.key] = valueWith(param, instance.radiant, instance.tuning);
  return out;
}

/**
 * B3.4 rule 3's "Number" row: the declared numbers one step in `direction` would change — up moves a
 * number the way its `better` says for an Upgrade, the other way for a Degrade — with how far each
 * would move, in declaration order. A number already at the bound it would move past is not one.
 */
export function steppableParams(
  state: GameState,
  instance: CardInstance,
  change: "upgrade" | "degrade",
): { param: Param; steps: number; delta: number }[] {
  return paramsOf(state, instance.defId).flatMap((param) => {
    const towardBetter = change === "upgrade" ? 1 : -1;
    const steps = param.better === "up" ? towardBetter : -towardBetter;
    const now = valueWith(param, instance.radiant, instance.tuning);
    const count = (instance.tuning?.numbers?.[param.key] ?? 0) + steps;
    const next = valueWith(param, instance.radiant, instance.tuning, count);
    return next === now ? [] : [{ param, steps, delta: next - now }];
  });
}

/** B3.4 rule 4: record `steps` more steps on a declared number (`tuning.numbers`). */
export function stepParam(instance: CardInstance, key: string, steps: number): void {
  const tuning = tuningOf(instance);
  tuning.numbers = addStep(tuning.numbers, key, steps);
  tidyTuning(instance);
}

/**
 * Classic+ #41 KY's Constant: set a declared number outright (`tuning.set`). The steps recorded
 * before it are spent — the number is the set value now — and a later Degrade or Upgrade steps from it.
 */
export function setParam(instance: CardInstance, key: string, value: number): void {
  const tuning = tuningOf(instance);
  tuning.set = { ...(tuning.set ?? {}), [key]: value };
  const numbers = { ...(tuning.numbers ?? {}) };
  delete numbers[key];
  tuning.numbers = numbers;
  tidyTuning(instance);
}
