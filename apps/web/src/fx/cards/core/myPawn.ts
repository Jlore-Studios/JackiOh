// R3402001: #96 My Pawn (`core-096`, bespoke "pawnStrings", takeover "pawn"), a Mythic Trap's signature
// (R3332000). Face down it draws nothing (R202); its firing is its takeover (takeover.ts), the screen
// turning indigo as a vast hooded hand reaches over a glass chessboard (its art, takeoverArt.ts). As the
// card lands, puppet strings snap taut from the top of the screen to every enemy Unit the board shows and
// to the enemy hero, at most FX_PAWN_STRINGS_MAX, and the enemy hero takes a sheen; on its
// `attackCancelled` the lethal attack freezes mid-swing and shatters into silver shards at the attacker.
// The strings "for the rest of the turn" stay a beat of its firing: a recipe draws only while its card
// resolves (R3402001's open question). It anchors on zones, heroes and readable cards and names none.

import { leadMs, locateInstance } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_CRACK_TAIL_MS, FX_PAWN_STRINGS_MAX, FX_PAWN_TRAUMA, FX_SLAM_AT } from "../../constants.ts";
import { otherSide } from "../../motifs/build.ts";
import type { FxAnchor, FxCue, FxPoint, FxShotCue } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";
import { isFirstBeat } from "./flourish.ts";

/** Every particle count the strings and the shattered attack throw, at intensity "normal" (rule 9). */
const TUNING = {
  shards: { count: 24, power: 1.2 },
} as const;

/** Where the hand's strings hang from: the top of the screen. */
const STRINGS_FROM: FxPoint = { x: 0.5, y: 0 };

/** The strings' silver. */
const STRING_COLORS = ["#ffffff", "#e5e7eb", "#9ca3af"] as const;

export const pawnStrings: CardRecipe = (event, p, play) => {
  if (play.hook !== "cast") return null;
  const i = p.env.intensity;
  if (isFirstBeat(event, play) && event.type === "trapFired") {
    const enemy = otherSide(sideOf(p.view, event.controller));
    // As the takeover lands its card (takeover.ts `entranceLandMs`): after the hold, at FX_SLAM_AT of the motion.
    const start = leadMs(p.entry);
    const land = start + frac(FX_SLAM_AT, p.D - start);
    const hero = tid(testid.hero(enemy));
    const zones = p.view[enemy].units.flatMap((unit, k) => (unit === null ? [] : [tid(testid.zone(enemy, "units", k + 1))]));
    const targets = [...zones, hero].slice(-FX_PAWN_STRINGS_MAX);
    const strings = targets.map(
      (to): FxShotCue => ({
        kind: "shot",
        shot: { motion: "bolt", trail: "frost" },
        colors: STRING_COLORS,
        preset: "frost",
        from: { kind: "viewport", at: STRINGS_FROM },
        to,
        delayMs: start,
        flightMs: land - start,
        density: i,
      }),
    );
    return { cues: [...strings, ringCue(p.D, "frost", hero, land), { kind: "sheen", at: hero, delayMs: land, durationMs: p.D - land }], row: "keep" };
  }
  if (event.type !== "attackCancelled") return null;
  const where = locateInstance(p.view, event.attackerId);
  const at: FxAnchor = where === null ? tid(testid.board) : tid(where);
  const cues: FxCue[] = [
    { kind: "crack", at, delayMs: 0, durationMs: p.D + FX_CRACK_TAIL_MS },
    ringCue(p.D, "frost", at, 0),
    tunedBurst(i, "shard", at, "area", 0, TUNING.shards),
    ...shakeCues(i, FX_PAWN_TRAUMA, 0),
  ];
  return { cues, row: "keep" };
};
