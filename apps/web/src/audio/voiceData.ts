// Card sound data access (docs/polish/2-sound.md, "voiceData.ts"; R651).
//
// `card-audio.json5` (the voices bank, the effects bank and every card's hooks) and
// `voice-manifest.json` (which rendered files exist, written by `apps/web/scripts/gen-voice.mjs`)
// are imported here and nowhere else. The file is JSON5, read as text and parsed once at import
// against the catalog, so a malformed table fails loudly at load, naming the path of the problem,
// instead of speaking garbage. Each card's kind comes from its catalog type: the file never states
// it. Everything here reads a `PlayerView` or a defId and nothing else (CLAUDE.md rule 7, R203): a
// redacted card carries `HIDDEN_DEF_ID`, which is never in the table, so it never yields a sound.

import JSON5 from "json5";

import catalogJson from "@jackioh/cards/catalog.json";
import type { PlayerView } from "@jackioh/shared";

import { CARD_HOOKS, CARD_HOOK_NAMES, HIDDEN_DEF_ID } from "./constants.ts";
import { SFX_IDS, SFX_TIMBRES } from "./sfx.ts";
import type {
  CardAudioEntry,
  CardAudioTable,
  CardEffect,
  CardHook,
  CardKind,
  HookAssignment,
  Persona,
  SfxId,
  SfxParams,
  SfxTimbre,
  VoiceKey,
  VoiceLineKind,
  VoiceManifest,
} from "./types.ts";
import rawCardAudio from "./card-audio.json5?raw";
import rawManifest from "./voice-manifest.json";

/* ------------------------------------------------------------------------------------------- *
 * Parsing
 * ------------------------------------------------------------------------------------------- */

/** The file's name in every error, so a designer knows which file to open. */
const FILE = "card-audio.json5";

