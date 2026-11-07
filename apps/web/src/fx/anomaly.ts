// R3337003: discovering an Anomaly is an event. An Anomaly's lay (a `boardReplaced` naming its code,
// R3337000) plays as one rift run (game/runs.ts `riftAt`), which the queue claims as the action's set
// piece (`anomalyClaim`, R2785000), and this file draws it inside the entry. Held in full, a rift tears
// down the middle of the caster's side, every card on it is pulled in as its `exiled` says, the rift
// flares white and the Anomaly's title card stamps across the screen (its name over its picture); as the
// hold ends the new board's cards fall out of the rift onto their zones as their `summoned` say, each as
// its board lands (asteroids tumble, drones and hulls power on, the outpost's lights come up). Every Cry
// the board fires plays its own entry after it. Big moments "short" draws only the title card and the
// landing, "off" the title card alone, and Reduce Motion one still title card. The board, its name and
// its zones are public (R202): no cue names a card's definition. Every cue starts inside its entry and
// ends within FX_MAX_TAIL_MS of it (R200), a `pull` and a `conceal` excepted, which the board's next
// view ends; the effects speed scales it with the entry (R201).

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { ANOMALIES, type AnomalyCode } from "../cards/anomalies.ts";
import { anomalyArtUri } from "../cards/art/svg.ts";
import { FLAVOR_FALLBACK } from "../cards/flavor.ts";
import { ANOMALY_RIFT_HOLD_MS, ANOMALY_SET_PIECE, SET_PIECE_SHORT_HOLD_MS, type AnimationEntry, type SetPieceClaim } from "../game/animations.ts";
import { testid } from "../game/contract.ts";
import type { AnomalyRift } from "../game/runs.ts";
import type { AuraLook } from "./aura/looks.ts";
import { frac, raysCue, ringCue, shakeCues, tid, tunedBurst } from "./build.ts";
import { FX_CENTER, FX_HOLD_MAX_MS, FX_MOTIF_CENTER_LANE } from "./constants.ts";
import { FLAVOR_LOOKS, paletteLook, type FlavorLook, type FxPalette } from "./flavors.ts";
import { takenOf, tintOf, type SetPiecePlan } from "./setPieces.ts";
import type { FxAnchor, FxAnomalyTitleCue, FxCue, FxPlanEnv, FxTint } from "./types.ts";

/** How an Anomaly's cards land out of the rift: asteroids tumble, drones and hulls power on, the outpost's lights come up. */
export type AnomalyLanding = "tumble" | "power" | "lights";

/** One Anomaly's look: the palette its set piece wears, how its cards land and the standing backdrop of its side (`fx/aura`). */
export type AnomalyFx = { palette: FxPalette; landing: AnomalyLanding; backdrop: AuraLook };

/** R3337003: every Anomaly's look, by its code. */
export const ANOMALY_FX: { readonly [C in AnomalyCode]: AnomalyFx } = {
  WH: { palette: "Tech/starfield", landing: "power", backdrop: "void" },
  BF: { palette: "Fire", landing: "power", backdrop: "ember" },
  AL: { palette: "Hustle/ore", landing: "tumble", backdrop: "drift" },
  AT: { palette: "Hustle/ore", landing: "tumble", backdrop: "drift" },
  SP: { palette: "Fire", landing: "power", backdrop: "toxic" },
  MT: { palette: "Hustle/ore", landing: "tumble", backdrop: "drift" },
  DH: { palette: "Tech/hologram", landing: "power", backdrop: "arcane" },
  FO: { palette: "Tech/laser", landing: "lights", backdrop: "holy" },
  AN: { palette: "Dark/blood", landing: "tumble", backdrop: "drift" },
  AA: { palette: "Hustle/ore", landing: "tumble", backdrop: "drift" },
};

