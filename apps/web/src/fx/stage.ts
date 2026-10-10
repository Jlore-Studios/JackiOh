// Stage directions (S7; B46–B48) bridge BUILD M5-T4's stale board with holds, conceals, and targeted lunges.
// R200: never pace or reschedule entries; FxLayer ends holds and conceals with the view or FX_HOLD_MAX_MS.
// R202: stand-ins show only viewer-visible cards, otherwise a card-shaped light. Pure, like `cues.ts`.

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { animTestid, locateInstance, targetFor, type AnimationEntry } from "../game/animations.ts";
import { sideOf, testid } from "../game/contract.ts";
import {
  FX_CONCEAL_AT,
  FX_HOLD_MAX_MS,
  FX_LUNGE_CONTACT_AT,
  FX_MIND_CONTROL_FLIGHT_FRACTION,
  FX_SLAM_AT,
} from "./constants.ts";
import type { FxAnchor, FxCue, FxPlanEnv } from "./types.ts";

/** The contact spark's base count and power (rule 9). */
const STAGE_TUNING = {
  contactSpark: { count: 14, power: 1 },
} as const;

const HERO_TARGET = /^hero-(p1|p2)$/;

function tid(id: string): FxAnchor {
  return { kind: "testid", testid: id };
}

const frac = (fraction: number, D: number): number => Math.round(fraction * D);

function attackTarget(targetId: string, view: PlayerView): string | null {
  const hero = HERO_TARGET.exec(targetId)?.[1];
  if (hero === "p1" || hero === "p2") return testid.hero(sideOf(view, hero));
  return locateInstance(view, targetId);
}

type Ctx = { entry: AnimationEntry; view: PlayerView; env: FxPlanEnv; D: number };

function stageOf(event: GameEvent, c: Ctx): FxCue[] {
  const { entry, view, D } = c;
  const hold = (from: FxAnchor | null, to: string, landMs: number): FxCue => ({
    kind: "hold",
    from,
    to: tid(to),
    delayMs: 0,
    landMs,
    durationMs: FX_HOLD_MAX_MS,
  });
  const conceal = (id: string, mode: "now" | "after"): FxCue => ({
    kind: "conceal",
    testid: id,
    mode,
    delayMs: mode === "now" ? 0 : frac(FX_CONCEAL_AT, D),
    durationMs: FX_HOLD_MAX_MS,
  });

  switch (event.type) {
    case "cardPlayed": {
      const paired = entry.events.some((e) => e.type === "summoned" && e.instanceId === event.instanceId);
      const at = targetFor(event, view);
      if (paired || at === null || !at.startsWith("hand-card-")) return [];
      return [conceal(at, "after")];
    }
    case "summoned": {
      const zone = targetFor(event, view);
      if (zone === null) return [];
      const played = entry.events.find(
        (e): e is Extract<GameEvent, { type: "cardPlayed" }> => e.type === "cardPlayed" && e.instanceId === event.instanceId,
      );
      const wait = entry.slam?.instanceId === event.instanceId ? entry.slam.anticipationMs : 0;
      const landMs = wait + frac(FX_SLAM_AT, D - wait);
      // R502: a card cast as it was drawn never was in a hand, so it comes out of its Deck pile.
      if (played !== undefined && c.env.memory.castOnDraw(played)) {
        return [hold(tid(animTestid.library(sideOf(view, played.player))), zone, landMs)];
      }
      if (played !== undefined && event.instanceId !== "hidden") {
        // R227: a card set face-down took a fresh id, and its hand card still has the old one.
        const inHand = locateInstance(view, event.formerId ?? event.instanceId);
        if (inHand !== null && inHand.startsWith("hand-card-")) {
          return [hold(tid(inHand), zone, landMs), conceal(inHand, "now")];
        }
      }
      if (played !== undefined && sideOf(view, played.player) === "opponent") {
        // Out of a hand the viewer only sees as backs: a back flies to the zone (R202).
        return [hold(tid(animTestid.hand("opponent")), zone, landMs)];
      }
      return [hold(null, zone, landMs)];
    }
    case "destroyed":
    case "exiled":
    case "bounced": {
      const at = targetFor(event, view);
      return at !== null && at.startsWith("card-") ? [conceal(at, "after")] : [];
    }
    case "discarded": {
      const at = targetFor(event, view);
      return at !== null && at.startsWith("hand-card-") ? [conceal(at, "after")] : [];
    }
    case "controlChanged": {
      const zone = targetFor(event, view);
      if (zone === null || event.instanceId === "hidden") return [];
      const from = locateInstance(view, event.instanceId);
      if (from === null || !from.startsWith("card-")) return [];
      return [hold(tid(from), zone, frac(FX_MIND_CONTROL_FLIGHT_FRACTION, D)), conceal(from, "now")];
    }
    case "attackDeclared": {
      const attacker = targetFor(event, view);
      const target = attackTarget(event.targetId, view);
      if (attacker === null || target === null || attacker === target) return [];
      const tuning = STAGE_TUNING.contactSpark;
      return [
        { kind: "lunge", attacker, target, delayMs: 0, durationMs: D },
        {
          kind: "burst",
          preset: "spark",
          at: tid(target),
          delayMs: frac(FX_LUNGE_CONTACT_AT, D),
          count: Math.max(1, Math.round(tuning.count * c.env.intensity)),
          spread: "point",
          power: tuning.power,
        },
      ];
    }
    default:
      return [];
  }
}

/** The layer is off at intensity 0. */
export function planStage(entry: AnimationEntry, view: PlayerView, env: FxPlanEnv): FxCue[] {
  if (!(env.intensity > 0)) return [];
  const cues: FxCue[] = [];
  for (const event of entry.events) cues.push(...stageOf(event, { entry, view, env, D: entry.durationMs }));
  return cues;
}
