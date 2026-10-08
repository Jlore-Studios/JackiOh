#!/usr/bin/env node
// Renders every music track in scripts/music/tracks.mjs to public/audio/music/<id>.m4a and records
// each in src/audio/music-manifest.json (SPEC §10.11 "Music", R631).
//
//   node apps/web/scripts/gen-music.mjs [--check] [--force] [--only <id>] [--root <webDir>]
//   pnpm --filter @jackioh/web gen:music
//
// THE MUSIC IS COMPOSED HERE. Each track is a score written as code (scripts/music/tracks.mjs, on
// the toolkit in compose.mjs): a key, a tempo, chord progressions, parts and melodies built on one
// shared motif. It becomes a MIDI file, FluidSynth renders that with the FluidR3 GM SoundFont (MIT
// licence), and ffmpeg encodes it. The sources and licences are in assets/music/LICENSES.md.
//
// SEAMLESS LOOPS. A looping track is an intro (maybe none) and a body. The MIDI plays the intro,
// the body twice, then silence, and the file keeps the intro, one body and LOOP_LEAD_S + LOOP_TAIL_S
// more. The loop runs from intro + LOOP_LEAD_S to intro + body + LOOP_LEAD_S. Both ends of that
// span are the same moment of the music with the same reverb tails ringing (the start is far enough
// into the first pass that the intro's tail has died, and the end is in the second pass, where the
// first pass's tail is still ringing as it would after any loop), so the jump is silent. The file is
// periodic for LOOP_TAIL_S past the loop's end as well. Lo-fi's wow and crackle are periodic in
// the body's length for the same reason. FluidSynth times events to 64-sample blocks, so the two
// passes differ by a block here and there; a short crossfade that ends LOOP_GUARD_S before the
// loop's end, and a copy of the loop's start from there on, make the file exactly periodic from
// LOOP_GUARD_S before the loop's end to LOOP_TAIL_S after it (`seam`). So the jump is exact even on
// a decoder that keeps AAC's encoder priming (2112 samples, about 48 ms, of delay) or trims more.
//
// CARD INTROS (R1352). A Legendary's or a Mythic's intro is a sting of its own: a few bars that play
// once, then a short tail (the score's `tail`, not STING_TAIL_S) whose last `post.fadeOutS` seconds
// fade to silence, so the file ends clean wherever the reverb is, and its loudness is measured over
// the file it keeps (`post.loudnessOverFile`), so it sits at every other track's level. A track that
// names none of these renders exactly as before, and its hash does not move.
//
// It is idempotent by input: a track renders again only when its hash (the MIDI bytes, the render
// and post settings and the encoding) differs from the manifest's, or its file is missing or has the
// wrong size. `--check` runs anywhere (it needs neither FluidSynth nor ffmpeg) and touches nothing:
// one line per problem and exit 1, or `gen-music: ok, <n> files, <bytes> bytes` and exit 0.
//
// Exit codes: 0 ok, 1 a problem was found, 2 bad arguments or a missing tool.
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { writeMidi } from "./music/midi.mjs";
import { TRACKS } from "./music/tracks.mjs";

/** Bump when the render, the post-processing or the encoding below changes, so every file renders again. */
const HASH_VERSION = 1;
const HASH_CHARS = 16;
const SAMPLE_RATE = 44100;
const FORMAT = "aac-lc 44100 stereo 80k";
const FFMPEG_ENCODE = [
  "-c:a", "aac", "-b:a", "80k", "-ar", String(SAMPLE_RATE), "-ac", "2",
  "-map_metadata", "-1", "-fflags", "+bitexact", "-flags:a", "+bitexact", "-f", "ipod",
];
const SOUNDFONT_NAME = "FluidR3_GM.sf2";
const SOUNDFONT_PATHS = ["/usr/share/sounds/sf2/FluidR3_GM.sf2", "/usr/share/soundfonts/FluidR3_GM.sf2"];
/** How far into the first pass of the body the loop starts: longer than any reverb or release tail. */
const LOOP_LEAD_S = 3;
/** How much periodic audio the file keeps past the loop's end, for a decoder's priming offset. */
const LOOP_TAIL_S = 0.5;
/** The crossfade that makes the loop's jump exact (see `seam`). */
const SEAM_FADE_S = 0.12;
/** How far before the loop's end the file is already exactly periodic: more than AAC's priming. */
const LOOP_GUARD_S = 0.1;
/** A sting keeps this long after its last bar, for the reverb to die. */
const STING_TAIL_S = 2.5;
/** Silence rendered after the MIDI's last note, so FluidSynth's reverb tail is not cut off. */
const RENDER_PAD_S = 4;
/** Loudness: every track is scaled to this RMS, then held under PEAK_DB by a soft limiter. */
const TARGET_RMS_DB = -20;
const PEAK_DB = -1.5;
/**
 * MUSIC_BUDGET_BYTES in src/audio/constants.ts: the whole set, counted in whole disk blocks. R1352
 * raised it from 24 MiB for the card intros (the shipped sets' and, with its release, the Meditative
 * set's), at the same encoding as every track.
 */
