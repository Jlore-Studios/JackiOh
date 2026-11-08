// R3405001: Meditative #97.8 Prison (`meditative-097-8`, a token printed Legendary, entrance "prisonBars",
// bespoke "prison", shadow drifting on its tile), a Legendary Unit's signature. Iron bars drop around its
// zone from above and clang down at the slam, a stone cell rising beneath them in a cloud of dust while a
// guard tower's lamp sweeps on. After an enemy Unit is played, its capture (`controlChanged`, a steal into
// its own zone) swings the lamp's beam onto that Unit, chains lock around it and drag it into the cell
// beneath Prison, dust settling there. A missed roll emits no event, so the lamp has nothing to swing onto.
// The Unit is found where the board shows it, else where it landed (its `summoned`, among the events the
// planner remembers), else at its old controller's hero. It keys on its own play or trigger and the
// event's type, never on a card's definition, and names no card (R202).

import type { GameEvent, PlayerId } from "@jackioh/shared";

import { locateInstance } from "../../../game/animations.ts";
import { sideOf, testid, type Side } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { otherSide } from "../../motifs/build.ts";
import type { FxAnchor, FxCue, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type CardFxPlan, type CardRecipe, type Entrance, type RecipeFixture } from "../recipe.ts";

/** The look its iron and stone wear: Dark's shadow sub-look, its aura's. */
const PRISON_PALETTE = "Dark/shadow";

/** The lane the gallery lands a Unit in (motifs/gallery.ts `GALLERY_ENTRANCE_LANE`), where its fixture's capture goes. */
const GALLERY_PRISON_LANE = 3;

/** Where the bars clang: the top of its zone. */
const CLANG_AT: FxPoint = { x: 0.5, y: 0.05 };

/** Every particle count its signature throws, at intensity "normal" (rule 9). */
const TUNING = {
  clang: { count: 10, power: 0.9 },
  cell: { count: 20, power: 0.9 },
  settle: { count: 12, power: 0.7 },
} as const;

/**
 * The bars last the entry and `tailMs` more. At a capture, as fractions of the entry: the lamp's beam
 * reaches the Unit at `lampEnd`, the chains drag it from `dragAt` into the cell by `cellAt`; the chains
 * last until `chainTailMs` past the entry.
 */
const TIMING = { tailMs: 300, lampEnd: 0.25, dragAt: 0.3, cellAt: 0.6, chainTailMs: 200 } as const;

export const prisonBars: Entrance = ({ D, tgt, intensity: i, view, landing, leadMs }) => {
  const look = paletteLook(PRISON_PALETTE) ?? FLAVOR_LOOKS.Dark;
  const at = tid(tgt);
  const lead = leadMs ?? 0;
  const slam = lead + frac(FX_SLAM_AT, D - lead);
  const side = view === undefined || landing === undefined ? {} : { side: sideOf(view, landing.player) };
  const cues: FxCue[] = [
    figureCue("prisonBars", at, figureTint(look), lead, D - lead + TIMING.tailMs, side),
    { ...tunedBurst(i, "spark", tid(tgt, CLANG_AT), "point", slam, TUNING.clang), colors: look.colors },
    { ...tunedBurst(i, look.particle, tid(tgt, FOOT), "area", slam, TUNING.cell), colors: look.colors },
    ringCue(D, look.particle, at, slam),
    ...shakeCues(i, ENTRANCE_TRAUMA.prisonBars, slam),
  ];
  return cues;
};

/** Where the captured Unit stood: on the board, else where it landed (its remembered `summoned`), else at `foe`'s hero. */
function capturedAt(instanceId: string, p: CardFxPlan, foe: Side): FxAnchor {
  const shown = locateInstance(p.view, instanceId);
  if (shown !== null) return tid(shown);
  const landed = [...p.env.memory.recent()].reverse().find((event) => event.type === "summoned" && event.instanceId === instanceId);
  return landed?.type === "summoned" ? tid(testid.zone(sideOf(p.view, landed.player), landed.row, landed.lane)) : tid(testid.hero(foe));
}

export const prison: CardRecipe = (event, p, play) => {
  // Its capture, into its own zone (a `trigger` entry; its play's in the gallery).
  if (event.type !== "controlChanged" || event.controller !== play.player) return null;
  if (play.hook !== undefined && play.hook !== "trigger") return null;
  const look = paletteLook(PRISON_PALETTE) ?? FLAVOR_LOOKS.Dark;
  const i = p.env.intensity;
  const caster = sideOf(p.view, play.player);
  const cellZone = testid.zone(caster, event.row, event.lane);
  const cell = tid(cellZone);
  const from = capturedAt(event.instanceId, p, otherSide(caster));
  const lamp = frac(TIMING.lampEnd, p.D);
  const drag = frac(TIMING.dragAt, p.D);
  const shut = frac(TIMING.cellAt, p.D);
  return {
    cues: [
      { kind: "shot", shot: { motion: "beam" }, colors: look.colors, preset: look.particle, from: cell, to: from, delayMs: 0, flightMs: lamp, density: i },
      figureCue("chains", from, figureTint(look), lamp, p.D - lamp + TIMING.chainTailMs, { side: caster }),
      { kind: "shot", shot: look.projectile, colors: look.colors, preset: look.particle, from, to: cell, delayMs: drag, flightMs: shut - drag, density: i },
      { ...tunedBurst(i, look.particle, tid(cellZone, FOOT), "area", shut, TUNING.settle), colors: look.colors },
    ],
    row: "replace",
  };
};

/** R3405001: its capture of the other player's first Unit into the zone the gallery lands it in. */
export const prisonFixture: RecipeFixture = (view, _instanceId, player: PlayerId): GameEvent[] => {
  const foe = (player === view.viewer ? view.opponent : view.you).units.find((unit) => unit !== null);
  return foe === undefined || foe === null ? [] : [{ type: "controlChanged", instanceId: foe.instanceId, controller: player, row: "units", lane: GALLERY_PRISON_LANE, how: "steal" }];
};
