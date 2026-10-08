// The composition toolkit for gen-music.mjs (SPEC §10.11 "Music"): a song is an intro and a loop
// body, each a run of sections over a chord progression, and parts write notes into a section. The
// body is built once and the renderer repeats it, so every pass of the loop is the same notes.

import { DRUMS } from "./midi.mjs";
import { chordClasses, chordDegrees, pitch, rng, seedOf, voicing } from "./theory.mjs";

export { DRUMS };

/* ------------------------------------------------------------------------------------------- *
 * Chords
 * ------------------------------------------------------------------------------------------- */

const NUMERALS = { i: 0, ii: 1, iii: 2, iv: 3, v: 4, vi: 5, vii: 6 };

/**
 * A Roman numeral read diatonically in the song's key: "I", "vi", "V7", "ii9", "Isus4", and "V+"
 * for a dominant with its third raised (the leading tone in a minor key). Case is only for the
 * reader: the key's mode decides each chord's quality.
 */
export function chord(token) {
  const m = /^(vii|vi|iv|v|iii|ii|i)(\+?)(7?)(9?)(sus[24])?$/i.exec(token);
  if (m === null) throw new Error(`bad chord ${token}`);
  const root = NUMERALS[m[1].toLowerCase()];
  const sus = m[5] === undefined ? 0 : Number(m[5].slice(3));
  const degrees = chordDegrees(root, { seventh: m[3] === "7", ninth: m[4] === "9", sus });
  const alter = new Map();
  if (m[2] === "+") alter.set((root + 2) % 7, 1);
  return { token, root, degrees, alter };
}

/** The semitones a chord alters a scale degree by (a raised leading tone), else 0. */
export function alterOf(c, degree) {
  return c?.alter.get(((degree % 7) + 7) % 7) ?? 0;
}

export function chordPitches(k, c, octave = 0) {
  return c.degrees.map((d) => pitch(k, d, octave, alterOf(c, d)));
}

/* ------------------------------------------------------------------------------------------- *
 * The song
 * ------------------------------------------------------------------------------------------- */

/**
 * `bpm`, `beatsPerBar`, the key (theory.key) and an id that seeds the variety. `swing` delays the
 * off-beat eighths (lo-fi) by that fraction of a beat.
 */
export function createSong({ id, bpm, beatsPerBar = 4, key, swing = 0, humanize = 0.012 }) {
  return {
    id,
    bpm,
    beatsPerBar,
    key,
    swing,
    humanize,
    rng: rng(seedOf(id)),
    channels: {},
    parts: { intro: { notes: [], ccs: [], bends: [], beats: 0 }, body: { notes: [], ccs: [], bends: [], beats: 0 } },
    post: {},
    reverb: { room: 0.6, damp: 0.4, width: 0.8, level: 0.55 },
  };
}

/** Names a channel's instrument: a General MIDI program (0-based), or a drum kit on DRUMS. */
export function instrument(song, ch, program, { volume = 100, pan = 64, reverb = 40 } = {}) {
  song.channels[ch] = { program, volume, pan, reverb };
}

/**
 * Appends a section of `bars` bars to the intro or the body. `progression` is one entry per bar:
 * a chord token for the whole bar, or a list of tokens that split the bar evenly. Returns the
 * section the parts write into.
 */
export function section(song, where, bars, progression) {
  const part = song.parts[where];
  const bpb = song.beatsPerBar;
  const spans = [];
  for (let bar = 0; bar < bars; bar += 1) {
    const entry = progression[bar % progression.length];
    const tokens = Array.isArray(entry) ? entry : [entry];
    const beats = bpb / tokens.length;
    tokens.forEach((token, i) => {
      spans.push({ chord: chord(token), t: bar * bpb + i * beats, beats, bar });
    });
  }
  const sec = { song, part, start: part.beats, bars, beats: bars * bpb, spans, k: song.key, rng: song.rng };
  part.beats += bars * bpb;
  return sec;
}

/** The chord sounding at beat `t` of the section. */
export function chordAt(sec, t) {
  let found = sec.spans[0].chord;
  for (const span of sec.spans) if (span.t <= t + 1e-9) found = span.chord;
  return found;
}