const BUDGET_BYTES = 28 * 1024 * 1024;
const BLOCK_BYTES = 4096;
const FILE_MODE = 0o644;
const USAGE = "usage: node scripts/gen-music.mjs [--check] [--force] [--only <id>] [--root <webDir>]";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));

function usage(message) {
  console.error(`gen-music: ${message}`);
  console.error(USAGE);
  process.exit(2);
}

function parseArgs(argv) {
  const opts = { check: false, force: false, only: null, root: path.resolve(SCRIPT_DIR, "..") };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--check") opts.check = true;
    else if (arg === "--force") opts.force = true;
    else if (arg === "--only" || arg === "--root") {
      const value = argv[++i];
      if (value === undefined || value.startsWith("--")) usage(`${arg} needs a value`);
      if (arg === "--only") opts.only = value;
      else opts.root = path.resolve(value);
    } else usage(`unknown argument ${arg}`);
  }
  return opts;
}

/* ------------------------------------------------------------------------------------------- *
 * The score
 * ------------------------------------------------------------------------------------------- */

const secondsPerBeat = (song) => 60 / song.bpm;

/** The MIDI score the renderer plays: intro, body, body again (loops only), then padding. */
function scoreOf(track) {
  const song = track.song;
  const { intro, body } = song.parts;
  const loops = track.loop;
  const passes = loops ? 2 : 0;
  const notes = [...intro.notes];
  const ccs = [...intro.ccs];
  const bends = [...intro.bends];
  for (let pass = 0; pass < passes; pass += 1) {
    const offset = intro.beats + pass * body.beats;
    for (const n of body.notes) notes.push({ ...n, t: n.t + offset });
    for (const c of body.ccs) ccs.push({ ...c, t: c.t + offset });
    for (const b of body.bends) bends.push({ ...b, t: b.t + offset });
  }
  const musicBeats = intro.beats + passes * body.beats;
  const endBeat = musicBeats + RENDER_PAD_S / secondsPerBeat(song);
  return { bpm: song.bpm, beatsPerBar: song.beatsPerBar, channels: song.channels, notes, ccs, bends, endBeat };
}

/** Where the file is cut and the loop sits, in seconds. */
function layoutOf(track) {
  const song = track.song;
  const spb = secondsPerBeat(song);
  const introS = song.parts.intro.beats * spb;
  const bodyS = song.parts.body.beats * spb;
  if (!track.loop) {
    // R1352: a card intro keeps its own, shorter tail; every other sting keeps STING_TAIL_S.
    return { duration: introS + (track.tail ?? STING_TAIL_S), handoff: introS, loopStart: null, loopEnd: null, introS };
  }
  const loopStart = introS + LOOP_LEAD_S;
  const loopEnd = loopStart + bodyS;
  return { duration: loopEnd + LOOP_TAIL_S, handoff: null, loopStart, loopEnd, introS, bodyS };
}

function trackHash(track, midi) {
  const input = {
    v: HASH_VERSION,
    format: FORMAT,
    soundfont: SOUNDFONT_NAME,
    midi: createHash("sha1").update(midi).digest("hex"),
    reverb: track.song.reverb,
    post: track.song.post,
    layout: layoutOf(track),
    loudness: [TARGET_RMS_DB, PEAK_DB],
    seam: [LOOP_LEAD_S, LOOP_TAIL_S, SEAM_FADE_S, LOOP_GUARD_S],
  };
  return createHash("sha1").update(JSON.stringify(input)).digest("hex").slice(0, HASH_CHARS);
}

