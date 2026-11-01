// Audition candidates for the stations' in-game music (SPEC §10.11 "Music", R631), and the House
// station's first tracks. Not shipped: `gen-music.mjs --candidates --root <dir>` renders these into a
// scratch tree for the owner to listen to, and the ones kept move into tracks.mjs under station ids.
//
// The four stations' first tracks share one arrangement each (tracks.mjs `tavern()`, `edm()`, …) and
// differ by key and progression. Each candidate here has an arrangement of its own instead, from a
// different corner of its station's genre: its own meter, tempo, groove, instruments, harmony and
// form. Every one still quotes the "Jack-i-Oh" motif (compose.mjs MOTIF_CELL, through `melody`'s
// `motifBars`, or the theme itself), loops a whole number of bars, and runs past two minutes (#51).

import {
  DR, DRUMS, GM, KIT, RHYTHMS,
  arp, bass, bend, cc, comp, createSong, drums, instrument, line, melody, note, pad, pulse, pump, roots, section,
} from "./compose.mjs";
import { key } from "./theory.mjs";
import { MOTIF_THEME } from "./tracks.mjs";

/** General MIDI programs compose.mjs's GM leaves out (0-based). */
const P = {
  ...GM,
  honkyTonk: 3, clav: 7, percOrgan: 17, rockOrgan: 18, churchOrgan: 19, harmonica: 22, tangoAccordion: 23,
  slapBass: 36, altoSax: 65, tenorSax: 66, synthStrings2: 51, synthBrass2: 63, shakuhachi: 77,
  shanai: 111, steelDrums: 114, taiko: 116, synthDrum: 118,
};
/** Drum notes compose.mjs's DR leaves out. */
const D = { ...DR, cowbell: 56, agogoHi: 67, agogoLo: 68, guiro: 73 };

/** #51: whatever plays for a long stretch loops for two minutes at least. */
const MIN_LOOP_S = 120;

/** One bar of a drum, hit at each of `times` at velocity `v`. */
const at = (n, v, times) => times.map((t) => [t, n, v]);
/** Every `step` beats from 0 up to `beats`. */
const every = (step, beats = 4) => Array.from({ length: Math.round(beats / step) }, (_, i) => i * step);

/**
 * The body as a run of sections: `form` is [bars, progression, part names], and each part is
 * `(section, index)` writing into it. Throws when the loop runs under MIN_LOOP_S.
 */
function arrange(song, parts, form) {
  form.forEach(([bars, prog, names], i) => {
    const s = section(song, "body", bars, prog);
    for (const name of names) {
      const part = parts[name];
      if (part === undefined) throw new Error(`${song.id}: no part ${name}`);
      part(s, i);
    }
  });
  const seconds = (song.parts.body.beats * 60) / song.bpm;
  if (seconds < MIN_LOOP_S) throw new Error(`${song.id}: its loop runs ${seconds.toFixed(1)} s, under ${MIN_LOOP_S}`);
  return { song };
}

/** A drum roll in bar `bar` of a section, from beat `from` to `to` every `step`, swelling `v0` → `v1`. */
function roll(s, bar, { n, from = 0, to = 4, step = 0.25, v0 = 30, v1 = 90 }) {
  const base = bar * s.song.beatsPerBar;
  const count = Math.round((to - from) / step);
  for (let i = 0; i < count; i += 1) note(s, DRUMS, base + from + i * step, 0.2, n, v0 + ((v1 - v0) * i) / Math.max(1, count - 1));
}

/** A volume LFO on CC11 (a wobble or a gate): one cycle every `cycle` beats, `depth` deep. */
function wobble(s, { ch, cycle = 0.5, depth = 0.85, from = 0, to = s.beats, shape = "saw" }) {
  const steps = 8;
  for (let t = from; t < to - 1e-9; t += cycle) {
    for (let k = 0; k < steps; k += 1) {
      const f = k / steps;
      const lfo = shape === "saw" ? 1 - f : 0.5 + 0.5 * Math.cos(2 * Math.PI * f);
      cc(s, ch, t + f * cycle, 11, 127 * (1 - depth + depth * lfo));
    }
  }
  cc(s, ch, to - 0.01, 11, 127);
}

/** CC11 rising from `lo` to full over the section (a filter-style build). */
function swell(s, { ch, lo = 40, steps = 32 }) {
  for (let k = 0; k <= steps; k += 1) cc(s, ch, (s.beats * k) / steps - (k === steps ? 0.01 : 0), 11, lo + ((127 - lo) * k) / steps);
}

/** The theme's first four bars (the question), and its answer. */
const QUESTION = MOTIF_THEME.slice(0, 18);
const ANSWER = MOTIF_THEME.slice(18);
/** The theme's first two bars, the fanfare a match-start sting opens with. */
const FANFARE = MOTIF_THEME.slice(0, 10);

/* ------------------------------------------------------------------------------------------- *
 * Tavern
 * ------------------------------------------------------------------------------------------- */

/** A hearth waltz: accordion and violin over an oom-pah-pah guitar, in A dorian. */
function tavernWaltz() {
  const song = createSong({ id: "tavern-waltz", bpm: 138, beatsPerBar: 3, key: key("A", "dorian") });
  song.reverb = { room: 0.6, damp: 0.45, width: 0.85, level: 0.5 };
  instrument(song, 0, GM.accordion, { volume: 92, pan: 52, reverb: 45 });
  instrument(song, 1, GM.violin, { volume: 86, pan: 78, reverb: 55 });
  instrument(song, 2, GM.nylon, { volume: 90, pan: 40, reverb: 40 });
  instrument(song, 3, GM.acBass, { volume: 86, pan: 64, reverb: 25 });
  instrument(song, 4, GM.strings, { volume: 68, pan: 64, reverb: 65 });
  instrument(song, 5, GM.dulcimer, { volume: 72, pan: 92, reverb: 55 });
  instrument(song, DRUMS, KIT.standard, { volume: 80, reverb: 35 });
  const A = ["i", "IV", "i", "VII", "i", "IV", ["v", "VII"], "i"];
  const B = ["III", "VII", "IV", "i", "ii", "III", "IV", ["v", "IV"]];
  const perc = [...at(D.tamb, 34, [1, 2]), [0, D.triOpen, 26]];
  const parts = {
    oompah: (s) => {
      bass(s, { ch: 3, pattern: [[0, "R", 0.9, 1]], vel: 84 });
      comp(s, { ch: 2, hits: [[1, 0.5, 0.8], [2, 0.5, 0.7]], center: 57, vel: 62 });
    },
    sparkle: (s) => arp(s, { ch: 5, step: 0.5, pattern: [0, 1, 2, 3, 2, 1], center: 69, vel: 44 }),
    perc: (s) => drums(s, { pattern: perc }),
    pad: (s) => pad(s, { ch: 4, center: 60, vel: 40 }),
    lead: (s, i) => melody(s, { ch: 0, rhythms: RHYTHMS.waltz, range: [2, 10], vel: 84, motifBars: [0, 8], start: 4, seed: i }),
    fiddle: (s, i) => melody(s, { ch: 1, rhythms: RHYTHMS.waltz, range: [4, 12], vel: 80, motifBars: [0, 8], start: 7, seed: i }),
    counter: (s, i) => melody(s, { ch: 1, rhythms: [[[3]], [[2], [1]]], range: [7, 13], vel: 58, motifBars: [], start: 9, seed: 10 + i }),
  };
  return arrange(song, parts, [
    [8, A, ["oompah", "sparkle"]],
    [16, A, ["oompah", "perc", "lead"]],
    [16, B, ["oompah", "pad", "fiddle"]],
    [16, A, ["oompah", "perc", "lead", "counter"]],
    [8, B, ["oompah", "pad", "sparkle", "counter"]],
    [16, B, ["oompah", "perc", "pad", "lead"]],
    [16, A, ["oompah", "perc", "sparkle", "fiddle", "pad"]],
  ]);
}

/** A sea shanty in 6/8 (a beat is an eighth): accordion calls, the crew answers, boots stomp. */
function tavernShanty() {
  const song = createSong({ id: "tavern-shanty", bpm: 240, beatsPerBar: 6, key: key("D", "minor"), humanize: 0.02 });
  song.reverb = { room: 0.5, damp: 0.5, width: 0.8, level: 0.45 };
  instrument(song, 0, P.tangoAccordion, { volume: 92, pan: 56, reverb: 40 });
  instrument(song, 1, GM.oohs, { volume: 88, pan: 70, reverb: 55 });
  instrument(song, 2, GM.tuba, { volume: 84, pan: 64, reverb: 30 });
  instrument(song, 3, GM.steel, { volume: 82, pan: 36, reverb: 40 });
  instrument(song, 4, GM.fiddle, { volume: 84, pan: 88, reverb: 45 });
  instrument(song, 5, GM.whistle, { volume: 72, pan: 76, reverb: 50 });
  instrument(song, DRUMS, KIT.room, { volume: 92, reverb: 35 });
  const A = ["i", "i", "VII", "VII", "i", "i", ["iv", "V+"], "i"];
  const B = ["III", "VII", "i", "i", "III", "VII", ["iv", "V+"], "i"];
  const SHANTY = [[[2], [1], [2], [1]], [[3], [2], [1]], [[1], [1], [1], [3]], [[2], [1], [1], [1], [1]]];
  // The call (the motif, stretched to 6/8) and the crew's answer, four bars each.
  const CALL = [[4, 1], [5, 1], [7, 3], [6, 1], [5, 2], [4, 1], [2, 3], [4, 1], [5, 1], [7, 3], [8, 1], [7, 6]];
  const REPLY = [[7, 2], [5, 1], [4, 3], [2, 2], [1, 1], [0, 3], [1, 1], [2, 1], [4, 1], [2, 3], [0, 6]];
  const stomp = [[0, DR.kick2, 96], [0, DR.floorTom, 62], [3, DR.clap, 78], [3, DR.kick2, 54], [5, DR.tamb, 40]];
  const parts = {
    strum: (s) => comp(s, { ch: 3, hits: [[0, 1.8, 1], [2, 0.8, 0.6], [3, 1.8, 0.8], [5, 0.8, 0.6]], center: 57, vel: 62 }),
    tuba: (s) => bass(s, { ch: 2, pattern: [[0, "R", 2, 1], [3, "5", 2, 0.8]], vel: 82, octave: -2 }),
    stomp: (s) => drums(s, { pattern: stomp }),
    lightStomp: (s) => drums(s, { pattern: [[0, DR.kick2, 70], [3, DR.clap, 52]] }),
    callAndReply: (s) => {
      for (let bar = 0; bar < s.bars; bar += 8) {
        line(s, { ch: 0, notes: CALL, vel: 88, at: bar * 6 });
        line(s, { ch: 1, notes: REPLY, octave: -1, vel: 92, at: (bar + 4) * 6 });
      }
    },
    verse: (s, i) => melody(s, { ch: 0, rhythms: SHANTY, range: [0, 8], vel: 84, motifBars: [], start: 4, seed: i }),
    crew: (s) => pad(s, { ch: 1, center: 55, vel: 48, every: 6 }),
    fiddle: (s, i) => melody(s, { ch: 4, rhythms: SHANTY, range: [4, 11], vel: 76, motifBars: [], start: 7, seed: 20 + i }),
    jig: (s) => arp(s, { ch: 4, step: 1, pattern: [0, 1, 2, 3, 2, 1], center: 69, vel: 52 }),
    whistle: (s, i) => melody(s, { ch: 5, rhythms: [[[3], [3]], [[2], [1], [3]]], range: [7, 13], vel: 58, motifBars: [], start: 9, seed: 30 + i }),
  };
  return arrange(song, parts, [
    [8, A, ["strum", "lightStomp", "callAndReply"]],
    [16, A, ["strum", "tuba", "stomp", "verse", "crew"]],
    [16, A, ["strum", "tuba", "stomp", "callAndReply", "jig"]],
    [8, B, ["strum", "fiddle", "crew"]],
    [16, B, ["strum", "tuba", "stomp", "verse", "whistle"]],
    [16, A, ["strum", "tuba", "stomp", "callAndReply", "jig"]],
    [8, A, ["strum", "tuba", "lightStomp", "fiddle"]],
  ]);
}

