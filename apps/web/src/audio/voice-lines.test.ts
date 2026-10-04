// Polish task 2 (docs/polish/2-sound.md), behaviours B33 and B34, and R651's file: `card-audio.json5`
// against the catalog it voices.
//
//   B33  exactly the catalog's ids (Core's 111 among them), each card's kind from its catalog type
//        (the file never states it); a unit carries a `play` and a `death` line (a hook with a voice
//        and a text) and no `cast` hook, a spell or trap a `cast` line and no `play`, `attack` or
//        `death` hook; no hook outside CARD_HOOK_NAMES and no field on a hook but voice, text and
//        effect; and every voice a line names exists, usable for its backend (a `say` voice with
//        in-range rate, pbas and pmod, or since R501 a SAPI voice with in-range rate and semitones and
//        a filter chain, and neither carrying the other's fields), with web values and (where set)
//        loudness trim `gain`, 0-2.
//   B34  every line is non-empty, uses only /^[A-Za-z ,.'!?-]+$/, stays within
//        VOICE_MAX_WORDS[hook] words (a word is a token that contains a letter), and uses a
//        BANNED_RULES_WORDS entry as a whole word, in any case, in fewer than BANNED_WORDS_MAX_SHARE
//        (2%) of the lines (issue #115). The lines that do are named in the failure and in the
//        passing test's title count, so none is lost by being under the allowance.
//   R651 the sections are voices, effects and cards, in that order; each card's entry names the card
//        in a comment on the line of its id, in catalog order; comments and trailing commas parse;
//        every effect a hook names is in the bank, every bank effect is a recipe sfx.ts has, and
//        every one is used.
//
// Both files are read off disk rather than imported, so this checks the committed file itself and
// not whatever `voiceData.ts` makes of it. Each test collects every offender before asserting, so
// a red run names all of them at once instead of the first.

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import JSON5 from "json5";
import { describe, expect, it } from "vitest";

import { BANNED_RULES_WORDS, BANNED_WORDS_MAX_SHARE, CARD_HOOK_NAMES, VOICE_MAX_WORDS } from "./constants.ts";
import { SFX_IDS } from "./sfx.ts";

const here = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(here, "../../../..");
const CATALOG_PATH = resolve(REPO, "packages/cards/catalog.json");
const AUDIO_PATH = resolve(here, "card-audio.json5");

type EntryKind = "unit" | "spell" | "trap";
type Hook = "play" | "attack" | "death" | "cast";

/** docs/polish/2-sound.md: Unit -> unit; Spell and Field Spell -> spell; Trap and Field Trap -> trap. */
const KIND_OF_TYPE: Readonly<Record<string, EntryKind>> = {
  Unit: "unit",
  Spell: "spell",
  "Field Spell": "spell",
  Trap: "trap",
  "Field Trap": "trap",
};

/** The hooks each kind may carry (constants.ts CARD_HOOKS), and the lines SPEC §10.11 requires of it. */
const HOOKS_OF_KIND: Readonly<Record<EntryKind, readonly Hook[]>> = {
  unit: ["play", "attack", "death"],
  spell: ["cast"],
  trap: ["cast"],
};
const LINES_OF_KIND: Readonly<Record<EntryKind, readonly Hook[]>> = {
  unit: ["play", "death"],
  spell: ["cast"],
  trap: ["cast"],
};
/** The fields a hook's assignment allows (types.ts HookAssignment). */
const ASSIGNMENT_FIELDS: readonly string[] = ["voice", "text", "effect"];

/** R501: the SAPI voices a Windows install ships, which the SAPI voices choose from. */
const SAPI_VOICES: readonly string[] = ["Microsoft David Desktop", "Microsoft Zira Desktop"];
/** gen-voice.mjs's FILTER_CHARSET: filter names, numbers, `=`, `:`, `,`, `.`, `|` and `-`. */
const FILTER_CHARSET = /^[a-z0-9_=:,.|-]*$/;
/** R501: the fields only a `say` voice carries, and those only a SAPI one does. */
const SAY_ONLY_FIELDS = ["say", "pbas", "pmod"] as const;
const SAPI_ONLY_FIELDS = ["voice", "semitones", "filter"] as const;