/* ------------------------------------------------------------------------------------------- *
 * Rendering
 * ------------------------------------------------------------------------------------------- */

function soundfont() {
  return SOUNDFONT_PATHS.find((p) => fs.existsSync(p)) ?? null;
}

function have(cmd, args) {
  const result = spawnSync(cmd, args, { stdio: "ignore" });
  return result.error === undefined;
}

/** Reads FluidSynth's 32-bit float stereo WAV into two channels. */
function readWav(file) {
  const buf = fs.readFileSync(file);
  let offset = 12;
  let format = null;
  while (offset + 8 <= buf.length) {
    const id = buf.toString("ascii", offset, offset + 4);
    const size = buf.readUInt32LE(offset + 4);
    if (id === "fmt ") {
      format = { tag: buf.readUInt16LE(offset + 8), channels: buf.readUInt16LE(offset + 10), rate: buf.readUInt32LE(offset + 12), bits: buf.readUInt16LE(offset + 22) };
    } else if (id === "data") {
      if (format === null || format.bits !== 32 || format.channels !== 2) throw new Error(`unexpected WAV format in ${file}`);
      const frames = Math.floor(size / 8);
      const pcm = new Float32Array(buf.buffer.slice(buf.byteOffset + offset + 8, buf.byteOffset + offset + 8 + frames * 8));
      const left = new Float32Array(frames);
      const right = new Float32Array(frames);
      for (let i = 0; i < frames; i += 1) {
        left[i] = pcm[2 * i];
        right[i] = pcm[2 * i + 1];
      }
      return { rate: format.rate, left, right };
    }
    offset += 8 + size + (size % 2);
  }
  throw new Error(`no data chunk in ${file}`);
}

function render(track, midi, scratch, sf2) {
  const midPath = path.join(scratch, `${track.id}.mid`);
  const wavPath = path.join(scratch, `${track.id}.wav`);
  fs.writeFileSync(midPath, midi);
  const r = track.song.reverb;
  const args = [
    "-ni", "-q", "-F", wavPath, "-T", "wav", "-O", "float", "-r", String(SAMPLE_RATE), "-g", "0.5",
    "-o", "synth.chorus.active=0",
    "-o", "synth.reverb.active=1",
    "-o", `synth.reverb.room-size=${r.room}`,
    "-o", `synth.reverb.damp=${r.damp}`,
    "-o", `synth.reverb.width=${r.width}`,
    "-o", `synth.reverb.level=${r.level}`,
    "-o", "synth.polyphony=512",
    sf2, midPath,
  ];
  const result = spawnSync("fluidsynth", args, { stdio: ["ignore", "ignore", "pipe"] });
  if (result.status !== 0) throw new Error(`fluidsynth failed for ${track.id}: ${result.stderr}`);
  return readWav(wavPath);
}

/* ----- post-processing ----- */

/** A seeded generator for the post effects, so a render is the same every time. */
function noise(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return (((t ^ (t >>> 14)) >>> 0) / 4294967296) * 2 - 1;
  };
}

/** An RBJ biquad low-pass, in place. */
function lowpass(data, rate, freq, q = 0.707) {
  const w = (2 * Math.PI * freq) / rate;
  const alpha = Math.sin(w) / (2 * q);
  const cos = Math.cos(w);
  const a0 = 1 + alpha;
  const b0 = (1 - cos) / 2 / a0;
  const b1 = (1 - cos) / a0;
  const b2 = b0;
  const a1 = (-2 * cos) / a0;
  const a2 = (1 - alpha) / a0;
  let x1 = 0;
  let x2 = 0;
  let y1 = 0;
  let y2 = 0;
  for (let i = 0; i < data.length; i += 1) {
    const x = data[i];
    const y = b0 * x + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
    x2 = x1;
    x1 = x;
    y2 = y1;
    y1 = y;
    data[i] = y;
  }
}

/**
 * Lo-fi: a tape wow (a slowly modulated delay) and a warm low-pass, then vinyl crackle and hiss.
 * The wow's period divides the body and both noises repeat every body, so the loop stays seamless.
 */
