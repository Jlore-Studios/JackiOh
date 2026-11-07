// The art slice's public surface (docs/polish/6-cards.md, Surface A). Code inside `cards/` imports
// by path; code outside goes through `cards/index.ts`, which re-exports from here.

export type { ArtThemeId, Composition } from "./themes.ts";
export { compositionFor, compositionOf, themeFor } from "./themes.ts";
export { motifFor } from "./motifs.ts";
export type { ArtSpec } from "./procedural.ts";
export { artSpec } from "./procedural.ts";
export { hashId, seededRandom } from "./hash.ts";
export { artDataUri } from "./svg.ts";
export { artPlaneUri, ART_PLANES, type ArtPlane } from "./pictures.ts";
export type { ArtManifest } from "./manifest.ts";
export { ART_MANIFEST, artUrl } from "./manifest.ts";
export type { ArtShape, CardArtProps } from "./CardArt.tsx";
export { CardArt } from "./CardArt.tsx";
