// Every music track, as a score (SPEC §10.11 "Music", R631). gen-music.mjs renders each one.
//
// One motif runs through all of them: "Jack-i-Oh", a rising 5–6–8 that falls back by step (the
// main menu theme states it in full, MOTIF_THEME below, and every other track quotes its first bar,
// compose.mjs MOTIF_CELL). Four stations share the game's structure and differ in instruments,
// rhythm and harmony:
//
//   tavern  lute (nylon guitar), harp, flute, fiddle and hand percussion, warm folk-orchestral
//   edm     saw lead, square plucks, pads and synth bass over four on the floor, side-chained
//   lofi    electric piano, vibraphone and brushed drums, swung, low-passed, with tape wow and crackle
//   epic    string ostinatos, horns, choir and timpani, restrained until low health
//
// Each station has two in-game tracks (rotated across matches), a low-health variant (minor, faster,
// with a low pulse and busier percussion) and a match-start sting. The menu theme, the three result
// stings with their loops, the nine Mythic themes and the two shared Legendary themes are shared.

import {
  DR, DRUMS, GM, KIT, RHYTHMS,
  arp, bass, comp, createSong, drums, instrument, line, melody, pad, pulse, pump, roots, section,
} from "./compose.mjs";
import { key } from "./theory.mjs";

/* ------------------------------------------------------------------------------------------- *
 * The motif
 * ------------------------------------------------------------------------------------------- */

/** The JackiOh theme, eight bars of [degree, beats]: the question (bars 1–4) and its answer (5–8). */
export const MOTIF_THEME = [
  [4, 0.5], [5, 0.5], [7, 1.5], [6, 0.5], [5, 0.5], [4, 0.5],
  [2, 1], [1, 0.5], [0, 0.5], [1, 2],
  [4, 0.5], [5, 0.5], [7, 1.5], [8, 0.5], [9, 0.5], [8, 0.5],
  [7, 3], [null, 1],
  [5, 1], [4, 0.5], [2, 0.5], [4, 1.5], [5, 0.5],
  [7, 1], [5, 1], [4, 1], [3, 1],
  [4, 1], [1, 1], [2, 0.5], [3, 0.5], [4, 1],
  [2, 0.5], [1, 0.5], [0, 2], [null, 1],
];
/** The progression the theme is written over, one entry per bar. */
const THEME_CHORDS = ["I", ["vi", "V"], "I", ["IV", "I"], "vi", "IV", "V7", "I"];
/** The theme's first two bars, the fanfare every sting opens with. */
const FANFARE = MOTIF_THEME.slice(0, 10);

const TAVERN_PERC = [
  [0, DR.bongoLo, 70], [0.5, DR.shaker, 38], [1, DR.bongoHi, 52], [1.5, DR.shaker, 34],
  [2, DR.bongoLo, 62], [2.5, DR.shaker, 38], [2.75, DR.bongoHi, 40], [3, DR.bongoHi, 54], [3.5, DR.shaker, 34],
];
const TAVERN_TAMB = [[1, DR.tamb, 46], [3, DR.tamb, 46]];

/* ------------------------------------------------------------------------------------------- *
 * The main menu theme
 * ------------------------------------------------------------------------------------------- */

function menu() {
  const song = createSong({ id: "menu", bpm: 92, key: key("D", "major") });
  song.reverb = { room: 0.7, damp: 0.35, width: 0.9, level: 0.6 };
  instrument(song, 0, GM.flute, { volume: 96, pan: 70, reverb: 60 });
  instrument(song, 1, GM.oboe, { volume: 88, pan: 54, reverb: 55 });
  instrument(song, 2, GM.nylon, { volume: 92, pan: 44, reverb: 45 });
  instrument(song, 3, GM.harp, { volume: 86, pan: 86, reverb: 60 });
  instrument(song, 4, GM.acBass, { volume: 82, pan: 64, reverb: 30 });
  instrument(song, 5, GM.strings, { volume: 74, pan: 64, reverb: 70 });
  instrument(song, 6, GM.horn, { volume: 84, pan: 58, reverb: 65 });
  instrument(song, 7, GM.timpani, { volume: 80, pan: 64, reverb: 60 });
  instrument(song, 8, GM.cello, { volume: 86, pan: 50, reverb: 55 });
  instrument(song, 10, GM.dulcimer, { volume: 70, pan: 80, reverb: 55 });
  instrument(song, 11, GM.choir, { volume: 62, pan: 64, reverb: 75 });
  instrument(song, DRUMS, KIT.standard, { volume: 84, reverb: 35 });

  const intro = section(song, "intro", 4, ["I", "IV", "vi", ["IVsus2", "V"]]);
  arp(intro, { ch: 3, step: 0.5, pattern: [0, 1, 2, 3, 4, 5, 4, 3], center: 62, vel: 54 });
  pad(intro, { ch: 5, center: 62, vel: 40 });
  line(intro, { ch: 6, notes: [[null, 8], [4, 0.5], [5, 0.5], [7, 3], [4, 4]], octave: -1, vel: 62 });

  const lute = { ch: 2, step: 0.5, pattern: [0, 2, 3, 4, 3, 2, 1, 2], center: 55, vel: 52 };
  const folkBass = { ch: 4, pattern: [[0, "R", 1.4, 1], [1.5, "5", 0.5, 0.7], [2, "5", 1.4, 0.85], [3.5, "A", 0.5, 0.7]], vel: 78 };

  const a1 = section(song, "body", 8, THEME_CHORDS);
  line(a1, { ch: 0, notes: MOTIF_THEME, vel: 82 });
  arp(a1, lute);
  bass(a1, folkBass);
  drums(a1, { pattern: TAVERN_PERC, vel: 0.85 });
  pad(a1, { ch: 5, center: 60, vel: 34 });

  const a2 = section(song, "body", 8, THEME_CHORDS);
  line(a2, { ch: 1, notes: MOTIF_THEME, vel: 80 });
  melody(a2, { ch: 0, rhythms: RHYTHMS.epic, range: [7, 14], vel: 58, motifBars: [], start: 9, seed: 1 });
  arp(a2, lute);
  arp(a2, { ch: 3, step: 1, pattern: [0, 2, 4, 2], center: 67, vel: 44 });
  bass(a2, folkBass);
  drums(a2, { pattern: [...TAVERN_PERC, ...TAVERN_TAMB] });
  pad(a2, { ch: 5, center: 60, vel: 40 });

  const bProg = ["vi", "IV", "I", "V", "vi", "IV", "ii7", ["Vsus4", "V"]];
  const b1 = section(song, "body", 8, bProg);
  melody(b1, { ch: 6, rhythms: RHYTHMS.epic, range: [0, 8], octave: 0, vel: 76, motifBars: [0, 4], start: 4, seed: 2 });
  pad(b1, { ch: 5, center: 62, vel: 50 });
  bass(b1, { ch: 8, pattern: [[0, "R", 2, 1], [2, "5", 2, 0.85]], octave: -2, vel: 66 });
  arp(b1, { ch: 10, step: 0.5, pattern: [0, 1, 2, 3, 2, 1, 2, 3], center: 69, vel: 40 });
  roots(b1, { ch: 7, octave: -2, vel: 70, hits: [[0, 1, 1]] });
  drums(b1, { pattern: TAVERN_TAMB, vel: 0.9 });

  const a3 = section(song, "body", 8, THEME_CHORDS);
  line(a3, { ch: 0, notes: MOTIF_THEME, octave: 1, vel: 80 });
  line(a3, { ch: 1, notes: MOTIF_THEME, vel: 76 });
  pad(a3, { ch: 5, center: 62, vel: 54 });
  pad(a3, { ch: 11, center: 60, vel: 40 });
  pad(a3, { ch: 6, center: 57, vel: 48 });
  arp(a3, lute);
  bass(a3, folkBass);
  drums(a3, { pattern: [...TAVERN_PERC, ...TAVERN_TAMB] });
  roots(a3, { ch: 7, octave: -2, vel: 62, hits: [[0, 1, 1]] });

  const dProg = ["I", "iii", "IV", "I", "vi", "iii", "IV", ["IV", "V"]];
  const d1 = section(song, "body", 8, dProg);
  arp(d1, { ch: 3, step: 0.5, pattern: [0, 1, 2, 3, 4, 3, 2, 1], center: 62, vel: 50 });
  melody(d1, { ch: 8, rhythms: RHYTHMS.epic, range: [-3, 5], octave: 0, vel: 70, motifBars: [0], start: 0, seed: 3 });
  pad(d1, { ch: 5, center: 60, vel: 32 });

  const b2 = section(song, "body", 8, bProg);
  melody(b2, { ch: 6, rhythms: RHYTHMS.epic, range: [0, 8], vel: 80, motifBars: [0, 4], start: 4, seed: 2 });
  melody(b2, { ch: 0, rhythms: RHYTHMS.folk, range: [7, 14], vel: 56, motifBars: [], start: 11, seed: 4 });
  pad(b2, { ch: 5, center: 62, vel: 58 });
  pad(b2, { ch: 11, center: 62, vel: 44 });
  bass(b2, { ch: 8, pattern: [[0, "R", 2, 1], [2, "5", 2, 0.85]], octave: -2, vel: 70 });
  bass(b2, folkBass);
  arp(b2, lute);
  roots(b2, { ch: 7, octave: -2, vel: 72, hits: [[0, 1, 1], [2, 1, 0.6]] });
  drums(b2, { pattern: [...TAVERN_PERC, ...TAVERN_TAMB] });

  const a4 = section(song, "body", 8, THEME_CHORDS);
  line(a4, { ch: 0, notes: MOTIF_THEME, vel: 78 });
  arp(a4, lute);
  arp(a4, { ch: 3, step: 1, pattern: [0, 2, 4, 2], center: 67, vel: 40 });
  bass(a4, folkBass);
  drums(a4, { pattern: TAVERN_PERC, vel: 0.8 });
  pad(a4, { ch: 5, center: 60, vel: 34 });
  return { song };
}

