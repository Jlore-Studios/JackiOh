#!/usr/bin/env node
// Renders every voice line in src/audio/card-audio.json5 to public/audio/voice/<key>.m4a and records
// each file in src/audio/voice-manifest.json (docs/polish/2-sound.md, "gen-voice.mjs"; SPEC §10.11,
// R501, R655).
//
// The file is JSON5 and only ever read. A voice line is a card hook's assignment that names a voice
// from the voices bank (with its text); its key is `<defId>-<hook>`. An assignment that is only an
// effect renders nothing. Which hooks a card may carry is voiceData.ts's to check (CARD_HOOKS); this
// script holds every catalog card to SPEC §10.11's lines: a Unit's play and death, anything else's cast.
//
//   node apps/web/scripts/gen-voice.mjs [--check] [--force] [--only <defId>] [--root <webDir>] [--catalog <file>]
//   pnpm --filter @jackioh/web gen:voice
//
// TWO BACKENDS (R501). Each voice names the synthesizer its lines render with:
//   - "say" (the default when a voice names none): macOS `say`, encoded by `afconvert`. Core's
//     lines were rendered this way, and their voiceHash is exactly the original formula, so adding the
//     second backend left every one of those files current.
//   - "sapi": Windows SAPI (`System.Speech`) driven through `powershell.exe`, natively or from WSL,
//     where the WAVs go through a Windows-visible temp dir; then trimmed, pitch-shifted, coloured by
//     the voice's own ffmpeg filter chain, peak-normalised and encoded by `ffmpeg` to the same mono
//     AAC at 22050 Hz and about 32 kbps. One PowerShell process renders a whole batch, because each
//     start costs about a second.
// Generate renders the keys whose backend this machine has. A key that needs rendering by a backend
// this machine lacks is reported as a problem naming that backend, and its file is left alone.
//
// It is idempotent by input, never by output bytes: legacy `say` voices such as Fred are not
// byte-deterministic from run to run, so a key is rendered again only when its voiceHash (the
// voice's backend fields and the text) differs from the manifest's, or its file is missing or has
// the wrong size. `--force` renders every key again and `--only <defId>` limits rendering to one card.
// Orphan files and manifest entries are always removed, and the manifest is rewritten only when its
// content changes, so a second run with no input change writes nothing.
//
// `--check` runs on any OS and touches nothing. It is what CI runs (B37): one line per problem, each
// starting with the key it concerns, and exit 1; or `gen-voice: ok, <n> files, <bytes> bytes` and exit 0.
// `--catalog <file>` checks the lines against another catalog.json than packages/cards/catalog.json.
//
// Exit codes: 0 ok, 1 a problem was found, 2 bad arguments or no synthesizer at all on this machine.
import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import JSON5 from "json5";

