// "A number on a card" (docs/classic-sets.md B3.4, R386): the numbers Degrade, Upgrade and Classic+
// #41 KY's Constant read and move — the card's own cost (never an X), its attack and health (a Unit,
// or an Animated card, which prints the stats of the Unit it becomes), a numbered keyword's value
// (Armor, Lucky, Spell Damage, Brittle, Echo, Activate, Tribute) and a declared number (`params.ts`).
// This module reads them; `effects/tune.ts` moves them.
//
// Every read is a pure function of the card as it stands, so a card script may ask it (KY's
// Constant's "a card in your hand with a number that isn't already 3" is a `targetChecks` predicate
// over `numbersOn`) and `legalActions`, `viewFor` and the verb all read the same answer.

import type { KeywordKind } from "@jackioh/shared";
import { hasKeyword } from "@jackioh/shared";
import { activeBrittleCount } from "./brittleCount";
import { cardTypeOf } from "./faces";
import { cardKeywords, printedKeywordsOf, statsWithBuffs, unitView } from "./layers";
import { isXCost, printedCost } from "./mana";
import { paramsOf, paramValue } from "./params";
import { flagsOf, scriptOf } from "./scripts";
import type { CardInstance, GameState } from "./state";
import { numberedSum, tunedCount } from "./tuning";

/** B3.4 rule 3: the numbered keywords, by the tuning key each is read under (`tuning.tunedCount`). */
export type NumberedKey = "Armor" | "Lucky" | "Spell Damage" | "Brittle" | "Echo" | "Activate" | "Tribute";

/** Which number on a card: its cost, attack or health, a numbered keyword, or a declared number. */
export type NumberRef =
  | { kind: "cost" }
  | { kind: "attack" }
  | { kind: "health" }
  | { kind: "keyword"; key: NumberedKey }
  | { kind: "param"; key: string };

/**
 * One number on a card as `numbersOn` lists it: `id` names it in a prompt's option and in a
 * continuation's data (`numberRefId`, `parseNumberRef`), `label` is what a client shows, `value` what
 * it is now.
 */
export type NumberOnCard = { id: string; ref: NumberRef; label: string; value: number };

/**
 * A numbered keyword on a card: its value now, which way is better for the card's controller, and
 * the printed value its tuning steps from (`tuning.tunedCount`) — null for a Brittle count in force,
 * which is a count on the instance and moves itself (B3.3 rule 5).
 */
export type NumberedKeyword = { key: NumberedKey; value: number; better: "up" | "down"; printed: number | null };

/** The numbered keywords that print as `Keyword`s, in the order `numbersOn` lists them. */
const PRINTED_NUMBERED: readonly Extract<NumberedKey, KeywordKind>[] = ["Armor", "Lucky", "Spell Damage"];

/** The field's unit row: where a card's stats are read through all five layers (§10.4). */
function inUnitRow(card: CardInstance): boolean {
  return card.zone.z === "field" && card.zone.row === "units";
}

/**
 * R65, B3.4 rule 3: the card's own cost — `costOverride` or its printed cost (a computed or embiggen
 * price where R65 reads one), plus its `costMod` — with no player discount, since a Degrade changes
 * the card and not a price for a play. Null for an X-cost card, whose X is no cost a number may move,
 * and for an always-playable one (R663, Glitch), whose (0) nothing moves.
 */
export function ownCost(state: GameState, card: CardInstance): number | null {
  if (isXCost(state, card) || scriptOf(card).staticFlags?.alwaysPlayable === true) return null;
  return (card.costOverride ?? printedCost(state, card)) + card.costMod;
}

/**
 * B3.4 rule 3: whether the card has stats a number may move — a Unit, or an Animated card, which
 * prints the attack and health of the Unit it becomes (B3.1 rule 1).
 */
export function hasStats(state: GameState, card: CardInstance): boolean {
  if (cardTypeOf(state, card) === "Unit") return true;
  const keywords = cardKeywords(state, card);
  return hasKeyword(keywords, "Animated") || hasKeyword(keywords, "Animated on your turn");
}

/**
 * B3.4 rule 6: the card's attack and current health as they stand — through all five layers in the
 * unit row (§10.4), and elsewhere its face with its buffs and tuning (§10.4 layers 1 to 4, R243) less
 * any damage it kept (an Animated card back in its backrow, B3.1 rule 5). Null for a card with no
 * stats (`hasStats`).
 */
export function currentStats(state: GameState, card: CardInstance): { attack: number; health: number } | null {
  if (!hasStats(state, card)) return null;
  if (inUnitRow(card)) {
    const view = unitView(state, card);
    return { attack: view.attack, health: view.health };
  }
  const stats = statsWithBuffs(state, card);
  return { attack: Math.max(0, stats.attack), health: stats.maxHealth - card.damage };
}

