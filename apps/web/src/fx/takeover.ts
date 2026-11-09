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
// R3400000: so does a signature Spell's (or Trap's) cast of its own (`ownCast`): its recipe draws its play,
// and a Mythic's takeover keeps only its own look, centred on the screen and ending
// FX_SIGNATURE_TAKEOVER_TAIL_MS after its entry, with no shared rings or shake, so the whole cast fits the
// Mythic Spell's ceiling (R3332000). R3389000: In Too Deep's takeover is its own, "abyss": a spiral shaft
// plunging into violet dark.
// R3407000: a look of `TAKEOVER_ART_LOOKS` also draws its card's own picture filling the screen behind the
// sweep (`takeoverArtOf`, the cue's `art`), the picture the card's face shows; every other look draws none.

import { isStinkyscapeArt } from "../cards/art/stinkyscape.ts";
import { stinkyscapeArtUri } from "../cards/art/svg.ts";
import { HIDDEN_ID, leadMs, type AnimationEntry } from "../game/animations.ts";
import { ART_PLANES, artPlaneBackground } from "../cards/art/pictures.ts";
import { signatureFaceOf } from "../cards/signatureFaces.ts";
import { sideOf, testid } from "../game/contract.ts";
import { CATALOG } from "../wire/cards.ts";
import { frac, raysCue, ringCue, shakeCues, tid, tunedBurst } from "./build.ts";
import { entranceKeyOf } from "./cardFx.ts";
import { FX_CARD_TABLE, isSignature, type FxCardEntry } from "./cards/tables.ts";
import { FX_CENTER, FX_LEGENDARY_TRAUMA, FX_SIGNATURE_TAKEOVER_TAIL_MS, FX_SLAM_AT, FX_TAKEOVER_TAIL_MS, FX_TAKEOVER_TRAUMA } from "./constants.ts";
import { entranceEvent } from "./entrances.ts";
import type { FxAnchor, FxCue, FxPlanEnv, FxTakeoverCue, FxTint, TakeoverLook } from "./types.ts";

