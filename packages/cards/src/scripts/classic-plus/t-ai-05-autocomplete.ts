// T-AI-5 Autocomplete (SPEC §8.7 row T-AI-5, §7, B8). (0) Spell, AI, Token.
//   Base:    "Add a copy of the last Unit, Spell or Field Spell your opponent played to your hand."
//   Radiant: "… It costs (0)."
//   Engine:  "Reads the last face-up card the opponent played (the per-game counts, §10.1: a record each
//            play overwrites and nothing clears) and adds a copy of its definition and face. Traps and
//            Field Traps are set face-down, so they never count and nothing hidden is copied; the record
//            never takes an AI generated card (the AI tag, a fused one included), so two Autocompletes
//            can't feed each other forever. Casts count (R70); a countered card was never played.
//            Nothing played yet: nothing. Tunes: none."
//
// The record is E4's (`lastFaceUpPlayed`, R451), which already passes over Traps and the AI tag
// (`LAST_FACE_UP_SKIPPED_TAGS`), so this card reads it and adds the copy through §6.3's Add to hand: a
// new card of yours, the hand cap burning it (§2.4), its face the one played (R57).

import { opponentOf } from "@jackioh/shared";
import type { Effect, EffectContext, Script } from "@jackioh/engine";
import { lastFaceUpPlayed } from "@jackioh/engine";
import { addToHand } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-t-ai-05");

/** §8.7 Radiant: "It costs (0)." */
const RADIANT_COST = 0;

function autocomplete(ctx: EffectContext, costOverride?: number): Effect[] {
  const last = lastFaceUpPlayed(ctx.state, opponentOf(ctx.controller));
  if (last === null) return [];
  return [addToHand({ defId: last.defId, radiant: last.radiant, ...(costOverride === undefined ? {} : { costOverride }) })];
}

export const base: Script = { cry: (ctx) => autocomplete(ctx) };

export const radiant: Script = { cry: (ctx) => autocomplete(ctx, RADIANT_COST) };
