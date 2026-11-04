// R2784000: every drawn motif by name, and the one way to plan one. Nothing plays with the effects off
// or under reduced motion (intensity 0, or an entry of no duration).

import type { GameEvent, PlayerView } from "@jackioh/shared";

import type { AnimationEntry } from "../../game/animations.ts";
import type { FxCue, FxPlanEnv } from "../types.ts";
import { bind, conjure, hex, mirror, offering, rise, shift, tether, transmute } from "./board.ts";
import { readEvents, type MotifPlanner, type MotifSpec } from "./build.ts";
import { barrage, bolt, chain, lance, nova, scorch, volley } from "./damage.ts";
import { deal, flight, glimpse, mana, recall, seal, send, tally, tithe } from "./hand.ts";
import { bless, curse, drain, mend, radiance, ward } from "./life.ts";
import type { DrawnMotif } from "./names.ts";
import { negate, vanish, wipe } from "./removal.ts";
import { snap } from "./trap.ts";

/** One planner per drawn motif; a missing one fails `typecheck`. */
export const MOTIF_PLANNERS: { readonly [M in DrawnMotif]: MotifPlanner } = {
  bolt,
  volley,
  barrage,
  chain,
  lance,
  nova,
  wipe,
  vanish,
  scorch,
  negate,
  drain,
  bless,
  curse,
  ward,
  conjure,
  rise,
  offering,
  shift,
  bind,
  tether,
  mirror,
  transmute,
  flight,
  glimpse,
  mana,
  tithe,
  tally,
  snap,
  send,
  radiance,
  mend,
  deal,
  seal,
  recall,
  hex,
};

export function planMotif(spec: MotifSpec, entry: AnimationEntry, view: PlayerView, env: FxPlanEnv): FxCue[] {
  if (!(env.intensity > 0) || !(entry.durationMs > 0) || !(spec.scale > 0)) return [];
  return MOTIF_PLANNERS[spec.motif].plan(spec, entry, view, env);
}

/** The entry's events the motif draws over in place of their own row recipe: none when its row is `keep`. */
export function motifReplaces(spec: MotifSpec, entry: AnimationEntry, view: PlayerView, env: FxPlanEnv): ReadonlySet<GameEvent> {
  const planner = MOTIF_PLANNERS[spec.motif];
  if (spec.row === "keep") return new Set();
  return new Set(readEvents(entry, view, env, planner.reads).flatMap((h) => (planner.replaces.includes(h.event.type) ? [h.event] : [])));
}
