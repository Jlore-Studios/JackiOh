// R3402001: ten Core Legendaries and Mythics sound a signature of their own (issue #3402), beside the sound
// table's own (cues.ts `cuesFor` appends it after the event's row, unpanned, as it does the Battleship's): a
// Unit's or a Spell's on its `cardPlayed`, a Trap's on its `trapFired`, once, on its first beat. A row reads
// the event alone (CLAUDE.md rule 7): R97's sentinel names no card, so a card the viewer cannot read never
// sounds one (R203), and a face-down set, a Trap's play or a Unit's (R227's `formerId`), sounds as a set.

import type { GameEvent } from "@jackioh/shared";

import { HIDDEN_DEF_ID } from "./constants.ts";
import type { CueContext } from "./cues.ts";
import type { SfxId, SoundCue } from "./types.ts";

/** A card's signature: its sound, and the one event it sounds on. */
export type SignatureSfx = { id: SfxId; on: "cardPlayed" | "trapFired" };

/** R3402001: the ten cards' signatures, by catalog id. A Trap's is its firing; every other card's, its play. */
export const SIGNATURE_SFX: Readonly<Record<string, SignatureSfx>> = {
  "core-052": { id: "silasScratch", on: "cardPlayed" }, // Silly Silas
  "core-083": { id: "crucibleChoir", on: "cardPlayed" }, // Transmogulate
  "core-085": { id: "hatchWeld", on: "trapFired" }, // Unlicensed Experimentation
  "core-087": { id: "pocketZip", on: "cardPlayed" }, // Pocket Chaos
  "core-093": { id: "stadiumCheer", on: "cardPlayed" }, // Combo-Index
  "core-095": { id: "jackpotReels", on: "cardPlayed" }, // Call to Chaos (Core Edition)
  "core-096": { id: "pawnWaltz", on: "trapFired" }, // My Pawn
  "core-097": { id: "wishWind", on: "cardPlayed" }, // Zephrys
  "core-098": { id: "heroicFanfare", on: "cardPlayed" }, // Heroic Power
  "core-099": { id: "forgeAnvil", on: "cardPlayed" }, // Craft a Card
};

const NONE: readonly SoundCue[] = [];

/** The card's signature, once, when `event` is its own beat; none for any other card or event. */
export function signatureCues(event: GameEvent, _ctx: CueContext): readonly SoundCue[] {
  if (event.type !== "cardPlayed" && event.type !== "trapFired") return NONE;
  if (event.defId === HIDDEN_DEF_ID) return NONE;
  const signature = SIGNATURE_SFX[event.defId];
  if (signature === undefined || signature.on !== event.type) return NONE;
  // A card set face down (R227) is a set, and a face-up Field Trap firing again reveals nothing (R2657001).
  if (event.type === "cardPlayed" && event.formerId !== undefined) return NONE;
  if (event.type === "trapFired" && event.again === true) return NONE;
  return [{ kind: "sfx", id: signature.id, delayMs: 0 }];
}
