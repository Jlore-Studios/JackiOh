// #1442: the four motions of a flavor's shot (`shots.ts`), through `canvasFx.shot` as the director
// calls it. A shot paints while it flies, ends exactly with its flight, and bursts its preset where
// it lands (R200); it never blurs, writes text or draws an image (B27).

import { describe, expect, it } from "vitest";

import { createCanvasFx } from "./canvasFx.ts";
import { FLAVOR_LOOKS } from "./flavors.ts";
import { createParticleSystem } from "./particles.ts";
import { createRng } from "./rng.ts";
import type { FxShot, FxVec } from "./types.ts";

/** A 2D context that accepts any call and records the method names it was sent. */
function recordingContext() {
  const calls: string[] = [];
  const gradient = { addColorStop: () => {} };
  const state: Record<string | symbol, unknown> = {
    globalAlpha: 1,
    globalCompositeOperation: "source-over",
    fillStyle: "#000",
    strokeStyle: "#000",
    lineWidth: 1,
    lineCap: "butt",
    lineJoin: "miter",
    shadowBlur: 0,
    filter: "none",
    canvas: { width: 1280, height: 720 },
  };
  const ctx = new Proxy(state, {
    get(target, prop) {
      if (prop in target) return target[prop];
      if (typeof prop === "symbol") return undefined;
      return (..._args: unknown[]) => {
        calls.push(prop);
        if (prop === "createRadialGradient" || prop === "createLinearGradient" || prop === "createConicGradient") return gradient;
        if (prop === "getTransform") return { a: 1, b: 0, c: 0, d: 1, e: 0, f: 0 };
        if (prop === "measureText") return { width: 0 };
        return undefined;
      };
    },
    set(target, prop, value) {
      target[prop] = value;
      return true;
    },
  }) as unknown as CanvasRenderingContext2D;
  return { ctx, calls, state };
}

const PAINTS = new Set(["stroke", "fill"]);

function setup(capacity = 4_000) {
  const particles = createParticleSystem({ capacity, rng: createRng(1) });
  const fx = createCanvasFx({ particles, rng: createRng(2) });
  return { particles, fx };
}

const FROM: FxVec = { x: 100, y: 500 };
const TO: FxVec = { x: 700, y: 200 };
const FLIGHT = 300;

const LOOKS = Object.entries(FLAVOR_LOOKS) as [string, (typeof FLAVOR_LOOKS)[keyof typeof FLAVOR_LOOKS]][];