/* ------------------------------------------------------------------------------------------- *
 * Tavern
 * ------------------------------------------------------------------------------------------- */

function tavern({ id, tonic, mode, bpm, A, B, lead, second, danger = false }) {
  const song = createSong({ id, bpm, key: key(tonic, mode) });
  song.reverb = { room: 0.55, damp: 0.45, width: 0.8, level: 0.5 };
  instrument(song, 0, lead, { volume: 94, pan: 72, reverb: 50 });
  instrument(song, 1, second, { volume: 86, pan: 52, reverb: 48 });
  instrument(song, 2, GM.nylon, { volume: 94, pan: 42, reverb: 40 });
  instrument(song, 3, GM.harp, { volume: 82, pan: 88, reverb: 55 });
  instrument(song, 4, GM.acBass, { volume: 82, pan: 64, reverb: 25 });
  instrument(song, 5, GM.strings, { volume: 70, pan: 64, reverb: 65 });
  instrument(song, 6, danger ? GM.cello : GM.accordion, { volume: danger ? 90 : 66, pan: 36, reverb: 40 });
  instrument(song, DRUMS, KIT.standard, { volume: 86, reverb: 30 });

  const lute = { ch: 2, step: danger ? 0.25 : 0.5, pattern: danger ? [0, 2, 3, 2] : [0, 2, 3, 4, 3, 2, 1, 2], center: 55, vel: danger ? 46 : 54 };
  const folkBass = { ch: 4, pattern: [[0, "R", 1.4, 1], [1.5, "5", 0.5, 0.7], [2, "5", 1.4, 0.85], [3.5, "A", 0.5, 0.7]], vel: 80 };
  // The in-game builds keep their folk modes and progressions but borrow the low-health build's
  // offbeat shakers and tom hits at a lower level, so the match tracks drive harder throughout.
  const perc = danger
    ? [...TAVERN_PERC, [0.25, DR.shaker, 30], [1.25, DR.shaker, 30], [2.25, DR.shaker, 30], [3.25, DR.shaker, 30], [0, DR.tomLo, 54], [2, DR.tomLo, 48], [3.5, DR.tomMid, 40]]
    : [...TAVERN_PERC, [0.25, DR.shaker, 28], [1.25, DR.shaker, 28], [2.25, DR.shaker, 28], [3.25, DR.shaker, 28], [0, DR.tomLo, 44], [2, DR.tomLo, 40]];
  const fill = [...perc, [3, DR.tomMid, 50], [3.5, DR.tomLo, 56]];

  const s1 = section(song, "body", 8, A);
  arp(s1, lute);
  bass(s1, folkBass);
  drums(s1, { pattern: perc, vel: 0.85 });
  melody(s1, { ch: 0, rhythms: RHYTHMS.folk, range: [3, 11], vel: 80, motifBars: [0, 4], start: 4, seed: 1 });
  if (danger) pulse(s1, { ch: 6, step: 0.5, octave: -2, vel: 58 });

  const s2 = section(song, "body", 8, A);
  arp(s2, lute);
  arp(s2, { ch: 3, step: 1, pattern: [0, 2, 4, 2], center: 67, vel: 42 });
  bass(s2, folkBass);
  drums(s2, { pattern: [...perc, ...TAVERN_TAMB], fill });
  melody(s2, { ch: 1, rhythms: RHYTHMS.folk, range: [2, 10], vel: 78, motifBars: [0], start: 4, seed: 2 });
  if (danger) pulse(s2, { ch: 6, step: 0.5, octave: -2, vel: 60 });
  else pad(s2, { ch: 6, center: 60, vel: 34, every: 2 });

  const s3 = section(song, "body", 8, B);
  pad(s3, { ch: 5, center: 62, vel: danger ? 54 : 46 });
  arp(s3, lute);
  bass(s3, folkBass);
  drums(s3, { pattern: [...TAVERN_TAMB, [0, DR.bongoLo, 60], [2, DR.bongoLo, 54]] });
  melody(s3, { ch: 0, rhythms: RHYTHMS.epic, range: [4, 12], vel: 76, motifBars: [0, 4], start: 7, seed: 3 });
  if (danger) pulse(s3, { ch: 6, step: 0.5, octave: -2, vel: 62 });

  const s4 = section(song, "body", 8, A);
  arp(s4, lute);
  arp(s4, { ch: 3, step: 0.5, pattern: [0, 1, 2, 3, 4, 3, 2, 1], center: 67, vel: 38 });
  bass(s4, folkBass);
  pad(s4, { ch: 5, center: 60, vel: 40 });
  drums(s4, { pattern: [...perc, ...TAVERN_TAMB], fill });
  melody(s4, { ch: 0, rhythms: RHYTHMS.folk, range: [3, 11], vel: 80, motifBars: [0, 4], start: 4, seed: 1 });
  melody(s4, { ch: 1, rhythms: RHYTHMS.epic, range: [-1, 6], vel: 58, motifBars: [], start: 2, seed: 4 });
  if (danger) pulse(s4, { ch: 6, step: 0.5, octave: -2, vel: 62 });

  const s5 = section(song, "body", 8, danger ? B : A);
  arp(s5, { ch: 3, step: 0.5, pattern: [0, 1, 2, 3, 4, 3, 2, 1], center: 62, vel: 50 });
  pad(s5, { ch: 5, center: 60, vel: 36 });
  melody(s5, { ch: 1, rhythms: RHYTHMS.epic, range: [0, 8], vel: 66, motifBars: [0], start: 2, seed: 5 });
  pulse(s5, { ch: 6, step: 0.5, octave: -2, vel: 56 });
  if (danger) {
    drums(s5, { pattern: [[0, DR.tomLo, 60], [2, DR.tomLo, 50], [3, DR.tomMid, 44], [3.5, DR.tomMid, 44]] });
  }

  const s6 = section(song, "body", 8, B);
  pad(s6, { ch: 5, center: 62, vel: 48 });
  arp(s6, lute);
  bass(s6, folkBass);
  drums(s6, { pattern: [...perc, ...TAVERN_TAMB], fill });
  melody(s6, { ch: 0, rhythms: RHYTHMS.epic, range: [4, 12], vel: 78, motifBars: [0, 4], start: 7, seed: 3 });
  melody(s6, { ch: 1, rhythms: RHYTHMS.folk, range: [0, 7], vel: 56, motifBars: [], start: 2, seed: 6 });
  if (danger) pulse(s6, { ch: 6, step: 0.5, octave: -2, vel: 60 });
  else pulse(s6, { ch: 6, step: 0.5, octave: -2, vel: 56 });

  // A reprise of the opening, varied, so the loop runs past two minutes (#51).
  const s7 = section(song, "body", 8, A);
  arp(s7, lute);
  arp(s7, { ch: 3, step: 1, pattern: [0, 2, 4, 2], center: 67, vel: 40 });
  bass(s7, folkBass);
  drums(s7, { pattern: perc, vel: 0.8 });
  melody(s7, { ch: 1, rhythms: RHYTHMS.folk, range: [3, 11], vel: 76, motifBars: [0, 4], start: 4, seed: 7 });
  if (danger) pulse(s7, { ch: 6, step: 0.5, octave: -2, vel: 58 });
  else pad(s7, { ch: 6, center: 60, vel: 30, every: 2 });

  if (!danger) {
    // The faster in-game tempo shortens the loop, so the match builds reprise the opening once
    // more instead of ending on s7.
    const s8 = section(song, "body", 8, A);
    arp(s8, lute);
    arp(s8, { ch: 3, step: 0.5, pattern: [0, 1, 2, 3, 4, 3, 2, 1], center: 67, vel: 42 });
    bass(s8, folkBass);
    drums(s8, { pattern: [...perc, ...TAVERN_TAMB], fill });
    melody(s8, { ch: 0, rhythms: RHYTHMS.folk, range: [4, 12], vel: 80, motifBars: [0, 4], start: 7, seed: 8 });
    pulse(s8, { ch: 6, step: 0.5, octave: -2, vel: 56 });
  }
  return { song };
}

/* ------------------------------------------------------------------------------------------- *
 * EDM
 * ------------------------------------------------------------------------------------------- */

const FOUR_FLOOR = [[0, DR.kick, 92], [1, DR.kick, 88], [2, DR.kick, 92], [3, DR.kick, 88]];
const EDM_HATS = [[0.5, DR.hat, 56], [1.5, DR.hat, 52], [2.5, DR.hat, 56], [3.5, DR.hat, 52]];
const EDM_CLAP = [[1, DR.clap, 70], [3, DR.clap, 70]];
const EDM_BUSY_HATS = [0, 0.25, 0.5, 0.75, 1, 1.25, 1.5, 1.75, 2, 2.25, 2.5, 2.75, 3, 3.25, 3.5, 3.75].map((t) => [t, t % 0.5 === 0 ? DR.hat : DR.pedal, t % 1 === 0.5 ? 58 : 38]);

