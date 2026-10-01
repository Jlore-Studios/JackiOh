// Copy the last Spell's text (docs/classic-sets.md B5 E14; Classic #57 Echo; SPEC §8.6 row 57, R399,
// R545–R547).
//
// "This has the text of the last Spell either player played." The record is E4's (`playCounts.ts`):
// `state.lastSpell = { defId, radiant }`, written at §10.5 step 4 by every Spell play and cast, never
// by a countered play (B5 E1), and never by a copier itself — its `recordsPlayAs` names the Spell it
// copied, or nothing when it copied nothing, so two copiers never loop (B5 E4). This module is the
// other half: a card whose running face sets `staticFlags.copiesLastSpell` HAS that Spell's text.
//
// What "has the text" reaches, and where (R399, R546, R547):
//   * In a hand or a deck the copy follows `state.lastSpell` live (`copiedTextOf`).
//   * As the card is played (§10.5 step 1) or cast, the copy it will resolve is fixed: a play keeps it
//     on its run until the announce moves the card into the resolving zone, and from there on the
//     card's own memory holds it (`fixCopiedText`), so its declared choices, its resolution, its Echo
//     repeats and every prompt continuation of the copied text keep that face even when a cast inside
//     the resolution records a newer Spell. The memory goes with R215's reset as the card lands.
//   * Declarations (targets, modes, R81) and the declaring card's own target checks are read off the
//     copied face: `textFaceOf` returns the card as its text runs — the copier's instance under the
//     copied definition and face — and `playChoices.resolvingFace` hands every declaration reader that
//     card. Its instance id is the copier's, so a hand pick never offers the card being played.
//   * The resolution runs the copied face's `cry` with the copier as `self` (`playSteps`' resolve and
//     Echo steps pass `textFaceOf` to `prompts.runHookResumable`), so the continuation a prompt in it
//     parks names the copied definition (`prompts.resumeSelf` reads the running `ctx.defId` first),
//     and `param(ctx, key)` reads the copied definition's declared numbers on the copier's instance.
//   * X (R545): the copier is played for its own printed cost; a copied X-cost text makes the play
//     choose X, from 1 up to the mana left once that price is paid (`copiedChoosesX`,
//     `playChoices.legalXValues`), and a cast of it asks its caster for X as any cast X card's does
//     (B5 E12). An embiggen text resolves at its base price: no embiggen price is paid for the copier.
//   * Echo (R546): the copied face's printed Echo X adds to the copier's own (`copiedEcho`).
//   * Cast on draw (R547): a copier drawn while the last Spell casts on draw is cast (`draw.castsOnDraw`).
//   * `preview` (R280) and `conditionMet` (R195) of the copied face answer for the copier in hand.
//   * The view (R243, R399): the owner's hand view of a copier carries the copied face as `copies`.
// Text that answers from a pile the card has landed in — §5.1's end-of-turn return (#23, #24, #31) —
// is not had (R547): step 7 writes that flag off the card's own face, and the copy ends as it lands.
//
// Everything else about the card is its own: its name, (1) Cost, type and tags (a copied Book does
// not make it a Book), its own Echo 1 on the Radiant face, and "this" in the copied text, which is the
// copier — the card that resolves, lands and is counted. A fused card is never a copier: a Fuse joins
// printed scripts (R77), and a copier prints none of the text it copies.

import { defOf, fusedIdParts } from "../catalog";
import { EMPTY_SCRIPT, type Script } from "../script";
import { flagsOf, scriptOf, scriptsFor } from "../scripts";
import type { CardInstance, GameState, PlayRecord } from "../state";

/**
 * Where a copier being played keeps the copy it resolves (R546): its memory, written as it enters
 * the resolving zone, JSON like all memory, reset by R215 as it leaves that zone. `null` is "fixed to
 * nothing" — no Spell had been played as the play began — which is not the same as absent.
 */
export const COPIED_TEXT_KEY = "__copiedText";

