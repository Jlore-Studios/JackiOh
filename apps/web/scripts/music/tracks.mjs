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
// Every Legendary and Mythic card, tokens printed so included, has an intro of its own (R1352, at the
// end of the file): a few bars on the motif, derived from the card and hand-tuned where it matters.

import {
  DR, DRUMS, GM, KIT, MOTIF_CELL, RHYTHMS,
  alterOf, arp, bass, bend, cc, chordAt, chordPitches, comp, createSong, drums, instrument, line, melody, note, pad, pulse, pump, roots, section,
} from "./compose.mjs";
import { TONIC, key, pitch, rng, seedOf, voicing } from "./theory.mjs";

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

/** Classic+ #29 Portal to the Past: a music box playing the last game's tune, viola and strings answering. */
function portalToThePast() {
  const song = createSong({ id: "mythic-portal-to-the-past", bpm: 80, key: key("Bb", "major") });
  song.reverb = { room: 0.85, damp: 0.35, width: 1, level: 0.65 };
  instrument(song, 0, GM.musicBox, { volume: 92, pan: 70, reverb: 75 });
  instrument(song, 1, GM.viola, { volume: 80, pan: 46, reverb: 70 });
  instrument(song, 2, GM.strings, { volume: 62, reverb: 75 });
  instrument(song, 3, GM.cello, { volume: 84, reverb: 55 });
  instrument(song, 4, GM.glock, { volume: 56, pan: 88, reverb: 80 });
  const intro = section(song, "intro", 1, ["I"]);
  arp(intro, { ch: 0, step: 0.25, pattern: [7, 6, 5, 4, 3, 2, 1, 0, 1, 2, 3, 4, 5, 6, 7, 8], center: 70, vel: 54 });
  const body = section(song, "body", 8, ["I", "vi", "IV", "V", "I", "vi", ["ii", "V"], "I"]);
  melody(body, { ch: 0, rhythms: RHYTHMS.lofi, range: [7, 14], vel: 72, motifBars: [0, 4], start: 10, seed: 1 });
  melody(body, { ch: 1, rhythms: RHYTHMS.folk, range: [0, 7], vel: 56, motifBars: [], start: 2, seed: 2 });
  pad(body, { ch: 2, center: 62, vel: 36 });
  bass(body, { ch: 3, pattern: [[0, "R", 1.9, 1], [2, "5", 1.9, 0.8]], octave: -1, vel: 64 });
  arp(body, { ch: 4, step: 1, pattern: [4, 2, 0, 2], center: 79, vel: 34 });
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
 * Card intros (R1352; docs/meditative-set.md M8, MN04)
 *
 * Every Legendary and Mythic card, and every token printed Legendary or Mythic, opens its play with a
 * few bars of its own, after Hearthstone's legendary music: src/audio/music-cards.json's `intro`
 * names each card's track, and the client plays it once over the board's music (R1350, R1351).
 *
 * An intro is derived from its card, so that no two sound alike:
 *   - its id seeds every choice the card does not make for itself: the key, the tempo, the meter,
 *     the motif's variant and the answer it gets, the progression, and a family's alternates;
 *   - its family picks the instruments, the accompaniment and the modes it may take: its station
 *     where music-cards.json gives one, else the first of its tags the card art reads (its tribe),
 *     then the tags the art has no family for (Jlockeed, Acclaimed), else its type;
 *   - its set lends a glint of its own into the cadence (Core a harp, Classic a dulcimer, Classic+ a
 *     celesta), and a Mythic adds a prismatic shimmer over a soft choir.
 * The melody is the JackiOh motif (MOTIF_CELL's contour, varied) and an answer that cadences on the
 * tonic: two bars of 4/4 or three of 3/4, whose music runs INTRO_SPAN_S and then rings out for
 * INTRO_TAIL_S, the last INTRO_FADE_S of it faded (gen-music.mjs's `tail` and `post.fadeOutS`).
 *
 * The Mythics are hand-tuned from their themes (each theme's key, tempo and forces, and the motif
 * as the theme quotes it), and so are the best-known Legendaries: `tune` holds what a card sets for
 * itself, and the derivation fills in the rest. A card's facts are written down here as the catalog
 * printed them when its intro was composed, so a later change to a card's tags moves no rendered
 * file without a new render.
 * ------------------------------------------------------------------------------------------- */

/** R1352: seconds of ring-out kept after an intro's last bar line, the last INTRO_FADE_S of them faded. */
const INTRO_TAIL_S = 1;
const INTRO_FADE_S = 0.6;
/**
 * R1352: how long an intro's music runs to its last bar line, in seconds (MUSIC_INTRO_MIN_S and,
 * with the tail, MUSIC_INTRO_MAX_S in src/audio/constants.ts hold the files to it).
 */
const INTRO_SPAN_S = [3, 5];
/** Where in INTRO_SPAN_S a derived intro's tempo lands, by its family's feel. */
const INTRO_FEELS = { slow: [4.2, 4.95], mid: [3.7, 4.6], fast: [3.2, 4.1] };
const INTRO_TONICS = Object.keys(TONIC);
const INTRO_SETS = ["Core", "Classic", "Classic+", "Meditative"];
/** The set's glint into the cadence (a set not listed here takes Classic+'s). */
const INTRO_SET_GLINT = { Core: GM.harp, Classic: GM.dulcimer, "Classic+": GM.celesta };

/** The first tag that names a family wins, in the card art's order, then the tags it draws none for. */
const INTRO_TAG_FAMILIES = [
  ["Call to Chaos", "chaos"], ["AI", "ai"], ["CN", "cn"], ["KY", "ky"], ["Book", "book"], ["Felinor", "felinor"],
  ["Pancake", "pancake"], ["Fruit", "fruit"], ["Quickdraw", "quickdraw"], ["Human", "human"],
  ["Jlockeed", "jlockeed"], ["Acclaimed", "acclaimed"],
];
const INTRO_TYPE_FAMILIES = { Unit: "unit", Spell: "spell", "Field Spell": "field-spell", Trap: "trap", "Field Trap": "field-trap" };

/**
 * Each family's forces: a lead (channel 0, the motif), a second (1, the harmony), a bass (2) and a
 * colour (3), the accompaniment `style`, the modes it may take (the set picks among them first),
 * its `feel` (how fast), the MIDI note its melody centres on, and alternates the id picks among.
 */
const INTRO_PALETTES = {
  human: { lead: GM.horn, second: GM.strings, bass: GM.contrabass, color: GM.harp, style: "fanfare", modes: ["major", "mixolydian", "dorian"], feel: "mid", center: 64 },
  felinor: { lead: GM.pizz, second: GM.strings, bass: GM.pizz, color: GM.glock, style: "pizz", modes: ["dorian", "major", "harmonic"], feel: "fast", center: 69, alts: { color: [GM.glock, GM.clarinet] } },
  ky: { lead: GM.bells, second: GM.celesta, bass: GM.fretless, color: GM.vibes, style: "chime", modes: ["lydian", "major"], feel: "slow", center: 72, alts: { lead: [GM.bells, GM.vibes] } },
  cn: { lead: GM.dulcimer, second: GM.strings, bass: GM.acBass, color: GM.kalimba, style: "sparkle", modes: ["major", "dorian"], feel: "mid", center: 72 },
  book: { lead: GM.harpsichord, second: GM.pizz, bass: GM.bassoon, color: GM.celesta, style: "baroque", modes: ["harmonic", "dorian", "major"], feel: "mid", center: 69 },
  chaos: { lead: GM.calliope, second: GM.xylophone, bass: GM.tuba, color: GM.glock, style: "carnival", modes: ["harmonic", "mixolydian"], feel: "fast", center: 74 },
  ai: { lead: GM.square, second: GM.polysynth, bass: GM.synthBass2, color: GM.crystal, style: "synth", modes: ["minor", "dorian"], feel: "fast", center: 67 },
  pancake: {
    lead: GM.trumpet, second: GM.brass, bass: GM.tuba, color: GM.glock, style: "brass", modes: ["major", "lydian", "mixolydian"], feel: "mid", center: 70,
    alts: { second: [GM.brass, GM.horn, GM.trombone], color: [GM.glock, GM.xylophone, GM.bells] },
  },
  fruit: { lead: GM.marimba, second: GM.kalimba, bass: GM.acBass, color: GM.steel, style: "bounce", modes: ["major", "mixolydian"], feel: "fast", center: 69 },
  quickdraw: { lead: GM.whistle, second: GM.steel, bass: GM.acBass, color: GM.banjo, style: "gallop", modes: ["mixolydian", "dorian"], feel: "fast", center: 76 },
  jlockeed: { lead: GM.synthBrass, second: GM.strings, bass: GM.contrabass, color: GM.bells, style: "march", modes: ["minor", "dorian"], feel: "mid", center: 65 },
  acclaimed: { lead: GM.trumpet, second: GM.brass, bass: GM.contrabass, color: GM.orchHit, style: "boom", modes: ["minor", "harmonic"], feel: "mid", center: 67 },
  tavern: { lead: GM.fiddle, second: GM.nylon, bass: GM.acBass, color: GM.harp, style: "bounce", modes: ["major", "mixolydian"], feel: "fast", center: 72 },
  edm: { lead: GM.saw, second: GM.warmPad, bass: GM.synthBass2, color: GM.square, style: "synth", modes: ["minor"], feel: "fast", center: 67 },
  lofi: { lead: GM.vibes, second: GM.ep1, bass: GM.fingerBass, color: GM.cleanGuitar, style: "chime", modes: ["major", "dorian"], feel: "slow", center: 69 },
  epic: { lead: GM.horn, second: GM.strings, bass: GM.contrabass, color: GM.choir, style: "orchestral", modes: ["minor", "dorian"], feel: "mid", center: 62 },
  unit: { lead: GM.trumpet, second: GM.horn, bass: GM.contrabass, color: GM.harp, style: "fanfare", modes: ["major", "dorian", "minor"], feel: "mid", center: 68, alts: { lead: [GM.trumpet, GM.horn], second: [GM.horn, GM.strings] } },
  spell: { lead: GM.celesta, second: GM.strings, bass: GM.cello, color: GM.harp, style: "sparkle", modes: ["lydian", "major", "dorian"], feel: "mid", center: 76, alts: { lead: [GM.celesta, GM.glock, GM.flute] } },
  "field-spell": { lead: GM.flute, second: GM.choir, bass: GM.cello, color: GM.harp, style: "swell", modes: ["major", "lydian", "mixolydian"], feel: "slow", center: 74, alts: { lead: [GM.flute, GM.oboe, GM.clarinet] } },
  trap: { lead: GM.oboe, second: GM.tremolo, bass: GM.contrabass, color: GM.celesta, style: "dark", modes: ["harmonic", "phrygian", "minor"], feel: "slow", center: 67 },
  "field-trap": { lead: GM.englishHorn, second: GM.organ, bass: GM.contrabass, color: GM.celesta, style: "dark", modes: ["phrygian", "harmonic"], feel: "slow", center: 64 },
};

/** The motif's contour as MOTIF_CELL steps it (rise 0–1–3, fall back by step), and its variants. */
const INTRO_CONTOURS = [
  MOTIF_CELL.map(([step]) => step),
  [0, -1, -3, -2, -1, 0],
  [0, 2, 4, 3, 2, 0],
  [0, 1, 4, 3, 1, 0],
  [0, 1, 3, 4, 3, 2],
];
/** Rhythms for the motif's six notes: a bar of 4/4, or a bar of 3/4. */
const INTRO_CELLS = {
  4: [MOTIF_CELL.map(([, beats]) => beats), [0.75, 0.25, 1.5, 0.5, 0.5, 0.5], [0.5, 0.5, 1, 0.5, 0.5, 1], [0.25, 0.25, 2, 0.5, 0.5, 0.5]],
  3: [[0.5, 0.5, 1, 0.5, 0.25, 0.25], [0.25, 0.25, 1.5, 0.5, 0.25, 0.25], [0.5, 0.25, 0.25, 1, 0.5, 0.5]],
};
/** The answer, a closing bar that lands on the tonic (or its octave), in degrees of the key. */
const INTRO_ANSWERS = {
  4: [
    [[4, 0.5], [3, 0.5], [2, 0.5], [1, 0.5], [0, 2]],
    [[2, 1], [1, 0.5], [-1, 0.5], [0, 2]],
    [[5, 0.5], [4, 0.5], [1, 1], [0, 2]],
    [[4, 1], [6, 0.5], [7, 2.5]],
    [[3, 0.5], [2, 0.5], [1, 0.5], [-1, 0.5], [0, 2]],
    [[6, 0.5], [4, 0.5], [6, 1], [7, 2]],
  ],
  3: [
    [[1, 0.5], [-1, 0.5], [0, 2]],
    [[2, 1], [1, 0.5], [0, 1.5]],
    [[4, 0.5], [6, 0.5], [7, 2]],
  ],
};
/** Cadencing progressions, one entry per bar, by mode and meter. Harmonic minor's V is major already. */
const INTRO_PROGRESSIONS = {
  major: { 4: ["I", ["V", "I"]], alt4: [[["I", "IV"], ["V", "I"]], [["I", "vi"], ["IV", "I"]], [["vi", "IV"], ["V", "I"]]], 3: [["I", "IV", ["V", "I"]], ["I", "vi", ["V", "I"]], ["I", "V", "I"]] },
  minor: { 4: ["i", ["V+", "i"]], alt4: [[["i", "VI"], ["V+", "i"]], [["i", "iv"], ["V+", "i"]], [["VI", "iv"], ["V+", "i"]]], 3: [["i", "iv", ["V+", "i"]], ["i", "VI", ["V+", "i"]], ["i", "VII", "i"]] },
  harmonic: { 4: ["i", ["V", "i"]], alt4: [[["i", "iv"], ["V", "i"]], [["i", "VI"], ["V", "i"]], [["iv", "VI"], ["V", "i"]]], 3: [["i", "iv", ["V", "i"]], ["i", "V", "i"], ["i", "VI", ["V", "i"]]] },
  dorian: { 4: ["i", ["IV", "i"]], alt4: [[["i", "VII"], ["IV", "i"]], [["i", "III"], ["IV", "i"]]], 3: [["i", "IV", "i"], ["i", "VII", ["IV", "i"]]] },
  mixolydian: { 4: ["I", ["VII", "I"]], alt4: [[["I", "IV"], ["VII", "I"]], [["I", "v"], ["IV", "I"]]], 3: [["I", "VII", "I"], ["I", "IV", ["VII", "I"]]] },
  lydian: { 4: ["I", ["II", "I"]], alt4: [[["I", "V"], ["II", "I"]], [["I", "vi"], ["II", "I"]]], 3: [["I", "II", "I"], ["I", "vi", ["II", "I"]]] },
  phrygian: { 4: ["i", ["II", "i"]], alt4: [[["i", "vii"], ["II", "i"]], [["i", "iv"], ["II", "i"]]], 3: [["i", "II", "i"], ["i", "iv", ["II", "i"]]] },
};
/** The styles that may take a waltz's three bars of 3/4, and how often a derived intro does. */
const INTRO_WALTZ_STYLES = new Set(["pizz", "carnival", "sparkle", "swell", "baroque", "bounce", "chime"]);
const INTRO_WALTZ_CHANCE = 0.3;

const introTrack = (card) => `intro-${card}`;

function introFamily(card) {
  if (card.station !== undefined) return card.station;
  for (const [tag, family] of INTRO_TAG_FAMILIES) if (card.tags.includes(tag)) return family;
  return INTRO_TYPE_FAMILIES[card.type];
}

/** The motif's bar and its answer (two bars of 4/4), or the motif, its sequence a step up and a close (3/4). */
function introMelody(r, bpb) {
  const anchor = r.pick([4, 4, 0, 2]);
  const contour = r.pick(INTRO_CONTOURS);
  const cell = r.pick(INTRO_CELLS[bpb]);
  const motif = contour.map((step, i) => [anchor + step, cell[i]]);
  const answer = r.pick(INTRO_ANSWERS[bpb]);
  if (bpb === 4) return [...motif, ...answer];
  const lift = r.pick([1, 2, -1]);
  const again = r.pick(INTRO_CELLS[3]);
  return [...motif, ...contour.map((step, i) => [anchor + lift + step, again[i]]), ...answer];
}

/** The beats of the voices in a melody, which must fill its bars exactly. */
const beatsOf = (notes) => notes.reduce((sum, [, beats]) => sum + beats, 0);

/** Everything an intro is built from: the derivation, with the card's own `tune` laid over it. */
function introSpec(card) {
  const r = rng(seedOf(`intro:${card.card}`));
  const tune = card.tune ?? {};
  const family = introFamily(card);
  const base = INTRO_PALETTES[family];
  const palette = { ...base };
  for (const [part, options] of Object.entries(base.alts ?? {})) palette[part] = r.pick(options);
  Object.assign(palette, tune.palette ?? {});
  const tonic = tune.tonic ?? r.pick(INTRO_TONICS);
  const setAt = Math.max(0, INTRO_SETS.indexOf(card.set));
  const preferred = palette.modes[setAt % palette.modes.length];
  const mode = tune.mode ?? (r.chance(0.65) ? preferred : r.pick(palette.modes));
  // A card that sets its own tempo sets its meter with it (4/4 unless it says otherwise).
  const waltz = INTRO_WALTZ_STYLES.has(palette.style) && r.chance(INTRO_WALTZ_CHANCE) && tune.bpm === undefined;
  const bpb = tune.beatsPerBar ?? (waltz ? 3 : 4);
  const bars = tune.bars ?? (bpb === 3 ? 3 : 2);
  const [lo, hi] = INTRO_FEELS[palette.feel];
  const span = lo + r.next() * (hi - lo);
  const bpm = tune.bpm ?? Math.round((bars * bpb * 60) / span);
  const derived = bpb === 4 || bpb === 3 ? introMelody(r, bpb) : null;
  const melodyNotes = tune.melody ?? derived;
  const progs = INTRO_PROGRESSIONS[mode];
  const prog = tune.prog ?? (bpb === 3 ? r.pick(progs[3]) : r.chance(0.4) ? progs[4] : r.pick(progs.alt4));
  const spec = {
    card, family, palette, tonic, mode, bpm, bpb, bars, prog, melody: melodyNotes,
    mythic: card.rarity === "Mythic", glint: INTRO_SET_GLINT[card.set] ?? INTRO_SET_GLINT["Classic+"], extra: tune.extra ?? null,
  };
  const seconds = (bars * bpb * 60) / bpm;
  if (melodyNotes === null || Math.abs(beatsOf(melodyNotes) - bars * bpb) > 1e-9) throw new Error(`intro ${card.card}: its melody does not fill its ${bars} bars`);
  if (prog.length !== bars) throw new Error(`intro ${card.card}: its progression has ${prog.length} bars, not ${bars}`);
  if (seconds < INTRO_SPAN_S[0] - 1e-9 || seconds > INTRO_SPAN_S[1] + 1e-9) throw new Error(`intro ${card.card}: ${seconds.toFixed(2)} s is outside ${INTRO_SPAN_S.join("–")} s`);
  return spec;
}

/* ----- the intros' gestures ----- */

/** Where the last bar starts, in beats from the intro's top. */
const lastBarOf = (sec) => (sec.bars - 1) * sec.song.beatsPerBar;

/** The cadence's chords held through the last bar. */
function holdCadence(sec, ch, center, vel) {
  comp(sec, { ch, hits: [[0, sec.song.beatsPerBar, 1]], center, vel, from: lastBarOf(sec) });
}

/** A quick run up (or down) the chord sounding at `at`: a harp's glissando, a celesta's glint. */
function glintRun(sec, ch, at, { count = 6, step = 0.125, center = 72, vel = 52, down = false } = {}) {
  const voiced = voicing(chordPitches(sec.k, chordAt(sec, at)), center);
  const ext = [...voiced, ...voiced.map((n) => n + 12), ...voiced.map((n) => n + 24)];
  const n = Math.min(count, ext.length);
  for (let i = 0; i < n; i += 1) {
    const at2 = at + i * step;
    if (at2 >= sec.beats - 1e-9) break;
    note(sec, ch, at2, step * 2, ext[down ? n - 1 - i : i], vel * (0.75 + (0.25 * i) / n));
  }
}

/** A timpani roll on the coming chord's root, from `at` for `beats`, into a stroke where it lands. */
function timpaniRoll(sec, ch, at, beats, vel = 70) {
  const land = Math.min(at + beats, sec.beats - 0.01);
  const c = chordAt(sec, land);
  const n = pitch(sec.k, c.root, -2, alterOf(c, c.root));
  for (let t = 0; t < beats - 1e-9; t += 0.125) note(sec, ch, at + t, 0.125, n, vel * (0.35 + (0.55 * t) / beats), { exact: true });
  if (at + beats < sec.beats - 1e-9) note(sec, ch, at + beats, 1.5, n, vel, { exact: true });
}

/** A drum part of one bar's pattern, bounded to the meter (a 4/4 pattern in a 3/4 bar loses its fourth beat). */
function introDrums(sec, pattern, opts = {}) {
  drums(sec, { pattern: pattern.filter(([t]) => t < sec.song.beatsPerBar - 1e-9), ...opts });
}

/** A drum part's `skip` that keeps only bar `n` (`drums` skips every bar it returns true for). */
const onlyBar = (n) => (bar) => bar !== n;
const onLastBar = (sec) => onlyBar(sec.bars - 1);
/** A drum part's `skip` that keeps every bar but the last, which the cadence holds. */
const beforeLastBar = (sec) => (bar) => bar === sec.bars - 1;

/**
 * The accompaniment under the lead, one per style: channel 1 the second, 2 the bass, 3 the colour,
 * 4 timpani, and the drum kit each style names (`kit`). Every style ends on the cadence held.
 */
const INTRO_STYLES = {
  /** Brass stabs, timpani, a harp's sweep up into the first chord: a herald. */
  fanfare: {
    kit: KIT.orchestra,
    build(sec, s) {
      const last = lastBarOf(sec);
      comp(sec, { ch: 1, hits: [[0, 0.45, 1], [1.5, 0.4, 0.75], [2, 1.9, 0.9]], center: 58, vel: 60, until: last });
      holdCadence(sec, 1, 58, 62);
      bass(sec, { ch: 2, pattern: [[0, "R", 1.9, 1], [2, "5", 1.9, 0.85]], vel: 72 });
      timpaniRoll(sec, 4, 0, 0.75, 66);
      timpaniRoll(sec, 4, last - 0.5, 0.5, 74);
      glintRun(sec, 3, 0, { count: 7, center: s.palette.center - 10, vel: 48 });
      introDrums(sec, [[0, DR.crash, 62]], { skip: onLastBar(sec) });
    },
  },
  /** Pizzicato: plucked chords on the off-beats over a walking bass, a triangle at either end. */
  pizz: {
    kit: KIT.standard,
    build(sec, s) {
      const bpb = sec.song.beatsPerBar;
      const last = lastBarOf(sec);
      comp(sec, { ch: 1, hits: bpb === 3 ? [[1, 0.3, 0.8], [2, 0.3, 0.7]] : [[1, 0.3, 0.8], [3, 0.3, 0.7]], center: 62, vel: 50, until: last });
      holdCadence(sec, 1, 62, 46);
      bass(sec, { ch: 2, pattern: [[0, "R", 0.45, 1], [1, "5", 0.45, 0.8], [2, "8", 0.45, 0.85], [3, "5", 0.45, 0.8]], vel: 70 });
      glintRun(sec, 3, last, { count: 5, step: 0.25, center: s.palette.center + 4, vel: 44 });
      introDrums(sec, [[0, DR.triOpen, 46]], { skip: (bar) => bar !== 0 && bar !== sec.bars - 1 });
      introDrums(sec, [[0.5, DR.claves, 26], [1.5, DR.claves, 22], [2.5, DR.claves, 26], [3.5, DR.claves, 22]], { skip: onlyBar(0) });
    },
  },
  /** Chimes over a warm hold: slow broken chords on the second, the colour holding the cadence. */
  chime: {
    kit: null,
    build(sec, s) {
      arp(sec, { ch: 1, step: 1, pattern: [0, 2, 1, 3], center: s.palette.center, vel: 44, gate: 1.6 });
      bass(sec, { ch: 2, pattern: [[0, "R", 2.9, 1]], vel: 60 });
      pad(sec, { ch: 7, center: 60, vel: 32 });
      holdCadence(sec, 3, s.palette.center - 5, 40);
    },
  },
  /** A baroque hall: sixteenth broken chords, a walking bass of eighths, a soft string hold. */
  baroque: {
    kit: null,
    build(sec, s) {
      const last = lastBarOf(sec);
      arp(sec, { ch: 1, step: 0.25, pattern: [0, 2, 1, 2, 0, 2, 1, 2, 3, 2, 1, 2, 0, 2, 1, 2], center: 62, vel: 46, until: last });
      holdCadence(sec, 1, 62, 46);
      bass(sec, { ch: 2, pattern: [[0, "R", 0.45, 1], [0.5, "5", 0.45, 0.75], [1, "8", 0.45, 0.85], [1.5, "5", 0.45, 0.75]], octave: -1, vel: 62 });
      pad(sec, { ch: 3, center: s.palette.center - 7, vel: 30 });
    },
  },
  /** The fairground: oom-pah under the lead, a snare roll into the last bar and a cymbal on it. */
  carnival: {
    kit: KIT.standard,
    build(sec, s) {
      const bpb = sec.song.beatsPerBar;
      const last = lastBarOf(sec);
      bass(sec, { ch: 2, pattern: [[0, "R", 0.45, 1], [2, "5", 0.45, 0.9]], vel: 76 });
      comp(sec, { ch: 1, hits: bpb === 3 ? [[1, 0.25, 0.9], [2, 0.25, 0.8]] : [[1, 0.25, 0.9], [3, 0.25, 0.8]], center: 64, vel: 52, until: last });
      holdCadence(sec, 1, 64, 50);
      glintRun(sec, 3, last - 0.75, { count: 6, center: s.palette.center, vel: 46 });
      for (let t = last - 1; t < last - 1e-9; t += 0.125) note(sec, DRUMS, t, 0.1, DR.snare, 30 + 40 * (t - last + 1), { exact: true });
      introDrums(sec, [[0, DR.crash, 66], [0, DR.kick, 70]], { skip: onLastBar(sec) });
    },
  },
  /** Synths: a pad, a sixteenth arpeggio, a pulsing bass, a kick on every beat. */
  synth: {
    kit: KIT.electronic,
    build(sec, s) {
      const last = lastBarOf(sec);
      pad(sec, { ch: 1, center: 60, vel: 44 });
      arp(sec, { ch: 3, step: 0.25, pattern: [0, 1, 2, 3, 2, 1, 2, 3], center: s.palette.center, vel: 38, gate: 0.5, until: last });
      pulse(sec, { ch: 2, step: 0.5, octave: -2, vel: 68, accent: [1, 0.7] });
      introDrums(sec, [[0, DR.kick, 84], [1, DR.kick, 78], [1, DR.clap, 56], [2, DR.kick, 84], [3, DR.kick, 78], [3, DR.clap, 56], [0.5, DR.hat, 40], [1.5, DR.hat, 36], [2.5, DR.hat, 40], [3.5, DR.hat, 36]], { skip: beforeLastBar(sec) });
      introDrums(sec, [[0, DR.kick, 88], [0, DR.open, 50]], { skip: onLastBar(sec) });
    },
  },
  /** Bright brass: punchy section stabs, a tuba, the colour doubling the lead an octave up. */
  brass: {
    kit: KIT.orchestra,
    build(sec, s) {
      const last = lastBarOf(sec);
      comp(sec, { ch: 1, hits: [[0, 0.4, 1], [1, 0.25, 0.7], [1.5, 0.4, 0.85]], center: 62, vel: 60, until: last });
      holdCadence(sec, 1, 62, 64);
      bass(sec, { ch: 2, pattern: [[0, "R", 0.9, 1], [1, "5", 0.45, 0.8], [1.5, "5", 0.45, 0.8]], vel: 74 });
      line(sec, { ch: 3, notes: s.melody, octave: s.leadOctave + 1, vel: 44 });
      roots(sec, { ch: 4, octave: -2, vel: 66, hits: [[0, 0.5, 1]] });
      introDrums(sec, [[0, DR.crash, 60]], { skip: onLastBar(sec) });
    },
  },
  /** A bouncing groove: a plucked broken chord, a skipping bass, hand drums. */
  bounce: {
    kit: KIT.standard,
    build(sec, s) {
      const last = lastBarOf(sec);
      arp(sec, { ch: 1, step: 0.5, pattern: [0, 2, 1, 2, 3, 2, 1, 2], center: s.palette.center - 5, vel: 46, until: last });
      bass(sec, { ch: 2, pattern: [[0, "R", 0.45, 1], [1.5, "5", 0.45, 0.8], [2, "8", 0.45, 0.9], [3, "5", 0.45, 0.8]], vel: 70 });
      holdCadence(sec, 3, s.palette.center - 7, 42);
      introDrums(sec, [[0, DR.bongoLo, 62], [0.5, DR.bongoHi, 42], [1, DR.congaHi, 50], [1.5, DR.bongoHi, 40], [2, DR.bongoLo, 58], [2.5, DR.shaker, 34], [3, DR.congaHi, 48], [3.5, DR.bongoHi, 40]], { skip: beforeLastBar(sec) });
      introDrums(sec, [[0, DR.congaLo, 64], [0, DR.shaker, 36]], { skip: onLastBar(sec) });
    },
  },
  /** A gallop: strummed chords on the long-short beat, an alternating bass, woodblock hooves. */
  gallop: {
    kit: KIT.standard,
    build(sec, s) {
      const last = lastBarOf(sec);
      comp(sec, { ch: 1, hits: [[0, 0.2, 1], [0.75, 0.2, 0.6], [1, 0.2, 0.8], [2, 0.2, 0.9], [2.75, 0.2, 0.6], [3, 0.2, 0.8]], center: 60, vel: 50, until: last });
      holdCadence(sec, 1, 60, 46);
      bass(sec, { ch: 2, pattern: [[0, "R", 0.45, 1], [1, "5", 0.45, 0.8], [2, "R", 0.45, 0.9], [3, "5", 0.45, 0.8]], vel: 70 });
      arp(sec, { ch: 3, step: 0.125, pattern: [0, 1, 2, 3, 4, 5, 4, 3], center: s.palette.center - 8, vel: 40, from: last, until: last + 1 });
      introDrums(sec, [[0, DR.blockLo, 54], [0.75, DR.blockHi, 40], [1, DR.blockLo, 48], [2, DR.blockLo, 54], [2.75, DR.blockHi, 40], [3, DR.blockLo, 48]], { skip: beforeLastBar(sec) });
    },
  },
  /** A march: chords on every beat, a snare's ruff and tap, timpani on the strong beats, bells to close. */
  march: {
    kit: KIT.orchestra,
    build(sec, s) {
      const last = lastBarOf(sec);
      comp(sec, { ch: 1, hits: [[0, 0.4, 1], [1, 0.4, 0.7], [2, 0.4, 0.85], [3, 0.4, 0.7]], center: 58, vel: 56, until: last });
      holdCadence(sec, 1, 58, 58);
      bass(sec, { ch: 2, pattern: [[0, "R", 0.9, 1], [2, "5", 0.9, 0.85]], vel: 72 });
      roots(sec, { ch: 4, octave: -2, vel: 64, hits: [[0, 0.5, 1], [2, 0.5, 0.8]] });
      holdCadence(sec, 3, s.palette.center + 2, 40);
      introDrums(sec, [[0, DR.snare, 62], [0.75, DR.snare, 38], [1, DR.snare, 50], [2, DR.snare, 62], [2.75, DR.snare, 38], [3, DR.snare, 50], [3.5, DR.snare, 42]], { skip: beforeLastBar(sec) });
      introDrums(sec, [[0, DR.crash, 58]], { skip: onLastBar(sec) });
    },
  },
  /** A blast: an orchestra hit on each downbeat, low stabs, a tom fill into the cadence. */
  boom: {
    kit: KIT.power,
    build(sec, _s) {
      const last = lastBarOf(sec);
      const bpb = sec.song.beatsPerBar;
      for (let bar = 0; bar < sec.bars; bar += 1) {
        const c = chordAt(sec, bar * bpb);
        note(sec, 3, bar * bpb, 0.9, pitch(sec.k, c.root, -1, alterOf(c, c.root)), 80, { exact: true });
      }
      comp(sec, { ch: 1, hits: [[0, 0.5, 1], [1.5, 0.3, 0.7]], center: 55, vel: 62, until: last });
      holdCadence(sec, 1, 55, 64);
      bass(sec, { ch: 2, pattern: [[0, "R", 1.9, 1], [2, "R", 1.9, 0.9]], vel: 76 });
      timpaniRoll(sec, 4, last - 1, 1, 78);
      introDrums(sec, [[0, DR.kick, 96], [0, DR.crash, 74], [2.5, DR.tomHi, 70], [3, DR.tomMid, 76], [3.5, DR.floorTom, 84]], { skip: beforeLastBar(sec) });
      introDrums(sec, [[0, DR.kick, 100], [0, DR.crash2, 76]], { skip: onLastBar(sec) });
    },
  },
  /** The orchestra: a string ostinato, the choir held, timpani strokes and a roll into the cadence. */
  orchestral: {
    kit: KIT.orchestra,
    build(sec, _s) {
      const last = lastBarOf(sec);
      arp(sec, { ch: 1, step: 0.5, pattern: [0, 1, 2, 1, 0, 2, 3, 2], center: 52, vel: 52, gate: 0.6, until: last });
      holdCadence(sec, 1, 55, 58);
      bass(sec, { ch: 2, pattern: [[0, "R", 1.9, 1], [2, "R", 1.9, 0.8]], vel: 72 });
      pad(sec, { ch: 3, center: 62, vel: 44 });
      roots(sec, { ch: 4, octave: -2, vel: 62, hits: [[0, 1, 1]] });
      timpaniRoll(sec, 4, last - 1, 1, 74);
      introDrums(sec, [[0, DR.crash, 56]], { skip: onLastBar(sec) });
    },
  },
  /** A spell's sparkle: strings held under a harp or celesta's rippling chord, a finger cymbal to close. */
  sparkle: {
    kit: KIT.standard,
    build(sec, s) {
      const last = lastBarOf(sec);
      pad(sec, { ch: 1, center: 60, vel: 42 });
      bass(sec, { ch: 2, pattern: [[0, "R", 1.9, 1], [2, "5", 1.9, 0.8]], octave: -1, vel: 58 });
      arp(sec, { ch: 3, step: 0.5, pattern: [0, 1, 2, 3, 4, 3, 2, 1], center: s.palette.center - 12, vel: 42, until: last });
      glintRun(sec, 3, last, { count: 7, center: s.palette.center - 12, vel: 46 });
      introDrums(sec, [[0, DR.triOpen, 40]], { skip: onLastBar(sec) });
    },
  },
  /** A field opening out: a choir and strings swelling in (expression), the harp walking, a soft roll. */
  swell: {
    kit: null,
    build(sec, s) {
      pad(sec, { ch: 1, center: 62, vel: 46 });
      pad(sec, { ch: 7, center: 55, vel: 34 });
      for (let t = 0; t < sec.song.beatsPerBar; t += 0.25) {
        const v = 70 + (57 * t) / sec.song.beatsPerBar;
        cc(sec, 1, t, 11, v);
        cc(sec, 7, t, 11, v);
      }
      arp(sec, { ch: 3, step: 1, pattern: [0, 2, 4, 2], center: s.palette.center - 10, vel: 44 });
      bass(sec, { ch: 2, pattern: [[0, "R", 1.9, 1]], octave: -1, vel: 58 });
      timpaniRoll(sec, 4, lastBarOf(sec) - 1, 1, 56);
    },
  },
  /** A trap springing: tremolo strings, a low drone, a timpani stroke and a roll, a glint falling. */
  dark: {
    kit: KIT.orchestra,
    build(sec, s) {
      const last = lastBarOf(sec);
      pad(sec, { ch: 1, center: 57, vel: 52 });
      bass(sec, { ch: 2, pattern: [[0, "R", 1.9, 1]], vel: 70 });
      note(sec, 4, 0, 1.5, pitch(sec.k, 0, -2), 76, { exact: true });
      timpaniRoll(sec, 4, last - 0.75, 0.75, 70);
      glintRun(sec, 3, last, { count: 6, center: s.palette.center + 5, vel: 40, down: true });
      introDrums(sec, [[0, DR.kick2, 58]], { skip: onlyBar(0) });
    },
  },
  /** A chant: an organ held, the choir under the lead, a timpani stroke on every beat of the first bar. */
  chant: {
    kit: null,
    build(sec, _s) {
      pad(sec, { ch: 1, center: 57, vel: 46 });
      pad(sec, { ch: 3, center: 62, vel: 40 });
      bass(sec, { ch: 2, pattern: [[0, "R", 1.9, 1]], vel: 64 });
      for (let t = 0; t < sec.song.beatsPerBar; t += 1) note(sec, 4, t, 0.5, pitch(sec.k, 0, -2), t === 0 ? 78 : 60, { exact: true });
      timpaniRoll(sec, 4, lastBarOf(sec) - 0.5, 0.5, 68);
    },
  },
};

/** An intro's score: the lead on the motif, its style's accompaniment, the set's glint, a Mythic's shimmer. */
function cardIntro(card) {
  const s = introSpec(card);
  const style = INTRO_STYLES[s.palette.style];
  if (style === undefined) throw new Error(`intro ${card.card}: no style ${s.palette.style}`);
  const song = createSong({ id: introTrack(card.card), bpm: s.bpm, beatsPerBar: s.bpb, key: key(s.tonic, s.mode) });
  song.reverb = s.mythic ? { room: 0.85, damp: 0.3, width: 1, level: 0.66 } : { room: 0.72, damp: 0.38, width: 1, level: 0.58 };
  song.post.fadeOutS = INTRO_FADE_S;
  song.post.loudnessOverFile = true;
  // The lead sits where its instrument sings: the melody's middle near the palette's centre.
  const mean = s.melody.reduce((sum, [d]) => sum + pitch(song.key, d), 0) / s.melody.length;
  s.leadOctave = Math.round((s.palette.center - mean) / 12);
  instrument(song, 0, s.palette.lead, { volume: 100, pan: 62, reverb: 55 });
  instrument(song, 1, s.palette.second, { volume: 80, pan: 50, reverb: 60 });
  instrument(song, 2, s.palette.bass, { volume: 86, pan: 64, reverb: 35 });
  instrument(song, 3, s.palette.color, { volume: 76, pan: 84, reverb: 65 });
  instrument(song, 4, GM.timpani, { volume: 84, pan: 64, reverb: 60 });
  instrument(song, 6, s.glint, { volume: 66, pan: 30, reverb: 70 });
  instrument(song, 7, s.palette.extra ?? GM.warmPad, { volume: 74, pan: 76, reverb: 60 });
  instrument(song, 10, GM.glock, { volume: 62, pan: 92, reverb: 55 });
  if (s.mythic) {
    instrument(song, 5, GM.choir, { volume: 62, pan: 64, reverb: 85 });
    instrument(song, 8, GM.crystal, { volume: 70, pan: 96, reverb: 80 });
  }
  if (style.kit !== null) instrument(song, DRUMS, style.kit, { volume: 84, reverb: 45 });

  const sec = section(song, "intro", s.bars, s.prog);
  const last = lastBarOf(sec);
  line(sec, { ch: 0, notes: s.melody, octave: s.leadOctave, vel: 90 });
  style.build(sec, s);
  // The set's glint sweeps up into the cadence; a Mythic's prism rings out of it over a soft choir.
  glintRun(sec, 6, last - 0.625, { count: 5, center: 70, vel: 46 });
  if (s.mythic) {
    pad(sec, { ch: 5, center: 64, vel: 34 });
    glintRun(sec, 8, last, { count: 8, step: 0.25, center: 76, vel: 44 });
  }
  s.extra?.(sec, s);
  return { song, tail: INTRO_TAIL_S };
}

/* ----- hand-tuned gestures ----- */

/** A theremin's wobble on `ch`: a slow vibrato in pitch bends across the whole intro, back to rest at its end. */
function wobble(sec, ch, { depth = 1100, perBeat = 2 } = {}) {
  for (let t = 0; t < sec.beats - 1e-9; t += 1 / 16) bend(sec, ch, t, depth * Math.sin(2 * Math.PI * perBeat * t));
  bend(sec, ch, sec.beats, 0);
}

/** A trombone's droop on `ch`: the last note sags a semitone from `at` to the end. */
function droop(sec, ch, at) {
  const steps = 12;
  for (let i = 0; i <= steps; i += 1) bend(sec, ch, at + ((sec.beats - at) * i) / steps, (-4096 * i) / steps);
  bend(sec, ch, sec.beats + 0.5, 0);
}

/** A slot machine's reels: sixteenth notes spinning through the chord on `ch` (10, a glockenspiel), then a bell's ding per effect on 7. */
function slotMachine(sec, ch, dings) {
  const last = lastBarOf(sec);
  arp(sec, { ch, step: 0.25, pattern: [0, 3, 1, 4, 2, 5, 3, 6], center: 74, vel: 38, gate: 0.4, until: last });
  for (let i = 0; i < dings; i += 1) note(sec, 7, last + i * 0.5, 1.5, pitch(sec.k, 7 + 2 * i, 1), 70, { exact: true });
}

/** The League of Losers' tune, which each Loser plays in their own lane's colours and lets fall flat. */
const LEAGUE_MOTIF = [[0, 0.5], [4, 0.5], [7, 1.5], [6, 0.5], [4, 0.5], [3, 0.5]];
const LOSER_MELODY = [...LEAGUE_MOTIF, [2, 0.5], [1, 0.5], [0, 1], [-3, 2]];
const LOSER_PROG = [["i", "VI"], ["iv", "i"]];

/** The motif from the fifth, as the menu theme and every Mythic theme state it. */
const THEME_MOTIF = MOTIF_CELL.map(([step, beats]) => [4 + step, beats]);
/** The motif an octave up from the fifth (degrees 7–10), where the airier themes' melodies sit. */
const HIGH_MOTIF = MOTIF_CELL.map(([step, beats]) => [7 + step, beats]);

/**
 * Every card intro, in catalog order: the card's id, name, set, type and tags as the catalog printed
 * them, its printed rarity, its station where music-cards.json gives one, and its `tune`.
 */
const INTRO_CARDS = [
  // ---- Core ----
  {
    card: "core-052", name: "Silly Silas", set: "Core", type: "Unit", tags: ["Human"], rarity: "Legendary",
    // A comic turn: bassoon over an oom-pah tuba, a trill and a pratfall to the low fifth before it lands.
    tune: {
      palette: { lead: GM.bassoon, second: GM.pizz, bass: GM.tuba, color: GM.xylophone, style: "carnival", center: 58 },
      tonic: "F", mode: "major", bpm: 126, prog: ["I", ["V", "I"]],
      melody: [...MOTIF_CELL, [4, 0.25], [3, 0.25], [4, 0.25], [3, 0.25], [2, 0.5], [-3, 0.5], [0, 2]],
    },
  },
  {
    card: "core-083", name: "Transmogulate", set: "Core", type: "Spell", tags: [], rarity: "Legendary",
    // Everything turns Legendary: a celesta in lydian, and brass taking the cadence as the change lands.
    tune: {
      palette: { lead: GM.celesta, color: GM.harp, extra: GM.brass },
      tonic: "Ab", mode: "lydian", bpm: 104,
      extra: (sec) => holdCadence(sec, 7, 60, 58),
    },
  },
  {
    card: "core-085", name: "Unlicensed Experimentation", set: "Core", type: "Trap", tags: [], rarity: "Legendary",
    // A lab after hours: a theremin's wobble over tremolo strings, in phrygian.
    tune: {
      palette: { lead: GM.whistle, center: 76 },
      tonic: "Gb", mode: "phrygian", bpm: 100,
      extra: (sec) => wobble(sec, 0),
    },
  },
  {
    card: "core-087", name: "Pocket Chaos", set: "Core", type: "Spell", tags: [], rarity: "Legendary",
    // A swap: the motif goes up on the xylophone and comes back upside down.
    tune: {
      palette: { lead: GM.xylophone, second: GM.calliope, bass: GM.tuba, color: GM.glock, style: "carnival", center: 76 },
      tonic: "B", mode: "harmonic", bpm: 138, prog: ["i", ["V", "i"]],
      melody: [...THEME_MOTIF, [3, 0.5], [1, 0.5], [2, 0.5], [-1, 0.5], [0, 2]],
    },
  },
  {
    card: "core-092", name: "Felinor Fiender", set: "Core", type: "Unit", tags: ["Human"], rarity: "Legendary",
    // A prowl in pizzicato, a clarinet stacked a third above it (Stack).
    tune: {
      palette: { lead: GM.pizz, second: GM.strings, bass: GM.pizz, color: GM.glock, style: "pizz", center: 67, extra: GM.clarinet },
      tonic: "D", mode: "dorian", bpm: 116, prog: ["i", ["IV", "i"]],
      melody: [...THEME_MOTIF, [3, 0.75], [2, 0.25], [1, 0.5], [-1, 0.5], [0, 2]],
      extra: (sec, s) => line(sec, { ch: 7, notes: s.melody.map(([d, b]) => [d + 2, b]), octave: s.leadOctave, vel: 58 }),
    },
  },
  {
    card: "core-093", name: "Combo-Index", set: "Core", type: "Field Spell", tags: [], rarity: "Legendary",
    // The grades climb from E to S: the motif, then a run up through the scale to the octave.
    tune: {
      palette: { lead: GM.vibes, color: GM.harp, center: 72 },
      tonic: "E", mode: "major", bpm: 112, prog: ["I", ["V", "I"]],
      melody: [...MOTIF_CELL, [1, 0.25], [2, 0.25], [3, 0.25], [4, 0.25], [5, 0.25], [6, 0.25], [7, 2.5]],
    },
  },
  {
    card: "core-095", name: "Call to Chaos (Core Edition)", set: "Core", type: "Spell", tags: ["Call to Chaos"], rarity: "Legendary",
    // The slot machine spins, and one bell rings for the one effect it rolls.
    tune: {
      palette: { extra: GM.bells },
      tonic: "C", mode: "mixolydian", bpm: 138,
      extra: (sec) => slotMachine(sec, 10, 1),
    },
  },
  {
    card: "core-096", name: "My Pawn", set: "Core", type: "Trap", tags: [], rarity: "Mythic",
    // Its theme's baroque hall: the motif on pizzicato, the harpsichord's sixteenths, a bassoon walking.
    tune: {
      palette: { lead: GM.pizz, second: GM.harpsichord, bass: GM.bassoon, color: GM.strings, style: "baroque", center: 69 },
      tonic: "E", mode: "harmonic", bpm: 104, prog: ["i", ["V", "i"]],
      melody: [...THEME_MOTIF, [3, 0.5], [2, 0.5], [1, 0.5], [-1, 0.5], [0, 2]],
    },
  },
  {
    card: "core-097", name: "Zephyrs", set: "Core", type: "Spell", tags: [], rarity: "Mythic",
    // Its theme's west wind: flute over string tremolo, the harp's glissando, the lydian fourth on top.
    tune: {
      palette: { lead: GM.flute, second: GM.tremolo, bass: GM.fretless, color: GM.harp, style: "swell", center: 79 },
      tonic: "F", mode: "lydian", bpm: 96, prog: ["I", ["ii", "I"]],
      melody: [...HIGH_MOTIF, [10, 0.5], [8, 0.5], [5, 0.5], [6, 0.5], [7, 2]],
      extra: (sec) => glintRun(sec, 3, 0, { count: 9, step: 0.125, center: 60, vel: 50 }),
    },
  },
  {
    card: "core-098", name: "Heroic Power", set: "Core", type: "Field Spell", tags: ["Quickdraw"], rarity: "Mythic",
    // Its theme's fanfare: the timpani's crescendo roll, then the motif on the trumpet, closed on the tonic.
    tune: {
      palette: { lead: GM.trumpet, second: GM.horn, bass: GM.contrabass, color: GM.strings, style: "fanfare", center: 70, extra: GM.trombone },
      tonic: "C", mode: "major", bpm: 96, prog: ["I", ["V", "I"]],
      melody: [...THEME_MOTIF, [2, 1], [1, 0.5], [-1, 0.5], [0, 2]],
      extra: (sec) => {
        timpaniRoll(sec, 4, 0, 1, 78);
        bass(sec, { ch: 7, pattern: [[0, "R", 0.9, 1], [2, "5", 0.9, 0.8]], octave: -1, vel: 60 });
      },
    },
  },
  {
    card: "core-099", name: "Craft a Card", set: "Core", type: "Spell", tags: [], rarity: "Mythic",
    // Its theme's workshop: kalimba on the motif over marimba and woodblocks, climbing to the octave.
    tune: {
      palette: { lead: GM.kalimba, second: GM.marimba, bass: GM.pizz, color: GM.clarinet, style: "bounce", center: 76 },
      tonic: "G", mode: "major", bpm: 108, prog: ["I", ["V", "I"]],
      melody: [...HIGH_MOTIF, [8, 0.5], [9, 0.5], [11, 0.5], [13, 0.5], [14, 2]],
      extra: (sec) => introDrums(sec, [[0, DR.blockLo, 56], [1, DR.blockHi, 44], [1.5, DR.blockHi, 36], [2, DR.blockLo, 52], [3, DR.blockHi, 44], [3.5, DR.claves, 40]], { skip: beforeLastBar(sec) }),
    },
  },
  {
    card: "core-100", name: "Ceaseless Void", set: "Core", type: "Unit", tags: [], rarity: "Mythic",
    // Its theme's void: the motif stretched to twice its length on celesta stars, over choir and drone.
    tune: {
      palette: { lead: GM.celesta, second: GM.choir, bass: GM.contrabass, color: GM.metalPad, style: "chant", center: 79 },
      tonic: "C", mode: "phrygian", bpm: 112, prog: ["i", ["ii", "i"]],
      melody: [[0, 1], [1, 1], [3, 2.5], [2, 0.5], [1, 1], [0, 2]],
    },
  },
  // ---- Classic ----
  { card: "classic-004", name: "Palantir", set: "Classic", type: "Field Spell", tags: ["Jlockeed"], rarity: "Legendary" },
  { card: "classic-007", name: "InfiniScepter", set: "Classic", type: "Field Spell", tags: [], rarity: "Legendary" },
  {
    card: "classic-009", name: "Income Tax", set: "Classic", type: "Trap", tags: [], rarity: "Legendary",
    // The tax collector's march: a muted trumpet counting coins over a tuba.
    tune: {
      palette: { lead: GM.mutedTrumpet, second: GM.strings, bass: GM.tuba, color: GM.celesta, style: "march", center: 69 },
      tonic: "G", mode: "harmonic", bpm: 116, prog: ["i", ["V", "i"]],
      melody: [[4, 0.25], [4, 0.25], [5, 0.5], [7, 1.5], [6, 0.5], [5, 0.5], [4, 0.5], [3, 0.5], [2, 0.5], [1, 0.5], [-1, 0.5], [0, 2]],
    },
  },
  { card: "classic-028", name: "Second Wind", set: "Classic", type: "Field Spell", tags: [], rarity: "Legendary" },
  {
    card: "classic-033", name: "Joro", set: "Classic", type: "Unit", tags: [], rarity: "Legendary",
    // A spider on its web: harpsichord in harmonic minor, the answer creeping over the flat sixth.
    tune: {
      palette: { lead: GM.harpsichord, second: GM.pizz, bass: GM.cello, color: GM.celesta, style: "baroque", center: 67 },
      tonic: "Db", mode: "harmonic", bpm: 108, prog: ["i", ["V", "i"]],
      melody: [...MOTIF_CELL, [6, 0.5], [5, 0.5], [6, 0.5], [4, 0.5], [0, 2]],
    },
  },
  { card: "classic-044", name: "Back from the GY", set: "Classic", type: "Spell", tags: [], rarity: "Legendary" },
  {
    card: "classic-045", name: "Nature Titan", set: "Classic", type: "Unit", tags: [], rarity: "Legendary",
    // Something huge waking: horns on the motif widened to a fifth and an octave, choir, timpani.
    tune: {
      palette: { lead: GM.horn, second: GM.strings, bass: GM.contrabass, color: GM.choir, style: "orchestral", center: 60 },
      tonic: "Eb", mode: "mixolydian", bpm: 96, prog: ["I", ["VII", "I"]],
      melody: [[0, 1], [4, 0.5], [7, 1.5], [6, 0.5], [4, 0.5], [6, 1], [3, 1], [0, 2]],
    },
  },
  {
    card: "classic-056", name: "Spell Tyrant", set: "Classic", type: "Unit", tags: [], rarity: "Legendary",
    // A tyrant's entrance: trombone and organ in B-flat minor.
    tune: {
      palette: { lead: GM.trombone, second: GM.organ, bass: GM.contrabass, color: GM.celesta, style: "fanfare", center: 58 },
      tonic: "Bb", mode: "minor", bpm: 104,
    },
  },
  {
    card: "classic-080", name: "BOOM! Big Max", set: "Classic", type: "Unit", tags: ["Acclaimed"], rarity: "Legendary",
    tune: { tonic: "D", mode: "harmonic", bpm: 120 },
  },
  {
    card: "classic-085", name: "King Wagtoggle", set: "Classic", type: "Unit", tags: [], rarity: "Legendary",
    // A royal herald: trumpet with a repeated-note pickup over strings, harpsichord and timpani.
    tune: {
      palette: { lead: GM.trumpet, second: GM.strings, bass: GM.contrabass, color: GM.harpsichord, style: "fanfare", center: 72 },
      tonic: "D", mode: "major", bpm: 112, prog: ["I", ["V", "I"]],
      melody: [[4, 0.25], [4, 0.25], [5, 0.5], [7, 1.5], [6, 0.5], [5, 0.5], [4, 0.5], [2, 0.5], [4, 0.5], [3, 0.5], [1, 0.5], [0, 2]],
    },
  },
  {
    card: "classic-090", name: "In Too Deep", set: "Classic", type: "Field Spell", tags: ["Quickdraw"], rarity: "Mythic",
    // Its theme's deep water: the harp descending, vibes on the motif sinking back to the tonic.
    tune: {
      palette: { lead: GM.vibes, second: GM.haloPad, bass: GM.fretless, color: GM.harp, style: "swell", center: 74, extra: GM.atmosphere },
      tonic: "Eb", mode: "lydian", bpm: 100, prog: ["I", ["ii", "I"]],
      melody: [...HIGH_MOTIF, [5, 0.5], [4, 0.5], [2, 0.5], [1, 0.5], [0, 2]],
      extra: (sec) => glintRun(sec, 3, 0, { count: 8, step: 0.25, center: 72, vel: 46, down: true }),
    },
  },
  // ---- Classic+ ----
  {
    card: "classicplus-012", name: "The Mother Pancake", set: "Classic+", type: "Unit", tags: ["Pancake"], rarity: "Legendary",
    // The whole griddle: bright brass, the motif opened to a fifth and an octave, a glockenspiel on top.
    tune: {
      palette: { second: GM.brass, color: GM.glock },
      tonic: "Bb", mode: "major", bpm: 108, prog: ["I", ["V", "I"]],
      melody: [[0, 0.5], [4, 0.5], [7, 1.5], [6, 0.5], [5, 0.5], [4, 0.5], [8, 0.5], [6, 0.5], [4, 0.5], [6, 0.5], [7, 2]],
    },
  },
  { card: "classicplus-012-1", name: "Devour", set: "Classic+", type: "Spell", tags: ["Pancake", "Token"], rarity: "Legendary", tune: { mode: "minor" } },
  { card: "classicplus-012-2", name: "Death Boil", set: "Classic+", type: "Spell", tags: ["Pancake", "Token"], rarity: "Legendary", tune: { mode: "harmonic" } },
  { card: "classicplus-012-3", name: "Fluffy Grip", set: "Classic+", type: "Spell", tags: ["Pancake", "Token"], rarity: "Legendary" },
  { card: "classicplus-012-4", name: "Powder Spray", set: "Classic+", type: "Spell", tags: ["Pancake", "Token"], rarity: "Legendary" },
  { card: "classicplus-012-5", name: "Anti-Waffle Shell", set: "Classic+", type: "Field Spell", tags: ["Pancake", "Token"], rarity: "Legendary" },
  { card: "classicplus-012-6", name: "Frozen Wastes", set: "Classic+", type: "Spell", tags: ["Pancake", "Token"], rarity: "Legendary", tune: { mode: "minor", palette: { color: GM.celesta } } },
  { card: "classicplus-012-7", name: "Legion of the Hungry", set: "Classic+", type: "Field Spell", tags: ["Pancake", "Token"], rarity: "Legendary", tune: { palette: { style: "march" } } },
  { card: "classicplus-012-8", name: "Frostspatula", set: "Classic+", type: "Field Spell", tags: ["Pancake", "Token"], rarity: "Legendary", tune: { palette: { color: GM.celesta } } },
  { card: "classicplus-013", name: "Mommy Barker", set: "Classic+", type: "Unit", tags: ["Human", "Pancake"], rarity: "Legendary" },
  {
    card: "classicplus-019", name: "League of Losers", set: "Classic+", type: "Spell", tags: [], rarity: "Legendary",
    // The match is starting: a synth-brass call to arms that its five Losers each go on to fumble.
    tune: {
      palette: { lead: GM.synthBrass, second: GM.strings, bass: GM.synthBass1, color: GM.bells, style: "orchestral", center: 64 },
      tonic: "A", mode: "minor", bpm: 104, prog: [["i", "VI"], ["V+", "i"]],
      melody: [...LEAGUE_MOTIF, [4, 0.5], [6, 0.5], [8, 1], [7, 2]],
    },
  },
  {
    card: "classicplus-019-1", name: "Top Loser", set: "Classic+", type: "Unit", tags: ["Token"], rarity: "Legendary",
    // The top lane's tank: the League's call on a trombone that sags on its last note.
    tune: {
      palette: { lead: GM.trombone, second: GM.strings, bass: GM.tuba, color: GM.harp, style: "fanfare", center: 56 },
      tonic: "A", mode: "minor", bpm: 100, prog: LOSER_PROG, melody: LOSER_MELODY,
      extra: (sec) => droop(sec, 0, 6.5),
    },
  },
  {
    card: "classicplus-019-2", name: "Jungle Loser", set: "Classic+", type: "Unit", tags: ["Token"], rarity: "Legendary",
    tune: {
      palette: { lead: GM.panFlute, second: GM.marimba, bass: GM.acBass, color: GM.kalimba, style: "bounce", center: 74 },
      tonic: "A", mode: "dorian", bpm: 120, prog: LOSER_PROG, melody: LOSER_MELODY,
    },
  },
  {
    card: "classicplus-019-3", name: "Mid Loser", set: "Classic+", type: "Unit", tags: ["Token"], rarity: "Legendary",
    // The mid lane's mage flips a coin: two bright tings over a synth that comes down on tails.
    tune: {
      palette: { lead: GM.square, second: GM.polysynth, bass: GM.synthBass2, color: GM.crystal, style: "synth", center: 67, extra: GM.bells },
      tonic: "A", mode: "minor", bpm: 128, prog: LOSER_PROG, melody: LOSER_MELODY,
      extra: (sec) => {
        note(sec, 7, 0, 0.5, pitch(sec.k, 7, 1), 64, { exact: true });
        note(sec, 7, 0.5, 0.75, pitch(sec.k, 11, 1), 58, { exact: true });
      },
    },
  },
  {
    card: "classicplus-019-4", name: "Support Loser", set: "Classic+", type: "Unit", tags: ["Token"], rarity: "Legendary",
    tune: {
      palette: { lead: GM.flute, second: GM.choir, bass: GM.cello, color: GM.harp, style: "swell", center: 74 },
      tonic: "A", mode: "dorian", bpm: 100, prog: LOSER_PROG, melody: LOSER_MELODY,
    },
  },
  {
    card: "classicplus-019-5", name: "Bot Loser", set: "Classic+", type: "Unit", tags: ["Token"], rarity: "Legendary",
    tune: {
      palette: { lead: GM.saw, second: GM.square, bass: GM.synthBass2, color: GM.crystal, style: "synth", center: 69 },
      tonic: "A", mode: "minor", bpm: 140, prog: LOSER_PROG, melody: LOSER_MELODY,
    },
  },
  {
    card: "classicplus-027", name: "Zephrys Zealotism", set: "Classic+", type: "Spell", tags: [], rarity: "Mythic",
    // Its theme's chant: timpani on every beat, the motif on pan flute over organ and choir, in dorian.
    tune: {
      palette: { lead: GM.panFlute, second: GM.organ, bass: GM.contrabass, color: GM.choir, style: "chant", center: 74 },
      tonic: "D", mode: "dorian", bpm: 104, prog: ["i", ["IV", "i"]],
      melody: [...THEME_MOTIF, [5, 0.5], [3, 0.5], [2, 0.5], [1, 0.5], [0, 2]],
    },
  },
  {
    card: "classicplus-029", name: "Portal to the Past", set: "Classic+", type: "Spell", tags: [], rarity: "Mythic",
    // Its theme's music box: the motif, then the motif turned back on itself, a viola rising against it.
    tune: {
      palette: { lead: GM.musicBox, second: GM.strings, bass: GM.cello, color: GM.glock, style: "sparkle", center: 79, extra: GM.viola },
      tonic: "Bb", mode: "major", bpm: 100, prog: [["I", "vi"], ["V", "I"]],
      melody: [...HIGH_MOTIF, [7, 0.25], [8, 0.25], [9, 0.5], [10, 1], [8, 0.5], [7, 1.5]],
      extra: (sec) => line(sec, { ch: 7, notes: [[2, 1], [3, 1], [4, 1], [5, 1], [4, 2], [2, 2]], octave: -1, vel: 60 }),
    },
  },
  { card: "classicplus-035", name: "Rollback", set: "Classic+", type: "Spell", tags: [], rarity: "Legendary" },
  {
    card: "classicplus-037", name: "Wardrum", set: "Classic+", type: "Unit", tags: ["Quickdraw", "Acclaimed"], rarity: "Legendary", station: "epic",
    // Its Epic Orchestral station's forces, under war drums.
    tune: {
      tonic: "D", mode: "minor", bpm: 108,
      extra: (sec) => introDrums(sec, [[0, DR.floorTom, 92], [0.75, DR.tomLo, 60], [1, DR.floorTom, 80], [2, DR.floorTom, 90], [2.5, DR.tomLo, 64], [3, DR.tomMid, 72], [3.5, DR.tomLo, 70]], { skip: beforeLastBar(sec) }),
    },
  },
  { card: "classicplus-042", name: "KY's Test", set: "Classic+", type: "Spell", tags: ["KY"], rarity: "Legendary" },
  { card: "classicplus-042-1", name: "KY's Gift", set: "Classic+", type: "Field Spell", tags: ["KY", "Token"], rarity: "Legendary" },
  {
    card: "classicplus-043", name: "AI Slop", set: "Classic+", type: "Spell", tags: [], rarity: "Legendary",
    // Generated: the motif stutters in on a square wave over synths.
    tune: {
      palette: { lead: GM.square, second: GM.polysynth, bass: GM.synthBass2, color: GM.crystal, style: "synth", center: 67 },
      tonic: "E", mode: "minor", bpm: 128, prog: ["i", ["V+", "i"]],
      melody: [[4, 0.25], [4, 0.25], [4, 0.25], [5, 0.25], [7, 1.5], [6, 0.5], [5, 0.5], [4, 0.5], [3, 0.25], [3, 0.25], [2, 0.5], [1, 0.5], [-1, 0.5], [0, 2]],
    },
  },
  { card: "classicplus-046", name: "Felinor Flagbearer", set: "Classic+", type: "Unit", tags: ["Felinor", "Catalyst"], rarity: "Legendary" },
  { card: "classicplus-046-1", name: "Felinor Flagbearer Prime", set: "Classic+", type: "Unit", tags: ["Felinor", "Prime", "Token"], rarity: "Legendary" },
  {
    card: "classicplus-047", name: "Jogg's Box", set: "Classic+", type: "Spell", tags: [], rarity: "Legendary",
    // A box of spells opening: a music box over strings, a celesta's ripple.
    tune: {
      palette: { lead: GM.musicBox, second: GM.strings, bass: GM.cello, color: GM.celesta, style: "sparkle", center: 79 },
      tonic: "G", mode: "major", bpm: 112,
    },
  },
  { card: "classicplus-048", name: "Jlockheed's Lobbyist", set: "Classic+", type: "Unit", tags: ["Jlockeed"], rarity: "Legendary" },
  { card: "classicplus-065-4", name: "Golden Grape", set: "Classic+", type: "Spell", tags: ["Fruit", "Token"], rarity: "Legendary" },
  {
    card: "classicplus-065-5", name: "Mythic Grape", set: "Classic+", type: "Spell", tags: ["Fruit", "Token"], rarity: "Mythic",
    // A grape gone prismatic: marimba and kalimba in lydian under the Mythic shimmer.
    tune: { tonic: "D", mode: "lydian", bpm: 112 },
  },
  { card: "classicplus-073", name: "Call to Chaos (Classic+ Edition)", set: "Classic+", type: "Spell", tags: ["Call to Chaos"], rarity: "Legendary",
    // Its own slot machine, on a square wave in harmonic minor.
    tune: {
      palette: { lead: GM.square, extra: GM.bells },
      tonic: "E", mode: "harmonic", bpm: 144,
      extra: (sec) => slotMachine(sec, 10, 1),
    },
  },
  {
    card: "classicplus-073-1", name: "Classic Golem", set: "Classic+", type: "Unit", tags: ["Token"], rarity: "Legendary",
    // Stone on the march: low brass and timpani.
    tune: { palette: { lead: GM.tuba, second: GM.trombone, bass: GM.contrabass, color: GM.harp, style: "march", center: 52 } },
  },
  {
    card: "classicplus-074", name: "Twice Forward One Step Backwards", set: "Classic+", type: "Field Trap", tags: [], rarity: "Mythic",
    // Its theme's lopsided waltz: the clarinet takes two steps forward and one back, bar after bar.
    tune: {
      palette: { lead: GM.clarinet, second: GM.pizz, bass: GM.bassoon, color: GM.accordion, style: "pizz", center: 70 },
      tonic: "A", mode: "minor", bpm: 132, beatsPerBar: 3, bars: 3, prog: ["i", "V+", "i"],
      melody: [[4, 0.5], [5, 0.5], [7, 1], [6, 0.5], [5, 0.5], [6, 0.5], [7, 0.5], [8, 1], [7, 0.5], [6, 0.5], [5, 0.5], [4, 0.5], [7, 2]],
    },
  },
  {
    card: "classicplus-075", name: "J-lease J-Jungle EX-plorer", set: "Classic+", type: "Unit", tags: [], rarity: "Legendary",
    // Into the jungle: pan flute over marimba and bongos.
    tune: {
      palette: { lead: GM.panFlute, second: GM.marimba, bass: GM.acBass, color: GM.kalimba, style: "bounce", center: 74 },
      tonic: "F", mode: "mixolydian", bpm: 120,
    },
  },
  { card: "classicplus-075-1", name: "J-lease J-Jungle EX-plorer Pack", set: "Classic+", type: "Spell", tags: ["Token"], rarity: "Legendary" },
  {
    card: "classicplus-078", name: "Claude's Datacenter", set: "Classic+", type: "Field Spell", tags: [], rarity: "Legendary",
    // Racks humming: an FM piano over a warm pad, crystal arpeggios, a pulse like fans spinning up.
    tune: {
      palette: { lead: GM.ep2, second: GM.warmPad, bass: GM.synthBass1, color: GM.crystal, style: "synth", center: 72 },
      tonic: "C", mode: "dorian", bpm: 116,
    },
  },
];

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
  { id: "mythic-twice-forward", loop: true, build: twiceForward },
  { id: "mythic-portal-to-the-past", loop: true, build: portalToThePast },
  { id: "legendary-1", loop: true, build: legendaryTheme1 },
  { id: "legendary-2", loop: true, build: legendaryTheme2 },
  // R1352: each Legendary's and Mythic's own intro, a sting that plays once.
  ...INTRO_CARDS.map((card) => ({ id: introTrack(card.card), loop: false, build: () => cardIntro(card) })),
];

/** R1352: which card each intro is, for music-cards.json's `intro` and the licence record. */
export const INTRO_TRACKS = INTRO_CARDS.map((card) => ({ card: card.card, name: card.name, track: introTrack(card.card), family: introFamily(card) }));