function edm({ id, tonic, mode, bpm, A, B, danger = false }) {
  const song = createSong({ id, bpm, key: key(tonic, mode), humanize: 0.004 });
  song.reverb = { room: 0.5, damp: 0.5, width: 1, level: 0.45 };
  instrument(song, 0, GM.saw, { volume: 72, pan: 64, reverb: 50 });
  instrument(song, 1, GM.square, { volume: 66, pan: 80, reverb: 45 });
  instrument(song, 2, danger ? GM.polysynth : GM.warmPad, { volume: 80, pan: 64, reverb: 60 });
  instrument(song, 3, GM.synthBass2, { volume: 74, pan: 64, reverb: 10 });
  instrument(song, 4, GM.haloPad, { volume: 62, pan: 48, reverb: 70 });
  instrument(song, 5, GM.voiceLead, { volume: 64, pan: 44, reverb: 55 });
  instrument(song, DRUMS, KIT.electronic, { volume: 80, reverb: 20 });

  const offbeatBass = { ch: 3, pattern: danger
    ? [0, 0.5, 1, 1.5, 2, 2.5, 3, 3.5].map((t) => [t, t % 1 === 0 ? "R" : "8", 0.4, t % 1 === 0 ? 0.8 : 1])
    : [[0.5, "R", 0.4, 1], [1.5, "R", 0.4, 0.9], [2.5, "R", 0.4, 1], [3.5, "8", 0.4, 0.9]], vel: 86, octave: -2 };
  const pluck = { ch: 1, step: 0.25, pattern: [0, 1, 2, 3, 2, 1, 2, 4], center: 67, vel: 54, accent: [1, 0.6, 0.8, 0.6], gate: 0.5 };
  const kicks = danger ? [...FOUR_FLOOR, [3.5, DR.kick, 70]] : FOUR_FLOOR;
  const hats = danger ? EDM_BUSY_HATS : EDM_HATS;
  const roll = [...FOUR_FLOOR, ...EDM_HATS, [2, DR.esnare, 50], [2.5, DR.esnare, 56], [3, DR.esnare, 62], [3.25, DR.esnare, 68], [3.5, DR.esnare, 74], [3.75, DR.esnare, 80]];

  const s1 = section(song, "body", 8, A);
  pad(s1, { ch: 2, center: 60, vel: 58 });
  bass(s1, offbeatBass);
  drums(s1, { pattern: [...kicks, ...hats] });
  arp(s1, { ...pluck, vel: 44 });
  pump(s1, { chs: [2, 3] });

  const s2 = section(song, "body", 8, A);
  pad(s2, { ch: 2, center: 60, vel: 58 });
  bass(s2, offbeatBass);
  arp(s2, pluck);
  drums(s2, { pattern: [...kicks, ...hats, ...EDM_CLAP], fill: roll });
  melody(s2, { ch: 0, rhythms: RHYTHMS.edm, range: [3, 11], vel: 74, motifBars: [0, 4], start: 4, seed: 1 });
  pump(s2, { chs: [2, 3] });

  const s3 = section(song, "body", 8, B);
  pad(s3, { ch: 4, center: 62, vel: 56 });
  pad(s3, { ch: 2, center: 60, vel: 44 });
  drums(s3, { pattern: danger ? [...hats, [0, DR.kick, 80], [2, DR.kick, 74]] : [...EDM_HATS, [0, DR.kick, 70]] });
  melody(s3, { ch: 5, rhythms: RHYTHMS.epic, range: [2, 9], vel: 68, motifBars: [0], start: 4, seed: 2 });
  arp(s3, { ...pluck, vel: 36, from: 16 });
  if (danger) pulse(s3, { ch: 3, step: 0.5, octave: -2, vel: 70 });

  const s4 = section(song, "body", 8, B);
  pad(s4, { ch: 2, center: 60, vel: 58 });
  pad(s4, { ch: 4, center: 64, vel: 44 });
  bass(s4, offbeatBass);
  arp(s4, pluck);
  drums(s4, { pattern: [...kicks, ...hats, ...EDM_CLAP], fill: roll });
  melody(s4, { ch: 0, rhythms: RHYTHMS.edm, range: [4, 12], vel: 74, motifBars: [0, 4], start: 7, seed: 3 });
  pump(s4, { chs: [2, 3, 4] });

  const s5 = section(song, "body", 8, A);
  pad(s5, { ch: 2, center: 60, vel: 60 });
  bass(s5, offbeatBass);
  arp(s5, { ...pluck, pattern: [0, 2, 4, 2, 1, 3, 5, 3] });
  drums(s5, { pattern: [...kicks, ...EDM_BUSY_HATS, ...EDM_CLAP, [0, DR.open, 40], [2, DR.open, 40]] });
  melody(s5, { ch: 0, rhythms: RHYTHMS.edm, range: [3, 11], vel: 76, motifBars: [0, 4], start: 4, seed: 1 });
  melody(s5, { ch: 5, rhythms: RHYTHMS.epic, range: [0, 7], vel: 54, motifBars: [], start: 2, seed: 4 });
  pump(s5, { chs: [2, 3] });

  // s6 doubles the kick: four on the floor plus an offbeat layer under the busy hats.
  const s6 = section(song, "body", 8, A);
  pad(s6, { ch: 4, center: 62, vel: 52 });
  bass(s6, { ...offbeatBass, vel: 70 });
  arp(s6, { ...pluck, vel: 48 });
  drums(s6, {
    pattern: danger
      ? [...EDM_HATS, [0, DR.kick, 84], [2, DR.kick, 80]]
      : [...FOUR_FLOOR, [0.5, DR.kick, 66], [1.5, DR.kick, 62], [2.5, DR.kick, 66], [3.5, DR.kick, 62], ...EDM_BUSY_HATS],
    fill: roll,
  });
  melody(s6, { ch: 5, rhythms: RHYTHMS.edm, range: [2, 9], vel: 62, motifBars: [0], start: 4, seed: 5 });
  pump(s6, { chs: [3, 4] });

  const s7 = section(song, "body", 8, B);
  pad(s7, { ch: 2, center: 60, vel: 58 });
  bass(s7, offbeatBass);
  arp(s7, pluck);
  drums(s7, { pattern: [...kicks, ...hats, ...EDM_CLAP] });
  melody(s7, { ch: 0, rhythms: RHYTHMS.edm, range: [4, 12], vel: 72, motifBars: [0, 4], start: 7, seed: 3 });
  pump(s7, { chs: [2, 3] });

  // The faster tracks run their loop short of two minutes at seven sections, so every build takes
  // an eighth (#51): the low-health build always did, and the match builds now match it.
  const s8 = section(song, "body", 8, A);
  pad(s8, { ch: 2, center: 60, vel: 56 });
  bass(s8, offbeatBass);
  arp(s8, { ...pluck, pattern: [0, 2, 4, 2, 1, 3, 5, 3], vel: 50 });
  drums(s8, { pattern: [...kicks, ...hats, ...EDM_CLAP], fill: roll });
  melody(s8, { ch: 5, rhythms: RHYTHMS.edm, range: [2, 9], vel: 60, motifBars: [0, 4], start: 4, seed: 6 });
  pump(s8, { chs: [2, 3] });
  return { song };
}

/* ------------------------------------------------------------------------------------------- *
 * Lo-fi
 * ------------------------------------------------------------------------------------------- */

const LOFI_DRUMS = [
  [0, DR.kick, 74], [1, DR.stick, 50], [1.75, DR.kick, 52], [2.5, DR.kick, 62], [3, DR.stick, 52],
  [0, DR.hat, 36], [0.5, DR.hat, 28], [1, DR.hat, 34], [1.5, DR.hat, 26], [2, DR.hat, 36], [2.5, DR.hat, 28], [3, DR.hat, 34], [3.5, DR.hat, 30],
];

