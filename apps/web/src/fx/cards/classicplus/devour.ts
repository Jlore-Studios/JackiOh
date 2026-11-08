// R3399000: Classic+ Devour (`classicplus-012-1`, entrance none, bespoke "pancakeMouth"), its signature (#3399):
// a Spell printed Legendary, so every burst starts by SPELL_BURST_LATEST_MS of its entry (1,200 ms ceiling).
// Cast (its `cardPlayed`): a huge pancake mouth rises over the board, a syrup ring opening wide and crumbs
// and syrup spilling from its grin, the board's light tipping toward it as rays. Text, at its `destroyed`:
// the mouth snaps shut on the Unit (a closing ring, a crunch of crumbs, a small shake) and chews; at its
// `damage`: the caster's hero clutches its belly as its Health drains away in red gulps, one per point
// swallowed, at most MAX_GULPS. Moved: the mouth rises over the middle of the board, not over the target,
// since `cardPlayed` names no target; the snap is on the target. R202: it draws only the public events'
// own ids, zones and heroes, never a card's id.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { targetFor } from "../../../game/animations.ts";
import { frac, raysCue, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FLAVOR_LOOKS } from "../../flavors.ts";
import { hitAnchor, motifBurst, motifScale, type CueSpec } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";

/** Every particle count the mouth throws at scale 1 (rule 9). */
const TUNING = {
  grin: { count: 14, power: 0.8 },
  chew: { count: 16, power: 1 },
  gulp: { count: 6, power: 0.6 },
} as const;

/** A Spell's bursts start no later than this into its entry, so they end inside its 1,200 ms ceiling (#3399). */
const SPELL_BURST_LATEST_MS = 300;
/** The mouth's centre, over the middle of the board (a viewport fraction). */
const MOUTH_AT = { x: 0.5, y: 0.45 } as const;
/** Where in its entry each beat starts. */
const AT = { open: 0, rays: 0.05, snap: 0.05, chew: 0.25, gulpFirst: 0.05, gulpStep: 0.1 } as const;
/** A gulp for each point swallowed, at most this many. */
const MAX_GULPS = 5;
/** The belly clutch's shake, a Spell's at most 1 (#3399). */
const SNAP_TRAUMA = 0.5;

function at(fraction: number, D: number): number {
  return Math.min(frac(fraction, D), SPELL_BURST_LATEST_MS);
}

/** Devour: the mouth opens on its cast, snaps on the Unit it destroys and drains its caster's hero. */
export const pancakeMouth: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined || !(p.D > 0)) return null;
  const scale = motifScale(play.defId, p.view, p.env.intensity);
  const spec: CueSpec = { look: FLAVOR_LOOKS.Snack, scale, row: "keep" };
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const mouth = { kind: "viewport", at: MOUTH_AT } as const;
    return {
      cues: [
        ringCue(p.D, "crumb", mouth, at(AT.open, p.D)),
        motifBurst(spec, mouth, "area", at(AT.open, p.D), TUNING.grin),
        raysCue(p.D, "legendary", mouth, at(AT.rays, p.D)),
      ],
      row: "keep",
    };
  }
  if (event.type === "destroyed") {
    const tgt = targetFor(event, p.view);
    if (tgt === null) return null;
    const unit = tid(tgt);
    return {
      cues: [
        ringCue(p.D, "crumb", unit, at(AT.snap, p.D)),
        motifBurst(spec, unit, "area", at(AT.chew, p.D), TUNING.chew),
        ...shakeCues(scale, SNAP_TRAUMA, at(AT.snap, p.D)),
      ],
      row: "keep",
    };
  }
  if (event.type === "damage" && !event.combat) {
    const hero = hitAnchor(p.view, event.targetId);
    if (hero === null) return null;
    const gulps = Math.min(MAX_GULPS, Math.max(1, event.amount));
    const cues: FxCue[] = Array.from({ length: gulps }, (_, k) =>
      tunedBurst(scale, "blood", hero, "area", at(AT.gulpFirst + k * AT.gulpStep, p.D), TUNING.gulp),
    );
    return { cues, row: "keep" };
  }
  return null;
};

/** The gallery's Devour: the viewer's opponent's first Unit is destroyed, and the caster's hero takes its Health. */
export function pancakeMouthFixture(view: PlayerView, instanceId: string, player: PlayerId): GameEvent[] {
  const enemy = player === view.viewer ? view.opponent : view.you;
  const unit = enemy.units.find((candidate) => candidate !== null);
  if (unit === undefined || unit === null) return [];
  return [
    { type: "destroyed", instanceId: unit.instanceId, defId: unit.defId, owner: unit.owner, controller: unit.controller, attack: unit.attack, maxHealth: unit.maxHealth, killerId: null },
    { type: "damage", sourceId: instanceId, targetId: `hero-${player}`, amount: unit.maxHealth, combat: false },
  ];
}