/** R3337003: where in the hold the rift tears open, flares white (the cards it took gone) and the title card stamps. */
export const ANOMALY_TEAR_AT = 0.1;
export const ANOMALY_FLARE_AT = 0.55;
export const ANOMALY_STAMP_AT = 0.62;
/** R3337003: how far into the landing the title card lingers before it has faded. */
export const ANOMALY_TITLE_LINGER_AT = 0.3;
/** R3337003: the share of the landing its cards' flights start over, the most one waits after the one before, and each one's flight. */
export const ANOMALY_LAND_SPREAD = 0.5;
export const ANOMALY_LAND_STAGGER_MS = 60;
export const ANOMALY_LAND_FLIGHT = 0.45;
/** R3337003: the rift lasts its entry and this much more, inside FX_MAX_TAIL_MS (R200). */
export const ANOMALY_TAIL_MS = 600;
/** R3337003: how long the still title card stands under Reduce Motion. */
export const ANOMALY_STILL_MS = 1_600;
/** R3337003: the shake at the flare, and at each tumbling landing. */
export const ANOMALY_TRAUMA = 0.6;
export const ANOMALY_LAND_TRAUMA = 0.2;

/** R3337003: every burst the set piece throws, at intensity "normal" (rule 9); small, for the phone frame budget (R2803000). */
export const ANOMALY_TUNING = {
  flare: { count: 36, power: 1.4 },
  dust: { count: 6, power: 0.7 },
  spark: { count: 6, power: 0.9 },
  lights: { count: 6, power: 0.6 },
} as const;

/** The form a rift plays in: its hold in full, Big moments "short", or no hold ("off", or a second set piece in the action). */
export type AnomalyForm = "full" | "short" | "off";

/** R3337003: an Anomaly's rift claims its entry as the action's set piece. */
export function anomalyClaim(entry: AnimationEntry): SetPieceClaim | null {
  return entry.rift === undefined ? null : { key: ANOMALY_SET_PIECE, holdMs: ANOMALY_RIFT_HOLD_MS };
}

/** The hold the entry plays its rift under, and so its form. */
function formOf(entry: AnimationEntry): { H: number; form: AnomalyForm } {
  const H = entry.setPiece?.key === ANOMALY_SET_PIECE ? entry.setPiece.holdMs : 0;
  return { H, form: H <= 0 ? "off" : H <= SET_PIECE_SHORT_HOLD_MS ? "short" : "full" };
}

function lookOf(code: AnomalyCode): FlavorLook {
  return paletteLook(ANOMALY_FX[code].palette) ?? FLAVOR_LOOKS[FLAVOR_FALLBACK];
}

/** The middle of the caster's side: the rift's centre, and where the new board's cards fall from. */
function centreOf(rift: AnomalyRift): FxAnchor {
  return tid(testid.zone(rift.side, "units", FX_MOTIF_CENTER_LANE));
}

/** The title card of `code`, across the middle of the screen: its name over its picture. */
function titleCue(code: AnomalyCode, tint: FxTint, delayMs: number, durationMs: number): FxAnomalyTitleCue {
  const anomaly = ANOMALIES[code];
  return {
    kind: "anomalyTitle",
    at: { kind: "viewport", at: FX_CENTER },
    text: anomaly.name,
    picture: anomalyArtUri(code, anomaly.rich),
    tint,
    delayMs,
    durationMs,
  };
}

/** Each card of the new board out of the rift onto its zone, `start` on, inside the `M` after the hold. */
function landingCues(rift: AnomalyRift, look: FlavorLook, centre: FxAnchor, i: number, H: number, M: number, D: number): FxCue[] {
  const n = rift.landings.length;
  const stagger = n > 1 ? Math.min(ANOMALY_LAND_STAGGER_MS, (M * ANOMALY_LAND_SPREAD) / (n - 1)) : 0;
  const flight = frac(ANOMALY_LAND_FLIGHT, M);
  const landing = ANOMALY_FX[rift.code].landing;
  const cues: FxCue[] = [];
  rift.landings.forEach((card, k) => {
    const start = H + Math.round(k * stagger);
    const land = start + flight;
    const to = tid(card.zone);
    cues.push({ kind: "ghost", from: centre, to, delayMs: start, durationMs: flight, ...(landing === "tumble" ? { spin: true as const } : {}) });
    if (landing === "tumble") cues.push(tunedBurst(i, "dust", to, "area", land, ANOMALY_TUNING.dust), ...shakeCues(i, ANOMALY_LAND_TRAUMA, land));
    else if (landing === "power") cues.push(ringCue(D, "spark", to, land), { ...tunedBurst(i, "spark", to, "point", land, ANOMALY_TUNING.spark), colors: look.colors });
    else cues.push({ ...tunedBurst(i, "sparkle", to, "point", land, ANOMALY_TUNING.lights), colors: look.colors });
  });
  return cues;
}

