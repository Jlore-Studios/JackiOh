// Polish task 2 (docs/polish/2-sound.md), behaviours B33 and B34: `voice-lines.json` against the
// catalog it voices.
//
//   B33  exactly the catalog's ids (Core's 111 among them), each `kind` from the catalog type, units
//        carry `play` and `death` and nothing else carries either, non-units carry `cast`, and every
//        referenced persona exists with a usable voice for its backend (a `say` voice with in-range
//        rate, pbas and pmod, or since R501 a SAPI voice with in-range rate and semitones and a
//        filter chain), web values and (where set) loudness trim `gain`, 0-2.
//   B34  every line is non-empty, uses only /^[A-Za-z ,.'!?-]+$/, stays within
//        VOICE_MAX_WORDS[line] words (a word is a token that contains a letter), and uses a
//        BANNED_RULES_WORDS entry as a whole word, in any case, in fewer than BANNED_WORDS_MAX_SHARE
//        (2%) of the lines (issue #115). The lines that do are named in the failure and in the
//        passing test's title count, so none is lost by being under the allowance.
//
// Both files are read off disk rather than imported, so this checks the committed JSON itself and
// not whatever `voiceData.ts` makes of it. Each test collects every offender before asserting, so
// a red run names all of them at once instead of the first.

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { BANNED_RULES_WORDS, BANNED_WORDS_MAX_SHARE, VOICE_MAX_WORDS } from "./constants.ts";

const here = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(here, "../../../..");
const CATALOG_PATH = resolve(REPO, "packages/cards/catalog.json");
const LINES_PATH = resolve(here, "voice-lines.json");

type EntryKind = "unit" | "spell" | "trap";
type LineKind = "play" | "death" | "cast";

const LINE_KINDS: readonly LineKind[] = ["play", "death", "cast"];

/** docs/polish/2-sound.md: Unit -> unit; Spell and Field Spell -> spell; Trap and Field Trap -> trap. */
const KIND_OF_TYPE: Readonly<Record<string, EntryKind>> = {
  Unit: "unit",
  Spell: "spell",
  "Field Spell": "spell",
  Trap: "trap",
  "Field Trap": "trap",
};

/** The fields `VoiceLineEntry` allows for each kind (types.ts), per-card overrides included. */
const OVERRIDE_FIELDS = ["rate", "pbas", "pmod"] as const;
const ALLOWED_FIELDS: Readonly<Record<EntryKind, readonly string[]>> = {
  unit: ["kind", "persona", "play", "death", ...OVERRIDE_FIELDS],
  spell: ["kind", "persona", "cast", ...OVERRIDE_FIELDS],
  trap: ["kind", "persona", "cast", ...OVERRIDE_FIELDS],
};

/** R501: the SAPI voices a Windows install ships, which the SAPI personas choose from. */
const SAPI_VOICES: readonly string[] = ["Microsoft David Desktop", "Microsoft Zira Desktop"];
/** gen-voice.mjs's FILTER_CHARSET: filter names, numbers, `=`, `:`, `,`, `.`, `|` and `-`. */
const FILTER_CHARSET = /^[a-z0-9_=:,.|-]*$/;