/** Writes one note at beat `t` of the section; melodic parts get a hair of timing and velocity life. */
export function note(sec, ch, t, dur, midi, vel, { exact = false } = {}) {
  const song = sec.song;
  let at = sec.start + t;
  if (song.swing > 0 && Math.abs(at - Math.floor(at) - 0.5) < 1e-6) at += song.swing;
  if (!exact && ch !== DRUMS) at += sec.rng.jitter(song.humanize);
  const v = vel + (exact ? 0 : sec.rng.jitter(5));
  sec.part.notes.push({ t: Math.max(0, at), dur: Math.max(0.05, dur), ch, note: midi, vel: Math.max(1, Math.min(127, v)) });
}

export function cc(sec, ch, t, controller, value) {
  sec.part.ccs.push({ t: sec.start + t, ch, cc: controller, value });
}

/**
 * A pitch bend at beat `t` of the section: `value` in MIDI's -8192..8191, which FluidSynth reads as
 * up to two semitones either way (a theremin's wobble, a trombone's droop). Bend back to 0 after.
 */
export function bend(sec, ch, t, value) {
  sec.part.bends.push({ t: sec.start + t, ch, value });
}

/* ------------------------------------------------------------------------------------------- *
 * Accompaniment
 * ------------------------------------------------------------------------------------------- */

/** Held chords, voice-led around `center`. `every` re-strikes a long chord each that many beats. */
export function pad(sec, { ch, center = 60, vel = 56, every = 0, octave = 0 }) {
  let prev;
  for (const span of sec.spans) {
    const voiced = voicing(chordPitches(sec.k, span.chord, octave), center, prev);
    prev = voiced;
    const step = every > 0 ? every : span.beats;
    for (let t = 0; t < span.beats - 1e-9; t += step) {
      for (const n of voiced) note(sec, ch, span.t + t, Math.min(step, span.beats - t) - 0.04, n, vel);
    }
  }
}

/**
 * An arpeggio over each chord: `pattern` indexes the voiced chord extended by an octave (0 is the
 * lowest tone), one index per `step` beats; `accent` scales the velocity in a repeating cycle.
 */
export function arp(sec, { ch, step = 0.5, pattern = [0, 1, 2, 3, 2, 1], center = 60, vel = 60, accent = [1], gate = 0.9, from = 0, until = Infinity }) {
  let prev;
  let i = 0;
  for (const span of sec.spans) {
    const voiced = voicing(chordPitches(sec.k, span.chord), center, prev);
    prev = voiced;
    const ext = [...voiced, ...voiced.map((n) => n + 12), ...voiced.map((n) => n + 24)];
    for (let t = 0; t < span.beats - 1e-9; t += step) {
      const at = span.t + t;
      if (at >= from && at < until) {
        const idx = pattern[i % pattern.length];
        if (idx !== null) note(sec, ch, at, step * gate, ext[idx], vel * accent[i % accent.length]);
      }
      i += 1;
    }
  }
}

/**
 * A bass line: per chord span, `pattern` is [beat, tone, beats, velocity scale] with tone "R" root,
 * "5" fifth, "8" octave, "3" third, "A" a step into the next chord's root.
 */
export function bass(sec, { ch, pattern = [[0, "R", 1.9, 1], [2, "5", 1.9, 0.9]], vel = 80, octave = -2 }) {
  sec.spans.forEach((span, s) => {
    const c = span.chord;
    const root = pitch(sec.k, c.root, octave, alterOf(c, c.root));
    const fifth = pitch(sec.k, c.root + 4, octave, alterOf(c, c.root + 4));
    const third = pitch(sec.k, c.root + 2, octave, alterOf(c, c.root + 2));
    const next = sec.spans[(s + 1) % sec.spans.length].chord;
    let nextRoot = pitch(sec.k, next.root, octave);
    while (nextRoot - root > 6) nextRoot -= 12;
    while (root - nextRoot > 6) nextRoot += 12;
    for (const [t, tone, dur, scale] of pattern) {
      if (t >= span.beats - 1e-9) continue;
      let n = root;
      if (tone === "5") n = fifth <= root ? fifth + 12 : fifth;
      if (tone === "5" && n - root > 7) n -= 12;
      if (tone === "8") n = root + 12;
      if (tone === "3") n = third;
      if (tone === "A") n = nextRoot === root ? root + 2 : nextRoot + (nextRoot > root ? -1 : 1);
      note(sec, ch, span.t + t, Math.min(dur, span.beats - t), n, vel * scale);
    }
  });
}

