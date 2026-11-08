// R3405001: Meditative #49.3 AI Girlfriend (`meditative-049-3`, a token printed Mythic, entrance "dojoBoot",
// bespoke "aiGirlfriend", takeover "dojo", a holy hologram glow on her tile), a Mythic Unit's signature in
// Tech's hologram. After her takeover's hold a wireframe dojo boots up over her zone in a flicker of light
// and a calm hologram martial artist bows in it, stepping into her zone at the slam in a ring of light. Each
// blow she parries leaves its attacker dimmed (`degraded`): a hex shield flashes at her palm, on her card
// while the board shows it (else on the attacker), and the attacker dims. In a game her start of turn's
// forced attacks each open with an `attackDeclared`, which ends her hook's entry (memory.ts), and her
// after-attacked hook opens none, so the parry plays where her play's entry runs it (the gallery), and in a
// game her beckon and the attacks play their own rows. It keys on her play and the event's card as the view
// shows it, never on a card's definition, and names no card (R202).

import { locateInstance } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { ENTRANCE_TRAUMA, FOOT, type CardRecipe, type Entrance, type RecipeFixture } from "../recipe.ts";

/** The look of her light: Tech's hologram, her aura's. */
const DOJO_PALETTE = "Tech/hologram";

/** Every particle count she throws, at intensity "normal" (rule 9): the dojo booting, her step in, a parry, the attacker dimming. */
const TUNING = {
  boot: { count: 14, power: 0.6 },
  step: { count: 18, power: 1 },
  parry: { count: 10, power: 1 },
  dim: { count: 8, power: 0.5 },
} as const;

/** The dojo stands from the lead to `dojoTailMs` past her entry; a parry's shield flashes its entry and `shieldTailMs` more. */
const TIMING = { dojoTailMs: 350, shieldTailMs: 200 } as const;

/** The gallery's parried blow: the first enemy Unit's attack Nerfed by one. */
const FIXTURE_NERF = { kind: "stats", attack: -1, health: 0 } as const;

export const dojoBoot: Entrance = ({ D, tgt, intensity: i, view, landing, leadMs }) => {
  const look = paletteLook(DOJO_PALETTE) ?? FLAVOR_LOOKS.Tech;
  const at = tid(tgt);
  const lead = leadMs ?? 0;
  const slam = lead + frac(FX_SLAM_AT, D - lead);
  return [
    figureCue("wireDojo", at, figureTint(look), lead, D - lead + TIMING.dojoTailMs, view === undefined || landing === undefined ? {} : { side: sideOf(view, landing.player) }),
    { ...tunedBurst(i, look.particle, at, "area", lead, TUNING.boot), colors: look.colors },
    ringCue(D, look.particle, at, slam),
    { ...tunedBurst(i, look.particle, tid(tgt, FOOT), "ring", slam, TUNING.step), colors: look.colors },
    ...shakeCues(i, ENTRANCE_TRAUMA.dojoBoot, slam),
  ];
};

export const aiGirlfriend: CardRecipe = (event, p, play) => {
  if (event.type !== "degraded") return null;
  const attacker = locateInstance(p.view, event.instanceId);
  const herself = locateInstance(p.view, play.instanceId);
  const palm = herself ?? attacker;
  if (palm === null) return null;
  const look = paletteLook(DOJO_PALETTE) ?? FLAVOR_LOOKS.Tech;
  const i = p.env.intensity;
  return {
    cues: [
      figureCue("hexShield", tid(palm), figureTint(look), 0, p.D + TIMING.shieldTailMs, { size: "small" }),
      { ...tunedBurst(i, look.particle, tid(palm), "point", 0, TUNING.parry), colors: look.colors },
      ...(attacker === null ? [] : [{ ...tunedBurst(i, "smoke", tid(attacker), "area", 0, TUNING.dim), colors: look.colors }]),
    ],
    row: "replace",
  };
};

/** The gallery's AI Girlfriend: one parried blow, the first enemy Unit dimmed. */
export const aiGirlfriendFixture: RecipeFixture = (view, _instanceId, player) => {
  const enemy = (player === view.viewer ? view.opponent : view.you).units.find((unit) => unit !== null);
  return enemy === undefined || enemy === null ? [] : [{ type: "degraded", instanceId: enemy.instanceId, defId: enemy.defId, change: FIXTURE_NERF }];
};
