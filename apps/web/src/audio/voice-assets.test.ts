// Polish task 2 (docs/polish/2-sound.md), behaviours B35, B36 and B37: the pre-rendered voice set
// on disk, its manifest, its size budget and `gen-voice.mjs --check`.
//
//   B35  the expected files (one `<defId>-<hook>` per hook in card-audio.json5 that names a voice,
//        which covers a play and a death line per catalog Unit and one cast line per anything else)
//        exist under apps/web/public/audio/voice/, each an MP4 with `ftyp` at byte 4 and brand
//        `M4A ` at byte 8; the manifest lists exactly those keys with each file's size and its
//        recomputed voiceHash; the directory holds nothing else.
//        Each file's MP4 header (moov/mvhd) also puts it within VOICE_FILE_MAX_MS.
//   B36  sum over the files of ceil(bytes / 4096) * 4096 <= VOICE_BUDGET_BYTES (6 MiB since R501).
//   B37  `node apps/web/scripts/gen-voice.mjs --check` exits 0 on the committed tree; with `--root`
//        on a temp copy whose core-004 play line was edited it exits 1 and prints a line starting
//        `core-004-play`.
//   R655 the move from voice-lines.json kept every line: the manifest's keys are exactly the file's
//        voice lines, each hashing as the manifest records; and a hook that is only an effect expects
//        no file.
//
// The Surface's `--check` contract (every expected key has a manifest entry whose hash matches and
// a file of that size, no orphan files or entries; otherwise one line per problem, each starting
// with the key, and exit 1) is what the further B37 cases below exercise, one broken thing at a
// time, each in its own copy of the tree.
//
// voiceHash = sha1(JSON.stringify({ v: 1, say, rate, pbas, pmod, text })).hex.slice(0, 16), keys in
// exactly that order, over the fields of the voice the hook names, for a voice rendered by macOS
// `say`; and, since R501, sha1(JSON.stringify({ v: 1, backend: "sapi", voice, rate, semitones,
// filter, text })) for one rendered by Windows SAPI and ffmpeg. Both are recomputed here from the
// formula, never imported, so the script and this test cannot share a mistake. The edits below
// parse a temp copy with JSON5 and write it back as JSON, which is JSON5 too.

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  appendFileSync,
  copyFileSync,
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import JSON5 from "json5";
import { afterAll, beforeAll, describe, expect, it } from "vitest";

import type { SetName } from "@jackioh/shared";
import { setShips } from "@jackioh/shared";

import { VOICE_BUDGET_BYTES, VOICE_FILE_MAX_MS } from "./constants.ts";

const here = dirname(fileURLToPath(import.meta.url));
const WEB = resolve(here, "../..");
const REPO = resolve(WEB, "../..");
const CATALOG_PATH = join(REPO, "crates/cards/catalog.json");
const AUDIO_PATH = join(here, "card-audio.json5");
const MANIFEST_PATH = join(here, "voice-manifest.json");
const VOICE_DIR = join(WEB, "public/audio/voice");
const GEN_VOICE = join(WEB, "scripts/gen-voice.mjs");

/** Where `--root` expects each file, relative to the web dir it is pointed at (Surface). */
const REL_AUDIO = join("src", "audio", "card-audio.json5");
const REL_MANIFEST = join("src", "audio", "voice-manifest.json");
const REL_VOICE_DIR = join("public", "audio", "voice");

const BLOCK = 4096;
const CHECK_TIMEOUT_MS = 60_000;

type Json = Record<string, unknown>;

