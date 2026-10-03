// A Standard MIDI File writer for gen-music.mjs: one format-0 track, every channel on it. A score
// is a list of plain events timed in beats; this turns them into the bytes FluidSynth renders.

export const PPQ = 480;
/** The General MIDI percussion channel (0-based). */
export const DRUMS = 9;

function varLen(value) {
  let v = Math.max(0, Math.round(value));
  const bytes = [v & 0x7f];
  v >>= 7;
  while (v > 0) {
    bytes.unshift((v & 0x7f) | 0x80);
    v >>= 7;
  }
  return bytes;
}

const clamp7 = (v) => Math.max(0, Math.min(127, Math.round(v)));

/**
 * `score`: { bpm, beatsPerBar, channels: { [ch]: { program, volume?, pan?, reverb?, bank? } },
 * notes: [{ t, dur, ch, note, vel }], ccs?: [{ t, ch, cc, value }], bends?: [{ t, ch, value }],
 * endBeat } with every time in beats. Returns the file's bytes.
 */
export function writeMidi(score) {
  const raw = [];
  const tick = (beats) => Math.max(0, Math.round(beats * PPQ));
  // Order at one tick: setup (0), controllers (1), note-offs (2), note-ons (3).
  const usPerBeat = Math.round(60_000_000 / score.bpm);
  raw.push({ at: 0, order: 0, bytes: [0xff, 0x51, 0x03, (usPerBeat >> 16) & 0xff, (usPerBeat >> 8) & 0xff, usPerBeat & 0xff] });
  raw.push({ at: 0, order: 0, bytes: [0xff, 0x58, 0x04, score.beatsPerBar, 2, 24, 8] });
  for (const [chText, setup] of Object.entries(score.channels)) {
    const ch = Number(chText);
    if (setup.bank !== undefined) raw.push({ at: 0, order: 0, bytes: [0xb0 | ch, 0, setup.bank] });
    raw.push({ at: 0, order: 0, bytes: [0xc0 | ch, setup.program] });
    raw.push({ at: 0, order: 0, bytes: [0xb0 | ch, 7, clamp7(setup.volume ?? 100)] });
    raw.push({ at: 0, order: 0, bytes: [0xb0 | ch, 10, clamp7(setup.pan ?? 64)] });
    raw.push({ at: 0, order: 0, bytes: [0xb0 | ch, 91, clamp7(setup.reverb ?? 40)] });
    raw.push({ at: 0, order: 0, bytes: [0xb0 | ch, 93, 0] });
    raw.push({ at: 0, order: 0, bytes: [0xb0 | ch, 11, 127] });
  }
  for (const c of score.ccs ?? []) raw.push({ at: tick(c.t), order: 1, bytes: [0xb0 | c.ch, c.cc, clamp7(c.value)] });
  for (const b of score.bends ?? []) {
    const v = Math.max(0, Math.min(16383, Math.round(b.value + 8192)));
    raw.push({ at: tick(b.t), order: 1, bytes: [0xe0 | b.ch, v & 0x7f, (v >> 7) & 0x7f] });
  }
  for (const n of score.notes) {
    const on = tick(n.t);
    const off = Math.max(on + 1, tick(n.t + n.dur));
    const note = Math.max(0, Math.min(127, Math.round(n.note)));
    raw.push({ at: on, order: 3, bytes: [0x90 | n.ch, note, Math.max(1, clamp7(n.vel))] });
    raw.push({ at: off, order: 2, bytes: [0x80 | n.ch, note, 0] });
  }
  raw.sort((a, b) => a.at - b.at || a.order - b.order);
  const end = Math.max(tick(score.endBeat), raw.length > 0 ? raw[raw.length - 1].at : 0);
  const body = [];
  let last = 0;
  for (const e of raw) {
    body.push(...varLen(e.at - last), ...e.bytes);
    last = e.at;
  }
  body.push(...varLen(end - last), 0xff, 0x2f, 0x00);
  const header = [0x4d, 0x54, 0x68, 0x64, 0, 0, 0, 6, 0, 0, 0, 1, (PPQ >> 8) & 0xff, PPQ & 0xff];
  const len = body.length;
  const track = [0x4d, 0x54, 0x72, 0x6b, (len >>> 24) & 0xff, (len >>> 16) & 0xff, (len >>> 8) & 0xff, len & 0xff];
  return Uint8Array.from([...header, ...track, ...body]);
}
