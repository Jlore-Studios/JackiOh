// Card sound data access (docs/polish/2-sound.md, "voiceData.ts"; R655, #4130).
//
// Pure lookup helpers for card audio tables. Tables load lazily in `soundTables.ts` through
// `soundTablesLoader.ts`.
// Everything here reads a `PlayerView` or a defId and nothing else (CLAUDE.md rule 7, R203):
// a redacted card carries `HIDDEN_DEF_ID`, which is never in the table, so it never yields a sound.

import { type PlayerView, type PortraitId, type VoiceEmoteId } from "@jackioh/shared";

import { CARD_HOOK_NAMES, HIDDEN_DEF_ID } from "./constants.ts";
import type {
  CardAudioEntry,
  CardAudioTable,
  CardEffect,
  CardHook,
  HookAssignment,
  Persona,
  PlayableLineKind,
  VoiceKey,
  VoiceLineKind,
} from "./types.ts";

/* Lookups */

export function voiceKey(defId: string, line: PlayableLineKind): VoiceKey {
  return `${defId}-${line}`;
}

export function voiceUrl(key: VoiceKey, hash: string): string {
  return `${import.meta.env.BASE_URL}audio/voice/${key}.m4a?v=${hash}`;
}

/**
 * The table entry for a card this viewer may name (R203's "readable"), or null: not the redaction
 * sentinel (R97, R154) and in the table (a fused transient definition, R77, has no entry).
 */
export function entryFor(table: CardAudioTable, defId: string): CardAudioEntry | null {
  if (defId === HIDDEN_DEF_ID || !Object.hasOwn(table.cards, defId)) return null;
  return table.cards[defId] ?? null;
}

/** A readable card's assignment for a hook, or null. */
export function hookFor(table: CardAudioTable, defId: string, hook: CardHook): HookAssignment | null {
  return entryFor(table, defId)?.[hook] ?? null;
}

/**
 * The hook's line and the voice that speaks it, or null. R644: an emote line resolves through the
 * emotes table instead, which lets `playVoice` carry emote defIds (`emote-vanilla`) unchanged (engine.ts).
 */
export function lineFor(
  table: CardAudioTable,
  defId: string,
  line: PlayableLineKind,
): { text: string; persona: Persona } | null {
  // R1088: `trigger<n>` is the n-th line of the card's trigger list (1-based). R5188000: any other hook's
  // `<hook><n>`, n from 2, is the n-th line of its pool; its first is the bare hook's.
  const numbered = /^([A-Za-z]+?)(\d+)$/.exec(line);
  const hook = numbered?.[1] ?? line;
  if (!isCardHook(hook)) return emoteLineFor(table, emotePortraitOf(defId), line as VoiceEmoteId);
  const n = numbered === null ? null : Number(numbered[2]);
  // A bare `trigger` is not a playable line: only its numbered lines are; nor is a pool's `<hook>1`.
  if (hook === "trigger" ? n === null : n !== null && n < 2) return null;
  const assignment = hookFor(table, defId, hook);
  if (assignment?.voice === undefined) return null;
  let spoken: string | undefined;
  if ("lines" in assignment) spoken = assignment.lines[(n ?? 1) - 1];
  else if (n === null) spoken = assignment.text;
  const persona = table.voices[assignment.voice];
  return spoken === undefined || persona === undefined ? null : { text: spoken, persona };
}

function isCardHook(name: string): name is CardHook {
  return (CARD_HOOK_NAMES as readonly string[]).includes(name);
}

/**
 * R1088, R5188000: every line a readable card's hook may speak, in the file's order: a trigger's
 * `trigger1` to `trigger<n>`, a pool's `<hook>`, `<hook>2` to `<hook><n>`, a single text's `<hook>`; none
 * without a voice.
 */
export function hookLines(table: CardAudioTable, defId: string, hook: CardHook): PlayableLineKind[] {
  const assignment = hookFor(table, defId, hook);
  if (assignment?.voice === undefined) return [];
  if (!("lines" in assignment)) return [hook];
  return assignment.lines.map((_, i): PlayableLineKind => (hook === "trigger" ? `trigger${i + 1}` : i === 0 ? hook : `${hook}${i + 1}`));
}