/** B34's charset, verbatim. */
const LINE_CHARSET = /^[A-Za-z ,.'!?-]+$/;

type Json = Record<string, unknown>;

function isRecord(value: unknown): value is Json {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function readJson(path: string): unknown {
  return JSON.parse(readFileSync(path, "utf8")) as unknown;
}

const catalogRaw = readJson(CATALOG_PATH);
if (!isRecord(catalogRaw)) throw new Error(`${CATALOG_PATH}: expected an object keyed by card id`);
const CATALOG = catalogRaw as Record<string, { type?: unknown }>;
const CATALOG_IDS: readonly string[] = Object.keys(CATALOG);

const tableRaw = readJson(LINES_PATH);
if (!isRecord(tableRaw)) throw new Error(`${LINES_PATH}: expected an object`);
const TABLE: Json = tableRaw;
const PERSONAS: Json = isRecord(TABLE.personas) ? TABLE.personas : {};
const CARDS: Record<string, Json> = Object.fromEntries(
  Object.entries(isRecord(TABLE.cards) ? TABLE.cards : {}).map(([id, entry]) => [id, isRecord(entry) ? entry : {}]),
);

function catalogKind(id: string): EntryKind | undefined {
  const type = CATALOG[id]?.type;
  return typeof type === "string" ? KIND_OF_TYPE[type] : undefined;
}

/** Every line the table carries, whatever its shape, so a malformed one is still checked. */
type Line = { key: string; line: LineKind; text: unknown };

function allLines(): Line[] {
  const out: Line[] = [];
  for (const [defId, entry] of Object.entries(CARDS)) {
    for (const line of LINE_KINDS) {
      if (line in entry) out.push({ key: `${defId}-${line}`, line, text: entry[line] });
    }
  }
  return out;
}

/** R501: a persona rendered by Windows SAPI rather than macOS `say`. */
function isSapi(name: string): boolean {
  const persona = PERSONAS[name];
  return isRecord(persona) && persona.backend === "sapi";
}

/** The personas the cards table actually names. */
function referencedPersonas(): string[] {
  const names = new Set<string>();
  for (const entry of Object.values(CARDS)) {
    if (typeof entry.persona === "string") names.add(entry.persona);
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

describe("voice-lines.json covers the catalog (B33)", () => {
  it("B33 declares version 1 and one cards entry per catalog card, Core's 111 among them", () => {
    expect(TABLE.version, "voice-lines.json version").toBe(1);
    expect(
      CATALOG_IDS.filter((id) => id.startsWith("core-")),
      "packages/cards/catalog.json holds the 100 Core cards and 11 tokens",
    ).toHaveLength(111);
    expect(Object.keys(CARDS), "one cards entry per catalog id").toHaveLength(CATALOG_IDS.length);
  });

  it("B33 leaves no catalog id, tokens included, without an entry", () => {
    const missing = CATALOG_IDS.filter((id) => !(id in CARDS));
    expect(missing, "catalog ids with no voice-lines entry").toEqual([]);
  });

  it("B33 has no entry for an id the catalog does not hold", () => {
    const known = new Set(CATALOG_IDS);
    const extra = Object.keys(CARDS).filter((id) => !known.has(id));
    expect(extra, "voice-lines entries for ids outside packages/cards/catalog.json").toEqual([]);
  });

  it("B33 gives every entry the kind its catalog type maps to", () => {
    const unknownTypes = CATALOG_IDS.filter((id) => catalogKind(id) === undefined).map(
      (id) => `${id}: catalog type ${JSON.stringify(CATALOG[id]?.type)}`,
    );
    expect(unknownTypes, "every catalog type is Unit, Spell, Field Spell, Trap or Field Trap").toEqual([]);

    const wrong = Object.entries(CARDS)
      .filter(([id, entry]) => CATALOG_IDS.includes(id) && entry.kind !== catalogKind(id))
      .map(([id, entry]) => `${id}: kind ${JSON.stringify(entry.kind)}, catalog says ${String(catalogKind(id))}`);
    expect(wrong, "entries whose kind disagrees with the catalog type").toEqual([]);
  });

  it("B33 gives every unit a play and a death line and no cast line", () => {
    const wrong: string[] = [];
    for (const id of CATALOG_IDS.filter((catalogId) => catalogKind(catalogId) === "unit")) {
      const entry = CARDS[id] ?? {};
      if (typeof entry.play !== "string") wrong.push(`${id}: no play line`);
      if (typeof entry.death !== "string") wrong.push(`${id}: no death line`);
      if ("cast" in entry) wrong.push(`${id}: a unit carries a cast line`);
    }
    expect(wrong).toEqual([]);
  });

  it("B33 gives every spell and trap a cast line and neither a play nor a death line", () => {
    const wrong: string[] = [];
    for (const id of CATALOG_IDS.filter((catalogId) => catalogKind(catalogId) !== "unit")) {
      const entry = CARDS[id] ?? {};
      if (typeof entry.cast !== "string") wrong.push(`${id}: no cast line`);
      if ("play" in entry) wrong.push(`${id}: a ${String(catalogKind(id))} carries a play line`);
      if ("death" in entry) wrong.push(`${id}: a ${String(catalogKind(id))} carries a death line`);
    }
    expect(wrong).toEqual([]);
  });

  it("B33 puts no field on an entry outside its kind's shape, and overrides are numbers", () => {
    const wrong: string[] = [];
    for (const [id, entry] of Object.entries(CARDS)) {
      const kind = entry.kind;
      if (kind !== "unit" && kind !== "spell" && kind !== "trap") {
        wrong.push(`${id}: kind ${JSON.stringify(kind)} is not unit, spell or trap`);
        continue;
      }
      for (const field of Object.keys(entry)) {
        if (!ALLOWED_FIELDS[kind].includes(field)) wrong.push(`${id}: unexpected field "${field}" on a ${kind}`);
      }
      for (const field of OVERRIDE_FIELDS) {
        if (field in entry && (typeof entry[field] !== "number" || !Number.isFinite(entry[field]))) {
          wrong.push(`${id}: override ${field} is ${JSON.stringify(entry[field])}, not a number`);
        }
      }
    }
    expect(wrong).toEqual([]);
  });

  it("B33 names only personas that exist", () => {
    const dangling = Object.entries(CARDS)
      .filter(([, entry]) => typeof entry.persona !== "string" || !isRecord(PERSONAS[entry.persona]))
      .map(([id, entry]) => `${id}: persona ${JSON.stringify(entry.persona)}`);
    expect(dangling, "entries whose persona is missing from personas").toEqual([]);
  });

  it("B33 gives every referenced `say` persona a non-empty say voice", () => {
    const personas = referencedPersonas();
    expect(personas.length, "the cards table names at least one persona").toBeGreaterThan(0);
    const wrong = personas
      .filter((name) => isRecord(PERSONAS[name]) && !isSapi(name))
      .filter((name) => {
        const say = (PERSONAS[name] as Json).say;
        return typeof say !== "string" || say.trim() === "";
      });
    expect(wrong, "personas with no usable `say -v` voice").toEqual([]);
  });

  it("R501 gives every referenced SAPI persona a SAPI voice, a rate in -50-100, semitones in -12-12 and a filter chain", () => {
    const wrong: string[] = [];
    for (const name of referencedPersonas().filter(isSapi)) {
      const persona = PERSONAS[name] as Json;
      if (typeof persona.voice !== "string" || !SAPI_VOICES.includes(persona.voice)) {
        wrong.push(`${name}: voice ${JSON.stringify(persona.voice)}`);
      }
      if (!inRange(persona.rate, -50, 100)) wrong.push(`${name}: rate ${JSON.stringify(persona.rate)}`);
      if (!inRange(persona.semitones, -12, 12)) wrong.push(`${name}: semitones ${JSON.stringify(persona.semitones)}`);
      if (typeof persona.filter !== "string" || !FILTER_CHARSET.test(persona.filter)) {
        wrong.push(`${name}: filter ${JSON.stringify(persona.filter)}`);
      }
      if ("say" in persona || "pbas" in persona || "pmod" in persona) wrong.push(`${name}: carries a say field`);
    }
    expect(wrong).toEqual([]);
  });

  it("R501 puts no rate, pbas or pmod override on a line a SAPI persona speaks", () => {
    const wrong = Object.entries(CARDS)
      .filter(([, entry]) => typeof entry.persona === "string" && isSapi(entry.persona))
      .filter(([, entry]) => OVERRIDE_FIELDS.some((field) => field in entry))
      .map(([id]) => id);
    expect(wrong).toEqual([]);
  });

  it("B33 keeps every referenced `say` persona's rate within 90-360 and its pbas and pmod within 0-127", () => {
    const wrong: string[] = [];
    for (const name of referencedPersonas().filter((persona) => !isSapi(persona))) {
      const persona = PERSONAS[name];
      if (!isRecord(persona)) continue;
      if (!inRange(persona.rate, 90, 360)) wrong.push(`${name}: rate ${JSON.stringify(persona.rate)}`);
      if (!inRange(persona.pbas, 0, 127)) wrong.push(`${name}: pbas ${JSON.stringify(persona.pbas)}`);
      if (!inRange(persona.pmod, 0, 127)) wrong.push(`${name}: pmod ${JSON.stringify(persona.pmod)}`);
      if ("gain" in persona && !inRange(persona.gain, 0, 2)) wrong.push(`${name}: gain ${JSON.stringify(persona.gain)}`);
    }
    expect(wrong).toEqual([]);
  });

  it("B33 keeps every referenced persona's web pitch within 0-2 and web rate within 0.1-10", () => {
    const wrong: string[] = [];
    for (const name of referencedPersonas()) {
      const persona = PERSONAS[name];
      if (!isRecord(persona)) continue;
      const web = persona.web;
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
    const core = Object.entries(CARDS).filter(([id]) => id.startsWith("core-"));
    const kinds = core.map(([, entry]) => entry.kind);
    expect(kinds.filter((kind) => kind === "unit"), "unit entries").toHaveLength(44);
    expect(kinds.filter((kind) => kind === "spell" || kind === "trap"), "spell and trap entries").toHaveLength(67);
    expect(allLines().filter(({ key }) => key.startsWith("core-")), "Core's lines in the table").toHaveLength(155);
  });

  it("B33 gives every card a line per its kind: twice the units plus the spells and traps", () => {
    const units = Object.values(CARDS).filter((entry) => entry.kind === "unit").length;
    expect(allLines(), "lines in the table").toHaveLength(units * 2 + (Object.keys(CARDS).length - units));
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

  it("B34 keeps every line within VOICE_MAX_WORDS for its kind (play 8, death 6, cast 8)", () => {
    expect(VOICE_MAX_WORDS, "the limits the Surface fixes").toEqual({ play: 8, death: 6, cast: 8 });
    // The counter itself: punctuation-only tokens are not words, hyphenated and elided ones are one.
    expect(wordCount("Too... slow...")).toBe(2);
    expect(wordCount("Oops - Surf's up!")).toBe(3);

    const long = allLines()
      .filter(({ line, text }) => typeof text === "string" && wordCount(text) > VOICE_MAX_WORDS[line])
      .map(({ key, line, text }) => `${key}: ${wordCount(String(text))} words > ${VOICE_MAX_WORDS[line]}`);
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
      "Flicker", "Plague Counter",
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
