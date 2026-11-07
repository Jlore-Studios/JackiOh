// R3344000 (#3344): the depth a rare card's picture has. A face that is Epic or rarer, or wears a full-art-shaped level,
// draws its picture as stacked planes by the roles its painter tags its layers with (art/motion.ts, R2715001): the
// background far, the props in the middle, the subject near, and what lies over it, its light and its motes nearest
// (art/pictures.ts `PLANE_OF_ROLE`). Nothing else has planes: a Common in its normal look, a delivered file (it has no
// layers), a Starship in play and a Legacy card (their painters draw one picture, CardArt `hasArtPlanes`), a restyle of
// the picture (a filter over one plane would not be one over the stack), Immersive, Out of frame and Extended art, which
// have layers and geometry of their own, and Glitch's blank. The Reduce motion setting, the system's, and the fx
// intensity off draw the flat picture, its foil still.
//
// The glare is the light the tilt (tilt.ts) catches: a white sheen that moves opposite it, in two layers each blended in
// the way of its area, `color-dodge` over the art and `overlay` over the frame, and each foil finish catches it by
// area, more on the art than the frame or the other way about (`GLARE_BY_FOIL`). A Radiant face's frame takes none, its
// gold edge left to Radiant's own shimmer (R2725004).
//
// R3344000 also lets a Legendary or Mythic face's subject break the face's top edge, in the viewer's own hand and in the
// inspect views, as Out of frame's does. Presentation only: nothing here reaches the engine, an action, `PlayerView` or
// the log (CLAUDE.md rule 7).

import type { Rarity } from "@jackioh/shared";

import { useOsReducedMotion } from "../game/useOsReducedMotion.ts";
import { useFxSettings } from "../fx/settings.ts";
import { useSetting } from "../settings/store.ts";
import type { TiltMode } from "./tilt.ts";
import { VERSION_EXTENDED_ART_ID, VERSION_IMMERSIVE_ID, VERSION_OUT_OF_FRAME_ID } from "./versions.ts";

/** R3344000: the rarities whose faces have depth: Epic and above. */
const DEPTH_RARITIES: ReadonlySet<Rarity> = new Set<Rarity>(["Epic", "Legendary", "Mythic"]);

/** R3344000: the versions that keep a layering of their own, so the face draws no planes over it. */
const OWN_LAYER_VERSIONS: ReadonlySet<string> = new Set([VERSION_IMMERSIVE_ID, VERSION_OUT_OF_FRAME_ID, VERSION_EXTENDED_ART_ID]);

/** What decides whether a face has depth (`hasDepth`). */
export type DepthInput = {
  /** The rarity the face's frame shows (frame.ts `frameRarity`). */
  rarity: Rarity | null;
  /** The face prints full art (fullArt.ts `showsFullArt`): every full-art-shaped level has depth, at any rarity. */
  fullArt: boolean;
  /** The face's version id (versions.ts), or null on the normal face. */
  version: string | null;
  /** The face is Glitch's blank card. */
  glitch: boolean;
  /** The face's picture can be drawn in planes (CardArt `hasArtPlanes`). */
  planed: boolean;
};

/** R3344000: whether a face wants depth, whatever the viewer's motion settings say (`flat` is theirs, `useDepthMotion`). */
export function hasDepth({ rarity, fullArt, version, glitch, planed }: DepthInput): boolean {
  if (!planed || glitch) return false;
  if (version !== null && OWN_LAYER_VERSIONS.has(version)) return false;
  return fullArt || (rarity !== null && DEPTH_RARITIES.has(rarity));
}

/** R3344000: the rarities whose faces may break the frame: Legendary and Mythic. */
const BREAKING_RARITIES: ReadonlySet<Rarity> = new Set<Rarity>(["Legendary", "Mythic"]);

/**
 * R3344000: whether a face's subject may break its top edge: a Legendary or Mythic face in the viewer's own hand
 * (`ownHand`, which Card.tsx sets) or in a view that tilts under the pointer anywhere or by a drag (the hover preview and
 * the inspect sheet), where the face is large and the player looks at it.
 */
export function breaksFrame(rarity: Rarity | null, ownHand: boolean, tilt: TiltMode): boolean {
  if (rarity === null || !BREAKING_RARITIES.has(rarity)) return false;
  return ownHand || tilt === "window" || tilt === "drag";
}

/** How much of the glare each area of a face takes, 0 to 1: the art window's, and the frame's around it. */
export type Glare = { readonly art: number; readonly frame: number };

/** The id a face with no foil has (the cosmetics registry's default). */
export const NO_FOIL_ID = "foil-none";

/**
 * R3344000: the glare each foil finish catches, by area, each a share of the full glare (0 to 1): the art window's and the
 * frame's apart, so no finish is one flat gradient over the face. The pairs are a design choice, one row per foil the
 * cosmetics registry has (`depth.test.tsx` holds the table to it), the metal foils (the sunburst, the molten gold) taking
 * the most on the frame and the ones that live in the picture (the glitter, the void's shimmer) the least.
 */
export const GLARE_BY_FOIL: Readonly<Record<string, Glare>> = {
  [NO_FOIL_ID]: { art: 0.3, frame: 0.16 },
  "foil-holo-sweep": { art: 0.55, frame: 0.2 },
  "foil-rainbow": { art: 0.5, frame: 0.2 },
  "foil-cracked-ice": { art: 0.6, frame: 0.12 },
  "foil-glitter": { art: 0.5, frame: 0.1 },
  "foil-sunburst": { art: 0.45, frame: 0.3 },
  "foil-aurora": { art: 0.45, frame: 0.18 },
  "foil-galaxy": { art: 0.4, frame: 0.14 },
  "foil-molten-gold": { art: 0.35, frame: 0.35 },
  "foil-void-shimmer": { art: 0.25, frame: 0.1 },
};

/**
 * R3344000: the glare a face catches: its worn foil's row (the none row for a face wearing none, or a foil the table does
 * not know), and for a Radiant face the none row's art with no frame at all, so its gold edge keeps only its own shimmer
 * (R2725004).
 */
export function glareFor(foilId: string | undefined, radiant: boolean): Glare {
  const none = GLARE_BY_FOIL[NO_FOIL_ID] as Glare;
  if (radiant) return { art: none.art, frame: 0 };
  return (foilId === undefined ? undefined : GLARE_BY_FOIL[foilId]) ?? none;
}

/** R3344000: whether a face's depth rests and its foil holds still: `flat` draws no planes at all, `fxOff` also stills the foil. */
export type DepthMotion = { flat: boolean; fxOff: boolean };

/**
 * R3344000: the Reduce motion setting, the system's setting and the fx intensity off each draw the flat picture;
 * the fx intensity off also holds a depth face's foil still (CardFace). Its hooks run on every face.
 */
export function useDepthMotion(): DepthMotion {
  const panel = useSetting("reduceMotion");
  const system = useOsReducedMotion();
  const [{ intensity }] = useFxSettings();
  const fxOff = intensity === "off";
  return { flat: panel || system || fxOff, fxOff };
}
