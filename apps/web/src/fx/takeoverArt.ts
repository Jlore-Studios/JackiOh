// R3400000: the art a Mythic's own takeover look draws over the dark, part by part (fx.css's
// `.takeover-art[data-look]`): Bob's Plan's ink-wash summit, Scratch Pad's blank canvas and Goetzmann
// Game Design's napkin. R3405000: and each Meditative Mythic's own: Reach the Summit's ascent, True Craft
// a Card's workshop, Aestheticize the Game's brush, Red Jade's mine, Jade Beauty's quarry, YileGPT's ai,
// YileGPT Prime's pavilion, Yile's Virus's virus, AI Girlfriend's dojo, Pareto Optimality's studyHall,
// The Cane's caneRack, Jlockeed's Evil Blueprints' warRoom and Jlockeed's Headquarters' fortress, each
// part named for its look. R3395001: and Stinkyscape's five, Tylannt's vault door, Rod of Ages' colossal
// rod, Great Paperclip Theory's wire, Paragon-X's white dwarf and Prototype-X2's forge. The takeover cue
// mounts them live (dom.ts), timed to its landing; Reduce Motion's still mounts the same parts at rest
// (game/TakeoverStill.tsx). Shapes only, never a card face (R202): True Craft's card is a blank card
// shape. A part is a class, `.takeover-art-<part>`, so each name is used by one look alone.

import type { TakeoverLook } from "./types.ts";

/** Each look with art, its parts bottom to top; a look not listed draws none. */
export const TAKEOVER_ART_PARTS: Readonly<Partial<Record<TakeoverLook, readonly string[]>>> = {
  summit: ["wash", "range", "arrow", "flag"],
  canvas: ["paper", "peaks", "cursor", "rows"],
  napkin: ["napkin", "lamp", "pencil", "sketch", "spiral"],
  ascent: ["ascent-stars", "ascent-ledges", "ascent-climber", "ascent-clouds"],
  workshop: ["workshop-bench", "workshop-maker", "workshop-threads", "workshop-card"],
  brush: ["brush-scroll", "brush-wash", "brush-bloom", "brush-pen"],
  mine: ["mine-rock", "mine-seam", "mine-lamp", "mine-shards"],
  quarry: ["quarry-cliffs", "quarry-boulder", "quarry-veins", "quarry-dust"],
  ai: ["ai-scroll", "ai-scan", "ai-strokes"],
  pavilion: ["pavilion-hall", "pavilion-ring", "pavilion-scholar"],
  virus: ["virus-snow", "virus-face", "virus-eyes"],
  dojo: ["dojo-floor", "dojo-mat", "dojo-beams"],
  studyHall: ["hall-scroll", "hall-curve", "hall-students", "hall-cane"],
  caneRack: ["rack-lacquer", "rack-cane", "rack-hand"],
  warRoom: ["war-table", "war-villain", "war-blueprint", "war-lamps"],
  fortress: ["fortress-keep", "fortress-peak", "fortress-racks", "fortress-beacons"],
  vault: ["murk", "gap", "door", "dial"],
  rod: ["gulf", "colossus", "bandlight"],
  wire: ["blueprint", "strand", "stairs"],
  whiteDwarf: ["dwarf", "flare", "halo", "cut"],
  forge: ["sphere", "magma", "convoy", "hull"],
};

/** The parts `look`'s art draws, none for a look without art. */
export function takeoverArtParts(look: TakeoverLook): readonly string[] {
  return TAKEOVER_ART_PARTS[look] ?? [];
}
