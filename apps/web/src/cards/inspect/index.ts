// Inspect: hover preview, touch sheet and the deck builder's detail view (docs/polish/6-cards.md,
// Surface C). cards/index.ts re-exports the public names from here; nothing inside cards/ imports
// this barrel but inspect's own tests. It holds only the names some importer takes through it.

export { useInspectTrigger } from "./useInspectTrigger.tsx";
export type { InspectOptions, InspectOverlayState, InspectRenderSubject, InspectSubject } from "./useInspectTrigger.tsx";
export { closeInspect } from "./store.ts";
export { INSPECT_KEY_SHORTCUT, isInspectKey } from "./keys.ts";
export { CardDetail } from "./CardDetail.tsx";
export { CardListPreview, CardListSheet } from "./CardList.tsx";
export { FaceDownPreview, FaceDownSheet } from "./FaceDown.tsx";
export type { CardListEntry } from "./CardList.tsx";
export { placePreview } from "./placement.ts";
export type { Rect } from "./placement.ts";
export { HOVER_DELAY_MS, LIST_PREVIEW_MAX, LONG_PRESS_MS } from "./constants.ts";
export {
  INSPECT_CLOSE,
  INSPECT_DETAIL,
  INSPECT_FACE,
  INSPECT_FACE_BASE,
  INSPECT_FACE_DOWN,
  INSPECT_FACE_DOWN_COST,
  INSPECT_FACE_RADIANT,
  INSPECT_HOVER,
  INSPECT_PRINTED,
  INSPECT_LIST_BACK,
  INSPECT_LIST_CARD,
  INSPECT_LIST_COUNT,
  INSPECT_LIST_DETAIL,
  INSPECT_LIST_HOVER,
  INSPECT_LIST_MORE,
  INSPECT_LIST_SHEET,
  INSPECT_NOTE,
  INSPECT_SHEET,
} from "./testids.ts";