function lofi(audio, track, layout) {
  const { rate } = audio;
  const post = track.song.post.lofi;
  // A sting does not loop: its whole length stands in for the body.
  const bodyS = layout.bodyS ?? layout.duration;
  const bodyN = Math.round(bodyS * rate);
  const introN = Math.round((layout.introS ?? 0) * rate);
  const cycles = Math.max(1, Math.round(bodyS / post.wowPeriodS));
  const depth = post.wowDepthS * rate;
  for (const data of [audio.left, audio.right]) {
    const src = Float32Array.from(data);
    for (let i = 0; i < data.length; i += 1) {
      const phase = (2 * Math.PI * cycles * (i - introN)) / bodyN;
      const pos = i - depth * (1 + Math.sin(phase));
      const j = Math.floor(pos);
      const f = pos - j;
      const a = src[Math.max(0, j)] ?? 0;
      const b = src[Math.max(0, j + 1)] ?? 0;
      data[i] = a + (b - a) * f;
    }
    lowpass(data, rate, post.lowpassHz);
    for (let i = 0; i < data.length; i += 1) data[i] = Math.tanh(data[i] * post.drive) / post.drive;
  }
  // One body's worth of crackle and hiss, laid on every pass from the body's start.
  const rand = noise(post.seed);
  const crackle = new Float32Array(bodyN);
  const clicks = Math.round(bodyS * post.clicksPerS);
  for (let c = 0; c < clicks; c += 1) {
    const at = Math.floor(((rand() + 1) / 2) * (bodyN - 40));
    const amp = post.crackle * (0.3 + 0.7 * Math.abs(rand()));
    for (let k = 0; k < 24; k += 1) crackle[at + k] += amp * rand() * Math.exp(-k / 5);
  }
  let pink = 0;
  for (let i = 0; i < bodyN; i += 1) {
    pink = pink * 0.97 + rand() * 0.03;
    crackle[i] += pink * post.hiss;
  }
  for (const data of [audio.left, audio.right]) {
    for (let i = 0; i < data.length; i += 1) {
      const k = ((((i - introN) % bodyN) + bodyN) % bodyN);
      data[i] += crackle[k];
    }
  }
}

/** R1352: a raised-cosine fade over the file's last `seconds`, so a short tail ends in silence. */
function fadeTail(audio, layout, seconds) {
  const end = Math.round(layout.duration * audio.rate);
  const n = Math.max(1, Math.round(seconds * audio.rate));
  for (const data of [audio.left, audio.right]) {
    for (let i = Math.max(0, end - n); i < data.length; i += 1) {
      const k = end - i;
      data[i] *= k <= 0 ? 0 : 0.5 - 0.5 * Math.cos((Math.PI * Math.min(k, n)) / n);
    }
  }
}

/**
 * Scales to TARGET_RMS_DB, then a soft knee keeps every peak under PEAK_DB. The RMS is measured over
 * the whole render, RENDER_PAD_S included, or over its first `frames` only (R1352: a card intro is
 * measured over the file it keeps, so a few seconds of music are not lifted for the padding after).
 */
function loudness(left, right, frames = left.length) {
  const n = Math.min(frames, left.length);
  let sum = 0;
  for (let i = 0; i < n; i += 1) sum += left[i] * left[i] + right[i] * right[i];
  const rms = Math.sqrt(sum / (2 * Math.max(1, n)));
  const gain = rms > 0 ? Math.pow(10, TARGET_RMS_DB / 20) / rms : 1;
  const ceiling = Math.pow(10, PEAK_DB / 20);
  const knee = ceiling * 0.7;
  const shape = (x) => {
    const v = x * gain;
    const a = Math.abs(v);
    if (a <= knee) return v;
    const over = (a - knee) / (ceiling - knee);
    return Math.sign(v) * (knee + (ceiling - knee) * Math.tanh(over));
  };
  for (let i = 0; i < left.length; i += 1) {
    left[i] = shape(left[i]);
    right[i] = shape(right[i]);
  }
}

