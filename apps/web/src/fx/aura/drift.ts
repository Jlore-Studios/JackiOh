// R2787000: the motes of every aura that shows, drawn on the aura layer's one canvas. Each aura keeps
// exactly its budget of motes inside its box: a begun aura's burst out of the box's centre, a standing
// one's at random places and ages, each drifting by its look, wrapping at the box's edges and respawning
// in place when its life runs out. An ended aura's motes fade over AURA_FADE_OUT_MS. Every random draw
// comes from the injected `FxRng`, so one seed gives the same motes.

import { AURA_BURST_DECAY, AURA_BURST_SPEED, AURA_FADE_OUT_MS } from "../constants.ts";
import type { FlavorColors } from "../flavors.ts";
import type { FxRng } from "../rng.ts";
import type { FxBox } from "../types.ts";
import { auraLookSpec, type AuraLook } from "./looks.ts";
import type { AuraPhase } from "./plan.ts";

const MS_PER_SECOND = 1000;
const TAU = Math.PI * 2;

/** One aura as the drift draws it. */
export type AuraDriftItem = { key: string; box: FxBox; look: AuraLook; colors: FlavorColors; budget: number; phase: AuraPhase };

type Mote = { x: number; y: number; vx: number; vy: number; bx: number; by: number; size: number; age: number; life: number; color: string };

type Drifting = { item: AuraDriftItem; motes: Mote[]; endingMs: number };

export type AuraMote = { x: number; y: number; age: number; life: number };

export type AuraDrift = {
  /** Takes the auras that show now: drops the gone, starts the new, and trims or tops each up to its budget. */
  sync(items: readonly AuraDriftItem[]): void;
  /** Moves and ages every mote by `dtMs`. */
  step(dtMs: number): void;
  draw(ctx: CanvasRenderingContext2D): void;
  /** Live motes: of `key`, or of every aura. */
  count(key?: string): number;
  /** The motes of `key`, or of every aura, for tests. */
  motes(key?: string): AuraMote[];
  /** How many auras it draws. */
  size(): number;
  clear(): void;
};

export function createAuraDrift(options: { rng: FxRng }): AuraDrift {
  const { rng } = options;
  const auras = new Map<string, Drifting>();

  const between = (range: readonly [number, number]): number => range[0] + (range[1] - range[0]) * rng();

  /** A mote of `item` at a random place in its box (or its centre, bursting), at `age`. */
  const spawn = (item: AuraDriftItem, burst: boolean, age: number | null): Mote => {
    const look = auraLookSpec(item.look);
    const { box } = item;
    const life = between(look.life);
    const angle = rng() * TAU;
    return {
      x: burst ? box.x + box.width / 2 : box.x + rng() * box.width,
      y: burst ? box.y + box.height / 2 : box.y + rng() * box.height,
      vx: between(look.vx),
      vy: between(look.vy),
      bx: burst ? Math.cos(angle) * AURA_BURST_SPEED : 0,
      by: burst ? Math.sin(angle) * AURA_BURST_SPEED : 0,
      size: between(look.size),
      age: age ?? rng() * life,
      life,
      color: item.colors[Math.floor(rng() * item.colors.length)] ?? item.colors[0],
    };
  };

  /** Back inside [start, start + extent). */
  const wrap = (value: number, start: number, extent: number): number => {
    if (extent <= 0) return start;
    const offset = (value - start) % extent;
    const inside = offset < 0 ? offset + extent : offset;
    // A tiny negative offset plus the extent can round up to the extent itself.
    return inside >= extent ? start : start + inside;
  };

  const fadeOf = (aura: Drifting): number => (aura.item.phase === "end" ? Math.max(0, 1 - aura.endingMs / AURA_FADE_OUT_MS) : 1);

  const all = (key?: string): Drifting[] => {
    if (key === undefined) return [...auras.values()];
    const one = auras.get(key);
    return one === undefined ? [] : [one];
  };

  return {
    sync(items) {
      const keys = new Set(items.map((item) => item.key));
      for (const key of [...auras.keys()]) if (!keys.has(key)) auras.delete(key);
      for (const item of items) {
        const budget = Math.max(0, Math.floor(item.budget));
        let aura = auras.get(item.key);
        if (aura === undefined) {
          const burst = item.phase === "begin";
          aura = { item, motes: Array.from({ length: budget }, () => spawn(item, burst, burst ? 0 : null)), endingMs: 0 };
          auras.set(item.key, aura);
          continue;
        }
        // An aura's fade-out runs from the sync that first sees it end.
        if (item.phase !== "end" || aura.item.phase !== "end") aura.endingMs = 0;
        aura.item = item;
        if (aura.motes.length > budget) aura.motes.length = budget;
        while (aura.motes.length < budget) aura.motes.push(spawn(item, false, null));
      }
    },
    step(dtMs) {
      const dts = Math.max(0, dtMs) / MS_PER_SECOND;
      const decay = Math.exp(-AURA_BURST_DECAY * dts);
      for (const aura of auras.values()) {
        const { item } = aura;
        const { box } = item;
        if (item.phase === "end") aura.endingMs += Math.max(0, dtMs);
        aura.motes.forEach((mote, i) => {
          mote.x = wrap(mote.x + (mote.vx + mote.bx) * dts, box.x, box.width);
          mote.y = wrap(mote.y + (mote.vy + mote.by) * dts, box.y, box.height);
          mote.bx *= decay;
          mote.by *= decay;
          mote.age += Math.max(0, dtMs);
          if (mote.age >= mote.life) aura.motes[i] = spawn(item, false, 0);
        });
      }
    },
    draw(ctx) {
      const alpha = ctx.globalAlpha;
      const blend = ctx.globalCompositeOperation;
      for (const aura of auras.values()) {
        const look = auraLookSpec(aura.item.look);
        const fade = fadeOf(aura);
        if (fade <= 0) continue;
        ctx.globalCompositeOperation = look.blend;
        for (const mote of aura.motes) {
          const a = look.alpha * Math.sin((Math.PI * mote.age) / mote.life) * fade;
          if (!(a > 0)) continue;
          ctx.globalAlpha = Math.min(1, a);
          ctx.fillStyle = mote.color;
          ctx.beginPath();
          ctx.arc(mote.x, mote.y, mote.size / 2, 0, TAU);
          ctx.fill();
        }
      }
      ctx.globalAlpha = alpha;
      ctx.globalCompositeOperation = blend;
    },
    count(key) {
      return all(key).reduce((sum, aura) => sum + aura.motes.length, 0);
    },
    motes(key) {
      return all(key).flatMap((aura) => aura.motes.map((mote) => ({ x: mote.x, y: mote.y, age: mote.age, life: mote.life })));
    },
    size: () => auras.size,
    clear() {
      auras.clear();
    },
  };
}