/** Bump when the encoding flags below change, so every file renders again. */
const HASH_VERSION = 1;
const HASH_CHARS = 16;
const FORMAT = "m4af aac@22050 mono 32000";
const AFCONVERT_FLAGS = ["-f", "m4af", "-d", "aac@22050", "-c", "1", "-b", "32000"];
/** The same format from ffmpeg: mono AAC-LC at 22050 Hz and 32 kbps in an M4A (`ipod`) container. */
const SAMPLE_RATE = 22050;
const FFMPEG_ENCODE = [
  "-ac", "1", "-ar", String(SAMPLE_RATE), "-c:a", "aac", "-b:a", "32k",
  "-map_metadata", "-1", "-fflags", "+bitexact", "-flags:a", "+bitexact", "-f", "ipod",
];
/** Silence below this level is trimmed from both ends of a SAPI render. */
const TRIM_THRESHOLD_DB = -50;
/** A SAPI render's loudest sample after normalising, in dBFS (Core's `say` files peak near -5). */
const PEAK_TARGET_DB = -4;
/** VOICE_BUDGET_BYTES in src/audio/constants.ts: the whole set, counted in whole disk blocks (R501). */
const BUDGET_BYTES = 6 * 1024 * 1024;
const BLOCK_BYTES = 4096;
/** The mode every rendered file is written with. */
const FILE_MODE = 0o644;
/** VOICE_FILE_MAX_MS in src/audio/constants.ts, measured by `afinfo` or `ffprobe` (voice-assets.test.ts reads the MP4 header). */
const MAX_SECONDS = 4.0;
/** Concurrent render jobs; the machine is shared, so two are plenty. */
const JOBS = 2;
/** Lines one PowerShell process renders before the next starts. */
const SAPI_BATCH = 60;
/** SPEC §10.11: the lines every card of a kind has. */
const LINES_BY_KIND = { unit: ["play", "death"], spell: ["cast"], trap: ["cast"] };
const KIND_OF_TYPE = { Unit: "unit", Spell: "spell", "Field Spell": "spell", Trap: "trap", "Field Trap": "trap" };
// R644's hero-portrait emotes (issue §3): the `emotes` section of card-audio.json5, one entry per
// portrait id, each with the five voice lines below. Files are keyed `emote-<portrait>-<line>`.
const EMOTE_PORTRAITS = ["vanilla", "gary", "timmy", "dfender", "felinors", "shredder"];
const EMOTE_LINES = ["greetings", "wellPlayed", "oops", "thanks", "threaten"];
const EMOTE_PREFIX = "emote-";
const BACKENDS = ["say", "sapi"];
/** SAPI voice ranges: SSML prosody rate in percent, the ffmpeg pitch shift in semitones. */
const SAPI_RATE_RANGE = [-50, 100];
const SAPI_SEMITONE_RANGE = [-12, 12];
/** What a voice's ffmpeg filter chain may contain: filter names, numbers, `=`, `:`, `,`, `.`, `|` and `-`. */
const FILTER_CHARSET = /^[a-z0-9_=:,.|-]*$/;
const USAGE =
  "usage: node scripts/gen-voice.mjs [--check] [--force] [--only <defId>] [--root <webDir>] [--catalog <file>]";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
/** Found from the script, not from `--root`, so a temp copy of the web dir is still checked against it. */
const CATALOG = path.resolve(SCRIPT_DIR, "../../../packages/cards/catalog.json");
const POWERSHELL_WSL = "/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe";

function usage(message) {
  console.error(`gen-voice: ${message}`);
  console.error(USAGE);
  process.exit(2);
}

function parseArgs(argv) {
  const opts = { check: false, force: false, only: null, root: path.resolve(SCRIPT_DIR, ".."), catalog: CATALOG };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--check") opts.check = true;
    else if (arg === "--force") opts.force = true;
    else if (arg === "--only" || arg === "--root" || arg === "--catalog") {
      const value = argv[++i];
      if (value === undefined || value.startsWith("--")) usage(`${arg} needs a value`);
      if (arg === "--only") opts.only = value;
      else if (arg === "--root") opts.root = path.resolve(value);
      else opts.catalog = path.resolve(value);
    } else usage(`unknown argument ${arg}`);
  }
  return opts;
}

function sha(input) {
  return createHash("sha1").update(JSON.stringify(input)).digest("hex").slice(0, HASH_CHARS);
}

/** `say`: sha1(JSON.stringify({ v, say, rate, pbas, pmod, text })) in exactly that key order, first 16 hex chars. */
function voiceHash({ say, rate, pbas, pmod, text }) {
  return sha({ v: HASH_VERSION, say, rate, pbas, pmod, text });
}

/** `sapi` (R501): sha1(JSON.stringify({ v, backend, voice, rate, semitones, filter, text })), first 16 hex chars. */
function sapiHash({ voice, rate, semitones, filter, text }) {
  return sha({ v: HASH_VERSION, backend: "sapi", voice, rate, semitones, filter, text });
}

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, "utf8"));
}

function readText(file) {
  try {
    return fs.readFileSync(file, "utf8");
  } catch {
    return null;
  }
}

function fileSize(file) {
  try {
    const stat = fs.statSync(file);
    return stat.isFile() ? stat.size : null;
  } catch {
    return null;
  }
}

function onDisk(bytes) {
  return Math.ceil(bytes / BLOCK_BYTES) * BLOCK_BYTES;
}

function backendOf(voice) {
  return voice?.backend ?? "say";
}

function inRange(value, [min, max]) {
  return typeof value === "number" && Number.isFinite(value) && value >= min && value <= max;
}

