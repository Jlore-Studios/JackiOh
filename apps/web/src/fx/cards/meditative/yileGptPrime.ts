// R3405001: Meditative #49.1 YileGPT Prime (`meditative-049-1`, a token printed Mythic, entrance
// "pavilionUnfurl", bespoke "yilePrime", takeover "pavilion", a hologram orbit on its tile), a Mythic Unit's
// signature in ink and light. After its takeover's hold the colossal scholar unfurls in a wash of ink, a
// hundred glowing brushes spinning into a ring behind it, and lands at the slam under mythic rays. Its Cry's
// brush ring then stops on each pick in turn: on each Claude's Datacenter it summons into its backrow (its
// `summoned`, after the Datacenter's own entrance, R3399000), on each enemy card the board or the viewer's
// hand shows as it dims (`degraded`; one in a Deck or a hidden hand has no anchor, so its row plays), and on
// each card it adds, whose card back flies from it to the hand pile, a sheen there on its owner's seat alone
// (a Radiant card's face is theirs to read). The AI Girlfriend and the Yile's Viruses it summons play their
// own entrances, never this recipe. It keys on its play and the event's player and zone, never on a card's
// definition, and draws card backs alone (R202).

import { animTestid, locateInstance } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, raysCue, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import type { FxAnchor, FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, heroOf, type CardRecipe, type Entrance, type RecipeFixture } from "../recipe.ts";

/** The look of its light: Tech's hologram, its aura's. */
const LIGHT_PALETTE = "Tech/hologram";
/** The look of its ink: Nerd's ink, the wash it unfurls in. */
const INK_PALETTE = "Nerd/ink";

/** Every particle count it throws, at intensity "normal" (rule 9): the unfurling ink, its landing, a pick's light, a card dimming. */
const TUNING = {
  unfurl: { count: 20, power: 0.9 },
  land: { count: 24, power: 1.1 },
  pick: { count: 12, power: 0.8 },
  dim: { count: 8, power: 0.5 },
} as const;

/** The ring spins from the lead to `ringTailMs` past its entry; a stop holds its entry and `stopTailMs` more; an added card's back leaves at `ghostAt` of it. */
const TIMING = { ringTailMs: 350, stopTailMs: 250, ghostAt: 0.25 } as const;

/** The gallery's pick: one Claude's Datacenter into the backrow zone beside the first. */
const FIXTURE_DATACENTER = { instanceId: "fx-datacenter", defId: "classicplus-078", row: "backrow", lane: 2 } as const;

export const pavilionUnfurl: Entrance = ({ D, tgt, intensity: i, view, landing, leadMs }) => {
  const light = paletteLook(LIGHT_PALETTE) ?? FLAVOR_LOOKS.Tech;
  const ink = paletteLook(INK_PALETTE) ?? FLAVOR_LOOKS.Nerd;
  const at = tid(tgt);
  const lead = leadMs ?? 0;
  const slam = lead + frac(FX_SLAM_AT, D - lead);
  return [
    figureCue("brushRing", at, figureTint(light), lead, D - lead + TIMING.ringTailMs, view === undefined || landing === undefined ? {} : { side: sideOf(view, landing.player) }),
    { ...tunedBurst(i, ink.particle, tid(tgt, FOOT), "area", lead, TUNING.unfurl), colors: ink.colors },
    raysCue(D, "mythic", at, slam),
    ringCue(D, light.particle, at, slam),
    { ...tunedBurst(i, light.particle, at, "ring", slam, TUNING.land), colors: light.colors },
    ...shakeCues(i, ENTRANCE_TRAUMA.pavilionUnfurl, slam),
  ];
};

export const yilePrime: CardRecipe = (event, p, play) => {
  // Its Cry is its play's own; it has no hook that draws.
  if (play.hook !== undefined) return null;
  const light = paletteLook(LIGHT_PALETTE) ?? FLAVOR_LOOKS.Tech;
  const tint = figureTint(light);
  const i = p.env.intensity;
  const stop = (at: FxAnchor): FxCue => figureCue("brushRing", at, tint, 0, p.D + TIMING.stopTailMs, { size: "small" });
  switch (event.type) {
    case "summoned": {
      // A Datacenter into its own backrow; its Girlfriend and the Viruses entered by their own entrances already.
      if (event.player !== play.player || event.row !== "backrow") return null;
      const at = tid(testid.zone(sideOf(p.view, event.player), event.row, event.lane));
      return { cues: [stop(at), { ...tunedBurst(i, light.particle, at, "area", 0, TUNING.pick), colors: light.colors }], row: "replace" };
    }
    case "degraded": {
      // An enemy card the view shows dims; one on the board is a stop of the ring.
      const shown = locateInstance(p.view, event.instanceId);
      if (shown === null) return null;
      const at = tid(shown);
      const dim: FxCue = { ...tunedBurst(i, "smoke", at, "area", 0, TUNING.dim), colors: light.colors };
      return { cues: shown === testid.card(event.instanceId) ? [stop(at), dim] : [dim], row: "replace" };
    }
    case "addedToHand": {
      if (event.player !== play.player) return null;
      const side = sideOf(p.view, play.player);
      const card = locateInstance(p.view, play.instanceId);
      const from = card === null ? heroOf(p.view, play) : tid(card);
      const hand = tid(animTestid.hand(side));
      const leave = frac(TIMING.ghostAt, p.D);
      const cues: FxCue[] = [stop(from), { kind: "ghost", from, to: hand, delayMs: leave, durationMs: p.D - leave }];
      // A Radiant card's face is its owner's to read: the sheen plays on their seat alone.
      if (p.view.viewer === play.player) cues.push({ kind: "sheen", at: hand, delayMs: 0, durationMs: p.D });
      return { cues, row: "replace" };
    }
    default:
      return null;
  }
};

/** The gallery's YileGPT Prime: one pick of its Cry, a Claude's Datacenter summoned into its backrow. */
export const yilePrimeFixture: RecipeFixture = (_view, _instanceId, player) => [{ type: "summoned", player, ...FIXTURE_DATACENTER }];