function lofi({ id, tonic, mode, bpm, A, B, danger = false }) {
  const song = createSong({ id, bpm, key: key(tonic, mode), swing: 0.09, humanize: 0.02 });
  song.reverb = { room: 0.45, damp: 0.6, width: 0.7, level: 0.4 };
  song.post.lofi = { lowpassHz: danger ? 3800 : 4200, drive: 1.6, wowPeriodS: 4.5, wowDepthS: 0.0011, crackle: 0.05, clicksPerS: 3, hiss: 0.02, seed: id.length * 977 };
  instrument(song, 0, GM.ep1, { volume: 92, pan: 58, reverb: 45 });
  instrument(song, 1, danger ? GM.mutedTrumpet : GM.vibes, { volume: 84, pan: 74, reverb: 50 });
  instrument(song, 2, GM.fingerBass, { volume: 84, pan: 64, reverb: 15 });
  instrument(song, 3, GM.warmPad, { volume: 60, pan: 64, reverb: 60 });
  instrument(song, 4, GM.cleanGuitar, { volume: 74, pan: 40, reverb: 50 });
  instrument(song, 5, GM.flute, { volume: 70, pan: 82, reverb: 55 });
  instrument(song, DRUMS, KIT.brush, { volume: 90, reverb: 25 });

  const keys = { ch: 0, hits: [[0, 1.4, 1], [1.5, 2.3, 0.8]], center: 62, vel: 58 };
  const walk = { ch: 2, pattern: [[0, "R", 1.4, 1], [1.5, "5", 0.4, 0.6], [2.5, "8", 0.9, 0.75], [3.5, "A", 0.4, 0.6]], vel: 78 };
  const beat = danger ? [...LOFI_DRUMS, [0.5, DR.kick, 46], [2, DR.tomLo, 34]] : LOFI_DRUMS;
  // The match builds double the brushed-drums density — offbeat hats, an extra kick and stick —
  // under a low pulse, so the swung groove drives instead of lounges.
  const drive = danger ? beat : [...LOFI_DRUMS,
    [0.25, DR.hat, 24], [0.75, DR.hat, 22], [1.25, DR.hat, 24], [1.75, DR.hat, 22],
    [2.25, DR.hat, 24], [2.75, DR.hat, 22], [3.25, DR.hat, 24], [3.75, DR.hat, 22],
    [1.5, DR.kick, 44], [3.5, DR.stick, 40]];
  const sparse = [[0, DR.kick, 50], [2.5, DR.kick, 40], [0, DR.hat, 24], [2, DR.hat, 24]];

  const s1 = section(song, "body", 8, A);
  comp(s1, keys);
  bass(s1, walk);
  drums(s1, { pattern: beat });
  if (danger) pulse(s1, { ch: 3, step: 1, octave: -1, vel: 46, gate: 0.9 });

  const s2 = section(song, "body", 8, A);
  comp(s2, keys);
  bass(s2, walk);
  drums(s2, { pattern: beat });
  melody(s2, { ch: 1, rhythms: RHYTHMS.lofi, range: [2, 10], vel: 70, motifBars: [0, 4], start: 4, seed: 1 });
  pad(s2, { ch: 3, center: 60, vel: 36 });

  const s3 = section(song, "body", 8, B);
  comp(s3, { ...keys, hits: [[0, 0.9, 1], [1.5, 0.4, 0.7], [2.5, 1.4, 0.8]] });
  bass(s3, walk);
  drums(s3, { pattern: beat });
  arp(s3, { ch: 4, step: 0.5, pattern: [0, 2, 1, 3, null, 2, null, 1], center: 64, vel: 46, gate: 0.7 });
  melody(s3, { ch: 5, rhythms: RHYTHMS.lofi, range: [4, 11], vel: 60, motifBars: [0], start: 7, seed: 2 });

  const s4 = section(song, "body", 8, A);
  comp(s4, keys);
  bass(s4, walk);
  drums(s4, { pattern: beat });
  pad(s4, { ch: 3, center: 60, vel: 40 });
  melody(s4, { ch: 1, rhythms: RHYTHMS.lofi, range: [2, 10], vel: 70, motifBars: [0, 4], start: 4, seed: 3 });
  if (danger) pulse(s4, { ch: 3, step: 1, octave: -1, vel: 46, gate: 0.9 });

  const s5 = section(song, "body", 4, A);
  comp(s5, { ...keys, hits: [[0, 3.8, 0.9]] });
  pad(s5, { ch: 3, center: 60, vel: 34 });
  drums(s5, { pattern: danger ? sparse : drive });
  if (!danger) pulse(s5, { ch: 3, step: 1, octave: -1, vel: 54, gate: 0.9 });

  const s6 = section(song, "body", 8, B);
  comp(s6, keys);
  bass(s6, walk);
  drums(s6, { pattern: danger ? beat : drive });
  melody(s6, { ch: 1, rhythms: RHYTHMS.lofi, range: [3, 11], vel: 66, motifBars: [0], start: 6, seed: 4 });
  arp(s6, { ch: 4, step: 0.5, pattern: [0, 2, 1, 3, null, 2, null, 1], center: 64, vel: 40, gate: 0.7 });
  if (danger) pulse(s6, { ch: 3, step: 1, octave: -1, vel: 46, gate: 0.9 });
  else pulse(s6, { ch: 3, step: 1, octave: -1, vel: 54, gate: 0.9 });

  if (!danger) {
    // The faster match tempo shortens the loop, so the match builds reprise the opening (#51).
    const s7 = section(song, "body", 8, A);
    comp(s7, keys);
    bass(s7, walk);
    drums(s7, { pattern: drive });
    pad(s7, { ch: 3, center: 60, vel: 38 });
    melody(s7, { ch: 1, rhythms: RHYTHMS.lofi, range: [3, 11], vel: 70, motifBars: [0, 4], start: 6, seed: 5 });
    pulse(s7, { ch: 3, step: 1, octave: -1, vel: 54, gate: 0.9 });
  }
  return { song };
}

/* ------------------------------------------------------------------------------------------- *
 * Epic orchestral
 * ------------------------------------------------------------------------------------------- */

function epic({ id, tonic, mode, bpm, A, B, danger = false }) {
  const song = createSong({ id, bpm, key: key(tonic, mode) });
  song.reverb = { room: 0.8, damp: 0.3, width: 1, level: 0.62 };
  instrument(song, 0, GM.strings, { volume: 82, pan: 56, reverb: 60 });
  instrument(song, 1, GM.strings2, { volume: 72, pan: 70, reverb: 75 });
  instrument(song, 2, GM.horn, { volume: 88, pan: 50, reverb: 70 });
  instrument(song, 3, GM.brass, { volume: 64, pan: 64, reverb: 70 });
  instrument(song, 4, GM.choir, { volume: 66, pan: 64, reverb: 80 });
  instrument(song, 5, GM.contrabass, { volume: 82, pan: 60, reverb: 50 });
  instrument(song, 6, GM.timpani, { volume: 86, pan: 64, reverb: 65 });
  instrument(song, 7, GM.harp, { volume: 80, pan: 90, reverb: 60 });
  instrument(song, 8, GM.flute, { volume: 80, pan: 76, reverb: 60 });
  instrument(song, DRUMS, KIT.orchestra, { volume: 76, reverb: 60 });

  const ostinato = danger
    ? { ch: 0, step: 0.25, pattern: [0, 0, 1, 0, 2, 0, 1, 0], center: 50, vel: 58, gate: 0.55, accent: [1, 0.7, 0.8, 0.7] }
    : { ch: 0, step: 0.5, pattern: [0, 1, 2, 1, 0, 2, 3, 2], center: 52, vel: 50, gate: 0.6 };
  const low = { ch: 5, pattern: [[0, "R", 1.9, 1], [2, "R", 1.9, 0.8]], vel: 72, octave: -2 };
  const timp = { ch: 6, octave: -2, vel: danger ? 76 : 60, hits: danger ? [[0, 0.5, 1], [0.5, 0.25, 0.6], [2, 0.5, 0.9], [3.5, 0.25, 0.7]] : [[0, 1, 1]] };

  const s1 = section(song, "body", 8, A);
  arp(s1, ostinato);
  bass(s1, low);
  arp(s1, { ch: 7, step: 0.5, pattern: [0, 1, 2, 3, 4, 3, 2, 1], center: 62, vel: 40 });
  melody(s1, { ch: 2, rhythms: RHYTHMS.epic, range: [0, 8], vel: 74, motifBars: [0, 4], start: 4, seed: 1 });
  if (danger) roots(s1, timp);

  const s2 = section(song, "body", 8, A);
  arp(s2, ostinato);
  bass(s2, low);
  pad(s2, { ch: 4, center: 62, vel: 46 });
  pad(s2, { ch: 1, center: 57, vel: 44 });
  melody(s2, { ch: 1, rhythms: RHYTHMS.epic, range: [4, 11], vel: 72, motifBars: [0], start: 7, seed: 2 });
  roots(s2, timp);

  const s3 = section(song, "body", 8, B);
  arp(s3, ostinato);
  bass(s3, low);
  comp(s3, { ch: 3, hits: [[0, 1.8, 1], [2, 1.8, 0.8]], center: 57, vel: danger ? 64 : 48 });
  pad(s3, { ch: 4, center: 62, vel: 50 });
  melody(s3, { ch: 2, rhythms: RHYTHMS.epic, range: [2, 9], vel: 78, motifBars: [0, 4], start: 4, seed: 3 });
  roots(s3, timp);
  drums(s3, { pattern: [[0, DR.kick2, 54]], skip: (bar) => bar % 4 !== 0 });

  const s4 = section(song, "body", 8, A);
  arp(s4, ostinato);
  bass(s4, low);
  pad(s4, { ch: 1, center: 57, vel: 50 });
  pad(s4, { ch: 4, center: 64, vel: 52 });
  melody(s4, { ch: 2, rhythms: RHYTHMS.epic, range: [0, 8], vel: 78, motifBars: [0, 4], start: 4, seed: 1 });
  melody(s4, { ch: 8, rhythms: RHYTHMS.folk, range: [7, 14], vel: 54, motifBars: [], start: 9, seed: 4 });
  roots(s4, timp);
  drums(s4, { pattern: [[0, DR.crash, 44]], skip: (bar) => bar !== 0 });

  const s5 = section(song, "body", 8, danger ? B : A);
  arp(s5, { ch: 7, step: 0.5, pattern: [0, 1, 2, 3, 4, 3, 2, 1], center: 62, vel: 48 });
  pad(s5, { ch: 4, center: 60, vel: 42 });
  pad(s5, { ch: 1, center: 55, vel: danger ? 34 : 42 });
  melody(s5, { ch: 8, rhythms: RHYTHMS.epic, range: [4, 11], vel: 66, motifBars: [0], start: 7, seed: 5 });
  if (danger) {
    arp(s5, ostinato);
    roots(s5, timp);
  } else {
    // Timpani under the match build too, and louder strings beside them.
    roots(s5, { ch: 6, octave: -2, vel: 66, hits: [[0, 1, 1]] });
  }

  const s6 = section(song, "body", 8, B);
  arp(s6, ostinato);
  bass(s6, low);
  comp(s6, { ch: 3, hits: [[0, 1.8, 1], [2, 1.8, 0.8]], center: 57, vel: danger ? 62 : 54 });
  pad(s6, { ch: 4, center: 62, vel: 54 });
  pad(s6, { ch: 1, center: 57, vel: danger ? 48 : 56 });
  melody(s6, { ch: 2, rhythms: RHYTHMS.epic, range: [2, 9], vel: 80, motifBars: [0, 4], start: 4, seed: 3 });
  roots(s6, danger ? timp : { ...timp, hits: [[0, 1, 1], [3, 0.5, 0.6], [3.5, 0.5, 0.7]] });

  if (!danger) {
    // The faster match tempo shortens the loop, so the match builds reprise the opening (#51).
    const s7 = section(song, "body", 8, A);
    arp(s7, ostinato);
    bass(s7, low);
    pad(s7, { ch: 4, center: 64, vel: 52 });
    pad(s7, { ch: 1, center: 57, vel: 56 });
    melody(s7, { ch: 2, rhythms: RHYTHMS.epic, range: [0, 8], vel: 78, motifBars: [0, 4], start: 4, seed: 6 });
    roots(s7, { ch: 6, octave: -2, vel: 66, hits: [[0, 1, 1], [3, 0.5, 0.6], [3.5, 0.5, 0.7]] });
    drums(s7, { pattern: [[0, DR.crash, 44]], skip: (bar) => bar !== 0 });
  }
  return { song };
}

