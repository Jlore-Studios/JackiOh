// What the cue planner remembers across the entries of one FxLayer mount (docs/polish/1-animations.md
// S6): who played which card, and which zone a trap fired from. A spell's `damage` names only its
// `sourceId`, and by the time it lands the spell may be rendered nowhere, so the planner falls back to
// the zone the trap flipped in or the hero of the player who cast it.
//
// R502 adds two things, because the runner plans one entry (usually one event) at a time and a
// flourish sometimes needs the events around it:
// - the last few events, in order, so a `cardPlayed` that comes right after its own `drawn` is known
//   to be a cast on draw (`castOnDraw.ts`), even though the two are separate entries;
// - the plays still resolving, innermost last: a `cardPlayed` opens one, its `cardResolved` (or its
//   `countered`) closes it. A per-card recipe (`cardFx.ts`) decorates the events inside its card's
//   own resolution (#21 Hinder's mana loss, #27 Blood Ridden's Radiant pick and its blood price), and
//   each play counts the events it has seen, in all and by type (Crushing Walls' first `destroyed`).
//
// R202: only ids the viewer may read are kept as ids. An event redacted to the "hidden" sentinel is
// ignored by the id maps, and a hidden play is kept as a hidden play, so every hidden card looks the
// same to the planner whatever it hides.
//
// Each map keeps at most `limit` entries in insertion order. Remembering an id again refreshes it, and
// the oldest entry is evicted first once the map is full.

import type { GameEvent, PlayerId } from "@jackioh/shared";

import { castOnDrawAt } from "./castOnDraw.ts";
import { FX_MEMORY_LIMIT, FX_MEMORY_RECENT, FX_MEMORY_RESOLVING } from "./constants.ts";
import type { FxMemory, FxPlay, FxTrapZone } from "./types.ts";

const HIDDEN_ID = "hidden";

function put<V>(map: Map<string, V>, key: string, value: V, limit: number): void {
  map.delete(key);
  map.set(key, value);
  while (map.size > limit) {
    const oldest = map.keys().next();
    if (oldest.done === true) return;
    map.delete(oldest.value);
  }
}

export function createFxMemory(limit: number = FX_MEMORY_LIMIT): FxMemory {
  const casters = new Map<string, PlayerId>();
  const traps = new Map<string, FxTrapZone>();
  let recent: GameEvent[] = [];
  let resolving: (FxPlay & { seen: Partial<Record<GameEvent["type"], number>> })[] = [];
  let castOnDraw = new WeakSet<GameEvent>();

  /** Closes the innermost play of `player` with this id (the sentinel matches the innermost of `player`). */
  const close = (player: PlayerId, instanceId: string): void => {
    for (let i = resolving.length - 1; i >= 0; i -= 1) {
      const play = resolving[i];
      if (play === undefined || play.player !== player) continue;
      if (play.instanceId === instanceId || play.instanceId === HIDDEN_ID || instanceId === HIDDEN_ID) {
        resolving.splice(i, 1);
        return;
      }
    }
  };

  return {
    remember(events: readonly GameEvent[]): void {
      for (const event of events) {
        for (const play of resolving) {
          play.step += 1;
          play.seen[event.type] = (play.seen[event.type] ?? 0) + 1;
        }
        recent.push(event);
        if (recent.length > FX_MEMORY_RECENT) recent = recent.slice(recent.length - FX_MEMORY_RECENT);
        if (event.type === "cardPlayed") {
          if (event.instanceId !== HIDDEN_ID) put(casters, event.instanceId, event.player, limit);
          const onDraw = castOnDrawAt(recent, recent.length - 1);
          if (onDraw) castOnDraw.add(event);
          resolving.push({ player: event.player, instanceId: event.instanceId, defId: event.defId, castOnDraw: onDraw, step: 0, seen: {} });
          if (resolving.length > FX_MEMORY_RESOLVING) resolving = resolving.slice(resolving.length - FX_MEMORY_RESOLVING);
        } else if (event.type === "cardResolved" || event.type === "countered") {
          close(event.player, event.instanceId);
        } else if (event.type === "trapFired") {
          if (event.instanceId !== HIDDEN_ID) {
            put(traps, event.instanceId, { player: event.controller, row: event.row, lane: event.lane }, limit);
          }
        }
      }
    },
    casterOf(instanceId: string): PlayerId | undefined {
      return casters.get(instanceId);
    },
    trapZoneOf(instanceId: string): FxTrapZone | undefined {
      const zone = traps.get(instanceId);
      return zone === undefined ? undefined : { player: zone.player, row: zone.row, lane: zone.lane };
    },
    resolving(): FxPlay | undefined {
      const play = resolving[resolving.length - 1];
      return play === undefined ? undefined : { ...play, seen: { ...play.seen } };
    },
    castOnDraw(event: GameEvent): boolean {
      return castOnDraw.has(event);
    },
    clear(): void {
      casters.clear();
      traps.clear();
      recent = [];
      resolving = [];
      castOnDraw = new WeakSet<GameEvent>();
    },
  };
}
