// Sound tables (docs/polish/2-sound.md; #4130, R655, R631).
//
// `card-audio.json5` (the voices bank, the effects bank, every card's hooks, and the portraits' emote voices),
// `voice-text.json` (the words those hooks and portraits speak), `voice-manifest.json` (rendered voice files),
// `music-manifest.json` (rendered music files) and `music-cards.json` (card intros and stations) are imported
// here and nowhere else in non-test source.
//
// This module is reached only via dynamic `import()` (through `soundTablesLoader.ts`) or in test files,
// keeping json5 and ~100 KB gzipped of sound tables out of the initial page load.

import JSON5 from "json5";

import catalogJson from "@jackioh/cards/catalog.json";
import { VOICE_EMOTE_IDS } from "@jackioh/shared";

import { CARD_HOOKS, CARD_HOOK_NAMES, HIDDEN_DEF_ID } from "./constants.ts";
import { SFX_IDS, SFX_TIMBRES } from "./sfx.ts";
import type {
  CardAudioEntry,
  CardAudioTable,
  CardEffect,
  CardHook,
  CardKind,
  EmoteLineEntry,
  HookAssignment,
  MusicCardEntry,
  MusicManifest,
  Persona,
  SfxId,
  SfxParams,
  SfxTimbre,
  VoiceManifest,
} from "./types.ts";
import { parseMusicCards, parseMusicManifest } from "./musicData.ts";
import type { SoundTables } from "./soundTablesLoader.ts";
import rawCardAudio from "./card-audio.json5?raw";
import rawManifest from "./voice-manifest.json";
import voiceTextJson from "./voice-text.json";
import rawCards from "./music-cards.json";
import rawMusicManifest from "./music-manifest.json";
import { isRecord } from "../wire/json.ts";
import { messageOf } from "../net/errors.ts";

/* Parsing */

/** The file's name in every error, so a designer knows which file to open. */
const FILE = "card-audio.json5";
/** The file a line's words are written in, named in an error about them. */
const TEXT_FILE = "card-text.yaml";

function fail(path: string, problem: string): never {
  throw new Error(`${FILE}: ${path}: ${problem}`);
}

