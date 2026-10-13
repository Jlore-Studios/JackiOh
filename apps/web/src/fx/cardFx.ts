// R502: named-card flourishes and Cast on draw are planned only from the redacted stream (R202).
// A card recipe may claim its own resolving events; hidden plays never key recipes.
// R670: marquee Legendary and Mythic Units replace a row entrance when their `summoned` has a `defId`.
// R200: every delay lands inside the entry and trails off within FX_MAX_TAIL_MS.

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { HIDDEN_ID, animTestid, locateInstance, targetFor, type AnimationEntry } from "../game/animations.ts";
import { sideOf, testid } from "../game/contract.ts";
import { frac, raysCue, ringCue, shakeCues, tid, tunedBurst } from "./build.ts";
import {
  FX_BLOOD_FLIGHT_FRACTION,
  FX_BLOOD_PICK_BASE,
  FX_BLOOD_PICK_STRIDE,
  FX_BLOOD_TRAUMA,
  FX_CAST_ON_DRAW_TRAUMA,
  FX_CRACK_HIT_AT,
  FX_CRACK_STAGGER_MS,
  FX_CRACK_TAIL_MS,
  FX_CRACK_TRAUMA,
  FX_FRACTURE_TAIL_MS,
  FX_NEXT_REFRESH_MODIFIER_ID,
  FX_CENTER,
  FX_RAYS_TAIL_MS,
  FX_WALLS_HIT_AT,
  FX_WALLS_REACH,
  FX_WALLS_TAIL_MS,
  FX_WALLS_TRAUMA,
} from "./constants.ts";
import { ENTRANCES, entranceEvent, isEntranceKey, type EntranceKey } from "./entrances.ts";
import { lostCrystals } from "./manaMarks.ts";
import type { FxAnchor, FxCue, FxPlanEnv, FxPlay, FxProjectileCue } from "./types.ts";


/** Every particle count the recipes here throw, at intensity "normal" (rule 9). */
const CARD_TUNING = {
  pileGold: { count: 34, power: 1.3 },
  pileArcane: { count: 20, power: 1 },
  crackFrost: { count: 22, power: 1.1 },
  crystalShard: { count: 10, power: 1 },
  boltFrost: { count: 12, power: 0.8 },
  bloodSpurt: { count: 18, power: 1 },
  bloodLanding: { count: 14, power: 0.9 },
  radiantGold: { count: 30, power: 1.1 },
  bloodPool: { count: 26, power: 1 },
  bloodRing: { count: 16, power: 1.2 },
  wallDust: { count: 24, power: 1.2 },
} as const;

export type CardFxPlan = { entry: AnimationEntry; view: PlayerView; env: FxPlanEnv; D: number };

/** R502: cast on draw bursts from the drawer's Deck pile; other events produce no cues. */
export function castOnDrawCues(event: GameEvent, p: CardFxPlan): FxCue[] {
  if (event.type !== "cardPlayed" || !p.env.memory.castOnDraw(event)) return [];
  const i = p.env.intensity;
  const pile = tid(animTestid.library(sideOf(p.view, event.player)));
  return [
    ringCue(p.D, "gold", pile, 0),
    raysCue(p.D, "radiant", pile, 0),
    tunedBurst(i, "gold", pile, "ring", 0, CARD_TUNING.pileGold),
    tunedBurst(i, "arcane", pile, "area", 0, CARD_TUNING.pileArcane),
    ...shakeCues(i, FX_CAST_ON_DRAW_TRAUMA, 0),
  ];
}

/** A recipe's answer: its cues, and whether the event's own row recipe still plays (`keep`) or not. */
export type CardFxResult = { cues: FxCue[]; row: "keep" | "replace" };

type CardRecipe = (event: GameEvent, p: CardFxPlan, play: FxPlay) => CardFxResult | null;

export type CardFxKey = "manaCrack" | "bloodDrain" | "crushingWalls" | EntranceKey;

function heroOf(view: PlayerView, play: FxPlay): FxAnchor {
  return tid(testid.hero(sideOf(view, play.player)));
}

function projectile(intensity: number, preset: FxProjectileCue["preset"], from: FxAnchor, to: FxAnchor, delayMs: number, flightMs: number): FxProjectileCue {
  return { kind: "projectile", preset, from, to, delayMs, flightMs, density: intensity };
}

/** #21 Hinder cracks next-refresh crystals using the view's count (manaMarks.ts). */
const manaCrack: CardRecipe = (event, p) => {
  if (event.type !== "modifierChanged" || event.modifierId !== FX_NEXT_REFRESH_MODIFIER_ID || !event.added) return null;
  const i = p.env.intensity;
  const side = sideOf(p.view, event.player);
  const tray = tid(animTestid.mana(side));
  const hit = frac(FX_CRACK_HIT_AT, p.D);
  const caster = p.env.memory.resolving();
  const cues: FxCue[] = [];
  if (caster !== undefined) cues.push(projectile(i, "frost", heroOf(p.view, caster), tray, 0, hit));
  cues.push(ringCue(p.D, "frost", tray, hit), tunedBurst(i, "frost", tray, "area", hit, CARD_TUNING.crackFrost));
  const lost = lostCrystals(p.env.next, p.view, event.player);
  if (lost.count === 0) {
    cues.push({ kind: "crack", at: tray, delayMs: hit, durationMs: p.D - hit + FX_CRACK_TAIL_MS });
  }
  for (let k = 0; k < lost.count; k += 1) {
    const crystal: FxAnchor = { kind: "crystal", side, index: lost.from + k };
    const at = Math.min(p.D, hit + k * FX_CRACK_STAGGER_MS);
    cues.push(
      { kind: "fracture", at: crystal, delayMs: at, durationMs: p.D - at + FX_FRACTURE_TAIL_MS },
      { kind: "crack", at: crystal, delayMs: at, durationMs: p.D - at + FX_CRACK_TAIL_MS },
      tunedBurst(i, "frost", crystal, "point", at, CARD_TUNING.crystalShard),
    );
  }
  cues.push(...shakeCues(i, FX_CRACK_TRAUMA, hit));
  return { cues, row: "replace" };
};