/** Chord stabs: `hits` is [beat, beats, velocity scale] within each chord span. */
export function comp(sec, { ch, hits, center = 60, vel = 60, from = 0, until = Infinity }) {
  let prev;
  for (const span of sec.spans) {
    const voiced = voicing(chordPitches(sec.k, span.chord), center, prev);
    prev = voiced;
    for (const [t, dur, scale] of hits) {
      const at = span.t + t;
      if (t >= span.beats - 1e-9 || at < from || at >= until) continue;
      voiced.forEach((n, i) => note(sec, ch, at + i * 0.012, Math.min(dur, span.beats - t), n, vel * scale));
    }
  }
}

/**
 * A drum part: `pattern` is one bar of [beat, drum note, velocity]. `fill` replaces the section's
 * last bar. `skip(bar)` leaves a bar out (a breakdown).
 */
export function drums(sec, { pattern, fill = null, vel = 1, skip = () => false, from = 0 }) {
  for (let bar = from; bar < sec.bars; bar += 1) {
    if (skip(bar)) continue;
    const use = fill !== null && bar === sec.bars - 1 ? fill : pattern;
    for (const [t, n, v] of use) note(sec, DRUMS, bar * sec.song.beatsPerBar + t, 0.25, n, v * vel);
  }
}

/** A held note per chord span on its root (a drone or a timpani roll's anchor). */
export function roots(sec, { ch, octave = -1, vel = 60, hits = null }) {
  for (const span of sec.spans) {
    const n = pitch(sec.k, span.chord.root, octave, alterOf(span.chord, span.chord.root));
    if (hits === null) note(sec, ch, span.t, span.beats - 0.05, n, vel);
    else for (const [t, dur, scale] of hits) if (t < span.beats) note(sec, ch, span.t + t, dur, n, vel * scale);
  }
}

/** Repeated notes on each chord's root (a pulse): one every `step` beats, accents cycling. */
export function pulse(sec, { ch, step = 0.5, octave = -1, vel = 60, accent = [1, 0.7], gate = 0.6, tone = "R" }) {
  let i = 0;
  for (const span of sec.spans) {
    const c = span.chord;
    const d = tone === "5" ? c.root + 4 : c.root;
    const n = pitch(sec.k, d, octave, alterOf(c, d));
    for (let t = 0; t < span.beats - 1e-9; t += step) {
      note(sec, ch, span.t + t, step * gate, n, vel * accent[i % accent.length]);
      i += 1;
    }
  }
}

/** Sidechain pumping: CC11 dips on every beat and swells back, on each of `chs` (EDM). */
export function pump(sec, { chs, depth = 0.55, steps = 8 }) {
  for (let beat = 0; beat < sec.beats; beat += 1) {
    for (let s = 0; s < steps; s += 1) {
      const frac = s / steps;
      const value = 127 * (1 - depth * Math.pow(1 - frac, 2));
      for (const ch of chs) cc(sec, ch, beat + frac * 0.75, 11, value);
    }
  }
}

/* ------------------------------------------------------------------------------------------- *
 * Melody
 * ------------------------------------------------------------------------------------------- */

/** A melody note's pitch: its scale degree, altered where the chord under it alters that degree. */
export function melodic(sec, degree, t, octave = 0) {
  return pitch(sec.k, degree, octave, alterOf(chordAt(sec, t), degree));
}

/**
 * An explicit line: `notes` is [degree or null (a rest), beats, velocity scale?] in order from beat
 * `at`. Degrees are the key's, from the tonic of `octave`.
 */
export function line(sec, { ch, notes, octave = 0, vel = 80, at = 0, legato = 0.95 }) {
  let t = at;
  for (const [degree, beats, scale = 1] of notes) {
    if (degree !== null) note(sec, ch, t, beats * legato, melodic(sec, degree, t, octave), vel * scale);
    t += beats;
  }
  return t;
}