function isRecord(value: unknown): value is Json {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function readJson(path: string): Json {
  const value = JSON.parse(readFileSync(path, "utf8")) as unknown;
  if (!isRecord(value)) throw new Error(`${path}: expected a JSON object`);
  return value;
}

function readJson5(path: string): Json {
  const value = JSON5.parse<unknown>(readFileSync(path, "utf8"));
  if (!isRecord(value)) throw new Error(`${path}: expected a JSON5 object`);
  return value;
}

function writeJson(path: string, value: unknown): void {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

// ------------------------------------------------------------------------------ expectations ---

const CATALOG = readJson(CATALOG_PATH) as Record<string, { type?: unknown; set?: unknown }>;

/** R1420: a card of a set that has not shipped renders no file until its set ships. */
function voicedNow(key: string): boolean {
  const set = CATALOG[splitKey(key).defId]?.set;
  return typeof set !== "string" || setShips(set as SetName);
}

/** SPEC §10.11's lines: a play and a death line per Unit (tokens included), one cast line per everything else. */
const REQUIRED_KEYS: readonly string[] = Object.entries(CATALOG)
  .flatMap(([id, card]) =>
    card.type === "Unit" ? [`${id}-play`, `${id}-death`] : [`${id}-cast`],
  )
  // R1420: a set that has not shipped owes no file
  .filter(voicedNow);

const AUDIO = readJson5(AUDIO_PATH);

function cardsOf(table: Json): Record<string, Json> {
  if (!isRecord(table.cards)) throw new Error("card-audio.json5: no cards section");
  return table.cards as Record<string, Json>;
}

function voicesOf(table: Json): Record<string, Json> {
  if (!isRecord(table.voices)) throw new Error("card-audio.json5: no voices section");
  return table.voices as Record<string, Json>;
}

/** The keys a table voices: `<defId>-<hook>` for every hook whose assignment names a voice. */
function voicedKeys(table: Json): string[] {
  return Object.entries(cardsOf(table)).flatMap(([defId, entry]) =>
    Object.entries(entry)
      .filter(([, assignment]) => isRecord(assignment) && assignment.voice !== undefined)
      .map(([hook]) => `${defId}-${hook}`),
  );
}

/** R644: the emotes section's five issue-§3 lines per portrait, keyed `emote-<portrait>-<line>`. */
const EMOTE_LINES = ["greetings", "wellPlayed", "oops", "thanks", "threaten"] as const;
const EXPECTED_EMOTE_KEYS: readonly string[] = Object.keys(
  isRecord(AUDIO.emotes) ? AUDIO.emotes : {},
).flatMap((portrait) => EMOTE_LINES.map((line) => `emote-${portrait}-${line}`));

/**
 * Every hook the committed file voices, plus every emote line; a hook that is only an effect
 * renders no file.
 */
const EXPECTED_KEYS: readonly string[] = [...voicedKeys(AUDIO).filter(voicedNow), ...EXPECTED_EMOTE_KEYS];
const EXPECTED_FILES: readonly string[] = EXPECTED_KEYS.map((key) => `${key}.m4a`);
/** Core's own count (44 units, 67 spells and traps) plus whatever the other sets bring. */
const EXPECTED_FILE_COUNT = EXPECTED_KEYS.length;
const CORE_FILE_COUNT = 155;

const MANIFEST = readJson(MANIFEST_PATH);
const MANIFEST_FILES: Json = isRecord(MANIFEST.files) ? MANIFEST.files : {};

/** "<defId>-<line>", parsed from the END because defIds contain "-" (types.ts). */
function splitKey(key: string): { defId: string; line: string } {
  const at = key.lastIndexOf("-");
  return { defId: key.slice(0, at), line: key.slice(at + 1) };
}

function voiceHash(input: { say: unknown; rate: unknown; pbas: unknown; pmod: unknown; text: unknown }): string {
  const { say, rate, pbas, pmod, text } = input;
  return createHash("sha1")
    .update(JSON.stringify({ v: 1, say, rate, pbas, pmod, text }))
    .digest("hex")
    .slice(0, 16);
}

/** R501: a SAPI voice's hash, over its own fields. */
function sapiHash(input: { voice: unknown; rate: unknown; semitones: unknown; filter: unknown; text: unknown }): string {
  const { voice, rate, semitones, filter, text } = input;
  return createHash("sha1")
    .update(JSON.stringify({ v: 1, backend: "sapi", voice, rate, semitones, filter, text }))
    .digest("hex")
    .slice(0, 16);
}

/** The hash `key` must carry, from a card-audio table (the committed one unless given): its voice's fields and its text. */
function expectedHash(key: string, table: Json = AUDIO): string | null {
  const { defId, line } = splitKey(key);
  const cards = isRecord(table.cards) ? table.cards : {};
  const emotes = isRecord(table.emotes) ? table.emotes : {};
  const voices = isRecord(table.voices) ? table.voices : {};
  if (defId.startsWith("emote-")) {
    // R644: `emote-<portrait>` keys resolve through the emotes section (voiceData.ts `emoteLineFor`).
    const entry = emotes[defId.slice("emote-".length)];
    if (!isRecord(entry)) return null;
    const personaName = entry.persona;
    if (typeof personaName !== "string") return null;
    const voice = voices[personaName];
    if (!isRecord(voice)) return null;
    const text = entry[line];
    if (typeof text !== "string") return null;
    if (voice.backend === "sapi") {
      return sapiHash({ voice: voice.voice, rate: voice.rate, semitones: voice.semitones, filter: voice.filter, text });
    }
    return voiceHash({
      say: voice.say,
      rate: entry.rate ?? voice.rate,
      pbas: entry.pbas ?? voice.pbas,
      pmod: entry.pmod ?? voice.pmod,
      text,
    });
  }
  const entry = cards[defId];
  if (!isRecord(entry)) return null;
  const assignment = entry[line];
  if (!isRecord(assignment) || typeof assignment.voice !== "string") return null;
  const voice = voices[assignment.voice];
  if (!isRecord(voice)) return null;
  const text = assignment.text;
  if (typeof text !== "string") return null;
  if (voice.backend === "sapi") {
    return sapiHash({ voice: voice.voice, rate: voice.rate, semitones: voice.semitones, filter: voice.filter, text });
  }
  return voiceHash({ say: voice.say, rate: voice.rate, pbas: voice.pbas, pmod: voice.pmod, text });
}

function voicePath(key: string, voiceDir = VOICE_DIR): string {
  return join(voiceDir, `${key}.m4a`);
}

function sizeOnDisk(key: string): number | null {
  const path = voicePath(key);
  return existsSync(path) ? statSync(path).size : null;
}

/** The box `type` among the ISO BMFF boxes in [start, end), or null; handles 64-bit and to-end sizes. */
function findBox(buf: Buffer, type: string, start: number, end: number): { body: number; end: number } | null {
  let at = start;
  while (at + 8 <= end) {
    let size = buf.readUInt32BE(at);
    let header = 8;
    if (size === 1) {
      size = Number(buf.readBigUInt64BE(at + 8));
      header = 16;
    } else if (size === 0) size = end - at;
    if (size < header) return null;
    if (buf.toString("latin1", at + 4, at + 8) === type) return { body: at + header, end: at + size };
    at += size;
  }
  return null;
}

/** moov/mvhd duration / timescale. afconvert writes no edit list, so this counts the AAC priming and
 *  padding frames too: about 0.13 s over `afinfo`'s estimate, and never under what a decoder yields. */
function movieSeconds(buf: Buffer): number | null {
  const moov = findBox(buf, "moov", 0, buf.length);
  const mvhd = moov && findBox(buf, "mvhd", moov.body, moov.end);
  if (!mvhd) return null;
  const v1 = buf[mvhd.body] === 1;
  const timescale = buf.readUInt32BE(mvhd.body + (v1 ? 20 : 12));
  const duration = v1 ? Number(buf.readBigUInt64BE(mvhd.body + 24)) : buf.readUInt32BE(mvhd.body + 16);
  return timescale > 0 ? duration / timescale : null;
}

// -------------------------------------------------------------------------------- the script ---

type CheckRun = { status: number | null; output: string; lines: string[]; error: string };

function runCheck(extraArgs: readonly string[] = []): CheckRun {
  const result = spawnSync(process.execPath, [GEN_VOICE, "--check", ...extraArgs], {
    cwd: REPO,
    encoding: "utf8",
    timeout: CHECK_TIMEOUT_MS - 5_000,
  });
  const output = `${result.stdout ?? ""}${result.stderr ?? ""}`;
  return {
    status: result.status,
    output,
    lines: output.split(/\r?\n/).filter((line) => line.length > 0),
    error: result.error === undefined ? "" : String(result.error),
  };
}

/** A problem line starts with the key it is about (Surface: "each starting with the key"). */
const KEY_AT_START = /^((?:core|classic|classicplus|meditative)-[a-z0-9]+(?:-[a-z0-9]+)*-(?:play|attack|death|cast))(?![a-z0-9])/;

/** The keys the run reported a problem for, sorted and deduplicated. */
function reportedKeys(run: CheckRun): string[] {
  const keys = new Set<string>();
  for (const line of run.lines) {
    const key = KEY_AT_START.exec(line)?.[1];
    if (key !== undefined) keys.add(key);
  }
  return [...keys].sort();
}

function describeRun(run: CheckRun): string {
  return `exit ${String(run.status)}${run.error === "" ? "" : ` (${run.error})`}; output:\n${run.output}`;
}

let scratch = "";
let copies = 0;

/** A fresh copy of everything `--root` reads, under the suite's temp dir. */
function copyWebTree(): string {
  copies += 1;
  const root = join(scratch, `web-${String(copies)}`);
  mkdirSync(join(root, "src", "audio"), { recursive: true });
  mkdirSync(join(root, "public", "audio"), { recursive: true });
  copyFileSync(AUDIO_PATH, join(root, REL_AUDIO));
  copyFileSync(MANIFEST_PATH, join(root, REL_MANIFEST));
  cpSync(VOICE_DIR, join(root, REL_VOICE_DIR), { recursive: true });
  return root;
}

/** Edits a copy's card-audio.json5, written back as JSON (which is JSON5 too). */
function editAudio(root: string, edit: (table: Json) => void): void {
  const path = join(root, REL_AUDIO);
  const table = readJson5(path);
  edit(table);
  writeJson(path, table);
}

function editManifest(root: string, edit: (files: Json) => void): void {
  const path = join(root, REL_MANIFEST);
  const manifest = readJson(path);
  const files = isRecord(manifest.files) ? manifest.files : {};
  edit(files);
  manifest.files = files;
  writeJson(path, manifest);
}

/** Expect `--check --root root` to exit 1 reporting exactly `keys`. */
function expectReported(root: string, keys: readonly string[]): void {
  const run = runCheck(["--root", root]);
  expect(run.status, describeRun(run)).toBe(1);
  for (const key of keys) {
    expect(
      run.lines.some((line) => line.startsWith(key)),
      `a line starting "${key}"; ${describeRun(run)}`,
    ).toBe(true);
  }
  expect(reportedKeys(run), `only the broken keys are reported; ${describeRun(run)}`).toEqual([...keys].sort());
}

// ------------------------------------------------------------------------------------ B35 ---

describe("the committed voice files (B35)", () => {
  it("B35 expects a play and a death line per unit and one cast line per spell and trap, Core's 155 among them", () => {
    expect(new Set(EXPECTED_KEYS).size, "no key twice").toBe(EXPECTED_FILE_COUNT);
    const voiced = new Set(EXPECTED_KEYS);
    expect(REQUIRED_KEYS.filter((key) => !voiced.has(key)), "lines SPEC §10.11 requires that no hook voices").toEqual([]);
    expect(EXPECTED_KEYS.filter((key) => key.startsWith("core-")), "Core's own lines").toHaveLength(CORE_FILE_COUNT);
  });

  it("R644 renders all five voice emotes of every portrait, on disk and in the manifest", () => {
    expect(EXPECTED_EMOTE_KEYS).toHaveLength(30);
    const missing = EXPECTED_EMOTE_KEYS.filter(
      (key) => !existsSync(voicePath(key)) || MANIFEST_FILES[key] === undefined,
    );
    expect(missing, "emote keys with no file or no manifest entry").toEqual([]);
  });

  it("B35 leaves no expected voice file missing from public/audio/voice", () => {
    const missing = EXPECTED_KEYS.filter((key) => !existsSync(voicePath(key)));
    expect(missing, `keys with no file under ${VOICE_DIR}`).toEqual([]);
  });

  it("B35 gives every voice file an MP4 header: ftyp at byte 4 and brand 'M4A ' at byte 8", () => {
    const wrong: string[] = [];
    for (const key of EXPECTED_KEYS) {
      const path = voicePath(key);
      if (!existsSync(path)) {
        wrong.push(`${key}: missing`);
        continue;
      }
      const head = readFileSync(path).subarray(0, 12);
      const box = head.subarray(4, 8).toString("latin1");
      const brand = head.subarray(8, 12).toString("latin1");
      if (box !== "ftyp" || brand !== "M4A ") {
        wrong.push(`${key}: box ${JSON.stringify(box)}, brand ${JSON.stringify(brand)}`);
      }
    }
    expect(wrong).toEqual([]);
  });

  it("B35 keeps every voice file within VOICE_FILE_MAX_MS by its MP4 header, priming frames included", () => {
    const wrong: string[] = [];
    for (const key of EXPECTED_KEYS) {
      const path = voicePath(key);
      if (!existsSync(path)) {
        wrong.push(`${key}: missing`);
        continue;
      }
      const seconds = movieSeconds(readFileSync(path));
      if (seconds === null) wrong.push(`${key}: no moov/mvhd box`);
      else if (seconds * 1000 > VOICE_FILE_MAX_MS) wrong.push(`${key}: ${seconds.toFixed(2)} s`);
    }
    expect(wrong, `files longer than ${VOICE_FILE_MAX_MS} ms`).toEqual([]);
  });

  it("B35 has a version 1 manifest with an entry for every expected key", () => {
    expect(MANIFEST.version, "voice-manifest.json version").toBe(1);
    expect(isRecord(MANIFEST.files), "voice-manifest.json files is an object").toBe(true);
    const missing = EXPECTED_KEYS.filter((key) => !isRecord(MANIFEST_FILES[key]));
    expect(missing, "expected keys the manifest does not list").toEqual([]);
  });

  it("B35 has no manifest entry for a key outside the expected set", () => {
    const expected = new Set(EXPECTED_KEYS);
    const extra = Object.keys(MANIFEST_FILES).filter((key) => !expected.has(key));
    expect(extra, "manifest entries no card's hook voices").toEqual([]);
  });

  it("B35 records each file's size on disk as its manifest bytes", () => {
    const wrong: string[] = [];
    for (const key of EXPECTED_KEYS) {
      const entry = MANIFEST_FILES[key];
      const size = sizeOnDisk(key);
      const bytes = isRecord(entry) ? entry.bytes : undefined;
      if (size === null || bytes !== size) {
        wrong.push(`${key}: manifest bytes ${JSON.stringify(bytes)}, on disk ${String(size)}`);
      }
    }
    expect(wrong).toEqual([]);
  });

  it("B35 records the voiceHash of each line's voice and text", () => {
    const wrong: string[] = [];
    for (const key of EXPECTED_KEYS) {
      const entry = MANIFEST_FILES[key];
      const hash = isRecord(entry) ? entry.hash : undefined;
      const recomputed = expectedHash(key);
      if (recomputed === null) {
        wrong.push(`${key}: card-audio.json5 has no text or voice for it`);
      } else if (hash !== recomputed) {
        wrong.push(`${key}: manifest ${JSON.stringify(hash)}, recomputed ${recomputed}`);
      }
    }
    expect(wrong).toEqual([]);
  });

  it("B35 keeps nothing in the voice directory but the expected files", () => {
    expect(existsSync(VOICE_DIR), `${VOICE_DIR} exists`).toBe(true);
    const expected = new Set(EXPECTED_FILES);
    const present = readdirSync(VOICE_DIR);
    const orphans = present.filter((name) => !expected.has(name)).sort();
    expect(orphans, `entries in ${VOICE_DIR} that are not an expected voice file`).toEqual([]);
    expect(present, "exactly the expected files").toHaveLength(EXPECTED_FILE_COUNT);
  });
});

// ------------------------------------------------------------------------------------ B36 ---

describe("the voice budget (B36)", () => {
  it("B36 R501 fits the whole set into VOICE_BUDGET_BYTES (6 MiB), counted in 4 KiB blocks", () => {
    expect(VOICE_BUDGET_BYTES, "the budget R501 fixes").toBe(6 * 1024 * 1024);
    const sizes = EXPECTED_KEYS.map((key) => sizeOnDisk(key));
    expect(sizes.filter((size) => size === null), "every expected file exists to be measured").toEqual([]);
    const onDisk = sizes.reduce<number>((sum, size) => sum + Math.ceil((size ?? 0) / BLOCK) * BLOCK, 0);
    expect(onDisk, "sum of ceil(bytes / 4096) * 4096").toBeLessThanOrEqual(VOICE_BUDGET_BYTES);
  });
});

// ----------------------------------------------------------------------------------- R655 ---

describe("the move to card-audio.json5 (R655)", () => {
  it("R655 carries every voice line over: the manifest's keys are the file's voice lines, each hashing as the manifest records", () => {
    const manifestKeys = Object.keys(MANIFEST_FILES).sort();
    expect(manifestKeys.length, "the manifest lists voice lines").toBeGreaterThan(0);
    expect([...EXPECTED_KEYS].sort(), "the file's voice lines are the manifest's keys").toEqual(manifestKeys);
    const wrong = manifestKeys
      .map((key) => {
        const entry = MANIFEST_FILES[key];
        return { key, hash: isRecord(entry) ? entry.hash : undefined, recomputed: expectedHash(key) };
      })
      .filter(({ hash, recomputed }) => recomputed === null || hash !== recomputed)
      .map(({ key, hash, recomputed }) => `${key}: manifest ${JSON.stringify(hash)}, recomputed ${String(recomputed)}`);
    expect(wrong, "lines whose voice or text changed in the move").toEqual([]);
  });

  it("R655 expects no file for a hook that is only an effect, core-066's attack among them", () => {
    expect(cardsOf(AUDIO)["core-066"]?.attack, "core-066's attack hook").toEqual({ effect: "rumble" });
    const effectOnly = Object.entries(cardsOf(AUDIO)).flatMap(([defId, entry]) =>
      Object.entries(entry)
        .filter(([, assignment]) => isRecord(assignment) && assignment.voice === undefined)
        .map(([hook]) => `${defId}-${hook}`),
    );
    expect(effectOnly, "the file has effect-only hooks").toContain("core-066-attack");
    const voiced = effectOnly.filter((key) => key in MANIFEST_FILES || existsSync(voicePath(key)));
    expect(voiced, "effect-only hooks with a manifest entry or a file").toEqual([]);
  });
});

// ------------------------------------------------------------------------------------ B37 ---

describe("gen-voice.mjs --check (B37)", () => {
  beforeAll(() => {
    scratch = mkdtempSync(join(tmpdir(), "jackioh-gen-voice-"));
  });

  afterAll(() => {
    if (scratch !== "") rmSync(scratch, { recursive: true, force: true });
  });

  it(
    "B37 exits 0 on the committed tree and reports every expected file and their bytes",
    () => {
      const run = runCheck();
      expect(run.status, describeRun(run)).toBe(0);
      const ok = /gen-voice: ok, (\d+) files, (\d+) bytes/.exec(run.output);
      expect(ok, `the "gen-voice: ok, <n> files, <bytes> bytes" line; ${describeRun(run)}`).not.toBeNull();
      expect(Number(ok?.[1]), "files counted").toBe(EXPECTED_FILE_COUNT);
      const total = EXPECTED_KEYS.reduce((sum, key) => sum + (sizeOnDisk(key) ?? 0), 0);
      expect(Number(ok?.[2]), "bytes counted").toBe(total);
      expect(reportedKeys(run), "no key reported as a problem").toEqual([]);
    },
    CHECK_TIMEOUT_MS,
  );

  it(
    "B37 exits 0 with --root on an unedited copy of the tree",
    () => {
      const root = copyWebTree();
      const run = runCheck(["--root", root]);
      expect(run.status, describeRun(run)).toBe(0);
      expect(reportedKeys(run)).toEqual([]);
    },
    CHECK_TIMEOUT_MS,
  );

  it(
    "B37 exits 1 and prints a line starting core-004-play when that line was edited",
    () => {
      const root = copyWebTree();
      editAudio(root, (table) => {
        const play = cardsOf(table)["core-004"]?.play;
        if (!isRecord(play)) throw new Error("card-audio.json5: no core-004 play hook to edit");
        play.text = play.text === "Double or nothing, pal!" ? "Double or nothing, baby!" : "Double or nothing, pal!";
      });
      expectReported(root, ["core-004-play"]);
    },
    CHECK_TIMEOUT_MS,
  );

  it(
    "B37 exits 1 naming the key when an expected voice file is missing",
    () => {
      const root = copyWebTree();
      unlinkSync(voicePath("core-004-death", join(root, REL_VOICE_DIR)));
      expectReported(root, ["core-004-death"]);
    },
    CHECK_TIMEOUT_MS,
  );

  it(
    "B37 exits 1 naming the key when a file's size differs from its manifest bytes",
    () => {
      const root = copyWebTree();
      appendFileSync(voicePath("core-004-death", join(root, REL_VOICE_DIR)), Buffer.from([0]));
      expectReported(root, ["core-004-death"]);
    },
    CHECK_TIMEOUT_MS,
  );

  it(
    "B37 exits 1 naming the key when an orphan voice file sits in the directory",
    () => {
      const root = copyWebTree();
      const voiceDir = join(root, REL_VOICE_DIR);
      // core-004 is a unit, so it has no cast hook: a well-formed key no card voices.
      copyFileSync(voicePath("core-004-play", voiceDir), voicePath("core-004-cast", voiceDir));
      expectReported(root, ["core-004-cast"]);
    },
    CHECK_TIMEOUT_MS,
  );

  it(
    "B37 exits 1 naming the key when the manifest carries an orphan entry",
    () => {
      const root = copyWebTree();
      editManifest(root, (files) => {
        files["core-004-cast"] = { hash: "0123456789abcdef", bytes: 1024 };
      });
      expectReported(root, ["core-004-cast"]);
    },
    CHECK_TIMEOUT_MS,
  );

  it(
    "B37 exits 1 naming the key when an expected key has no manifest entry",
    () => {
      const root = copyWebTree();
      editManifest(root, (files) => {
        delete files["core-004-death"];
      });
      expectReported(root, ["core-004-death"]);
    },
    CHECK_TIMEOUT_MS,
  );

  it(
    "B37 exits 1 naming every key that speaks in a voice whose rate changed",
    () => {
      const root = copyWebTree();
      const committedCards = cardsOf(AUDIO);
      const play = committedCards["core-008"]?.play;
      const voice = isRecord(play) ? play.voice : undefined;
      expect(typeof voice, "core-008's play line names a voice").toBe("string");
      // Every line in that voice goes stale, whichever card speaks it.
      const stale = EXPECTED_KEYS.filter((key) => {
        const { defId, line } = splitKey(key);
        const assignment = committedCards[defId]?.[line];
        return isRecord(assignment) && assignment.voice === voice;
      });
      expect(stale, "the voice speaks at least core-008's two lines").toEqual(
        expect.arrayContaining(["core-008-play", "core-008-death"]),
      );
      editAudio(root, (table) => {
        const target = voicesOf(table)[String(voice)];
        const rate = isRecord(target) ? target.rate : undefined;
        if (!isRecord(target) || typeof rate !== "number") throw new Error("voice has no rate");
        target.rate = rate >= 350 ? rate - 10 : rate + 10;
      });
      expectReported(root, stale);
    },
    CHECK_TIMEOUT_MS,
  );

  /**
   * R501: a line voiced by a Windows SAPI voice, on a card a given catalog holds. The card is
   * invented (`classicplus-999`), so no real catalog ever has it, and its file is a copy of a real one.
   */
  function sapiTree(): { root: string; catalog: string } {
    const root = copyWebTree();
    const voice = { backend: "sapi", voice: "Microsoft Zira Desktop", rate: 5, semitones: -2, filter: "lowpass=f=3500", web: { pitch: 1, rate: 1 } };
    const text = "A voice from another machine.";
    editAudio(root, (table) => {
      voicesOf(table)["test-sapi"] = voice;
      cardsOf(table)["classicplus-999"] = { cast: { voice: "test-sapi", text } };
    });
    const voiceDir = join(root, REL_VOICE_DIR);
    copyFileSync(voicePath("core-005-cast", voiceDir), voicePath("classicplus-999-cast", voiceDir));
    const bytes = statSync(voicePath("classicplus-999-cast", voiceDir)).size;
    const hash = sapiHash({ voice: voice.voice, rate: voice.rate, semitones: voice.semitones, filter: voice.filter, text });
    editManifest(root, (files) => {
      files["classicplus-999-cast"] = { hash, bytes };
    });
    const catalog = join(root, "catalog.json");
    writeJson(catalog, { ...CATALOG, "classicplus-999": { type: "Spell" } });
    return { root, catalog };
  }

  /**
   * R1420: a voiced card of `set` that the catalog --catalog names holds and no file renders, with
   * the committed voices and an invented id (`meditative-999`), so no real catalog has it.
   */
  function unrenderedTree(set: string): { root: string; catalog: string } {
    const root = copyWebTree();
    editAudio(root, (table) => {
      cardsOf(table)["meditative-999"] = { cast: { voice: "narrator", text: "Breathe in. Breathe out." } };
    });
    const catalog = join(root, "catalog.json");
    writeJson(catalog, { ...CATALOG, "meditative-999": { type: "Spell", set } });
    return { root, catalog };
  }

  it(
    "R1420 owes no file for a line of a set that has not shipped, and the same line on a shipped card is missing",
    () => {
      const unshipped = unrenderedTree("Meditative");
      const quiet = runCheck(["--root", unshipped.root, "--catalog", unshipped.catalog]);
      expect(quiet.status, describeRun(quiet)).toBe(0);
      expect(reportedKeys(quiet)).toEqual([]);

      const shipped = unrenderedTree("Core");
      const loud = runCheck(["--root", shipped.root, "--catalog", shipped.catalog]);
      expect(loud.status, describeRun(loud)).toBe(1);
      expect(reportedKeys(loud)).toContain("meditative-999-cast");
    },
    CHECK_TIMEOUT_MS,
  );

  it(
    "R501 accepts a line voiced by a SAPI voice, hashed over its own fields, against the catalog --catalog names",
    () => {
      const { root, catalog } = sapiTree();
      const run = runCheck(["--root", root, "--catalog", catalog]);
      expect(run.status, describeRun(run)).toBe(0);
      expect(reportedKeys(run)).toEqual([]);
    },
    CHECK_TIMEOUT_MS,
  );

  it(
    "R501 reports a SAPI line whose pitch shift changed, and a card the default catalog does not hold",
    () => {
      const { root, catalog } = sapiTree();
      editAudio(root, (table) => {
        const voice = voicesOf(table)["test-sapi"];
        if (!isRecord(voice)) throw new Error("no test-sapi voice");
        voice.semitones = 3;
      });
      expectReported(root, ["classicplus-999-cast"]);
      const withCatalog = runCheck(["--root", root, "--catalog", catalog]);
      expect(withCatalog.status, describeRun(withCatalog)).toBe(1);
      expect(reportedKeys(withCatalog)).toEqual(["classicplus-999-cast"]);
      const withoutCatalog = runCheck(["--root", root]);
      expect(
        withoutCatalog.lines.some((line) => line.startsWith("classicplus-999: not in the catalog")),
        describeRun(withoutCatalog),
      ).toBe(true);
    },
    CHECK_TIMEOUT_MS,
  );

  it(
    "R501 refuses a SAPI voice whose pitch shift is out of range, and the old per-card override on its card",
    () => {
      const { root, catalog } = sapiTree();
      editAudio(root, (table) => {
        const voice = voicesOf(table)["test-sapi"];
        if (!isRecord(voice)) throw new Error("no test-sapi voice");
        voice.semitones = 13;
      });
      const outOfRange = runCheck(["--root", root, "--catalog", catalog]);
      expect(outOfRange.status, describeRun(outOfRange)).toBe(1);
      expect(outOfRange.lines.some((line) => line.startsWith("voice test-sapi: semitones 13")), describeRun(outOfRange)).toBe(true);
      expect(
        outOfRange.lines.some((line) => line.startsWith("classicplus-999-cast: voice test-sapi cannot render")),
        describeRun(outOfRange),
      ).toBe(true);

      // voice-lines.json's override, a number on the card beside its lines, is no hook: it is refused, never ignored.
      const second = sapiTree();
      editAudio(second.root, (table) => {
        const entry = cardsOf(table)["classicplus-999"];
        if (!isRecord(entry)) throw new Error("no classicplus-999 entry");
        entry.rate = 200;
      });
      const override = runCheck(["--root", second.root, "--catalog", second.catalog]);
      expect(override.status, describeRun(override)).toBe(1);
      expect(override.lines.some((line) => line.startsWith("classicplus-999-rate: ")), describeRun(override)).toBe(true);
    },
    CHECK_TIMEOUT_MS,
  );

  it(
    "B37 exits 1 naming a card's keys when it moves to a voice of its own (hashes use the named voice's values)",
    () => {
      const root = copyWebTree();
      editAudio(root, (table) => {
        const entry = cardsOf(table)["core-004"];
        const play = entry?.play;
        const death = entry?.death;
        if (!isRecord(play) || !isRecord(death)) throw new Error("no core-004 play and death lines");
        const voices = voicesOf(table);
        const base = voices[String(play.voice)];
        const rate = isRecord(base) ? base.rate : undefined;
        if (!isRecord(base) || typeof rate !== "number") throw new Error("core-004's voice has no rate");
        // The way the three old overrides became voices: a copy of the card's voice, at another rate.
        voices["core-004-own"] = { ...base, rate: rate >= 350 ? rate - 5 : rate + 5 };
        play.voice = "core-004-own";
        death.voice = "core-004-own";
      });
      expectReported(root, ["core-004-death", "core-004-play"]);
    },
    CHECK_TIMEOUT_MS,
  );

  it(
    "R655 exits 0 when a card gains an attack hook that is only an effect, which renders no file",
    () => {
      const root = copyWebTree();
      editAudio(root, (table) => {
        const entry = cardsOf(table)["core-004"];
        if (!isRecord(entry)) throw new Error("no core-004 entry");
        expect(entry.attack, "core-004 has no attack hook yet").toBeUndefined();
        entry.attack = { effect: "zip" };
      });
      const run = runCheck(["--root", root]);
      expect(run.status, describeRun(run)).toBe(0);
      expect(reportedKeys(run)).toEqual([]);
      expect(existsSync(voicePath("core-004-attack", join(root, REL_VOICE_DIR)))).toBe(false);
    },
    CHECK_TIMEOUT_MS,
  );
});
