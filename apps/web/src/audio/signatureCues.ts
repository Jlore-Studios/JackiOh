// R3400001: the signature sounds of Meditative+'s and Training Grounds' Legendaries and Mythics, beside the
// sound table's own (cues.ts `cuesFor` appends them after the event's row, as it does the Battleship's): a
// Spell's as its play is read (`cardPlayed`), a Unit's as it lands face up in a unit zone (`summoned`), the
// moment its signature's own cast or entrance starts. A row reads only the `defId` the event itself carries,
// so a hidden play (R97's sentinel), a card set face down (its `formerId`, R227) and every other card sound
// nothing more (R203).
// R3407001: Stinkyscape's 23 signature cards joined, one sound each, so that every card with a signature of
// its own also sounds its own; a Field Spell lands face up in the backrow with a `summoned` of its own, the
// moment its signature starts, so it sounds there (never on its `cardPlayed`, which would sound it twice),
// under the same R97 and R227 guards.

import type { GameEvent, Row } from "@jackioh/shared";

import { HIDDEN_DEF_ID } from "./constants.ts";
import type { SfxId, SoundCue } from "./types.ts";

/** A signature card's own sound and its moment: a Spell's play, or a Unit's or Field Spell's landing in its row. */
export type SignatureSound =
  | { readonly sfx: SfxId; readonly on: "cardPlayed" }
  | { readonly sfx: SfxId; readonly on: "summoned"; readonly row: Row };