function failText(path: string, problem: string): never {
  throw new Error(`${TEXT_FILE}: ${path}: ${problem}`);
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
/** R655: an effect's pitch, a frequency ratio: two octaves either way. */
const EFFECT_PITCH_RANGE = [0.25, 4] as const;
/** `amount` as the recipes read it (types.ts SfxParams): a count, clamped by each recipe. */
const EFFECT_AMOUNT_RANGE = [0, 100] as const;

const SAY_FIELDS = ["backend", "say", "rate", "pbas", "pmod", "web", "gain"] as const;
const SAPI_FIELDS = ["backend", "voice", "rate", "semitones", "filter", "web", "gain"] as const;
const EFFECT_FIELDS = ["sfx", "pitch", "gain", "params"] as const;
const PARAM_FIELDS = ["amount", "mine", "timbre", "mythic", "urgent", "release"] as const;
const ASSIGNMENT_FIELDS = ["voice", "text", "lines", "effect"] as const;
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
  // R1088: a trigger lists its lines; any other hook speaks one text.
  if (o.lines !== undefined) {
    if (!Array.isArray(o.lines) || o.lines.length === 0) fail(`${path}.lines`, "must be a non-empty list of strings");
    const lines = o.lines.map((line, i) => text(line, `${path}.lines[${i}]`));
    if (o.text !== undefined) fail(`${path}.text`, "a trigger lists its lines instead");
    return effect === undefined ? { voice, lines } : { voice, lines, effect };
  }
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

/** The `emotes` section's one entry shape (R644): a voice and the five issue-§3 lines. */
function parseEmoteEntry(raw: unknown, path: string, voices: Record<string, Persona>): EmoteLineEntry {
  const o = record(raw, path);
  const persona = text(o.persona, `${path}.persona`);
  if (!Object.hasOwn(voices, persona)) fail(`${path}.persona`, `unknown voice "${persona}"`);
  const overrides: { rate?: number; pbas?: number; pmod?: number } = {};
  if (o.rate !== undefined) overrides.rate = numberIn(o.rate, `${path}.rate`, ...RATE_RANGE);
  if (o.pbas !== undefined) overrides.pbas = numberIn(o.pbas, `${path}.pbas`, ...PITCH_BASE_RANGE);
  if (o.pmod !== undefined) overrides.pmod = numberIn(o.pmod, `${path}.pmod`, ...PITCH_MOD_RANGE);
  return {
    persona,
    greetings: text(o.greetings, `${path}.greetings`),
    wellPlayed: text(o.wellPlayed, `${path}.wellPlayed`),
    oops: text(o.oops, `${path}.oops`),
    thanks: text(o.thanks, `${path}.thanks`),
    threaten: text(o.threaten, `${path}.threaten`),
    ...overrides,
  };
}

/**
 * The parsed file, checked against `catalog`; throws Error("card-audio.json5: <path>: <problem>") on a shape
 * error. That every catalog card has its lines, and the word limits, the tests check (B33, B34). The
 * `emotes` section (R644) is optional, so a small test table without it still parses.
 */
export function parseCardAudio(raw: unknown, catalog: CatalogTypes): CardAudioTable {
  const root = record(raw, "(root)");
  const sections = Object.keys(root);
  const headed = sections.slice(0, 3).join() === SECTIONS.join();
  const tailed = sections.length === 3 || (sections.length === 4 && sections[3] === "emotes");
  if (!headed || !tailed) {
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
  const emotes: Record<string, EmoteLineEntry> = {};
  for (const [portrait, value] of Object.entries(record(root.emotes ?? {}, "emotes"))) {
    emotes[portrait] = parseEmoteEntry(value, `emotes.${portrait}`, voices);
  }
  return { voices, effects, cards, emotes };
}

/** A JSON5 word: an identifier, a number or a literal (`true`, `Infinity`, `-0.5`, `1e3`). */
const WORD = /[\w$+\-.]+/y;

/**
 * The path of the first key written twice in one object, or null: JSON5.parse silently keeps the last,
 * so a second entry left by a merge would replace the first. `source` has already parsed, so it is well formed.
 */
export function duplicateKeyPath(source: string): string | null {
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
 * R655, #2498: the raw table with its words filled in from `voiceText` (`voice-text.json`'s shape,
 * `{ cards: { <id>: { <hook>: <line>, trigger: [<line>, …] } }, emotes: { <portrait>: { <line id>: <line> } } }`):
 * each hook that names a voice gets its card's line as its `text` (a trigger its list as its `lines`), and
 * each portrait its five lines. Nothing else moves. Throws when the table writes a line itself, which is the
 * YAML's to write, when the YAML has a line no voice speaks, and when a voiced hook or a portrait has no lines.
 */
export function withVoiceText(raw: unknown, voiceText: unknown): unknown {
  if (!isRecord(raw)) return raw;
  const words = isRecord(voiceText) ? voiceText : {};
  const lines = isRecord(words.cards) ? words.cards : {};
  const spoken = isRecord(words.emotes) ? words.emotes : {};
  const table = { ...raw };
  const cards = isRecord(raw.cards) ? raw.cards : {};
  if (isRecord(raw.cards)) {
    const filled: Record<string, unknown> = {};
    for (const [defId, entry] of Object.entries(raw.cards)) {
      const own = isRecord(lines[defId]) ? lines[defId] : {};
      if (!isRecord(entry)) {
        filled[defId] = entry;
        continue;
      }
      const hooks: Record<string, unknown> = {};
      for (const [hook, assignment] of Object.entries(entry)) {
        if (!isRecord(assignment)) {
          hooks[hook] = assignment;
          continue;
        }
        for (const field of ["text", "lines"]) {
          if (field in assignment) fail(`cards.${defId}.${hook}.${field}`, `a line belongs in ${TEXT_FILE}`);
        }
        if (assignment.voice === undefined) {
          hooks[hook] = assignment;
          continue;
        }
        if (!Object.hasOwn(own, hook)) failText(`${defId}.voice.${hook}`, `missing (${FILE} gives the hook a voice)`);
        const { voice, ...rest } = assignment;
        hooks[hook] = { voice, [hook === "trigger" ? "lines" : "text"]: own[hook], ...rest };
      }
      filled[defId] = hooks;
    }
    table.cards = filled;
  }
  for (const [defId, own] of Object.entries(lines)) {
    const entry = cards[defId];
    for (const hook of Object.keys(isRecord(own) ? own : {})) {
      const assignment = isRecord(entry) ? entry[hook] : undefined;
      if (!isRecord(assignment) || assignment.voice === undefined) {
        failText(`${defId}.voice.${hook}`, `no voice in ${FILE} speaks it`);
      }
    }
  }
  const emotes = isRecord(raw.emotes) ? raw.emotes : {};
  for (const portrait of Object.keys(spoken)) {
    if (!isRecord(emotes[portrait])) failText(`emotes.${portrait}`, `${FILE} has no such portrait`);
  }
  if (isRecord(raw.emotes)) {
    const filled: Record<string, unknown> = {};
    for (const [portrait, entry] of Object.entries(raw.emotes)) {
      if (!isRecord(entry)) {
        filled[portrait] = entry;
        continue;
      }
      for (const line of VOICE_EMOTE_IDS) {
        if (line in entry) fail(`emotes.${portrait}.${line}`, `a line belongs in ${TEXT_FILE}`);
      }
      const own = spoken[portrait];
      if (!isRecord(own)) failText(`emotes.${portrait}`, `missing (${FILE} gives the portrait a voice)`);
      filled[portrait] = { ...entry, ...own };
    }
    table.emotes = filled;
  }
  return table;
}

/**
 * `parseCardAudio` of JSON5 text, its words filled in from `voiceText` when it is given (`withVoiceText`).
 * A syntax error or a key written twice in one object is reported with its line and column.
 */
export function parseCardAudioText(source: string, catalog: CatalogTypes, voiceText?: unknown): CardAudioTable {
  let raw: unknown;
  try {
    raw = JSON5.parse(source);
  } catch (err) {
    fail("(syntax)", messageOf(err));
  }
  const twice = duplicateKeyPath(source);
  if (twice !== null) fail(twice, "written twice in one object (only the last would count)");
  return parseCardAudio(voiceText === undefined ? raw : withVoiceText(raw, voiceText), catalog);
}

export const CARD_AUDIO: CardAudioTable = parseCardAudioText(
  rawCardAudio,
  catalogJson as unknown as CatalogTypes,
  voiceTextJson,
);
/** Generated by gen-voice.mjs and checked against the files by its `--check` (B35, B37). */
export const VOICE_MANIFEST: VoiceManifest = rawManifest as VoiceManifest;

export const MUSIC_MANIFEST: MusicManifest = parseMusicManifest(rawMusicManifest);
export const MUSIC_CARDS: Readonly<Record<string, MusicCardEntry>> = parseMusicCards(rawCards, MUSIC_MANIFEST);

export const SOUND_TABLES: SoundTables = {
  cardAudio: CARD_AUDIO,
  voiceManifest: VOICE_MANIFEST,
  musicManifest: MUSIC_MANIFEST,
  musicCards: MUSIC_CARDS,
};

export default SOUND_TABLES;