function fail(path: string, problem: string): never {
  throw new Error(`${FILE}: ${path}: ${problem}`);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function record(value: unknown, path: string): Record<string, unknown> {
  if (!isRecord(value)) fail(path, "must be an object");
  return value;
}

/** Fails on the first field not in `allowed`, so a misspelt one is never silently ignored. */
function onlyFields(o: Record<string, unknown>, path: string, allowed: readonly string[]): void {
  for (const field of Object.keys(o)) {
    if (!allowed.includes(field)) fail(`${path}.${field}`, `unknown field (expected ${allowed.join(", ")})`);
  }
}

function text(value: unknown, path: string): string {
  if (typeof value !== "string" || value.trim() === "") fail(path, "must be a non-empty string");
  return value;
}

function numberIn(value: unknown, path: string, min: number, max: number): number {
  if (typeof value !== "number" || !Number.isFinite(value) || value < min || value > max) {
    fail(path, `must be a number from ${min} to ${max}`);
  }
  return value;
}

function flag(value: unknown, path: string): boolean {
  if (typeof value !== "boolean") fail(path, "must be true or false");
  return value;
}

const RATE_RANGE = [90, 360] as const;
const PITCH_BASE_RANGE = [0, 127] as const;
const PITCH_MOD_RANGE = [0, 127] as const;
const WEB_PITCH_RANGE = [0, 2] as const;
const WEB_RATE_RANGE = [0.1, 10] as const;
const GAIN_RANGE = [0, 2] as const;
/** R501's SAPI voices: SSML prosody rate in percent, and the ffmpeg pitch shift in semitones. */
const SAPI_RATE_RANGE = [-50, 100] as const;
const SAPI_SEMITONE_RANGE = [-12, 12] as const;
/** R651: an effect's pitch, a frequency ratio: two octaves either way. */
const EFFECT_PITCH_RANGE = [0.25, 4] as const;
/** `amount` as the recipes read it (types.ts SfxParams): a count, clamped by each recipe. */
const EFFECT_AMOUNT_RANGE = [0, 100] as const;

const SAY_FIELDS = ["backend", "say", "rate", "pbas", "pmod", "web", "gain"] as const;
const SAPI_FIELDS = ["backend", "voice", "rate", "semitones", "filter", "web", "gain"] as const;
const EFFECT_FIELDS = ["sfx", "pitch", "gain", "params"] as const;
const PARAM_FIELDS = ["amount", "mine", "timbre", "mythic", "urgent", "release"] as const;
const ASSIGNMENT_FIELDS = ["voice", "text", "effect"] as const;
/** The file's sections, in the order it keeps them. */
const SECTIONS = ["voices", "effects", "cards"] as const;

/** docs/polish/2-sound.md: Unit -> unit; Spell and Field Spell -> spell; Trap and Field Trap -> trap. */
const KIND_OF_TYPE: Readonly<Record<string, CardKind>> = {
  Unit: "unit",
  Spell: "spell",
  "Field Spell": "spell",
  Trap: "trap",
  "Field Trap": "trap",
};

/** What the parser needs of the catalog: each id's type. */
export type CatalogTypes = Readonly<Record<string, { readonly type: string }>>;

function parsePersona(raw: unknown, path: string): Persona {
  const o = record(raw, path);
  const rawWeb = record(o.web, `${path}.web`);
  onlyFields(rawWeb, `${path}.web`, ["pitch", "rate"]);
  const web = {
    pitch: numberIn(rawWeb.pitch, `${path}.web.pitch`, ...WEB_PITCH_RANGE),
    rate: numberIn(rawWeb.rate, `${path}.web.rate`, ...WEB_RATE_RANGE),
  };
  let persona: Persona;
  if (o.backend === "sapi") {
    onlyFields(o, path, SAPI_FIELDS);
    if (typeof o.filter !== "string") fail(`${path}.filter`, "must be a string");
    persona = {
      backend: "sapi",
      voice: text(o.voice, `${path}.voice`),
      rate: numberIn(o.rate, `${path}.rate`, ...SAPI_RATE_RANGE),
      semitones: numberIn(o.semitones, `${path}.semitones`, ...SAPI_SEMITONE_RANGE),
      filter: o.filter,
      web,
    };
  } else {
    if (o.backend !== undefined && o.backend !== "say") fail(`${path}.backend`, 'must be "say" or "sapi"');
    onlyFields(o, path, SAY_FIELDS);
    persona = {
      say: text(o.say, `${path}.say`),
      rate: numberIn(o.rate, `${path}.rate`, ...RATE_RANGE),
      pbas: numberIn(o.pbas, `${path}.pbas`, ...PITCH_BASE_RANGE),
      pmod: numberIn(o.pmod, `${path}.pmod`, ...PITCH_MOD_RANGE),
      web,
    };
  }
  if (o.gain !== undefined) persona.gain = numberIn(o.gain, `${path}.gain`, ...GAIN_RANGE);
  return persona;
}

function parseParams(raw: unknown, path: string): SfxParams {
  const o = record(raw, path);
  onlyFields(o, path, PARAM_FIELDS);
  const params: SfxParams = {};
  if (o.amount !== undefined) params.amount = numberIn(o.amount, `${path}.amount`, ...EFFECT_AMOUNT_RANGE);
  if (o.timbre !== undefined) {
    if (typeof o.timbre !== "string" || !(SFX_TIMBRES as readonly string[]).includes(o.timbre)) {
      fail(`${path}.timbre`, `unknown timbre ${JSON.stringify(o.timbre)} (expected one of ${SFX_TIMBRES.join(", ")})`);
    }
    params.timbre = o.timbre as SfxTimbre;
  }
  for (const name of ["mine", "mythic", "urgent", "release"] as const) {
    if (o[name] !== undefined) params[name] = flag(o[name], `${path}.${name}`);
  }
  return params;
}

function parseEffect(raw: unknown, path: string): CardEffect {
  const o = record(raw, path);
  onlyFields(o, path, EFFECT_FIELDS);
  if (typeof o.sfx !== "string" || !(SFX_IDS as readonly string[]).includes(o.sfx)) {
    fail(`${path}.sfx`, `unknown recipe ${JSON.stringify(o.sfx)} (expected one of sfx.ts SFX_IDS)`);
  }
  const effect: CardEffect = {
    sfx: o.sfx as SfxId,
    pitch: o.pitch === undefined ? 1 : numberIn(o.pitch, `${path}.pitch`, ...EFFECT_PITCH_RANGE),
    gain: o.gain === undefined ? 1 : numberIn(o.gain, `${path}.gain`, ...GAIN_RANGE),
  };
  if (o.params !== undefined) effect.params = parseParams(o.params, `${path}.params`);
  return effect;
}

function parseAssignment(
  raw: unknown,
  path: string,
  voices: Record<string, Persona>,
  effects: Record<string, CardEffect>,
): HookAssignment {
  const o = record(raw, path);
  onlyFields(o, path, ASSIGNMENT_FIELDS);
  let effect: string | undefined;
  if (o.effect !== undefined) {
    effect = text(o.effect, `${path}.effect`);
    if (!Object.hasOwn(effects, effect)) fail(`${path}.effect`, `unknown effect "${effect}"`);
  }
  if (o.voice === undefined) {
    if (o.text !== undefined) fail(`${path}.voice`, "a text needs the voice that speaks it");
    if (effect === undefined) fail(path, "needs a voice and a text, an effect, or both");
    return { effect };
  }
  const voice = text(o.voice, `${path}.voice`);
  if (!Object.hasOwn(voices, voice)) fail(`${path}.voice`, `unknown voice "${voice}"`);
  const spoken = text(o.text, `${path}.text`);
  return effect === undefined ? { voice, text: spoken } : { voice, text: spoken, effect };
}

function parseCard(
  raw: unknown,
  path: string,
  kind: CardKind,
  voices: Record<string, Persona>,
  effects: Record<string, CardEffect>,
): CardAudioEntry {
  const o = record(raw, path);
  const entry: CardAudioEntry = { kind };
  for (const [hook, value] of Object.entries(o)) {
    if (!(CARD_HOOK_NAMES as readonly string[]).includes(hook)) {
      fail(`${path}.${hook}`, `unknown hook (expected one of ${CARD_HOOK_NAMES.join(", ")})`);
    }
    const kinds: readonly CardKind[] = CARD_HOOKS[hook as CardHook].kinds;
    if (!kinds.includes(kind)) fail(`${path}.${hook}`, `a ${kind} has no ${hook} hook`);
    entry[hook as CardHook] = parseAssignment(value, `${path}.${hook}`, voices, effects);
  }
  return entry;
}

/**
 * The parsed file, checked against `catalog`. Throws Error("card-audio.json5: <path>: <problem>") on
 * a shape error: an unknown section, voice, effect, recipe, hook or field, a missing text, a value
 * out of range, or a card id the catalog does not hold. Does NOT check that every catalog card has
 * its lines, or the word limits: the tests do (B33, B34).
 */
export function parseCardAudio(raw: unknown, catalog: CatalogTypes): CardAudioTable {
  const root = record(raw, "(root)");
  const sections = Object.keys(root);
  if (sections.join() !== SECTIONS.join()) {
    fail("(root)", `must hold ${SECTIONS.join(", ")}, in that order (found ${sections.join(", ") || "nothing"})`);
  }
  const voices: Record<string, Persona> = {};
  for (const [name, value] of Object.entries(record(root.voices, "voices"))) {
    voices[name] = parsePersona(value, `voices.${name}`);
  }
  const effects: Record<string, CardEffect> = {};
  for (const [name, value] of Object.entries(record(root.effects, "effects"))) {
    effects[name] = parseEffect(value, `effects.${name}`);
  }
  const cards: Record<string, CardAudioEntry> = {};
  for (const [defId, value] of Object.entries(record(root.cards, "cards"))) {
    if (defId === HIDDEN_DEF_ID) fail(`cards.${defId}`, "the hidden sentinel cannot have sounds");
    const type = Object.hasOwn(catalog, defId) ? catalog[defId]?.type : undefined;
    if (type === undefined) fail(`cards.${defId}`, "not a card in the catalog");
    const kind = KIND_OF_TYPE[type];
    if (kind === undefined) fail(`cards.${defId}`, `the catalog type ${JSON.stringify(type)} has no hooks`);
    cards[defId] = parseCard(value, `cards.${defId}`, kind, voices, effects);
  }
  return { voices, effects, cards };
}

/** A JSON5 word: an identifier, a number or a literal (`true`, `Infinity`, `-0.5`, `1e3`). */
const WORD = /[\w$+\-.]+/y;

/**
 * The path of the first key written twice in one object, or null. JSON5.parse keeps the last of
 * them without a word, so a second entry for a card pasted in by hand (or left by a merge) would
 * silently replace the first. `source` has already parsed, so it is well formed.
 */
function duplicateKeyPath(source: string): string | null {
  type Frame = { path: string; keys: Set<string> | null; key: string | null };
  const frames: Frame[] = [];
  const pathOf = (frame: Frame, key: string): string => (frame.path === "" ? key : `${frame.path}.${key}`);
  /** Past whitespace and comments, from `from`. */
  const skip = (from: number): number => {
    let j = from;
    for (;;) {
      if (/\s/.test(source[j] ?? "")) {
        j += 1;
      } else if (source.startsWith("//", j) || source.startsWith("/*", j)) {
        const close = source.startsWith("//", j) ? "\n" : "*/";
        const end = source.indexOf(close, j + 2);
        j = end < 0 ? source.length : end + close.length;
      } else {
        return j;
      }
    }
  };
  for (let i = skip(0); i < source.length; i = skip(i)) {
    const c = source[i] ?? "";
    let word: string;
    if (c === '"' || c === "'") {
      let j = i + 1;
      word = "";
      while (j < source.length && source[j] !== c) {
        if (source[j] === "\\") j += 1;
        word += source[j] ?? "";
        j += 1;
      }
      i = j + 1;
    } else {
      WORD.lastIndex = i;
      const match = WORD.exec(source);
      if (match === null) {
        const top = frames.at(-1);
        if (c === "{" || c === "[") {
          const path = top === undefined ? "" : top.key === null ? top.path : pathOf(top, top.key);
          frames.push({ path, keys: c === "{" ? new Set() : null, key: null });
        } else if (c === "}" || c === "]") {
          frames.pop();
        }
        i += 1;
        continue;
      }
      word = match[0];
      i += word.length;
    }
    const top = frames.at(-1);
    const next = skip(i);
    if (top?.keys != null && source[next] === ":") {
      if (top.keys.has(word)) return pathOf(top, word);
      top.keys.add(word);
      top.key = word;
      i = next + 1;
    }
  }
  return null;
}

/**
 * `parseCardAudio` of JSON5 text. A syntax error is reported the same way, with its line and
 * column, and so is a key written twice in one object.
 */
export function parseCardAudioText(source: string, catalog: CatalogTypes): CardAudioTable {
  let raw: unknown;
  try {
    raw = JSON5.parse(source);
  } catch (err) {
    fail("(syntax)", err instanceof Error ? err.message : String(err));
  }
  const twice = duplicateKeyPath(source);
  if (twice !== null) fail(twice, "written twice in one object (only the last would count)");
  return parseCardAudio(raw, catalog);
}

export const CARD_AUDIO: CardAudioTable = parseCardAudioText(rawCardAudio, catalogJson as unknown as CatalogTypes);
/** Generated by gen-voice.mjs and checked against the files by its `--check` (B35, B37). */
export const VOICE_MANIFEST = rawManifest as VoiceManifest;

/* ------------------------------------------------------------------------------------------- *
 * Lookups
 * ------------------------------------------------------------------------------------------- */

export function voiceKey(defId: string, line: VoiceLineKind): VoiceKey {
  return `${defId}-${line}`;
}

export function voiceUrl(key: VoiceKey): string {
  return `${import.meta.env.BASE_URL}audio/voice/${key}.m4a`;
}

/**
 * The table entry for a card this viewer may name, or null. This is R203's "readable": the defId is
 * not the redaction sentinel (R97, R154) and the table has an entry for it (a fused transient
 * definition, R77, has none).
 */
export function entryFor(table: CardAudioTable, defId: string): CardAudioEntry | null {
  if (defId === HIDDEN_DEF_ID || !Object.hasOwn(table.cards, defId)) return null;
  return table.cards[defId] ?? null;
}

/** A readable card's assignment for a hook, or null. */
export function hookFor(table: CardAudioTable, defId: string, hook: CardHook): HookAssignment | null {
  return entryFor(table, defId)?.[hook] ?? null;
}

/** The hook's line: its text and the voice that speaks it, or null when the hook has no line. */
export function lineFor(
  table: CardAudioTable,
  defId: string,
  line: VoiceLineKind,
): { text: string; persona: Persona } | null {
  const assignment = hookFor(table, defId, line);
  if (assignment?.voice === undefined) return null;
  const persona = table.voices[assignment.voice];
  return persona === undefined ? null : { text: assignment.text, persona };
}

/** R651: the hook's effect, with its bank name, or null when the hook has none. */
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

/**
 * Keys worth preloading for a view, deduped, in this order: the viewer's hand (unit → play, spell →
 * cast; traps none), every unit on both boards (death), the viewer's own units (attack, R651), the
 * viewer's own face-up backrow traps (cast). Only hooks that have a line: an effect renders no file.
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