/** A bard's ballad: harp, recorder and dulcimer over a held open fifth, a frame drum and no kit. */
function tavernBallad() {
  const song = createSong({ id: "tavern-ballad", bpm: 80, key: key("E", "minor"), humanize: 0.018 });
  song.reverb = { room: 0.75, damp: 0.35, width: 0.9, level: 0.6 };
  instrument(song, 0, GM.recorder, { volume: 90, pan: 70, reverb: 60 });
  instrument(song, 1, GM.harp, { volume: 90, pan: 44, reverb: 60 });
  instrument(song, 2, GM.dulcimer, { volume: 80, pan: 88, reverb: 55 });
  instrument(song, 3, GM.cello, { volume: 74, pan: 56, reverb: 60 });
  instrument(song, 4, GM.viola, { volume: 62, pan: 72, reverb: 60 });
  instrument(song, 5, GM.strings2, { volume: 64, pan: 64, reverb: 75 });
  instrument(song, 6, GM.flute, { volume: 70, pan: 84, reverb: 65 });
  instrument(song, DRUMS, KIT.room, { volume: 78, reverb: 50 });
  const A = ["i", "VII", "VI", "VII", "i", "iv", "VII", "i"];
  const B = ["III", "VII", "iv", "i", "VI", "III", "iv", ["v", "i"]];
  const parts = {
    drone: (s) => {
      for (let bar = 0; bar < s.bars; bar += 1) {
        line(s, { ch: 3, notes: [[0, 4]], octave: -2, vel: 54, at: bar * 4, legato: 1 });
        line(s, { ch: 4, notes: [[4, 4]], octave: -1, vel: 40, at: bar * 4, legato: 1 });
      }
    },
    harp: (s) => arp(s, { ch: 1, step: 0.5, pattern: [0, 2, 4, 5, 3, 4, 2, 1], center: 57, vel: 56 }),
    recorder: (s, i) => melody(s, { ch: 0, rhythms: RHYTHMS.folk, range: [2, 10], vel: 80, motifBars: [0, 4], start: 4, seed: i }),
    dulcimer: (s, i) => melody(s, { ch: 2, rhythms: RHYTHMS.folk, range: [4, 11], vel: 72, motifBars: [0], start: 7, seed: 10 + i }),
    flute: (s, i) => melody(s, { ch: 6, rhythms: RHYTHMS.epic, range: [7, 13], vel: 56, motifBars: [], start: 9, seed: 20 + i }),
    strings: (s) => pad(s, { ch: 5, center: 60, vel: 40 }),
    frame: (s) => drums(s, { pattern: [[0, DR.bongoLo, 54], [1.5, DR.bongoHi, 30], [2, DR.bongoLo, 44], [3, DR.bongoHi, 34], [3.5, DR.bongoHi, 26]] }),
  };
  return arrange(song, parts, [
    [4, A, ["drone", "harp"]],
    [8, A, ["drone", "harp", "recorder"]],
    [8, A, ["drone", "harp", "dulcimer", "flute"]],
    [8, B, ["harp", "strings", "recorder"]],
    [8, A, ["drone", "harp", "frame", "recorder", "flute"]],
    [8, B, ["harp", "strings", "frame", "dulcimer"]],
  ]);
}

/** A hoedown reel: fiddle and banjo rolls over boom-chick guitar and spoons, in G major. */
function tavernReel() {
  const song = createSong({ id: "tavern-reel", bpm: 140, key: key("G", "major") });
  song.reverb = { room: 0.45, damp: 0.5, width: 0.8, level: 0.4 };
  instrument(song, 0, GM.fiddle, { volume: 92, pan: 74, reverb: 40 });
  instrument(song, 1, GM.banjo, { volume: 86, pan: 40, reverb: 30 });
  instrument(song, 2, GM.steel, { volume: 80, pan: 30, reverb: 35 });
  instrument(song, 3, GM.acBass, { volume: 88, pan: 64, reverb: 20 });
  instrument(song, 4, GM.whistle, { volume: 80, pan: 90, reverb: 45 });
  instrument(song, 5, P.harmonica, { volume: 78, pan: 56, reverb: 40 });
  instrument(song, DRUMS, KIT.standard, { volume: 84, reverb: 25 });
  const A = ["I", "IV", "I", "V", "I", "IV", ["I", "V"], "I"];
  const B = ["IV", "IV", "I", "I", "V", "V", "I", ["V", "I"]];
  const REEL = [
    [[0.5], [0.5], [0.5], [0.5], [0.5], [0.5], [0.5], [0.5]],
    [[0.5], [0.5], [1], [0.5], [0.5], [1]],
    [[1], [0.5], [0.5], [0.5], [0.5], [1]],
    [[0.5], [0.5], [0.5], [0.5], [2]],
  ];
  const kit = [...at(DR.kick, 84, [0, 2]), ...at(DR.snare, 48, [1, 3]), ...at(DR.claves, 44, [0.5, 1.5, 2.5, 3.5])];
  const parts = {
    banjo: (s) => arp(s, { ch: 1, step: 0.25, pattern: [0, 2, 4, 1, 3, 5, 0, 4], center: 60, vel: 52, accent: [1, 0.7, 0.8, 0.7], gate: 0.5 }),
    loudBanjo: (s) => arp(s, { ch: 1, step: 0.25, pattern: [0, 2, 4, 1, 3, 5, 0, 4], center: 64, vel: 68, accent: [1, 0.7, 0.8, 0.7], gate: 0.5 }),
    boomChick: (s) => {
      bass(s, { ch: 3, pattern: [[0, "R", 0.9, 1], [2, "5", 0.9, 0.9]], vel: 86 });
      comp(s, { ch: 2, hits: [[1, 0.4, 0.8], [3, 0.4, 0.8]], center: 59, vel: 58 });
    },
    kit: (s) => drums(s, { pattern: kit, fill: [...kit, ...at(DR.snare, 60, [3.25, 3.5, 3.75])] }),
    fiddle: (s, i) => melody(s, { ch: 0, rhythms: REEL, range: [4, 12], vel: 84, motifBars: [0, 8], start: 7, seed: i }),
    whistle: (s, i) => melody(s, { ch: 4, rhythms: REEL, range: [7, 14], vel: 76, motifBars: [0], start: 9, seed: 10 + i }),
    harmonica: (s, i) => melody(s, { ch: 5, rhythms: RHYTHMS.folk, range: [2, 9], vel: 70, motifBars: [], start: 4, seed: 20 + i }),
  };
  return arrange(song, parts, [
    [8, A, ["banjo", "boomChick"]],
    [16, A, ["banjo", "boomChick", "kit", "fiddle"]],
    [16, B, ["banjo", "boomChick", "kit", "whistle", "harmonica"]],
    [8, A, ["loudBanjo", "boomChick", "harmonica"]],
    [8, B, ["banjo", "boomChick", "kit", "fiddle"]],
    [16, A, ["banjo", "boomChick", "kit", "fiddle", "whistle"]],
  ]);
}

/** A desert caravanserai: sitar, shanai and an oud-like guitar over darbuka, in E phrygian dominant. */
function tavernCaravan() {
  const song = createSong({ id: "tavern-caravan", bpm: 104, key: key("E", "phrygianDominant"), humanize: 0.015 });
  song.reverb = { room: 0.65, damp: 0.4, width: 0.9, level: 0.5 };
  instrument(song, 0, GM.sitar, { volume: 86, pan: 60, reverb: 50 });
  instrument(song, 1, P.shanai, { volume: 76, pan: 80, reverb: 55 });
  instrument(song, 2, GM.nylon, { volume: 90, pan: 42, reverb: 40 });
  instrument(song, 3, GM.fretless, { volume: 88, pan: 64, reverb: 25 });
  instrument(song, 4, GM.strings, { volume: 70, pan: 64, reverb: 70 });
  instrument(song, 5, GM.dulcimer, { volume: 76, pan: 92, reverb: 55 });
  instrument(song, 6, GM.cello, { volume: 70, pan: 50, reverb: 55 });
  instrument(song, DRUMS, KIT.standard, { volume: 88, reverb: 30 });
  const A = ["I", "I", "II", "I", "iv", "II", "vii", ["II", "I"]];
  const B = ["iv", "iv", "I", "I", "vii", "vii", "II", ["II", "I"]];
  // Maqsum on the darbuka (dum, tek, tek, dum, tek) with ka ornaments, and a riq in eighths.
  const maqsum = [
    [0, DR.congaLo, 88], [0.5, DR.congaHi, 64], [1.5, DR.congaHi, 66], [2, DR.congaLo, 82], [3, DR.congaHi, 68],
    [2.5, DR.congaMute, 38], [3.5, DR.congaMute, 40], [3.75, DR.congaMute, 30],
    ...at(DR.tamb, 26, every(0.5)),
  ];
  const parts = {
    drone: (s) => roots(s, { ch: 6, octave: -2, vel: 50 }),
    oud: (s) => arp(s, { ch: 2, step: 0.25, pattern: [0, null, 0, 1, 2, null, 1, 0], center: 52, vel: 58, gate: 0.6 }),
    bass: (s) => bass(s, { ch: 3, pattern: [[0, "R", 1.4, 1], [1.5, "R", 0.4, 0.7], [2, "5", 1.4, 0.85], [3.5, "A", 0.4, 0.6]], vel: 82 }),
    darbuka: (s) => drums(s, { pattern: maqsum }),
    sitar: (s, i) => melody(s, { ch: 0, rhythms: RHYTHMS.folk, range: [2, 10], vel: 82, motifBars: [0, 4], start: 4, seed: i }),
    shanai: (s, i) => melody(s, { ch: 1, rhythms: RHYTHMS.epic, range: [4, 11], vel: 76, motifBars: [0], start: 7, seed: 10 + i }),
    strings: (s) => pad(s, { ch: 4, center: 60, vel: 44 }),
    qanun: (s) => arp(s, { ch: 5, step: 0.25, pattern: [0, 1, 2, 3, 4, 3, 2, 1], center: 64, vel: 46, gate: 0.5 }),
    oudLead: (s, i) => melody(s, { ch: 2, rhythms: RHYTHMS.folk, range: [0, 8], vel: 78, motifBars: [0, 4], start: 4, seed: 20 + i }),
  };
  return arrange(song, parts, [
    [8, A, ["drone", "oud"]],
    [16, A, ["drone", "oud", "bass", "darbuka", "sitar"]],
    [8, B, ["bass", "darbuka", "strings", "shanai"]],
    [8, A, ["drone", "qanun", "bass", "darbuka", "oudLead"]],
    [8, B, ["bass", "darbuka", "strings", "sitar"]],
    [8, A, ["drone", "oud", "qanun", "bass", "darbuka", "shanai", "strings"]],
  ]);
}

/* ------------------------------------------------------------------------------------------- *
 * EDM
 * ------------------------------------------------------------------------------------------- */

/** Liquid drum & bass: a broken beat at 172, a detuned reese bass, electric piano and pads. */
function edmDnb() {
  const song = createSong({ id: "edm-dnb", bpm: 172, key: key("F", "minor"), humanize: 0.004 });
  song.reverb = { room: 0.6, damp: 0.45, width: 1, level: 0.5 };
  instrument(song, 0, GM.atmosphere, { volume: 70, pan: 64, reverb: 75 });
  instrument(song, 1, GM.saw, { volume: 66, pan: 40, reverb: 15 });
  instrument(song, 2, GM.saw, { volume: 66, pan: 88, reverb: 15 });
  instrument(song, 3, GM.ep2, { volume: 84, pan: 56, reverb: 55 });
  instrument(song, 4, GM.choirPad, { volume: 62, pan: 70, reverb: 75 });
  instrument(song, 5, GM.voiceLead, { volume: 74, pan: 60, reverb: 60 });
  instrument(song, 6, GM.synthBass2, { volume: 82, pan: 64, reverb: 5 });
  instrument(song, DRUMS, KIT.room, { volume: 92, reverb: 25 });
  const A = ["i7", "i7", "VI7", "VI7", "III7", "III7", "VII", "VII"];
  const B = ["iv7", "iv7", "VI7", "VI7", "i7", "i7", "V+", "V+"];
  const brk = [
    [0, DR.kick, 100], [2.5, DR.kick, 92], [1, DR.snare, 96], [3, DR.snare, 98],
    [1.75, DR.snare, 34], [3.25, DR.snare, 30], [3.75, DR.snare, 40],
    ...at(DR.hat, 50, every(0.5)), ...at(DR.pedal, 26, [0.25, 1.25, 2.25, 3.25]), [0, DR.ride, 40], [2, DR.ride, 36],
  ];
  const DNB = [[[1.5], [0.5], [2]], [[1], [0.5], [0.5], [2]], [[3], [1]], [[0.5], [0.5], [1], [1], [1]]];
  const parts = {
    pads: (s) => pad(s, { ch: 0, center: 60, vel: 54 }),
    choir: (s) => pad(s, { ch: 4, center: 64, vel: 48 }),
    keys: (s) => comp(s, { ch: 3, hits: [[0, 1.4, 1], [1.5, 0.4, 0.6], [2.5, 1.4, 0.8]], center: 62, vel: 60 }),
    keyArp: (s) => arp(s, { ch: 3, step: 0.5, pattern: [0, 2, 1, 3, 2, 4, 3, 1], center: 64, vel: 50 }),
    reese: (s) => {
      bend(s, 2, 0, 520);
      bass(s, { ch: 1, pattern: [[0, "R", 3.9, 1]], vel: 74, octave: -2 });
      bass(s, { ch: 2, pattern: [[0, "R", 3.9, 1]], vel: 70, octave: -2 });
      bass(s, { ch: 6, pattern: [[0, "R", 1.4, 1], [2.5, "R", 1.4, 0.9]], vel: 76, octave: -2 });
    },
    break: (s) => drums(s, { pattern: brk, fill: [...brk, ...at(DR.snare, 70, [2.5, 2.75, 3.25, 3.5])] }),
    lead: (s, i) => melody(s, { ch: 5, rhythms: DNB, range: [4, 11], vel: 76, motifBars: [0, 8], start: 7, seed: i }),
    build: (s) => {
      drums(s, { pattern: at(DR.kick, 84, [0]) });
      for (let bar = s.bars - 4; bar < s.bars; bar += 1) roll(s, bar, { n: DR.snare, step: bar === s.bars - 1 ? 0.25 : 0.5, v0: 40 + bar * 3, v1: 60 + bar * 4 });
    },
  };
  return arrange(song, parts, [
    [8, A, ["pads", "keys"]],
    [16, A, ["pads", "keys", "reese", "break", "lead"]],
    [16, A, ["pads", "keyArp", "reese", "break"]],
    [8, B, ["pads", "choir", "keyArp"]],
    [8, B, ["pads", "choir", "keys", "build"]],
    [16, A, ["pads", "keys", "reese", "break", "lead"]],
    [16, B, ["choir", "keyArp", "reese", "break", "lead"]],
  ]);
}

