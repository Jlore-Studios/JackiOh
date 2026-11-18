// R3345000's depth-plane picture URIs: one plane of a card's picture, or its whole picture, as a
// data URI, and either as a CSS `background-image` value (the effects layer holds no `url(` of its
// own: B27). Split from `pictures.ts` (#4180): these read the module's catalog
// (`game/currentCatalog.ts`), which the Vite config bundle (`static-pages.ts`, through
// `traininggrounds.ts`) must never evaluate, so the pure picture helpers stay importable there.
// Presentation only (CLAUDE.md rule 7): nothing here reaches the engine, an action, `PlayerView`
// or the log.

import { currentCatalog } from "../../game/currentCatalog.ts";

import type { ArtMotion } from "./motion.ts";
import { motifFor } from "./motifs.ts";
import type { ArtPlane } from "./pictures.ts";
import { isStinkyscapeArt } from "./stinkyscape.ts";
import { isStinkyscapePlusArt } from "./stinkyscapeplus.ts";
import { proceduralArtUri, stinkyscapeArtUri, stinkyscapePlusArtUri, trainingGroundsArtUri } from "./svg.ts";
import { compositionOf, themeFor } from "./themes.ts";
import { isTrainingGroundsArt } from "./traininggrounds.ts";

/**
 * R3345000: returns a data URI for one depth plane of `defId`'s picture. R3350000: or for its whole picture (`plane`
 * "whole"), which a Mythic's diorama cuts its slices from (dioramas/pieces.ts), moving as `motion` says (still with none).
 */
export function artPlaneUri(defId: string, plane: ArtPlane | "whole", radiant: boolean = false, motion: ArtMotion | null = null): string {
  // #4180: the module's catalog, read in the call, never at import: a face draws once the module is
  // in (landing and shop wait for it, the rest sits behind a gate).
  const def = currentCatalog().cards[defId];
  if (!def) return "";
  const { name, type, tags } = def;
  const theme = themeFor(tags, type);
  if (isStinkyscapeArt(defId)) {
    return stinkyscapeArtUri(defId, name, tags, type, radiant, motion, defId, null, plane);
  }
  if (isStinkyscapePlusArt(defId)) {
    return stinkyscapePlusArtUri(defId, name, tags, type, radiant, motion, defId, null, plane);
  }
  if (isTrainingGroundsArt(defId)) {
    return trainingGroundsArtUri(defId, name, tags, type, radiant, motion, defId, null, plane);
  }
  return proceduralArtUri(defId, theme, compositionOf(defId, type), radiant, motifFor(name, theme), motion, null, plane);
}

/** R3345000: `artPlaneUri` as a CSS `background-image` value (the effects layer holds no `url(` of its own: B27). */
export function artPlaneBackground(defId: string, plane: ArtPlane, radiant: boolean = false): string {
  return `url("${artPlaneUri(defId, plane, radiant)}")`;
}