/** The events the set piece draws in place of their own rows: the replacement, the cards going and the board's coming. */
function replacedBy(entry: AnimationEntry): ReadonlySet<GameEvent> {
  return new Set(entry.events.filter((event) => event.type === "boardReplaced" || event.type === "exiled" || event.type === "summoned"));
}

/**
 * R3337003: an Anomaly's set piece for a rift entry, and the events it draws in place of their own rows;
 * null for any other entry, the effects off or an entry of no duration (Reduce Motion, `planAnomalyStill`).
 */
export function planAnomaly(entry: AnimationEntry, view: PlayerView, env: Pick<FxPlanEnv, "intensity">): SetPiecePlan | null {
  const rift = entry.rift;
  const i = env.intensity;
  const D = entry.durationMs;
  if (rift === undefined || !(i > 0) || !(D > 0)) return null;
  const look = lookOf(rift.code);
  const tint = tintOf(look);
  const { H, form } = formOf(entry);
  const M = Math.max(0, D - H);
  const centre = centreOf(rift);
  const taken = takenOf(entry, view);
  const cues: FxCue[] = [];
  if (form === "off") {
    // No hold: the title card over the entry, the old side gone at once; the board's cards land as any summon does.
    cues.push(titleCue(rift.code, tint, 0, D));
    for (const card of taken) cues.push({ kind: "conceal", testid: card, mode: "now", delayMs: 0, durationMs: FX_HOLD_MAX_MS });
    return { cues, replaces: replacedBy(entry) };
  }
  const linger = H + frac(ANOMALY_TITLE_LINGER_AT, M);
  if (form === "full") {
    const tearMs = frac(ANOMALY_TEAR_AT, H);
    const flareMs = frac(ANOMALY_FLARE_AT, H);
    const stampMs = frac(ANOMALY_STAMP_AT, H);
    cues.push({ kind: "rift", at: centre, side: rift.side, tint, tearMs, flareMs, delayMs: 0, durationMs: D + ANOMALY_TAIL_MS });
    for (const card of taken) {
      cues.push(
        { kind: "pull", testid: card, to: centre, delayMs: 0, durationMs: FX_HOLD_MAX_MS },
        { kind: "conceal", testid: card, mode: "after", delayMs: flareMs, durationMs: FX_HOLD_MAX_MS },
      );
    }
    cues.push(
      raysCue(D, "radiant", centre, flareMs),
      { ...tunedBurst(i, "prismatic", centre, "area", flareMs, ANOMALY_TUNING.flare), colors: look.colors },
      ringCue(D, "void", centre, flareMs),
      ...shakeCues(i, ANOMALY_TRAUMA, flareMs),
      titleCue(rift.code, tint, stampMs, linger - stampMs),
    );
  } else {
    // "short": the title card over the hold, the old side gone at once, and the landing.
    cues.push(titleCue(rift.code, tint, 0, linger));
    for (const card of taken) cues.push({ kind: "conceal", testid: card, mode: "now", delayMs: 0, durationMs: FX_HOLD_MAX_MS });
  }
  cues.push(...landingCues(rift, look, centre, i, H, M, D));
  return { cues, replaces: replacedBy(entry) };
}

/**
 * R3337003: a rift under Reduce Motion (an entry of no duration): the Anomaly's title card, still. It
 * holds nothing: the queue has already moved on, as the Starship's still launch does (R2801000).
 */
export function planAnomalyStill(entry: AnimationEntry, _view: PlayerView, env: Pick<FxPlanEnv, "intensity">): FxCue[] {
  const rift = entry.rift;
  if (rift === undefined || !(env.intensity > 0)) return [];
  return [{ ...titleCue(rift.code, tintOf(lookOf(rift.code)), 0, ANOMALY_STILL_MS), still: true }];
}