/**
 * B3.4 rule 3's numbered keywords on the card, with their values now: Armor, Lucky and Spell Damage
 * its running face prints (and no Degrade removed); its Brittle (the count in force, else the printed
 * one that has not started); and the Echo, Activate and Tribute its text declares (`staticFlags.echo`,
 * the numbered `ActivationDecl.uses`, `staticFlags.tribute`). A Vanilla card prints and declares none
 * of them (§6.3), though a Brittle count it was given stays (B3.3 rule 5).
 */
export function numberedKeywordsOn(state: GameState, card: CardInstance): NumberedKeyword[] {
  const out: NumberedKeyword[] = [];
  const add = (key: NumberedKey, printed: number, better: "up" | "down"): void => {
    if (printed > 0) out.push({ key, value: tunedCount(card, key, printed), better, printed });
  };
  const removed = new Set<KeywordKind>(card.tuning?.removeKeywords ?? []);
  const keywords = card.vanilla ? [] : printedKeywordsOf(state, card);
  for (const key of PRINTED_NUMBERED) {
    if (!removed.has(key)) add(key, numberedSum(keywords, key) ?? 0, "up");
  }
  const count = activeBrittleCount(card);
  if (count !== null) {
    if (count > 0) out.push({ key: "Brittle", value: count, better: "up", printed: null });
  } else {
    add("Brittle", numberedSum(keywords, "Brittle") ?? 0, "up");
  }
  const flags = flagsOf(card);
  add("Echo", flags.echo ?? 0, "up");
  add("Activate", activateUses(card) ?? 0, "up");
  add("Tribute", flags.tribute ?? 0, "down");
  return out;
}

/**
 * B3.2 rule 9, B3.4: the printed N of the card's first numbered Activate ability ("Activate" is 1,
 * "Activate N" is N), or null when it has none — Activate ♾️ has no number to move.
 */
function activateUses(card: CardInstance): number | null {
  for (const ability of scriptOf(card).activations ?? []) {
    if (typeof ability.uses === "number" && ability.uses > 0) return ability.uses;
  }
  return null;
}

/** The string a `NumberRef` travels as: in a prompt's option and in a continuation's data. */
export function numberRefId(ref: NumberRef): string {
  if (ref.kind === "keyword") return `keyword:${ref.key}`;
  if (ref.kind === "param") return `param:${ref.key}`;
  return ref.kind;
}

const NUMBERED_KEYS: readonly NumberedKey[] = ["Armor", "Lucky", "Spell Damage", "Brittle", "Echo", "Activate", "Tribute"];

/** The inverse of `numberRefId`, or null for a string that names no number. */
export function parseNumberRef(id: string): NumberRef | null {
  if (id === "cost" || id === "attack" || id === "health") return { kind: id };
  if (id.startsWith("keyword:")) {
    const key = id.slice("keyword:".length);
    return (NUMBERED_KEYS as readonly string[]).includes(key) ? { kind: "keyword", key: key as NumberedKey } : null;
  }
  if (id.startsWith("param:")) {
    const key = id.slice("param:".length);
    return key.length === 0 ? null : { kind: "param", key };
  }
  return null;
}

/** What the event that sets a number names it by (`numberChanged.key`): its stat, keyword or param key. */
export function numberKey(ref: NumberRef): string {
  return ref.kind === "keyword" || ref.kind === "param" ? ref.key : ref.kind;
}

/**
 * B3.4, Classic+ #41 KY's Constant: every number on the card now, in a fixed order — cost, attack,
 * health, the numbered keywords, the declared numbers. An Immutable card has none a change may reach
 * (B3.4 rule 2), so it lists none.
 */
export function numbersOn(state: GameState, card: CardInstance): NumberOnCard[] {
  const keywords = inUnitRow(card) ? unitView(state, card).keywords : cardKeywords(state, card);
  if (hasKeyword(keywords, "Immutable")) return [];
  const out: NumberOnCard[] = [];
  const add = (ref: NumberRef, label: string, value: number): void => {
    out.push({ id: numberRefId(ref), ref, label, value });
  };
  const cost = ownCost(state, card);
  if (cost !== null) add({ kind: "cost" }, "Cost", cost);
  const stats = currentStats(state, card);
  if (stats !== null) {
    add({ kind: "attack" }, "Attack", stats.attack);
    add({ kind: "health" }, "Health", stats.health);
  }
  for (const entry of numberedKeywordsOn(state, card)) add({ kind: "keyword", key: entry.key }, entry.key, entry.value);
  for (const param of paramsOf(state, card.defId)) {
    // The key as a word, never a raw `{key}` placeholder (a label is shown as it is).
    add({ kind: "param", key: param.key }, param.key, paramValue(state, card, param.key));
  }
  return out;
}

/** The number a `NumberRef` names on the card now, or null when the card has no such number. */
export function numberOn(state: GameState, card: CardInstance, ref: NumberRef): number | null {
  const id = numberRefId(ref);
  return numbersOn(state, card).find((entry) => entry.id === id)?.value ?? null;
}