/** Synthwave: gated snare, octave synth-bass eighths, polysynth and an arp under a synth-brass lead. */
function edmSynthwave() {
  const song = createSong({ id: "edm-synthwave", bpm: 104, key: key("E", "minor"), humanize: 0.003 });
  song.reverb = { room: 0.75, damp: 0.35, width: 1, level: 0.6 };
  instrument(song, 0, GM.synthBrass, { volume: 82, pan: 60, reverb: 60 });
  instrument(song, 1, GM.synthBass1, { volume: 82, pan: 64, reverb: 10 });
  instrument(song, 2, GM.polysynth, { volume: 74, pan: 48, reverb: 60 });
  instrument(song, 3, GM.warmPad, { volume: 66, pan: 64, reverb: 70 });
  instrument(song, 4, GM.square, { volume: 60, pan: 84, reverb: 55 });
  instrument(song, 5, GM.synthStrings, { volume: 70, pan: 76, reverb: 70 });
  instrument(song, DRUMS, KIT.power, { volume: 86, reverb: 70 });
  const A = ["i", "VI", "III", "VII", "i", "VI", "iv", "VII"];
  const B = ["VI", "VII", "i", "i", "VI", "VII", "III", "V+"];
  const SYNTHWAVE = [[[1.5], [0.5], [2]], [[1], [1], [1], [1]], [[2], [1], [1]], [[0.5], [0.5], [1], [2]]];
  const beat = [...at(DR.kick, 92, [0, 2, 2.5]), ...at(DR.snare, 96, [1, 3]), ...at(DR.hat, 46, every(0.5))];
  const parts = {
    pad: (s) => pad(s, { ch: 3, center: 60, vel: 52 }),
    arp: (s) => arp(s, { ch: 4, step: 0.25, pattern: [0, 1, 2, 3, 4, 3, 2, 1], center: 64, vel: 46, gate: 0.5 }),
    bass: (s) => bass(s, { ch: 1, pattern: every(0.5).map((t, i) => [t, i % 2 === 0 ? "R" : "8", 0.4, i % 2 === 0 ? 1 : 0.8]), vel: 84, octave: -2 }),
    chords: (s) => comp(s, { ch: 2, hits: [[0, 1.9, 1], [2, 1.9, 0.85]], center: 60, vel: 60 }),
    drums: (s) => drums(s, { pattern: beat, fill: [...beat, ...at(DR.tomLo, 80, [3, 3.25]), ...at(DR.floorTom, 86, [3.5, 3.75])] }),
    lead: (s, i) => melody(s, { ch: 0, rhythms: SYNTHWAVE, range: [3, 11], vel: 80, motifBars: [0, 4], start: 4, seed: i }),
    strings: (s, i) => melody(s, { ch: 5, rhythms: RHYTHMS.epic, range: [4, 11], vel: 72, motifBars: [0], start: 7, seed: 10 + i }),
    polyArp: (s) => arp(s, { ch: 2, step: 0.5, pattern: [0, 2, 4, 2, 1, 3, 5, 3], center: 60, vel: 52 }),
  };
  return arrange(song, parts, [
    [8, A, ["pad", "arp", "bass"]],
    [16, A, ["pad", "arp", "bass", "drums", "lead"]],
    [8, B, ["bass", "chords", "drums", "strings"]],
    [8, A, ["pad", "polyArp", "bass", "drums"]],
    [8, B, ["arp", "bass", "chords", "drums", "lead"]],
    [8, A, ["pad", "arp", "bass", "chords", "drums", "lead", "strings"]],
  ]);
}

/** Halftime bass music: wobbling growl bass, a sub, a snare on three, bells and orchestra hits. */
function edmHalftime() {
  const song = createSong({ id: "edm-halftime", bpm: 140, key: key("C", "minor"), humanize: 0.003 });
  song.reverb = { room: 0.55, damp: 0.5, width: 1, level: 0.45 };
  instrument(song, 0, GM.saw, { volume: 74, pan: 64, reverb: 55 });
  instrument(song, 1, GM.bassLead, { volume: 84, pan: 64, reverb: 10 });
  instrument(song, 2, GM.synthBass1, { volume: 86, pan: 64, reverb: 5 });
  instrument(song, 3, GM.sweepPad, { volume: 70, pan: 64, reverb: 75 });
  instrument(song, 4, GM.choirPad, { volume: 60, pan: 50, reverb: 75 });
  instrument(song, 5, GM.bells, { volume: 76, pan: 80, reverb: 70 });
  instrument(song, 6, GM.orchHit, { volume: 72, pan: 64, reverb: 50 });
  instrument(song, DRUMS, KIT.electronic, { volume: 88, reverb: 25 });
  const A = ["i", "i", "VI", "VI", "iv", "iv", "V+", "V+"];
  const B = ["VI", "VI", "VII", "VII", "i", "i", "V+", "V+"];
  const halftime = [...at(DR.kick, 100, [0, 1.75, 2.75]), [2, DR.snare, 104], [2, DR.clap, 70], ...at(DR.hat, 44, every(0.5)), [3.75, DR.hat, 56]];
  // The wobble's speed changes from bar to bar: quarters, eighths, triplets, sixteenths.
  const rates = [1, 0.5, 1 / 3, 0.5, 1, 0.25, 0.5, 1 / 3];
  const parts = {
    pad: (s) => pad(s, { ch: 3, center: 60, vel: 54 }),
    choir: (s) => pad(s, { ch: 4, center: 64, vel: 46 }),
    bells: (s, i) => melody(s, { ch: 5, rhythms: RHYTHMS.epic, range: [4, 11], vel: 70, motifBars: [0, 4], start: 7, seed: i }),
    hats: (s) => drums(s, { pattern: at(DR.hat, 40, every(0.5)) }),
    build: (s) => {
      for (let bar = 0; bar < s.bars; bar += 1) roll(s, bar, { n: DR.snare, step: bar < 4 ? 1 : bar < 6 ? 0.5 : bar < 7 ? 0.25 : 0.125, v0: 40 + bar * 6, v1: 50 + bar * 8 });
      swell(s, { ch: 3, lo: 30 });
    },
    drop: (s) => {
      drums(s, { pattern: halftime, fill: [...halftime, ...at(DR.snare, 80, [3, 3.25, 3.5])] });
      roots(s, { ch: 2, octave: -2, vel: 78 });
      bass(s, { ch: 1, pattern: [[0, "R", 3.9, 1]], vel: 82, octave: -1 });
      for (let bar = 0; bar < s.bars; bar += 1) wobble(s, { ch: 1, cycle: rates[bar % rates.length], from: bar * 4, to: bar * 4 + 4 });
      comp(s, { ch: 6, hits: [[0, 0.5, 1]], center: 60, vel: 66 });
    },
    lead: (s, i) => melody(s, { ch: 0, rhythms: RHYTHMS.edm, range: [4, 11], vel: 74, motifBars: [0, 4], start: 7, seed: 10 + i }),
  };
  return arrange(song, parts, [
    [8, A, ["pad", "choir", "bells", "hats"]],
    [8, A, ["pad", "choir", "build"]],
    [16, A, ["drop", "lead"]],
    [8, B, ["pad", "choir", "bells"]],
    [8, B, ["pad", "build"]],
    [16, B, ["drop", "choir"]],
    [8, A, ["pad", "bells", "hats"]],
  ]);
}

/** Chiptune: square-wave lead and fast chord arps, a square bass and noise drums, in C major. */
function edmChip() {
  const song = createSong({ id: "edm-chip", bpm: 150, key: key("C", "major"), humanize: 0 });
  song.reverb = { room: 0.25, damp: 0.6, width: 0.6, level: 0.2 };
  instrument(song, 0, GM.square, { volume: 80, pan: 58, reverb: 20 });
  instrument(song, 1, GM.square, { volume: 62, pan: 80, reverb: 20 });
  instrument(song, 2, GM.calliope, { volume: 70, pan: 44, reverb: 20 });
  instrument(song, 3, GM.square, { volume: 78, pan: 64, reverb: 5 });
  instrument(song, DRUMS, KIT.electronic, { volume: 80, reverb: 10 });
  const A = ["I", "V", "vi", "IV", "I", "V", "IV", ["ii", "V"]];
  const B = ["vi", "IV", "I", "V", "vi", "IV", "ii", "V"];
  const C = ["vi", "iii", "IV", "I", "ii", "vi", "IV", "V"];
  const CHIP = [
    [[0.25], [0.25], [0.25], [0.25], [0.5], [0.5], [1], [1]],
    [[0.5], [0.5], [0.5], [0.5], [1], [0.5], [0.5]],
    [[0.75], [0.75], [0.5], [0.5], [0.5], [1]],
    [[1.5], [0.5], [0.5], [0.5], [1]],
  ];
  const beat = [...at(DR.kick, 90, [0, 2]), ...at(DR.esnare, 80, [1, 3]), ...at(DR.hat, 50, every(0.5))];
  const parts = {
    vibrato: (s) => cc(s, 0, 0, 1, 24),
    arp: (s) => arp(s, { ch: 1, step: 0.125, pattern: [0, 1, 2, 3], center: 67, vel: 50, gate: 0.8 }),
    bass: (s) => bass(s, { ch: 3, pattern: [[0, "R", 0.4, 1], [0.5, "8", 0.4, 0.8], [1, "R", 0.4, 0.9], [1.5, "8", 0.4, 0.8], [2, "5", 0.4, 0.9], [2.5, "8", 0.4, 0.8], [3, "R", 0.4, 0.9], [3.5, "A", 0.4, 0.8]], vel: 82, octave: -2 }),
    drums: (s) => drums(s, { pattern: beat, fill: [...at(DR.kick, 90, [0, 2]), ...at(DR.esnare, 84, [1, 2.5, 3, 3.25, 3.5, 3.75])] }),
    lead: (s, i) => melody(s, { ch: 0, rhythms: CHIP, range: [4, 12], vel: 82, motifBars: [0, 4, 8], start: 7, seed: i }),
    harmony: (s, i) => melody(s, { ch: 2, rhythms: RHYTHMS.edm, range: [2, 9], vel: 66, motifBars: [], start: 4, seed: 10 + i }),
    hats: (s) => drums(s, { pattern: [...at(DR.hat, 44, every(0.5)), [0, DR.kick, 70]] }),
  };
  return arrange(song, parts, [
    [8, A, ["vibrato", "arp", "bass"]],
    [16, A, ["arp", "bass", "drums", "lead"]],
    [16, B, ["arp", "bass", "drums", "lead", "harmony"]],
    [8, C, ["arp", "bass", "harmony", "hats"]],
    [16, A, ["arp", "bass", "drums", "lead", "harmony"]],
    [16, B, ["arp", "bass", "drums", "lead"]],
  ]);
}

