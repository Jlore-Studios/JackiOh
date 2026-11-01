#!/usr/bin/env node
// Turns a reel script and a gameplay clip into the files Reel Studio
// (https://github.com/AidenJohnston/reel-studio-app) edits, laid out as its Library (#2616):
//
//   <root>/Library/Raw takes/<name>.mp4   gameplay with the script read over it: Reel Studio's "talking head"
//                                         (a clip of 12 s or more with voice for most of it, its probe.py)
//   <root>/Library/B-roll/<name>-gameplay.mp4   the silent gameplay, for cut-aways
//   <root>/Library/Music/jackioh-<track>.m4a    the game's own music (assets/music/LICENSES.md)
//
//   node marketing/reels/kit.mjs --script marketing/reels/scripts/first-look.txt \
//        --gameplay marketing/reels/out/gameplay.mp4 [--root "marketing/reels/out/Reel Studio"] [--music edm-start]
//
// The voice is the machine's text to speech: macOS `say`, else `espeak-ng`. A person reading the
// script to a camera makes a better take; drop that file in Raw takes instead.
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, extname, join, resolve } from "node:path";
import { parseArgs } from "node:util";

const { values: opt } = parseArgs({
  options: {
    script: { type: "string" },
    gameplay: { type: "string", default: "marketing/reels/out/gameplay.mp4" },
    root: { type: "string", default: "marketing/reels/out/Reel Studio" },
    music: { type: "string", default: "edm-start" },
  },
});
if (!opt.script) throw new Error("--script <file> is required: one spoken line per line");

// The silence between lines: Reel Studio cuts dead air down to its own gap, so this only has to
// keep the lines apart.
const LINE_GAP_S = 0.4;
const RATE = 22050;

const run = (cmd, args) => execFileSync(cmd, args, { stdio: ["ignore", "pipe", "inherit"] }).toString();
const has = (cmd) => {
  try {
    execFileSync("sh", ["-c", `command -v ${cmd}`], { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
};

const name = basename(opt.script, extname(opt.script));
const lines = readFileSync(opt.script, "utf8").split("\n").map((l) => l.trim()).filter((l) => l && !l.startsWith("#"));
const work = mkdtempSync(join(tmpdir(), "jackioh-kit-"));
const lib = (folder) => {
  const dir = join(resolve(opt.root), "Library", folder);
  mkdirSync(dir, { recursive: true });
  return dir;
};

// 1. Each line to a mono wav at one rate, then all of them with a gap after each.
const say = has("say") ? "say" : has("espeak-ng") ? "espeak-ng" : null;
if (say === null) throw new Error("no text to speech: install espeak-ng (Linux) or run on macOS");
const parts = lines.map((line, i) => {
  const voiced = join(work, `line-${i}.${say === "say" ? "aiff" : "wav"}`);
  if (say === "say") run("say", ["-o", voiced, line]);
  else run("espeak-ng", ["-v", "en-us", "-s", "160", "-w", voiced, line]);
  const wav = join(work, `line-${i}-n.wav`);
  run("ffmpeg", ["-y", "-loglevel", "error", "-i", voiced, "-af", `apad=pad_dur=${LINE_GAP_S}`, "-ac", "1", "-ar", `${RATE}`, wav]);
  return wav;
});
const list = join(work, "lines.txt");
writeFileSync(list, parts.map((p) => `file '${p}'`).join("\n"));
const narration = join(work, "narration.wav");
run("ffmpeg", ["-y", "-loglevel", "error", "-f", "concat", "-safe", "0", "-i", list, "-c", "copy", narration]);

// 2. The take: the gameplay (looped if the narration runs longer) under the narration, as long as the narration.
const take = join(lib("Raw takes"), `${name}.mp4`);
run("ffmpeg", ["-y", "-loglevel", "error", "-stream_loop", "-1", "-i", resolve(opt.gameplay), "-i", narration,
  "-map", "0:v", "-map", "1:a", "-shortest", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "18",
  "-c:a", "aac", "-b:a", "192k", "-movflags", "+faststart", take]);

// 3. The b-roll and the music, as they are.
const broll = join(lib("B-roll"), `${name}-gameplay.mp4`);
copyFileSync(resolve(opt.gameplay), broll);
const song = join(lib("Music"), `jackioh-${opt.music}.m4a`);
copyFileSync(resolve("apps/web/public/audio/music", `${opt.music}.m4a`), song);

const seconds = Number(run("ffprobe", ["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", take]));
rmSync(work, { recursive: true, force: true });
console.log(JSON.stringify({ take, seconds, lines: lines.length, voice: say, broll, song }, null, 2));