/** "Jack-i-Oh": the motif every track quotes. The first bar, as [degree step from its start, beats]. */
export const MOTIF_CELL = [[0, 0.5], [1, 0.5], [3, 1.5], [2, 0.5], [1, 0.5], [0, 0.5]];

/**
 * A generated melody over the section's chords. Rhythms come from `rhythms` (bars of [beats, rest?]),
 * repeated in a phrase shape (A B A C, then A B D cadence) so the line sounds composed rather than
 * random; strong beats land on chord tones near the last note, weak beats move by step, and every
 * fourth bar closes on a long chord tone (the eighth on the tonic). `motifBars` start with the motif
 * cell. `range` bounds the degrees.
 */
export function melody(sec, { ch, rhythms, range = [2, 11], octave = 0, vel = 78, motifBars = [0], start = 4, legato = 0.92, seed = 0, cadence = true }) {
  const r = rng(seedOf(`${sec.song.id}:${ch}:${sec.start}:${seed}`));
  const bpb = sec.song.beatsPerBar;
  const shape = [0, 1, 0, 2, 0, 1, 3, 4];
  const chosen = [0, 1, 2, 3, 4].map(() => r.pick(rhythms));
  let prev = start;
  let dir = 1;
  const clampDeg = (d) => Math.max(range[0], Math.min(range[1], d));
  const nearestTone = (classes, from, avoid) => {
    let best = null;
    for (let d = range[0]; d <= range[1]; d += 1) {
      if (!classes.has(((d % 7) + 7) % 7)) continue;
      if (avoid && d === from) continue;
      if (best === null || Math.abs(d - from) < Math.abs(best - from) || (Math.abs(d - from) === Math.abs(best - from) && r.chance(0.5))) best = d;
    }
    return best ?? from;
  };
  for (let bar = 0; bar < sec.bars; bar += 1) {
    const t0 = bar * bpb;
    const inPhrase = bar % 8;
    const closing = cadence && (inPhrase === 3 || inPhrase === 7);
    if (motifBars.includes(bar)) {
      const c = chordAt(sec, t0);
      const anchor = nearestTone(chordClasses(c.degrees), prev, false);
      const base = clampDeg(anchor + 3 > range[1] ? anchor - 3 : anchor);
      let t = t0;
      for (const [step, beats] of MOTIF_CELL) {
        if (t - t0 >= bpb - 1e-9) break;
        const d = base + step;
        note(sec, ch, t, Math.min(beats, t0 + bpb - t) * legato, melodic(sec, d, t, octave), vel * (step === 3 ? 1.1 : 1));
        prev = d;
        t += beats;
      }
      continue;
    }
    if (closing) {
      const c = chordAt(sec, t0);
      const classes = inPhrase === 7 ? new Set([c.root % 7]) : chordClasses(c.degrees);
      // A pickup into the held note, then the note itself.
      const target = nearestTone(classes, prev, false);
      const pickup = clampDeg(target + (target > prev ? -1 : 1));
      note(sec, ch, t0, 0.9 * legato, melodic(sec, pickup, t0, octave), vel * 0.85);
      note(sec, ch, t0 + 1, (bpb - 1.1) * legato, melodic(sec, target, t0 + 1, octave), vel);
      prev = target;
      continue;
    }
    const rhythm = chosen[shape[inPhrase]];
    let t = t0;
    for (const [beats, rest] of rhythm) {
      if (t - t0 >= bpb - 1e-9) break;
      if (!rest) {
        const strong = Math.abs((t - t0) % 2) < 1e-9 || (bpb === 3 && Math.abs(t - t0) < 1e-9);
        let d;
        if (strong) {
          d = nearestTone(chordClasses(chordAt(sec, t).degrees), prev, r.chance(0.4));
        } else {
          if (r.chance(0.25)) dir = -dir;
          d = prev + dir;
          if (d > range[1] || d < range[0]) {
            dir = -dir;
            d = prev + dir;
          }
        }
        d = clampDeg(d);
        if (d > range[1] - 1) dir = -1;
        if (d < range[0] + 1) dir = 1;
        note(sec, ch, t, Math.min(beats, t0 + bpb - t) * legato, melodic(sec, d, t, octave), vel * (strong ? 1 : 0.86));
        prev = d;
      }
      t += beats;
    }
  }
}