/** Orchestral trap: 808 kit with hat rolls, gliding 808 bass, flute and bells, pizzicato and choir. */
function edmTrap() {
  const song = createSong({ id: "edm-trap", bpm: 140, key: key("G", "harmonic"), humanize: 0.004 });
  song.reverb = { room: 0.6, damp: 0.45, width: 1, level: 0.5 };
  instrument(song, 0, GM.flute, { volume: 86, pan: 60, reverb: 55 });
  instrument(song, 1, GM.bells, { volume: 72, pan: 82, reverb: 70 });
  instrument(song, 2, GM.synthBass1, { volume: 92, pan: 64, reverb: 5 });
  instrument(song, 3, GM.choirPad, { volume: 66, pan: 64, reverb: 75 });
  instrument(song, 4, GM.pizz, { volume: 84, pan: 40, reverb: 50 });
  instrument(song, 5, GM.brass, { volume: 70, pan: 56, reverb: 60 });
  instrument(song, DRUMS, KIT.tr808, { volume: 92, reverb: 20 });
  const A = ["i", "VI", "iv", "V+", "i", "VI", "III", "V+"];
  const B = ["VI", "VI", "iv", "iv", "i", "i", "V+", "V+"];
  const hats = (bar) => {
    const base = at(DR.hat, 50, every(0.5));
    if (bar % 2 === 1) base.push(...at(DR.hat, 40, [2.25, 2.75]), ...[3, 3 + 1 / 6, 3 + 2 / 6, 3.5, 3.5 + 1 / 6, 3.5 + 2 / 6].map((t) => [t, DR.hat, 36]));
    return base;
  };
  const kit = [...at(DR.kick, 104, [0, 1.75, 2.75]), [2, DR.clap, 96], [2, DR.snare, 70]];
  const parts = {
    pizz: (s) => arp(s, { ch: 4, step: 0.5, pattern: [0, 2, 1, 3, 0, 2, 4, 2], center: 60, vel: 60, gate: 0.5 }),
    bells: (s, i) => melody(s, { ch: 1, rhythms: RHYTHMS.epic, range: [7, 13], vel: 66, motifBars: [0, 4], start: 9, seed: i }),
    choir: (s) => pad(s, { ch: 3, center: 62, vel: 50 }),
    drums: (s) => {
      for (let bar = 0; bar < s.bars; bar += 1) drums(s, { pattern: [...kit, ...hats(bar)], from: bar, skip: (b) => b !== bar });
    },
    eight08: (s) => {
      bass(s, { ch: 2, pattern: [[0, "R", 1.6, 1], [1.75, "R", 0.9, 0.8], [2.75, "R", 1.1, 0.9]], vel: 100, octave: -2 });
      // A glide down into each second bar's last note, and back.
      for (let bar = 1; bar < s.bars; bar += 2) {
        bend(s, 2, bar * 4 + 3.2, -4096);
        bend(s, 2, bar * 4 + 3.95, 0);
      }
    },
    flute: (s, i) => melody(s, { ch: 0, rhythms: RHYTHMS.edm, range: [3, 10], vel: 82, motifBars: [0, 4], start: 4, seed: 10 + i }),
    brass: (s) => comp(s, { ch: 5, hits: [[0, 0.6, 1], [2.5, 0.4, 0.8]], center: 55, vel: 60 }),
  };
  return arrange(song, parts, [
    [8, A, ["pizz", "choir", "bells", "eight08"]],
    [16, A, ["pizz", "drums", "eight08", "flute"]],
    [8, B, ["choir", "flute", "bells"]],
    [16, A, ["pizz", "drums", "eight08", "flute", "brass"]],
    [8, B, ["pizz", "bells", "eight08"]],
    [16, A, ["pizz", "choir", "drums", "eight08", "flute", "brass"]],
  ]);
}

/* ------------------------------------------------------------------------------------------- *
 * Lo-fi
 * ------------------------------------------------------------------------------------------- */

const lofiPost = (over) => ({ lowpassHz: 4200, drive: 1.6, wowPeriodS: 4.5, wowDepthS: 0.0011, crackle: 0.05, clicksPerS: 3, hiss: 0.02, ...over });

/** Jazzhop: acoustic piano voicings with ninths, a walking upright, muted trumpet and tenor sax. */
function lofiJazzhop() {
  const song = createSong({ id: "lofi-jazzhop", bpm: 88, key: key("Eb", "major"), swing: 0.12, humanize: 0.02 });
  song.reverb = { room: 0.5, damp: 0.55, width: 0.75, level: 0.4 };
  song.post.lofi = lofiPost({ lowpassHz: 5200, crackle: 0.06, clicksPerS: 4, seed: 7103 });
  instrument(song, 0, GM.piano, { volume: 90, pan: 56, reverb: 45 });
  instrument(song, 1, GM.mutedTrumpet, { volume: 84, pan: 72, reverb: 50 });
  instrument(song, 2, GM.acBass, { volume: 90, pan: 64, reverb: 20 });
  instrument(song, 3, P.tenorSax, { volume: 80, pan: 44, reverb: 50 });
  instrument(song, 4, GM.strings2, { volume: 54, pan: 64, reverb: 70 });
  instrument(song, DRUMS, KIT.jazz, { volume: 88, reverb: 25 });
  const A = ["ii79", "V79", "I79", "vi79", "ii79", "V79", "iii7", "vi7"];
  const B = ["IV79", "vii7", "iii79", "vi79", "ii79", "V79", "I79", ["ii7", "V7"]];
  const beat = [...at(DR.kick, 80, [0, 2.5]), [1.75, DR.kick, 50], ...at(DR.stick, 62, [1, 3]), ...at(DR.ride, 40, every(0.5)), [1, DR.pedal, 34], [3, DR.pedal, 34]];
  const parts = {
    piano: (s) => comp(s, { ch: 0, hits: [[0, 1.4, 1], [1.5, 0.4, 0.6], [2.5, 1.4, 0.85]], center: 60, vel: 58 }),
    walk: (s) => bass(s, { ch: 2, pattern: [[0, "R", 0.9, 1], [1, "3", 0.9, 0.8], [2, "5", 0.9, 0.9], [3, "A", 0.9, 0.8]], vel: 84 }),
    drums: (s) => drums(s, { pattern: beat }),
    trumpet: (s, i) => melody(s, { ch: 1, rhythms: RHYTHMS.lofi, range: [2, 10], vel: 74, motifBars: [0, 4], start: 4, seed: i }),
    sax: (s, i) => melody(s, { ch: 3, rhythms: RHYTHMS.lofi, range: [-1, 7], vel: 72, motifBars: [0], start: 2, seed: 10 + i }),
    pianoSolo: (s, i) => melody(s, { ch: 0, rhythms: RHYTHMS.edm, range: [7, 14], vel: 62, motifBars: [0], start: 9, seed: 20 + i }),
    strings: (s) => pad(s, { ch: 4, center: 62, vel: 36 }),
  };
  return arrange(song, parts, [
    [4, A, ["piano", "walk", "strings"]],
    [8, A, ["piano", "walk", "drums"]],
    [8, A, ["piano", "walk", "drums", "trumpet"]],
    [8, B, ["piano", "walk", "drums", "sax", "strings"]],
    [8, A, ["piano", "walk", "drums", "trumpet", "sax"]],
    [8, B, ["walk", "drums", "pianoSolo", "strings"]],
    [4, A, ["piano", "walk"]],
  ]);
}

/** Bossa lo-fi: straight eighths, nylon guitar on the bossa rhythm, rim clave, flute and clarinet. */
function lofiBossa() {
  const song = createSong({ id: "lofi-bossa", bpm: 96, key: key("G", "major"), humanize: 0.016 });
  song.reverb = { room: 0.5, damp: 0.5, width: 0.8, level: 0.42 };
  song.post.lofi = lofiPost({ lowpassHz: 4600, crackle: 0.04, seed: 7211 });
  instrument(song, 0, GM.nylon, { volume: 92, pan: 46, reverb: 40 });
  instrument(song, 1, GM.flute, { volume: 84, pan: 74, reverb: 55 });
  instrument(song, 2, GM.acBass, { volume: 86, pan: 64, reverb: 20 });
  instrument(song, 3, GM.clarinet, { volume: 78, pan: 58, reverb: 50 });
  instrument(song, 4, GM.strings2, { volume: 56, pan: 64, reverb: 70 });
  instrument(song, DRUMS, KIT.brush, { volume: 88, reverb: 25 });
  const A = ["I79", "IV79", "iii79", "vi79", "ii79", "V79", "I79", ["ii7", "V7"]];
  const B = ["IV79", "IV79", "iii79", "vi79", "ii79", "V79", "iii7", ["vi7", "V7"]];
  const clave = [...at(DR.stick, 58, [0, 0.75, 1.5, 2.5, 3]), ...at(DR.shaker, 26, every(0.25)), ...at(DR.kick, 56, [0, 1.5, 2, 3.5])];
  const parts = {
    guitar: (s) => comp(s, { ch: 0, hits: [[0, 0.45, 1], [0.75, 0.45, 0.8], [1.5, 0.45, 0.85], [2.5, 0.45, 0.8], [3, 0.45, 0.9]], center: 57, vel: 60 }),
    guitarArp: (s) => arp(s, { ch: 0, step: 0.5, pattern: [0, 2, 1, 3, 2, 4, 3, 1], center: 60, vel: 54 }),
    bass: (s) => bass(s, { ch: 2, pattern: [[0, "R", 1.4, 1], [1.5, "5", 0.5, 0.8], [2, "5", 1.4, 0.9], [3.5, "R", 0.5, 0.8]], vel: 82 }),
    clave: (s) => drums(s, { pattern: clave }),
    flute: (s, i) => melody(s, { ch: 1, rhythms: RHYTHMS.lofi, range: [4, 11], vel: 76, motifBars: [0, 4], start: 7, seed: i }),
    clarinet: (s, i) => melody(s, { ch: 3, rhythms: RHYTHMS.lofi, range: [0, 8], vel: 72, motifBars: [0], start: 4, seed: 10 + i }),
    strings: (s) => pad(s, { ch: 4, center: 62, vel: 34 }),
  };
  return arrange(song, parts, [
    [4, A, ["guitar"]],
    [8, A, ["guitar", "bass", "clave"]],
    [8, A, ["guitar", "bass", "clave", "flute"]],
    [8, B, ["guitar", "bass", "clave", "clarinet", "strings"]],
    [8, A, ["guitar", "bass", "clave", "flute", "clarinet"]],
    [8, B, ["guitarArp", "bass", "clave", "flute", "strings"]],
    [8, A, ["guitar", "bass", "clave", "clarinet"]],
  ]);
}

/** Ambient rain: kalimba, music box and celesta over warm pads, mostly drumless, with heavy hiss. */
function lofiRain() {
  const song = createSong({ id: "lofi-rain", bpm: 70, key: key("Db", "lydian"), swing: 0.06, humanize: 0.02 });
  song.reverb = { room: 0.8, damp: 0.4, width: 1, level: 0.6 };
  song.post.lofi = lofiPost({ lowpassHz: 3800, drive: 1.3, wowPeriodS: 6, wowDepthS: 0.0016, crackle: 0.035, clicksPerS: 10, hiss: 0.045, seed: 7307 });
  instrument(song, 0, GM.kalimba, { volume: 92, pan: 56, reverb: 60 });
  instrument(song, 1, GM.musicBox, { volume: 74, pan: 80, reverb: 70 });
  instrument(song, 2, GM.warmPad, { volume: 72, pan: 64, reverb: 75 });
  instrument(song, 3, GM.haloPad, { volume: 56, pan: 50, reverb: 80 });
  instrument(song, 4, GM.fretless, { volume: 76, pan: 64, reverb: 30 });
  instrument(song, 5, GM.celesta, { volume: 70, pan: 36, reverb: 70 });
  instrument(song, DRUMS, KIT.brush, { volume: 74, reverb: 40 });
  const A = ["I7", "I7", "II", "II", "vi7", "vi7", "iii7", "II"];
  const B = ["vi7", "vi7", "vii7", "vii7", "I7", "I7", "II", "II"];
  const parts = {
    pads: (s) => {
      pad(s, { ch: 2, center: 60, vel: 54 });
      pad(s, { ch: 3, center: 67, vel: 40 });
    },
    bass: (s) => bass(s, { ch: 4, pattern: [[0, "R", 3.8, 1]], vel: 66 }),
    kalimba: (s, i) => melody(s, { ch: 0, rhythms: RHYTHMS.lofi, range: [2, 10], vel: 74, motifBars: [0, 4], start: 4, seed: i }),
    musicBox: (s) => arp(s, { ch: 1, step: 0.5, pattern: [0, 2, 4, null, 3, null, 1, null], center: 72, vel: 46, gate: 0.8 }),
    celesta: (s, i) => melody(s, { ch: 5, rhythms: RHYTHMS.lofi, range: [7, 13], vel: 60, motifBars: [], start: 9, seed: 10 + i }),
    soft: (s) => drums(s, { pattern: [[0, DR.kick, 52], [2.5, DR.kick, 40], [1, DR.stick, 36], [3, DR.stick, 38], ...at(DR.shaker, 22, every(0.5))] }),
  };
  return arrange(song, parts, [
    [4, A, ["pads", "musicBox"]],
    [8, A, ["pads", "bass", "kalimba"]],
    [8, B, ["pads", "bass", "musicBox", "celesta"]],
    [8, A, ["pads", "bass", "kalimba", "soft"]],
    [8, B, ["pads", "musicBox", "celesta", "soft"]],
  ]);
}

