// R3338000, S #154 Dice of Doom (`diceOfDoom`): its d100 (two d80s, Radiant) tumbles across the board,
// slowing, and lands on its `rolled` result halfway through the span (R2652001's faces, public to both
// seats). On a 67 the board goes dark, a star lights for each Terminus card it adds, and they rain into
// its caster's hand as card backs once the hold ends; the opponent sees how many, never which (R202).
// On a miss the die crumbles. A die that rolled more than once (Lucky, an Embiggened Dice's for good)
// keeps a Lucky glint. The jackpot is read off what the run did (the cards it added or burned), never
// off the face, so the client restates no card's number. It draws its run in place of their own rows.

import { animTestid } from "../../game/animations.ts";
import { sideOf } from "../../game/contract.ts";
import { frac, ringCue, tid, tunedBurst } from "../build.ts";
import { FX_CENTER, FX_CRACK_TAIL_MS, FX_TERMINUS_RAIN_STEP } from "../constants.ts";
import { rollBatchOf, rollCues } from "../rolls.ts";
import { beatAt, doomHit } from "../setPieces.ts";
import type { FxAnchor, FxCue } from "../types.ts";
import { BOARD, TERMINUS_TUNING, eventsOf, momentPiece, type TerminusPlanner } from "./build.ts";

/** The roll panel's place, where the die lands (rolls.ts's PANEL). */
const PANEL: FxAnchor = { kind: "viewport", at: { x: FX_CENTER.x, y: FX_CENTER.y } };

export const doom: TerminusPlanner = (m) => {
  const land = frac(TERMINUS_TUNING.doomLandAt, m.S);
  const rolled = m.entry.events.find((event) => event.type === "rolled");
  const cues: FxCue[] = [];
  const rain = eventsOf(m.entry, ["addedToHand", "burned"]);
  const hit = doomHit(m.entry);
  // A jackpot's stars light one by one between the landing and the end of the span.
  const marks = hit ? rain.map((_, k) => land + beatAt(k, rain.length, m.S - land)) : [];
  cues.push(momentPiece(m, BOARD, marks));
  if (rolled !== undefined) cues.push(...rollCues(rolled, m.D, m.i, { landMs: land, look: "doom" }));
  if (hit) {
    // They rain into the hand over the motion after the hold (unheld, after the span), FX_TERMINUS_RAIN_STEP apart.
    const after = m.H > 0 ? m.M : m.D - m.S;
    const fall = Math.max(1, frac(TERMINUS_TUNING.rainFall, after));
    const hand = tid(animTestid.hand(sideOf(m.view, m.signature.player)));
    rain.forEach((_, k) => {
      const at = m.S + frac(k * FX_TERMINUS_RAIN_STEP, after);
      cues.push({ kind: "ghost", from: BOARD, to: hand, delayMs: at, durationMs: fall });
    });
    cues.push({ ...tunedBurst(m.i, "star", hand, "area", m.S + fall, TERMINUS_TUNING.rain), colors: m.look.colors });
  } else {
    // A miss: the die crumbles where it landed.
    const at = frac(TERMINUS_TUNING.crumbleAt, m.S);
    cues.push(
      { kind: "crack", at: PANEL, delayMs: at, durationMs: m.D - at + FX_CRACK_TAIL_MS },
      { ...tunedBurst(m.i, "dust", PANEL, "area", at, TERMINUS_TUNING.crumble), colors: m.look.colors },
      tunedBurst(m.i, "shard", PANEL, "point", at, TERMINUS_TUNING.crumble),
    );
  }
  // A die Lucky rolled again keeps its glint.
  const batch = rolled === undefined ? null : rollBatchOf(rolled);
  if (batch !== null && batch.rolls.some((roll) => roll.faces.length > 1)) {
    const at = frac(TERMINUS_TUNING.glintAt, m.S);
    cues.push(tunedBurst(m.i, "sparkle", PANEL, "area", at, TERMINUS_TUNING.glint), ringCue(m.D, "gold", PANEL, at));
  }
  return { cues, replaces: new Set(m.entry.events) };
};
