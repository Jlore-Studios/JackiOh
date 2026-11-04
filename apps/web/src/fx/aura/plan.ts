// R2787000: what wears an aura and how auras stack, read off the view the board is showing and nothing
// else (R202, CLAUDE.md rule 7). Pure: the layer (AuraLayer.tsx) feeds it views and a clock.
//
// A face-up Field Spell or Field Trap whose table entry has an `aura` wears it over its own zone, its
// lane, its side or the board; a face-down card, the controller's own unrevealed Field Trap and a Unit
// standing in a backrow zone wear none. A listed modifier wears the aura of the readable play that was
// resolving when it was added (`attributeModifiers`), since its view names no source (R169). An aura
// begins as it appears and ends as it leaves, and at most AURA_MAX_PER_ZONE cover a zone, nearest scope
// first and then newest.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { FLAVOR_FALLBACK } from "../../cards/flavor.ts";
import { laneZoneTestids } from "../../game/actions.ts";
import { LANES, sideView, testid, type Side } from "../../game/contract.ts";
import { CATALOG } from "../../wire/cards.ts";
import { FX_CARD_TABLE, type FxAuraSpec, type FxCardEntry } from "../cards/tables.ts";
import { AURA_FADE_IN_MS, AURA_FADE_OUT_MS, AURA_MAX_PER_ZONE, AURA_MOTES_PER_AURA, AURA_PARTICLE_SHARE, AURA_SOURCES_LIMIT } from "../constants.ts";
import { FLAVOR_LOOKS, paletteLook, type FlavorColors } from "../flavors.ts";
import { flavorLookOf } from "../flavorSource.ts";
import type { FxMemory } from "../types.ts";
import { AURA_SCOPES, isAuraFieldType, type AuraLook, type AuraScope } from "./looks.ts";

const HIDDEN_ID = "hidden";
const SIDES: readonly Side[] = ["you", "opponent"];

/** One aura as planned: what it covers and how it looks. `defId` is the planner's; it never reaches the page (R202). */
export type AuraSpec = {
  /** `field-<instanceId>` for a Field card, `mod-<player>-<modifierId>` for a modifier. */
  key: string;
  defId: string;
  instanceId: string;
  side: Side;
  /** The Field card's backrow lane; null for a modifier, whose `zone` or `lane` scope is its side. */
  lane: number | null;
  scope: AuraScope;
  look: AuraLook;
  colors: FlavorColors;
};

/** The card a modifier's aura is drawn from. */
export type AuraSource = { defId: string; instanceId: string };

/** Each modifier's source, keyed by `sourceKey`, oldest first. */
export type AuraSources = Map<string, AuraSource>;

export function sourceKey(player: PlayerId, modifierId: string): string {
  return `${player}:${modifierId}`;
}

export function auraTestid(key: string): string {
  return `aura-${key}`;
}

function specOf(key: string, defId: string, instanceId: string, side: Side, lane: number | null, aura: FxAuraSpec, view: PlayerView): AuraSpec {
  const look = (aura.palette !== undefined ? paletteLook(aura.palette) : undefined) ?? flavorLookOf(defId, instanceId, view) ?? FLAVOR_LOOKS[FLAVOR_FALLBACK];
  const colors = look.colors;
  const scope = lane === null && (aura.scope === "zone" || aura.scope === "lane") ? "side" : aura.scope;
  return { key, defId, instanceId, side, lane, scope, look: aura.look, colors };
}

function auraOf(table: ReadonlyMap<string, FxCardEntry>, defId: string): FxAuraSpec | undefined {
  const entry = table.get(defId);
  return entry?.kind === "hooks" ? entry.aura : undefined;
}

/** Every aura the view shows, your side first: its standing Field cards, then its attributed modifiers. */
export function planAuras(view: PlayerView, sources: ReadonlyMap<string, AuraSource>, table: ReadonlyMap<string, FxCardEntry> = FX_CARD_TABLE): AuraSpec[] {
  const out: AuraSpec[] = [];
  for (const side of SIDES) {
    const seat = sideView(view, side);
    seat.backrow.forEach((slot, index) => {
      if (slot === null || slot === undefined || slot.faceDown || slot.unrevealed === true || slot.unit !== undefined) return;
      if (slot.defId === HIDDEN_ID || !isAuraFieldType(slot.type)) return;
      const aura = auraOf(table, slot.defId);
      if (aura !== undefined) out.push(specOf(`field-${slot.instanceId}`, slot.defId, slot.instanceId, side, index + 1, aura, view));
    });
    for (const modifier of seat.modifiers) {
      const source = sources.get(sourceKey(seat.player, modifier.id));
      if (source === undefined) continue;
      // A Field card's aura stands with the card itself, not with what it installs.
      const type = (CATALOG[source.defId] ?? view.defs?.[source.defId])?.type;
      if (type !== undefined && isAuraFieldType(type)) continue;
      const aura = auraOf(table, source.defId);
      if (aura !== undefined) out.push(specOf(`mod-${seat.player}-${modifier.id}`, source.defId, source.instanceId, side, null, aura, view));
    }
  }
  return out;
}

/**
 * Reads `events` in order into `memory` and records, for each modifier added, the readable play
 * resolving as it was added: none for a hidden play (R202) or for one inside which a trap fired, since
 * the trap may be what added it, and none for a hook running inside or after it (R2797000: a Death, a
 * trigger, an Activate or a Trap's firing), whose card is no play. A removal keeps its record, so the
 * aura ends when the shown view drops the modifier, not when the newest view does; a later add
 * overwrites it.
 */