/** Dusty boom-bap: chopped organ stabs, a hard swung kick and snare, jazz guitar licks and alto sax. */
function lofiBoombap() {
  const song = createSong({ id: "lofi-boombap", bpm: 86, key: key("E", "dorian"), swing: 0.15, humanize: 0.012 });
  song.reverb = { room: 0.35, damp: 0.6, width: 0.6, level: 0.3 };
  song.post.lofi = lofiPost({ lowpassHz: 3600, drive: 2.2, crackle: 0.07, clicksPerS: 5, hiss: 0.025, seed: 7411 });
  instrument(song, 0, P.percOrgan, { volume: 84, pan: 56, reverb: 30 });
  instrument(song, 1, GM.jazzGuitar, { volume: 82, pan: 34, reverb: 40 });
  instrument(song, 2, GM.fingerBass, { volume: 92, pan: 64, reverb: 10 });
  instrument(song, 3, GM.strings, { volume: 66, pan: 72, reverb: 40 });
  instrument(song, 4, P.altoSax, { volume: 80, pan: 76, reverb: 45 });
  instrument(song, 5, GM.brass, { volume: 64, pan: 60, reverb: 40 });
  instrument(song, DRUMS, KIT.room, { volume: 96, reverb: 20 });
  const A = ["i7", "i7", "IV7", "IV7", "i7", "i7", "v7", ["IV7", "III"]];
  const B = ["III7", "III7", "IV7", "IV7", "ii7", "ii7", "v7", "v7"];
  const boom = [...at(DR.kick, 104, [0, 0.75, 2.5]), [1.75, DR.kick, 60], ...at(DR.snare, 100, [1, 3]), ...at(DR.hat, 50, every(0.5)), [3.75, DR.hat, 34]];
  const parts = {
    chops: (s) => comp(s, { ch: 0, hits: [[0, 0.3, 1], [0.75, 0.2, 0.8], [2.25, 0.3, 0.9], [3.5, 0.2, 0.7]], center: 62, vel: 66 }),
    stringChops: (s) => comp(s, { ch: 3, hits: [[0, 0.25, 1], [1.5, 0.25, 0.8]], center: 60, vel: 58 }),
    horns: (s) => comp(s, { ch: 5, hits: [[3.5, 0.3, 1]], center: 58, vel: 58 }),
    bass: (s) => bass(s, { ch: 2, pattern: [[0, "R", 0.6, 1], [0.75, "R", 0.4, 0.8], [2.5, "5", 0.6, 0.9], [3.5, "A", 0.4, 0.7]], vel: 90 }),
    boom: (s) => drums(s, { pattern: boom }),
    sax: (s, i) => melody(s, { ch: 4, rhythms: RHYTHMS.lofi, range: [2, 10], vel: 76, motifBars: [0, 4], start: 4, seed: i }),
    guitar: (s, i) => melody(s, { ch: 1, rhythms: RHYTHMS.edm, range: [0, 8], vel: 72, motifBars: [0], start: 4, seed: 10 + i }),
  };
  return arrange(song, parts, [
    [4, A, ["chops"]],
    [8, A, ["chops", "bass", "boom"]],
    [8, A, ["chops", "bass", "boom", "sax"]],
    [8, B, ["stringChops", "bass", "boom", "guitar"]],
    [8, A, ["chops", "horns", "bass", "boom", "sax"]],
    [8, B, ["chops", "stringChops", "bass", "boom", "guitar"]],
  ]);
}

/** Vapor dream: FM electric piano and synth pads, slow and dark under a deep, slow tape wow. */
function lofiVapor() {
  const song = createSong({ id: "lofi-vapor", bpm: 78, key: key("Ab", "major"), swing: 0.05, humanize: 0.015 });
  song.reverb = { room: 0.85, damp: 0.3, width: 1, level: 0.65 };
  song.post.lofi = lofiPost({ lowpassHz: 3000, drive: 1.3, wowPeriodS: 6.15, wowDepthS: 0.0022, crackle: 0.02, clicksPerS: 2, hiss: 0.015, seed: 7523 });
  instrument(song, 0, GM.ep2, { volume: 90, pan: 56, reverb: 60 });
  instrument(song, 1, GM.synthStrings, { volume: 66, pan: 64, reverb: 75 });
  instrument(song, 2, GM.fretless, { volume: 84, pan: 64, reverb: 30 });
  instrument(song, 3, GM.sweepPad, { volume: 60, pan: 50, reverb: 80 });
  instrument(song, 4, GM.crystal, { volume: 76, pan: 78, reverb: 70 });
  instrument(song, 5, GM.synthVoice, { volume: 66, pan: 40, reverb: 75 });
  instrument(song, DRUMS, KIT.electronic, { volume: 80, reverb: 35 });
  const A = ["I7", "iii7", "vi7", "IV7", "I7", "iii7", "ii7", "V7sus4"];
  const B = ["IV7", "IV7", "iii7", "vi7", "ii7", "V7", "I7", "I7"];
  const slow = [[0, DR.kick, 80], [2.5, DR.kick, 64], [2, DR.snare, 70], ...at(DR.hat, 36, every(0.5))];
  const parts = {
    keys: (s) => comp(s, { ch: 0, hits: [[0, 2.4, 1], [2.5, 1.4, 0.7]], center: 62, vel: 60 }),
    strings: (s) => pad(s, { ch: 1, center: 60, vel: 48 }),
    sweep: (s) => pad(s, { ch: 3, center: 67, vel: 40 }),
    bass: (s) => bass(s, { ch: 2, pattern: [[0, "R", 2.4, 1], [2.5, "5", 1.4, 0.8]], vel: 80 }),
    drums: (s) => drums(s, { pattern: slow }),
    crystal: (s, i) => melody(s, { ch: 4, rhythms: RHYTHMS.lofi, range: [4, 11], vel: 72, motifBars: [0, 4], start: 7, seed: i }),
    voice: (s, i) => melody(s, { ch: 5, rhythms: RHYTHMS.epic, range: [2, 9], vel: 64, motifBars: [0], start: 4, seed: 10 + i }),
  };
  return arrange(song, parts, [
    [8, A, ["keys", "sweep", "bass"]],
    [8, A, ["keys", "strings", "bass", "drums", "crystal"]],
    [8, B, ["keys", "sweep", "bass", "drums", "voice"]],
    [8, A, ["keys", "strings", "bass", "drums", "crystal", "voice"]],
    [8, B, ["keys", "strings", "sweep", "bass", "crystal"]],
  ]);
}

/* ------------------------------------------------------------------------------------------- *
 * Epic orchestral
 * ------------------------------------------------------------------------------------------- */

/** A heroic adventure in F lydian: trumpet and horns over driving strings, a march snare, glockenspiel. */
function epicHeroic() {
  const song = createSong({ id: "epic-heroic", bpm: 120, key: key("F", "lydian") });
  song.reverb = { room: 0.8, damp: 0.3, width: 1, level: 0.6 };
  instrument(song, 0, GM.trumpet, { volume: 88, pan: 60, reverb: 60 });
  instrument(song, 1, GM.horn, { volume: 86, pan: 46, reverb: 65 });
  instrument(song, 2, GM.strings, { volume: 80, pan: 64, reverb: 60 });
  instrument(song, 3, GM.brass, { volume: 70, pan: 56, reverb: 65 });
  instrument(song, 4, GM.timpani, { volume: 84, pan: 64, reverb: 60 });
  instrument(song, 5, GM.contrabass, { volume: 82, pan: 60, reverb: 50 });
  instrument(song, 6, GM.glock, { volume: 66, pan: 90, reverb: 60 });
  instrument(song, 7, GM.violin, { volume: 80, pan: 76, reverb: 60 });
  instrument(song, DRUMS, KIT.orchestra, { volume: 80, reverb: 55 });
  const A = ["I", "II", "I", "II", "vi", "V", "I", "V"];
  const B = ["vi", "iii", "II", "V", "vi", "iii", "II", ["Vsus4", "V"]];
  const march = [
    [0, DR.snare, 70], [0.75, DR.snare, 40], [1, DR.snare, 54], [2, DR.snare, 70], [2.5, DR.snare, 44], [2.75, DR.snare, 44],
    [3, DR.snare, 60], [3.5, DR.snare, 40], [0, DR.kick2, 70], [2, DR.kick2, 60],
  ];
  const parts = {
    fanfare: (s) => {
      comp(s, { ch: 3, hits: [[0, 0.9, 1], [1, 0.4, 0.7], [1.5, 0.4, 0.7], [2, 1.9, 0.9]], center: 57, vel: 70 });
      roots(s, { ch: 4, octave: -2, vel: 76, hits: [[0, 0.5, 1], [3.5, 0.25, 0.6]] });
      line(s, { ch: 0, notes: [...FANFARE, [0, 4], ...FANFARE.map(([d, b]) => [d === null ? null : d + 2, b]), [2, 4]], vel: 84 });
    },
    ostinato: (s) => arp(s, { ch: 2, step: 0.5, pattern: [0, 2, 1, 2, 0, 2, 1, 3], center: 52, vel: 54, gate: 0.55 }),
    low: (s) => bass(s, { ch: 5, pattern: [[0, "R", 0.9, 1], [1, "R", 0.4, 0.7], [2, "5", 0.9, 0.9], [3, "R", 0.9, 0.8]], vel: 76, octave: -2 }),
    march: (s) => drums(s, { pattern: march, fill: [...march, ...at(DR.snare, 66, [3.25, 3.75]), [0, DR.crash, 50]] }),
    trumpet: (s, i) => melody(s, { ch: 0, rhythms: RHYTHMS.epic, range: [2, 10], vel: 84, motifBars: [0, 4, 8], start: 4, seed: i }),
    horn: (s, i) => melody(s, { ch: 1, rhythms: RHYTHMS.epic, range: [0, 8], vel: 80, motifBars: [0, 4], start: 4, seed: 10 + i }),
    violin: (s, i) => melody(s, { ch: 7, rhythms: RHYTHMS.folk, range: [7, 14], vel: 70, motifBars: [], start: 9, seed: 20 + i }),
    brassPad: (s) => pad(s, { ch: 3, center: 57, vel: 52 }),
    glock: (s) => arp(s, { ch: 6, step: 1, pattern: [3, 4, 5, 4], center: 72, vel: 44 }),
    timp: (s) => roots(s, { ch: 4, octave: -2, vel: 70, hits: [[0, 1, 1]] }),
  };
  return arrange(song, parts, [
    [8, A, ["fanfare", "ostinato"]],
    [16, A, ["ostinato", "low", "march", "trumpet", "glock"]],
    [8, B, ["ostinato", "low", "horn", "brassPad", "timp"]],
    [8, A, ["ostinato", "low", "violin", "march"]],
    [8, B, ["ostinato", "low", "trumpet", "horn", "timp"]],
    [16, A, ["ostinato", "low", "march", "trumpet", "violin", "brassPad", "glock", "timp"]],
  ]);
}

/** A dark requiem in C phrygian: choir-led, church organ, low brass, tolling bells. */
function epicRequiem() {
  const song = createSong({ id: "epic-requiem", bpm: 72, key: key("C", "phrygian") });
  song.reverb = { room: 0.9, damp: 0.25, width: 1, level: 0.7 };
  instrument(song, 0, GM.choir, { volume: 92, pan: 60, reverb: 80 });
  instrument(song, 1, P.churchOrgan, { volume: 70, pan: 64, reverb: 75 });
  instrument(song, 2, GM.trombone, { volume: 82, pan: 48, reverb: 65 });
  instrument(song, 3, GM.cello, { volume: 82, pan: 72, reverb: 60 });
  instrument(song, 4, GM.contrabass, { volume: 82, pan: 60, reverb: 55 });
  instrument(song, 5, GM.bells, { volume: 70, pan: 84, reverb: 80 });
  instrument(song, 6, GM.timpani, { volume: 86, pan: 64, reverb: 70 });
  instrument(song, 7, GM.oohs, { volume: 70, pan: 40, reverb: 80 });
  instrument(song, 8, GM.tuba, { volume: 76, pan: 56, reverb: 60 });
  instrument(song, DRUMS, KIT.orchestra, { volume: 78, reverb: 70 });
  const A = ["i", "II", "i", "vii", "i", "VI", "II", "i"];
  const B = ["iv", "VI", "III", "II", "iv", "vii", "II", ["II", "i"]];
  const parts = {
    organ: (s) => pad(s, { ch: 1, center: 57, vel: 52 }),
    bells: (s) => roots(s, { ch: 5, octave: 0, vel: 60, hits: [[0, 3, 1]] }),
    low: (s) => bass(s, { ch: 4, pattern: [[0, "R", 3.8, 1]], vel: 76, octave: -2 }),
    choir: (s, i) => melody(s, { ch: 0, rhythms: RHYTHMS.epic, range: [-1, 7], vel: 84, motifBars: [0, 4], start: 2, seed: i }),
    oohs: (s) => pad(s, { ch: 7, center: 55, vel: 48 }),
    brass: (s, i) => melody(s, { ch: 2, rhythms: RHYTHMS.epic, range: [-3, 4], vel: 80, motifBars: [0], start: 0, seed: 10 + i }),
    tuba: (s) => bass(s, { ch: 8, pattern: [[0, "R", 1.9, 1], [2, "5", 1.9, 0.8]], vel: 70, octave: -2 }),
    cello: (s) => arp(s, { ch: 3, step: 0.5, pattern: [0, 1, 2, 1, 0, 1, 2, 3], center: 48, vel: 54, gate: 0.8 }),
    timp: (s) => {
      roots(s, { ch: 6, octave: -2, vel: 74, hits: [[0, 1, 1], [3, 0.25, 0.5], [3.5, 0.25, 0.7]] });
      drums(s, { pattern: [[0, DR.kick2, 66]], skip: (bar) => bar % 2 === 1 });
    },
  };
  return arrange(song, parts, [
    [4, A, ["organ", "oohs", "bells", "low"]],
    [8, A, ["organ", "low", "choir", "bells"]],
    [8, B, ["organ", "low", "brass", "tuba"]],
    [8, A, ["oohs", "low", "cello", "choir"]],
    [8, B, ["organ", "oohs", "low", "cello", "choir", "brass", "timp"]],
    [4, A, ["organ", "bells", "low"]],
  ]);
}