function encode(track, audio, layout, outFile, scratch) {
  const frames = Math.round(layout.duration * audio.rate);
  const pcm = Buffer.alloc(frames * 8);
  for (let i = 0; i < frames; i += 1) {
    pcm.writeFloatLE(audio.left[i] ?? 0, i * 8);
    pcm.writeFloatLE(audio.right[i] ?? 0, i * 8 + 4);
  }
  const rawPath = path.join(scratch, `${track.id}.f32`);
  fs.writeFileSync(rawPath, pcm);
  const tmpOut = path.join(scratch, `${track.id}.m4a`);
  const args = ["-y", "-hide_banner", "-loglevel", "error", "-f", "f32le", "-ar", String(audio.rate), "-ac", "2", "-i", rawPath, ...FFMPEG_ENCODE, tmpOut];
  const result = spawnSync("ffmpeg", args, { stdio: ["ignore", "ignore", "pipe"] });
  if (result.status !== 0) throw new Error(`ffmpeg failed for ${track.id}: ${result.stderr}`);
  fs.copyFileSync(tmpOut, outFile);
  fs.chmodSync(outFile, FILE_MODE);
}

/**
 * Makes the loop's jump sample-exact. FluidSynth starts each MIDI event on a 64-sample block, so the
 * second pass of the body sits up to a block off the first, note by note, and the two are close but
 * not equal. SEAM_FADE_S of audio ending LOOP_GUARD_S before the loop's end fades into the matching
 * audio before its start, and from there to the end of the file every sample is a copy of the one a
 * loop's length earlier. A decoder that shifts the samples later by up to LOOP_GUARD_S, or earlier by
 * up to LOOP_TAIL_S, still jumps between two identical samples.
 */
function seam(audio, layout) {
  const a = Math.round(layout.loopStart * audio.rate);
  const b = Math.round(layout.loopEnd * audio.rate);
  const x = Math.round(SEAM_FADE_S * audio.rate);
  const g = Math.round(LOOP_GUARD_S * audio.rate);
  const tail = Math.round(LOOP_TAIL_S * audio.rate) + 1;
  for (const data of [audio.left, audio.right]) {
    for (let i = 0; i < x; i += 1) {
      const w = 0.5 - 0.5 * Math.cos((Math.PI * (i + 1)) / x);
      data[b - g - x + i] = (1 - w) * data[b - g - x + i] + w * data[a - g - x + i];
    }
    for (let k = -g; k < tail; k += 1) data[b + k] = data[a + k];
  }
}

function build(track, midi, scratch, sf2, outFile) {
  const audio = render(track, midi, scratch, sf2);
  const layout = layoutOf(track);
  if (track.song.post.lofi !== undefined) lofi(audio, track, layout);
  loudness(audio.left, audio.right, track.song.post.loudnessOverFile === true ? Math.round(layout.duration * audio.rate) : undefined);
  if (track.loop) seam(audio, layout);
  if (track.song.post.fadeOutS !== undefined) fadeTail(audio, layout, track.song.post.fadeOutS);
  encode(track, audio, layout, outFile, scratch);
}

/* ------------------------------------------------------------------------------------------- *
 * The manifest
 * ------------------------------------------------------------------------------------------- */

const round = (x) => (x === null ? null : Math.round(x * 1e6) / 1e6);

function entryOf(track, hash, bytes) {
  const layout = layoutOf(track);
  return {
    hash,
    bytes,
    bpm: track.song.bpm,
    beatsPerBar: track.song.beatsPerBar,
    loop: track.loop,
    intro: round(layout.introS),
    duration: round(layout.duration),
    loopStart: round(layout.loopStart),
    loopEnd: round(layout.loopEnd),
    handoff: round(layout.handoff),
  };
}

function onDisk(bytes) {
  return Math.ceil(bytes / BLOCK_BYTES) * BLOCK_BYTES;
}

function fileSize(file) {
  try {
    const stat = fs.statSync(file);
    return stat.isFile() ? stat.size : null;
  } catch {
    return null;
  }
}