describe("#1442 flavor shots", () => {
  it("#1442 every flavor's shot paints mid-flight and ends with its flight", () => {
    for (const [flavor, look] of LOOKS) {
      const { particles, fx } = setup();
      fx.shot(look.projectile, look.colors, look.particle, FROM, TO, FLIGHT, 1);
      expect(fx.alive(), flavor).toBe(1);

      fx.step(150);
      expect(fx.alive(), flavor).toBe(1);
      const mid = recordingContext();
      fx.draw(mid.ctx);
      expect(mid.calls.some((call) => PAINTS.has(call)), flavor).toBe(true);
      // The canvas is left as the next effect expects it.
      expect(mid.state.globalAlpha, flavor).toBe(1);
      expect(mid.state.globalCompositeOperation, flavor).toBe("source-over");

      fx.step(170);
      expect(fx.alive(), flavor).toBe(0);
      expect(particles.alive(), flavor).toBeGreaterThan(0);
      const after = recordingContext();
      fx.draw(after.ctx);
      expect(after.calls, flavor).toEqual([]);
    }
  });

  it("#1442 a shot bursts its preset where it lands", () => {
    const { particles, fx } = setup();
    const look = FLAVOR_LOOKS.Hustle;
    fx.shot({ motion: "beam" }, look.colors, "gold", FROM, TO, FLIGHT);
    fx.step(FLIGHT);
    expect(fx.alive()).toBe(0);
    const burst = particles.inspect();
    expect(burst.length).toBeGreaterThan(0);
    expect(burst.every((p) => p.preset === "gold")).toBe(true);
    for (const p of burst) expect(Math.hypot(p.x - TO.x, p.y - TO.y)).toBeLessThan(40);
  });

  it("#1442 a bolt leaves its trail, and an arc only when it has one", () => {
    const bolt = setup();
    bolt.fx.shot({ motion: "bolt", trail: "fire" }, FLAVOR_LOOKS.Fire.colors, "fire", FROM, TO, FLIGHT);
    bolt.fx.step(100);
    expect(bolt.particles.alive()).toBeGreaterThan(0);
    expect(bolt.particles.inspect().every((p) => p.preset === "fire")).toBe(true);

    const arrow = setup();
    arrow.fx.shot(FLAVOR_LOOKS.Physical.projectile, FLAVOR_LOOKS.Physical.colors, "spark", FROM, TO, FLIGHT);
    arrow.fx.step(100);
    expect(arrow.particles.alive()).toBe(0);

    const goo = setup();
    goo.fx.shot(FLAVOR_LOOKS.Toxic.projectile, FLAVOR_LOOKS.Toxic.colors, "poison", FROM, TO, FLIGHT);
    goo.fx.step(100);
    expect(goo.particles.alive()).toBeGreaterThan(0);
  });

  it("#1442 a volley draws a sprite for each head, fanned out and landing apart", () => {
    const { particles, fx } = setup();
    const look = FLAVOR_LOOKS.Nature;
    fx.shot(look.projectile, look.colors, look.particle, FROM, TO, FLIGHT, 1);
    const mid = recordingContext();
    fx.step(150);
    fx.draw(mid.ctx);
    // Three seeds: each saves, translates, rotates and restores around its own path.
    expect(mid.calls.filter((call) => call === "translate")).toHaveLength(3);
    expect(mid.calls.filter((call) => call === "restore")).toHaveLength(3);
    fx.step(150);
    const landed = particles.inspect();
    const xs = landed.map((p) => p.x);
    expect(Math.max(...xs) - Math.min(...xs)).toBeGreaterThan(10);
  });

  it("#1442 a shot never blurs, writes text or draws an image", () => {
    for (const [flavor, look] of LOOKS) {
      const { fx } = setup();
      fx.shot(look.projectile, look.colors, look.particle, FROM, TO, FLIGHT, 1);
      const rec = recordingContext();
      for (let at = 0; at < FLIGHT; at += 20) {
        fx.step(20);
        fx.draw(rec.ctx);
      }
      for (const banned of ["fillText", "strokeText", "drawImage", "createPattern", "putImageData"]) {
        expect(rec.calls, `${flavor} ${banned}`).not.toContain(banned);
      }
      expect(rec.state.shadowBlur, flavor).toBe(0);
    }
  });

  it("#1442 a zero-length shot ends after one step", () => {
    const { fx } = setup();
    fx.shot({ motion: "bolt", trail: "fire" }, FLAVOR_LOOKS.Fire.colors, "fire", FROM, TO, 0);
    expect(fx.alive()).toBe(1);
    fx.step(16);
    expect(fx.alive()).toBe(0);
  });

  it("#1442 a stalled frame never stretches a shot past its flight", () => {
    const { fx } = setup();
    const shot: FxShot = { motion: "swipe" };
    fx.shot(shot, FLAVOR_LOOKS.Beast.colors, "shard", FROM, TO, FLIGHT);
    // Clamped frame time 16 ms, real time the whole flight.
    fx.step(16, FLIGHT);
    expect(fx.alive()).toBe(0);
  });

  it("#1442 the same seed draws the same shot", () => {
    const run = () => {
      const { particles, fx } = setup();
      const look = FLAVOR_LOOKS.Hustle;
      fx.shot(look.projectile, look.colors, look.particle, FROM, TO, FLIGHT, 1);
      for (let at = 0; at < FLIGHT; at += 16) fx.step(16);
      return particles.inspect().map((p) => [p.x, p.y]);
    };
    expect(run()).toEqual(run());
  });

  it("#1442 clear removes every shot", () => {
    const { fx } = setup();
    for (const [, look] of LOOKS) fx.shot(look.projectile, look.colors, look.particle, FROM, TO, FLIGHT);
    expect(fx.alive()).toBe(LOOKS.length);
    fx.clear();
    expect(fx.alive()).toBe(0);
  });
});
