// R3405001: Meditative #75 Ever Growing Tree (`meditative-075`, entrance "treeSprout", bespoke
// "everGrowingTree", a bloom down its lane), a Legendary Field Spell's signature. As it lands a seed drops
// into its zone and splits, a sapling corkscrews up into a broad tree, and at the slam its roots grip the
// zone's edges in a ring of leaves. Its Activate makes the tree glow (`activated`); the Discover that
// follows is its owner's to read, so its offer shows only as a sheen over the tree on their seat; and the
// Field Spell chosen sinks into its roots, bark closing over it as a new branch sprouts (`fused`). The
// branch wears the tree's own green: the field's colour would key on the chosen card's definition, which
// no recipe reads. It keys on its play or hook and the events naming its own card, never on a card's
// definition, and names no card (R202); its beats key on the event, not on the hook (the gallery plays
// them under its play, a game under its Activate's).

import { sideOf } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { ENTRANCE_TRAUMA, FOOT, type CardRecipe, type Entrance, type RecipeFixture } from "../recipe.ts";
import { fieldTile } from "./introspection.ts";

/** The look its leaves and bark wear: Nature's green. */
const TREE_PALETTE = "Nature";

/** Its own definition, which the gallery's Activate and fusion name. */
const TREE_ID = "meditative-075";

/** Its Activate's id in its script. */
const GROW_ABILITY = "grow";

/** Every particle count it throws, at intensity "normal" (rule 9). */
const TUNING = {
  seed: { count: 8, power: 0.4 },
  roots: { count: 20, power: 1 },
  glow: { count: 12, power: 0.5 },
} as const;

/**
 * The sapling lasts from its lead to its entry's end and `treeTailMs` more, its seed splitting `splitAt`
 * of the way to the slam; a fusion's branch lasts its entry and `branchTailMs` more.
 */
const TIMING = { treeTailMs: 300, splitAt: 0.3, branchTailMs: 200 } as const;

export const treeSprout: Entrance = ({ D, tgt, intensity: i, view, landing, leadMs }) => {
  const look = paletteLook(TREE_PALETTE) ?? FLAVOR_LOOKS.Nature;
  const at = tid(tgt);
  const lead = leadMs ?? 0;
  const slam = lead + frac(FX_SLAM_AT, D - lead);
  const split = lead + frac(TIMING.splitAt, slam - lead);
  const side = view === undefined || landing === undefined ? {} : { side: sideOf(view, landing.player) };
  return [
    figureCue("sapling", at, figureTint(look), lead, D - lead + TIMING.treeTailMs, side),
    { ...tunedBurst(i, look.particle, tid(tgt, FOOT), "point", split, TUNING.seed), colors: look.colors },
    ringCue(D, look.particle, at, slam),
    { ...tunedBurst(i, look.particle, tid(tgt, FOOT), "ring", slam, TUNING.roots), colors: look.colors },
    ...shakeCues(i, ENTRANCE_TRAUMA.treeSprout, slam),
  ];
};

export const everGrowingTree: CardRecipe = (event, p, play) => {
  const look = paletteLook(TREE_PALETTE) ?? FLAVOR_LOOKS.Nature;
  switch (event.type) {
    case "activated": {
      if (event.instanceId !== play.instanceId) return null;
      const tile = fieldTile(p.view, play);
      return { cues: [ringCue(p.D, look.particle, tile, 0), { ...tunedBurst(p.env.intensity, look.particle, tile, "area", 0, TUNING.glow), colors: look.colors }], row: "replace" };
    }
    case "promptOpened":
      // The Discover's offer is its owner's to read: a sheen over the tree on their seat, nothing on the other.
      if (event.player !== play.player || p.view.viewer !== play.player) return null;
      return { cues: [{ kind: "sheen", at: fieldTile(p.view, play), delayMs: 0, durationMs: p.D }], row: "replace" };
    case "fused": {
      if (event.resultInstanceId !== play.instanceId) return null;
      const tile = fieldTile(p.view, play);
      return { cues: [figureCue("branch", tile, figureTint(look), 0, p.D + TIMING.branchTailMs, { side: sideOf(p.view, play.player) }), ringCue(p.D, look.particle, tile, 0)], row: "replace" };
    }
    default:
      return null;
  }
};

/** Its Activate: the tree glows, its owner's Discover opens, and the Field Spell chosen fuses into it. */
export const everGrowingTreeFixture: RecipeFixture = (_view, instanceId, player) => [
  { type: "activated", player, instanceId, defId: TREE_ID, ability: GROW_ABILITY },
  { type: "promptOpened", player },
  { type: "fused", instanceIds: [instanceId, "fx-fused-field"], resultInstanceId: instanceId, defId: TREE_ID },
];
