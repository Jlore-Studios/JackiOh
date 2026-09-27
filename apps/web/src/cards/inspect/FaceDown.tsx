// The inspect overlays of a face-down backrow card (R370): what a resting mouse, a long-press, a tap
// or Enter opens on a back. There is no face to show, so they show what the view does give — that
// it is a face-down trap and, since v0.1.1, its cost — and say that only the player who set it can
// see what it is. They take the one inspect slot like a card's preview and sheet (B23), through
// `useInspectTrigger`'s `render`.
//
// - `FaceDownPreview`: fixed beside the back, click-through and hidden from assistive tech, like
//   HoverPreview; the back itself carries the same words as its label.
// - `FaceDownSheet`: a modal dialog with Close, as InspectSheet is (useModalOverlay).

import { useRef } from "react";
import type { ReactElement } from "react";
import { createPortal } from "react-dom";
import { CardBack } from "../CardBack.tsx";
import { FACE_ASPECT } from "../constants.ts";
import { FACE_DOWN_HINT, FACE_DOWN_TITLE, costPhrase } from "../faceDown.ts";
import { FACE_DOWN_PREVIEW_HEIGHT_PX, PREVIEW_GLOSSARY_GAP_PX, PREVIEW_GLOSSARY_WIDTH_PX } from "./constants.ts";
import { placePreview, type PreviewPrefer, type Rect } from "./placement.ts";
import { OVERLAY_ROOT_PROPS, useModalOverlay } from "./store.ts";
import { INSPECT_CLOSE, INSPECT_FACE_DOWN, INSPECT_FACE_DOWN_COST, INSPECT_SCRIM } from "./testids.ts";
import "./inspect.css";

type FaceDownProps = { cost: number | undefined };

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
function Words({ cost }: FaceDownProps): ReactElement {
  return (
    <span className="inspect-facedown-words">
      <span className="inspect-facedown-title">{FACE_DOWN_TITLE}</span>
      {cost === undefined ? null : (
        <span className="inspect-facedown-cost" data-testid={INSPECT_FACE_DOWN_COST} data-cost={cost}>
          {costPhrase(cost)}
        </span>
      )}
      <span className="inspect-facedown-hint">{FACE_DOWN_HINT}</span>
    </span>
  );
}

function viewportSize(): { width: number; height: number } {
  return { width: window.innerWidth, height: window.innerHeight };
}

export function FaceDownPreview({ cost, anchor, prefer = "beside" }: FaceDownProps & { anchor: Rect; prefer?: PreviewPrefer }): ReactElement {
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
      <Words cost={cost} />
    </div>,
    document.body,
  );
}

export function FaceDownSheet({ cost, onClose }: FaceDownProps & { onClose: () => void }): ReactElement {
  const closeButton = useRef<HTMLButtonElement>(null);
  const modal = useModalOverlay(onClose, closeButton);
  return createPortal(
    <div className="inspect-layer inspect-layer--sheet" {...OVERLAY_ROOT_PROPS}>
      <div className="inspect-scrim" data-testid={INSPECT_SCRIM} aria-hidden="true" {...modal.dismissProps} />
      <div
        className="inspect-sheet inspect-facedown"
        data-testid={INSPECT_FACE_DOWN}
        data-mode="sheet"
        role="dialog"
        aria-modal="true"
        aria-label={FACE_DOWN_TITLE}
      >
        <span className="inspect-grip" aria-hidden="true" />
        <div className="inspect-sheet-body">
          <span className="inspect-facedown-frame inspect-facedown-frame--sheet">
            <BigBack cost={cost} />
          </span>
          <Words cost={cost} />
        </div>
        <button ref={closeButton} type="button" className="inspect-close" data-testid={INSPECT_CLOSE} {...modal.dismissProps}>
          Close
        </button>
      </div>
    </div>,
    document.body,
  );
}