/* ------------------------------------------------------------------------------------------- *
 * Stings: a match start per station, and the results
 * ------------------------------------------------------------------------------------------- */

/** The fanfare (the theme's first two bars) over I | vi V, closing on a held tonic chord. */
function matchStart(station) {
  const plans = {
    tavern: { tonic: "D", mode: "major", bpm: 96, lead: GM.flute, chord: GM.nylon, low: GM.acBass, kit: KIT.standard },
    edm: { tonic: "A", mode: "minor", bpm: 112, lead: GM.saw, chord: GM.warmPad, low: GM.synthBass2, kit: KIT.electronic },
    lofi: { tonic: "F", mode: "major", bpm: 82, lead: GM.vibes, chord: GM.ep1, low: GM.fingerBass, kit: KIT.brush },
    epic: { tonic: "D", mode: "minor", bpm: 88, lead: GM.horn, chord: GM.strings, low: GM.contrabass, kit: KIT.orchestra },
  };
  const p = plans[station];
  const song = createSong({ id: `${station}-start`, bpm: p.bpm, key: key(p.tonic, p.mode), swing: station === "lofi" ? 0.09 : 0 });
  song.reverb = { room: 0.7, damp: 0.4, width: 0.9, level: 0.6 };
  if (station === "lofi") song.post.lofi = { lowpassHz: 4200, drive: 1.6, wowPeriodS: 4.5, wowDepthS: 0.0011, crackle: 0.05, clicksPerS: 3, hiss: 0.02, seed: 4021 };
  instrument(song, 0, p.lead, { volume: 96, reverb: 60 });
  instrument(song, 1, p.chord, { volume: 86, reverb: 60 });
  instrument(song, 2, p.low, { volume: 92, reverb: 30 });
  instrument(song, 3, GM.timpani, { volume: 80, reverb: 60 });
  instrument(song, DRUMS, p.kit, { volume: 84, reverb: 40 });
  const s = section(song, "intro", 3, p.mode === "minor" ? ["i", ["VI", "v"], "i"] : ["I", ["vi", "V"], "I"]);
  line(s, { ch: 0, notes: [...FANFARE, [0, 4]], vel: 88 });
  comp(s, { ch: 1, hits: [[0, 1.9, 1]], center: 60, vel: 62 });
  bass(s, { ch: 2, pattern: [[0, "R", 1.9, 1]], vel: 80 });
  if (station === "edm") drums(s, { pattern: [...FOUR_FLOOR, ...EDM_HATS] });
  else if (station === "lofi") drums(s, { pattern: LOFI_DRUMS });
  else roots(s, { ch: 3, octave: -2, vel: 74, hits: [[0, 1, 1]] });
  return { song };
}

function victory() {
  const song = createSong({ id: "victory", bpm: 100, key: key("D", "major") });
  song.reverb = { room: 0.75, damp: 0.35, width: 1, level: 0.6 };
  instrument(song, 0, GM.trumpet, { volume: 90, reverb: 60 });
  instrument(song, 1, GM.brass, { volume: 78, reverb: 65 });
  instrument(song, 2, GM.strings, { volume: 82, reverb: 70 });
  instrument(song, 3, GM.timpani, { volume: 88, reverb: 60 });
  instrument(song, 4, GM.harp, { volume: 82, pan: 88, reverb: 60 });
  instrument(song, 5, GM.flute, { volume: 86, pan: 70, reverb: 60 });
  instrument(song, 6, GM.contrabass, { volume: 84, reverb: 50 });
  instrument(song, DRUMS, KIT.orchestra, { volume: 80, reverb: 60 });
  const sting = section(song, "intro", 3, ["I", ["IV", "V"], "I"]);
  line(sting, { ch: 0, notes: [[4, 0.5], [5, 0.5], [7, 1.5], [6, 0.5], [5, 0.5], [4, 0.5], [5, 1], [6, 1], [7, 4]], vel: 92 });
  comp(sting, { ch: 1, hits: [[0, 0.9, 1], [1, 0.9, 0.8], [2, 1.9, 0.9]], center: 57, vel: 70 });
  pad(sting, { ch: 2, center: 62, vel: 64 });
  roots(sting, { ch: 3, octave: -2, vel: 84, hits: [[0, 0.5, 1], [3, 0.25, 0.6], [3.5, 0.25, 0.8]] });
  drums(sting, { pattern: [[0, DR.crash, 70]], skip: (bar) => bar !== 2 });
  const loop = section(song, "body", 8, ["I", "IV", "vi", "V", "vi", "IV", "V7", "I"]);
  arp(loop, { ch: 4, step: 0.5, pattern: [0, 1, 2, 3, 4, 3, 2, 1], center: 62, vel: 46 });
  pad(loop, { ch: 2, center: 60, vel: 40 });
  line(loop, { ch: 5, notes: MOTIF_THEME.slice(18), vel: 66, at: 16 });
  bass(loop, { ch: 6, pattern: [[0, "R", 3.8, 1]], vel: 60 });
  return { song };
}

function defeat() {
  const song = createSong({ id: "defeat", bpm: 72, key: key("D", "minor") });
  song.reverb = { room: 0.8, damp: 0.4, width: 1, level: 0.62 };
  instrument(song, 0, GM.cello, { volume: 92, reverb: 65 });
  instrument(song, 1, GM.strings, { volume: 78, reverb: 75 });
  instrument(song, 2, GM.harp, { volume: 80, pan: 84, reverb: 65 });
  instrument(song, 3, GM.timpani, { volume: 82, reverb: 65 });
  instrument(song, 4, GM.oboe, { volume: 82, pan: 56, reverb: 60 });
  instrument(song, 5, GM.contrabass, { volume: 84, reverb: 50 });
  const sting = section(song, "intro", 3, ["i", ["iv", "V+"], "i"]);
  line(sting, { ch: 4, notes: [[4, 0.75], [5, 0.75], [7, 2.5], [6, 1], [5, 1], [4, 1], [3, 1], [0, 4]], vel: 82 });
  pad(sting, { ch: 1, center: 57, vel: 56 });
  roots(sting, { ch: 3, octave: -2, vel: 78, hits: [[0, 1, 1]] });
  bass(sting, { ch: 5, pattern: [[0, "R", 1.9, 1]], vel: 70 });
  const loop = section(song, "body", 8, ["i", "VI", "iv", "V+", "i", "VI", ["iv", "V+"], "i"]);
  arp(loop, { ch: 2, step: 0.5, pattern: [0, 1, 2, 3, 2, 1, 2, 1], center: 60, vel: 42 });
  pad(loop, { ch: 1, center: 57, vel: 36 });
  melody(loop, { ch: 0, rhythms: RHYTHMS.epic, range: [-3, 5], vel: 64, motifBars: [0, 4], start: 0, seed: 1 });
  return { song };
}

function draw() {
  const song = createSong({ id: "draw", bpm: 84, key: key("G", "mixolydian") });
  song.reverb = { room: 0.75, damp: 0.4, width: 1, level: 0.6 };
  instrument(song, 0, GM.musicBox, { volume: 84, pan: 70, reverb: 70 });
  instrument(song, 1, GM.strings, { volume: 74, reverb: 75 });
  instrument(song, 2, GM.harp, { volume: 80, pan: 40, reverb: 65 });
  instrument(song, 3, GM.clarinet, { volume: 82, pan: 56, reverb: 60 });
  instrument(song, 4, GM.acBass, { volume: 80, reverb: 30 });
  const sting = section(song, "intro", 3, ["I", ["IVsus2", "vii"], "IVsus2"]);
  line(sting, { ch: 3, notes: [[4, 0.5], [5, 0.5], [7, 1.5], [6, 0.5], [5, 0.5], [4, 0.5], [3, 2], [3, 4]], vel: 80 });
  pad(sting, { ch: 1, center: 60, vel: 52 });
  const loop = section(song, "body", 8, ["Isus2", "IVsus2", "vii", "IVsus2", "Isus2", "v", "IVsus2", ["vii", "Isus4"]]);
  arp(loop, { ch: 2, step: 0.5, pattern: [0, 2, 1, 3, 2, 4, 3, 1], center: 62, vel: 42 });
  pad(loop, { ch: 1, center: 60, vel: 34 });
  melody(loop, { ch: 0, rhythms: RHYTHMS.lofi, range: [4, 12], vel: 58, motifBars: [0], start: 7, seed: 1, cadence: false });
  bass(loop, { ch: 4, pattern: [[0, "R", 3.8, 1]], vel: 58 });
  return { song };
}

/* ------------------------------------------------------------------------------------------- *
 * Mythic themes (src/audio/music-cards.json names which card plays which)
 * ------------------------------------------------------------------------------------------- */