/** A battle in 5/4 (3 + 2): taiko and floor toms, staccato strings, horn calls and choir. */
function epicFiveFour() {
  const song = createSong({ id: "epic-54", bpm: 132, beatsPerBar: 5, key: key("E", "minor") });
  song.reverb = { room: 0.75, damp: 0.35, width: 1, level: 0.58 };
  instrument(song, 0, GM.strings, { volume: 82, pan: 56, reverb: 55 });
  instrument(song, 1, GM.horn, { volume: 88, pan: 50, reverb: 65 });
  instrument(song, 2, GM.brass, { volume: 74, pan: 64, reverb: 60 });
  instrument(song, 3, P.taiko, { volume: 92, pan: 64, reverb: 55 });
  instrument(song, 4, GM.contrabass, { volume: 82, pan: 60, reverb: 45 });
  instrument(song, 5, GM.choir, { volume: 70, pan: 64, reverb: 80 });
  instrument(song, 6, GM.timpani, { volume: 84, pan: 64, reverb: 60 });
  instrument(song, 7, GM.violin, { volume: 78, pan: 80, reverb: 60 });
  instrument(song, DRUMS, KIT.orchestra, { volume: 82, reverb: 55 });
  const A = ["i", "i", "VI", "VII", "i", "iv", "VI", "V+"];
  const B = ["iv", "VI", "VII", "i", "iv", "VI", "VII", "V+"];
  const FIVE = [[[1.5], [1.5], [1], [1]], [[1], [0.5], [1.5], [2]], [[2], [1], [2]], [[0.5], [0.5], [0.5], [1.5], [2]]];
  // 3 + 2: accents on the bar's 1 and on beat 2.5 (the "and" of two).
  const accent = [1, 0.55, 0.6, 0.55, 0.6, 1, 0.55, 0.6, 1, 0.55];
  const taiko = [[0, DR.floorTom, 92], [1.5, DR.floorTom, 80], [3, DR.floorTom, 86], [4, DR.tomLo, 70], [4.5, DR.tomLo, 64], [0, DR.kick2, 80], [3, DR.kick2, 70]];
  const parts = {
    ostinato: (s) => arp(s, { ch: 0, step: 0.5, pattern: [0, 0, 1, 0, 0, 2, 0, 1, 2, 1], center: 52, vel: 62, gate: 0.5, accent }),
    taiko: (s) => {
      drums(s, { pattern: taiko, fill: [...taiko, ...at(DR.tomMid, 72, [3.5, 3.75, 4.25, 4.75])] });
      roots(s, { ch: 3, octave: -2, vel: 80, hits: [[0, 0.5, 1], [1.5, 0.5, 0.8], [3, 0.5, 0.9]] });
    },
    low: (s) => bass(s, { ch: 4, pattern: [[0, "R", 1.4, 1], [1.5, "R", 1.4, 0.8], [3, "5", 1.9, 0.9]], vel: 78, octave: -2 }),
    horn: (s, i) => melody(s, { ch: 1, rhythms: FIVE, range: [0, 8], vel: 84, motifBars: [0, 4], start: 4, seed: i }),
    choir: (s, i) => melody(s, { ch: 5, rhythms: FIVE, range: [2, 9], vel: 76, motifBars: [0], start: 4, seed: 10 + i }),
    choirPad: (s) => pad(s, { ch: 5, center: 62, vel: 48 }),
    stabs: (s) => comp(s, { ch: 2, hits: [[0, 0.4, 1], [1.5, 0.4, 0.8], [3, 0.9, 0.9]], center: 57, vel: 66 }),
    violin: (s, i) => melody(s, { ch: 7, rhythms: FIVE, range: [7, 14], vel: 72, motifBars: [], start: 9, seed: 20 + i }),
    timp: (s) => roots(s, { ch: 6, octave: -2, vel: 72, hits: [[0, 1, 1], [3, 0.5, 0.7]] }),
  };
  return arrange(song, parts, [
    [8, A, ["taiko", "low"]],
    [16, A, ["ostinato", "taiko", "low", "horn"]],
    [8, B, ["ostinato", "low", "choir", "choirPad", "stabs", "timp"]],
    [8, A, ["ostinato", "taiko", "low", "stabs", "violin"]],
    [8, B, ["ostinato", "taiko", "low", "horn", "choirPad", "stabs", "timp"]],
    [8, A, ["ostinato", "taiko", "low", "choir", "violin"]],
  ]);
}

/** A hybrid trailer cue: a sixteenth synth pulse, string ostinato, low-brass braams and a lone piano. */
function epicTrailer() {
  const song = createSong({ id: "epic-trailer", bpm: 90, key: key("Db", "minor"), humanize: 0.006 });
  song.reverb = { room: 0.85, damp: 0.3, width: 1, level: 0.62 };
  instrument(song, 0, GM.synthBass2, { volume: 80, pan: 64, reverb: 15 });
  instrument(song, 1, GM.strings, { volume: 80, pan: 56, reverb: 55 });
  instrument(song, 2, GM.brass, { volume: 86, pan: 64, reverb: 70 });
  instrument(song, 3, GM.tuba, { volume: 80, pan: 60, reverb: 60 });
  instrument(song, 4, GM.choir, { volume: 76, pan: 64, reverb: 80 });
  instrument(song, 5, GM.piano, { volume: 84, pan: 70, reverb: 75 });
  instrument(song, 6, GM.timpani, { volume: 86, pan: 64, reverb: 60 });
  instrument(song, 7, GM.atmosphere, { volume: 64, pan: 64, reverb: 80 });
  instrument(song, DRUMS, KIT.power, { volume: 90, reverb: 60 });
  const A = ["i", "i", "VI", "VI", "III", "III", "VII", "VII"];
  const B = ["iv", "iv", "VI", "VI", "i", "i", "V+", "V+"];
  const hits = [[0, DR.kick, 104], [0, DR.floorTom, 80], [1.5, DR.kick, 84], [2, DR.snare, 96], [2, DR.tomLo, 72], [3, DR.floorTom, 76], [3.5, DR.tomLo, 64]];
  const parts = {
    atmos: (s) => pad(s, { ch: 7, center: 60, vel: 50 }),
    piano: (s) => {
      for (let bar = 0; bar < s.bars; bar += 4) line(s, { ch: 5, notes: [...QUESTION.slice(0, 6), [null, 4], ...QUESTION.slice(6, 10), [null, 4]], octave: 1, vel: 66, at: bar * 4 });
    },
    pulse: (s) => pulse(s, { ch: 0, step: 0.25, octave: -2, vel: 70, accent: [1, 0.6, 0.75, 0.6], gate: 0.5 }),
    ostinato: (s) => arp(s, { ch: 1, step: 0.25, pattern: [0, 0, 1, 0, 2, 0, 1, 0], center: 50, vel: 60, gate: 0.5, accent: [1, 0.7, 0.8, 0.7] }),
    braam: (s) => {
      comp(s, { ch: 2, hits: [[0, 3.5, 1]], center: 48, vel: 84, until: s.beats });
      bass(s, { ch: 3, pattern: [[0, "R", 3.5, 1]], vel: 80, octave: -2 });
    },
    drums: (s) => drums(s, { pattern: hits, fill: [...hits, ...at(DR.snare, 80, [3, 3.25, 3.5, 3.75])] }),
    choir: (s, i) => melody(s, { ch: 4, rhythms: RHYTHMS.epic, range: [0, 8], vel: 80, motifBars: [0, 4], start: 4, seed: i }),
    brassLead: (s, i) => melody(s, { ch: 2, rhythms: RHYTHMS.epic, range: [0, 7], vel: 86, motifBars: [0, 4], start: 4, seed: 10 + i }),
    timp: (s) => roots(s, { ch: 6, octave: -2, vel: 78, hits: [[0, 1, 1], [3, 0.25, 0.6], [3.5, 0.25, 0.8]] }),
  };
  return arrange(song, parts, [
    [8, A, ["atmos", "piano", "pulse"]],
    [8, A, ["atmos", "piano", "pulse", "ostinato"]],
    [8, A, ["pulse", "ostinato", "braam", "drums", "timp"]],
    [8, B, ["pulse", "ostinato", "choir", "atmos"]],
    [8, A, ["pulse", "ostinato", "braam", "drums", "brassLead", "timp"]],
    [8, B, ["pulse", "piano", "choir", "timp"]],
  ]);
}

/** A wizard's scherzo in 3/4: pizzicato, bassoon and clarinet, piccolo flourishes and xylophone. */
function epicScherzo() {
  const song = createSong({ id: "epic-scherzo", bpm: 150, beatsPerBar: 3, key: key("A", "harmonic") });
  song.reverb = { room: 0.7, damp: 0.4, width: 0.9, level: 0.5 };
  instrument(song, 0, GM.pizz, { volume: 88, pan: 50, reverb: 50 });
  instrument(song, 1, GM.bassoon, { volume: 92, pan: 56, reverb: 50 });
  instrument(song, 2, GM.clarinet, { volume: 86, pan: 72, reverb: 55 });
  instrument(song, 3, GM.piccolo, { volume: 72, pan: 86, reverb: 60 });
  instrument(song, 4, GM.celesta, { volume: 70, pan: 90, reverb: 60 });
  instrument(song, 5, GM.horn, { volume: 76, pan: 40, reverb: 60 });
  instrument(song, 6, GM.xylophone, { volume: 80, pan: 76, reverb: 50 });
  instrument(song, 7, GM.strings, { volume: 66, pan: 64, reverb: 65 });
  instrument(song, DRUMS, KIT.orchestra, { volume: 74, reverb: 50 });
  const A = ["i", "i", "iv", "V", "i", "VI", "iv", "V"];
  const B = ["VI", "iv", "V", "i", "VI", "iv", ["ii", "V"], "i"];
  const SCHERZO = [[[0.5], [0.5], [0.5], [0.5], [1]], [[1], [0.5], [0.5], [1]], [[0.5], [0.5], [2]], [[1], [1, true], [1]]];
  const parts = {
    pizz: (s) => {
      bass(s, { ch: 0, pattern: [[0, "R", 0.4, 1]], vel: 80, octave: -1 });
      comp(s, { ch: 0, hits: [[1, 0.3, 0.75], [2, 0.3, 0.7]], center: 60, vel: 62 });
    },
    bassoon: (s, i) => melody(s, { ch: 1, rhythms: SCHERZO, range: [-4, 4], vel: 86, motifBars: [0, 8], start: 0, seed: i }),
    clarinet: (s, i) => melody(s, { ch: 2, rhythms: SCHERZO, range: [2, 10], vel: 82, motifBars: [0, 8], start: 4, seed: 10 + i }),
    piccolo: (s) => {
      for (let bar = 3; bar < s.bars; bar += 4) line(s, { ch: 3, notes: [[11, 0.25], [12, 0.25], [13, 0.25], [14, 0.25], [12, 1]], vel: 64, at: bar * 3 + 1 });
    },
    xylo: (s, i) => melody(s, { ch: 6, rhythms: SCHERZO, range: [4, 12], vel: 74, motifBars: [0], start: 7, seed: 20 + i }),
    hornStabs: (s) => comp(s, { ch: 5, hits: [[0, 0.5, 1]], center: 55, vel: 62 }),
    celesta: (s) => arp(s, { ch: 4, step: 0.5, pattern: [0, 2, 4, 3, 2, 1], center: 72, vel: 44 }),
    strings: (s) => pad(s, { ch: 7, center: 60, vel: 40 }),
    blocks: (s) => drums(s, { pattern: [[1, DR.blockHi, 46], [2, DR.blockLo, 40], [0, DR.triMute, 30]] }),
  };
  return arrange(song, parts, [
    [8, A, ["pizz", "bassoon"]],
    [16, A, ["pizz", "bassoon", "blocks"]],
    [16, A, ["pizz", "clarinet", "piccolo", "blocks"]],
    [16, B, ["pizz", "xylo", "hornStabs", "strings"]],
    [16, A, ["pizz", "bassoon", "clarinet", "celesta", "blocks"]],
    [16, B, ["pizz", "xylo", "clarinet", "hornStabs", "piccolo", "strings"]],
    [16, A, ["pizz", "bassoon", "celesta", "blocks"]],
  ]);
}

/* ------------------------------------------------------------------------------------------- *
 * House
 * ------------------------------------------------------------------------------------------- */

const FLOOR = at(DR.kick, 96, [0, 1, 2, 3]);
const OFF_OPEN = at(DR.open, 54, [0.5, 1.5, 2.5, 3.5]);
const CLAP24 = at(DR.clap, 76, [1, 3]);

