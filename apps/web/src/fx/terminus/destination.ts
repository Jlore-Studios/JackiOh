// R3338000, S #4.5 1v1 Final Destination (`finalDestination`): one lane of light stays open over the
// abyss, centred on its caster's side of it; every other lane chains shut and drops into the dark, one
// beat a lane as its `locked` events come; both hero health counters spin down to what the trap set them
// to, and the Spells each hand discards burn off its edge. Its firing plays its own reveal; the rest of
// its run plays here in place of their own rows. It reads only public events: the lanes, the health and
// how many cards each hand discarded (R202).

import { animTestid } from "../../game/animations.ts";
import { sideOf, testid, type Side } from "../../game/contract.ts";
import { frac, tid, tunedBurst } from "../build.ts";
import { FX_CRACK_TAIL_MS, FX_MOTIF_CENTER_LANE, FX_ZONE_TAIL_MS } from "../constants.ts";
import { beatAt, tintOf } from "../setPieces.ts";
import type { FxCue } from "../types.ts";
import { TERMINUS_TUNING, eventsOf, momentPiece, type TerminusPlanner } from "./build.ts";

/** The lanes of the board, 1 on (§3.1). */
const LANES = [1, 2, 3, 4, 5] as const;

export const destination: TerminusPlanner = (m) => {
  const locked = eventsOf(m.entry, ["locked"]);
  const shut = LANES.filter((lane) => locked.some((event) => event.lane === lane));
  const open = LANES.find((lane) => !shut.includes(lane)) ?? FX_MOTIF_CENTER_LANE;
  const caster = sideOf(m.view, m.signature.player);
  const marks = shut.map((_, k) => beatAt(k, shut.length, m.S));
  const cues: FxCue[] = [momentPiece(m, tid(testid.zone(caster, "units", open)), marks)];
  // Each lane chains shut at its beat: a crack across both sides' units zones, and frost on the caster's.
  shut.forEach((lane, k) => {
    const at = marks[k] ?? 0;
    for (const side of ["you", "opponent"] as const) {
      cues.push({ kind: "crack", at: tid(testid.zone(side, "units", lane)), delayMs: at, durationMs: m.D - at + FX_CRACK_TAIL_MS });
    }
    cues.push({ ...tunedBurst(m.i, "frost", tid(testid.zone(caster, "units", lane)), "area", at, TERMINUS_TUNING.chain), colors: m.look.colors });
  });
  // Both heroes' health spins down to what it was set to.
  const healthAt = frac(TERMINUS_TUNING.healthAt, m.S);
  for (const set of eventsOf(m.entry, ["healthSet"])) {
    cues.push({
      kind: "zone",
      at: tid(testid.hero(sideOf(m.view, set.player))),
      tint: tintOf(m.look),
      text: String(set.health),
      direction: "down",
      delayMs: healthAt,
      durationMs: m.D - healthAt + FX_ZONE_TAIL_MS,
    });
  }
  // The discarded Spells burn off each hand's edge: one burst a hand, as big as its count (to a cap).
  const burnAt = frac(TERMINUS_TUNING.burnAt, m.S);
  const discards = new Map<Side, number>();
  for (const card of eventsOf(m.entry, ["discarded"])) {
    const side = sideOf(m.view, card.owner);
    discards.set(side, (discards.get(side) ?? 0) + 1);
  }
  for (const [side, n] of discards) {
    const tuning = { count: TERMINUS_TUNING.burn.count * Math.min(n, TERMINUS_TUNING.burnMax), power: TERMINUS_TUNING.burn.power };
    cues.push({ ...tunedBurst(m.i, "ember", tid(animTestid.hand(side)), "area", burnAt, tuning), colors: m.look.colors });
  }
  return { cues, replaces: new Set(m.entry.events.filter((event) => event.type !== "trapFired")) };
};
