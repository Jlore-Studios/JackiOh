// R3400001: the signature sounds of Meditative+'s and Training Grounds' Legendaries and Mythics, beside the
// sound table's own (cues.ts `cuesFor` appends them after the event's row, as it does the Battleship's): a
// Spell's as its play is read (`cardPlayed`), a Unit's as it lands face up in a unit zone (`summoned`), the
// moment its signature's own cast or entrance starts. A row reads only the `defId` the event itself carries,
// so a hidden play (R97's sentinel), a card set face down (its `formerId`, R227) and every other card sound
// nothing more (R203). R3395000: Stinkyscape 001 to 090's Legendaries' and Mythics' signatures join them, a
// Field Spell's on its play as a Spell's is.

import type { GameEvent } from "@jackioh/shared";

import { HIDDEN_DEF_ID } from "./constants.ts";
import type { SfxId, SoundCue } from "./types.ts";

/** Each signature card's own sound, and the event it sounds on: a Spell's play or a Unit's landing. */
export const SIGNATURE_SFX: Readonly<Record<string, { sfx: SfxId; on: "cardPlayed" | "summoned" }>> = {
  // Bob's Plan: a brush slapped on paper, a war-drum hit, a satisfied grunt.
  "meditativeplus-001": { sfx: "inkSummit", on: "cardPlayed" },
  // Bob, Wisconsin's Strongest Soldier: a snowplow revving, a grunt of effort, a deep thud.
  "meditativeplus-024": { sfx: "plowHeave", on: "summoned" },
  // Scratch Pad: a cursor tick, keys clattering into silence, paper tearing.
  "meditativeplus-026": { sfx: "cursorTear", on: "cardPlayed" },
  // Seance: a murmured chant, candles guttering, a ghostly exhale.
  "traininggrounds-037": { sfx: "seanceChant", on: "cardPlayed" },
  // Tutorial Master: a pointer rapped on slate, a weary sigh, a chalk squeak.
  "traininggrounds-038": { sfx: "slateRap", on: "summoned" },
  // Call to Chaos (Training-Grounds Edition): dice clattering, a held breath, a gong.
  "traininggrounds-039": { sfx: "diceGong", on: "cardPlayed" },
  // The Training-Grounds Golem: timber creaking, ropes snapping taut, a hollow thump.
  "traininggrounds-039-1": { sfx: "timberLash", on: "summoned" },
  // Goetzmann Game Design: a pencil scratching, a napkin rustle, a rising arpeggio.
  "traininggrounds-040": { sfx: "pencilArpeggio", on: "cardPlayed" },
  // R3395000. Tylannt: vault tumblers clicking, then a heavy door's hiss.
  "stinkyscape-006": { sfx: "vaultHiss", on: "summoned" },
  // The Greatest Heist: safe-dial ticks and a soft alarm chirp.
  "stinkyscape-007": { sfx: "safeChirp", on: "cardPlayed" },
  // Rod of Ages: a deep stone hum, a chime for each band.
  "stinkyscape-016": { sfx: "stoneChime", on: "cardPlayed" },
  // Imitated Instrumentality: three rising boot tones, then a cold electronic chord.
  "stinkyscape-018": { sfx: "magiChord", on: "cardPlayed" },
  // ZolarKeth: a lecture-hall bell, then a rocket's distant rumble.
  "stinkyscape-026": { sfx: "lectureRumble", on: "summoned" },
  // Drone Core VIII: a factory press stamping faster and faster.
  "stinkyscape-034": { sfx: "pressStamp", on: "cardPlayed" },
  // Drone Core IX: three deep engine roars, one after another.
  "stinkyscape-035": { sfx: "corvetteRoar", on: "cardPlayed" },
  // Kwzka: a crowd's gasp and a bright pop-chorus sting.
  "stinkyscape-037": { sfx: "popGasp", on: "summoned" },
  // The Narcor Project: glass clinking, a Geiger crackle, a low hum.
  "stinkyscape-039": { sfx: "geigerHum", on: "cardPlayed" },
  // Great Paperclip Theory: a soft metallic twang, then a rising, endless ratchet.
  "stinkyscape-044": { sfx: "wireRatchet", on: "cardPlayed" },
  // Beam v5: a rising whine, a deep beam hum, a cooling tick.
  "stinkyscape-058": { sfx: "beamWhine", on: "summoned" },
  // Repeater v5: rapid rattling bursts rising in pitch, a steam hiss.
  "stinkyscape-059": { sfx: "rattleSteam", on: "summoned" },
  // Cannon v5: a breech clank and one thunderous boom.
  "stinkyscape-060": { sfx: "breechBoom", on: "summoned" },
  // Flak v5: three rolls of flak, then silence and a thin wind.
  "stinkyscape-061": { sfx: "flakWind", on: "summoned" },
  // Miner v5: a grinding crunch, then a bright jackpot chime.
  "stinkyscape-066": { sfx: "crunchJackpot", on: "summoned" },
  // Z'Zoth: a low choir hum and creaking hulls.
  "stinkyscape-070": { sfx: "choirHulls", on: "summoned" },
  // A'Zoth: a brassy horn blast echoing back in layers.
  "stinkyscape-071": { sfx: "hornEcho", on: "summoned" },
  // Paragon Mk2: a jet scream and a shield's glassy snap.
  "stinkyscape-073": { sfx: "jetSnap", on: "summoned" },
  // Paragon-X: a white-noise roar, then a clean high ring.
  "stinkyscape-074": { sfx: "whiteRing", on: "summoned" },
  // Galaxy: a deep hull groan, two drills whining up.
  "stinkyscape-080": { sfx: "hullDrills", on: "summoned" },
  // Prototype-X1: a test-range klaxon and a sputtering engine catching.
  "stinkyscape-085": { sfx: "klaxonSputter", on: "summoned" },
  // Prototype-X2: a molten roar, shrieking metal, a furious engine snarl.
  "stinkyscape-086": { sfx: "moltenSnarl", on: "summoned" },
  // Siva 1: camera shutters and a smug purr.
  "stinkyscape-087": { sfx: "shutterPurr", on: "summoned" },
  // Siva 2: a dark reboot sting and a hissing drip.
  "stinkyscape-088": { sfx: "venomDrip", on: "summoned" },
};

const NONE: readonly SoundCue[] = [];

/** R3400001: the signature sound `event` starts, or none: another event, another card, or a card the viewer cannot read. */
export function signatureCues(event: GameEvent): readonly SoundCue[] {
  if (event.type !== "cardPlayed" && event.type !== "summoned") return NONE;
  if (event.defId === HIDDEN_DEF_ID || event.formerId !== undefined) return NONE;
  if (event.type === "summoned" && event.row !== "units") return NONE;
  const own = Object.prototype.hasOwnProperty.call(SIGNATURE_SFX, event.defId) ? SIGNATURE_SFX[event.defId] : undefined;
  if (own === undefined || own.on !== event.type) return NONE;
  return [{ kind: "sfx", id: own.sfx, delayMs: 0 }];
}