/**
 * R670, R2722001, R3332000: a Mythic's takeover look: its table entry's `takeover` (#100 Ceaseless Void's
 * "void", #96 My Pawn's "pawn", #97 Zephrys's "genie", Meditative #49 YileGPT's "ai"; R3400000 Meditative+
 * #1 Bob's Plan's "summit", #26 Scratch Pad's "canvas", Training Grounds #40 Goetzmann Game Design's
 * "napkin"; R3389000 Classic #90 In Too Deep's "abyss"; R3405000 Meditative's other Mythics' twelve, from
 * #8 Reach the Summit's "ascent" to #97.9 Jlockeed's Headquarters' "fortress"; R3399002 Classic+'s four
 * Mythics' "zealot", "portal", "stair" and "vineyard"; R3395001 Stinkyscape #6 Tylannt's "vault", #16 Rod of
 * Ages' "rod", #44 Great Paperclip Theory's "wire", #74 Paragon-X's "whiteDwarf", #86 Prototype-X2's "forge"),
 * else the shared "prismatic".
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
  // R3400000: Bob's red brushstroke over an ink-wash range, Scratch Pad's cyan cursor on paper, the napkin's lamplight in pixel colors.
  summit: { rim: "#dc2626", core: "#f5f5f4", glow: "#57534e" },
  canvas: { rim: "#22d3ee", core: "#fafaf9", glow: "#a8a29e" },
  napkin: { rim: "#e879f9", core: "#fef3c7", glow: "#22d3ee" },
  // R3389000: In Too Deep's black-blue water over violet dark.
  abyss: { rim: "#1e3a8a", core: "#0b1026", glow: "#7c3aed" },
  // R3407000: Stinkyscape's Mythics of part two, each its own.
  freshJack: { rim: "#2dd4bf", core: "#fdf4ff", glow: "#e879f9" },
  ikwzki: { rim: "#fb7185", core: "#fff1f2", glow: "#67e8f9" },
  shockwave: { rim: "#3b82f6", core: "#eff6ff", glow: "#60a5fa" },
  alteran: { rim: "#8b5cf6", core: "#ede9fe", glow: "#a78bfa" },
  edge: { rim: "#f8fafc", core: "#ffffff", glow: "#e2e8f0" },
  redNarcor: { rim: "#dc2626", core: "#fee2e2", glow: "#f87171" },
  // R3405000: Meditative's Mythics': the summit's starlit ink, the workshop's pixel blue, the brush's vermilion
  // on paper, the mine's red seam, the quarry's jade mist, the pavilion's ink and light, the virus's error red,
  // the dojo's hologram cyan, the study hall's white curve, the cane's red cord on lacquer, the war room's
  // blueprint under red lamps and the fortress's gunmetal and beacons.
  ascent: { rim: "#e0e7ff", core: "#f8fafc", glow: "#64748b" },
  workshop: { rim: "#38bdf8", core: "#e0f2fe", glow: "#1e3a8a" },
  brush: { rim: "#dc2626", core: "#fafaf9", glow: "#44403c" },
  mine: { rim: "#dc2626", core: "#fecaca", glow: "#7f1d1d" },
  quarry: { rim: "#34d399", core: "#ecfdf5", glow: "#f9a8d4" },
  pavilion: { rim: "#22d3ee", core: "#f5f5f4", glow: "#ef4444" },
  virus: { rim: "#ef4444", core: "#fde68a", glow: "#a3e635" },
  dojo: { rim: "#22d3ee", core: "#ecfeff", glow: "#a78bfa" },
  studyHall: { rim: "#f5f5f4", core: "#fafaf9", glow: "#78716c" },
  caneRack: { rim: "#b91c1c", core: "#fef3c7", glow: "#f5f5f4" },
  warRoom: { rim: "#22d3ee", core: "#e0f2fe", glow: "#dc2626" },
  fortress: { rim: "#ef4444", core: "#e5e7eb", glow: "#475569" },
  // R3399002: Classic+'s Mythics: Zephrys Zealotism's indigo cyclone, Portal to the Past's sepia vortex,
  // Twice Forward's violet stair and its cyan cracks, Mythic Grape's violet grape over its green vines.
  zealot: { rim: "#4f46e5", core: "#e0e7ff", glow: "#818cf8" },
  portal: { rim: "#a16207", core: "#fef3c7", glow: "#d6a35c" },
  stair: { rim: "#7c3aed", core: "#ede9fe", glow: "#22d3ee" },
  vineyard: { rim: "#9333ea", core: "#f3e8ff", glow: "#65a30d" },
  // R3395001: Tylannt's steel vault and its green gap, Rod of Ages' pale bands in an indigo gulf, the paperclip's silver wire
  // and blue cores, Paragon-X's white dwarf, Prototype-X2's molten forge.
  vault: { rim: "#94a3b8", core: "#e2e8f0", glow: "#22c55e" },
  rod: { rim: "#cbd5e1", core: "#f8fafc", glow: "#a5b4fc" },
  wire: { rim: "#d1d5db", core: "#f9fafb", glow: "#60a5fa" },
  whiteDwarf: { rim: "#f8fafc", core: "#ffffff", glow: "#7dd3fc" },
  forge: { rim: "#f97316", core: "#fed7aa", glow: "#ef4444" },
};

/** R3407000: the looks that draw their card's own picture filling the screen (`takeoverArtOf`). */
export const TAKEOVER_ART_LOOKS: ReadonlySet<TakeoverLook> = new Set<TakeoverLook>(["freshJack", "ikwzki", "shockwave", "alteran", "edge", "redNarcor"]);

/**
 * R3407000: the picture a Mythic's takeover draws over the dark, its card's own (the Stinkyscape painter's,
 * as its face shows it), when its look is one of `TAKEOVER_ART_LOOKS`; else none. It is only ever asked for
 * the card an entrance names, which the seat's own event names (R202).
 */
