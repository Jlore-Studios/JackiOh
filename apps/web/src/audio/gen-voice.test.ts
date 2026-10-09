// Polish task 2 (docs/polish/2-sound.md), B45: `gen-voice.mjs` in its default, generate mode (B37
// covers `--check`).
//
//   B45  With no synthesizer (neither macOS `say`, `afconvert`, `afinfo` nor Windows SAPI through
//        `powershell.exe` with `ffmpeg`) it exits 2 with its needs-macOS line. On macOS it is
//        idempotent by input hash, deletes orphans, and renders only an edited line's key.
//   R501 Where Windows SAPI and ffmpeg are, it renders a SAPI voice's line to an M4A in the committed
//        format within the length cap, and reports a stale `say` line as needing macOS.
//
// Every run points `--root` at a temp copy of the tree, never the committed files. The macOS cases
// need the real `say` and the SAPI ones need SAPI; CI (Linux) runs the first case.

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import JSON5 from "json5";
import { afterAll, beforeAll, describe, expect, it } from "vitest";

const here = dirname(fileURLToPath(import.meta.url));
const WEB = resolve(here, "../..");
const GEN_VOICE = join(WEB, "scripts/gen-voice.mjs");
const REL_AUDIO = join("src", "audio", "card-audio.json5");
const REL_MANIFEST = join("src", "audio", "voice-manifest.json");
const REL_VOICE_DIR = join("public", "audio", "voice");
const RUN_TIMEOUT_MS = 120_000;

type Manifest = { version: 1; format: string; files: Record<string, { hash: string; bytes: number }> };
type Voice = { say: string; rate: number; pbas: number; pmod: number };
type Assignment = { voice?: string; text?: string; effect?: string };
type Audio = { voices: Record<string, unknown>; effects: Record<string, unknown>; cards: Record<string, Record<string, Assignment>> };

function hasTool(name: string): boolean {
  return spawnSync("which", [name], { stdio: "ignore" }).status === 0;
}

const ON_MAC = process.platform === "darwin" && ["say", "afconvert", "afinfo"].every(hasTool);
const POWERSHELL_WSL = "/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe";
/** gen-voice.mjs's own test for its SAPI backend: Windows' PowerShell from WSL, and ffmpeg. */
const ON_SAPI =
  !ON_MAC &&
  process.platform === "linux" &&
  existsSync(POWERSHELL_WSL) &&
  ["wslpath", "ffmpeg", "ffprobe"].every(hasTool);
const CATALOG_PATH = resolve(WEB, "../../crates/cards/catalog.json");
/** VOICE_FILE_MAX_MS in constants.ts, in seconds. */
const MAX_SECONDS = 4;

/** The hash formula, recomputed here rather than imported from the script. */
function voiceHash(values: { say: string; rate: number; pbas: number; pmod: number; text: string }): string {
  const { say, rate, pbas, pmod, text } = values;
  return createHash("sha1").update(JSON.stringify({ v: 1, say, rate, pbas, pmod, text })).digest("hex").slice(0, 16);
}

function run(
  root: string,
  env: NodeJS.ProcessEnv = process.env,
  extra: readonly string[] = [],
): { status: number | null; output: string } {
  const result = spawnSync(process.execPath, [GEN_VOICE, "--root", root, ...extra], {
    encoding: "utf8",
    env,
    timeout: RUN_TIMEOUT_MS,
  });
  return { status: result.status, output: `${result.stdout}\n${result.stderr}` };
}

/** R501: a SAPI voice's hash, recomputed from the formula rather than imported. */
function sapiHash(values: { voice: string; rate: number; semitones: number; filter: string; text: string }): string {
  const { voice, rate, semitones, filter, text } = values;
  return createHash("sha1")
    .update(JSON.stringify({ v: 1, backend: "sapi", voice, rate, semitones, filter, text }))
    .digest("hex")
    .slice(0, 16);
}

let scratch = "";
let copies = 0;

/** A fresh copy of everything `--root` reads and writes. */
function copyTree(): string {
  copies += 1;
  const root = join(scratch, `tree-${String(copies)}`);
  mkdirSync(join(root, "src", "audio"), { recursive: true });
  cpSync(join(WEB, REL_AUDIO), join(root, REL_AUDIO));
  cpSync(join(WEB, REL_MANIFEST), join(root, REL_MANIFEST));
  cpSync(join(WEB, REL_VOICE_DIR), join(root, REL_VOICE_DIR), { recursive: true });
  return root;
}

