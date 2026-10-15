// The creator's editor state (#920): one card being made or edited, as plain data, and the pure steps
// that move it. The page holds a `Draft`; the builder, the code view and the frame form each hand
// back a new one through these functions, so a test can walk the whole round trip without a page.
//
// K3: the builder's model writes the script (`writeScript`), the card's numbers (`numbersOf`) and,
// until the player writes their own, the card's text (`writeText`). The code view edits the script
// itself; a card whose script no longer equals what its model writes is code-only from then on, and
// `builderRefusal` says why.

import type { CustomCard, CustomFrame, CustomParam } from "@jackioh/shared";

import { builderShows, emptyModel, isBuilderModel, numbersOf, writeScript, writeText, type BuilderModel } from "./builder.ts";

export type CreatorView = "builder" | "code";

export type Draft = {
  /** The saved card's id, or null for one not saved yet. */
  id: string | null;
  /** The saved version this draft was read from, or null for a new card. */
  version: number | null;
  frame: CustomFrame;
  source: string;
  /** The builder's model, or null for a card written in code. */
  model: BuilderModel | null;
  view: CreatorView;
};

/** The id a card not saved yet is checked under: well formed, and never one the server mints (`custom-` and 32 hex digits). */
export const DRAFT_ID = "custom-draft";

/** A new card: a 1-mana 1/1 Unit with no text, in the builder. */
export function newDraft(): Draft {
  const model = emptyModel();
  return {
    id: null,
    version: null,
    frame: { name: "", type: "Unit", cost: 1, rarity: "Common", attack: 1, health: 1, keywords: [], text: "", params: [] },
    source: writeScript(model),
    model,
    view: "builder",
  };
}

/** A saved card, opened in the builder when its model still writes its script, else in code. */
export function draftOf(card: CustomCard): Draft {
  const model = isBuilderModel(card.builder) ? card.builder : null;
  return {
    id: card.id,
    version: card.version,
    frame: card.frame,
    source: card.source,
    model,
    view: builderShows(model, card.source) ? "builder" : "code",
  };
}

/** The card as the checks and a save read it: the draft's id or `DRAFT_ID`, and the model while it still writes the script. */
export function cardOf(draft: Draft): CustomCard {
  const keepModel = draft.model !== null && builderShows(draft.model, draft.source);
  return {
    id: draft.id ?? DRAFT_ID,
    version: draft.version ?? 1,
    frame: draft.frame,
    source: draft.source,
    ...(keepModel ? { builder: draft.model } : {}),
  };
}

function paramsOf(model: BuilderModel): CustomParam[] {
  return numbersOf(model).map(({ key, base, radiant }) => ({ key, base, radiant }));
}

/**
 * A builder edit: the script and the numbers follow the model, and the text too while it is still
 * the one the last model wrote (or empty), so a text the player wrote is never overwritten.
 */
export function withModel(draft: Draft, model: BuilderModel): Draft {
  const written = draft.model === null ? "" : writeText(draft.model);
  const followText = draft.frame.text === "" || draft.frame.text === written;
  return {
    ...draft,
    model,
    source: writeScript(model),
    frame: { ...draft.frame, params: paramsOf(model), ...(followText ? { text: writeText(model) } : {}) },
  };
}

/** A frame edit from the form. A Spell has no attack, health or keywords, so a switch to Spell drops them. */
export function withFrame(draft: Draft, frame: CustomFrame): Draft {
  if (frame.type === "Spell") {
    const { attack: _attack, health: _health, radiantAttack: _ra, radiantHealth: _rh, ...rest } = frame;
    return { ...draft, frame: { ...rest, keywords: [] } };
  }
  return {
    ...draft,
    frame: { ...frame, attack: frame.attack ?? 1, health: frame.health ?? 1, keywords: frame.keywords ?? [] },
  };
}

/** A code edit: the script as typed. The model stays, so undoing the edit brings the builder back. */
export function withSource(draft: Draft, source: string): Draft {
  return { ...draft, source };
}

/** Why the builder cannot show the draft, or null when it can (K3). */
export function builderRefusal(draft: Draft): string | null {
  if (draft.model === null) return "This card was written in code, so only the code view can show it.";
  if (!builderShows(draft.model, draft.source)) {
    return "This card's script was changed in the code view, so the builder can no longer show it. Undo the change to go back, or keep it in code.";
  }
  return null;
}

/** A view switch; to the builder only when it can show the card, so a switch never loses an edit. */
export function withView(draft: Draft, view: CreatorView): Draft {
  if (view === "builder" && builderRefusal(draft) !== null) return draft;
  return { ...draft, view };
}

/** Starts the card over in the builder, its script replaced by an empty model's (the page asks first). */
export function restartInBuilder(draft: Draft): Draft {
  const model = emptyModel();
  return { ...withModel({ ...draft, model: null }, model), view: "builder" };
}

/**
 * A frame or model as the server writes it back: keys in one order, and an empty list or an unset
 * value the same as none, since serde skips them (`CustomFrame`'s `keywords`, `params` and stats).
 */
function canonical(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(canonical);
  if (value === null || typeof value !== "object") return value;
  const kept: Record<string, unknown> = {};
  for (const [key, held] of Object.entries(value).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))) {
    if (held === undefined || held === null || (Array.isArray(held) && held.length === 0)) continue;
    kept[key] = canonical(held);
  }
  return kept;
}

function sameJson(a: unknown, b: unknown): boolean {
  return JSON.stringify(canonical(a)) === JSON.stringify(canonical(b));
}

/** Whether the draft differs from the card it was read from (a new card is unsaved until its first save). */
export function isDirty(draft: Draft, saved: CustomCard | null): boolean {
  if (saved === null) return true;
  const card = cardOf(draft);
  return !sameJson(card.frame, saved.frame) || card.source !== saved.source || !sameJson(card.builder ?? null, saved.builder ?? null);
}