/* Rhythm banks, one bar each: [beats, rest?]. */
export const RHYTHMS = {
  folk: [
    [[1], [0.5], [0.5], [1.5], [0.5]],
    [[0.5], [0.5], [1], [1], [1]],
    [[1.5], [0.5], [1], [1]],
    [[1], [1], [0.5], [0.5], [1]],
    [[0.5], [0.5], [0.5], [0.5], [2]],
  ],
  waltz: [
    [[1], [1], [1]],
    [[2], [1]],
    [[1.5], [0.5], [1]],
    [[0.5], [0.5], [1], [1]],
  ],
  edm: [
    [[0.75], [0.75], [0.5], [1], [1]],
    [[0.5], [0.5, true], [0.5], [0.5], [1], [1]],
    [[1.5], [0.5], [0.5], [0.5], [1]],
    [[0.75], [0.75], [0.75], [0.75], [1]],
  ],
  lofi: [
    [[1.5], [0.5], [2, true]],
    [[0.5, true], [0.5], [0.5], [0.5], [2]],
    [[1], [0.5], [1.5], [1, true]],
    [[2], [0.5], [0.5], [1, true]],
    [[0.75], [0.75], [0.5], [2]],
  ],
  epic: [
    [[2], [1], [1]],
    [[1.5], [0.5], [2]],
    [[1], [1], [2]],
    [[3], [1]],
  ],
};

/* General MIDI drum notes. */
export const DR = {
  kick: 36, kick2: 35, stick: 37, snare: 38, clap: 39, esnare: 40, tomLo: 45, tomMid: 47, tomHi: 50, floorTom: 41,
  hat: 42, pedal: 44, open: 46, crash: 49, ride: 51, rideBell: 53, tamb: 54, splash: 55, crash2: 57,
  bongoHi: 60, bongoLo: 61, congaMute: 62, congaHi: 63, congaLo: 64, timbHi: 65, timbLo: 66,
  cabasa: 69, maracas: 70, claves: 75, blockHi: 76, blockLo: 77, triMute: 80, triOpen: 81, shaker: 82,
};

/* General MIDI programs (0-based). */
export const GM = {
  piano: 0, brightPiano: 1, ep1: 4, ep2: 5, harpsichord: 6, celesta: 8, glock: 9, musicBox: 10, vibes: 11,
  marimba: 12, xylophone: 13, bells: 14, dulcimer: 15, organ: 16, accordion: 21,
  nylon: 24, steel: 25, jazzGuitar: 26, cleanGuitar: 27, mutedGuitar: 28,
  acBass: 32, fingerBass: 33, fretless: 35, synthBass1: 38, synthBass2: 39,
  violin: 40, viola: 41, cello: 42, contrabass: 43, tremolo: 44, pizz: 45, harp: 46, timpani: 47,
  strings: 48, strings2: 49, synthStrings: 50, choir: 52, oohs: 53, synthVoice: 54, orchHit: 55,
  trumpet: 56, trombone: 57, tuba: 58, mutedTrumpet: 59, horn: 60, brass: 61, synthBrass: 62,
  oboe: 68, englishHorn: 69, bassoon: 70, clarinet: 71, piccolo: 72, flute: 73, recorder: 74, panFlute: 75,
  whistle: 78, ocarina: 79, square: 80, saw: 81, calliope: 82, chiff: 83, charang: 84, voiceLead: 85, fifths: 86, bassLead: 87,
  newAge: 88, warmPad: 89, polysynth: 90, choirPad: 91, bowedPad: 92, metalPad: 93, haloPad: 94, sweepPad: 95,
  crystal: 98, atmosphere: 99, brightness: 100, sitar: 104, banjo: 105, kalimba: 108, bagpipe: 109, fiddle: 110,
};

/* Drum kits (program on DRUMS). */
export const KIT = { standard: 0, room: 8, power: 16, electronic: 24, tr808: 25, jazz: 32, brush: 40, orchestra: 48 };
