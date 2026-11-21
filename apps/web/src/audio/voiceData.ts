// Card sound data access (docs/polish/2-sound.md, "voiceData.ts"; R655, #4130).
//
// Pure lookup helpers for card audio tables. Tables load lazily in `soundTables.ts` through
// `soundTablesLoader.ts`.
// Everything here reads a `PlayerView` or a defId and nothing else (CLAUDE.md rule 7, R203):
// a redacted card carries `HIDDEN_DEF_ID`, which is never in the table, so it never yields a sound.

import { type PlayerView, type PortraitId, type VoiceEmoteId } from "@jackioh/shared";

import { HIDDEN_DEF_ID } from "./constants.ts";
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
  // R1088: `trigger<n>` is the n-th line of the card's trigger list (1-based).
  const trigger = /^trigger(\d+)$/.exec(line);
  if (trigger !== null) {
    const assignment = hookFor(table, defId, "trigger");
    const spoken = assignment !== null && "lines" in assignment ? assignment.lines[Number(trigger[1]) - 1] : undefined;
    if (spoken === undefined || assignment?.voice === undefined) return null;
    const persona = table.voices[assignment.voice];
    return persona === undefined ? null : { text: spoken, persona };
  }
  // A bare `trigger` is not a playable line: only its numbered lines are.
  if (line === "trigger") return null;
  if (line !== "play" && line !== "death" && line !== "cast" && line !== "attack") {
    return emoteLineFor(table, emotePortraitOf(defId), line as VoiceEmoteId);
  }
  const assignment = hookFor(table, defId, line);
  // R1088: only a trigger lists lines; any other hook speaks its one text.
  if (assignment?.voice === undefined || !("text" in assignment)) return null;
  const persona = table.voices[assignment.voice];
  return persona === undefined ? null : { text: assignment.text, persona };
}

/** R655: the hook's effect, with its bank name, or null when the hook has none. */
export function effectFor(
  table: CardAudioTable,
  defId: string,
  hook: CardHook,
): { name: string; effect: CardEffect } | null {
  const name = hookFor(table, defId, hook)?.effect;
  if (name === undefined) return null;
  const effect = table.effects[name];
  return effect === undefined ? null : { name, effect };
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
 * the viewer's own units (attack, R655), their face-up backrow traps (cast). Only hooks that have a line.
 */
export function voiceKeysForView(view: PlayerView, table: CardAudioTable): VoiceKey[] {
  const keys: VoiceKey[] = [];
  const seen = new Set<string>();
  const add = (defId: string, line: VoiceLineKind): void => {
    const key = voiceKey(defId, line);
    if (seen.has(key) || lineFor(table, defId, line) === null) return;
    seen.add(key);
    keys.push(key);
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
