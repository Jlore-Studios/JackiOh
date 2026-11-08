// R3400001: the signature sounds of Meditative+'s and Training Grounds' Legendaries and Mythics, beside the
// sound table's own (cues.ts `cuesFor` appends them after the event's row, as it does the Battleship's): a
// Spell's as its play is read (`cardPlayed`), a Unit's as it lands face up in a unit zone (`summoned`), the
// moment its signature's own cast or entrance starts. A row reads only the `defId` the event itself carries,
// so a hidden play (R97's sentinel), a card set face down (its `formerId`, R227) and every other card sound
// nothing more (R203).

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
