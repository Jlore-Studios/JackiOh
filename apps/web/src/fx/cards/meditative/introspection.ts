// R3405001: Meditative #7 Introspection (`meditative-007`, entrance "poolBreath", bespoke "introspection",
// a pale ripple aura on its zone), a Legendary Field Spell's signature. As it lands a still pool opens on
// its zone and a seated monk's silhouette forms in the water, eyes closing; at the slam one slow breath of
// white light rolls out from it. Its start of turn then reads in its written order: the breath draws in
// and each mend glows on the hero or Unit it heals; the card it draws flies home as a card back; the hand
// pile wears a sheen as the gems dim, on its owner's seat alone (the costs are theirs to read); and a
// thought-dart snaps out from its zone at each enemy it hits. A run that heals or hits every Unit on a
// side plays as the board's sweep, which no recipe draws. It keys on its play or hook and what that has
// seen (`seen`), never on a card's definition, and draws card backs alone (R202). Its beats key on the
// event, not on the hook (the gallery plays them under its play, a game under its hook's marker).

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { animTestid, locateInstance, targetFor } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_CENTER, FX_HEAL_SPLAT_AT, FX_SLAM_AT, FX_SPLAT_HOLD_MS } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { hit, isFirstEntry } from "../../motifs/build.ts";
import type { FxAnchor, FxCue, FxPlay } from "../../types.ts";
import { ENTRANCE_TRAUMA, heroOf, projectile, type CardFxPlan, type CardRecipe, type Entrance, type RecipeFixture } from "../recipe.ts";

/** The look its pool and breath wear: Holy's, as its aura's. */
const INTROSPECTION_PALETTE = "Holy";

/** Every particle count it throws, at intensity "normal" (rule 9). */
const TUNING = {
  drip: { count: 8, power: 0.4 },
  breath: { count: 24, power: 1 },
  mend: { count: 10, power: 0.6 },
  dart: { count: 12, power: 1 },
} as const;

/**
 * The pool lasts from its lead to its entry's end and `poolTailMs` more, the breath from the slam to
 * `breathTailMs` past the end; a thought-dart flies `dartFlight` of its entry.
 */
const TIMING = { poolTailMs: 300, breathTailMs: 300, dartFlight: 0.3 } as const;

/**
 * R3405001: where a Field Spell's beat stands: its card while the board shows it face up; else the first
 * face-up backrow card of its definition on its side (the gallery's stand-in, which lands it in the first
 * backrow zone); else its hero. Introspection's batch of Field Spells shares it.
 */
export function fieldTile(view: PlayerView, play: FxPlay): FxAnchor {
  const card = locateInstance(view, play.instanceId);
  if (card !== null && card.startsWith("card-")) return tid(card);
  const side = sideOf(view, play.player);
  const lane = view[side].backrow.findIndex((slot) => slot !== null && !slot.faceDown && slot.defId === play.defId);
  return lane < 0 ? heroOf(view, play) : tid(testid.zone(side, "backrow", lane + 1));
}

/** R3405001: whether `event` is the first of its type the play or hook resolving has seen. */
export function firstOfPlay(event: GameEvent, p: CardFxPlan): boolean {
  return isFirstEntry(p.env, p.entry, [event.type]) && p.entry.events.find((other) => other.type === event.type) === event;
}

/** Where a heal or a hit lands as the view shows it; the board's centre for one it hides. */
function landing(event: GameEvent, view: PlayerView): FxAnchor {
  const at = targetFor(event, view);
  return at === null ? tid(testid.board, FX_CENTER) : tid(at);
}

export const poolBreath: Entrance = ({ D, tgt, intensity: i, view, landing: landed, leadMs }) => {
  const look = paletteLook(INTROSPECTION_PALETTE) ?? FLAVOR_LOOKS.Holy;
  const tint = figureTint(look);
  const at = tid(tgt);
  const lead = leadMs ?? 0;
  const slam = lead + frac(FX_SLAM_AT, D - lead);
  const side = view === undefined || landed === undefined ? {} : { side: sideOf(view, landed.player) };
  return [
    figureCue("stillPool", at, tint, lead, D - lead + TIMING.poolTailMs, side),
    { ...tunedBurst(i, look.particle, at, "point", lead, TUNING.drip), colors: look.colors },
    figureCue("breathRing", at, tint, slam, D - slam + TIMING.breathTailMs, side),
    ringCue(D, look.particle, at, slam),
    { ...tunedBurst(i, look.particle, at, "ring", slam, TUNING.breath), colors: look.colors },
    ...shakeCues(i, ENTRANCE_TRAUMA.poolBreath, slam),
  ];
};

export const introspection: CardRecipe = (event, p, play) => {
  const look = paletteLook(INTROSPECTION_PALETTE) ?? FLAVOR_LOOKS.Holy;
  const i = p.env.intensity;
  const side = sideOf(p.view, play.player);
  const hand = tid(animTestid.hand(side));
  switch (event.type) {
    case "healed": {
      // The breath draws in on the play's first mend; each mend glows where it lands, with its number.
      const at = landing(event, p.view);
      const splatAt = frac(FX_HEAL_SPLAT_AT, p.D);
      const cues: FxCue[] = firstOfPlay(event, p) ? [figureCue("breathRing", fieldTile(p.view, play), figureTint(look), 0, p.D, { side, size: "small" })] : [];
      cues.push(
        { ...tunedBurst(i, look.particle, at, "area", 0, TUNING.mend), colors: look.colors },
        { kind: "splat", tone: "heal", amount: event.amount, at, delayMs: splatAt, durationMs: p.D - splatAt + FX_SPLAT_HOLD_MS },
      );
      return { cues, row: "replace" };
    }
    case "drawn":
      if (event.player !== play.player) return null;
      return { cues: [{ kind: "ghost", from: tid(animTestid.library(side)), to: hand, delayMs: 0, durationMs: p.D }], row: "replace" };
    case "costChanged": {
      // The gems dim on its owner's seat alone, once for the whole hand.
      const owner = p.view.viewer === play.player && firstOfPlay(event, p);
      return { cues: owner ? [{ kind: "sheen", at: hand, delayMs: 0, durationMs: p.D }] : [], row: "replace" };
    }
    case "damage": {
      const at = landing(event, p.view);
      const flight = frac(TIMING.dartFlight, p.D);
      const spec = { look, scale: i, row: "replace" } as const;
      return { cues: [projectile(i, look.particle, fieldTile(p.view, play), at, 0, flight), ...hit(spec, event, at, p.D, flight, TUNING.dart)], row: "replace" };
    }
    default:
      return null;
  }
};

/** Its last two beats, in its order: the first hand card's gem dims, then a dart hits the first enemy Unit. */
export const introspectionFixture: RecipeFixture = (view, instanceId, player) => {
  const mine = player === view.viewer ? view.you : view.opponent;
  const foes = player === view.viewer ? view.opponent : view.you;
  const card = Array.isArray(mine.hand) ? mine.hand[0] : undefined;
  const foe = foes.units.find((unit) => unit !== null);
  return [
    ...(card === undefined ? [] : [{ type: "costChanged" as const, instanceId: card.instanceId, cost: Math.max(0, card.cost - 1) }]),
    { type: "damage", sourceId: instanceId, targetId: foe?.instanceId ?? `hero-${foes.player}`, amount: 1, combat: false },
  ];
};
