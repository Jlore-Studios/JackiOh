// R3399000: Classic+ Frozen Wastes (`classicplus-012-6`, entrance none, bespoke "whiteout"), its signature (#3399):
// a Spell printed Legendary, so every burst starts by SPELL_BURST_LATEST_MS of its entry (1,200 ms ceiling);
// it was the shared `vortex` set piece and has no hold now. Cast (its `cardPlayed`): frost creeps in from the
// board's left and right edges and a howling white-out sweeps across (a frost shot edge to edge), every
// Unit's zone rimed. Text: at each `destroyed` the Unit shatters like a frozen pancake (ice shards and crumbs);
// at each `exiled` a card back flies off the top of the caster's deck pile into the snow and is gone, a
// ghost from the library pile to the snow over the board (never the card, R202).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { animTestid, targetFor } from "../../../game/animations.ts";
import { LANES, sideOf, testid } from "../../../game/contract.ts";
import { frac, tid, tunedBurst } from "../../build.ts";
import { FLAVOR_LOOKS } from "../../flavors.ts";
import type { FlavorLook } from "../../flavors.ts";
import { motifBurst, motifScale, motifShot, type CueSpec } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";

/** Every particle count the white-out throws at scale 1 (rule 9). */
const TUNING = {
  creep: { count: 8, power: 0.7 },
  rime: { count: 5, power: 0.5 },
  shatter: { count: 8, power: 1 },
  crumbs: { count: 4, power: 0.7 },
  snow: { count: 5, power: 0.6 },
} as const;

/** The frost's look: ice white and blue, the frost particle. */
const FROST: FlavorLook = { ...FLAVOR_LOOKS.Snack, colors: ["#ffffff", "#bfe6ff", "#5aa9e6"], particle: "frost" };

/** A Spell's bursts start no later than this into its entry, so they end inside its 1,200 ms ceiling (#3399). */
const SPELL_BURST_LATEST_MS = 300;
/** Where in its entry each beat starts: the creep, the sweep's start and end, the shatter and the snow's fall. */
const AT = { creep: 0, sweep: 0.05, sweepEnd: 0.35, shatter: 0.1, exile: 0.05, snow: 0.3, stagger: 0.04 } as const;
/** The viewport's left and right edges, at mid height, where the frost creeps in. */
const EDGE = { left: { x: 0, y: 0.5 }, right: { x: 1, y: 0.5 } } as const;
/** The snow over the board a ghost card flies into. */
const SNOW = { x: 0.5, y: 0.5 } as const;
/** The lane whose zones are rimed on each side. */
const RIME_LANE = Math.ceil(LANES.length / 2);
/** The cards the gallery's Frozen Wastes exiles for each Unit. */
const FIXTURE_EXILE_ID = "fx-snow";

function at(fraction: number, D: number): number {
  return Math.min(frac(fraction, D), SPELL_BURST_LATEST_MS);
}

/** Frozen Wastes: frost and a white-out on its cast, a shatter for each Unit, a card into the snow for each exile. */
export const whiteout: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined || !(p.D > 0)) return null;
  const spec: CueSpec = { look: FROST, scale: motifScale(play.defId, p.view, p.env.intensity), row: "keep" };
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const left = { kind: "viewport", at: EDGE.left } as const;
    const right = { kind: "viewport", at: EDGE.right } as const;
    const start = at(AT.sweep, p.D);
    const cues: FxCue[] = [
      motifBurst(spec, left, "area", at(AT.creep, p.D), TUNING.creep),
      motifBurst(spec, right, "area", at(AT.creep, p.D), TUNING.creep),
      motifShot(spec, "bolt", left, right, start, Math.max(0, at(AT.sweepEnd, p.D) - start)),
    ];
    for (const side of ["you", "opponent"] as const) cues.push(motifBurst(spec, tid(testid.zone(side, "units", RIME_LANE)), "area", at(AT.sweepEnd, p.D), TUNING.rime));
    return { cues, row: "keep" };
  }
  if (event.type === "destroyed") {
    const tgt = targetFor(event, p.view);
    if (tgt === null) return null;
    const unit = tid(tgt);
    return {
      cues: [motifBurst(spec, unit, "area", at(AT.shatter, p.D), TUNING.shatter), tunedBurst(spec.scale, "crumb", unit, "area", at(AT.shatter, p.D), TUNING.crumbs)],
      // The shatter is the Unit's death, drawn in place of its row, as the set piece it replaces drew them (phone budget).
      row: "replace",
    };
  }
  if (event.type === "exiled") {
    const deck = tid(animTestid.library(sideOf(p.view, play.player)));
    const snow = { kind: "viewport", at: SNOW } as const;
    const start = at(AT.exile, p.D);
    return {
      cues: [
        { kind: "ghost", from: deck, to: snow, delayMs: start, durationMs: frac(AT.snow, p.D) },
        motifBurst(spec, snow, "area", at(AT.snow, p.D), TUNING.snow),
      ],
      row: "replace",
    };
  }
  return null;
};

/** The gallery's Frozen Wastes: every Unit on both sides is destroyed, and a card of the caster's deck is exiled for each. */
export function whiteoutFixture(view: PlayerView, _instanceId: string, player: PlayerId): GameEvent[] {
  const units = [...view.you.units, ...view.opponent.units].flatMap((unit) => (unit === null ? [] : [unit]));
  const destroyed: GameEvent[] = units.map((unit) => ({
    type: "destroyed",
    instanceId: unit.instanceId,
    defId: unit.defId,
    owner: unit.owner,
    controller: unit.controller,
    attack: unit.attack,
    maxHealth: unit.maxHealth,
    killerId: null,
  }));
  const exiled: GameEvent[] = units.map((unit, k) => ({ type: "exiled", instanceId: `${FIXTURE_EXILE_ID}-${String(k)}`, defId: unit.defId, owner: player }));
  return [...destroyed, ...exiled];
}
