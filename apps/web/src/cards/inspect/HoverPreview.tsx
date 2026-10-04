// The enlarged card a resting mouse or pen pointer opens (B22): the live face at
// PREVIEW_HEIGHT_PX with its glossary beside it, fixed beside the anchor card. It never takes
// pointer events and is hidden from assistive tech, so it can never cover what a click aims at.
// A face in play whose printed text differs (SPEC §10.10) has that text above its glossary, and a
// face whose text names other cards has their faces in a column of their own beyond it
// (References.tsx, R279), since a reference inside a preview that takes no pointer events cannot
// be hovered itself.
//
// Patch v0.2.0 (SPEC §10.8): a face in play whose view gives it states — tuned (R386), Brittle (R385),
// enchantments (E39), standing as a Unit (R383) — has them in words at the top of that column
// (StateNotes.tsx), since the badges' tooltips cannot be hovered here, and a printed text beside a
// tuned face whose numbers moved. Whenever the column is drawn it ends with the card's lines of code
// (E36): a meta line fits there, and a face with nothing beside it stays alone. Above that line, the
// card's flavour line and artist credit (R658, Flavour.tsx), which draw the column on their own.

import { useLayoutEffect, useRef } from "react";
import type { ReactElement } from "react";
import { createPortal } from "react-dom";
import { CardFace } from "../CardFace.tsx";
import { FACE_ASPECT } from "../constants.ts";
import type { FaceModel } from "../model.ts";
import { glossaryFor } from "../rules.ts";
import {
  PREVIEW_GLOSSARY_GAP_PX,
  PREVIEW_GLOSSARY_WIDTH_PX,
  PREVIEW_HEIGHT_PX,
  PREVIEW_MAX_VIEWPORT_SHARE,
  PREVIEW_REFS_WIDTH_PX,
} from "./constants.ts";
import { Glossary } from "./Glossary.tsx";
import { placePreview, type PreviewPrefer, type Rect } from "./placement.ts";
import { Flavour } from "./Flavour.tsx";
import { flavourFor } from "../flavour.ts";
import { InspectNote } from "./InspectNote.tsx";
import { Printed } from "./Printed.tsx";
import { LocLine, StateNotes, hasStateNotes } from "./StateNotes.tsx";
import { namedCards, References } from "./References.tsx";
import { useDefResolver } from "../refContext.tsx";
import { OVERLAY_ROOT_PROPS } from "./store.ts";
import { INSPECT_FACE, INSPECT_HOVER } from "./testids.ts";
import "./inspect.css";

type HoverPreviewProps = { face: FaceModel; anchor: Rect; prefer?: PreviewPrefer; note?: string };

function viewportSize(): { width: number; height: number } {
  return { width: window.innerWidth, height: window.innerHeight };
}

/** The preview's size before it has been laid out, from the same numbers inspect.css uses. */
function estimatedSize(withGlossary: boolean, withRefs: boolean): { width: number; height: number } {
  const height = Math.min(PREVIEW_HEIGHT_PX, window.innerHeight * PREVIEW_MAX_VIEWPORT_SHARE);
  const cardWidth = height * FACE_ASPECT;
  const glossary = withGlossary ? PREVIEW_GLOSSARY_GAP_PX + PREVIEW_GLOSSARY_WIDTH_PX : 0;
  const refs = withRefs ? PREVIEW_GLOSSARY_GAP_PX + PREVIEW_REFS_WIDTH_PX : 0;
  return { width: cardWidth + glossary + refs, height };
}

export function HoverPreview({ face, anchor, prefer = "beside", note }: HoverPreviewProps): ReactElement {
  const ref = useRef<HTMLDivElement>(null);
  const entries = glossaryFor(face);
  const resolve = useDefResolver();
  const named = resolve === null ? 0 : namedCards(face, resolve).length;
  const notes = hasStateNotes(face);
  const flavoured = face.known && flavourFor(face.defId) !== null;
  const side = notes || face.printed !== null || flavoured;
  const placed = placePreview(anchor, viewportSize(), estimatedSize(entries.length > 0 || side, named > 0), prefer);

  // Once laid out, place it again by its real size. jsdom has no layout and keeps the estimate.
  useLayoutEffect(() => {
    const element = ref.current;
    if (element === null) return;
    const width = element.offsetWidth;
    const height = element.offsetHeight;
    if (width === 0 || height === 0) return;
    const measured = placePreview(anchor, viewportSize(), { width, height }, prefer);
    element.style.left = `${measured.left}px`;
    element.style.top = `${measured.top}px`;
    element.dataset.side = measured.side;
  });

  return createPortal(
    <div
      ref={ref}
      className="inspect-hover"
      data-testid={INSPECT_HOVER}
      data-side={placed.side}
      aria-hidden="true"
      style={{ position: "fixed", left: placed.left, top: placed.top, pointerEvents: "none" }}
      {...OVERLAY_ROOT_PROPS}
    >
      <div className="inspect-face" data-testid={INSPECT_FACE} style={{ height: PREVIEW_HEIGHT_PX }}>
        <CardFace face={face} layout="full" />
        <InspectNote note={note} />
      </div>
      {entries.length > 0 || side ? (
        <div className="inspect-side">
          {notes ? <StateNotes face={face} /> : null}
          <Printed face={face} />
          <Glossary entries={entries} />
          {flavoured ? <Flavour defId={face.defId} /> : null}
          <LocLine face={face} />
        </div>
      ) : null}
      <References face={face} />
    </div>,
    document.body,
  );
}
