// #2792: presentation-only sub-looks for the crowded flavors, and particles of their own
// for Nature, Tech and Snack. No card's `flavor` changes; the event rows keep the base look.

import type { Flavor } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { PARTICLE_PRESETS } from "./presets.ts";
import { FLAVOR_LOOKS, FLAVOR_SUB_LOOKS, FX_PALETTES, flavorLook, isPalette, paletteLook } from "./flavors.ts";
import type { FxShotMotion } from "./types.ts";

const MOTIONS: readonly FxShotMotion[] = ["arc", "bolt", "beam", "swipe"];

describe("#2792 sub-looks", () => {
  it("#2792 the crowded flavors' sub-looks, at least 3 for Tech, Physical, Nerd and Dark and 2 for Beast and Hustle", () => {
    expect(Object.keys(FLAVOR_SUB_LOOKS.Physical ?? {})).toEqual(["steel", "wind", "sport", "blade"]);
    expect(Object.keys(FLAVOR_SUB_LOOKS.Beast ?? {})).toEqual(["felinor", "swarm", "wool"]);
    expect(Object.keys(FLAVOR_SUB_LOOKS.Dark ?? {})).toEqual(["blood", "shadow", "curse"]);
    expect(Object.keys(FLAVOR_SUB_LOOKS.Nerd ?? {})).toEqual(["ink", "chalk", "pixel"]);
    expect(Object.keys(FLAVOR_SUB_LOOKS.Tech ?? {})).toEqual(["laser", "rocket", "starfield", "hologram", "glitch"]);
    expect(Object.keys(FLAVOR_SUB_LOOKS.Hustle ?? {})).toEqual(["jade", "cash", "ore"]);
    expect(Object.keys(FLAVOR_SUB_LOOKS).sort()).toEqual(["Beast", "Dark", "Hustle", "Nerd", "Physical", "Tech"]);
    for (const [flavor, min] of [["Tech", 3], ["Physical", 3], ["Nerd", 3], ["Dark", 3], ["Beast", 2], ["Hustle", 2]] as const) {
      expect(Object.keys(FLAVOR_SUB_LOOKS[flavor] ?? {}).length, flavor).toBeGreaterThanOrEqual(min);
    }
  });

  it("#2792 every sub-look is a whole look, unlike its base and its siblings", () => {
    for (const [flavor, subs] of Object.entries(FLAVOR_SUB_LOOKS)) {
      const base = FLAVOR_LOOKS[flavor as Flavor];
      for (const [name, look] of Object.entries(subs ?? {})) {
        const label = `${flavor}/${name}`;
        expect(look.colors.length, label).toBeGreaterThanOrEqual(2);
        expect(look.colors.length, label).toBeLessThanOrEqual(3);
        for (const color of look.colors) expect(color, label).toMatch(/^#[0-9a-f]{6}$/);
        expect(Object.keys(PARTICLE_PRESETS), label).toContain(look.particle);
        expect(MOTIONS, label).toContain(look.projectile.motion);
        if (look.projectile.motion === "bolt") expect(Object.keys(PARTICLE_PRESETS), label).toContain(look.projectile.trail);
        expect(JSON.stringify(look.colors), label).not.toBe(JSON.stringify(base.colors));
        for (const [other, sibling] of Object.entries(subs ?? {})) {
          if (other === name) continue;
          expect(JSON.stringify(look.colors), `${label} vs ${other}`).not.toBe(JSON.stringify(sibling.colors));
        }
      }
    }
  });

  it("#2792 no two flavors' base looks share both a particle and a projectile; Nature, Tech and Snack have their own", () => {
    const flavors = Object.keys(FLAVOR_LOOKS) as Flavor[];
    for (let i = 0; i < flavors.length; i += 1) {
      for (let j = i + 1; j < flavors.length; j += 1) {
        const a = FLAVOR_LOOKS[flavors[i]!];
        const b = FLAVOR_LOOKS[flavors[j]!];
        expect(
          !(a.particle === b.particle && JSON.stringify(a.projectile) === JSON.stringify(b.projectile)),
          `${flavors[i]} vs ${flavors[j]}`,
        ).toBe(true);
      }
    }
    expect(FLAVOR_LOOKS.Nature.particle).toBe("leaf");
    expect(FLAVOR_LOOKS.Tech.particle).toBe("circuit");
    expect(FLAVOR_LOOKS.Snack.particle).toBe("crumb");
    expect(FLAVOR_SUB_LOOKS.Tech?.rocket?.particle).toBe("exhaust");
    expect(FLAVOR_SUB_LOOKS.Tech?.starfield?.particle).toBe("star");
  });

  it("#2792 paletteLook names a base look or a sub-look and nothing else", () => {
    expect(paletteLook("Tech")).toBe(FLAVOR_LOOKS.Tech);
    expect(paletteLook("Tech/laser")).toBe(FLAVOR_SUB_LOOKS.Tech?.laser);
    for (const bad of ["Tech/lazer", "Tech/", "/laser", "Tech/laser/x", "Brute/laser", "Plaid", "tech/laser", "toString", "Tech/toString"]) {
      expect(paletteLook(bad), bad).toBeUndefined();
    }
  });

  it("#2792 FX_PALETTES lists every flavor then every sub-look, each once", () => {
    expect(FX_PALETTES.slice(0, 15)).toEqual(Object.keys(FLAVOR_LOOKS));
    expect(FX_PALETTES).toHaveLength(36);
    expect(new Set(FX_PALETTES).size).toBe(FX_PALETTES.length);
    for (const palette of FX_PALETTES) expect(isPalette(palette)).toBe(true);
  });

  it("#2792 the event rows keep the base look", () => {
    for (const flavor of Object.keys(FLAVOR_LOOKS).filter((f) => f !== "Chaos") as Flavor[]) {
      expect(flavorLook(flavor, "u1")).toBe(FLAVOR_LOOKS[flavor]);
    }
  });
});
