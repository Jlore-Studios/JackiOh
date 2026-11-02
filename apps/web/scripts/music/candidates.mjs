// Audition candidates for the stations' in-game music (SPEC §10.11 "Music", R631). Not shipped:
// `gen-music.mjs --candidates --root <dir>` renders CANDIDATES into a scratch tree for the owner to
// listen to, and the ones kept move into tracks.mjs under station ids.
//
// Round 2. The owner kept these arrangements from round 1 (each its own corner of its station's
// genre, quoting the "Jack-i-Oh" motif, looping past two minutes, #51) and asked for changes to some:
// Bossa's woodwinds softer, Jazzhop without its trumpet, and more moments of tension in every Tavern
// track. CANDIDATES lists those revisions; the rest wait here unchanged until they ship together.
// A revision keeps its round-1 `createSong` id, which seeds its melodies, so only the change is heard.

import {
  DR, DRUMS, GM, KIT, RHYTHMS,
  arp, arrange, at, bass, bend, cc, comp, createSong, drums, every, instrument, line, melody, pad, pulse, roll, roots, swell, wobble,
} from "./compose.mjs";
import { key } from "./theory.mjs";
import { TRACKS } from "./tracks.mjs";

/* ------------------------------------------------------------------------------------------- *
 * Tavern
 * ------------------------------------------------------------------------------------------- */

/** A sea shanty in 6/8 (a beat is an eighth): accordion calls, the crew answers, boots stomp. */
function tavernShanty() {
  const song = createSong({ id: "tavern-shanty", bpm: 240, beatsPerBar: 6, key: key("D", "minor"), humanize: 0.02 });
  song.reverb = { room: 0.5, damp: 0.5, width: 0.8, level: 0.45 };
  instrument(song, 0, GM.tangoAccordion, { volume: 92, pan: 56, reverb: 40 });
  instrument(song, 1, GM.oohs, { volume: 88, pan: 70, reverb: 55 });
  instrument(song, 2, GM.tuba, { volume: 84, pan: 64, reverb: 30 });
  instrument(song, 3, GM.steel, { volume: 82, pan: 36, reverb: 40 });
  instrument(song, 4, GM.fiddle, { volume: 84, pan: 88, reverb: 45 });
  instrument(song, 5, GM.whistle, { volume: 72, pan: 76, reverb: 50 });
  instrument(song, 6, GM.tremolo, { volume: 80, pan: 52, reverb: 60 });
  instrument(song, DRUMS, KIT.room, { volume: 92, reverb: 35 });
  const A = ["i", "i", "VII", "VII", "i", "i", ["iv", "V+"], "i"];
  const B = ["III", "VII", "i", "i", "III", "VII", ["iv", "V+"], "i"];
  // Round 2, "more moments of tension": a storm that sits on the dominant before the last chorus.
  const STORM = ["i", "i", "VI", "V+", "i", "iv", "V+", "V+"];
  const STOP = 7 * 6 + 3;
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
    tremolo: (s) => {
      pad(s, { ch: 6, center: 57, vel: 62, every: 6 });
      swell(s, { ch: 6, lo: 40 });
    },
    // The storm: tremolo strings swell over a driving tuba, the fiddle saws, the crew holds its
    // chords, the stomps gather into a roll and everything stops on one hit before the chorus.
    storm: (s) => {
      pad(s, { ch: 6, center: 57, vel: 70, every: 6 });
      swell(s, { ch: 6, lo: 45 });
      cc(s, 6, STOP + 0.5, 11, 0);
      cc(s, 6, s.beats - 0.01, 11, 127);
      pad(s, { ch: 1, center: 55, vel: 56 });
      pulse(s, { ch: 2, step: 1, octave: -2, vel: 82, accent: [1, 0.6, 0.6, 0.85, 0.6, 0.6], until: STOP });
      arp(s, { ch: 4, step: 0.5, pattern: [0, 1, 0, 1, 2, 1], center: 62, vel: 58, gate: 0.7, until: STOP });
      drums(s, { pattern: [[0, DR.kick2, 96], [0, DR.floorTom, 70], [3, DR.kick2, 80], [3, DR.floorTom, 62], [1.5, DR.clap, 50], [4.5, DR.clap, 50]], skip: (bar) => bar >= 6 });
      roll(s, 6, { n: DR.floorTom, step: 0.5, v0: 50, v1: 86 });
      roll(s, 7, { n: DR.floorTom, to: 3, step: 0.25, v0: 70, v1: 110 });
      drums(s, { pattern: [[3, DR.crash, 96], [3, DR.kick2, 110]], skip: (bar) => bar !== 7 });
      comp(s, { ch: 3, hits: [[3, 1, 1]], center: 57, vel: 96, from: STOP, until: STOP + 1 });
      comp(s, { ch: 0, hits: [[3, 1, 1]], center: 62, vel: 92, from: STOP, until: STOP + 1 });
    },
  };
  return arrange(song, parts, [
    [8, A, ["strum", "lightStomp", "callAndReply"]],
    [16, A, ["strum", "tuba", "stomp", "verse", "crew"]],
    [16, A, ["strum", "tuba", "stomp", "callAndReply", "jig"]],
    [8, B, ["strum", "fiddle", "crew", "tremolo"]],
    [16, B, ["strum", "tuba", "stomp", "verse", "whistle"]],
    [8, STORM, ["storm"]],
    [16, A, ["strum", "tuba", "stomp", "callAndReply", "jig"]],
    [8, A, ["strum", "tuba", "lightStomp", "fiddle"]],
  ]);
}

