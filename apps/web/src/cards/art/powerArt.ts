// Each Heroic Power's own art on the hero panel (SPEC §8 #98, patch v0.2.1: "each power has its own
// art", the way a Hearthstone hero power is a round crest of its own). A power's art is one of the
// procedural art's glyphs (emblems.ts, drawn for this project in the same 24×24 box) on a crest in
// the power's own two colours, so no two of the thirteen look alike and no binary asset is needed.
//
// The table is keyed by the power's stored name (R103: "recruit", "ping", …), which is the name the
// view gives it (`HeroPowerView.name`) and never changes with the face — a printed title does: Armor
// Up's Radiant face is Tank Up, and both wear the shield. Presentation only (CLAUDE.md rule 7): no
// rule reads it, and a name the table does not know is drawn with `DEFAULT_POWER_ART`.

import { EMBLEM_BOX, EMBLEMS, type EmblemGlyph } from "./emblems.ts";
import type { GlyphPath } from "./motifGlyphs.ts";

/** A power's crest: the glyph at its heart and the light and dark ends of its gradient. */
export type PowerArt = { glyph: EmblemGlyph; light: string; dark: string };

/** The thirteen powers' crests, by stored name, in the engine's roll order. */
export const POWER_ART: Readonly<Record<string, PowerArt>> = {
  // Expedition Map: a planted flag, old-map amber.
  recruit: { glyph: "flag", light: "hsl(40 92% 72%)", dark: "hsl(32 70% 22%)" },
  // Life Tap: a drop of blood, warlock violet.
  draw: { glyph: "drop", light: "hsl(272 88% 78%)", dark: "hsl(272 62% 22%)" },
  // Ping: a bolt, arcane blue.
  ping: { glyph: "bolt", light: "hsl(212 95% 76%)", dark: "hsl(220 70% 24%)" },
  // Steady Shot: a target, hunter green.
  burn: { glyph: "target", light: "hsl(102 70% 70%)", dark: "hsl(108 55% 18%)" },
  // Ranching: a horned head, rust.
  rush: { glyph: "horns", light: "hsl(20 92% 72%)", dark: "hsl(14 70% 22%)" },
  // Cat Cafe: a cat, rose.
  felinor: { glyph: "cat", light: "hsl(326 88% 80%)", dark: "hsl(326 58% 24%)" },
  // Witness Value: an open eye, teal.
  discover: { glyph: "eye", light: "hsl(168 78% 70%)", dark: "hsl(174 70% 17%)" },
  // Stitching: two strands fused, indigo.
  stitching: { glyph: "helix", light: "hsl(244 80% 82%)", dark: "hsl(244 46% 26%)" },
  // Armor Up and Tank Up: a shield, steel.
  armor: { glyph: "shield", light: "hsl(212 26% 86%)", dark: "hsl(215 22% 28%)" },
  // Die Insect: a flame, fire.
  insect: { glyph: "flame", light: "hsl(36 100% 66%)", dark: "hsl(4 78% 26%)" },
  // KY Brainstorm: a brain, magenta.
  brainstorm: { glyph: "brain", light: "hsl(296 80% 80%)", dark: "hsl(296 52% 24%)" },
  // Pluck: a fruit, lime.
  pluck: { glyph: "fruit", light: "hsl(72 84% 66%)", dark: "hsl(80 62% 18%)" },
  // Terminus Tricks: a mask, cyan.
  terminus: { glyph: "mask", light: "hsl(190 88% 72%)", dark: "hsl(196 72% 20%)" },
};

/** A power the table does not know: a plain sparkle in the board's blue. */
export const DEFAULT_POWER_ART: PowerArt = { glyph: "star", light: "hsl(214 90% 80%)", dark: "hsl(222 52% 20%)" };

/** The crest's glyph box, which an `<svg viewBox>` draws it in. */
export const POWER_ART_BOX = EMBLEM_BOX;

/** A power's crest and its glyph's path data, by stored name. */
export function powerArtOf(name: string): PowerArt & { path: GlyphPath } {
  const art = Object.hasOwn(POWER_ART, name) ? (POWER_ART[name] ?? DEFAULT_POWER_ART) : DEFAULT_POWER_ART;
  return { ...art, path: EMBLEMS[art.glyph] };
}
