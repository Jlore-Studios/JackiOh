// #2681: the art a card's window draws, as one data URI, from the card's public definition alone and
// with no React: the release artwork (releaseArt.ts) is built in Node as well as the browser. It sits
// here, not beside the art, because it changes no card's look: it reads the art's own builders
// (cards/art/svg.ts) and adds nothing to them.
//
// It makes the choices `CardArt` makes, in its order (R503, R2170000, R2202000, R2511000): a Stinkyscape
// card and a Training Grounds card draw their set's own family, a Legacy card its Core card's picture
// read by the Core card's name, and every other card its theme, composition and motif.

import type { CardDef } from "@jackioh/shared";

import { motifFor } from "../cards/art/motifs.ts";
import { isStinkyscapeArt } from "../cards/art/stinkyscape.ts";
import { proceduralArtUri, stinkyscapeArtUri, trainingGroundsArtUri } from "../cards/art/svg.ts";
import { compositionFor, themeFor } from "../cards/art/themes.ts";
import { isTrainingGroundsArt, LEGACY_PREFIX, snapshotArtOf } from "../cards/art/traininggrounds.ts";

export function proceduralCardArtUri(def: Pick<CardDef, "id" | "name" | "tags" | "type">, radiant: boolean): string {
  const theme = themeFor(def.tags, def.type);
  const coreId = snapshotArtOf(def.id);
  const name = coreId === null ? def.name : def.name.replace(LEGACY_PREFIX, "");
  if (isStinkyscapeArt(def.id)) return stinkyscapeArtUri(def.id, name, def.tags, def.type, radiant);
  if (isTrainingGroundsArt(def.id)) return trainingGroundsArtUri(def.id, name, def.tags, def.type, radiant);
  return proceduralArtUri(coreId ?? def.id, theme, compositionFor(def.type), radiant, motifFor(name, theme));
}