export function takeoverArtOf(defId: string, table: ReadonlyMap<string, FxCardEntry> = FX_CARD_TABLE): string | undefined {
  if (defId === HIDDEN_ID || !TAKEOVER_ART_LOOKS.has(takeoverLookOf(defId, table)) || !isStinkyscapeArt(defId)) return undefined;
  const def = CATALOG[defId];
  return def === undefined ? undefined : stinkyscapeArtUri(defId, def.name, def.tags, def.type, false);
}

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
 * R3332000: the zone `entry`'s entrance lands its Field Spell in face up with an entrance of its own
 * (cardFx.ts plans it on the `summoned`), which stands in for the shared rays, burst and ring; else null.
 */
function ownFieldLanding(entry: AnimationEntry, table: ReadonlyMap<string, FxCardEntry>): FxAnchor | null {
  const entrance = entry.entrance;
  if (entrance === undefined || entrance.reveal || entranceKeyOf(entrance.defId, table) === undefined) return null;
  const type = (CATALOG[entrance.defId] ?? entry.view.defs?.[entrance.defId])?.type;
  if (type !== "Field Spell") return null;
  for (const e of entry.events) {
    const landing = entranceEvent(e, HIDDEN_ID, type);
    if (landing?.instanceId === entrance.instanceId) return tid(testid.zone(sideOf(entry.view, landing.player), landing.row, landing.lane));
  }
  return null;
}

/**
 * R3400000: whether `defId`'s signature is a cast of its own: a signature entry (R3332000) of a card that
 * neither lands as a Unit nor as a Field Spell, so its recipe draws its play in place of the shared rays,
 * burst and rings.
 */
export function ownCast(defId: string, type: string | undefined, table: ReadonlyMap<string, FxCardEntry> = FX_CARD_TABLE): boolean {
  return isSignature(table.get(defId)) && type !== undefined && type !== "Unit" && type !== "Field Spell";
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
  // R3400000: a signature Spell's cast of its own draws its play itself.
  const own = ownCast(entrance.defId, (CATALOG[entrance.defId] ?? entry.view.defs?.[entrance.defId])?.type, table);
  // R3407000: such a Field Spell's Mythic takeover sweeps into the zone it lands in, not the hand card it left,
  // which the board no longer shows (and the other seat never did).
  const field = ownFieldLanding(entry, table);
  if (!summoned && !own && field === null) {
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
  const defId = entrance.defId;
  const name = (CATALOG[defId] ?? entry.view.defs?.[defId])?.name ?? defId;
  const radiant = entry.events.some((e) => "radiant" in e && (e as { radiant?: boolean }).radiant === true);
  const look = takeoverLookOf(defId, table);
  const tail = own ? FX_SIGNATURE_TAKEOVER_TAIL_MS : FX_TAKEOVER_TAIL_MS;
  // R3400000: a signature Spell's own takeover fills the screen from its middle, as its recipe draws at the board's centre.
  const art = takeoverArtOf(entrance.defId, table);
  const takeover: FxTakeoverCue = {
    kind: "takeover",
    stage: entry.setPiece === undefined ? null : { planes: ART_PLANES.map((plane) => artPlaneBackground(defId, plane, radiant)), name, face: signatureFaceOf(defId) },
    look,
    tint: TAKEOVER_TINTS[look],
    at: own ? CENTER : (field ?? at),
    landMs,
    delayMs: 0,
    durationMs: D + tail,
    ...(art === undefined ? {} : { art }),
  };
  if (own) return [...cues, takeover];
  cues.push(
    takeover,
    ringCue(D, "prismatic", CENTER, landMs),
    ringCue(D, "prismatic", CENTER, entranceEchoMs(D, landMs)),
    ...shakeCues(i, FX_TAKEOVER_TRAUMA, landMs),
  );
  return cues;
}
