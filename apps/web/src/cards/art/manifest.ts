// Real art lives in `apps/web/public/art/`; this manifest avoids requests for absent files (Surface A, B5).
// ART.md and convention.test.ts keep it in step with that directory (R660).
// Missing radiant art uses a gold-tinted base file.

export type ArtManifest = Readonly<Record<string, { readonly base?: true; readonly radiant?: true }>>;

export const ART_MANIFEST: ArtManifest = {};

const ART_DIRECTORY = "art/";
const ART_EXTENSION = ".webp";
const RADIANT_SUFFIX = "-radiant";

export function artFileName(defId: string, radiant: boolean): string {
  return `${defId}${radiant ? RADIANT_SUFFIX : ""}${ART_EXTENSION}`;
}

export function artUrl(
  defId: string,
  radiant: boolean,
  manifest: ArtManifest = ART_MANIFEST,
): { src: string; tint: boolean } | null {
  if (!Object.hasOwn(manifest, defId)) return null;
  const entry = manifest[defId];
  if (entry === undefined) return null;
  const root = `${import.meta.env.BASE_URL}${ART_DIRECTORY}`;
  if (radiant && entry.radiant === true) return { src: `${root}${artFileName(defId, true)}`, tint: false };
  if (entry.base === true) return { src: `${root}${artFileName(defId, false)}`, tint: radiant };
  return null;
}
