// R3405001: Meditative #11 Double Header (`meditative-011`, entrance "floodlights", bespoke "doubleHeader",
// an arcane aura on its zone), a Legendary Field Spell's signature. As it lands two floodlights swing on
// over its zone and at the slam a ticket stub drops into it, tearing along its perforation into two
// matching halves. When the turn's first card resolves, the copy it adds peels off that card as an arcane
// after-image (off its own zone when the board no longer shows the card: a Spell's, a hidden one's), folds
// into a ticket half and flies to its owner's hand pile as a card back; the copy's face is its owner's to
// read, so the hand pile's sheen plays on their seat alone. It keys on its play or hook and the copy's
// owner, never on a card's definition, and draws card backs alone (R202); its beat keys on the event, not
// on the hook (the gallery plays it under its play, a game under its trigger's marker).

import type { GameEvent } from "@jackioh/shared";

import { animTestid, locateInstance } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import type { FxAnchor, FxCue, FxPlay } from "../../types.ts";
import { ENTRANCE_TRAUMA, type CardFxPlan, type CardRecipe, type Entrance, type RecipeFixture } from "../recipe.ts";
import { fieldTile } from "./introspection.ts";

/** The look its floodlights and tickets wear: Physical's sport sub-look, a ballpark at night. */
const DOUBLE_HEADER_PALETTE = "Physical/sport";

/** Every particle count it throws, at intensity "normal" (rule 9). */
const TUNING = {
  stub: { count: 18, power: 0.9 },
  peel: { count: 12, power: 0.6 },
} as const;

/**
 * The floodlights last from its lead to its entry's end and `lightsTailMs` more, the stub from the slam
 * to `stubTailMs` past the end; the copy's ticket half leaves the after-image `foldAt` into its entry.
 */
const TIMING = { lightsTailMs: 300, stubTailMs: 300, foldAt: 0.3 } as const;

/** The card the copy peels off: the last card its owner resolved that the board still shows face up, else its own zone. */
function afterImage(p: CardFxPlan, play: FxPlay): FxAnchor {
  const first = [...p.env.memory.recent()]
    .reverse()
    .find((event): event is Extract<GameEvent, { type: "cardResolved" }> => event.type === "cardResolved" && event.player === play.player && event.instanceId !== play.instanceId);
  const at = first === undefined ? null : locateInstance(p.view, first.instanceId);
  return at !== null && at.startsWith("card-") ? tid(at) : fieldTile(p.view, play);
}

export const floodlights: Entrance = ({ D, tgt, intensity: i, view, landing, leadMs }) => {
  const look = paletteLook(DOUBLE_HEADER_PALETTE) ?? FLAVOR_LOOKS.Physical;
  const tint = figureTint(look);
  const at = tid(tgt);
  const lead = leadMs ?? 0;
  const slam = lead + frac(FX_SLAM_AT, D - lead);
  const side = view === undefined || landing === undefined ? {} : { side: sideOf(view, landing.player) };
  return [
    figureCue("floodlights", at, tint, lead, D - lead + TIMING.lightsTailMs, side),
    figureCue("ticketStub", at, tint, slam, D - slam + TIMING.stubTailMs, side),
    ringCue(D, look.particle, at, slam),
    { ...tunedBurst(i, look.particle, at, "ring", slam, TUNING.stub), colors: look.colors },
    ...shakeCues(i, ENTRANCE_TRAUMA.floodlights, slam),
  ];
};

export const doubleHeader: CardRecipe = (event, p, play) => {
  if (event.type !== "addedToHand" || event.player !== play.player) return null;
  const look = paletteLook(DOUBLE_HEADER_PALETTE) ?? FLAVOR_LOOKS.Physical;
  const side = sideOf(p.view, play.player);
  const hand = tid(animTestid.hand(side));
  const from = afterImage(p, play);
  const fold = frac(TIMING.foldAt, p.D);
  const cues: FxCue[] = [
    { ...tunedBurst(p.env.intensity, "arcane", from, "point", 0, TUNING.peel), colors: look.colors },
    figureCue("ticketStub", from, figureTint(look), 0, p.D, { side, size: "small" }),
    { kind: "ghost", from, to: hand, delayMs: fold, durationMs: p.D - fold },
  ];
  // The copy's face is its owner's to read.
  if (p.view.viewer === play.player) cues.push({ kind: "sheen", at: hand, delayMs: fold, durationMs: p.D - fold });
  return { cues, row: "replace" };
};

/** The copy of the turn's first card, added to its owner's hand. */
export const doubleHeaderFixture: RecipeFixture = (_view, _instanceId, player) => [{ type: "addedToHand", player, instanceId: "fx-second-game", defId: "core-055" }];
