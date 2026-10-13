// One effect, many looks: a look is one family's choice of particles, ring, DOM tint and emblem, so a
// single recipe reads differently for a Felinor, a CN virus or a Book, and a new family is one row.
//
// The families are the art themes (`cards/art/themes.ts`), picked as a card's picture is: its tags,
// then Token, then its type. The tint is the theme's palette and the emblem its glyph.
//
// R202: a look is chosen from the public catalog facts (`FxCardFacts`) of a definition the viewer can
// read, never from a hidden card: no facts, no look, and the caller falls back to a tone's.

import { THEME_PALETTES, themeFor, type ArtThemeId } from "../cards/art/themes.ts";
import { EMBLEMS, type EmblemGlyph } from "../cards/art/emblems.ts";
import type { FxCardFacts, FxIcon, FxPreset, FxTint } from "./types.ts";

/** One family's look: the particles it throws, the ring it flares with, its tint and its emblem. */
export type FxLook = { preset: FxPreset; ring: FxPreset; tint: FxTint; icon: FxIcon };

/** Each family's particles and ring (the canvas half of its look); its colours come from its palette. */
const THEME_PARTICLES: Readonly<Record<ArtThemeId, { preset: FxPreset; ring: FxPreset }>> = {
  human: { preset: "holy", ring: "gold" },
  felinor: { preset: "sparkle", ring: "prismatic" },
  ky: { preset: "frost", ring: "sparkle" },
  cn: { preset: "poison", ring: "poison" },
  fruit: { preset: "ember", ring: "fire" },
  chaos: { preset: "prismatic", ring: "void" },
  quickdraw: { preset: "spark", ring: "frost" },
  book: { preset: "arcane", ring: "gold" },
  pancake: { preset: "gold", ring: "ember" },
  ai: { preset: "frost", ring: "arcane" },
  token: { preset: "dust", ring: "dust" },
  unit: { preset: "ember", ring: "dust" },
  spell: { preset: "arcane", ring: "arcane" },
  "field-spell": { preset: "gold", ring: "holy" },
  trap: { preset: "fire", ring: "blood" },
  "field-trap": { preset: "void", ring: "arcane" },
};

function iconOf(glyph: EmblemGlyph): FxIcon {
  const path = EMBLEMS[glyph];
  return { d: path.d, rule: path.rule };
}

function lookOfTheme(theme: ArtThemeId): FxLook {
  const palette = THEME_PALETTES[theme];
  const particles = THEME_PARTICLES[theme];
  return {
    preset: particles.preset,
    ring: particles.ring,
    tint: { rim: palette.emblem.fill, core: palette.mote, glow: palette.glow },
    icon: iconOf(palette.emblem.glyph),
  };
}

/** Every family's look, by art theme. */
export const FX_LOOKS: Readonly<Record<ArtThemeId, FxLook>> = Object.fromEntries(
  (Object.keys(THEME_PARTICLES) as ArtThemeId[]).map((theme) => [theme, lookOfTheme(theme)]),
) as Record<ArtThemeId, FxLook>;

/**
 * The looks of what nothing readable cast: hits in fire with a flame, heals in holy light with a
 * heart, and the arcane of a card cast by a card the viewer may not read.
 */
export const TONE_LOOKS: Readonly<Record<"damage" | "heal" | "cast", FxLook>> = {
  damage: {
    preset: "fire",
    ring: "ember",
    tint: { rim: "#ffb35c", core: "#fff1c2", glow: "#ff6a2b" },
    icon: iconOf("flame"),
  },
  heal: {
    preset: "holy",
    ring: "gold",
    tint: { rim: "#bff7c9", core: "#fffbe6", glow: "#7fe3a0" },
    icon: iconOf("heart"),
  },
  cast: {
    preset: "arcane",
    ring: "arcane",
    tint: { rim: "#c9a4ff", core: "#f3eaff", glow: "#8a5cff" },
    icon: iconOf("star"),
  },
};

/** The look of a card the viewer can read, by its public facts; undefined when they do not say. */
export function lookOf(facts: FxCardFacts | undefined): FxLook | undefined {
  if (facts?.type === undefined) return undefined;
  return FX_LOOKS[themeFor(facts.tags ?? [], facts.type)];
}

/** The look of a whole pile reached at once (a zone wave), and which way the wave runs. */
export type ZoneLook = { look: FxLook; direction: "up" | "down" | "none" };

const ZONE_DOWN: FxLook = {
  preset: "void",
  ring: "void",
  tint: { rim: "#b48cff", core: "#efe2ff", glow: "#7a48b4" },
  icon: iconOf("moon"),
};
const ZONE_UP: FxLook = {
  preset: "gold",
  ring: "gold",
  tint: { rim: "#ffd54a", core: "#fff4c2", glow: "#e0a800" },
  icon: iconOf("crown"),
};
const ZONE_GONE: FxLook = {
  preset: "smoke",
  ring: "void",
  tint: { rim: "#a26bff", core: "#e6d6ff", glow: "#3d1f66" },
  icon: iconOf("portal"),
};

/**
 * Each event's look on a whole pile: Degrade sinks in violet, Upgrade rises in gold, a card made Radiant
 * shines gold, cards leaving (exiled, crumbled) go in smoke and void; anything else is arcane.
 */
export const ZONE_LOOKS: Readonly<Partial<Record<string, ZoneLook>>> = {
  degraded: { look: ZONE_DOWN, direction: "down" },
  upgraded: { look: ZONE_UP, direction: "up" },
  radiantSet: { look: { ...ZONE_UP, preset: "prismatic" }, direction: "up" },
  exiled: { look: ZONE_GONE, direction: "none" },
  crumbled: { look: { ...ZONE_GONE, preset: "dust" }, direction: "down" },
};

/** A zone wave's look when its event names none: arcane, across. */
export const ZONE_DEFAULT: ZoneLook = { look: TONE_LOOKS.cast, direction: "none" };