/**
 * R1088, R5188000: the line a hook speaks this time, or null when it has none: one of a pool's, drawn
 * by the client's cosmetic `random` (never the match's), and a hook's only line without a draw.
 */
export function pickLine(
  table: CardAudioTable,
  defId: string,
  hook: CardHook,
  random: () => number,
): PlayableLineKind | null {
  const lines = hookLines(table, defId, hook);
  const n = lines.length;
  if (n <= 1) return lines[0] ?? null;
  return lines[Math.min(n - 1, Math.floor(random() * n))] ?? null;
}

/**
 * R655: the hook's effects, with their bank names, in the order they play: one, or (R5188000) a list's
 * each; none when the hook has no effect.
 */
export function effectsFor(
  table: CardAudioTable,
  defId: string,
  hook: CardHook,
): { name: string; effect: CardEffect }[] {
  const named = hookFor(table, defId, hook)?.effect;
  if (named === undefined) return [];
  const found: { name: string; effect: CardEffect }[] = [];
  for (const name of typeof named === "string" ? [named] : named) {
    const effect = table.effects[name];
    if (effect !== undefined) found.push({ name, effect });
  }
  return found;
}

/** R655: the hook's first effect, with its bank name, or null when the hook has none. */
export function effectFor(
  table: CardAudioTable,
  defId: string,
  hook: CardHook,
): { name: string; effect: CardEffect } | null {
  return effectsFor(table, defId, hook)[0] ?? null;
}

/** `emote-vanilla` -> `vanilla`; anything else -> itself (only emote keys carry the prefix). */
function emotePortraitOf(defId: string): string {
  return defId.startsWith("emote-") ? defId.slice("emote-".length) : defId;
}

/** A portrait's bubble text and emote voice for one issue-§3 line, or null; `emote-<portrait>` also works. */
export function emoteLineFor(
  table: CardAudioTable,
  portrait: string,
  line: VoiceEmoteId,
): { text: string; persona: Persona } | null {
  const entry = table.emotes[emotePortraitOf(portrait)];
  if (entry === undefined) return null;
  const base = table.voices[entry.persona];
  if (base === undefined) return null;
  // A SAPI voice takes no per-entry overrides (R501); a `say` one applies them.
  const persona: Persona =
    base.backend === "sapi"
      ? base
      : { ...base, rate: entry.rate ?? base.rate, pbas: entry.pbas ?? base.pbas, pmod: entry.pmod ?? base.pmod };
  return { text: entry[line], persona };
}

/** The `playVoice` defId that resolves a portrait's lines: `emote-vanilla` etc. */
export function emoteVoiceDef(portrait: PortraitId): string {
  return `emote-${portrait}`;
}

/**
 * Keys worth preloading for a view, deduped, in order: the viewer's hand, every unit on both boards (death),
 * the viewer's own units (attack, R655), their face-up backrow traps (cast). Only hooks that have a line,
 * each line of a pool (R5188000).
 */
export function voiceKeysForView(view: PlayerView, table: CardAudioTable): VoiceKey[] {
  const keys: VoiceKey[] = [];
  const seen = new Set<string>();
  const add = (defId: string, hook: VoiceLineKind): void => {
    for (const line of hookLines(table, defId, hook)) {
      const key = voiceKey(defId, line);
      if (seen.has(key) || lineFor(table, defId, line) === null) continue;
      seen.add(key);
      keys.push(key);
    }
  };

  const hand = view.you.hand;
  if (Array.isArray(hand)) {
    for (const card of hand) {
      const entry = entryFor(table, card.defId);
      if (entry?.kind === "unit") add(card.defId, "play");
      else if (entry?.kind === "spell") add(card.defId, "cast");
    }
  }

  for (const side of [view.you, view.opponent]) {
    for (const u of side.units) {
      if (u === null) continue;
      if (entryFor(table, u.defId)?.kind === "unit") add(u.defId, "death");
    }
  }

  for (const u of view.you.units) {
    if (u !== null && entryFor(table, u.defId)?.kind === "unit") add(u.defId, "attack");
  }

  for (const side of [view.you, view.opponent]) {
    for (const slot of side.backrow) {
      if (slot === null || slot.faceDown) continue;
      if (slot.controller !== view.viewer) continue;
      if (entryFor(table, slot.defId)?.kind === "trap") add(slot.defId, "cast");
    }
  }

  return keys;
}