/** Whether this card's running face copies the last Spell's text (`staticFlags.copiesLastSpell`). */
export function copiesText(card: CardInstance): boolean {
  if (fusedIdParts(card.defId) !== null) return false;
  return flagsOf(card).copiesLastSpell === true;
}

function recordFrom(raw: unknown): PlayRecord | null | undefined {
  if (raw === null) return null;
  if (typeof raw !== "object") return undefined;
  const record = raw as { defId?: unknown; radiant?: unknown };
  if (typeof record.defId !== "string") return undefined;
  return { defId: record.defId, radiant: record.radiant === true };
}

/** The copy fixed on a copier being played, `null` for one fixed to nothing, `undefined` for none. */
function fixedCopyOf(card: CardInstance): PlayRecord | null | undefined {
  if (!(COPIED_TEXT_KEY in card.memory)) return undefined;
  return recordFrom(card.memory[COPIED_TEXT_KEY]);
}

/**
 * R399, R546: the Spell whose text this card has now — the copy fixed on it while it is played, else
 * the last Spell either player played — or null: not a copier, or nothing to copy.
 */
export function copiedTextOf(state: GameState, card: CardInstance): PlayRecord | null {
  if (!copiesText(card)) return null;
  const fixed = fixedCopyOf(card);
  if (fixed !== undefined) return fixed === null ? null : { ...fixed };
  const last = state.lastSpell;
  return last === undefined ? null : { defId: last.defId, radiant: last.radiant === true };
}

/**
 * R546: fix the copy a copier resolves, as its play or cast moves it into the resolving zone —
 * `record` when the play fixed it earlier (§10.5 step 1), else the last Spell now. Nothing for a card
 * that copies nothing.
 */
export function fixCopiedText(state: GameState, card: CardInstance, record?: PlayRecord | null): void {
  if (!copiesText(card)) return;
  const copy = record === undefined ? copiedTextOf(state, card) : record;
  card.memory[COPIED_TEXT_KEY] = copy === null ? null : { defId: copy.defId, radiant: copy.radiant };
}

/**
 * R399, R546: the card as its text runs — for a copier with a copy, its own instance under the copied
 * definition and face, which every reader of a declaration, a target check, a `preview` or a
 * `conditionMet` is handed; every other card is itself. A read: the copier's instance is not changed.
 */
export function textFaceOf(state: GameState, card: CardInstance): CardInstance {
  const copy = copiedTextOf(state, card);
  if (copy === null) return card;
  return { ...card, defId: copy.defId, radiant: copy.radiant };
}

/** The script a card's text runs now: the copied face's for a copier with a copy, its own otherwise. */
export function runningScriptOf(state: GameState, card: CardInstance): Script {
  const copy = copiedTextOf(state, card);
  if (copy === null) return scriptOf(card);
  const entry = scriptsFor(copy.defId);
  return (copy.radiant ? entry.radiant : entry.base) ?? EMPTY_SCRIPT;
}

/** R546: the Echo X the copied face prints, which adds to the copier's own Echo (0 for any other card). */
export function copiedEcho(state: GameState, card: CardInstance): number {
  if (copiedTextOf(state, card) === null) return 0;
  return Math.max(0, Math.trunc(runningScriptOf(state, card).staticFlags?.echo ?? 0));
}

/** R547: whether the copied face casts on draw (§2.4), for a copier being drawn. */
export function copiedCastsOnDraw(state: GameState, card: CardInstance): boolean {
  if (copiedTextOf(state, card) === null) return false;
  return runningScriptOf(state, card).staticFlags?.castOnDraw === true;
}

/**
 * R545: whether a play of this copier chooses an X for the text it copies — an X-cost Spell's whose X
 * no `cost` hook fixes (§2.3, R43). The copier itself is never an X-cost card: it pays its own price.
 */
export function copiedChoosesX(state: GameState, card: CardInstance): boolean {
  const copy = copiedTextOf(state, card);
  if (copy === null) return false;
  if (defOf(state, copy.defId).cost !== "X") return false;
  return runningScriptOf(state, card).cost === undefined;
}