/** #96 My Pawn: a chess problem in a baroque hall, harpsichord and pizzicato. */
function myPawn() {
  const song = createSong({ id: "mythic-my-pawn", bpm: 104, key: key("E", "harmonic") });
  song.reverb = { room: 0.6, damp: 0.4, width: 0.9, level: 0.5 };
  instrument(song, 0, GM.harpsichord, { volume: 90, pan: 56, reverb: 45 });
  instrument(song, 1, GM.pizz, { volume: 86, pan: 76, reverb: 50 });
  instrument(song, 2, GM.bassoon, { volume: 84, pan: 46, reverb: 50 });
  instrument(song, 3, GM.strings, { volume: 64, reverb: 65 });
  const intro = section(song, "intro", 1, ["V+"]);
  arp(intro, { ch: 0, step: 0.25, pattern: [0, 1, 2, 3, 4, 5, 6, 7, 6, 5, 4, 3, 2, 1, 2, 3], center: 64, vel: 66 });
  const prog = ["i", "iv", "V+", "i", "VI", "iv", ["ii", "V+"], "i"];
  const body = section(song, "body", 8, prog);
  arp(body, { ch: 0, step: 0.25, pattern: [0, 2, 1, 2, 0, 2, 1, 2, 3, 2, 1, 2, 0, 2, 1, 2], center: 60, vel: 52 });
  melody(body, { ch: 1, rhythms: RHYTHMS.folk, range: [4, 11], vel: 76, motifBars: [0, 4], start: 4, seed: 1 });
  bass(body, { ch: 2, pattern: [[0, "R", 0.9, 1], [1, "5", 0.9, 0.8], [2, "8", 0.9, 0.85], [3, "5", 0.9, 0.8]], octave: -1, vel: 70 });
  pad(body, { ch: 3, center: 60, vel: 30 });
  return { song };
}

/** #97 Zephyrs: a west wind, flutes over shimmering strings and harp. */
function zephyrs() {
  const song = createSong({ id: "mythic-zephyrs", bpm: 92, key: key("F", "lydian") });
  song.reverb = { room: 0.85, damp: 0.3, width: 1, level: 0.65 };
  instrument(song, 0, GM.flute, { volume: 92, pan: 70, reverb: 70 });
  instrument(song, 1, GM.panFlute, { volume: 80, pan: 46, reverb: 70 });
  instrument(song, 2, GM.tremolo, { volume: 66, reverb: 75 });
  instrument(song, 3, GM.harp, { volume: 84, pan: 90, reverb: 70 });
  instrument(song, 4, GM.fretless, { volume: 80, reverb: 40 });
  const intro = section(song, "intro", 1, ["I"]);
  arp(intro, { ch: 3, step: 0.125, pattern: [0, 1, 2, 3, 4, 5, 6, 7, 8, 7, 6, 5, 4, 3, 2, 1], center: 65, vel: 56 });
  const body = section(song, "body", 8, ["I", "ii", "I", "vii", "I", "ii", "IV", "V"]);
  melody(body, { ch: 0, rhythms: RHYTHMS.epic, range: [4, 12], vel: 76, motifBars: [0, 4], start: 7, seed: 1 });
  melody(body, { ch: 1, rhythms: RHYTHMS.waltz.map((r) => [...r, [1]]), range: [0, 7], vel: 54, motifBars: [], start: 4, seed: 2 });
  pad(body, { ch: 2, center: 62, vel: 42 });
  arp(body, { ch: 3, step: 0.5, pattern: [0, 2, 4, 5, 4, 2, 1, 3], center: 62, vel: 42 });
  bass(body, { ch: 4, pattern: [[0, "R", 1.9, 1], [2, "5", 1.9, 0.8]], vel: 62 });
  return { song };
}

/** #98 Heroic Power: the motif as a brass fanfare over timpani. */
function heroicPower() {
  const song = createSong({ id: "mythic-heroic-power", bpm: 96, key: key("C", "major") });
  song.reverb = { room: 0.8, damp: 0.3, width: 1, level: 0.62 };
  instrument(song, 0, GM.trumpet, { volume: 90, pan: 60, reverb: 60 });
  instrument(song, 1, GM.horn, { volume: 86, pan: 48, reverb: 65 });
  instrument(song, 2, GM.strings, { volume: 80, reverb: 70 });
  instrument(song, 3, GM.timpani, { volume: 86, reverb: 60 });
  instrument(song, 4, GM.trombone, { volume: 80, pan: 70, reverb: 60 });
  instrument(song, 5, GM.contrabass, { volume: 84, reverb: 50 });
  instrument(song, DRUMS, KIT.orchestra, { volume: 76, reverb: 60 });
  const intro = section(song, "intro", 1, ["V"]);
  roots(intro, { ch: 3, octave: -2, vel: 70, hits: [0, 0.25, 0.5, 0.75, 1, 1.25, 1.5, 1.75, 2, 2.25, 2.5, 2.75, 3, 3.25, 3.5, 3.75].map((t, i) => [t, 0.25, 0.5 + i / 32]) });
  const body = section(song, "body", 8, THEME_CHORDS);
  line(body, { ch: 0, notes: MOTIF_THEME, vel: 84 });
  comp(body, { ch: 1, hits: [[0, 0.9, 1], [1.5, 0.4, 0.7], [2, 1.9, 0.85]], center: 57, vel: 56 });
  pad(body, { ch: 2, center: 62, vel: 50 });
  bass(body, { ch: 4, pattern: [[0, "R", 0.9, 1], [2, "5", 0.9, 0.8]], octave: -1, vel: 62 });
  bass(body, { ch: 5, pattern: [[0, "R", 1.9, 1], [2, "R", 1.9, 0.8]], vel: 70 });
  roots(body, { ch: 3, octave: -2, vel: 66, hits: [[0, 1, 1], [3.5, 0.5, 0.6]] });
  return { song };
}

/** #99 Craft a Card: a workshop, marimba and kalimba with woodblocks. */
function craftACard() {
  const song = createSong({ id: "mythic-craft-a-card", bpm: 108, key: key("G", "major") });
  song.reverb = { room: 0.45, damp: 0.5, width: 0.8, level: 0.4 };
  instrument(song, 0, GM.marimba, { volume: 92, pan: 56, reverb: 40 });
  instrument(song, 1, GM.kalimba, { volume: 86, pan: 78, reverb: 45 });
  instrument(song, 2, GM.pizz, { volume: 80, pan: 40, reverb: 40 });
  instrument(song, 3, GM.clarinet, { volume: 80, pan: 64, reverb: 45 });
  instrument(song, DRUMS, KIT.standard, { volume: 80, reverb: 25 });
  const intro = section(song, "intro", 1, ["V"]);
  arp(intro, { ch: 1, step: 0.25, pattern: [0, 1, 2, 3, 4, 5, 6, 7, 8, 7, 6, 5, 4, 3, 2, 1], center: 67, vel: 60 });
  const body = section(song, "body", 8, ["I", "V", "vi", "IV", "I", "V", ["IV", "V"], "I"]);
  arp(body, { ch: 0, step: 0.5, pattern: [0, 2, 1, 2, 3, 2, 1, 2], center: 64, vel: 54 });
  melody(body, { ch: 1, rhythms: RHYTHMS.folk, range: [4, 12], vel: 74, motifBars: [0, 4], start: 7, seed: 1 });
  melody(body, { ch: 3, rhythms: RHYTHMS.epic, range: [-1, 6], vel: 50, motifBars: [], start: 2, seed: 2 });
  bass(body, { ch: 2, pattern: [[0, "R", 0.5, 1], [1, "5", 0.5, 0.8], [2, "8", 0.5, 0.9], [3, "5", 0.5, 0.8]], octave: -1, vel: 70 });
  drums(body, { pattern: [[0, DR.blockLo, 56], [1, DR.blockHi, 44], [1.5, DR.blockHi, 36], [2, DR.blockLo, 52], [3, DR.blockHi, 44], [3.5, DR.claves, 40]] });
  return { song };
}

/** #100 Ceaseless Void: a dark choir over a drone, with stars of celesta. */
function ceaselessVoid() {
  const song = createSong({ id: "mythic-ceaseless-void", bpm: 70, key: key("C", "phrygian") });
  song.reverb = { room: 0.95, damp: 0.25, width: 1, level: 0.75 };
  instrument(song, 0, GM.choir, { volume: 84, reverb: 85 });
  instrument(song, 1, GM.metalPad, { volume: 66, reverb: 80 });
  instrument(song, 2, GM.contrabass, { volume: 88, reverb: 60 });
  instrument(song, 3, GM.celesta, { volume: 74, pan: 84, reverb: 85 });
  instrument(song, 4, GM.timpani, { volume: 76, reverb: 75 });
  const intro = section(song, "intro", 1, ["i"]);
  roots(intro, { ch: 4, octave: -2, vel: 70, hits: [[0, 1, 1]] });
  pad(intro, { ch: 1, center: 55, vel: 50 });
  const body = section(song, "body", 8, ["i", "ii", "i", "vii", "i", "ii", "vi", "vii"]);
  pad(body, { ch: 0, center: 58, vel: 52 });
  pad(body, { ch: 1, center: 52, vel: 38 });
  roots(body, { ch: 2, octave: -2, vel: 64 });
  melody(body, { ch: 3, rhythms: RHYTHMS.lofi, range: [7, 14], vel: 52, motifBars: [0, 4], start: 11, seed: 1, cadence: false });
  roots(body, { ch: 4, octave: -2, vel: 54, hits: [[0, 1, 1]] });
  return { song };
}

