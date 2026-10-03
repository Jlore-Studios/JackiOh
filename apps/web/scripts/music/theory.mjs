// Music theory for gen-music.mjs (SPEC §10.11 "Music"): scales, diatonic chords, a seeded random
// source and the notes a part is built from. Everything here is deterministic, so a score is the
// same every time it is built and its hash says whether its file is current.

/** Semitones above the tonic, per mode. */
export const SCALES = {
  major: [0, 2, 4, 5, 7, 9, 11],
  minor: [0, 2, 3, 5, 7, 8, 10],
  dorian: [0, 2, 3, 5, 7, 9, 10],
  mixolydian: [0, 2, 4, 5, 7, 9, 10],
  harmonic: [0, 2, 3, 5, 7, 8, 11],
  phrygian: [0, 1, 3, 5, 7, 8, 10],
  lydian: [0, 2, 4, 6, 7, 9, 11],
};

/** MIDI note numbers of the tonics used, octave 4 (C4 = 60). */
export const TONIC = { C: 60, Db: 61, D: 62, Eb: 63, E: 64, F: 65, Gb: 66, G: 67, Ab: 68, A: 69, Bb: 70, B: 71 };

/** A key: its tonic MIDI note and its mode's steps. */
export function key(tonic, mode) {
  const steps = SCALES[mode];
  if (steps === undefined) throw new Error(`unknown mode ${mode}`);
  return { tonic: TONIC[tonic] ?? tonic, steps, mode };
}

/**
 * The MIDI note of a scale degree. Degrees count from 0 (the tonic) and run past 6 into the next
 * octave and below 0 into the one beneath, so 7 is the tonic an octave up and -3 is the fifth below.
 * `octave` shifts by whole octaves from the key's octave 4; `alter` adds semitones (a raised seventh).
 */
export function pitch(k, degree, octave = 0, alter = 0) {
  const n = k.steps.length;
  const oct = Math.floor(degree / n);
  const step = ((degree % n) + n) % n;
  return k.tonic + 12 * (oct + octave) + k.steps[step] + alter;
}

/** The scale degrees of a diatonic chord on `root`: a triad, or with `seventh` its seventh too. */
export function chordDegrees(root, { seventh = false, ninth = false, sus = 0 } = {}) {
  const third = sus === 2 ? root + 1 : sus === 4 ? root + 3 : root + 2;
  const tones = [root, third, root + 4];
  if (seventh) tones.push(root + 6);
  if (ninth) tones.push(root + 8);
  return tones;
}

/** Pitch classes (degree mod 7) of a chord, for "is this a chord tone". */
export function chordClasses(chord) {
  return new Set(chord.map((d) => ((d % 7) + 7) % 7));
}

/** Mulberry32: a small seeded generator. The score's variety comes from here and nowhere else. */
export function rng(seed) {
  let a = seed >>> 0;
  const next = () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
  return {
    next,
    /** An integer in [lo, hi]. */
    int: (lo, hi) => lo + Math.floor(next() * (hi - lo + 1)),
    pick: (list) => list[Math.floor(next() * list.length)],
    chance: (p) => next() < p,
    /** A value in [-amount, amount]. */
    jitter: (amount) => (next() * 2 - 1) * amount,
  };
}

/** A string's 32-bit FNV-1a hash, to seed a track's generator from its id. */
export function seedOf(text) {
  let h = 0x811c9dc5;
  for (let i = 0; i < text.length; i += 1) {
    h ^= text.charCodeAt(i);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return h;
}

/**
 * Voice-leads a chord's pitches into the register around `center`: of every inversion in every nearby octave,
 * the one that moves least from the previous voicing (and stays near the center), so pads and keys
 * move by step rather than jumping with every root. Returns MIDI notes, low to high.
 */
export function voicing(pitches, center, previous) {
  const base = [...pitches].sort((a, b) => a - b);
  let best = base;
  let bestScore = Infinity;
  for (let inversion = 0; inversion < base.length; inversion += 1) {
    const inverted = base.map((note, i) => (i < inversion ? note + 12 : note)).sort((a, b) => a - b);
    for (let shift = -36; shift <= 36; shift += 12) {
      const notes = inverted.map((note) => note + shift);
      const mean = notes.reduce((sum, note) => sum + note, 0) / notes.length;
      let score = Math.abs(mean - center);
      if (previous !== undefined && previous.length === notes.length) {
        score = score * 0.5 + notes.reduce((sum, note, i) => sum + Math.abs(note - previous[i]), 0);
      }
      if (score < bestScore) {
        bestScore = score;
        best = notes;
      }
    }
  }
  return best;
}