/** Where a card made Radiant in the caster's hand is: the viewer's own hand card, else a back (R202). */
function radiantTarget(event: Extract<GameEvent, { type: "radiantSet" }>, p: CardFxPlan, play: FxPlay): FxAnchor {
  const side = sideOf(p.view, play.player);
  const named = event.instanceId === HIDDEN_ID ? null : locateInstance(p.view, event.instanceId);
  if (named !== null && named.startsWith("hand-card-")) return tid(named);
  return { kind: "handCard", side, pick: FX_BLOOD_PICK_BASE + (play.step - 1) * FX_BLOOD_PICK_STRIDE };
}

/** #27 Blood Ridden Glowy Jelly Bean targets its Radiant hand card, or an opponent's back (R97). */
const bloodDrain: CardRecipe = (event, p, play) => {
  const i = p.env.intensity;
  if (event.type === "radiantSet") {
    if (event.zone.z !== "hand" || event.zone.player !== play.player) return null;
    const hero = heroOf(p.view, play);
    const target = radiantTarget(event, p, play);
    const flight = frac(FX_BLOOD_FLIGHT_FRACTION, p.D);
    return {
      cues: [
        tunedBurst(i, "blood", hero, "point", 0, CARD_TUNING.bloodSpurt),
        projectile(i, "blood", hero, target, 0, flight),
        tunedBurst(i, "blood", target, "point", flight, CARD_TUNING.bloodLanding),
        tunedBurst(i, "gold", target, "area", flight, CARD_TUNING.radiantGold),
        { kind: "sheen", at: target, delayMs: flight, durationMs: p.D - flight + FX_RAYS_TAIL_MS },
      ],
      row: "replace",
    };
  }
  if (event.type === "healthLost" && event.player === play.player) {
    const hero = heroOf(p.view, play);
    return {
      cues: [
        tunedBurst(i, "blood", hero, "area", 0, CARD_TUNING.bloodPool),
        tunedBurst(i, "blood", hero, "ring", 0, CARD_TUNING.bloodRing),
        ...shakeCues(i, FX_BLOOD_TRAUMA, 0),
      ],
      row: "keep",
    };
  }
  return null;
};

/** Classic+ #24 Crushing Walls plays at its first destroyed card; empty or countered plays draw none. */
const crushingWalls: CardRecipe = (event, p, play) => {
  if (event.type !== "destroyed" || play.seen.destroyed !== 1) return null;
  const i = p.env.intensity;
  const durationMs = p.D + FX_WALLS_TAIL_MS;
  // The walls reach the cards at FX_WALLS_HIT_AT of the cue; the dust and shake go then, inside the entry.
  const hit = Math.min(p.D, frac(FX_WALLS_HIT_AT, durationMs));
  const edge = (x: number): FxAnchor => tid(testid.board, { x, y: FX_CENTER.y });
  return {
    cues: [
      { kind: "walls", at: tid(testid.board), reach: FX_WALLS_REACH, delayMs: 0, durationMs },
      tunedBurst(i, "dust", edge(FX_WALLS_REACH), "point", hit, CARD_TUNING.wallDust),
      tunedBurst(i, "dust", edge(1 - FX_WALLS_REACH), "point", hit, CARD_TUNING.wallDust),
      ...shakeCues(i, FX_WALLS_TRAUMA, hit),
    ],
    row: "keep",
  };
};

export const CARD_RECIPES: { readonly [K in Exclude<CardFxKey, EntranceKey>]: CardRecipe } = { manaCrack, bloodDrain, crushingWalls };

export const CARD_FX: Readonly<Record<string, CardFxKey>> = {
  "core-021": "manaCrack", // #21 Hinder
  "core-027": "bloodDrain", // #27 Blood Ridden Glowy Jelly Bean
  "classicplus-024": "crushingWalls", // C+ #24 Crushing Walls
  // R670: marquee Legendary and Mythic Units' own entrances (entrances.ts).
  "core-100": "voidCollapse", // #100 Ceaseless Void (Mythic)
  "classic-080": "bigBoom", // BOOM! Big Max
  "classic-045": "titanBloom", // Nature Titan
  "classic-056": "tyrantSigil", // Spell Tyrant
};

function keyOf(defId: string): CardFxKey | undefined {
  return Object.prototype.hasOwnProperty.call(CARD_FX, defId) ? CARD_FX[defId] : undefined;
}

function planEntrance(event: GameEvent, p: CardFxPlan): CardFxResult | null {
  const summoned = entranceEvent(event, HIDDEN_ID);
  if (summoned === null) return null;
  const key = keyOf(summoned.defId);
  if (key === undefined || !isEntranceKey(key)) return null;
  const tgt = targetFor(summoned, p.view);
  if (tgt === null) return null;
  return { cues: ENTRANCES[key]({ D: p.D, tgt, intensity: p.env.intensity }), row: "replace" };
}

export function planCardFx(event: GameEvent, p: CardFxPlan): CardFxResult | null {
  const entrance = planEntrance(event, p);
  if (entrance !== null) return entrance;
  const play = p.env.memory.resolving();
  if (play === undefined || play.defId === HIDDEN_ID) return null;
  const key = keyOf(play.defId);
  return key === undefined || isEntranceKey(key) ? null : CARD_RECIPES[key](event, p, play);
}
