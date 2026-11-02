/**
 * What every flavor looks like (#1442, docs/polish/8-flavor.md): its colors, its particle shape and
 * its projectile, the one table every flavor effect of #1436 reads. Pure data, so a flavor with no
 * look fails `typecheck`.
 */
import type { Flavor } from "@jackioh/shared";

import { FLAVOR_FALLBACK } from "../cards/flavor.ts";
import { hashId } from "../cards/art/hash.ts";
import { createRng } from "./rng.ts";
import type { FxPreset, FxShot } from "./types.ts";

/** Two or three `#rrggbb` colors, hot first: the core, the body and the glow. */
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
  Tech: { colors: ["#e8fdff", "#38e1ff", "#0077ff"], particle: "circuit", projectile: { motion: "beam" } },
  Nature: {
    colors: ["#eaffc2", "#5fbf3a", "#2e7d32"],
    particle: "leaf",
    projectile: { motion: "arc", sprites: ["seed"], turn: "point", count: 3 },
  },
  Toxic: {
    colors: ["#efffb0", "#8ee03a", "#4f9a1a"],
    particle: "poison",
    projectile: { motion: "arc", sprites: ["goo"], turn: "point", count: 1, trail: "poison" },
  },
  Snack: {
    colors: ["#fff1c2", "#d9a066", "#8b5a2b"],
    particle: "crumb",
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

const CHAOS_PICKS = (Object.keys(FLAVOR_LOOKS) as Flavor[]).filter(
  (flavor): flavor is Exclude<Flavor, "Chaos"> => flavor !== "Chaos",
);

/** A Chaos card's look, drawn from its instance id so one card always throws the same thing; every flavor effect uses this pick. */
export function chaosFlavor(instanceId: string): Exclude<Flavor, "Chaos"> {
  return CHAOS_PICKS[Math.floor(createRng(hashId(instanceId))() * CHAOS_PICKS.length)] ?? FLAVOR_FALLBACK;
}

/** The look `flavor` draws for the card `instanceId`. Chaos's own row is for accents; its shot is never fired. */
export function flavorLook(flavor: Flavor, instanceId: string): FlavorLook {
  return FLAVOR_LOOKS[flavor === "Chaos" ? chaosFlavor(instanceId) : flavor];
}

/** #2792: a flavor's look or one of its sub-looks, presentation only; no card's `flavor` changes. */
export type FxPalette = Flavor | `${Flavor}/${string}`;

/**
 * #2792: presentation-only variants of the crowded flavors' looks. A card's fx table entry picks one
 * with `palette: "<Flavor>/<sub-look>"`; a card with no sub-look keeps its flavor's base look, which
 * the event rows always draw.
 */
export const FLAVOR_SUB_LOOKS: { readonly [F in Flavor]?: Readonly<Record<string, FlavorLook>> } = {
  Physical: {
    steel: { colors: ["#ffffff", "#c7d0db", "#5d6773"], particle: "shard", projectile: FLAVOR_LOOKS.Physical.projectile },
    wind: { colors: ["#f4fbff", "#cde6ef", "#86aebf"], particle: "dust", projectile: { motion: "bolt", trail: "dust", wave: true } },
    sport: { colors: ["#ffffff", "#ff7a2e", "#2f6fdb"], particle: "confetti", projectile: { motion: "arc", sprites: ["boulder"], turn: "spin", count: 1 } },
    blade: { colors: ["#ffffff", "#e3e8ef", "#b3202d"], particle: "frost", projectile: { motion: "swipe" } },
  },
  Beast: {
    felinor: { colors: ["#fff0f6", "#ffb3d1", "#a77bd6"], particle: "sparkle", projectile: FLAVOR_LOOKS.Beast.projectile },
    swarm: { colors: ["#fff7b3", "#f2c200", "#2b2200"], particle: "ember", projectile: FLAVOR_LOOKS.Beast.projectile },
    wool: { colors: ["#ffffff", "#f1ede4", "#b9b2a5"], particle: "smoke", projectile: FLAVOR_LOOKS.Beast.projectile },
  },
  Dark: {
    blood: { colors: ["#ffd6dc", "#c8102e", "#4a0010"], particle: "blood", projectile: { motion: "bolt", trail: "blood" } },
    shadow: { colors: ["#c9c9d6", "#3a3a4a", "#0b0b12"], particle: "smoke", projectile: { motion: "bolt", trail: "smoke" } },
    curse: { colors: ["#eaffd6", "#6bbf3a", "#2a1a3d"], particle: "arcane", projectile: { motion: "bolt", trail: "arcane" } },
  },
  Nerd: {
    ink: { colors: ["#e8ecf5", "#2b3a67", "#0d1430"], particle: "blood", projectile: FLAVOR_LOOKS.Nerd.projectile },
    chalk: { colors: ["#ffffff", "#e9efe6", "#3f5a48"], particle: "dust", projectile: FLAVOR_LOOKS.Nerd.projectile },
    pixel: { colors: ["#ffffff", "#5bff8a", "#3a7bff"], particle: "circuit", projectile: FLAVOR_LOOKS.Nerd.projectile },
  },
  Tech: {
    laser: { colors: ["#fff0f0", "#ff3b4a", "#b0001e"], particle: "spark", projectile: { motion: "beam" } },
    rocket: { colors: ["#fff4d6", "#ff9a3c", "#6b7a99"], particle: "exhaust", projectile: { motion: "bolt", trail: "exhaust" } },
    starfield: { colors: ["#ffffff", "#bfd4ff", "#1b2a5c"], particle: "star", projectile: { motion: "bolt", trail: "star" } },
    hologram: { colors: ["#f0fffb", "#7dffe0", "#b48cff"], particle: "sparkle", projectile: { motion: "beam" } },
    glitch: { colors: ["#ffffff", "#ff2bd6", "#2bffea"], particle: "circuit", projectile: { motion: "bolt", trail: "circuit" } },
  },
  Hustle: {
    jade: { colors: ["#e9fff2", "#3fbf7f", "#0e6b44"], particle: "shard", projectile: FLAVOR_LOOKS.Hustle.projectile },
    cash: { colors: ["#f1ffe8", "#7fcf6a", "#2e7d32"], particle: "confetti", projectile: { motion: "arc", sprites: ["bill"], turn: "spin", count: 4, scatter: true } },
    ore: { colors: ["#f4f0ff", "#9aa7c7", "#4b3f72"], particle: "crumb", projectile: { motion: "arc", sprites: ["boulder"], turn: "spin", count: 3, scatter: true } },
  },
};

const FLAVOR_NAMES = Object.keys(FLAVOR_LOOKS) as Flavor[];

/** #2792: every flavor then every sub-look, each once; the event rows keep the base look. */
export const FX_PALETTES: readonly FxPalette[] = [
  ...FLAVOR_NAMES,
  ...FLAVOR_NAMES.flatMap((flavor) => Object.keys(FLAVOR_SUB_LOOKS[flavor] ?? {}).map((name): FxPalette => `${flavor}/${name}`)),
];

const own = (table: object, key: string): boolean => Object.prototype.hasOwnProperty.call(table, key);

/** #2792: the base look a flavor names, or the sub-look `"<Flavor>/<sub-look>"` names; nothing else. */
export function paletteLook(palette: string): FlavorLook | undefined {
  const [flavor = "", sub, ...rest] = palette.split("/");
  if (rest.length > 0 || !own(FLAVOR_LOOKS, flavor)) return undefined;
  if (sub === undefined) return FLAVOR_LOOKS[flavor as Flavor];
  const subs = FLAVOR_SUB_LOOKS[flavor as Flavor];
  return subs !== undefined && own(subs, sub) ? subs[sub] : undefined;
}

/** #2792: whether `name` is a flavor or one of its sub-looks. */
export function isPalette(name: string): name is FxPalette {
  return paletteLook(name) !== undefined;
}
