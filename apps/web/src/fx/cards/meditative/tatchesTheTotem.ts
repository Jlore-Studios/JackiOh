// R3405001: Meditative #87 Tatches the Totem (`meditative-087`, entrance "totemStack", bespoke "tatches", a
// holy glow on its tile), a Legendary Unit's signature. It bursts up out of its owner's deck pile as a holy
// bolt into its zone, where five carved tiers stack in turn, each face waking bottom first (human,
// cat-folk, scholar, lion-dog, helmet), and it settles at the slam. At the end of its turn each card it
// Buffs (`upgraded`) draws light running up its five tiers and arcing off its top: to the Unit beside it
// (its Radiant face's Buff, whose own stat tick still plays), to its owner's hand pile for a hand card its
// owner reads, else to its owner's deck pile (a deck card, or any card the viewer cannot read). The pile
// wears a sheen on its owner's seat alone, so the other seat never learns which card, or whether hand or
// deck (R202). It keys on its own play or turn hook and the event's type, never on a card's definition, and
// draws no card.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { HIDDEN_ID, animTestid, locateInstance } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS } from "../../flavors.ts";
import type { FxCue, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, projectile, type CardRecipe, type Entrance, type RecipeFixture } from "../recipe.ts";

/** The look its light wears: Holy's, its aura's. */
const TOTEM_LOOK = FLAVOR_LOOKS.Holy;

/** Its five tiers' faces, bottom first, as heights in its zone. */
const TIERS: readonly number[] = [0.85, 0.68, 0.51, 0.34, 0.17];

/** The top of the totem, where its light arcs off. */
const CROWN: FxPoint = { x: 0.5, y: 0.1 };

/** Every particle count its signature throws, at intensity "normal" (rule 9). */
const TUNING = {
  pile: { count: 12, power: 0.8 },
  wake: { count: 5, power: 0.5 },
  climb: { count: 3, power: 0.4 },
  settle: { count: 16, power: 0.9 },
} as const;

/**
 * As fractions of the motion after the lead: the bolt out of the deck lands at `riseEnd`, the totem
 * starts stacking at `stackAt` and lasts the entry and `tailMs` more, its faces wake from `wakeFirst`
 * one `wakeStep` apart. At its Buff the light climbs one `climbStep` a tier, arcs off at `arcAt` and lands
 * at `arcEnd`, its sheen lasting the rest of the entry and `sheenTailMs` more.
 */
const TIMING = { riseEnd: 0.25, stackAt: 0.2, wakeFirst: 0.25, wakeStep: 0.08, tailMs: 300, climbStep: 0.07, arcAt: 0.35, arcEnd: 0.85, sheenTailMs: 300 } as const;

export const totemStack: Entrance = ({ D, tgt, intensity: i, view, landing, leadMs }) => {
  const look = TOTEM_LOOK;
  const at = tid(tgt);
  const lead = leadMs ?? 0;
  const span = D - lead;
  const slam = lead + frac(FX_SLAM_AT, span);
  const cues: FxCue[] = [];
  // It bursts up out of its owner's deck pile into its zone.
  if (view !== undefined && landing !== undefined) {
    const pile = tid(animTestid.library(sideOf(view, landing.player)));
    cues.push(
      { ...tunedBurst(i, look.particle, pile, "point", lead, TUNING.pile), colors: look.colors },
      { kind: "shot", shot: look.projectile, colors: look.colors, preset: look.particle, from: pile, to: at, delayMs: lead, flightMs: frac(TIMING.riseEnd, span), density: i },
    );
  }
  const stack = lead + frac(TIMING.stackAt, span);
  const side = view === undefined || landing === undefined ? {} : { side: sideOf(view, landing.player) };
  cues.push(figureCue("totem", at, figureTint(look), stack, D - stack + TIMING.tailMs, side));
  // Each tier's face wakes in turn, bottom first.
  TIERS.forEach((y, k) => {
    cues.push({ ...tunedBurst(i, look.particle, tid(tgt, { x: 0.5, y }), "point", lead + frac(TIMING.wakeFirst + k * TIMING.wakeStep, span), TUNING.wake), colors: look.colors });
  });
  cues.push(ringCue(D, look.particle, at, slam), { ...tunedBurst(i, look.particle, at, "ring", slam, TUNING.settle), colors: look.colors }, ...shakeCues(i, ENTRANCE_TRAUMA.totemStack, slam));
  return cues;
};

export const tatches: CardRecipe = (event, p, play) => {
  if (event.type !== "upgraded") return null;
  // Its Buff is its end-of-turn hook's (a `trigger` entry), its play's in the gallery.
  if (play.hook !== undefined && play.hook !== "trigger") return null;
  const look = TOTEM_LOOK;
  const i = p.env.intensity;
  const caster = sideOf(p.view, play.player);
  const own = locateInstance(p.view, play.instanceId) ?? testid.hero(caster);
  const target = locateInstance(p.view, event.instanceId);
  const arc = frac(TIMING.arcAt, p.D);
  const lands = frac(TIMING.arcEnd, p.D);
  // Light runs up its five tiers, bottom first.
  const cues: FxCue[] = TIERS.map((y, k) => ({ ...tunedBurst(i, look.particle, tid(own, { x: 0.5, y }), "point", frac(k * TIMING.climbStep, p.D), TUNING.climb), colors: look.colors }));
  if (target === testid.card(event.instanceId)) {
    // A Unit on the field: the light lands on it, and its own stat tick plays.
    cues.push(projectile(i, look.particle, tid(own, CROWN), tid(target), arc, lands - arc));
    return { cues, row: "keep" };
  }
  const pile = tid(target === testid.handCard(event.instanceId) ? animTestid.hand(caster) : animTestid.library(caster));
  cues.push(projectile(i, look.particle, tid(own, CROWN), pile, arc, lands - arc));
  // The Buff is its owner's to read: the sheen plays on their seat alone.
  if (p.view.viewer === play.player) cues.push({ kind: "sheen", at: pile, delayMs: lands, durationMs: p.D - lands + TIMING.sheenTailMs });
  return { cues, row: "replace" };
};

/** R3405001: its end-of-turn Buff on the first card of its owner's hand, read by its owner; on the other seat a card it cannot read. */
export const tatchesFixture: RecipeFixture = (view: PlayerView, _instanceId: string, player: PlayerId): GameEvent[] => {
  const hand = player === view.viewer && Array.isArray(view.you.hand) ? view.you.hand[0] : undefined;
  return [
    hand === undefined
      ? { type: "upgraded", instanceId: HIDDEN_ID, defId: HIDDEN_ID, change: { kind: "none" } }
      : { type: "upgraded", instanceId: hand.instanceId, defId: hand.defId, change: { kind: "stats", attack: 1, health: 1 } },
  ];
};