function main() {
  const opts = parseArgs(process.argv.slice(2));
  const manifestPath = path.join(opts.root, "src/audio/music-manifest.json");
  const outDir = path.join(opts.root, "public/audio/music");
  const old = fs.existsSync(manifestPath) ? JSON.parse(fs.readFileSync(manifestPath, "utf8")) : { version: 1, format: FORMAT, files: {} };
  const tracks = TRACKS.map((def) => ({ ...def.build(), id: def.id, loop: def.loop }));
  if (opts.only !== null && !tracks.some((t) => t.id === opts.only)) usage(`no track ${opts.only}`);
  const problems = [];
  const files = {};
  let sf2 = null;
  let scratch = null;

  for (const track of tracks) {
    const midi = writeMidi(scoreOf(track));
    const hash = trackHash(track, midi);
    const file = path.join(outDir, `${track.id}.m4a`);
    const current = old.files[track.id];
    const size = fileSize(file);
    const fresh = current !== undefined && current.hash === hash && size !== null && size === current.bytes;
    if (opts.check) {
      if (!fresh) problems.push(`${track.id}: ${current === undefined ? "missing from the manifest" : size === null ? "file missing" : current.hash !== hash ? "score changed since its render" : "file size differs from the manifest"}`);
      else {
        files[track.id] = current;
        const expected = entryOf(track, current.hash, current.bytes);
        if (JSON.stringify(expected) !== JSON.stringify(current)) problems.push(`${track.id}: manifest entry is stale (run gen:music)`);
      }
      continue;
    }
    // `--only` renders that one track; every other keeps whatever entry it has.
    const skip = opts.only !== null ? opts.only !== track.id : fresh && !opts.force;
    if (skip) {
      if (current !== undefined) files[track.id] = entryOf(track, current.hash, current.bytes);
      continue;
    }
    if (sf2 === null) {
      sf2 = soundfont();
      if (sf2 === null || !have("fluidsynth", ["--version"]) || !have("ffmpeg", ["-version"])) {
        console.error(`gen-music: needs fluidsynth, ffmpeg and ${SOUNDFONT_NAME} (apt: fluidsynth fluid-soundfont-gm ffmpeg)`);
        process.exit(2);
      }
      scratch = fs.mkdtempSync(path.join(os.tmpdir(), "jackioh-gen-music-"));
      fs.mkdirSync(outDir, { recursive: true });
    }
    build(track, midi, scratch, sf2, file);
    const bytes = fileSize(file);
    files[track.id] = entryOf(track, hash, bytes);
    console.log(`gen-music: ${track.id} ${bytes} bytes, ${files[track.id].duration.toFixed(1)} s`);
  }

  const known = new Set(tracks.map((t) => t.id));
  const strays = fs.existsSync(outDir) ? fs.readdirSync(outDir).filter((f) => f.endsWith(".m4a") && !known.has(f.slice(0, -4))) : [];
  const total = Object.values(files).reduce((sum, f) => sum + onDisk(f.bytes), 0);
  if (opts.check) {
    for (const stray of strays) problems.push(`${stray}: not a track in scripts/music/tracks.mjs`);
    for (const id of Object.keys(old.files)) if (!known.has(id)) problems.push(`${id}: in the manifest but not a track`);
    if (total > BUDGET_BYTES) problems.push(`budget: ${total} bytes on disk is over ${BUDGET_BYTES}`);
    if (problems.length > 0) {
      for (const p of problems) console.log(p);
      process.exit(1);
    }
    console.log(`gen-music: ok, ${Object.keys(files).length} files, ${Object.values(files).reduce((s, f) => s + f.bytes, 0)} bytes`);
    return;
  }
  for (const stray of strays) fs.rmSync(path.join(outDir, stray));
  const ordered = {};
  for (const track of tracks) if (files[track.id] !== undefined) ordered[track.id] = files[track.id];
  const next = `${JSON.stringify({ version: 1, format: FORMAT, files: ordered }, null, 2)}\n`;
  if (!fs.existsSync(manifestPath) || fs.readFileSync(manifestPath, "utf8") !== next) fs.writeFileSync(manifestPath, next);
  if (scratch !== null) fs.rmSync(scratch, { recursive: true, force: true });
  if (total > BUDGET_BYTES) {
    console.error(`gen-music: ${total} bytes on disk is over the ${BUDGET_BYTES} budget`);
    process.exit(1);
  }
  console.log(`gen-music: done, ${Object.keys(ordered).length} files, ${total} bytes on disk`);
}

main();
