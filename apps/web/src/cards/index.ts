// The card presentation module's barrel (docs/polish/6-cards.md, Surface): only the names code outside
// `src/cards/` takes through it, which may also import a `cards/` module by path. Inside `src/cards/`,
// files import each other by path, never through this barrel, so it cannot create an import cycle.

export { ALL_TRIBES, faceModel } from "./model.ts";
export type { FaceModel, InPlay } from "./model.ts";

export { CardDefsProvider } from "./refContext.tsx";
export { TERM_TOOLTIP_TESTID, TermTips } from "./TermTip.tsx";

// What a face in play prints where play and print part ways (SPEC §10.10).
export { CONCEALED_TEXT, POWER_WORDS, VANILLA_TEXT } from "./inPlay.ts";
export type { RolledPower } from "./inPlay.ts";

export { LONG_FACES, ascentLevelOf } from "./shortText.ts";

export { GLOSSARY, KEYWORD_MARK } from "./glossary.ts";

export { FACE_ASPECT } from "./constants.ts";

// E21: a backrow pile's depth, which game/Backrow.tsx draws beside the card.
export { PileDepth } from "./CardStates.tsx";

export { CardFace, faceText } from "./CardFace.tsx";
export { RulesText } from "./RulesText.tsx";
export { MinionFace } from "./MinionFace.tsx";
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

// Slice C: card settings, which task 7's panel mounts at integration.
export { CARD_SETTINGS_DEFAULTS, useCardSettings, writeCardSettings } from "./settings.ts";
