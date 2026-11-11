// R3405001: Meditative #95.1 CN Golem (`meditative-095-1`, a token printed Legendary, entrance "golemLock",
// bespoke "cnGolem", toxic timber on its tile), a Legendary Unit's signature. Red lacquered panels and
// carved stone blocks tumble into its zone and lock together at the slam with heavy clacks, dust bursting
// from its joints as it straightens. Whenever it destroys a Unit, each Super-Virus its kill trigger shuffles
// into the opponent's deck (`shuffledIn`) leaks out of it as a green virus mote, its card back flying into
// that deck pile. Its Cleave charge and the kill itself (`destroyed`) are its attack's, outside its own play
// and hook, so they keep their own rows. It keys on its own play or trigger and the event's type, never on a
// card's definition, and draws card backs alone (R202).

import type { GameEvent, PlayerId } from "@jackioh/shared";

import { animTestid, locateInstance } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import type { FxCue, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, projectile, type CardRecipe, type Entrance, type RecipeFixture } from "../recipe.ts";

/** The look its joints and its viruses glow in: Toxic, its aura's. */
const VIRUS_PALETTE = "Toxic";

/** The look its stone blocks shed: Brute's dust. */
const STONE_LOOK = FLAVOR_LOOKS.Brute;

/** The Super-Virus its kill trigger shuffles in (crates/cards c095_1_cn_golem.rs `SUPER_VIRUS`), for its gallery fixture. */
const SUPER_VIRUS = "core-090-1";

/** Its shoulder joints, where dust bursts as it straightens. */
const JOINTS: readonly FxPoint[] = [
  { x: 0.25, y: 0.35 },
  { x: 0.75, y: 0.35 },
];

/** Every particle count its signature throws, at intensity "normal" (rule 9). */
const TUNING = {
  clack: { count: 18, power: 1.1 },
  joint: { count: 8, power: 0.7 },
  leak: { count: 8, power: 0.6 },
} as const;

/** The blocks last the entry and `tailMs` more, tumbling in from the lead; a virus reaches the deck pile at `leakEnd` of its entry. */
const TIMING = { tailMs: 300, leakEnd: 0.85 } as const;

export const golemLock: Entrance = ({ D, tgt, intensity: i, view, landing, leadMs }) => {
  const at = tid(tgt);
  const lead = leadMs ?? 0;
  const slam = lead + frac(FX_SLAM_AT, D - lead);
  const tint = figureTint(paletteLook(VIRUS_PALETTE) ?? FLAVOR_LOOKS.Toxic);
  const side = view === undefined || landing === undefined ? {} : { side: sideOf(view, landing.player) };
  const cues: FxCue[] = [
    figureCue("golemBlocks", at, tint, lead, D - lead + TIMING.tailMs, side),
    { ...tunedBurst(i, "shard", at, "ring", slam, TUNING.clack), colors: STONE_LOOK.colors },
    ...JOINTS.map((joint) => ({ ...tunedBurst(i, STONE_LOOK.particle, tid(tgt, joint), "point", slam, TUNING.joint), colors: STONE_LOOK.colors })),
    ringCue(D, STONE_LOOK.particle, tid(tgt, FOOT), slam),
    ...shakeCues(i, ENTRANCE_TRAUMA.golemLock, slam),
  ];
  return cues;
};

export const cnGolem: CardRecipe = (event, p, play) => {
  // Its kill trigger's viruses, into the other player's deck (a `trigger` entry; its play's in the gallery).
  if (event.type !== "shuffledIn" || event.player === play.player) return null;
  if (play.hook !== undefined && play.hook !== "trigger") return null;
  const look = paletteLook(VIRUS_PALETTE) ?? FLAVOR_LOOKS.Toxic;
  const i = p.env.intensity;
  // Out of the Golem while the board shows it (it may have left the field since the kill), else its hero.
  const from = tid(locateInstance(p.view, play.instanceId) ?? testid.hero(sideOf(p.view, play.player)));
  const pile = tid(animTestid.library(sideOf(p.view, event.player)));
  const lands = frac(TIMING.leakEnd, p.D);
  return {
    cues: [
      { ...tunedBurst(i, look.particle, from, "point", 0, TUNING.leak), colors: look.colors },
      projectile(i, look.particle, from, pile, 0, lands),
      { kind: "ghost", from, to: pile, delayMs: 0, durationMs: lands },
    ],
    row: "replace",
  };
};

/** R3405001: one Super-Virus its kill shuffles into the other player's deck. */
export const cnGolemFixture: RecipeFixture = (_view, _instanceId, player: PlayerId): GameEvent[] => [
  { type: "shuffledIn", player: player === "p1" ? "p2" : "p1", instanceId: "fx-virus", defId: SUPER_VIRUS, position: 0, creator: player },
];
