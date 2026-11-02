// The inspect overlays of a face-down backrow card (R370). With no face to show, they show what the
// view gives (a face-down trap, its cost and its Plague Counters, R471) and that only its setter can
// see it, in the one inspect slot (B23) through `useInspectTrigger`'s `render`. `FaceDownPreview` is
// click-through and aria-hidden like HoverPreview (the back carries its words as its label);
// `FaceDownSheet` is a modal dialog like InspectSheet, closed by a swipe down too (R2121000).

import { useRef } from "react";
import type { ReactElement } from "react";
import { createPortal } from "react-dom";
import { t, useLocale } from "../../i18n/i18n.ts";
import { CardBack } from "../CardBack.tsx";
import { FACE_ASPECT } from "../constants.ts";
import { FACE_DOWN_HINT, FACE_DOWN_TITLE, PLAGUE_COUNT_WORDS, costPhrase } from "../faceDown.ts";
import { FACE_DOWN_PREVIEW_HEIGHT_PX, PREVIEW_GLOSSARY_GAP_PX, PREVIEW_GLOSSARY_WIDTH_PX } from "./constants.ts";
import { placePreview, type PreviewPrefer, type Rect } from "./placement.ts";
import { OVERLAY_ROOT_PROPS, useModalOverlay } from "./store.ts";
import { SheetGrip, useSheetSwipe } from "./useSheetSwipe.tsx";
import { INSPECT_CLOSE, INSPECT_FACE_DOWN, INSPECT_FACE_DOWN_COST, INSPECT_SCRIM } from "./testids.ts";
import "./inspect.css";

type FaceDownProps = { cost: number | undefined; plague?: number };

/** The back, large, with its cost gem when the view gives one. */
function BigBack({ cost }: FaceDownProps): ReactElement {
  return (
    <span className="inspect-facedown-back">
      <CardBack />
      {cost === undefined ? null : (
        <span className="facedown-cost facedown-cost--large" aria-hidden="true">
          {cost}
        </span>
      )}
    </span>
  );
}

/** The words beside it: what it is, its cost and who can see it. */
function Words({ cost, plague }: FaceDownProps): ReactElement {
  useLocale();
  return (
    <span className="inspect-facedown-words">
      <span className="inspect-facedown-title">{t(FACE_DOWN_TITLE)}</span>
      {cost === undefined ? null : (
        <span className="inspect-facedown-cost" data-testid={INSPECT_FACE_DOWN_COST} data-cost={cost}>
          {t(costPhrase(cost))}
        </span>
      )}
      {plague === undefined ? null : (
        <span className="inspect-facedown-plague" data-plague={plague}>
          {t(PLAGUE_COUNT_WORDS, { count: plague })}
        </span>
      )}
      <span className="inspect-facedown-hint">{t(FACE_DOWN_HINT)}</span>
    </span>
  );
}

function viewportSize(): { width: number; height: number } {
  return { width: window.innerWidth, height: window.innerHeight };
}

export function FaceDownPreview({ cost, plague, anchor, prefer = "beside" }: FaceDownProps & { anchor: Rect; prefer?: PreviewPrefer }): ReactElement {
  const size = {
    width: FACE_DOWN_PREVIEW_HEIGHT_PX * FACE_ASPECT + PREVIEW_GLOSSARY_GAP_PX + PREVIEW_GLOSSARY_WIDTH_PX,
    height: FACE_DOWN_PREVIEW_HEIGHT_PX,
  };
  const placed = placePreview(anchor, viewportSize(), size, prefer);
  return createPortal(
    <div
      className="inspect-hover inspect-facedown"
      data-testid={INSPECT_FACE_DOWN}
      data-mode="hover"
      data-side={placed.side}
      aria-hidden="true"
      style={{ position: "fixed", left: placed.left, top: placed.top, pointerEvents: "none" }}
      {...OVERLAY_ROOT_PROPS}
    >
      <span className="inspect-facedown-frame" style={{ height: FACE_DOWN_PREVIEW_HEIGHT_PX }}>
        <BigBack cost={cost} />
      </span>
      <Words cost={cost} plague={plague} />
    </div>,
    document.body,
  );
}

export function FaceDownSheet({ cost, plague, onClose }: FaceDownProps & { onClose: () => void }): ReactElement {
  useLocale();
  const closeButton = useRef<HTMLButtonElement>(null);
  const modal = useModalOverlay(onClose, closeButton);
  const swipe = useSheetSwipe(modal.close);
  return createPortal(
    <div className="inspect-layer inspect-layer--sheet" {...OVERLAY_ROOT_PROPS}>
      <div className="inspect-scrim" ref={swipe.scrimRef} data-testid={INSPECT_SCRIM} aria-hidden="true" {...modal.dismissProps} />
      <div
        className="inspect-sheet inspect-facedown"
        ref={swipe.sheetRef}
        data-testid={INSPECT_FACE_DOWN}
        data-mode="sheet"
        role="dialog"
        aria-modal="true"
        aria-label={t(FACE_DOWN_TITLE)}
        {...swipe.handlers}
      >
        <SheetGrip />
        <div className="inspect-sheet-body">
          <span className="inspect-facedown-frame inspect-facedown-frame--sheet">
            <BigBack cost={cost} />
          </span>
          <Words cost={cost} plague={plague} />
        </div>
        <button ref={closeButton} type="button" className="inspect-close" data-testid={INSPECT_CLOSE} {...modal.dismissProps}>
          {t("Close")}
        </button>
      </div>
    </div>,
    document.body,
  );
}