export function attributeModifiers(events: readonly GameEvent[], memory: FxMemory, sources: AuraSources): void {
  for (const event of events) {
    memory.remember([event]);
    if (event.type !== "modifierChanged" || !event.added) continue;
    const key = sourceKey(event.player, event.modifierId);
    const play = memory.resolving();
    sources.delete(key);
    if (play === undefined || play.hook !== undefined || play.defId === HIDDEN_ID || play.instanceId === HIDDEN_ID || (play.seen.trapFired ?? 0) > 0) continue;
    sources.set(key, { defId: play.defId, instanceId: play.instanceId });
    while (sources.size > AURA_SOURCES_LIMIT) {
      const oldest = sources.keys().next();
      if (oldest.done === true) break;
      sources.delete(oldest.value);
    }
  }
}

function sideZones(side: Side): string[] {
  return LANES.flatMap((lane) => [testid.zone(side, "units", lane), testid.zone(side, "backrow", lane)]);
}

/** The zones an aura covers, by testid. */
export function auraZones(spec: Pick<AuraSpec, "scope" | "side" | "lane">): string[] {
  if (spec.scope === "board") return [...sideZones("you"), ...sideZones("opponent")];
  if (spec.scope === "side" || spec.lane === null) return sideZones(spec.side);
  if (spec.scope === "lane") return laneZoneTestids(spec.lane);
  return [testid.zone(spec.side, "backrow", spec.lane)];
}

/** `begin` while it fades in with its burst, `standing` while it holds, `end` while it fades out. */
export type AuraPhase = "begin" | "standing" | "end";

/** `since` is when its phase began; `seen` orders arrivals, the newest highest. */
export type TrackedAura = { spec: AuraSpec; phase: AuraPhase; since: number; seen: number };

/** `started` once a first view has been planned: what that view already shows stands, without a begin. */
export type AuraTrack = { auras: readonly TrackedAura[]; seen: number; started: boolean };

export const NO_AURAS: AuraTrack = { auras: [], seen: 0, started: false };

/**
 * The track after the board shows a view whose auras are `planned`. A new aura begins (it stands on
 * the first view, or without `animate`), one that left ends (and is gone at once without `animate`),
 * and one that comes back while it fades begins again. Without `animate` nothing begins or ends.
 */
export function reconcileAuras(prev: AuraTrack, planned: readonly AuraSpec[], now: number, animate: boolean): AuraTrack {
  const byKey = new Map(planned.map((spec) => [spec.key, spec]));
  const kept = new Set<string>();
  const auras: TrackedAura[] = [];
  for (const aura of prev.auras) {
    const spec = byKey.get(aura.spec.key);
    if (spec !== undefined) {
      kept.add(spec.key);
      if (!animate) auras.push({ spec, phase: "standing", since: aura.phase === "standing" ? aura.since : now, seen: aura.seen });
      else if (aura.phase === "end") auras.push({ spec, phase: "begin", since: now, seen: aura.seen });
      else auras.push({ ...aura, spec });
    } else if (animate) {
      auras.push(aura.phase === "end" ? aura : { ...aura, phase: "end", since: now });
    }
  }
  let seen = prev.seen;
  for (const spec of planned) {
    if (kept.has(spec.key)) continue;
    kept.add(spec.key);
    seen += 1;
    auras.push({ spec, phase: animate && prev.started ? "begin" : "standing", since: now, seen });
  }
  return { auras, seen, started: true };
}

/** A begun aura stands once AURA_FADE_IN_MS has passed and an ended one is gone after AURA_FADE_OUT_MS; the same track when nothing changed. */
export function settleAuras(track: AuraTrack, now: number): AuraTrack {
  let changed = false;
  const auras: TrackedAura[] = [];
  for (const aura of track.auras) {
    if (aura.phase === "begin" && now - aura.since >= AURA_FADE_IN_MS) {
      auras.push({ ...aura, phase: "standing", since: now });
      changed = true;
    } else if (aura.phase === "end" && now - aura.since >= AURA_FADE_OUT_MS) {
      changed = true;
    } else {
      auras.push(aura);
    }
  }
  return changed ? { ...track, auras } : track;
}

/**
 * R2787000: the auras that show, most important first: those still holding before those fading out,
 * then the nearest scope (zone, lane, side, board), then the newest. Each is kept only while every
 * zone it covers holds fewer than `max` kept already.
 */
export function stackAuras(auras: readonly TrackedAura[], max: number = AURA_MAX_PER_ZONE): TrackedAura[] {
  const ranked = auras
    .map((aura, index) => ({ aura, index }))
    .sort(
      (a, b) =>
        Number(a.aura.phase === "end") - Number(b.aura.phase === "end") ||
        AURA_SCOPES.indexOf(a.aura.spec.scope) - AURA_SCOPES.indexOf(b.aura.spec.scope) ||
        b.aura.seen - a.aura.seen ||
        a.index - b.index,
    );
  const counts = new Map<string, number>();
  const shown: TrackedAura[] = [];
  for (const { aura } of ranked) {
    const zones = auraZones(aura.spec);
    if (zones.some((zone) => (counts.get(zone) ?? 0) >= max)) continue;
    for (const zone of zones) counts.set(zone, (counts.get(zone) ?? 0) + 1);
    shown.push(aura);
  }
  return shown;
}

/** Each of `n` auras' motes: AURA_MOTES_PER_AURA times the intensity, and all of them together at most AURA_PARTICLE_SHARE of `cap`. */
export function auraBudget(n: number, intensity: number, cap: number): number {
  if (n === 0) return 0;
  return Math.max(0, Math.min(Math.floor(AURA_MOTES_PER_AURA * intensity), Math.floor((AURA_PARTICLE_SHARE * cap) / n)));
}