/** Deep house in Bb minor: minor-seventh electric-piano stabs, warm pads, a rolling deep bass. */
function houseDeep() {
  const song = createSong({ id: "house-deep", bpm: 122, key: key("Bb", "minor"), swing: 0.04, humanize: 0.004 });
  song.reverb = { room: 0.6, damp: 0.45, width: 1, level: 0.5 };
  instrument(song, 0, GM.ep2, { volume: 84, pan: 56, reverb: 55 });
  instrument(song, 1, GM.warmPad, { volume: 70, pan: 64, reverb: 70 });
  instrument(song, 2, GM.synthBass2, { volume: 86, pan: 64, reverb: 5 });
  instrument(song, 3, GM.oohs, { volume: 74, pan: 72, reverb: 70 });
  instrument(song, 4, GM.crystal, { volume: 64, pan: 86, reverb: 70 });
  instrument(song, DRUMS, KIT.electronic, { volume: 88, reverb: 20 });
  const A = ["i7", "i7", "iv7", "iv7", "VI7", "VI7", "v7", "v7"];
  const B = ["VI7", "VII", "i7", "i7", "iv79", "VII7", "III7", "v7"];
  const beat = [...FLOOR, ...CLAP24, ...OFF_OPEN, ...at(DR.hat, 32, every(0.25)), ...at(DR.shaker, 28, every(0.5))];
  const parts = {
    drums: (s) => drums(s, { pattern: beat }),
    noKick: (s) => drums(s, { pattern: [...at(DR.hat, 30, every(0.25)), ...CLAP24] }),
    bass: (s) => bass(s, { ch: 2, pattern: [[0.5, "R", 0.4, 1], [1.5, "R", 0.4, 0.8], [1.75, "8", 0.2, 0.6], [2.5, "R", 0.4, 1], [3.5, "5", 0.4, 0.8]], vel: 88, octave: -2 }),
    stabs: (s) => comp(s, { ch: 0, hits: [[0, 0.35, 1], [0.75, 0.3, 0.7], [1.5, 0.4, 0.9], [2.75, 0.3, 0.7], [3.5, 0.4, 0.8]], center: 60, vel: 64 }),
    pad: (s) => pad(s, { ch: 1, center: 62, vel: 50 }),
    oohs: (s, i) => melody(s, { ch: 3, rhythms: RHYTHMS.epic, range: [2, 9], vel: 74, motifBars: [0, 4], start: 4, seed: i }),
    crystal: (s) => arp(s, { ch: 4, step: 0.25, pattern: [0, 2, 1, 3, 2, 4, 3, 5], center: 72, vel: 40, gate: 0.4 }),
    pump: (s) => pump(s, { chs: [0, 1], depth: 0.4 }),
  };
  return arrange(song, parts, [
    [8, A, ["drums", "bass"]],
    [16, A, ["drums", "bass", "stabs", "pad", "pump"]],
    [8, B, ["noKick", "pad", "oohs"]],
    [16, A, ["drums", "bass", "stabs", "crystal", "oohs", "pump"]],
    [16, B, ["drums", "bass", "stabs", "pad", "crystal", "oohs", "pump"]],
  ]);
}

/** Nineties piano house in Eb major: big piano chord stabs, a choir hook on the theme, tambourine. */
function housePiano() {
  const song = createSong({ id: "house-piano", bpm: 124, key: key("Eb", "major"), humanize: 0.004 });
  song.reverb = { room: 0.6, damp: 0.4, width: 1, level: 0.5 };
  instrument(song, 0, GM.brightPiano, { volume: 92, pan: 58, reverb: 50 });
  instrument(song, 1, GM.choir, { volume: 84, pan: 64, reverb: 65 });
  instrument(song, 2, GM.synthBass1, { volume: 84, pan: 64, reverb: 5 });
  instrument(song, 3, GM.strings, { volume: 70, pan: 70, reverb: 65 });
  instrument(song, 4, GM.organ, { volume: 70, pan: 44, reverb: 40 });
  instrument(song, DRUMS, KIT.electronic, { volume: 88, reverb: 20 });
  const A = ["I", "V", "vi", "IV", "I", "V", "IV", ["ii", "V"]];
  const B = ["vi", "IV", "I", "V", "vi", "IV", "ii7", "V"];
  const beat = [...FLOOR, ...CLAP24, ...OFF_OPEN, ...at(DR.hat, 40, every(0.5)), ...at(DR.tamb, 44, [0.5, 1.5, 2.5, 3.5, 3.75])];
  const parts = {
    drums: (s) => drums(s, { pattern: beat, fill: [...beat, ...at(DR.snare, 70, [3, 3.25, 3.5, 3.75])] }),
    bass: (s) => bass(s, { ch: 2, pattern: [[0.5, "R", 0.4, 1], [1.5, "8", 0.4, 0.8], [2.5, "R", 0.4, 1], [3.5, "8", 0.4, 0.8]], vel: 86, octave: -2 }),
    piano: (s) => comp(s, { ch: 0, hits: [[0, 0.4, 1], [0.75, 0.3, 0.75], [1.5, 0.4, 0.9], [2.5, 0.4, 0.85], [3.25, 0.3, 0.75]], center: 64, vel: 72 }),
    hook: (s) => {
      for (let bar = 0; bar < s.bars; bar += 8) line(s, { ch: 1, notes: [...QUESTION, ...ANSWER.slice(0, 9)], vel: 84, at: bar * 4 });
    },
    hookHigh: (s) => {
      for (let bar = 0; bar < s.bars; bar += 8) line(s, { ch: 1, notes: [...QUESTION, ...ANSWER.slice(0, 9)], octave: 1, vel: 80, at: bar * 4 });
    },
    strings: (s) => pad(s, { ch: 3, center: 62, vel: 50 }),
    organ: (s) => comp(s, { ch: 4, hits: [[0.5, 0.3, 1], [2.5, 0.3, 0.9]], center: 55, vel: 54 }),
    solo: (s, i) => melody(s, { ch: 0, rhythms: RHYTHMS.edm, range: [7, 14], vel: 70, motifBars: [0, 4], start: 9, seed: i }),
  };
  return arrange(song, parts, [
    [8, A, ["drums", "bass", "organ"]],
    [16, A, ["drums", "bass", "piano", "hook"]],
    [8, B, ["bass", "strings", "organ", "solo"]],
    [16, A, ["drums", "bass", "piano", "hookHigh", "strings"]],
    [16, B, ["drums", "bass", "piano", "organ", "solo"]],
  ]);
}

/** French disco house in E dorian: funk guitar chops, slap-bass octaves, disco strings, swells. */
function houseDisco() {
  const song = createSong({ id: "house-disco", bpm: 120, key: key("E", "dorian"), humanize: 0.004 });
  song.reverb = { room: 0.55, damp: 0.45, width: 1, level: 0.45 };
  instrument(song, 0, GM.mutedGuitar, { volume: 84, pan: 38, reverb: 30 });
  instrument(song, 1, P.slapBass, { volume: 88, pan: 64, reverb: 10 });
  instrument(song, 2, GM.strings, { volume: 78, pan: 76, reverb: 60 });
  instrument(song, 3, P.clav, { volume: 76, pan: 52, reverb: 35 });
  instrument(song, 4, GM.brass, { volume: 76, pan: 60, reverb: 50 });
  instrument(song, 5, GM.voiceLead, { volume: 78, pan: 64, reverb: 55 });
  instrument(song, DRUMS, KIT.standard, { volume: 90, reverb: 25 });
  const A = ["i7", "IV7", "i7", "IV7", "i7", "IV7", "III", ["IV", "v7"]];
  const B = ["VII", "III", "IV7", "i7", "VII", "III", "IV7", "v7"];
  const beat = [...FLOOR, ...at(DR.snare, 70, [1, 3]), ...OFF_OPEN, ...at(DR.tamb, 34, every(0.25))];
  const parts = {
    drums: (s) => drums(s, { pattern: beat }),
    guitar: (s) => arp(s, { ch: 0, step: 0.25, pattern: [1, null, 2, 1, null, 2, 1, 2], center: 64, vel: 58, gate: 0.4 }),
    bass: (s) => bass(s, { ch: 1, pattern: every(0.5).map((t, i) => [t, i % 2 === 0 ? "R" : "8", 0.35, i % 2 === 0 ? 1 : 0.75]), vel: 86, octave: -2 }),
    strings: (s) => pad(s, { ch: 2, center: 64, vel: 50 }),
    stringRuns: (s) => {
      for (let bar = 3; bar < s.bars; bar += 4) line(s, { ch: 2, notes: [[7, 0.25], [8, 0.25], [9, 0.25], [10, 0.25], [11, 0.25], [12, 0.25], [13, 0.25], [14, 0.25]], vel: 70, at: bar * 4 + 2 });
    },
    clav: (s) => comp(s, { ch: 3, hits: [[0.5, 0.2, 1], [1.25, 0.2, 0.7], [2.5, 0.2, 0.9], [3.25, 0.2, 0.7]], center: 60, vel: 60 }),
    horns: (s) => comp(s, { ch: 4, hits: [[0, 0.3, 1], [1.5, 0.5, 0.8]], center: 60, vel: 66 }),
    hook: (s, i) => melody(s, { ch: 5, rhythms: RHYTHMS.edm, range: [3, 10], vel: 78, motifBars: [0, 4], start: 4, seed: i }),
    swell: (s) => {
      swell(s, { ch: 0, lo: 30 });
      swell(s, { ch: 2, lo: 30 });
    },
  };
  return arrange(song, parts, [
    [8, A, ["drums", "guitar", "swell", "strings"]],
    [16, A, ["drums", "guitar", "bass", "strings", "hook"]],
    [8, B, ["drums", "bass", "stringRuns", "horns"]],
    [16, A, ["drums", "guitar", "bass", "clav", "hook"]],
    [16, B, ["drums", "guitar", "bass", "clav", "strings", "stringRuns", "horns", "hook"]],
  ]);
}

/** Tech house in G minor: a rolling sixteenth bass, rim and conga syncopation, one chord stab. */
function houseTech() {
  const song = createSong({ id: "house-tech", bpm: 126, key: key("G", "minor"), humanize: 0.002 });
  song.reverb = { room: 0.45, damp: 0.55, width: 1, level: 0.4 };
  instrument(song, 0, GM.polysynth, { volume: 74, pan: 60, reverb: 60 });
  instrument(song, 1, GM.synthBass1, { volume: 90, pan: 64, reverb: 5 });
  instrument(song, 2, GM.synthVoice, { volume: 70, pan: 72, reverb: 65 });
  instrument(song, 3, GM.metalPad, { volume: 56, pan: 50, reverb: 70 });
  instrument(song, DRUMS, KIT.electronic, { volume: 92, reverb: 15 });
  const A = ["i", "i", "i", "i", "VI", "VI", "VII", "VII"];
  const B = ["iv", "iv", "VI", "VI", "i", "i", "VII", "VII"];
  const perc = [...at(DR.stick, 56, [0.75, 2.25, 3.5]), ...at(DR.congaHi, 50, [1.5, 3.25]), ...at(DR.congaLo, 54, [2.75]), ...at(DR.shaker, 30, every(0.25))];
  const groove = [...FLOOR, ...CLAP24, ...OFF_OPEN, ...at(DR.hat, 36, [0.25, 1.25, 2.25, 3.25])];
  const parts = {
    groove: (s) => drums(s, { pattern: groove }),
    perc: (s) => drums(s, { pattern: perc }),
    roll: (s) => bass(s, { ch: 1, pattern: [[0.5, "R", 0.2, 1], [0.75, "R", 0.2, 0.7], [1.5, "8", 0.2, 0.9], [1.75, "R", 0.2, 0.6], [2.5, "R", 0.2, 1], [2.75, "5", 0.2, 0.7], [3.25, "R", 0.2, 0.6], [3.5, "8", 0.2, 0.9]], vel: 92, octave: -2 }),
    stab: (s) => comp(s, { ch: 0, hits: [[0.75, 0.25, 1], [2.5, 0.2, 0.7]], center: 60, vel: 64 }),
    voice: (s, i) => melody(s, { ch: 2, rhythms: RHYTHMS.edm, range: [2, 9], vel: 72, motifBars: [0, 4], start: 4, seed: i }),
    pad: (s) => pad(s, { ch: 3, center: 62, vel: 46 }),
    hats: (s) => drums(s, { pattern: [...at(DR.hat, 32, every(0.25)), ...CLAP24] }),
  };
  return arrange(song, parts, [
    [8, A, ["groove"]],
    [8, A, ["groove", "roll"]],
    [16, A, ["groove", "perc", "roll", "stab"]],
    [8, B, ["hats", "voice", "pad"]],
    [16, B, ["groove", "perc", "roll", "stab", "voice"]],
    [8, A, ["groove", "perc", "roll"]],
  ]);
}

