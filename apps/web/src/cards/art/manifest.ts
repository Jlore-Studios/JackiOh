// Which cards have real art (docs/polish/6-cards.md, Surface A, B5).
//
// Real art is a file in `apps/web/public/art/` per card id and face: `<id>.webp` and
// `<id>-radiant.webp`. The client asks for a file only when this manifest lists it, so a card
// without art never costs a request and the board never fires a storm of 404s. Until an artist
// delivers, every card is procedural, and the procedural art stays every card's fallback.
//
// The delivery convention (format, size, weight) is in ART.md beside this file, and convention.ts
// holds its numbers; convention.test.ts fails on a listed file that is missing or breaks it, and on
// a file in the directory that no line here lists (R658). To add art: drop the file(s) in
// `apps/web/public/art/`, then add a line here, for example
//   "core-002": { base: true, radiant: true },
// and, if the artist is credited, their name as the card's `artist` in packages/cards/flavour.json.
// A radiant face whose own file is missing shows the base file under a gold tint.

export type ArtManifest = Readonly<Record<string, { readonly base?: true; readonly radiant?: true }>>;

/** Which ids have real art in apps/web/public/art/. Ships empty. */
export const ART_MANIFEST: ArtManifest = {};

const ART_DIRECTORY = "art/";
const ART_EXTENSION = ".webp";
const RADIANT_SUFFIX = "-radiant";

/** The file a card's face is drawn from, inside `apps/web/public/art/`: `<id>.webp` or `<id>-radiant.webp`. */
export function artFileName(defId: string, radiant: boolean): string {
  return `${defId}${radiant ? RADIANT_SUFFIX : ""}${ART_EXTENSION}`;
}

/** `${import.meta.env.BASE_URL}art/<id>.webp` or `…/<id>-radiant.webp`; `tint` = gold overlay on base art. */
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
