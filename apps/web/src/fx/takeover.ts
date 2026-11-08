// R2722001: a Legendary's or a Mythic's entrance (animations.ts `entranceEntries`), and a Mythic's
// takeover of the whole board, after Hearthstone's Deathwing. R2803000: both land in the motif
// language's two-wave landing, the Legendary one volume down: its gold echo follows its ring as the
// Mythic's second shockwave follows its first, and the shared prismatic sweep has its own keyframes
// (fx.css), so the loudest tiers read as one language with the set passes' Legendary and Mythic looks.
//
// The queue gives a readable Mythic's entry the takeover's hold (`TAKEOVER_HOLD_MS`, its set piece, R2785000),
// so the board can darken and a card-shaped silhouette sweep across it before the card lands at
// `entranceLandMs`; there the rays turn, a shockwave crosses both sides and the board shakes as hard
// as it ever does. Everything here is cue data built as cues.ts builds its own: it starts inside the
// entry and is gone within FX_MAX_TAIL_MS of its end (R200), it keys only on the entry's `entrance`,
// which names a card the seat's own event names (R202), and the effects off or reduced motion draws
// none of it (planFx is never asked). The marquee Mythics take a look of their own (R670), the rest the
// shared prismatic one. A Legendary takes no takeover; its entrance is the rays, a gold burst, a ring
// with its echo and FX_LEGENDARY_TRAUMA, drawn here when no summon of it draws them already (a Spell's cast, a reveal).
// R3332000: a Mythic's own look is its table entry's `takeover` (`takeoverLookOf`), and a Field Spell
// landing face up with an `entrance` of its own draws that in place of the shared rays, burst and ring.
// R3389000: so does a Spell's, a Trap's or a Field Trap's signature `{ bespoke }` cast of its own
// (`ownCast`), and In Too Deep's takeover is its own, "abyss": a spiral shaft plunging into violet dark.

import { HIDDEN_ID, leadMs, type AnimationEntry } from "../game/animations.ts";
import { CATALOG } from "../wire/cards.ts";
import { frac, raysCue, ringCue, shakeCues, tid, tunedBurst } from "./build.ts";
import { entranceKeyOf } from "./cardFx.ts";
import { FX_CARD_TABLE, isSignature, type FxCardEntry } from "./cards/tables.ts";
import { FX_CENTER, FX_LEGENDARY_TRAUMA, FX_SLAM_AT, FX_TAKEOVER_TAIL_MS, FX_TAKEOVER_TRAUMA } from "./constants.ts";
import { entranceEvent } from "./entrances.ts";
import type { FxAnchor, FxCue, FxPlanEnv, FxTakeoverCue, FxTint, TakeoverLook } from "./types.ts";

/**
 * R670, R2722001, R3332000: a Mythic's takeover look: its table entry's `takeover` (#100 Ceaseless Void's
 * "void", #96 My Pawn's "pawn", #97 Zephrys's "genie", Meditative #49 YileGPT's "ai", R3389000 Classic #90
 * In Too Deep's "abyss"), else the shared "prismatic".
 */
export function takeoverLookOf(defId: string, table: ReadonlyMap<string, FxCardEntry> = FX_CARD_TABLE): TakeoverLook {
  const entry = table.get(defId);
  return (entry === undefined || entry.kind === "none" ? undefined : entry.takeover) ?? "prismatic";
}

/** R2722001: each look's colors, the sweep's rim, the light's core and the glow over the dark. */
export const TAKEOVER_TINTS: Readonly<Record<TakeoverLook, FxTint>> = {
  prismatic: { rim: "#f0abfc", core: "#fef9c3", glow: "#a5f3fc" },
  void: { rim: "#7c3aed", core: "#1e1b4b", glow: "#c4b5fd" },
  pawn: { rim: "#e5e7eb", core: "#fde68a", glow: "#f87171" },
  genie: { rim: "#fbbf24", core: "#fff7d6", glow: "#38bdf8" },
  ai: { rim: "#22c55e", core: "#dcfce7", glow: "#4ade80" },
  abyss: { rim: "#1e3a8a", core: "#0b1026", glow: "#7c3aed" },
};

/** Every burst an entrance throws, at intensity "normal" (rule 9). R2803000: the Mythic's eased for the phone frame budget. */
const TAKEOVER_TUNING = {
  legendaryBurst: { count: 34, power: 1.2 },
  mythicBurst: { count: 28, power: 1.4 },
} as const;

/** Where the takeover's two shockwaves cross the board from: the middle of the screen. */
const CENTER: FxAnchor = { kind: "viewport", at: { x: FX_CENTER.x, y: FX_CENTER.y } };
/** The second shockwave follows the first by this share of what is left of the entry after the landing. */
const SECOND_WAVE_AT = 0.3;