/** B34's charset, verbatim. */
const LINE_CHARSET = /^[A-Za-z ,.'!?-]+$/;

/** R651: a card entry's first line, `    "<id>": { // <the card's name>`. */
const ENTRY_LINE = /^ {4}"([^"]+)": \{ \/\/ (.+)$/;

type Json = Record<string, unknown>;

function isRecord(value: unknown): value is Json {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

const catalogRaw = JSON.parse(readFileSync(CATALOG_PATH, "utf8")) as unknown;
if (!isRecord(catalogRaw)) throw new Error(`${CATALOG_PATH}: expected an object keyed by card id`);
const CATALOG = catalogRaw as Record<string, { type?: unknown; name?: unknown }>;
const CATALOG_IDS: readonly string[] = Object.keys(CATALOG);

const SOURCE = readFileSync(AUDIO_PATH, "utf8");
const tableRaw = JSON5.parse<unknown>(SOURCE);
if (!isRecord(tableRaw)) throw new Error(`${AUDIO_PATH}: expected an object`);
const TABLE: Json = tableRaw;
const VOICES: Json = isRecord(TABLE.voices) ? TABLE.voices : {};
const EFFECTS: Json = isRecord(TABLE.effects) ? TABLE.effects : {};
const CARDS: Record<string, Json> = Object.fromEntries(
  Object.entries(isRecord(TABLE.cards) ? TABLE.cards : {}).map(([id, entry]) => [id, isRecord(entry) ? entry : {}]),
);

function catalogKind(id: string): EntryKind | undefined {
  const type = CATALOG[id]?.type;
  return typeof type === "string" ? KIND_OF_TYPE[type] : undefined;
}

/** Every hook assignment in the file, whatever its shape, so a malformed one is still checked. */
type Assignment = { defId: string; key: string; hook: string; value: unknown };

function allAssignments(): Assignment[] {
  return Object.entries(CARDS).flatMap(([defId, entry]) =>
    Object.entries(entry).map(([hook, value]) => ({ defId, key: `${defId}-${hook}`, hook, value })),
  );
}

/** A hook's voice line: an assignment with a voice and a text. */
function isLine(value: unknown): boolean {
  return isRecord(value) && typeof value.voice === "string" && typeof value.text === "string";
}

/** Every line the file carries (an assignment that gives a voice or a text), so a malformed one is still checked. */
type Line = { key: string; hook: Hook; text: unknown };

function allLines(): Line[] {
  return allAssignments()
    .filter(({ hook, value }) => (CARD_HOOK_NAMES as readonly string[]).includes(hook) && isRecord(value) && ("voice" in value || "text" in value))
    .map(({ key, hook, value }) => ({ key, hook: hook as Hook, text: (value as Json).text }));
}

/** R501: a voice rendered by Windows SAPI rather than macOS `say`. */
function isSapi(name: string): boolean {
  const voice = VOICES[name];
  return isRecord(voice) && voice.backend === "sapi";
}

/** The voices the cards table actually names. */
function referencedVoices(): string[] {
  const names = new Set<string>();
  for (const { value } of allAssignments()) {
    if (isRecord(value) && typeof value.voice === "string") names.add(value.voice);
  }
  return [...names].sort();
}

function inRange(value: unknown, min: number, max: number): boolean {
  return typeof value === "number" && Number.isFinite(value) && value >= min && value <= max;
}

/** B34: a word is a whitespace-separated token that contains a letter ("..." alone is not one). */
function wordCount(text: string): number {
  return text.split(/\s+/).filter((token) => /[A-Za-z]/.test(token)).length;
}

/** B34 (issue #115): whether `offending` lines out of `total` are strictly fewer than the allowed share. */
function withinBannedShare(offending: number, total: number): boolean {
  return total > 0 && offending / total < BANNED_WORDS_MAX_SHARE;
}

/** B34: whole word, case-insensitive; a multi-word entry matches across any run of whitespace. */
function bannedMatcher(word: string): RegExp {
  const body = word
    .trim()
    .split(/\s+/)
    .map((part) => part.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"))
    .join("\\s+");
  return new RegExp(`(?<![A-Za-z])${body}(?![A-Za-z])`, "i");
}

function collapse(text: string): string {
  return text.replace(/\s+/g, " ").trim();
}

/** R651: each card entry's id and the name its comment gives, in file order, read off the raw text. */
function entryComments(): { id: string; name: string }[] {
  const lines = SOURCE.split(/\r?\n/);
  const start = lines.findIndex((line) => /^ {2}cards: \{\s*$/.test(line));
  if (start < 0) return [];
  const end = lines.findIndex((line, at) => at > start && /^ {2}\}/.test(line));
  return lines
    .slice(start + 1, end < 0 ? lines.length : end)
    .flatMap((line) => {
      const match = ENTRY_LINE.exec(line);
      return match?.[1] === undefined || match[2] === undefined ? [] : [{ id: match[1], name: collapse(match[2]) }];
    });
}

describe("card-audio.json5 covers the catalog (B33)", () => {
  it("B33 holds one cards entry per catalog card, Core's 111 among them", () => {
    expect(
      CATALOG_IDS.filter((id) => id.startsWith("core-")),
      "packages/cards/catalog.json holds the 100 Core cards and 11 tokens",
    ).toHaveLength(111);
    expect(Object.keys(CARDS), "one cards entry per catalog id").toHaveLength(CATALOG_IDS.length);
  });

  it("B33 leaves no catalog id, tokens included, without an entry", () => {
    const missing = CATALOG_IDS.filter((id) => !(id in CARDS));
    expect(missing, "catalog ids with no card-audio entry").toEqual([]);
  });

  it("B33 has no entry for an id the catalog does not hold", () => {
    const known = new Set(CATALOG_IDS);
    const extra = Object.keys(CARDS).filter((id) => !known.has(id));
    expect(extra, "card-audio entries for ids outside packages/cards/catalog.json").toEqual([]);
  });

  it("B33 takes every entry's kind from its catalog type, which the file never states", () => {
    const unknownTypes = CATALOG_IDS.filter((id) => catalogKind(id) === undefined).map(
      (id) => `${id}: catalog type ${JSON.stringify(CATALOG[id]?.type)}`,
    );
    expect(unknownTypes, "every catalog type is Unit, Spell, Field Spell, Trap or Field Trap").toEqual([]);

    const stated = Object.entries(CARDS)
      .filter(([, entry]) => "kind" in entry || "persona" in entry)
      .map(([id]) => id);
    expect(stated, "entries that state a kind or a persona of their own").toEqual([]);
  });

  it("B33 gives every unit a play and a death line and no cast hook", () => {
    const wrong: string[] = [];
    for (const id of CATALOG_IDS.filter((catalogId) => catalogKind(catalogId) === "unit")) {
      const entry = CARDS[id] ?? {};
      if (!isLine(entry.play)) wrong.push(`${id}: no play line`);
      if (!isLine(entry.death)) wrong.push(`${id}: no death line`);
      if ("cast" in entry) wrong.push(`${id}: a unit carries a cast hook`);
    }
    expect(wrong).toEqual([]);
  });

  it("B33 gives every spell and trap a cast line and no play, attack or death hook", () => {
    const wrong: string[] = [];
    for (const id of CATALOG_IDS.filter((catalogId) => catalogKind(catalogId) !== "unit")) {
      const entry = CARDS[id] ?? {};
      if (!isLine(entry.cast)) wrong.push(`${id}: no cast line`);
      for (const hook of ["play", "attack", "death"] as const) {
        if (hook in entry) wrong.push(`${id}: a ${String(catalogKind(id))} carries a ${hook} hook`);
      }
    }
    expect(wrong).toEqual([]);
  });

  it("B33 puts no hook on an entry outside CARD_HOOK_NAMES and its kind's, and no field on a hook but voice, text and effect", () => {
    expect([...CARD_HOOK_NAMES], "the hooks constants.ts CARD_HOOKS names").toEqual(["play", "attack", "death", "cast"]);
    const wrong: string[] = [];
    for (const { defId, key, hook, value } of allAssignments()) {
      const kind = catalogKind(defId);
      if (!(CARD_HOOK_NAMES as readonly string[]).includes(hook)) {
        wrong.push(`${key}: "${hook}" is not a hook`);
        continue;
      }
      if (kind !== undefined && !HOOKS_OF_KIND[kind].includes(hook as Hook)) wrong.push(`${key}: a ${kind} has no ${hook} hook`);
      if (!isRecord(value)) {
        wrong.push(`${key}: ${JSON.stringify(value)} is not an object`);
        continue;
      }
      for (const field of Object.keys(value)) {
        if (!ASSIGNMENT_FIELDS.includes(field)) wrong.push(`${key}: unexpected field "${field}"`);
      }
      if ("voice" in value !== "text" in value) wrong.push(`${key}: a voice without a text, or a text without a voice`);
      if (!("voice" in value) && !("effect" in value)) wrong.push(`${key}: neither a line nor an effect`);
      for (const field of ASSIGNMENT_FIELDS) {
        if (field in value && typeof value[field] !== "string") wrong.push(`${key}: ${field} is ${JSON.stringify(value[field])}`);
      }
    }
    expect(wrong).toEqual([]);
  });

  it("B33 names only voices that exist", () => {
    const dangling = allAssignments()
      .filter(({ value }) => isRecord(value) && "voice" in value)
      .filter(({ value }) => {
        const voice = (value as Json).voice;
        return typeof voice !== "string" || !isRecord(VOICES[voice]);
      })
      .map(({ key, value }) => `${key}: voice ${JSON.stringify((value as Json).voice)}`);
    expect(dangling, "lines whose voice is missing from voices").toEqual([]);
  });

  it("B33 gives every referenced `say` voice a non-empty say voice", () => {
    const voices = referencedVoices();
    expect(voices.length, "the cards table names at least one voice").toBeGreaterThan(0);
    const wrong = voices
      .filter((name) => isRecord(VOICES[name]) && !isSapi(name))
      .filter((name) => {
        const say = (VOICES[name] as Json).say;
        return typeof say !== "string" || say.trim() === "";
      });
    expect(wrong, "voices with no usable `say -v` voice").toEqual([]);
  });

  it("R501 gives every referenced SAPI voice a SAPI voice, a rate in -50-100, semitones in -12-12 and a filter chain", () => {
    const sapi = referencedVoices().filter(isSapi);
    expect(sapi.length, "the cards table names at least one SAPI voice").toBeGreaterThan(0);
    const wrong: string[] = [];
    for (const name of sapi) {
      const voice = VOICES[name] as Json;
      if (typeof voice.voice !== "string" || !SAPI_VOICES.includes(voice.voice)) {
        wrong.push(`${name}: voice ${JSON.stringify(voice.voice)}`);
      }
      if (!inRange(voice.rate, -50, 100)) wrong.push(`${name}: rate ${JSON.stringify(voice.rate)}`);
      if (!inRange(voice.semitones, -12, 12)) wrong.push(`${name}: semitones ${JSON.stringify(voice.semitones)}`);
      if (typeof voice.filter !== "string" || !FILTER_CHARSET.test(voice.filter)) {
        wrong.push(`${name}: filter ${JSON.stringify(voice.filter)}`);
      }
    }
    expect(wrong).toEqual([]);
  });

  it("R501 keeps each backend's fields to its own voices: no say, pbas or pmod on a SAPI voice, no voice, semitones or filter on a `say` one", () => {
    const wrong: string[] = [];
    for (const [name, voice] of Object.entries(VOICES)) {
      if (!isRecord(voice)) continue;
      const foreign = isSapi(name) ? SAY_ONLY_FIELDS : SAPI_ONLY_FIELDS;
      for (const field of foreign) {
        if (field in voice) wrong.push(`${name}: a ${isSapi(name) ? "SAPI" : "say"} voice carries ${field}`);
      }
    }
    expect(wrong).toEqual([]);
  });

  it("B33 keeps every referenced `say` voice's rate within 90-360 and its pbas and pmod within 0-127", () => {
    const wrong: string[] = [];
    for (const name of referencedVoices().filter((voice) => !isSapi(voice))) {
      const voice = VOICES[name];
      if (!isRecord(voice)) continue;
      if (!inRange(voice.rate, 90, 360)) wrong.push(`${name}: rate ${JSON.stringify(voice.rate)}`);
      if (!inRange(voice.pbas, 0, 127)) wrong.push(`${name}: pbas ${JSON.stringify(voice.pbas)}`);
      if (!inRange(voice.pmod, 0, 127)) wrong.push(`${name}: pmod ${JSON.stringify(voice.pmod)}`);
      if ("gain" in voice && !inRange(voice.gain, 0, 2)) wrong.push(`${name}: gain ${JSON.stringify(voice.gain)}`);
    }
    expect(wrong).toEqual([]);
  });

  it("B33 keeps every referenced voice's web pitch within 0-2 and web rate within 0.1-10", () => {
    const wrong: string[] = [];
    for (const name of referencedVoices()) {
      const voice = VOICES[name];
      if (!isRecord(voice)) continue;
      const web = voice.web;
      if (!isRecord(web)) {
        wrong.push(`${name}: no web block`);
        continue;
      }
      if (!inRange(web.pitch, 0, 2)) wrong.push(`${name}: web pitch ${JSON.stringify(web.pitch)}`);
      if (!inRange(web.rate, 0.1, 10)) wrong.push(`${name}: web rate ${JSON.stringify(web.rate)}`);
    }
    expect(wrong).toEqual([]);
  });

  it("B33 gives Core 44 units (the Ghoul Token included, R353) and 67 spells and traps (The Coin included, R245), which is 155 lines", () => {
    const kinds = Object.keys(CARDS)
      .filter((id) => id.startsWith("core-"))
      .map(catalogKind);
    expect(kinds.filter((kind) => kind === "unit"), "unit entries").toHaveLength(44);
    expect(kinds.filter((kind) => kind === "spell" || kind === "trap"), "spell and trap entries").toHaveLength(67);
    // The lines SPEC §10.11 requires: an attack line (R651) is optional and not counted.
    const coreLines = allAssignments().filter(({ defId, hook, value }) => defId.startsWith("core-") && hook !== "attack" && isLine(value));
    expect(coreLines, "Core's lines in the table").toHaveLength(155);
  });

  it("B33 gives every card a line per its kind: twice the units plus the spells and traps", () => {
    const units = Object.keys(CARDS).filter((id) => catalogKind(id) === "unit").length;
    // An attack line (R651) is a Unit's option beside these, and is held to B34 like any other.
    const lines = allAssignments().filter(({ hook, value }) => hook !== "attack" && isLine(value));
    expect(lines, "lines in the table").toHaveLength(units * 2 + (Object.keys(CARDS).length - units));
    const required = Object.keys(CARDS).flatMap((id) => {
      const kind = catalogKind(id);
      return kind === undefined ? [] : LINES_OF_KIND[kind].map((hook) => `${id}-${hook}`);
    });
    expect(lines.map(({ key }) => key).sort(), "the lines are exactly the ones SPEC §10.11 requires").toEqual(required.sort());
  });
});

describe("every voice line is short, plain flavour (B34)", () => {
  it("B34 has no empty or blank line", () => {
    const lines = allLines();
    expect(lines.length, "lines to check").toBeGreaterThan(0);
    const empty = lines.filter(({ text }) => typeof text !== "string" || text.trim() === "").map(({ key }) => key);
    expect(empty, "lines that are missing, not strings, or blank").toEqual([]);
  });

  it("B34 uses only letters, spaces and , . ' ! ? - in every line", () => {
    const wrong = allLines()
      .filter(({ text }) => typeof text !== "string" || !LINE_CHARSET.test(text))
      .map(({ key, text }) => `${key}: ${JSON.stringify(text)}`);
    expect(wrong, "lines outside /^[A-Za-z ,.'!?-]+$/").toEqual([]);
  });

  it("B34 keeps every line within VOICE_MAX_WORDS for its hook (play 8, attack 4, death 6, cast 8)", () => {
    expect(VOICE_MAX_WORDS, "the limits the Surface fixes").toEqual({ play: 8, attack: 4, death: 6, cast: 8 });
    // The counter itself: punctuation-only tokens are not words, hyphenated and elided ones are one.
    expect(wordCount("Too... slow...")).toBe(2);
    expect(wordCount("Oops - Surf's up!")).toBe(3);

    const long = allLines()
      .filter(({ hook, text }) => typeof text === "string" && wordCount(text) > VOICE_MAX_WORDS[hook])
      .map(({ key, hook, text }) => `${key}: ${wordCount(String(text))} words > ${VOICE_MAX_WORDS[hook]}`);
    expect(long, "lines over their word limit").toEqual([]);
  });

  it("B34 passes a corpus with fewer than 2% of its lines restating rules vocabulary, and no more", () => {
    expect(BANNED_WORDS_MAX_SHARE).toBe(0.02);
    // 1 line in 100 is 1%; 2 in 100 is 2%, which is not fewer than 2%.
    expect(withinBannedShare(0, 100)).toBe(true);
    expect(withinBannedShare(1, 100)).toBe(true);
    expect(withinBannedShare(2, 100)).toBe(false);
    // 1 in 51 is 1.96% and passes; 1 in 50 is exactly 2% and does not.
    expect(withinBannedShare(1, 51)).toBe(true);
    expect(withinBannedShare(1, 50)).toBe(false);
    // An empty corpus proves nothing.
    expect(withinBannedShare(0, 0)).toBe(false);
  });

  it("B34 uses a BANNED_RULES_WORDS entry as a whole word, in any case, in fewer than 2% of the lines", () => {
    expect([...BANNED_RULES_WORDS], "the rules vocabulary the Surface bans").toEqual([
      "Taunt", "Divine Shield", "Reborn", "Lifesteal", "Poisonous", "First Strike", "Trample", "Cleave", "Pierce",
      "Immutable", "Indestructible", "Stack", "Echo", "Combo", "Discover", "Recruit", "Tribute",
      "Embiggen", "Radiant", "Armor", "Rush", "Charge", "Cry", "Deathrattle", "Battlecry", "mana",
      "damage", "summon", "exile", "fatigue", "backrow", "graveyard",
      "Animated", "Activate", "Brittle", "Degrade", "Upgrade", "Spell Damage", "Immune to Spells", "Counter",
      "Flicker", "Plague Token",
    ]);
    // The matcher itself: whole words in any case, across whitespace, and never inside a longer word.
    expect(bannedMatcher("Taunt").test("I TAUNT you!")).toBe(true);
    expect(bannedMatcher("Divine Shield").test("a divine  shield, dear")).toBe(true);
    expect(bannedMatcher("Echo").test("Echoes of the past")).toBe(false);
    expect(bannedMatcher("mana").test("Banana!")).toBe(false);

    const matchers = BANNED_RULES_WORDS.map((word) => ({ word, pattern: bannedMatcher(word) }));
    const offences: string[] = [];
    const offendingLines = new Set<string>();
    const lines = allLines().filter(({ text }) => typeof text === "string");
    for (const { key, text } of lines) {
      for (const { word, pattern } of matchers) {
        if (pattern.test(String(text))) {
          offences.push(`${key}: "${word}" in ${JSON.stringify(text)}`);
          offendingLines.add(key);
        }
      }
    }
    // Each offender is named, so a pass under the allowance still shows what stands.
    expect(
      withinBannedShare(offendingLines.size, lines.length),
      `${String(offendingLines.size)} of ${String(lines.length)} lines restate rules vocabulary, which must be under ${String(BANNED_WORDS_MAX_SHARE * 100)}%: ${offences.join("; ")}`,
    ).toBe(true);
  });
});

describe("card-audio.json5's layout and effects (R651)", () => {
  it("R651 holds three sections, voices, effects and cards, in that order", () => {
    expect(Object.keys(TABLE)).toEqual(["voices", "effects", "cards"]);
  });

  it("R651 names each card in a comment on the line of its id, as the catalog does, in catalog order", () => {
    const comments = entryComments();
    const wrong = comments
      .filter(({ id, name }) => {
        const catalogName = CATALOG[id]?.name;
        return typeof catalogName !== "string" || collapse(catalogName) !== name;
      })
      .map(({ id, name }) => `${id}: comment "${name}", catalog name ${JSON.stringify(CATALOG[id]?.name)}`);
    expect(wrong, "entries whose comment is not the card's catalog name").toEqual([]);

    // An entry written without its comment on the line of its id is not found above.
    const commented = new Set(comments.map(({ id }) => id));
    const uncommented = Object.keys(CARDS).filter((id) => !commented.has(id));
    expect(uncommented, "entries with no name comment on the line of their id").toEqual([]);
    expect([...commented].sort(), "the commented ids are the cards' ids").toEqual(Object.keys(CARDS).sort());

    const known = new Set(Object.keys(CARDS));
    expect(
      Object.keys(CARDS).filter((id) => id in CATALOG),
      "entries in the order packages/cards/catalog.json lists their cards",
    ).toEqual(CATALOG_IDS.filter((id) => known.has(id)));
  });

  it("R651 is JSON5: its comments and trailing commas parse, where JSON would refuse them", () => {
    expect(SOURCE, "the file carries // comments").toMatch(/^\s*\/\/ /m);
    expect(SOURCE, "the file carries a trailing comma before a closing brace").toMatch(/,\s*\n\s*\}/);
    expect(() => JSON.parse(SOURCE) as unknown, "JSON.parse refuses the file").toThrow();
    expect(() => JSON5.parse<unknown>(SOURCE), "JSON5.parse reads it").not.toThrow();
    expect(JSON5.parse<unknown>('{ a: 1, // a comment\n b: { c: "d", },\n}\n')).toEqual({ a: 1, b: { c: "d" } });
  });

  it("R651 names only effects the effects bank holds", () => {
    const dangling = allAssignments()
      .filter(({ value }) => isRecord(value) && "effect" in value)
      .filter(({ value }) => {
        const effect = (value as Json).effect;
        return typeof effect !== "string" || !Object.hasOwn(EFFECTS, effect);
      })
      .map(({ key, value }) => `${key}: effect ${JSON.stringify((value as Json).effect)}`);
    expect(dangling, "hooks whose effect is missing from effects").toEqual([]);
  });

  it("R651 builds every bank effect on a recipe sfx.ts has (SFX_IDS)", () => {
    expect(Object.keys(EFFECTS).length, "the bank holds at least one effect").toBeGreaterThan(0);
    const wrong = Object.entries(EFFECTS)
      .filter(([, effect]) => !isRecord(effect) || typeof effect.sfx !== "string" || !(SFX_IDS as readonly string[]).includes(effect.sfx))
      .map(([name, effect]) => `${name}: sfx ${JSON.stringify(isRecord(effect) ? effect.sfx : effect)}`);
    expect(wrong, "effects whose sfx is not a recipe").toEqual([]);
  });

  it("R651 uses every bank effect on at least one card", () => {
    const used = new Set(
      allAssignments().flatMap(({ value }) => (isRecord(value) && typeof value.effect === "string" ? [value.effect] : [])),
    );
    const dead = Object.keys(EFFECTS).filter((name) => !used.has(name));
    expect(dead, "effects no card's hook names").toEqual([]);
  });
});