/** Each signature card's own sound, and the event it sounds on. */
export const SIGNATURE_SFX: Readonly<Record<string, SignatureSound>> = {
  // Bob's Plan: a brush slapped on paper, a war-drum hit, a satisfied grunt.
  "meditativeplus-001": { sfx: "inkSummit", on: "cardPlayed" },
  // Bob, Wisconsin's Strongest Soldier: a snowplow revving, a grunt of effort, a deep thud.
  "meditativeplus-024": { sfx: "plowHeave", on: "summoned", row: "units" },
  // Scratch Pad: a cursor tick, keys clattering into silence, paper tearing.
  "meditativeplus-026": { sfx: "cursorTear", on: "cardPlayed" },
  // Seance: a murmured chant, candles guttering, a ghostly exhale.
  "traininggrounds-037": { sfx: "seanceChant", on: "cardPlayed" },
  // Tutorial Master: a pointer rapped on slate, a weary sigh, a chalk squeak.
  "traininggrounds-038": { sfx: "slateRap", on: "summoned", row: "units" },
  // Call to Chaos (Training-Grounds Edition): dice clattering, a held breath, a gong.
  "traininggrounds-039": { sfx: "diceGong", on: "cardPlayed" },
  // The Training-Grounds Golem: timber creaking, ropes snapping taut, a hollow thump.
  "traininggrounds-039-1": { sfx: "timberLash", on: "summoned", row: "units" },
  // Goetzmann Game Design: a pencil scratching, a napkin rustle, a rising arpeggio.
  "traininggrounds-040": { sfx: "pencilArpeggio", on: "cardPlayed" },
  // R3407001: Stinkyscape.
  // MELCHIOR: a sterile beep count, a ballot clack, a rising tone.
  "stinkyscape-018-1": { sfx: "ballotBeep", on: "summoned", row: "backrow" },
  // BALTHASAR: a soft monitor blip, a lullaby hum, a worried sigh.
  "stinkyscape-018-2": { sfx: "lullabySigh", on: "summoned", row: "backrow" },
  // CASPER: a compact snapping shut, a sultry chime, a tut.
  "stinkyscape-018-3": { sfx: "compactTut", on: "summoned", row: "backrow" },
  // xxFreshJack: a glitch stutter, a crowd roar, a bass drop.
  "stinkyscape-109": { sfx: "glitchDrop", on: "summoned", row: "units" },
  // iKwzki: a harp glissando, a warm two-note chime.
  "stinkyscape-110": { sfx: "harpGliss", on: "summoned", row: "units" },
  // Shockwave447: fast-forwarded chatter, a marker squeak, an electric thunderclap.
  "stinkyscape-111": { sfx: "markerThunder", on: "summoned", row: "units" },
  // Job Application: a pneumatic thunk, a desk-phone trill, a rubber stamp.
  "stinkyscape-112": { sfx: "stampTrill", on: "summoned", row: "backrow" },
  // Paycheck: a paper feed, a roulette rattle, a till drawer.
  "stinkyscape-112-1": { sfx: "tillRattle", on: "cardPlayed" },
  // Scornboi: a gavel crack, a scoff, a heavy hatch slam.
  "stinkyscape-115": { sfx: "gavelSlam", on: "summoned", row: "units" },
  // Megabase: a deep docking clank, a lobby chime, a coin cascade.
  "stinkyscape-116": { sfx: "dockingChime", on: "summoned", row: "units" },
  // IN33DH3: a keyboard flurry, a chirpy AI ping, a smug click.
  "stinkyscape-147": { sfx: "keyboardPing", on: "summoned", row: "units" },
  // IN33DH3 Prime: a low synth swell, a confident snap, a drum tap.
  "stinkyscape-147-1": { sfx: "synthSnap", on: "summoned", row: "units" },
  // Wardrum Blob: a squishy plop, a two-beat hand drum, a happy hum.
  "stinkyscape-147-2": { sfx: "blobDrum", on: "summoned", row: "backrow" },
  // Alteran Reach: a warp whoosh, a sparkling chime per node.
  "stinkyscape-148": { sfx: "warpSparkle", on: "summoned", row: "backrow" },
  // Edge of the Universe: paper tearing, a long held choir note, then silence.
  "stinkyscape-148-1": { sfx: "tearChoir", on: "cardPlayed" },
  // Wildar: a cackle, a crack of thunder, two coin flicks.
  "stinkyscape-149": { sfx: "cackleThunder", on: "cardPlayed" },
  // Call to Chaos (Stinky Edition): a wet squelch, a spinning ratchet, a kazoo fanfare.
  "stinkyscape-150": { sfx: "kazooRatchet", on: "cardPlayed" },
  // Stinkyscape Golem: a grinding rock thud, a thruster roar, a loud sniff.
  "stinkyscape-150-1": { sfx: "thrusterSniff", on: "summoned", row: "units" },
  // Call to Chaos (Scape Edition): a deep tilting groan, rolling marbles, a cosmic gong.
  "stinkyscape-151": { sfx: "marbleGong", on: "cardPlayed" },
  // Sanctum: a hushed choir breath, a page turn, a soft bell.
  "stinkyscape-152": { sfx: "sanctumBell", on: "summoned", row: "backrow" },
  // Stimmycheck: a cash-drawer ping, then a slowly rising whistle.
  "stinkyscape-153": { sfx: "cashWhistle", on: "cardPlayed" },
  // Narcor: a whispered breath, a shimmer, a soft clink.
  "stinkyscape-t-narcor": { sfx: "whisperClink", on: "cardPlayed" },
  // Red Narcor: a deep resonant hum rising to a bright crystal ring.
  "stinkyscape-t-red-narcor": { sfx: "crystalRise", on: "cardPlayed" },
};

const NONE: readonly SoundCue[] = [];

/** R3400001, R3407001: the signature sound `event` starts, or none: another event or row, another card, or a card the viewer cannot read. */
export function signatureCues(event: GameEvent): readonly SoundCue[] {
  if (event.type !== "cardPlayed" && event.type !== "summoned") return NONE;
  if (event.defId === HIDDEN_DEF_ID || event.formerId !== undefined) return NONE;
  const own = Object.prototype.hasOwnProperty.call(SIGNATURE_SFX, event.defId) ? SIGNATURE_SFX[event.defId] : undefined;
  if (own === undefined || own.on !== event.type) return NONE;
  if (own.on === "summoned" && event.type === "summoned" && own.row !== event.row) return NONE;
  return [{ kind: "sfx", id: own.sfx, delayMs: 0 }];
}