/** Classic #90 In Too Deep: under water, a halo pad, harp and vibraphone, slow. */
function inTooDeep() {
  const song = createSong({ id: "mythic-in-too-deep", bpm: 76, key: key("Eb", "lydian") });
  song.reverb = { room: 0.9, damp: 0.5, width: 1, level: 0.7 };
  instrument(song, 0, GM.haloPad, { volume: 76, reverb: 80 });
  instrument(song, 1, GM.harp, { volume: 84, pan: 86, reverb: 75 });
  instrument(song, 2, GM.vibes, { volume: 82, pan: 50, reverb: 75 });
  instrument(song, 3, GM.fretless, { volume: 84, reverb: 50 });
  instrument(song, 4, GM.atmosphere, { volume: 60, reverb: 85 });
  const intro = section(song, "intro", 1, ["I"]);
  arp(intro, { ch: 1, step: 0.25, pattern: [8, 7, 6, 5, 4, 3, 2, 1, 0, 1, 2, 3, 4, 3, 2, 1], center: 62, vel: 52 });
  const body = section(song, "body", 8, ["I", "ii", "vi", "I", "IV", "ii", "vi", "V"]);
  pad(body, { ch: 0, center: 60, vel: 50 });
  arp(body, { ch: 1, step: 0.5, pattern: [0, 2, 4, 3, 1, 3, 5, 2], center: 60, vel: 44 });
  melody(body, { ch: 2, rhythms: RHYTHMS.lofi, range: [4, 11], vel: 64, motifBars: [0, 4], start: 7, seed: 1 });
  bass(body, { ch: 3, pattern: [[0, "R", 2.9, 1], [3, "5", 0.9, 0.7]], vel: 62 });
  pad(body, { ch: 4, center: 67, vel: 32 });
  return { song };
}

/** Classic+ #27 Zephrys Zealotism: a zealots' chant, choir and organ over timpani. */
function zephrysZealotism() {
  const song = createSong({ id: "mythic-zephrys-zealotism", bpm: 84, key: key("D", "dorian") });
  song.reverb = { room: 0.9, damp: 0.3, width: 1, level: 0.7 };
  instrument(song, 0, GM.choir, { volume: 88, reverb: 85 });
  instrument(song, 1, GM.organ, { volume: 68, reverb: 75 });
  instrument(song, 2, GM.timpani, { volume: 84, reverb: 65 });
  instrument(song, 3, GM.panFlute, { volume: 80, pan: 76, reverb: 70 });
  instrument(song, 4, GM.contrabass, { volume: 84, reverb: 55 });
  const intro = section(song, "intro", 1, ["i"]);
  roots(intro, { ch: 2, octave: -2, vel: 74, hits: [[0, 0.5, 1], [1, 0.5, 0.7], [2, 0.5, 0.9], [3, 0.5, 0.7]] });
  const body = section(song, "body", 8, ["i", "IV", "i", "VII", "i", "IV", "v", "i"]);
  melody(body, { ch: 0, rhythms: RHYTHMS.epic, range: [2, 9], vel: 70, motifBars: [0, 4], start: 4, seed: 1 });
  pad(body, { ch: 1, center: 57, vel: 44 });
  roots(body, { ch: 2, octave: -2, vel: 64, hits: [[0, 0.5, 1], [2, 0.5, 0.8], [3.5, 0.5, 0.6]] });
  melody(body, { ch: 3, rhythms: RHYTHMS.folk, range: [7, 13], vel: 52, motifBars: [], start: 9, seed: 2 });
  bass(body, { ch: 4, pattern: [[0, "R", 3.9, 1]], vel: 64 });
  return { song };
}

/** Classic+ #29 Portal to the Past: a clockwork music box that winds time back — celesta and
 *  musicBox falling figures over strings, the arps running down where the others rise. */
function portalToThePast() {
  const song = createSong({ id: "mythic-portal-to-the-past", bpm: 88, key: key("A", "minor") });
  song.reverb = { room: 0.85, damp: 0.35, width: 1, level: 0.65 };
  instrument(song, 0, GM.musicBox, { volume: 88, pan: 64, reverb: 70 });
  instrument(song, 1, GM.celesta, { volume: 80, pan: 40, reverb: 70 });
  instrument(song, 2, GM.strings, { volume: 70, reverb: 75 });
  instrument(song, 3, GM.harp, { volume: 78, pan: 84, reverb: 70 });
  instrument(song, 4, GM.contrabass, { volume: 80, reverb: 55 });
  const intro = section(song, "intro", 1, ["i"]);
  arp(intro, { ch: 0, step: 0.25, pattern: [7, 6, 5, 4, 3, 2, 1, 0, 1, 2, 3, 4, 5, 6, 7, 8], center: 67, vel: 60 });
  const body = section(song, "body", 8, ["i", "VI", "iv", "V+", "i", "VI", "ii", "V+"]);
  melody(body, { ch: 0, rhythms: RHYTHMS.lofi, range: [7, 14], vel: 70, motifBars: [0, 4], start: 9, seed: 1 });
  arp(body, { ch: 1, step: 0.5, pattern: [0, 2, 4, 2, 5, 4, 2, 4], center: 62, vel: 46 });
  pad(body, { ch: 2, center: 57, vel: 44 });
  arp(body, { ch: 3, step: 0.5, pattern: [4, 3, 2, 1, 0, 1, 2, 3], center: 69, vel: 40 });
  bass(body, { ch: 4, pattern: [[0, "R", 1.9, 1], [2, "5", 1.9, 0.8]], vel: 62 });
  return { song };
}

/** Classic+ #74 Twice Forward One Step Backwards: a lopsided waltz whose motif walks back. */
function twiceForward() {
  const song = createSong({ id: "mythic-twice-forward", bpm: 132, beatsPerBar: 3, key: key("A", "minor") });
  song.reverb = { room: 0.55, damp: 0.45, width: 0.8, level: 0.45 };
  instrument(song, 0, GM.clarinet, { volume: 90, pan: 60, reverb: 45 });
  instrument(song, 1, GM.bassoon, { volume: 84, pan: 44, reverb: 45 });
  instrument(song, 2, GM.pizz, { volume: 84, pan: 76, reverb: 45 });
  instrument(song, 3, GM.accordion, { volume: 60, pan: 64, reverb: 40 });
  const intro = section(song, "intro", 2, ["i", "V+"]);
  line(intro, { ch: 0, notes: [[4, 0.5], [5, 0.5], [7, 1], [7, 0.5], [5, 0.5], [4, 1]], vel: 78 });
  const body = section(song, "body", 16, ["i", "i", "iv", "iv", "V+", "V+", "i", "i", "VI", "VI", "iv", "iv", "ii", "V+", "i", "V+"]);
  melody(body, { ch: 0, rhythms: RHYTHMS.waltz, range: [2, 10], vel: 74, motifBars: [0, 8], start: 4, seed: 1 });
  bass(body, { ch: 1, pattern: [[0, "R", 0.9, 1]], octave: -1, vel: 72 });
  comp(body, { ch: 2, hits: [[1, 0.4, 0.8], [2, 0.4, 0.7]], center: 60, vel: 54 });
  pad(body, { ch: 3, center: 60, vel: 30 });
  return { song };
}

/* ------------------------------------------------------------------------------------------- *
 * Shared Legendary entrance themes (src/audio/music-cards.json names which cards play which).
 * One shape for both, brass-forward next to the Mythics' prismatic themes: a two-bar fanfare
 * opening on the motif, then a twelve-bar body (twelve, not eight, so the loop still runs past
 * fifteen seconds at this tempo) quoting the motif over brass, timpani and a high glint run.
 * ------------------------------------------------------------------------------------------- */

/** Shared Legendary theme 1: D minor, trumpet over horn, trombone and timpani, celesta glint. */
function legendaryTheme1() {
  const song = createSong({ id: "legendary-1", bpm: 132, key: key("D", "minor") });
  song.reverb = { room: 0.8, damp: 0.3, width: 1, level: 0.62 };
  instrument(song, 0, GM.trumpet, { volume: 90, pan: 60, reverb: 60 });
  instrument(song, 1, GM.horn, { volume: 86, pan: 48, reverb: 65 });
  instrument(song, 2, GM.celesta, { volume: 72, pan: 80, reverb: 70 });
  instrument(song, 3, GM.timpani, { volume: 86, reverb: 60 });
  instrument(song, 4, GM.trombone, { volume: 80, pan: 70, reverb: 60 });
  instrument(song, 5, GM.contrabass, { volume: 84, reverb: 50 });
  instrument(song, DRUMS, KIT.orchestra, { volume: 76, reverb: 60 });
  const intro = section(song, "intro", 2, ["i", "V+"]);
  line(intro, { ch: 0, notes: FANFARE, vel: 86 });
  comp(intro, { ch: 1, hits: [[0, 0.9, 1], [1.5, 0.4, 0.7]], center: 57, vel: 58 });
  roots(intro, { ch: 3, octave: -2, vel: 72, hits: [[0, 1, 1], [1.5, 0.5, 0.7]] });
  const prog = ["i", "VI", "III", "VII", "i", "iv", "V+", "i", "VI", "VII", "iv", "V+"];
  const body = section(song, "body", 12, prog);
  melody(body, { ch: 0, rhythms: RHYTHMS.epic, range: [2, 9], vel: 82, motifBars: [0, 4, 8], start: 4, seed: 1 });
  comp(body, { ch: 1, hits: [[0, 0.9, 1], [1.5, 0.4, 0.7], [2, 1.9, 0.85]], center: 57, vel: 56 });
  arp(body, { ch: 2, step: 0.25, pattern: [0, 1, 2, 3, 4, 5, 6, 7, 6, 5, 4, 3, 2, 1, 2, 3], center: 72, vel: 48 });
  roots(body, { ch: 3, octave: -2, vel: 66, hits: [[0, 1, 1], [3.5, 0.5, 0.6]] });
  bass(body, { ch: 4, pattern: [[0, "R", 0.9, 1], [2, "5", 0.9, 0.8]], octave: -1, vel: 62 });
  bass(body, { ch: 5, pattern: [[0, "R", 1.9, 1], [2, "R", 1.9, 0.8]], vel: 70 });
  return { song };
}