function readAudio(root: string): Audio {
  return JSON5.parse<Audio>(readFileSync(join(root, REL_AUDIO), "utf8"));
}

function writeAudio(root: string, audio: Audio): void {
  writeFileSync(join(root, REL_AUDIO), `${JSON.stringify(audio, null, 2)}\n`);
}

function readManifest(root: string): Manifest {
  return JSON.parse(readFileSync(join(root, REL_MANIFEST), "utf8")) as Manifest;
}

function mtimes(root: string, manifest: Manifest): Map<string, number> {
  return new Map(Object.keys(manifest.files).map((key) => [key, statSync(join(root, REL_VOICE_DIR, `${key}.m4a`)).mtimeMs]));
}

beforeAll(() => {
  scratch = mkdtempSync(join(tmpdir(), "jackioh-gen-voice-generate-"));
});

afterAll(() => {
  if (scratch !== "") rmSync(scratch, { recursive: true, force: true });
});

describe("B45 gen-voice.mjs generate mode", () => {
  it(
    "B45 exits 2 with the needs-macOS line when say, afconvert or afinfo cannot be found",
    () => {
      const root = copyTree();
      const before = readFileSync(join(root, REL_MANIFEST), "utf8");

      // A PATH holding one empty directory: `which` itself cannot be found, so neither can `say`.
      const empty = join(scratch, "empty-path");
      mkdirSync(empty, { recursive: true });
      const result = run(root, { ...process.env, PATH: empty });

      expect(result.status, result.output).toBe(2);
      expect(result.output).toContain("gen-voice: needs macOS say and afconvert, or Windows SAPI (powershell.exe) and ffmpeg");
      expect(readFileSync(join(root, REL_MANIFEST), "utf8")).toBe(before);
    },
    RUN_TIMEOUT_MS,
  );

  it.runIf(ON_MAC)(
    "B45 on an unchanged tree renders nothing and writes nothing",
    () => {
      const root = copyTree();
      const text = readFileSync(join(root, REL_MANIFEST), "utf8");
      const manifest = readManifest(root);
      const before = mtimes(root, manifest);

      const result = run(root);

      expect(result.status, result.output).toBe(0);
      expect(result.output).toContain(`rendered 0, kept ${String(before.size)}`);
      expect(result.output).not.toMatch(/gen-voice: (rendered core-|wrote )/);
      expect(readFileSync(join(root, REL_MANIFEST), "utf8")).toBe(text);
      expect(mtimes(root, manifest)).toEqual(before);
    },
    RUN_TIMEOUT_MS,
  );

  it.runIf(ON_MAC)(
    "B45 deletes an orphan file and an orphan manifest entry",
    () => {
      const root = copyTree();
      const text = readFileSync(join(root, REL_MANIFEST), "utf8");
      const orphanFile = join(root, REL_VOICE_DIR, "core-999-play.m4a");
      writeFileSync(orphanFile, "not audio");
      const manifest = readManifest(root);
      manifest.files["core-998-death"] = { hash: "0123456789abcdef", bytes: 1 };
      writeFileSync(join(root, REL_MANIFEST), `${JSON.stringify(manifest, null, 2)}\n`);

      const result = run(root);

      expect(result.status, result.output).toBe(0);
      expect(result.output).toContain("removed orphan core-999-play.m4a");
      expect(existsSync(orphanFile)).toBe(false);
      expect(readFileSync(join(root, REL_MANIFEST), "utf8")).toBe(text);
    },
    RUN_TIMEOUT_MS,
  );

  it.runIf(ON_MAC)(
    "B45 an edited line renders only its own key and records its new hash",
    () => {
      const root = copyTree();
      const audio = readAudio(root);
      const play = audio.cards["core-004"]?.play;
      const voice = play?.voice === undefined ? undefined : (audio.voices[play.voice] as Voice | undefined);
      if (play === undefined || voice === undefined) throw new Error("core-004's play line and its voice should be in the file");
      play.text = "Let it ride, baby!";
      writeAudio(root, audio);
      const manifest = readManifest(root);
      const before = mtimes(root, manifest);

      const result = run(root);

      expect(result.status, result.output).toBe(0);
      expect(result.output).toContain("gen-voice: rendered core-004-play");
      expect(result.output).toContain(`rendered 1, kept ${String(before.size - 1)}`);

      const after = readManifest(root);
      const hash = voiceHash({ say: voice.say, rate: voice.rate, pbas: voice.pbas, pmod: voice.pmod, text: "Let it ride, baby!" });
      expect(after.files["core-004-play"]).toEqual({
        hash,
        bytes: statSync(join(root, REL_VOICE_DIR, "core-004-play.m4a")).size,
      });
      expect(hash).not.toBe(manifest.files["core-004-play"]?.hash);
      const { "core-004-play": _edited, ...rest } = after.files;
      const { "core-004-play": _original, ...unchanged } = manifest.files;
      expect(rest).toEqual(unchanged);

      const touched = [...mtimes(root, after)].filter(([key, at]) => at !== before.get(key)).map(([key]) => key);
      expect(touched).toEqual(["core-004-play"]);

      const check = spawnSync(process.execPath, [GEN_VOICE, "--check", "--root", root], { encoding: "utf8" });
      expect(check.status, `${check.stdout}\n${check.stderr}`).toBe(0);
    },
    RUN_TIMEOUT_MS,
  );

  it.runIf(ON_SAPI)(
    "R501 renders a SAPI voice's new line alone, in the committed format, and records its hash",
    () => {
      const root = copyTree();
      const voice = { backend: "sapi", voice: "Microsoft Zira Desktop", rate: 5, semitones: -2, filter: "lowpass=f=3500", web: { pitch: 1, rate: 1 } };
      const text = "A voice from another machine.";
      const audio = readAudio(root);
      audio.voices["test-sapi"] = voice;
      audio.cards["classicplus-999"] = { cast: { voice: "test-sapi", text } };
      writeAudio(root, audio);
      const catalog = join(root, "catalog.json");
      const real = JSON.parse(readFileSync(CATALOG_PATH, "utf8")) as Record<string, unknown>;
      writeFileSync(catalog, JSON.stringify({ ...real, "classicplus-999": { type: "Spell" } }));
      const before = readManifest(root);

      const result = run(root, process.env, ["--catalog", catalog]);

      expect(result.status, result.output).toBe(0);
      expect(result.output).toContain("gen-voice: rendered classicplus-999-cast");
      expect(result.output).toContain(`rendered 1, kept ${String(Object.keys(before.files).length)}`);
      const file = join(root, REL_VOICE_DIR, "classicplus-999-cast.m4a");
      const head = readFileSync(file).subarray(0, 12);
      expect(head.subarray(4, 8).toString("latin1")).toBe("ftyp");
      expect(head.subarray(8, 12).toString("latin1")).toBe("M4A ");
      const probe = spawnSync("ffprobe", ["-v", "error", "-show_entries", "stream=sample_rate,channels", "-show_entries", "format=duration", "-of", "json", file], { encoding: "utf8" });
      const info = JSON.parse(probe.stdout) as { streams: { sample_rate: string; channels: number }[]; format: { duration: string } };
      expect(info.streams[0]?.sample_rate).toBe("22050");
      expect(info.streams[0]?.channels).toBe(1);
      expect(Number(info.format.duration)).toBeLessThanOrEqual(MAX_SECONDS);
      expect(readManifest(root).files["classicplus-999-cast"]).toEqual({
        hash: sapiHash({ ...voice, text }),
        bytes: statSync(file).size,
      });

      const again = run(root, process.env, ["--catalog", catalog]);
      expect(again.status, again.output).toBe(0);
      expect(again.output).toContain(`rendered 0, kept ${String(Object.keys(before.files).length + 1)}`);
    },
    RUN_TIMEOUT_MS,
  );

  it.runIf(ON_SAPI)(
    "R501 reports a stale macOS line as needing say, and leaves its file and entry alone",
    () => {
      const root = copyTree();
      const audio = readAudio(root);
      const play = audio.cards["core-004"]?.play;
      if (play === undefined) throw new Error("core-004's play line should be in the file");
      play.text = "Let it ride, baby!";
      writeAudio(root, audio);
      const manifest = readManifest(root);
      const before = mtimes(root, manifest);

      const result = run(root);

      expect(result.status, result.output).toBe(1);
      expect(result.output).toContain("core-004-play: needs macOS say to render");
      expect(readManifest(root).files["core-004-play"]).toEqual(manifest.files["core-004-play"]);
      expect(mtimes(root, manifest)).toEqual(before);
    },
    RUN_TIMEOUT_MS,
  );
});
