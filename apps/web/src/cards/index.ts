// The card presentation module's barrel (docs/polish/6-cards.md, Surface): only the names code outside
// `src/cards/` takes through it, which may also import a `cards/` module by path. Inside `src/cards/`,
// files import each other by path, never through this barrel, so it cannot create an import cycle.

export { faceModel } from "./model.ts";
export type { FaceModel, InPlay, RolledPower } from "./model.ts";
export { ALL_TRIBES } from "./frame.ts";

export { CardDefsProvider } from "./refContext.tsx";
export { TERM_TOOLTIP_TESTID, TermTips } from "./TermTip.tsx";

// The face model's data, for specs (jackioh-shared's files).
export { LONG_FACES, POWER_WORDS } from "./faceData.ts";

export { KEYWORD_MARK } from "./keywordMarks.ts";

export { FACE_ASPECT } from "./constants.ts";

// E21: a backrow pile's depth, which game/Backrow.tsx draws beside the card.
export { PileDepth } from "./CardStates.tsx";

export { CardFace, faceText } from "./CardFace.tsx";
export { RulesText } from "./RulesText.tsx";
export { KeywordIcons, MinionFace } from "./MinionFace.tsx";
export { CardBack } from "./CardBack.tsx";
export { Icon } from "./icons.tsx";
export type { IconName } from "./icons.tsx";

// R370, R371: the words a face-down backrow card is drawn with.
export { FACE_DOWN_TAG, REVEAL_AT_LABEL, REVEAL_AT_NOTE, UNREVEALED_NOTE, costPhrase, faceDownLabel } from "./faceDown.ts";

// Slice A: art.
export { CardArt } from "./art/index.ts";

// Slice C: inspect.
export {
  CardDetail,
  CardListPreview,
  CardListSheet,
  closeInspect,
  FaceDownPreview,
  FaceDownSheet,
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
  INSPECT_KEY_SHORTCUT,
  isInspectKey,
  useInspectTrigger,
} from "./inspect/index.ts";
export type { CardListEntry, InspectOverlayState, InspectRenderSubject, InspectSubject } from "./inspect/index.ts";

// R3344001: Tilt and queue hold state
export { isHeld, useTiltHold } from "./tilt.ts";

// Slice C: card settings, which task 7's panel mounts at integration.
export { CARD_SETTINGS_DEFAULTS, useCardSettings, writeCardSettings } from "./settings.ts";

// R3345000: Full-screen art view and trigger
export {
  canOpenFullScreen,
  closeFullScreenArt,
  openFullScreenArt,
  useFullScreenArt,
  useFullScreenTrigger,
  type FullScreenState,
  type FullScreenTriggerOptions,
} from "./fullscreen.ts";
export { FullScreenArt, FullScreenArtHost, type FullScreenArtProps } from "./FullScreenArt.tsx";
