// R3400000: the art a Mythic's own takeover look draws over the dark, part by part (fx.css's
// `.takeover-art[data-look]`): Bob's Plan's ink-wash summit, Scratch Pad's blank canvas and Goetzmann
// Game Design's napkin. R3395001: and Stinkyscape's five, Tylannt's vault door, Rod of Ages' colossal rod,
// Great Paperclip Theory's wire, Paragon-X's white dwarf and Prototype-X2's forge. The takeover cue mounts
// them live (dom.ts), timed to its landing; Reduce Motion's still mounts the same parts at rest
// (game/TakeoverStill.tsx). Shapes only, never a card face (R202). A part is a class, `.takeover-art-<part>`,
// so each name is used by one look alone.

import type { TakeoverLook } from "./types.ts";

/** Each look with art, its parts bottom to top; a look not listed draws none. */
export const TAKEOVER_ART_PARTS: Readonly<Partial<Record<TakeoverLook, readonly string[]>>> = {
  summit: ["wash", "range", "arrow", "flag"],
  canvas: ["paper", "peaks", "cursor", "rows"],
  napkin: ["napkin", "lamp", "pencil", "sketch", "spiral"],
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