/**
 * R2803000: when a landing's echo follows it: a wave behind the landing, in the entry's own time.
 * The Mythic's second shockwave and the Legendary's echo share it, so both tiers land in one language.
 */
export function entranceEchoMs(D: number, landMs: number): number {
  return landMs + frac(SECOND_WAVE_AT, D - landMs);
}

/** R2722001: when in its entry an entrance's card lands: after the hold (and a slam's anticipation), at FX_SLAM_AT of the motion. */
export function entranceLandMs(entry: Pick<AnimationEntry, "slam" | "setPiece">, D: number): number {
  const wait = leadMs(entry);
  return wait + frac(FX_SLAM_AT, D - wait);
}

/**
 * R3332000: whether `entry` lands its entrance's Field Spell face up with an entrance of its own
 * (cardFx.ts plans it on the `summoned`), which stands in for the shared rays, burst and ring.
 */
function ownFieldLanding(entry: AnimationEntry, table: ReadonlyMap<string, FxCardEntry>): boolean {
  const entrance = entry.entrance;
  if (entrance === undefined || entrance.reveal || entranceKeyOf(entrance.defId, table) === undefined) return false;
  const type = (CATALOG[entrance.defId] ?? entry.view.defs?.[entrance.defId])?.type;
  if (type !== "Field Spell") return false;
  return entry.events.some((e) => entranceEvent(e, HIDDEN_ID, type)?.instanceId === entrance.instanceId);
}

/** R3389000: the card types whose signature `{ bespoke }` is a cast of its own in place of the shared entrance. */
const OWN_CAST_TYPES: readonly string[] = ["Spell", "Trap", "Field Trap"];

/**
 * R3389000: whether `defId` is a Spell, a Trap or a Field Trap whose signature is a `{ bespoke }` cast of
 * its own, which stands in for the shared Legendary or Mythic rays, burst and ring (its takeover stays).
 */
export function ownCast(defId: string, table: ReadonlyMap<string, FxCardEntry> = FX_CARD_TABLE, type: string | undefined = CATALOG[defId]?.type): boolean {
  const entry = table.get(defId);
  return isSignature(entry) && entry?.kind === "bespoke" && type !== undefined && OWN_CAST_TYPES.includes(type);
}

/** R2722001: the cues of `entry`'s entrance, or none: no entrance, or the effects off. */
export function entranceCues(entry: AnimationEntry, env: Pick<FxPlanEnv, "intensity">, D: number, table: ReadonlyMap<string, FxCardEntry> = FX_CARD_TABLE): FxCue[] {
  const entrance = entry.entrance;
  const i = env.intensity;
  if (entrance === undefined || !(i > 0)) return [];
  const mythic = entrance.tier === "mythic";
  const landMs = entranceLandMs(entry, D);
  const at: FxAnchor = entrance.anchor === null ? CENTER : tid(entrance.anchor);
  const cues: FxCue[] = [];
  // A Unit summoned face-up draws its rarity's rays, burst and shake in cues.ts `summon` already.
  const summoned = !entrance.reveal && entry.events.some((e) => e.type === "summoned" && e.row === "units" && e.instanceId === entrance.instanceId);
  const type = (CATALOG[entrance.defId] ?? entry.view.defs?.[entrance.defId])?.type;
  if (!summoned && !ownFieldLanding(entry, table) && !ownCast(entrance.defId, table, type)) {
    // R2803000: the Legendary lands in the same two-wave language as the Mythic's takeover below,
    // one volume down: its gold echo follows its ring a wave behind, with no takeover and no second shake.
    const echoMs = entranceEchoMs(D, landMs);
    cues.push(
      raysCue(D, mythic ? "mythic" : "legendary", at, 0),
      tunedBurst(i, mythic ? "prismatic" : "gold", at, "area", landMs, mythic ? TAKEOVER_TUNING.mythicBurst : TAKEOVER_TUNING.legendaryBurst),
      ringCue(D, mythic ? "prismatic" : "gold", at, landMs),
      ...(mythic ? [] : [ringCue(D, "gold", at, echoMs)]),
      ...shakeCues(i, FX_LEGENDARY_TRAUMA, landMs),
    );
  }
  // R2800000: a Mythic whose own wipe is the action's set piece yields its takeover to it.
  if (!mythic || entrance.yielded === true) return cues;
  const look = takeoverLookOf(entrance.defId, table);
  const takeover: FxTakeoverCue = { kind: "takeover", look, tint: TAKEOVER_TINTS[look], at, landMs, delayMs: 0, durationMs: D + FX_TAKEOVER_TAIL_MS };
  cues.push(
    takeover,
    ringCue(D, "prismatic", CENTER, landMs),
    ringCue(D, "prismatic", CENTER, entranceEchoMs(D, landMs)),
    ...shakeCues(i, FX_TAKEOVER_TRAUMA, landMs),
  );
  return cues;
}
