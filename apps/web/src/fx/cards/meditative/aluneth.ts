// R3405001: Meditative #80 Aluneth (`meditative-080`, entrance "lanternLight", bespoke "aluneth", frost
// on its zone), a Legendary Field Spell's signature. As it lands an iron lantern drops into its zone among
// stacked books and lights at the slam with an ice-blue flame no wind bends, frost creeping out under the
// books. At its end of turn each card it draws tears off the stack as an ink bird, flapping off its zone,
// and lands in its owner's hand pile as a card back; as each grows cheaper (`costChanged`) the hand pile
// frosts with a sheen, on its owner's seat alone (the costs are theirs to read). It keys on its play or
// hook and the drawer, never on a card's definition, and draws card backs alone (R202); its beats key on
// the event, not on the hook (the gallery plays them under its play, a game under its hook's marker).

import { animTestid } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import type { FxCue, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type CardRecipe, type Entrance, type RecipeFixture } from "../recipe.ts";
import { fieldTile } from "./introspection.ts";

/** The look its flame and frost wear: Spirit's ice blue. */
const FLAME_PALETTE = "Spirit";

/** The look its books and ink birds wear: Nerd's ink, its aura's. */
const INK_PALETTE = "Nerd/ink";

/** Every particle count it throws, at intensity "normal" (rule 9). */
const TUNING = {
  light: { count: 10, power: 0.5 },
  frost: { count: 20, power: 0.7 },
  tear: { count: 8, power: 0.6 },
  rime: { count: 8, power: 0.4 },
} as const;

/** The lantern lasts from its lead to its entry's end and `lanternTailMs` more; a drawn card's back leaves the bird `flyAt` into its entry. */
const TIMING = { lanternTailMs: 300, flyAt: 0.25 } as const;

/** Where its flame burns in its zone, as fx.css draws the lantern. */
const FLAME: FxPoint = { x: 0.5, y: 0.48 };

/** The gallery's draw: one card into its owner's hand, made cheaper. */
const FIXTURE_DRAW = { instanceId: "fx-book", defId: "core-002", cost: 0 } as const;

export const lanternLight: Entrance = ({ D, tgt, intensity: i, leadMs }) => {
  const flame = paletteLook(FLAME_PALETTE) ?? FLAVOR_LOOKS.Spirit;
  const at = tid(tgt);
  const lead = leadMs ?? 0;
  const slam = lead + frac(FX_SLAM_AT, D - lead);
  // Upright on both seats: no mirror for the other side.
  return [
    figureCue("lantern", at, figureTint(flame), lead, D - lead + TIMING.lanternTailMs),
    { ...tunedBurst(i, flame.particle, tid(tgt, FLAME), "point", slam, TUNING.light), colors: flame.colors },
    ringCue(D, "frost", at, slam),
    { ...tunedBurst(i, "frost", tid(tgt, FOOT), "ring", slam, TUNING.frost), colors: flame.colors },
    ...shakeCues(i, ENTRANCE_TRAUMA.lanternLight, slam),
  ];
};

export const aluneth: CardRecipe = (event, p, play) => {
  const hand = tid(animTestid.hand(sideOf(p.view, play.player)));
  switch (event.type) {
    case "drawn": {
      if (event.player !== play.player) return null;
      const ink = paletteLook(INK_PALETTE) ?? FLAVOR_LOOKS.Nerd;
      const tile = fieldTile(p.view, play);
      const fly = frac(TIMING.flyAt, p.D);
      const cues: FxCue[] = [
        figureCue("inkBird", tile, figureTint(ink), 0, p.D, { side: sideOf(p.view, play.player) }),
        { ...tunedBurst(p.env.intensity, ink.particle, tile, "point", 0, TUNING.tear), colors: ink.colors },
        { kind: "ghost", from: tile, to: hand, delayMs: fly, durationMs: p.D - fly },
      ];
      return { cues, row: "replace" };
    }
    case "costChanged": {
      // Its gem frosts cheaper on its owner's seat alone.
      if (p.view.viewer !== play.player) return { cues: [], row: "replace" };
      const flame = paletteLook(FLAME_PALETTE) ?? FLAVOR_LOOKS.Spirit;
      return { cues: [{ kind: "sheen", at: hand, delayMs: 0, durationMs: p.D }, { ...tunedBurst(p.env.intensity, "frost", hand, "area", 0, TUNING.rime), colors: flame.colors }], row: "replace" };
    }
    default:
      return null;
  }
};

/** One of its end of turn's draws: the card to its owner's hand, then its price falling. */
export const alunethFixture: RecipeFixture = (_view, _instanceId, player) => [
  { type: "drawn", player, instanceId: FIXTURE_DRAW.instanceId, defId: FIXTURE_DRAW.defId },
  { type: "costChanged", instanceId: FIXTURE_DRAW.instanceId, cost: FIXTURE_DRAW.cost },
];
