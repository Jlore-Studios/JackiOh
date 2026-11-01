// R2722001: a Legendary's or a Mythic's entrance (animations.ts `entranceEntries`), and a Mythic's
// takeover of the whole board, after Hearthstone's Deathwing.
//
// The queue gives a readable Mythic's entry the takeover's hold (`TAKEOVER_HOLD_MS`, one per action),
// so the board can darken and a card-shaped silhouette sweep across it before the card lands at
// `entranceLandMs`; there the rays turn, a shockwave crosses both sides and the board shakes as hard
// as it ever does. Everything here is cue data built as cues.ts builds its own: it starts inside the
// entry and is gone within FX_MAX_TAIL_MS of its end (R200), it keys only on the entry's `entrance`,
// which names a card the seat's own event names (R202), and the effects off or reduced motion draws
// none of it (planFx is never asked). The marquee Mythics take a look of their own (R670), the rest the
// shared prismatic one. A Legendary takes no takeover; its entrance is the rays, a gold burst and ring
// and FX_LEGENDARY_TRAUMA, drawn here when no summon of it draws them already (a Spell's cast, a reveal).

import { leadMs, type AnimationEntry } from "../game/animations.ts";
import { frac, raysCue, ringCue, shakeCues, tid, tunedBurst } from "./build.ts";
import { FX_CENTER, FX_LEGENDARY_TRAUMA, FX_SLAM_AT, FX_TAKEOVER_TAIL_MS, FX_TAKEOVER_TRAUMA } from "./constants.ts";
import type { FxAnchor, FxCue, FxPlanEnv, FxTakeoverCue, FxTint, TakeoverLook } from "./types.ts";

/** R670, R2722001: the marquee Mythics' own takeovers; every other Mythic takes the shared "prismatic". */
export const TAKEOVER_LOOKS: Readonly<Record<string, TakeoverLook>> = {
  "core-100": "void", // #100 Ceaseless Void: the board falls into the dark it leaves
  "core-096": "pawn", // #96 My Pawn: a hand of cards dealt across the table
  "core-097": "genie", // #97 Zephrys: the west wind's gold
  "meditative-049": "ai", // Meditative #49 YileGPT: a terminal's green
};

/** R2722001: each look's colors, the sweep's rim, the light's core and the glow over the dark. */
export const TAKEOVER_TINTS: Readonly<Record<TakeoverLook, FxTint>> = {
  prismatic: { rim: "#f0abfc", core: "#fef9c3", glow: "#a5f3fc" },
  void: { rim: "#7c3aed", core: "#1e1b4b", glow: "#c4b5fd" },
  pawn: { rim: "#e5e7eb", core: "#fde68a", glow: "#f87171" },
  genie: { rim: "#fbbf24", core: "#fff7d6", glow: "#38bdf8" },
  ai: { rim: "#22c55e", core: "#dcfce7", glow: "#4ade80" },
};

/** Every burst an entrance throws, at intensity "normal" (rule 9). */
const TAKEOVER_TUNING = {
  legendaryBurst: { count: 34, power: 1.2 },
  mythicBurst: { count: 48, power: 1.4 },
} as const;

/** Where the takeover's two shockwaves cross the board from: the middle of the screen. */
const CENTER: FxAnchor = { kind: "viewport", at: { x: FX_CENTER.x, y: FX_CENTER.y } };
/** The second shockwave follows the first by this share of what is left of the entry after the landing. */
const SECOND_WAVE_AT = 0.3;

/** R2722001: when in its entry an entrance's card lands: after the hold (and a slam's anticipation), at FX_SLAM_AT of the motion. */
export function entranceLandMs(entry: Pick<AnimationEntry, "slam" | "entrance">, D: number): number {
  const wait = leadMs(entry);
  return wait + frac(FX_SLAM_AT, D - wait);
}

/** R2722001: the cues of `entry`'s entrance, or none: no entrance, or the effects off. */
export function entranceCues(entry: AnimationEntry, env: Pick<FxPlanEnv, "intensity">, D: number): FxCue[] {
  const entrance = entry.entrance;
  const i = env.intensity;
  if (entrance === undefined || !(i > 0)) return [];
  const mythic = entrance.tier === "mythic";
  const landMs = entranceLandMs(entry, D);
  const at: FxAnchor = entrance.anchor === null ? CENTER : tid(entrance.anchor);
  const cues: FxCue[] = [];
  // A Unit summoned face-up draws its rarity's rays, burst and shake in cues.ts `summon` already.
  const summoned = !entrance.reveal && entry.events.some((e) => e.type === "summoned" && e.row === "units" && e.instanceId === entrance.instanceId);
  if (!summoned) {
    cues.push(
      raysCue(D, mythic ? "mythic" : "legendary", at, 0),
      tunedBurst(i, mythic ? "prismatic" : "gold", at, "area", landMs, mythic ? TAKEOVER_TUNING.mythicBurst : TAKEOVER_TUNING.legendaryBurst),
      ringCue(D, mythic ? "prismatic" : "gold", at, landMs),
      ...shakeCues(i, FX_LEGENDARY_TRAUMA, landMs),
    );
  }
  if (!mythic) return cues;
  const look = TAKEOVER_LOOKS[entrance.defId] ?? "prismatic";
  const takeover: FxTakeoverCue = { kind: "takeover", look, tint: TAKEOVER_TINTS[look], at, landMs, delayMs: 0, durationMs: D + FX_TAKEOVER_TAIL_MS };
  cues.push(
    takeover,
    ringCue(D, "prismatic", CENTER, landMs),
    ringCue(D, "prismatic", CENTER, landMs + frac(SECOND_WAVE_AT, D - landMs)),
    ...shakeCues(i, FX_TAKEOVER_TRAUMA, landMs),
  );
  return cues;
}
