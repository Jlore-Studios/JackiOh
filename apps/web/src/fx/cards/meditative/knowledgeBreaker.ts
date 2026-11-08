// R3405001: Meditative #45 Knowledge Breaker (`meditative-045`, entrance "chalkMask", bespoke
// "knowledgeBreaker", a chalk haze on his tile), a Legendary Unit's signature. He lands in a puff of chalk
// dust, his cracked porcelain mask turning slowly to face the board while every lamp on the field dims to a
// chalk-white haze around his zone, and at the slam chalk rings out from it. His Cry lashes a chalk tendril
// onto each other CN and KY card it Nerfs, arrows down over it: a card on the board or in the viewer's own
// hand. A card the viewer cannot read (one in the opponent's hand, a face-down one) has no place on this
// seat, so its Nerf draws nothing (R202). At his Death his Prime's card back shuffles from where he stood
// into his owner's deck pile. The plan's Cloak haze is his tile's aura (fx/aura/), not a cue on each
// friendly Unit. It keys on his play or his Death's entry and the event's type, never on a card's
// definition, and draws card backs alone (R202).

import type { GameEvent } from "@jackioh/shared";

import { animTestid, locateInstance } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_ARROWS_TAIL_MS, FX_SLAM_AT } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import type { FxAnchor, FxCue, FxPlay } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, heroOf, type CardFxPlan, type CardRecipe, type Entrance, type RecipeFixture } from "../recipe.ts";

/** The look his chalk wears: Nerd's chalk sub-look. */
const BREAKER_PALETTE = "Nerd/chalk";

/** Every particle count his signature throws, at intensity "normal" (rule 9). */
const TUNING = {
  puff: { count: 20, power: 0.8 },
  ring: { count: 16, power: 1 },
  lash: { count: 6, power: 0.7 },
  home: { count: 10, power: 0.6 },
} as const;

/** The mask lasts its entry and `maskTailMs` more; a tendril its Nerf's entry and `tendrilTailMs` more. */
const TIMING = { maskTailMs: 300, tendrilTailMs: 250 } as const;

/** The Nerf the gallery's Cry lays on each card it reaches: one off its attack and its health. */
const FIXTURE_NERF = { kind: "stats", attack: -1, health: -1 } as const;

/**
 * Where the card `play` is resolving for stands, as this batch's Units read it: its card while the board's
 * view shows it, else the zone its own landing named (the view is the board before the batch, so a Unit
 * landing in it is not on it yet; the play's recent events still hold its `summoned`), else its hero.
 */
export function standingAt(p: CardFxPlan, play: FxPlay): FxAnchor {
  const card = locateInstance(p.view, play.instanceId);
  if (card !== null) return tid(card);
  const landing = p.env.memory.recent().find((event) => event.type === "summoned" && event.instanceId === play.instanceId);
  return landing?.type === "summoned" ? tid(testid.zone(sideOf(p.view, landing.player), landing.row, landing.lane)) : heroOf(p.view, play);
}

export const chalkMask: Entrance = ({ D, tgt, intensity: i, view, landing, leadMs }) => {
  const look = paletteLook(BREAKER_PALETTE) ?? FLAVOR_LOOKS.Nerd;
  const lead = leadMs ?? 0;
  const slam = lead + frac(FX_SLAM_AT, D - lead);
  const at = tid(tgt);
  const side = view === undefined || landing === undefined ? {} : { side: sideOf(view, landing.player) };
  return [
    { ...tunedBurst(i, look.particle, tid(tgt, FOOT), "area", lead, TUNING.puff), colors: look.colors },
    // The mask turns to the board as the field around his zone dims to chalk-white (its figure's veil).
    figureCue("porcelainMask", at, figureTint(look), lead, D - lead + TIMING.maskTailMs, side),
    ringCue(D, look.particle, at, slam),
    { ...tunedBurst(i, look.particle, at, "ring", slam, TUNING.ring), colors: look.colors },
    ...shakeCues(i, ENTRANCE_TRAUMA.chalkMask, slam),
  ];
};

export const knowledgeBreaker: CardRecipe = (event, p, play) => {
  const look = paletteLook(BREAKER_PALETTE) ?? FLAVOR_LOOKS.Nerd;
  if (play.hook === undefined && event.type === "degraded") {
    // R202: a card the viewer cannot read is in no place this seat draws.
    const card = event.hiddenFrom?.includes(p.view.viewer) === true ? null : locateInstance(p.view, event.instanceId);
    if (card === null) return { cues: [], row: "replace" };
    const at = tid(card);
    const cues: FxCue[] = [
      figureCue("chalkTendril", at, figureTint(look), 0, p.D + TIMING.tendrilTailMs, { side: sideOf(p.view, play.player) }),
      { kind: "arrows", direction: "down", at, delayMs: 0, durationMs: p.D + FX_ARROWS_TAIL_MS },
      { ...tunedBurst(p.env.intensity, look.particle, at, "point", 0, TUNING.lash), colors: look.colors },
    ];
    return { cues, row: "replace" };
  }
  if (play.hook === "death" && event.type === "shuffledIn") {
    const card = locateInstance(p.view, play.instanceId);
    const from = card === null ? heroOf(p.view, play) : tid(card);
    const deck = tid(animTestid.library(sideOf(p.view, event.player)));
    const cues: FxCue[] = [
      { ...tunedBurst(p.env.intensity, look.particle, from, "area", 0, TUNING.home), colors: look.colors },
      { kind: "ghost", from, to: deck, delayMs: 0, durationMs: p.D },
    ];
    return { cues, row: "replace" };
  }
  return null;
};

/** The Cry as `player` sees it: the first enemy Unit and the viewer's own first hand card Nerfed (an opponent's hand card would be hidden). */
export const knowledgeBreakerFixture: RecipeFixture = (view, _instanceId, player) => {
  const foe = (player === view.viewer ? view.opponent : view.you).units.find((unit) => unit !== null);
  const hand = player === view.viewer && Array.isArray(view.you.hand) ? view.you.hand[0] : undefined;
  const nerfed = [foe, hand].flatMap((card) => (card === undefined || card === null ? [] : [card]));
  return nerfed.map((card): GameEvent => ({ type: "degraded", instanceId: card.instanceId, defId: card.defId, change: FIXTURE_NERF }));
};