/** A voice's problems, or [] when it can render. */
function voiceProblems(name, voice) {
  const backend = backendOf(voice);
  if (!BACKENDS.includes(backend)) return [`voice ${name}: unknown backend ${JSON.stringify(backend)}`];
  if (backend === "say") {
    return typeof voice.say === "string" && voice.say !== "" ? [] : [`voice ${name}: no say voice`];
  }
  const problems = [];
  if (typeof voice.voice !== "string" || voice.voice === "") problems.push(`voice ${name}: no SAPI voice`);
  if (!inRange(voice.rate, SAPI_RATE_RANGE)) problems.push(`voice ${name}: SAPI rate ${JSON.stringify(voice.rate)}`);
  if (!inRange(voice.semitones, SAPI_SEMITONE_RANGE)) {
    problems.push(`voice ${name}: semitones ${JSON.stringify(voice.semitones)}`);
  }
  if (typeof voice.filter !== "string" || !FILTER_CHARSET.test(voice.filter)) {
    problems.push(`voice ${name}: filter ${JSON.stringify(voice.filter)}`);
  }
  return problems;
}

function isObject(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Every key the file expects (each hook assignment that names a voice), with its backend, values and hash. */
function expectedKeys(table, problems) {
  const expected = new Map();
  const voices = isObject(table.voices) ? table.voices : {};
  const badVoices = new Set();
  for (const [name, voice] of Object.entries(voices)) {
    const found = voiceProblems(name, voice);
    if (found.length > 0) badVoices.add(name);
    problems.push(...found);
  }
  for (const [defId, entry] of Object.entries(isObject(table.cards) ? table.cards : {})) {
    if (!isObject(entry)) {
      problems.push(`${defId}: not an object`);
      continue;
    }
    for (const [hook, assignment] of Object.entries(entry)) {
      const key = `${defId}-${hook}`;
      if (!isObject(assignment)) {
        problems.push(`${key}: not an object`);
        continue;
      }
      if (assignment.voice === undefined) continue;
      const voice = voices[assignment.voice];
      if (!isObject(voice)) {
        problems.push(`${key}: unknown voice ${JSON.stringify(assignment.voice)}`);
        continue;
      }
      if (badVoices.has(assignment.voice)) {
        problems.push(`${key}: voice ${assignment.voice} cannot render`);
        continue;
      }
      const text = assignment.text;
      if (typeof text !== "string" || text.trim() === "") {
        problems.push(`${key}: no text for its voice`);
        continue;
      }
      const backend = backendOf(voice);
      if (backend === "say") {
        const values = { say: voice.say, rate: voice.rate, pbas: voice.pbas, pmod: voice.pmod, text };
        expected.set(key, { defId, line: hook, backend, ...values, hash: voiceHash(values) });
      } else {
        const values = { voice: voice.voice, rate: voice.rate, semitones: voice.semitones, filter: voice.filter, text };
        expected.set(key, { defId, line: hook, backend, ...values, hash: sapiHash(values) });
      }
    }
  }
  // R644: one expected `emote-<portrait>-<line>` key per line, resolved through the same voice
  // machinery as a card's lines; the caller names the defId the file is written under.
  const expectEmote = (defId, entry, line) => {
    const voice = voices[entry.persona];
    const backend = backendOf(voice);
    if (backend === "say") {
      const values = {
        say: voice.say,
        rate: entry.rate ?? voice.rate,
        pbas: entry.pbas ?? voice.pbas,
        pmod: entry.pmod ?? voice.pmod,
        text: entry[line],
      };
      expected.set(`${defId}-${line}`, { defId, line, backend, ...values, hash: voiceHash(values) });
    } else {
      const values = {
        voice: voice.voice,
        rate: voice.rate,
        semitones: voice.semitones,
        filter: voice.filter,
        text: entry[line],
      };
      expected.set(`${defId}-${line}`, { defId, line, backend, ...values, hash: sapiHash(values) });
    }
  };
  // The emotes section: its own shape — a voice and the five issue-§3 lines, no hooks.
  for (const [portrait, entry] of Object.entries(isObject(table.emotes) ? table.emotes : {})) {
    const defId = `${EMOTE_PREFIX}${portrait}`;
    if (!EMOTE_PORTRAITS.includes(portrait)) {
      problems.push(`${defId}: not a portrait id`);
      continue;
    }
    const voice = voices[entry?.persona];
    if (!voice) {
      problems.push(`${defId}: unknown voice ${JSON.stringify(entry?.persona)}`);
      continue;
    }
    if (badVoices.has(entry.persona)) {
      problems.push(`${defId}: voice ${entry.persona} cannot render`);
      continue;
    }
    for (const line of EMOTE_LINES) {
      const text = entry[line];
      if (typeof text !== "string" || text.trim() === "") {
        problems.push(`${defId}-${line}: no ${line} line`);
        continue;
      }
      expectEmote(defId, entry, line);
    }
  }
  for (const portrait of EMOTE_PORTRAITS) {
    if (!Object.hasOwn(isObject(table.emotes) ? table.emotes : {}, portrait)) {
      problems.push(`${EMOTE_PREFIX}${portrait}: missing from card-audio.json5 emotes`);
    }
  }
  return expected;
}

/** Every catalog card has an entry and SPEC §10.11's lines; every entry is a catalog card. */
function catalogProblems(catalog, table, expected) {
  const problems = [];
  const cards = isObject(table.cards) ? table.cards : {};
  const ids = new Set();
  for (const [id, card] of Object.entries(catalog)) {
    ids.add(id);
    if (!Object.hasOwn(cards, id)) {
      problems.push(`${id}: missing from card-audio.json5`);
      continue;
    }
    const lines = LINES_BY_KIND[KIND_OF_TYPE[card.type]];
    if (!lines) {
      problems.push(`${id}: unknown catalog type ${JSON.stringify(card.type)}`);
      continue;
    }
    for (const line of lines) {
      const key = `${id}-${line}`;
      if (!expected.has(key) && !problems.some((problem) => problem.startsWith(`${key}:`))) {
        problems.push(`${key}: no ${line} line`);
      }
    }
  }
  for (const id of Object.keys(cards)) if (!ids.has(id)) problems.push(`${id}: not in the catalog`);
  return problems;
}

function load(root, catalogPath) {
  const files = {
    table: path.join(root, "src/audio/card-audio.json5"),
    manifest: path.join(root, "src/audio/voice-manifest.json"),
    voiceDir: path.join(root, "public/audio/voice"),
  };
  const dataProblems = [];
  let table = {};
  try {
    table = JSON5.parse(fs.readFileSync(files.table, "utf8"));
    if (!isObject(table)) {
      dataProblems.push("card-audio.json5: not an object");
      table = {};
    }
  } catch (err) {
    dataProblems.push(`card-audio.json5: ${err.message}`);
  }
  const expected = expectedKeys(table, dataProblems);

  try {
    dataProblems.push(...catalogProblems(readJson(catalogPath), table, expected));
  } catch (err) {
    dataProblems.push(`catalog: ${err.message}`);
  }

  const manifestProblems = [];
  let manifest = { version: 1, format: FORMAT, files: {} };
  const manifestText = readText(files.manifest);
  if (manifestText === null) manifestProblems.push("voice-manifest.json: missing");
  else {
    try {
      manifest = JSON.parse(manifestText);
      if (manifest.version !== 1) manifestProblems.push(`voice-manifest.json: version ${JSON.stringify(manifest.version)}, expected 1`);
      if (manifest.format !== FORMAT) manifestProblems.push(`voice-manifest.json: format ${JSON.stringify(manifest.format)}, expected "${FORMAT}"`);
      if (!manifest.files || typeof manifest.files !== "object") manifest.files = {};
    } catch (err) {
      manifestProblems.push(`voice-manifest.json: ${err.message}`);
      manifest = { version: 1, format: FORMAT, files: {} };
    }
  }
  return { files, table, expected, dataProblems, manifest, manifestText, manifestProblems };
}

function dirEntries(dir) {
  try {
    return fs.readdirSync(dir, { withFileTypes: true });
  } catch {
    return [];
  }
}

/** Anything in the voice dir that is not an expected `<key>.m4a`, named by its key where it has one. */
function orphans(dir, expected) {
  const out = [];
  for (const entry of dirEntries(dir)) {
    const key = entry.name.endsWith(".m4a") ? entry.name.slice(0, -".m4a".length) : null;
    if (key !== null && expected.has(key) && entry.isFile()) continue;
    out.push({ name: entry.name, label: key ?? entry.name, isFile: entry.isFile() || entry.isSymbolicLink() });
  }
  return out;
}

function totals(dir, expected) {
  let bytes = 0;
  let blocks = 0;
  let count = 0;
  for (const key of expected.keys()) {
    const size = fileSize(path.join(dir, `${key}.m4a`));
    if (size === null) continue;
    count++;
    bytes += size;
    blocks += onDisk(size);
  }
  return { bytes, blocks, count };
}

function budgetProblem(blocks) {
  return blocks > BUDGET_BYTES ? `budget: ${blocks} bytes on disk (4 KiB blocks) is over ${BUDGET_BYTES}` : null;
}

function report(problems) {
  for (const problem of problems) console.log(problem);
  console.error(`gen-voice: ${problems.length} problem${problems.length === 1 ? "" : "s"}`);
}

function check(ctx) {
  const { expected, manifest, files } = ctx;
  const problems = [...ctx.dataProblems, ...ctx.manifestProblems];
  const entries = manifest.files;
  for (const [key, want] of expected) {
    const record = entries[key];
    const size = fileSize(path.join(files.voiceDir, `${key}.m4a`));
    if (!record) problems.push(`${key}: missing from the manifest`);
    else if (record.hash !== want.hash) problems.push(`${key}: stale hash`);
    if (size === null) problems.push(`${key}: missing file`);
    else if (record && record.bytes !== size) problems.push(`${key}: file is ${size} bytes, the manifest says ${record.bytes}`);
  }
  for (const key of Object.keys(entries)) if (!expected.has(key)) problems.push(`${key}: orphan manifest entry`);
  for (const orphan of orphans(files.voiceDir, expected)) problems.push(`${orphan.label}: orphan file`);
  const sum = totals(files.voiceDir, expected);
  const over = budgetProblem(sum.blocks);
  if (over) problems.push(over);

  if (problems.length > 0) {
    report(problems);
    return 1;
  }
  console.log(`gen-voice: ok, ${sum.count} files, ${sum.bytes} bytes`);
  return 0;
}

function hasTool(name) {
  return spawnSync("which", [name], { stdio: "ignore" }).status === 0;
}

function run(cmd, args) {
  return new Promise((resolve) => {
    const child = spawn(cmd, args, { stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => (stdout += chunk));
    child.stderr.on("data", (chunk) => (stderr += chunk));
    child.on("error", (err) => resolve({ code: -1, stdout, stderr: err.message }));
    child.on("close", (code) => resolve({ code, stdout, stderr: stderr.trim() }));
  });
}

async function pool(items, jobs, work) {
  let next = 0;
  const worker = async () => {
    while (next < items.length) {
      const item = items[next++];
      await work(item);
    }
  };
  await Promise.all(Array.from({ length: Math.min(jobs, items.length) }, worker));
}

/* ------------------------------------------------------------------------------ the say backend --- */

function sayAvailable() {
  return process.platform === "darwin" && hasTool("say") && hasTool("afconvert") && hasTool("afinfo");
}

async function renderSay(key, want, tmp, voiceDir) {
  const aiff = path.join(tmp, `${key}.aiff`);
  const m4a = path.join(tmp, `${key}.m4a`);
  const spoken = `[[rate ${want.rate}]] [[pbas ${want.pbas}]] [[pmod ${want.pmod}]] ${want.text}`;
  const said = await run("say", ["-v", want.say, "-o", aiff, spoken]);
  if (said.code !== 0 || fileSize(aiff) === null) return `say failed (${said.code}) ${said.stderr}`.trim();
  const converted = await run("afconvert", [...AFCONVERT_FLAGS, aiff, m4a]);
  if (converted.code !== 0 || fileSize(m4a) === null) return `afconvert failed (${converted.code}) ${converted.stderr}`.trim();
  fs.copyFileSync(m4a, path.join(voiceDir, `${key}.m4a`));
  return null;
}

/* ----------------------------------------------------------------------------- the SAPI backend --- */

/** `powershell.exe` natively on Windows, or Windows' own from WSL; null when neither is there. */
function powershell() {
  if (process.platform === "win32") return hasTool("powershell.exe") ? { exe: "powershell.exe", wsl: false } : null;
  if (process.platform === "linux" && fileSize(POWERSHELL_WSL) !== null && hasTool("wslpath")) {
    return { exe: POWERSHELL_WSL, wsl: true };
  }
  return null;
}

function sapiAvailable() {
  return powershell() !== null && hasTool("ffmpeg") && hasTool("ffprobe");
}

/** A path as Windows spells it (from WSL through `wslpath -w`). */
function winPath(shell, file) {
  if (!shell.wsl) return file;
  const out = spawnSync("wslpath", ["-w", file], { encoding: "utf8" });
  if (out.status !== 0) throw new Error(`wslpath -w ${file} failed`);
  return out.stdout.trim();
}

/** A scratch dir both sides can read: Windows' temp dir, seen from WSL through `wslpath -u`. */
async function sapiScratch(shell) {
  if (!shell.wsl) return fs.mkdtempSync(path.join(os.tmpdir(), "gen-voice-sapi-"));
  const temp = await run(shell.exe, ["-NoProfile", "-NonInteractive", "-Command", "[System.IO.Path]::GetTempPath()"]);
  if (temp.code !== 0) throw new Error(`powershell could not name its temp dir: ${temp.stderr}`);
  const linux = spawnSync("wslpath", ["-u", temp.stdout.trim()], { encoding: "utf8" });
  if (linux.status !== 0) throw new Error("wslpath -u failed on the Windows temp dir");
  return fs.mkdtempSync(path.join(linux.stdout.trim(), "jackioh-gen-voice-"));
}

const SAPI_SCRIPT = `param([string]$Jobs, [string]$OutDir)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Speech
$items = Get-Content -Raw -LiteralPath $Jobs | ConvertFrom-Json
$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer
foreach ($item in $items) {
  try {
    $synth.SelectVoice($item.voice)
    $synth.SetOutputToWaveFile((Join-Path $OutDir ($item.key + '.wav')))
    $synth.SpeakSsml($item.ssml)
    $synth.SetOutputToNull()
    Write-Output ('ok ' + $item.key)
  } catch {
    $synth.SetOutputToNull()
    Write-Output ('fail ' + $item.key + ' ' + $_.Exception.Message)
  }
}
$synth.Dispose()
`;

function xmlEscape(text) {
  return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/'/g, "&apos;").replace(/"/g, "&quot;");
}

function ssmlFor(want) {
  const rate = `${want.rate >= 0 ? "+" : ""}${want.rate}%`;
  return (
    '<speak version="1.0" xmlns="http://www.w3.org/2001/10/synthesis" xml:lang="en-US">' +
    `<prosody rate="${rate}">${xmlEscape(want.text)}</prosody></speak>`
  );
}

/** Speak a batch to WAVs in `dir`; returns key → failure text for the ones that failed. */
async function speakBatch(shell, dir, batch) {
  const jobs = path.join(dir, `jobs-${batch[0][0]}.json`);
  const script = path.join(dir, "speak.ps1");
  if (fileSize(script) === null) fs.writeFileSync(script, SAPI_SCRIPT);
  fs.writeFileSync(jobs, JSON.stringify(batch.map(([key, want]) => ({ key, voice: want.voice, ssml: ssmlFor(want) }))));
  const out = await run(shell.exe, [
    "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass",
    "-File", winPath(shell, script), "-Jobs", winPath(shell, jobs), "-OutDir", winPath(shell, dir),
  ]);
  const failures = new Map();
  const done = new Set();
  for (const line of out.stdout.split(/\r?\n/)) {
    const ok = /^ok (\S+)$/.exec(line);
    if (ok) done.add(ok[1]);
    const failed = /^fail (\S+) (.*)$/.exec(line);
    if (failed) failures.set(failed[1], `SAPI failed: ${failed[2]}`);
  }
  for (const [key] of batch) {
    if (!done.has(key) && !failures.has(key)) failures.set(key, `SAPI failed (${out.code}) ${out.stderr}`.trim());
  }
  return failures;
}

/** asetrate + aresample + atempo: a pitch shift that keeps the tempo. */
function pitchChain(semitones) {
  if (semitones === 0) return [];
  const factor = 2 ** (semitones / 12);
  return [`asetrate=${Math.round(SAMPLE_RATE * factor)}`, `aresample=${SAMPLE_RATE}`, `atempo=${(1 / factor).toFixed(6)}`];
}

function trimChain() {
  const trim = `silenceremove=start_periods=1:start_threshold=${TRIM_THRESHOLD_DB}dB`;
  return [trim, "areverse", trim, "areverse"];
}

async function peakDb(file) {
  const out = await run("ffmpeg", ["-hide_banner", "-nostats", "-i", file, "-af", "volumedetect", "-f", "null", "-"]);
  const match = /max_volume:\s*(-?[\d.]+) dB/.exec(out.stderr);
  return match ? Number(match[1]) : null;
}

/** WAV → trimmed, pitched, filtered, normalised M4A in the voice dir. */
async function encodeSapi(key, want, wav, dir, voiceDir) {
  const shaped = path.join(dir, `${key}.shaped.wav`);
  const m4a = path.join(dir, `${key}.m4a`);
  const chain = [`aresample=${SAMPLE_RATE}`, ...trimChain(), ...pitchChain(want.semitones)];
  if (want.filter !== "") chain.push(want.filter);
  const shapedRun = await run("ffmpeg", [
    "-hide_banner", "-loglevel", "error", "-y", "-i", wav, "-af", chain.join(","), "-ac", "1", "-ar", String(SAMPLE_RATE), shaped,
  ]);
  if (shapedRun.code !== 0 || fileSize(shaped) === null) return `ffmpeg shaping failed (${shapedRun.code}) ${shapedRun.stderr}`.trim();
  const peak = await peakDb(shaped);
  if (peak === null) return "ffmpeg could not measure the peak";
  const gain = (PEAK_TARGET_DB - peak).toFixed(2);
  const encoded = await run("ffmpeg", [
    "-hide_banner", "-loglevel", "error", "-y", "-i", shaped, "-af", `volume=${gain}dB`, ...FFMPEG_ENCODE, m4a,
  ]);
  if (encoded.code !== 0 || fileSize(m4a) === null) return `ffmpeg encoding failed (${encoded.code}) ${encoded.stderr}`.trim();
  const out = path.join(voiceDir, `${key}.m4a`);
  fs.copyFileSync(m4a, out);
  // A file made on a Windows drive reads as executable from WSL; a voice file is plain data.
  fs.chmodSync(out, FILE_MODE);
  return null;
}

async function renderSapiAll(todo, voiceDir, onDone) {
  const shell = powershell();
  const dir = await sapiScratch(shell);
  try {
    const batches = [];
    for (let i = 0; i < todo.length; i += SAPI_BATCH) batches.push(todo.slice(i, i + SAPI_BATCH));
    await pool(batches, JOBS, async (batch) => {
      const failures = await speakBatch(shell, dir, batch);
      await pool(batch, 1, async ([key, want]) => {
        const spoken = failures.get(key);
        if (spoken) return onDone(key, want, spoken);
        const failure = await encodeSapi(key, want, path.join(dir, `${key}.wav`), dir, voiceDir);
        return onDone(key, want, failure);
      });
    });
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

/* ----------------------------------------------------------------------------------- durations --- */

async function durationSeconds(file) {
  if (process.platform === "darwin" && hasTool("afinfo")) {
    const info = await run("afinfo", [file]);
    const match = /estimated duration:\s*([\d.]+)\s*sec/.exec(info.stdout);
    return match ? Number(match[1]) : null;
  }
  const probe = await run("ffprobe", ["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", file]);
  const seconds = Number(probe.stdout.trim());
  return probe.code === 0 && Number.isFinite(seconds) ? seconds : null;
}

function sortedObject(record) {
  const out = {};
  for (const key of Object.keys(record).sort()) out[key] = record[key];
  return out;
}

async function generate(ctx, opts) {
  const available = { say: sayAvailable(), sapi: sapiAvailable() };
  if (!available.say && !available.sapi) {
    console.error("gen-voice: needs macOS say and afconvert, or Windows SAPI (powershell.exe) and ffmpeg");
    return 2;
  }
  const { expected, manifest, files } = ctx;
  // Never render or delete anything from a card table that doesn't hold together.
  if (ctx.dataProblems.length > 0) {
    report(ctx.dataProblems);
    return 1;
  }
  if (opts.only !== null && ![...expected.values()].some((want) => want.defId === opts.only)) {
    usage(`--only ${opts.only}: no card with a voice line by that id in card-audio.json5`);
  }
  fs.mkdirSync(files.voiceDir, { recursive: true });

  // Manifest entries for keys that are no longer expected are orphans and are dropped here.
  const entries = {};
  for (const key of expected.keys()) if (manifest.files[key]) entries[key] = manifest.files[key];

  const todo = { say: [], sapi: [] };
  const problems = [];
  let kept = 0;
  for (const [key, want] of expected) {
    if (opts.only !== null && want.defId !== opts.only) continue;
    const record = entries[key];
    const size = fileSize(path.join(files.voiceDir, `${key}.m4a`));
    if (!opts.force && record && record.hash === want.hash && size !== null && size === record.bytes) {
      kept++;
      continue;
    }
    if (!available[want.backend]) {
      // Stale, but this machine can't render it: leave its file and entry alone and say what it needs.
      problems.push(`${key}: needs ${want.backend === "say" ? "macOS say" : "Windows SAPI and ffmpeg"} to render`);
      continue;
    }
    todo[want.backend].push([key, want]);
  }

  let rendered = 0;
  const finish = (key, failure) => {
    if (failure) {
      delete entries[key];
      fs.rmSync(path.join(files.voiceDir, `${key}.m4a`), { force: true });
      problems.push(`${key}: ${failure}`);
      return;
    }
    entries[key] = { hash: expected.get(key).hash, bytes: fileSize(path.join(files.voiceDir, `${key}.m4a`)) };
    rendered++;
    console.log(`gen-voice: rendered ${key}`);
  };

  if (todo.say.length > 0) {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "gen-voice-"));
    try {
      await pool(todo.say, JOBS, async ([key, want]) => finish(key, await renderSay(key, want, tmp, files.voiceDir)));
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  }
  if (todo.sapi.length > 0) {
    await renderSapiAll(todo.sapi, files.voiceDir, (key, _want, failure) => finish(key, failure));
  }

  for (const orphan of orphans(files.voiceDir, expected)) {
    if (orphan.isFile) {
      fs.rmSync(path.join(files.voiceDir, orphan.name), { force: true });
      console.log(`gen-voice: removed orphan ${orphan.name}`);
    } else problems.push(`${orphan.label}: unexpected directory ${orphan.name} in the voice dir`);
  }

  const next = { version: 1, format: FORMAT, files: sortedObject(entries) };
  const nextText = `${JSON.stringify(next, null, 2)}\n`;
  if (nextText !== ctx.manifestText) {
    fs.mkdirSync(path.dirname(files.manifest), { recursive: true });
    fs.writeFileSync(files.manifest, nextText);
    console.log(`gen-voice: wrote ${path.relative(process.cwd(), files.manifest) || files.manifest}`);
  }

  const du = spawnSync("du", ["-sk", files.voiceDir], { encoding: "utf8" });
  if (du.status === 0) console.log(`gen-voice: du -sk ${du.stdout.trim()}`);

  const present = [...expected.keys()].filter((key) => fileSize(path.join(files.voiceDir, `${key}.m4a`)) !== null);
  await pool(present, JOBS, async (key) => {
    const seconds = await durationSeconds(path.join(files.voiceDir, `${key}.m4a`));
    if (seconds === null) problems.push(`${key}: no duration could be read`);
    else if (seconds > MAX_SECONDS) problems.push(`${key}: ${seconds.toFixed(2)} s is longer than ${MAX_SECONDS.toFixed(1)} s`);
  });

  const sum = totals(files.voiceDir, expected);
  const over = budgetProblem(sum.blocks);
  if (over) problems.push(over);

  console.log(`gen-voice: rendered ${rendered}, kept ${kept}, ${sum.count} files, ${sum.bytes} bytes`);
  if (problems.length > 0) {
    report(problems.sort());
    return 1;
  }
  return 0;
}

const opts = parseArgs(process.argv.slice(2));
const ctx = load(opts.root, opts.catalog);
process.exitCode = opts.check ? check(ctx) : await generate(ctx, opts);
