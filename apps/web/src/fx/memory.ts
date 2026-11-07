// What the cue planner remembers across the entries of one FxLayer mount (docs/polish/1-animations.md
// S6): who played which card, which zone a trap fired from, the last few events (so a `cardPlayed` right
// after its own `drawn` is a cast on draw, `castOnDraw.ts`) and the plays still resolving, innermost
// last, for `cardFx.ts`'s recipes (#21 Hinder, #27 Blood Ridden) and motifs (R2784000, with the ids its
// hits landed on): a `cardPlayed` opens one, its `cardResolved` or `countered` closes it (R502). R2797000:
// a card's hook is an entry too, innermost while it runs: a Death, a turn hook or a Whenever (`trigger`)
// and an Activate open one at their `hookResolving` marker (R2783000; a Cry's opens none, its play is the
// entry), a Trap's `cast` at its `trapFired`; the next marker or firing, a `cardResolved` or `countered`,
// an attack, a turn's end or start and the game's end close it. R202: only ids the viewer may read are
// kept, so every hidden card looks the same to the planner. R3338000: a Terminus card that owes its
// controller a turn (Event Horizon, Last Word: a `modifierChanged` added as it resolves) is owed its
// follow-up, which the next turn that player starts opens (`opening`), and `openPlay` finds the outermost
// play of a card still resolving (The Big Crunch's, around its casts).

import type { GameEvent, PlayerId } from "@jackioh/shared";

import { castOnDrawAt } from "./castOnDraw.ts";
import { FX_MEMORY_HITS, FX_MEMORY_LIMIT, FX_MEMORY_RECENT, FX_MEMORY_RESOLVING } from "./constants.ts";
import type { FxCardHook } from "./motifs/names.ts";
import { TURN_FOLLOWS } from "./terminus/ids.ts";
import type { FxMemory, FxPlay, FxTrapZone } from "./types.ts";

const HIDDEN_ID = "hidden";

/**
 * R2797000: the table hook a `hookResolving` marker opens an entry for: none for a Cry, which runs inside
 * its card's own play (a Spell's effect is one), so its play keys it by `cast` or `cry`.
 */
const HOOK_OF: Readonly<Record<Extract<GameEvent, { type: "hookResolving" }>["hook"], FxCardHook | null>> = {
  cry: null,
  death: "death",
  startOfTurn: "trigger",
  endOfTurn: "trigger",
  whenever: "trigger",
  activate: "activate",
};

/** R2797000: the events after which no hook entry still runs: an attack, a turn's end or start, the game's end. */
const HOOK_CLOSERS: ReadonlySet<GameEvent["type"]> = new Set(["attackDeclared", "turnEnded", "turnAutoEnded", "turnStarted", "gameOver"]);

/** R2784000: the public id a hit lands on (a damage's target, a card destroyed, exiled or bounced), else null. */
export function hitIdOf(event: GameEvent): string | null {
  const id =
    event.type === "damage"
      ? event.targetId
      : event.type === "destroyed" || event.type === "exiled" || event.type === "bounced"
        ? event.instanceId
        : null;
  return id === null || id === HIDDEN_ID ? null : id;
}

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
  // R2789000: the targets each readable attacker declared since the turn began (Windfury's second attack, Cleave's origin).
  const attacks = new Map<string, string[]>();
  let recent: GameEvent[] = [];
  let resolving: (FxPlay & { seen: Partial<Record<GameEvent["type"], number>> })[] = [];
  let castOnDraw = new WeakSet<GameEvent>();
  // R3338000: the Terminus card each player is owed a turn by, and the one whose turn the last `remember` opened.
  const owed = new Map<PlayerId, string>();
  let opening: { defId: string; player: PlayerId } | undefined;

  /** R2797000: ends every hook entry; plays stay open. */
  const dropHooks = (): void => {
    resolving = resolving.filter((play) => play.hook === undefined);
  };

  /** R2797000: opens a hook's entry, innermost, so the hook's events are its own and not the play around it. */
  const openHook = (player: PlayerId, instanceId: string, defId: string, hook: FxCardHook): void => {
    resolving.push({ player, instanceId, defId, castOnDraw: false, step: 0, seen: {}, hook });
    if (resolving.length > FX_MEMORY_RESOLVING) resolving = resolving.slice(resolving.length - FX_MEMORY_RESOLVING);
  };

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
      opening = undefined;
      for (const event of events) {
        const hit = hitIdOf(event);
        if (event.type === "modifierChanged" && event.added) {
          // The innermost card resolving owes its controller the turn (the modifier may be the other player's: Last Word ends theirs).
          const owner = resolving[resolving.length - 1];
          if (owner !== undefined && TURN_FOLLOWS.has(owner.defId)) owed.set(owner.player, owner.defId);
        }
        for (const play of resolving) {
          play.step += 1;
          play.seen[event.type] = (play.seen[event.type] ?? 0) + 1;
          if (hit !== null) play.hits = [...(play.hits ?? []), hit].slice(-FX_MEMORY_HITS);
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
          dropHooks();
          close(event.player, event.instanceId);
        } else if (event.type === "hookResolving") {
          dropHooks();
          // A hidden card's marker opens its entry all the same, so its events go to no play around it (R202).
          const hook = HOOK_OF[event.hook];
          if (hook !== null) openHook(event.controller, event.sourceId, event.defId, hook);
        } else if (event.type === "trapFired") {
          if (event.instanceId !== HIDDEN_ID) {
            put(traps, event.instanceId, { player: event.controller, row: event.row, lane: event.lane }, limit);
          }
          dropHooks();
          openHook(event.controller, event.instanceId, event.defId, "cast");
        } else if (HOOK_CLOSERS.has(event.type)) {
          dropHooks();
          if (event.type === "attackDeclared") {
            if (event.attackerId !== HIDDEN_ID) put(attacks, event.attackerId, [...(attacks.get(event.attackerId) ?? []), event.targetId], limit);
          } else if (event.type === "turnStarted") {
            attacks.clear();
            const follow = owed.get(event.player);
            if (follow !== undefined) {
              owed.delete(event.player);
              opening = { defId: follow, player: event.player };
            }
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
      if (play === undefined) return undefined;
      return { ...play, seen: { ...play.seen }, ...(play.hits === undefined ? {} : { hits: [...play.hits] }) };
    },
    castOnDraw(event: GameEvent): boolean {
      return castOnDraw.has(event);
    },
    recent(): readonly GameEvent[] {
      return [...recent];
    },
    attacksThisTurn(attackerId: string): readonly string[] {
      return [...(attacks.get(attackerId) ?? [])];
    },
    opening() {
      return opening === undefined ? undefined : { ...opening };
    },
    openPlay(defId: string): FxPlay | undefined {
      const play = resolving.find((open) => open.hook === undefined && open.defId === defId);
      if (play === undefined) return undefined;
      return { ...play, seen: { ...play.seen }, ...(play.hits === undefined ? {} : { hits: [...play.hits] }) };
    },
    clear(): void {
      casters.clear();
      traps.clear();
      attacks.clear();
      owed.clear();
      opening = undefined;
      recent = [];
      resolving = [];
      castOnDraw = new WeakSet<GameEvent>();
    },
  };
}