/** A desert caravanserai: sitar, shanai and an oud-like guitar over darbuka, in E phrygian dominant. */
function tavernCaravan() {
  const song = createSong({ id: "tavern-caravan", bpm: 104, key: key("E", "phrygianDominant"), humanize: 0.015 });
  song.reverb = { room: 0.65, damp: 0.4, width: 0.9, level: 0.5 };
  instrument(song, 0, GM.sitar, { volume: 86, pan: 60, reverb: 50 });
  instrument(song, 1, GM.shanai, { volume: 76, pan: 80, reverb: 55 });
  instrument(song, 2, GM.nylon, { volume: 90, pan: 42, reverb: 40 });
  instrument(song, 3, GM.fretless, { volume: 88, pan: 64, reverb: 25 });
  instrument(song, 4, GM.strings, { volume: 70, pan: 64, reverb: 70 });
  instrument(song, 5, GM.dulcimer, { volume: 76, pan: 92, reverb: 55 });
  instrument(song, 6, GM.cello, { volume: 70, pan: 50, reverb: 55 });
  instrument(song, 7, GM.tremolo, { volume: 78, pan: 72, reverb: 65 });
  instrument(song, DRUMS, KIT.standard, { volume: 88, reverb: 30 });
  const A = ["I", "I", "II", "I", "iv", "II", "vii", ["II", "I"]];
  const B = ["iv", "iv", "I", "I", "vii", "vii", "II", ["II", "I"]];
  // Round 2, "more moments of tension": the flat second grinds over a held tonic before the return.
  const PEDAL = ["II", "II", "I", "II", "vii", "II", "vii", "II"];
  const STOP = 7 * 4 + 2;
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
    tremolo: (s) => {
      pad(s, { ch: 7, center: 59, vel: 56 });
      swell(s, { ch: 7, lo: 40 });
    },
    // The pedal: the cello holds the tonic under the flat second, tremolo strings swell, the shanai
    // wails, and the darbuka speeds up from maqsum to eighths to a roll, stopping on two hits.
    pedal: (s, i) => {
      for (let bar = 0; bar < s.bars; bar += 1) line(s, { ch: 6, notes: [[0, 4]], octave: -2, vel: 66, at: bar * 4, legato: 1 });
      pad(s, { ch: 7, center: 60, vel: 66 });
      swell(s, { ch: 7, lo: 45 });
      cc(s, 7, STOP + 1.5, 11, 0);
      cc(s, 7, s.beats - 0.01, 11, 127);
      melody(s, { ch: 1, rhythms: RHYTHMS.epic, range: [5, 12], vel: 80, motifBars: [0, 4], start: 8, seed: 30 + i });
      arp(s, { ch: 2, step: 0.25, pattern: [0, 1, 0, 2], center: 52, vel: 56, gate: 0.5, until: STOP });
      drums(s, { pattern: maqsum, skip: (bar) => bar >= 4 });
      drums(s, { pattern: [...at(DR.congaHi, 66, every(0.5)), [0, DR.congaLo, 90], [2, DR.congaLo, 84]], skip: (bar) => bar < 4 || bar >= 6 });
      roll(s, 6, { n: DR.congaHi, step: 0.25, v0: 56, v1: 90 });
      roll(s, 7, { n: DR.congaHi, to: 2, step: 0.125, v0: 80, v1: 110 });
      drums(s, { pattern: [[2, DR.congaLo, 112], [2, DR.crash, 84], [3, DR.congaLo, 112], [3, DR.tomLo, 96]], skip: (bar) => bar !== 7 });
      comp(s, { ch: 4, hits: [[2, 0.5, 1], [3, 0.8, 1]], center: 60, vel: 96, from: STOP, until: STOP + 2 });
    },
  };
  return arrange(song, parts, [
    [8, A, ["drone", "oud"]],
    [16, A, ["drone", "oud", "bass", "darbuka", "sitar"]],
    [8, B, ["bass", "darbuka", "strings", "shanai", "tremolo"]],
    [8, A, ["drone", "qanun", "bass", "darbuka", "oudLead"]],
    [8, B, ["bass", "darbuka", "strings", "sitar"]],
    [8, PEDAL, ["pedal"]],
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

/** Jazzhop: acoustic piano voicings with ninths, a walking upright and tenor sax. */
function lofiJazzhop() {
  const song = createSong({ id: "lofi-jazzhop", bpm: 88, key: key("Eb", "major"), swing: 0.12, humanize: 0.02 });
  song.reverb = { room: 0.5, damp: 0.55, width: 0.75, level: 0.4 };
  song.post.lofi = lofiPost({ lowpassHz: 5200, crackle: 0.06, clicksPerS: 4, seed: 7103 });
  instrument(song, 0, GM.piano, { volume: 90, pan: 56, reverb: 45 });
  // The owner asked for no trumpet: channel 1 keeps its melodies (their seeds name the channel) on
  // a tenor sax an octave down, and the piano takes the counter-line where two saxes would meet.
  instrument(song, 1, GM.tenorSax, { volume: 82, pan: 70, reverb: 50 });
  instrument(song, 2, GM.acBass, { volume: 90, pan: 64, reverb: 20 });
  instrument(song, 3, GM.tenorSax, { volume: 80, pan: 44, reverb: 50 });
  instrument(song, 4, GM.strings2, { volume: 54, pan: 64, reverb: 70 });
  instrument(song, DRUMS, KIT.jazz, { volume: 88, reverb: 25 });
  const A = ["ii79", "V79", "I79", "vi79", "ii79", "V79", "iii7", "vi7"];
  const B = ["IV79", "vii7", "iii79", "vi79", "ii79", "V79", "I79", ["ii7", "V7"]];
  const beat = [...at(DR.kick, 80, [0, 2.5]), [1.75, DR.kick, 50], ...at(DR.stick, 62, [1, 3]), ...at(DR.ride, 40, every(0.5)), [1, DR.pedal, 34], [3, DR.pedal, 34]];
  const parts = {
    piano: (s) => comp(s, { ch: 0, hits: [[0, 1.4, 1], [1.5, 0.4, 0.6], [2.5, 1.4, 0.85]], center: 60, vel: 58 }),
    walk: (s) => bass(s, { ch: 2, pattern: [[0, "R", 0.9, 1], [1, "3", 0.9, 0.8], [2, "5", 0.9, 0.9], [3, "A", 0.9, 0.8]], vel: 84 }),
    drums: (s) => drums(s, { pattern: beat }),
    lead: (s, i) => melody(s, { ch: 1, rhythms: RHYTHMS.lofi, range: [2, 10], octave: -1, vel: 74, motifBars: [0, 4], start: 4, seed: i }),
    sax: (s, i) => melody(s, { ch: 3, rhythms: RHYTHMS.lofi, range: [-1, 7], vel: 72, motifBars: [0], start: 2, seed: 10 + i }),
    pianoSolo: (s, i) => melody(s, { ch: 0, rhythms: RHYTHMS.edm, range: [7, 14], vel: 62, motifBars: [0], start: 9, seed: 20 + i }),
    strings: (s) => pad(s, { ch: 4, center: 62, vel: 36 }),
  };
  return arrange(song, parts, [
    [4, A, ["piano", "walk", "strings"]],
    [8, A, ["piano", "walk", "drums"]],
    [8, A, ["piano", "walk", "drums", "lead"]],
    [8, B, ["piano", "walk", "drums", "sax", "strings"]],
    [8, A, ["piano", "walk", "drums", "lead", "pianoSolo"]],
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
  // The owner asked for gentler woodwinds: both sit lower in level and velocity, with more room
  // and longer notes, and the flute stays out of its piercing top register.
  instrument(song, 1, GM.flute, { volume: 70, pan: 74, reverb: 68 });
  instrument(song, 2, GM.acBass, { volume: 86, pan: 64, reverb: 20 });
  instrument(song, 3, GM.clarinet, { volume: 68, pan: 58, reverb: 62 });
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
    flute: (s, i) => melody(s, { ch: 1, rhythms: RHYTHMS.lofi, range: [2, 9], vel: 58, motifBars: [0, 4], start: 6, seed: i, legato: 0.97 }),
    clarinet: (s, i) => melody(s, { ch: 3, rhythms: RHYTHMS.lofi, range: [0, 8], vel: 56, motifBars: [0], start: 4, seed: 10 + i, legato: 0.97 }),
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
  instrument(song, 0, GM.percOrgan, { volume: 84, pan: 56, reverb: 30 });
  instrument(song, 1, GM.jazzGuitar, { volume: 82, pan: 34, reverb: 40 });
  instrument(song, 2, GM.fingerBass, { volume: 92, pan: 64, reverb: 10 });
  instrument(song, 3, GM.strings, { volume: 66, pan: 72, reverb: 40 });
  instrument(song, 4, GM.altoSax, { volume: 80, pan: 76, reverb: 45 });
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

/** A dark requiem in C phrygian: choir-led, church organ, low brass, tolling bells. */
function epicRequiem() {
  const song = createSong({ id: "epic-requiem", bpm: 72, key: key("C", "phrygian") });
  song.reverb = { room: 0.9, damp: 0.25, width: 1, level: 0.7 };
  instrument(song, 0, GM.choir, { volume: 92, pan: 60, reverb: 80 });
  instrument(song, 1, GM.churchOrgan, { volume: 70, pan: 64, reverb: 75 });
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
  instrument(song, 3, GM.taiko, { volume: 92, pan: 64, reverb: 55 });
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

/* ------------------------------------------------------------------------------------------- *
 * The list
 * ------------------------------------------------------------------------------------------- */

/** `title` and `blurb` are for the audition page; gen-music reads `id`, `loop` and `build`. */
export const CANDIDATES = [
  { id: "tavern-1-r2", station: "tavern", title: "Tavern 1, with tension", before: "tavern-1", blurb: "Two tense moments added: a build in the parallel minor with tremolo strings and a tom roll into a stop, and a darker swell before the middle.", loop: true, build: () => TRACKS.find((t) => t.id === "tavern-1").build() },
  { id: "tavern-shanty-r2", station: "tavern", title: "Sea Shanty, with a storm", before: "tavern-shanty", blurb: "A storm before the last chorus: tremolo strings, held choir, a driving tuba and a rising stomp roll into a stop.", loop: true, build: tavernShanty },
  { id: "tavern-caravan-r2", station: "tavern", title: "Caravanserai, with tension", before: "tavern-caravan", blurb: "A pedal section on the flat second over tremolo strings, an accelerating darbuka roll and stop hits, before the return.", loop: true, build: tavernCaravan },
  { id: "lofi-bossa-r2", station: "lofi", title: "Bossa Tape, softer woodwinds", before: "lofi-bossa", blurb: "Flute and clarinet quieter and rounder: lower velocity and level, the flute out of its piercing top register, more room.", loop: true, build: lofiBossa },
  { id: "lofi-jazzhop-r2", station: "lofi", title: "Jazzhop, no trumpet", before: "lofi-jazzhop", blurb: "The muted trumpet is gone: the tenor sax takes its melodies and the piano takes the sax's turns.", loop: true, build: lofiJazzhop },
];

/** Kept from round 1 unchanged: these ship with the round-2 picks, into tracks.mjs under station ids. */
export const KEPT = { edmDnb, edmSynthwave, edmHalftime, edmTrap, lofiBoombap, lofiRain, lofiVapor, epicFiveFour, epicRequiem };
