// #2099: the open deck's Undo and Redo, each an ordinary `updateDeck` edit that saves and syncs (R256).
// A rename's keys are one step until the name field loses the focus.

import type { DeckItem, DeckStore } from "./sync.ts";

export type DeckPatch = Parameters<DeckStore["updateDeck"]>[1];
/** The fields as they were and as the edit left them; `naming` while its rename is under way. */
export type DeckStep = { before: DeckPatch; after: DeckPatch; naming: boolean };
/** The open deck's steps, oldest first: `past` for Undo, `future` for Redo. */
export type DeckHistory = { deckId: string | null; past: readonly DeckStep[]; future: readonly DeckStep[] };

export function emptyHistory(deckId: string | null): DeckHistory {
  return { deckId, past: [], future: [] };
}

/** `deck`'s values of the fields `patch` names, in this order, a cleared pick as null. */
export function fieldsOf(deck: DeckItem, patch: DeckPatch): DeckPatch {
  return {
    ...(patch.name === undefined ? {} : { name: deck.name }),
    ...(patch.cards === undefined ? {} : { cards: deck.cards }),
    ...(patch.portrait === undefined ? {} : { portrait: deck.portrait }),
    ...(patch.tableCosmetic === undefined ? {} : { tableCosmetic: deck.tableCosmetic ?? null }),
    ...(patch.fieldCosmetic === undefined ? {} : { fieldCosmetic: deck.fieldCosmetic ?? null }),
    ...(patch.cardBackCosmetic === undefined ? {} : { cardBackCosmetic: deck.cardBackCosmetic ?? null }),
  };
}

function same(a: DeckPatch, b: DeckPatch): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/** The rename under way ends: the next edit is a step of its own. */
export function sealed(history: DeckHistory): DeckHistory {
  const top = history.past.at(-1);
  if (top?.naming !== true) return history;
  return { ...history, past: [...history.past.slice(0, -1), { ...top, naming: false }] };
}

/** `history` with `patch`'s edit of `deck` on top and Redo emptied; unchanged when the edit changes nothing. */
export function recorded(history: DeckHistory, deck: DeckItem, patch: DeckPatch, naming: boolean): DeckHistory {
  const base = history.deckId === deck.id ? history : emptyHistory(deck.id);
  const before = fieldsOf(deck, patch);
  const after = fieldsOf({ ...deck, ...patch }, patch);
  if (same(before, after)) return base;
  const top = base.past.at(-1);
  if (naming && top?.naming === true) {
    const rest = base.past.slice(0, -1);
    return { ...base, past: same(top.before, after) ? rest : [...rest, { before: top.before, after, naming }], future: [] };
  }
  return { ...base, past: [...sealed(base).past, { before, after, naming }], future: [] };
}

/** What Undo sends and the history after it; null with nothing to undo. */
export function undone(history: DeckHistory): { patch: DeckPatch; history: DeckHistory } | null {
  const step = history.past.at(-1);
  if (step === undefined) return null;
  return {
    patch: step.before,
    history: { ...history, past: history.past.slice(0, -1), future: [...history.future, { ...step, naming: false }] },
  };
}

/** What Redo sends and the history after it; null with nothing to redo. */
export function redone(history: DeckHistory): { patch: DeckPatch; history: DeckHistory } | null {
  const step = history.future.at(-1);
  if (step === undefined) return null;
  return { patch: step.after, history: { ...history, past: [...history.past, step], future: history.future.slice(0, -1) } };
}

/** Ctrl/Cmd+Z is Undo and Ctrl/Cmd+Shift+Z Redo; anything else, Alt held included, is null. */
export function historyKey(event: Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "shiftKey" | "altKey">): "undo" | "redo" | null {
  if (event.altKey || !(event.ctrlKey || event.metaKey) || event.key.toLowerCase() !== "z") return null;
  return event.shiftKey ? "redo" : "undo";
}

/** Input types that take no typed text, where the keys stay the deck's. */
const UNTYPED_INPUTS: ReadonlySet<string> = new Set(["button", "checkbox", "color", "file", "image", "radio", "range", "reset", "submit"]);

/** Whether a key went to a text field, whose own undo the keys then are. */
export function inTextField(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable || target instanceof HTMLTextAreaElement) return true;
  return target instanceof HTMLInputElement && !UNTYPED_INPUTS.has(target.type);
}
