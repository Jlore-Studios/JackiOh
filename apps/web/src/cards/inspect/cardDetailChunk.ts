// #4517: the card detail dialog's chunk, owned by the landing's fan. The dialog (its patch
// history and win-rate box) arrives the first time a fan card opens, prefetched when pointer,
// touch or focus shows interest on a fan card. Only code is fetched, never game state
// (CLAUDE.md rule 7). This module never value-imports the dialog file, so the entry keeps only
// the fan, not the view it opens.

import { routeChunk } from "../../routes/routeChunk.tsx";
import { savingData } from "../../net/saveData.ts";
import type { CardDetailProps } from "./CardDetail.tsx";

/** The detail dialog, drawn at once when its code is in, else through `lazy()`. */
export const cardDetailChunk = routeChunk<CardDetailProps>(() =>
  import("./CardDetail.tsx").then((module) => ({ default: module.CardDetail })),
);

/** The dialog chunk once per page load, as `prefetchRoute` holds each route's. */
let prefetched = false;

/** Test seam: a new page. */
export function resetCardDetailPrefetchForTests(): void {
  prefetched = false;
}

/**
 * #4517: intent on a fan card starts the dialog chunk, once per page load. Skipped while the
 * browser asks to save data (#4299). No session check, unlike the Friends drawer: signed-out
 * visitors see the fan. A failure is the dialog's to report.
 */
export function prefetchCardDetail(): void {
  if (prefetched) return;
  if (savingData()) return;
  prefetched = true;
  void cardDetailChunk.load().catch(() => undefined);
}
