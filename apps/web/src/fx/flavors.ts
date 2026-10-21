/**
 * What every flavor looks like (#1442, docs/polish/8-flavor.md): its colours, its particle shape and
 * its projectile. The one table every flavor effect reads: the Ranged shot here, then the damage
 * projectile, casts, melee impacts, summons, deaths and trap reveals of the later parts of #1436.
 * Pure data, so a flavor with no look fails `typecheck`.
 */
import type { Flavor } from "@jackioh/shared";

import { FLAVOR_FALLBACK } from "../cards/flavor.ts";
import { hashId } from "../cards/art/hash.ts";
import { createRng } from "./rng.ts";
import type { FxPreset, FxShot } from "./types.ts";

/** Two or three `#rrggbb` colours, hot first: the core, the body and the glow. */
export type FlavorColors = readonly [string, string] | readonly [string, string, string];

/** `particle` is the preset a flavor's bursts use; `projectile` is the shot a Ranged attacker fires. */
export type FlavorLook = { colors: FlavorColors; particle: FxPreset; projectile: FxShot };

export const FLAVOR_LOOKS: Readonly<Record<Flavor, FlavorLook>> = {
  Physical: {
    colors: ["#f4ead2", "#a87a4a", "#9aa3ad"],
    particle: "spark",
    projectile: { motion: "arc", sprites: ["arrow"], turn: "point", count: 1 },
  },
  Brute: {
    colors: ["#d9cbb5", "#8a7a66", "#4a3f35"],
    particle: "dust",
    projectile: { motion: "arc", sprites: ["boulder"], turn: "spin", count: 1, shake: true },
  },
  Beast: { colors: ["#fff4e0", "#ff9d3c", "#c2410c"], particle: "shard", projectile: { motion: "swipe" } },
  Fire: {
    colors: ["#fff3b0", "#ff7a1a", "#e8401c"],
    particle: "fire",
    projectile: { motion: "bolt", trail: "fire" },
  },
  Dark: {
    colors: ["#e6d6ff", "#7a48b4", "#2a1240"],
    particle: "void",
    projectile: { motion: "bolt", trail: "void" },
  },
  Holy: {
    colors: ["#fffbe6", "#ffec9e", "#ffd24a"],
    particle: "holy",
    projectile: { motion: "bolt", trail: "sparkle" },
  },
  Nerd: {
    colors: ["#f3eaff", "#5b6ee1", "#8a5cff"],
    particle: "arcane",
    projectile: { motion: "arc", sprites: ["book"], turn: "spin", count: 1 },
  },
  Tech: { colors: ["#e8fdff", "#38e1ff", "#0077ff"], particle: "frost", projectile: { motion: "beam" } },
  Nature: {
    colors: ["#eaffc2", "#5fbf3a", "#2e7d32"],
    particle: "poison",
    projectile: { motion: "arc", sprites: ["seed"], turn: "point", count: 3 },
  },
  Toxic: {
    colors: ["#efffb0", "#8ee03a", "#4f9a1a"],
    particle: "poison",
    projectile: { motion: "arc", sprites: ["goo"], turn: "point", count: 1, trail: "poison" },
  },
  Snack: {
    colors: ["#fff1c2", "#d9a066", "#8b5a2b"],
    particle: "ember",
    projectile: { motion: "arc", sprites: ["snack"], turn: "spin", count: 1 },
  },
  Hustle: {
    colors: ["#fff4c2", "#ffd54a", "#3fae5a"],
    particle: "gold",
    projectile: { motion: "arc", sprites: ["coin", "bill"], turn: "spin", count: 4, scatter: true },
  },
  Clout: {
    colors: ["#ffe0f0", "#ff5aa5", "#ffd84a"],
    particle: "confetti",
    projectile: { motion: "arc", sprites: ["heart", "emoji"], turn: "float", count: 3 },
  },
  Spirit: {
    colors: ["#f0fbff", "#9fe3ff", "#6fa8dc"],
    particle: "sparkle",
    projectile: { motion: "bolt", trail: "sparkle", wave: true },
  },
  Chaos: {
    colors: ["#ff5ad1", "#5ad1ff", "#ffe45a"],
    particle: "prismatic",
    projectile: { motion: "bolt", trail: "prismatic" },
  },
};

/** The flavors Chaos can take the look of: every one but itself. */
const CHAOS_PICKS = (Object.keys(FLAVOR_LOOKS) as Flavor[]).filter(
  (flavor): flavor is Exclude<Flavor, "Chaos"> => flavor !== "Chaos",
);

/**
 * Which flavor a Chaos card takes the look of: drawn from its instance id, so one card always throws
 * the same thing and a test can name it. Never Chaos. Every later flavor effect uses this one pick.
 */
export function chaosFlavor(instanceId: string): Exclude<Flavor, "Chaos"> {
  return CHAOS_PICKS[Math.floor(createRng(hashId(instanceId))() * CHAOS_PICKS.length)] ?? FLAVOR_FALLBACK;
}

/**
 * The look `flavor` draws for the card `instanceId`: its own, or for Chaos the pick above. Chaos's own
 * row exists for the later parts' accents; its shot is never fired.
 */
export function flavorLook(flavor: Flavor, instanceId: string): FlavorLook {
  return FLAVOR_LOOKS[flavor === "Chaos" ? chaosFlavor(instanceId) : flavor];
}