/** Shared Legendary theme 2: G minor, horn-led answer to theme 1 over the same forces. */
function legendaryTheme2() {
  const song = createSong({ id: "legendary-2", bpm: 132, key: key("G", "minor") });
  song.reverb = { room: 0.8, damp: 0.3, width: 1, level: 0.62 };
  instrument(song, 0, GM.trumpet, { volume: 88, pan: 68, reverb: 60 });
  instrument(song, 1, GM.horn, { volume: 90, pan: 52, reverb: 65 });
  instrument(song, 2, GM.piccolo, { volume: 70, pan: 80, reverb: 65 });
  instrument(song, 3, GM.timpani, { volume: 86, reverb: 60 });
  instrument(song, 4, GM.trombone, { volume: 82, pan: 44, reverb: 60 });
  instrument(song, 5, GM.contrabass, { volume: 84, reverb: 50 });
  instrument(song, DRUMS, KIT.orchestra, { volume: 76, reverb: 60 });
  const intro = section(song, "intro", 2, ["i", "V+"]);
  line(intro, { ch: 1, notes: FANFARE, vel: 84 });
  roots(intro, { ch: 3, octave: -2, vel: 70, hits: [0, 0.5, 1, 1.5, 2, 2.5, 3, 3.5].map((t) => [t, 0.5, 0.6]) });
  const prog = ["i", "iv", "VII", "III", "VI", "iv", "V+", "i", "iv", "VI", "VII", "V+"];
  const body = section(song, "body", 12, prog);
  melody(body, { ch: 1, rhythms: RHYTHMS.epic, range: [0, 7], vel: 82, motifBars: [0, 4, 8], start: 7, seed: 2 });
  comp(body, { ch: 0, hits: [[0, 0.9, 1], [2, 1.9, 0.85]], center: 60, vel: 52 });
  arp(body, { ch: 2, step: 0.25, pattern: [7, 6, 5, 4, 3, 2, 1, 0, 1, 2, 3, 4, 5, 6, 5, 4], center: 74, vel: 46 });
  roots(body, { ch: 3, octave: -2, vel: 66, hits: [[0, 0.5, 1], [2, 0.5, 0.8], [3.5, 0.5, 0.6]] });
  bass(body, { ch: 4, pattern: [[0, "R", 0.9, 1], [1, "5", 0.9, 0.8], [2, "8", 0.9, 0.85], [3, "5", 0.9, 0.8]], octave: -1, vel: 64 });
  bass(body, { ch: 5, pattern: [[0, "R", 1.9, 1], [2, "R", 1.9, 0.8]], vel: 70 });
  return { song };
}

/* ------------------------------------------------------------------------------------------- *
 * The track list
 * ------------------------------------------------------------------------------------------- */

const TAVERN_A = ["I", "V", "vi", "IV", "I", "IV", ["ii", "V"], "I"];
const TAVERN_B = ["IV", "V", "iii", "vi", "IV", "I", "ii7", ["Vsus4", "V"]];
const MIXO_A = ["I", "vii", "IV", "I", "I", "vii", "IV", ["v", "I"]];
const MIXO_B = ["IV", "I", "vii", "IV", "ii", "IV", "vii", ["IV", "I"]];
const TAVERN_DANGER_A = ["i", "VI", "VII", "i", "i", "iv", "V+", "i"];
const TAVERN_DANGER_B = ["VI", "VII", "i", "i", "iv", "VI", ["iv", "V+"], "i"];

const EDM_A = ["i", "VI", "III", "VII", "i", "VI", "III", "VII"];
const EDM_B = ["VI", "VII", "i", "i", "VI", "VII", "III", ["VII", "V+"]];
const EDM2_A = ["i", "iv", "VI", "V+", "i", "iv", "VII", "III"];
const EDM2_B = ["VI", "III", "VII", "i", "VI", "III", "iv", "V+"];
const EDM_DANGER_A = ["i", "i", "VI", "VII", "i", "i", "iv", "V+"];
const EDM_DANGER_B = ["VI", "iv", "i", "V+", "VI", "iv", "VII", "V+"];

const LOFI_A = ["ii7", "V7", "I7", "vi7", "ii7", "V7", "iii7", "vi7"];
const LOFI_B = ["IV7", "iii7", "ii7", "I7", "IV7", "iii7", "ii7", "V7sus4"];
const LOFI2_A = ["I7", "vi7", "ii7", "V7", "I7", "iii7", "IV7", "V7"];
const LOFI2_B = ["vi7", "iii7", "IV7", "I7", "ii7", "V7", "iii7", "vi7"];
const LOFI_DANGER_A = ["i7", "iv7", "VII7", "III7", "VI7", "ii7", "V+7", "i7"];
const LOFI_DANGER_B = ["VI7", "VII7", "i7", "i7", "iv7", "VII7", "V+7", "i7"];

const EPIC_A = ["i", "VI", "III", "VII", "i", "VI", "iv", "V+"];
const EPIC_B = ["VI", "VII", "i", "i", "iv", "VI", "VII", "V+"];
const EPIC2_A = ["i", "iv", "VII", "III", "VI", "iv", "V+", "i"];
const EPIC2_B = ["VI", "III", "VII", "i", "iv", "i", "VI", "V+"];
const EPIC_DANGER_A = ["i", "i", "VI", "V+", "i", "iv", "VI", "V+"];
const EPIC_DANGER_B = ["iv", "VI", "VII", "i", "iv", "VI", ["VII", "V+"], "i"];

/** Every track, in the manifest's order. `loop` false: a sting that plays once. */
export const TRACKS = [
  { id: "menu", loop: true, build: menu },
  { id: "tavern-1", loop: true, build: () => tavern({ id: "tavern-1", tonic: "D", mode: "major", bpm: 110, A: TAVERN_A, B: TAVERN_B, lead: GM.flute, second: GM.fiddle }) },
  { id: "tavern-2", loop: true, build: () => tavern({ id: "tavern-2", tonic: "G", mode: "mixolydian", bpm: 116, A: MIXO_A, B: MIXO_B, lead: GM.fiddle, second: GM.recorder }) },
  { id: "tavern-danger", loop: true, build: () => tavern({ id: "tavern-danger", tonic: "D", mode: "minor", bpm: 112, A: TAVERN_DANGER_A, B: TAVERN_DANGER_B, lead: GM.fiddle, second: GM.oboe, danger: true }) },
  { id: "tavern-start", loop: false, build: () => matchStart("tavern") },
  { id: "edm-1", loop: true, build: () => edm({ id: "edm-1", tonic: "A", mode: "minor", bpm: 122, A: EDM_A, B: EDM_B }) },
  { id: "edm-2", loop: true, build: () => edm({ id: "edm-2", tonic: "D", mode: "minor", bpm: 120, A: EDM2_A, B: EDM2_B }) },
  { id: "edm-danger", loop: true, build: () => edm({ id: "edm-danger", tonic: "A", mode: "minor", bpm: 124, A: EDM_DANGER_A, B: EDM_DANGER_B, danger: true }) },
  { id: "edm-start", loop: false, build: () => matchStart("edm") },
  { id: "lofi-1", loop: true, build: () => lofi({ id: "lofi-1", tonic: "F", mode: "major", bpm: 94, A: LOFI_A, B: LOFI_B }) },
  { id: "lofi-2", loop: true, build: () => lofi({ id: "lofi-2", tonic: "Bb", mode: "major", bpm: 98, A: LOFI2_A, B: LOFI2_B }) },
  { id: "lofi-danger", loop: true, build: () => lofi({ id: "lofi-danger", tonic: "F", mode: "minor", bpm: 88, A: LOFI_DANGER_A, B: LOFI_DANGER_B, danger: true }) },
  { id: "lofi-start", loop: false, build: () => matchStart("lofi") },
  { id: "epic-1", loop: true, build: () => epic({ id: "epic-1", tonic: "D", mode: "minor", bpm: 100, A: EPIC_A, B: EPIC_B }) },
  { id: "epic-2", loop: true, build: () => epic({ id: "epic-2", tonic: "G", mode: "minor", bpm: 104, A: EPIC2_A, B: EPIC2_B }) },
  { id: "epic-danger", loop: true, build: () => epic({ id: "epic-danger", tonic: "D", mode: "minor", bpm: 96, A: EPIC_DANGER_A, B: EPIC_DANGER_B, danger: true }) },
  { id: "epic-start", loop: false, build: () => matchStart("epic") },
  { id: "victory", loop: true, build: victory },
  { id: "defeat", loop: true, build: defeat },
  { id: "draw", loop: true, build: draw },
  { id: "mythic-my-pawn", loop: true, build: myPawn },
  { id: "mythic-zephyrs", loop: true, build: zephyrs },
  { id: "mythic-heroic-power", loop: true, build: heroicPower },
  { id: "mythic-craft-a-card", loop: true, build: craftACard },
  { id: "mythic-ceaseless-void", loop: true, build: ceaselessVoid },
  { id: "mythic-in-too-deep", loop: true, build: inTooDeep },
  { id: "mythic-zephrys-zealotism", loop: true, build: zephrysZealotism },
  { id: "mythic-portal-to-the-past", loop: true, build: portalToThePast },
  { id: "mythic-twice-forward", loop: true, build: twiceForward },
  { id: "legendary-1", loop: true, build: legendaryTheme1 },
  { id: "legendary-2", loop: true, build: legendaryTheme2 },
];