/** Afro house in D dorian: conga and shaker polyrhythm, a marimba riff, a chant, a pan flute. */
function houseAfro() {
  const song = createSong({ id: "house-afro", bpm: 120, key: key("D", "dorian"), humanize: 0.006 });
  song.reverb = { room: 0.6, damp: 0.4, width: 1, level: 0.5 };
  instrument(song, 0, GM.marimba, { volume: 88, pan: 50, reverb: 45 });
  instrument(song, 1, GM.kalimba, { volume: 76, pan: 82, reverb: 55 });
  instrument(song, 2, GM.oohs, { volume: 80, pan: 64, reverb: 70 });
  instrument(song, 3, GM.fingerBass, { volume: 88, pan: 64, reverb: 10 });
  instrument(song, 4, GM.panFlute, { volume: 84, pan: 70, reverb: 60 });
  instrument(song, 5, GM.warmPad, { volume: 60, pan: 64, reverb: 75 });
  instrument(song, DRUMS, KIT.standard, { volume: 90, reverb: 25 });
  const A = ["i7", "i7", "IV", "IV", "i7", "i7", "VII", "IV"];
  const B = ["III", "VII", "IV", "i7", "III", "VII", "IV", "v7"];
  // Three against four on the congas, a son clave, shakers and a bell.
  const poly = [
    ...at(DR.congaHi, 58, [0, 0.75, 1.5, 2.25, 3, 3.75]), ...at(DR.congaLo, 64, [0.5, 2]), ...at(DR.congaMute, 40, [1.25, 2.75]),
    ...at(DR.claves, 44, [0, 0.75, 1.5, 2.5, 3]), ...at(DR.shaker, 30, every(0.25)), ...at(D.agogoHi, 34, [1, 3.5]),
  ];
  const kick = [...at(DR.kick, 88, [0, 1, 2, 3]), ...at(DR.open, 40, [0.5, 1.5, 2.5, 3.5])];
  const parts = {
    perc: (s) => drums(s, { pattern: poly }),
    kick: (s) => drums(s, { pattern: kick }),
    marimba: (s) => arp(s, { ch: 0, step: 0.25, pattern: [0, null, 2, 1, null, 3, 2, null, 4, 2, null, 1, 3, null, 2, null], center: 62, vel: 62, gate: 0.6 }),
    kalimba: (s, i) => melody(s, { ch: 1, rhythms: RHYTHMS.edm, range: [7, 13], vel: 62, motifBars: [], start: 9, seed: 10 + i }),
    bass: (s) => bass(s, { ch: 3, pattern: [[0, "R", 0.4, 1], [0.75, "R", 0.4, 0.7], [1.5, "5", 0.4, 0.8], [2.5, "R", 0.4, 0.9], [3.25, "8", 0.4, 0.7]], vel: 86 }),
    flute: (s, i) => melody(s, { ch: 4, rhythms: RHYTHMS.folk, range: [3, 11], vel: 80, motifBars: [0, 4], start: 4, seed: i }),
    chant: (s) => {
      for (let bar = 0; bar < s.bars; bar += 4) line(s, { ch: 2, notes: [[4, 1], [4, 0.5], [5, 0.5], [7, 2], [null, 4], [5, 1], [4, 1], [2, 2], [null, 4]], vel: 78, at: bar * 4 });
    },
    pad: (s) => pad(s, { ch: 5, center: 60, vel: 44 }),
  };
  return arrange(song, parts, [
    [8, A, ["perc", "pad"]],
    [8, A, ["perc", "kick", "marimba"]],
    [16, A, ["perc", "kick", "marimba", "bass", "flute"]],
    [8, B, ["perc", "chant", "pad", "kalimba"]],
    [16, B, ["perc", "kick", "marimba", "bass", "chant", "flute"]],
    [8, A, ["perc", "kick", "marimba", "bass", "kalimba"]],
  ]);
}

/** The House station's low-health track: acid house in F minor, a squelching line over a low pulse. */
function houseDanger() {
  const song = createSong({ id: "house-danger", bpm: 128, key: key("F", "minor"), humanize: 0.002 });
  song.reverb = { room: 0.5, damp: 0.5, width: 1, level: 0.42 };
  instrument(song, 0, GM.bassLead, { volume: 84, pan: 60, reverb: 25 });
  instrument(song, 1, GM.synthBass2, { volume: 86, pan: 64, reverb: 5 });
  instrument(song, 2, GM.polysynth, { volume: 72, pan: 50, reverb: 55 });
  instrument(song, 3, GM.sweepPad, { volume: 64, pan: 64, reverb: 75 });
  instrument(song, 4, GM.saw, { volume: 70, pan: 76, reverb: 50 });
  instrument(song, DRUMS, KIT.electronic, { volume: 92, reverb: 15 });
  const A = ["i", "i", "VI", "V+", "i", "i", "iv", "V+"];
  const B = ["iv", "VI", "i", "V+", "iv", "VI", "VII", "V+"];
  const beat = [...FLOOR, ...CLAP24, ...OFF_OPEN, ...at(DR.hat, 40, every(0.25)), [3.5, DR.kick, 70]];
  const parts = {
    drums: (s) => drums(s, { pattern: beat, fill: [...FLOOR, ...at(DR.clap, 80, [1, 2, 2.5, 3, 3.25, 3.5, 3.75])] }),
    acid: (s) => {
      arp(s, { ch: 0, step: 0.25, pattern: [0, 0, 3, 0, 1, 0, 4, 2, 0, 3, 0, 1, 5, 0, 2, 1], center: 50, vel: 74, gate: 0.45, accent: [1, 0.55, 0.9, 0.55, 1, 0.5, 0.85, 0.7] });
      for (let bar = 0; bar < s.bars; bar += 1) wobble(s, { ch: 0, cycle: 2, depth: 0.5, from: bar * 4, to: bar * 4 + 4, shape: "cos" });
    },
    pulse: (s) => pulse(s, { ch: 1, step: 0.5, octave: -2, vel: 84, accent: [1, 0.75] }),
    stab: (s) => comp(s, { ch: 2, hits: [[0, 0.3, 1], [1.5, 0.3, 0.8], [3, 0.3, 0.9]], center: 60, vel: 62 }),
    pad: (s) => pad(s, { ch: 3, center: 60, vel: 50 }),
    lead: (s, i) => melody(s, { ch: 4, rhythms: RHYTHMS.edm, range: [4, 11], vel: 74, motifBars: [0, 4], start: 7, seed: i }),
  };
  return arrange(song, parts, [
    [8, A, ["drums", "pulse"]],
    [16, A, ["drums", "pulse", "acid"]],
    [8, B, ["drums", "pulse", "pad", "lead"]],
    [16, A, ["drums", "pulse", "acid", "stab"]],
    [8, B, ["pulse", "pad", "acid"]],
    [16, A, ["drums", "pulse", "acid", "stab", "lead"]],
  ]);
}

/** The House station's match-start sting: the fanfare on piano over a four-on-the-floor, in Eb. */
function houseStart() {
  const song = createSong({ id: "house-start", bpm: 124, key: key("Eb", "major") });
  song.reverb = { room: 0.7, damp: 0.4, width: 0.9, level: 0.6 };
  instrument(song, 0, GM.brightPiano, { volume: 96, reverb: 60 });
  instrument(song, 1, GM.warmPad, { volume: 86, reverb: 60 });
  instrument(song, 2, GM.synthBass1, { volume: 92, reverb: 30 });
  instrument(song, DRUMS, KIT.electronic, { volume: 84, reverb: 40 });
  const s = section(song, "intro", 3, ["I", ["vi", "V"], "I"]);
  line(s, { ch: 0, notes: [...FANFARE, [0, 4]], vel: 88 });
  comp(s, { ch: 1, hits: [[0, 1.9, 1]], center: 60, vel: 62 });
  bass(s, { ch: 2, pattern: [[0.5, "R", 0.4, 1], [1.5, "8", 0.4, 0.8], [2.5, "R", 0.4, 1], [3.5, "8", 0.4, 0.8]], vel: 80 });
  drums(s, { pattern: [...FLOOR, ...CLAP24, ...OFF_OPEN] });
  return { song };
}

/* ------------------------------------------------------------------------------------------- *
 * The list
 * ------------------------------------------------------------------------------------------- */

/** `title` and `blurb` are for the audition page; gen-music reads `id`, `loop` and `build`. */
export const CANDIDATES = [
  { id: "tavern-waltz", station: "tavern", title: "Hearth Waltz", blurb: "3/4 at 138, A dorian. Accordion and violin over an oom-pah-pah guitar, dulcimer sparkle.", loop: true, build: tavernWaltz },
  { id: "tavern-shanty", station: "tavern", title: "Sea Shanty", blurb: "6/8, D minor. Accordion calls the theme, a crew of voices answers, boots stomp and hands clap.", loop: true, build: tavernShanty },
  { id: "tavern-ballad", station: "tavern", title: "Bard's Ballad", blurb: "80 bpm, E minor. Harp, recorder and dulcimer over a held open fifth; a frame drum, no kit.", loop: true, build: tavernBallad },
  { id: "tavern-reel", station: "tavern", title: "Hoedown Reel", blurb: "140 bpm, G major. Fiddle and banjo rolls over boom-chick guitar, spoons, whistle and harmonica.", loop: true, build: tavernReel },
  { id: "tavern-caravan", station: "tavern", title: "Caravanserai", blurb: "104 bpm, E phrygian dominant. Sitar and shanai over darbuka (maqsum), an oud-like guitar and a qanun.", loop: true, build: tavernCaravan },
  { id: "edm-dnb", station: "edm", title: "Liquid DnB", blurb: "172 bpm, F minor. Broken beat, detuned reese bass, electric piano and pads.", loop: true, build: edmDnb },
  { id: "edm-synthwave", station: "edm", title: "Outrun", blurb: "104 bpm, E minor. Synthwave: gated snare, octave bass eighths, polysynth, synth-brass lead.", loop: true, build: edmSynthwave },
  { id: "edm-halftime", station: "edm", title: "Halftime Wobble", blurb: "140 bpm halftime, C minor. Growl bass wobbling at changing rates, a sub, bells and orchestra hits.", loop: true, build: edmHalftime },
  { id: "edm-chip", station: "edm", title: "Chiptune", blurb: "150 bpm, C major. Square-wave lead and fast chord arps, square bass, noise drums.", loop: true, build: edmChip },
  { id: "edm-trap", station: "edm", title: "Orchestral Trap", blurb: "140 bpm, G harmonic minor. 808 kit with hat rolls, gliding 808 bass, flute, bells and pizzicato.", loop: true, build: edmTrap },
  { id: "lofi-jazzhop", station: "lofi", title: "Jazzhop", blurb: "88 bpm swung, Eb major. Acoustic piano ninths, walking upright, muted trumpet and tenor sax.", loop: true, build: lofiJazzhop },
  { id: "lofi-bossa", station: "lofi", title: "Bossa Tape", blurb: "96 bpm straight, G major. Nylon guitar on the bossa rhythm, rim clave, flute and clarinet.", loop: true, build: lofiBossa },
  { id: "lofi-rain", station: "lofi", title: "Rain on Glass", blurb: "70 bpm, Db lydian. Kalimba, music box and celesta over pads, mostly drumless, rain-like hiss.", loop: true, build: lofiRain },
  { id: "lofi-boombap", station: "lofi", title: "Dusty Boom-bap", blurb: "86 bpm heavy swing, E dorian. Chopped organ stabs, hard kick and snare, jazz guitar and alto sax.", loop: true, build: lofiBoombap },
  { id: "lofi-vapor", station: "lofi", title: "Vapor Dream", blurb: "78 bpm, Ab major. FM electric piano and synth pads, darker low-pass, slow deep tape wow.", loop: true, build: lofiVapor },
  { id: "epic-heroic", station: "epic", title: "Heroic Adventure", blurb: "120 bpm, F lydian. Trumpet and horns over driving strings, march snare and glockenspiel.", loop: true, build: epicHeroic },
  { id: "epic-requiem", station: "epic", title: "Dark Requiem", blurb: "72 bpm, C phrygian. Choir-led, church organ, trombone and tuba, tolling bells.", loop: true, build: epicRequiem },
  { id: "epic-54", station: "epic", title: "Battle in Five", blurb: "5/4 (3+2) at 132, E minor. Taiko and floor toms, staccato strings, horn calls and choir.", loop: true, build: epicFiveFour },
  { id: "epic-trailer", station: "epic", title: "Hybrid Trailer", blurb: "90 bpm, C# minor. Sixteenth synth pulse, string ostinato, low-brass braams and a lone piano.", loop: true, build: epicTrailer },
  { id: "epic-scherzo", station: "epic", title: "Wizard's Scherzo", blurb: "3/4 at 150, A harmonic minor. Pizzicato, bassoon and clarinet, piccolo flourishes, xylophone.", loop: true, build: epicScherzo },
  { id: "house-deep", station: "house", title: "Deep House", blurb: "122 bpm, Bb minor. Minor-seventh electric-piano stabs, warm pads, rolling deep bass, oohs.", loop: true, build: houseDeep },
  { id: "house-piano", station: "house", title: "Piano House", blurb: "124 bpm, Eb major. 90s piano stabs and a choir hook on the JackiOh theme, tambourine.", loop: true, build: housePiano },
  { id: "house-disco", station: "house", title: "Disco House", blurb: "120 bpm, E dorian. French-touch funk guitar, slap-bass octaves, disco strings, swells.", loop: true, build: houseDisco },
  { id: "house-tech", station: "house", title: "Tech House", blurb: "126 bpm, G minor. Rolling sixteenth bass, rim and conga syncopation, one chord stab, vocal synth.", loop: true, build: houseTech },
  { id: "house-afro", station: "house", title: "Afro House", blurb: "120 bpm, D dorian. Conga polyrhythm and son clave, marimba riff, chant and pan flute.", loop: true, build: houseAfro },
  { id: "house-danger", station: "house", title: "Acid (low health)", blurb: "128 bpm, F minor. The station's low-health track: a squelching acid line over a low pulse.", loop: true, build: houseDanger },
  { id: "house-start", station: "house", title: "Match start", blurb: "The station's match-start sting: the fanfare on piano over four-on-the-floor.", loop: false, build: houseStart },
];
