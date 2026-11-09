// R3395000: Stinkyscape #7 The Greatest Heist (`stinkyscape-007`, entrance "heistDrop", bespoke
// "heistLocks", a laser grid in its zone), a Legendary Field Spell's signature. A rope line drops into its
// zone and the vault crashes down on it, three nested lock rings spinning; its Cry's Discover pick flies to
// the hand as a card back. Each `questProgressed` of its quests turns one lock ring (a small ring figure)
// and each `questCompleted` swings the vault open on its reward: a ring where its reward Unit lands, a card
// back for each card it adds, and at Stage 3 the opponent's mana crystals stream from their mana bar into
// the caster's. It keys on its play or its hook and the Heist's own public id, never on a card's definition;
// the Discover's offer is a prompt, so no beat shows it (R202). Moved: the steal that Stage 3 asks for
// happens in other plays, which key nothing of the Heist's, and the laser grid on its tile is a sweep of
// green motes (an aura draws motes and an edge).

import { animTestid } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { ringCue, tid, tunedBurst } from "../../build.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { motifShot, otherSide } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import { FOOT, type CardRecipe, type Entrance } from "../recipe.ts";
import { cardAt, cardBack, entranceFigure, handOf, lookOf, replace, replaceMoment, slamAt, slamShake, summonZone } from "./signatureKit.ts";

/** The cash-green of its vault and its lasers. */
const CASH = "Hustle/cash";

/** Where the rope drops from: above its zone, by the zone's height. */
const ROPE_FROM = { x: 0.5, y: -1.5 } as const;

/** Every particle count the Heist throws, at intensity "normal" (rule 9). */
const TUNING = {
  crash: { count: 22, power: 1.1 },
  dust: { count: 12, power: 0.8 },
} as const;

/** As shares of the entry: when a reward's ring flashes, and how long the mana's stream flies. */
const TIMING = { streamFlight: 0.5 } as const;

export const heistDrop: Entrance = (plan) => {
  const look = lookOf(CASH);
  const at = tid(plan.tgt);
  const slam = slamAt(plan);
  const i = plan.intensity;
  const rope = motifShot({ look, scale: i, row: "replace" }, "bolt", tid(plan.tgt, ROPE_FROM), at, 0, slam);
  return [
    rope,
    entranceFigure("lockRings", CASH, plan),
    { ...tunedBurst(i, "shard", at, "area", slam, TUNING.crash), colors: look.colors },
    tunedBurst(i, "dust", tid(plan.tgt, FOOT), "area", slam, TUNING.dust),
    ringCue(plan.D, "spark", at, slam),
    ...slamShake(plan, "heistDrop"),
  ];
};

export const heistLocks: CardRecipe = (event, p, play) => {
  const look = lookOf(CASH);
  const vault = cardAt(p.view, play);
  if (event.type === "questProgressed" && event.instanceId === play.instanceId) {
    // One ring of the lock turns, in place of the shared quest tally too (R3395001).
    return replaceMoment([figureCue("lockRings", vault, figureTint(look), 0, p.D, { size: "small" }), ringCue(p.D, "spark", vault, 0)]);
  }
  if (event.type === "questCompleted" && event.instanceId === play.instanceId) {
    // The vault swings open on its reward, in place of the shared quest moment too (R3395001).
    return replaceMoment([figureCue("lockRings", vault, figureTint(look), 0, p.D), ringCue(p.D, "gold", vault, 0), { kind: "sheen", at: vault, delayMs: 0, durationMs: p.D }]);
  }
  if (event.type === "addedToHand" && event.player === play.player) {
    return replace([cardBack(vault, handOf(p.view, event.player), 0, p.D)]);
  }
  if (play.hook === undefined) return null;
  if (event.type === "summoned" && event.player === play.player) {
    const at = tid(summonZone(p.view, event));
    return replace([ringCue(p.D, "gold", at, 0), { kind: "sheen", at, delayMs: 0, durationMs: p.D }]);
  }
  if (event.type === "manaChanged") {
    // Stage 3: the opponent's crystals stream into the caster's mana bar.
    const caster = sideOf(p.view, play.player);
    const side = sideOf(p.view, event.player);
    if (side === caster) return replace([ringCue(p.D, "gold", tid(animTestid.mana(caster)), 0)]);
    const from = tid(animTestid.mana(otherSide(caster)));
    const stream: FxCue = { ...motifShot({ look, scale: p.env.intensity, row: "replace" }, "bolt", from, tid(animTestid.mana(caster)), 0, Math.round(TIMING.streamFlight * p.D)) };
    return replace([stream]);
  }
  return null;
};
