// R3405001: Meditative #45.1 Knowledge Breaker Prime (`meditative-045-1`, a token printed Legendary,
// entrance "maskSplit", bespoke "breakerPrime", a chalk storm on his tile), a Legendary Unit's signature.
// The cracked mask splits wide and the titan shoulders up through it, chalk-white light pouring from the
// cracks, and at the slam he rushes a step forward, chalk kicking up toward the enemy's half. His Cry slams
// a card back from where he stands into each backrow zone a Trap is set in: a set is a face-down summon
// whose face this seat never sees, so it is drawn at its zone and named nowhere (R202). Then, at the first
// card it destroys (on the Radiant face, exiles), a chalk shockwave rolls out from him, and every other CN
// or KY permanent the board shows crumbles in chalk; a face-down one the viewer cannot read crumbles unseen.
// It keys on his play and what that play has seen (`seen`), never on a card's definition (R202).

import type { GameEvent } from "@jackioh/shared";

import { locateInstance } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, raysCue, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_CRACK_TAIL_MS, FX_SLAM_AT } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { isFirstEntry } from "../../motifs/build.ts";
import type { FxCue, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type CardRecipe, type Entrance, type RecipeFixture } from "../recipe.ts";
import { standingAt } from "./knowledgeBreaker.ts";

/** The look his chalk wears: Nerd's chalk sub-look, as his tile's storm. */
const PRIME_PALETTE = "Nerd/chalk";

/** Every particle count his signature throws, at intensity "normal" (rule 9). */
const TUNING = {
  crack: { count: 18, power: 1.1 },
  stride: { count: 24, power: 1.3 },
  set: { count: 8, power: 0.7 },
  crumble: { count: 14, power: 0.9 },
} as const;

/**
 * As fractions of the entry: the light pours from the cracks at `pourAt`, a set's card back lands at
 * `setAt`. The split mask lasts its entry and `splitTailMs` more, the shockwave its entry and `waveTailMs`.
 */
const TIMING = { pourAt: 0.3, setAt: 0.6, splitTailMs: 300, waveTailMs: 300 } as const;

/** The top edge of a zone, the side toward the enemy's half on the viewer's own side. */
const BROW: FxPoint = { x: 0.5, y: 0 };

/** What his Cry removes: destroyed on the base face, exiled on the Radiant one. */
const REMOVALS: readonly GameEvent["type"][] = ["destroyed", "exiled"];

/** The backrow lane the gallery's Cry sets its Trap in. */
const FIXTURE_TRAP_LANE = 3;

const HIDDEN_ID = "hidden";

export const maskSplit: Entrance = ({ D, tgt, intensity: i, view, landing, leadMs }) => {
  const look = paletteLook(PRIME_PALETTE) ?? FLAVOR_LOOKS.Nerd;
  const lead = leadMs ?? 0;
  const slam = lead + frac(FX_SLAM_AT, D - lead);
  const pour = lead + frac(TIMING.pourAt, D - lead);
  const at = tid(tgt);
  const side = view === undefined || landing === undefined ? undefined : sideOf(view, landing.player);
  // He rushes a step toward the enemy's half: his zone's top edge on the viewer's side, its foot on the other.
  const ahead = tid(tgt, side === "opponent" ? FOOT : BROW);
  return [
    figureCue("maskSplit", at, figureTint(look), lead, D - lead + TIMING.splitTailMs, side === undefined ? {} : { side }),
    raysCue(D, "holy", at, pour),
    { ...tunedBurst(i, look.particle, at, "area", pour, TUNING.crack), colors: look.colors },
    ringCue(D, look.particle, at, slam),
    { ...tunedBurst(i, look.particle, ahead, "ring", slam, TUNING.stride), colors: look.colors },
    ...shakeCues(i, ENTRANCE_TRAUMA.maskSplit, slam),
  ];
};

export const breakerPrime: CardRecipe = (event, p, play) => {
  // His Cry is his play's own.
  if (play.hook !== undefined) return null;
  const look = paletteLook(PRIME_PALETTE) ?? FLAVOR_LOOKS.Nerd;
  const i = p.env.intensity;
  if (event.type === "summoned" && event.row === "backrow" && event.player === play.player) {
    const zone = tid(testid.zone(sideOf(p.view, event.player), "backrow", event.lane));
    const land = frac(TIMING.setAt, p.D);
    const cues: FxCue[] = [
      { kind: "ghost", from: standingAt(p, play), to: zone, delayMs: 0, durationMs: land },
      { ...tunedBurst(i, look.particle, zone, "ring", land, TUNING.set), colors: look.colors },
    ];
    return { cues, row: "replace" };
  }
  if (event.type !== "destroyed" && event.type !== "exiled") return null;
  const cues: FxCue[] = [];
  const first = isFirstEntry(p.env, p.entry, REMOVALS) && p.entry.events.find((candidate) => REMOVALS.includes(candidate.type)) === event;
  if (first) {
    const from = standingAt(p, play);
    cues.push(figureCue("chalkWave", from, figureTint(look), 0, p.D + TIMING.waveTailMs, { side: sideOf(p.view, play.player) }), ringCue(p.D, look.particle, from, 0));
  }
  const card = event.instanceId === HIDDEN_ID ? null : locateInstance(p.view, event.instanceId);
  if (card !== null) {
    const at = tid(card);
    cues.push({ ...tunedBurst(i, look.particle, at, "area", 0, TUNING.crumble), colors: look.colors }, { kind: "crack", at, delayMs: 0, durationMs: p.D + FX_CRACK_TAIL_MS });
  }
  return { cues, row: "replace" };
};

/** The Cry as `player` sees it: a Trap set face down in its backrow, then the first enemy Unit destroyed. */
export const breakerPrimeFixture: RecipeFixture = (view, _instanceId, player) => {
  const set: GameEvent = { type: "summoned", player, instanceId: HIDDEN_ID, defId: HIDDEN_ID, row: "backrow", lane: FIXTURE_TRAP_LANE };
  const foe = (player === view.viewer ? view.opponent : view.you).units.find((unit) => unit !== null);
  if (foe === undefined || foe === null) return [set];
  return [
    set,
    { type: "destroyed", instanceId: foe.instanceId, defId: foe.defId, owner: foe.owner, controller: foe.controller, attack: foe.attack, maxHealth: foe.maxHealth, killerId: null },
  ];
};
