// The enlarged card a resting mouse or pen pointer opens (B22), fixed beside the anchor. It takes no
// pointer events and is aria-hidden, so it never covers what a click aims at; for the same reason
// nothing in it can be hovered, so the cards its text names get their own column (References.tsx,
// R279) and badge states are put in words (StateNotes.tsx: tuned R386, Brittle R385, enchantments
// E39, a Unit R383; SPEC §10.8, §10.10) above the printed text, glossary, flavour (R660) and lines
// of code (E36).
//
// #1017: a face that prints less than its whole text has it on its left (Extended.tsx), grouped so it
// stays there whichever side the preview opens on; a Summit adds its next Ascent line (R280).
// R2126000: a trigger with a `keyHint` says under the face how to inspect by key (the key works only
// from the focused trigger, never a hovered one); without one the face stays the group's last child.

import { useLayoutEffect, useRef } from "react";
import type { ReactElement } from "react";
import { createPortal } from "react-dom";
import { CardFace, faceText } from "../CardFace.tsx";
import { FACE_ASPECT } from "../constants.ts";
import { useFullArt, useOwnCopy } from "../fullArt.ts";
import type { FaceModel } from "../model.ts";
import { glossaryFor } from "../rules.ts";
import {
  PREVIEW_EXTENDED_WIDTH_PX,
  PREVIEW_GLOSSARY_GAP_PX,
  PREVIEW_GLOSSARY_WIDTH_PX,
  PREVIEW_HEIGHT_PX,
  PREVIEW_MAX_VIEWPORT_SHARE,
  PREVIEW_REFS_WIDTH_PX,
} from "./constants.ts";
import { Extended } from "./Extended.tsx";
import { Glossary } from "./Glossary.tsx";
import { placePreview, type PreviewPrefer, type Rect } from "./placement.ts";
import { Flavour, showsEasterEgg } from "./Flavour.tsx";
import { flavourFor } from "../flavour.ts";
import { InspectNote } from "./InspectNote.tsx";
import { Printed } from "./Printed.tsx";
import { LocLine, StateNotes, hasStateNotes } from "./StateNotes.tsx";
import { namedCards, References } from "./References.tsx";
import { useDefResolver } from "../refContext.tsx";
import { t } from "../../i18n/i18n.ts";
import { ShortcutHint } from "../../shortcuts/Kbd.tsx";
import type { Shortcut } from "../../shortcuts/shortcuts.ts";
import { ascentLevelOf } from "../shortText.ts";
import { OVERLAY_ROOT_PROPS } from "./store.ts";
import { INSPECT_FACE, INSPECT_HOVER } from "./testids.ts";
import "./inspect.css";

type HoverPreviewProps = { face: FaceModel; anchor: Rect; prefer?: PreviewPrefer; note?: string; keyHint?: Shortcut };

function viewportSize(): { width: number; height: number } {
  return { width: window.innerWidth, height: window.innerHeight };
}

/** The preview's size before it has been laid out, from the same numbers inspect.css uses. */
function estimatedSize(withGlossary: boolean, withRefs: boolean, withExtended: boolean): { width: number; height: number } {
  const height = Math.min(PREVIEW_HEIGHT_PX, window.innerHeight * PREVIEW_MAX_VIEWPORT_SHARE);
  const cardWidth = height * FACE_ASPECT;
  const glossary = withGlossary ? PREVIEW_GLOSSARY_GAP_PX + PREVIEW_GLOSSARY_WIDTH_PX : 0;
  const refs = withRefs ? PREVIEW_GLOSSARY_GAP_PX + PREVIEW_REFS_WIDTH_PX : 0;
  const extended = withExtended ? PREVIEW_GLOSSARY_GAP_PX + PREVIEW_EXTENDED_WIDTH_PX : 0;
  return { width: cardWidth + glossary + refs + extended, height };
}

export function HoverPreview({ face, anchor, prefer = "beside", note, keyHint }: HoverPreviewProps): ReactElement {
  const ref = useRef<HTMLDivElement>(null);
  const entries = glossaryFor(face);
  const resolve = useDefResolver();
  const fullArtOn = useFullArt(face.defId);
  // R1655000: the viewer's own card's preview wears its cosmetics, as it prints its full art.
  const own = useOwnCopy();
  const named = resolve === null ? 0 : namedCards(face, resolve).length;
  const notes = hasStateNotes(face);
  const flavoured = face.known && (flavourFor(face.defId) !== null || showsEasterEgg(face));
  const side = notes || face.printed !== null || flavoured;
  const ascentLevel = ascentLevelOf(face.values);
  const extended = faceText(face, ascentLevel, fullArtOn).less;
  const placed = placePreview(anchor, viewportSize(), estimatedSize(entries.length > 0 || side, named > 0, extended), prefer);

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

  const faceBox = (
    <div className="inspect-face" data-testid={INSPECT_FACE} style={{ height: PREVIEW_HEIGHT_PX }}>
      <CardFace face={face} layout="full" ascentLevel={ascentLevel} worn={own} />
      <InspectNote note={note} />
    </div>
  );

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
      <div className="inspect-facegroup">
        <Extended face={face} ascentLevel={ascentLevel} />
        {keyHint === undefined ? (
          faceBox
        ) : (
          <div className="inspect-face-column">
            {faceBox}
            <ShortcutHint keys={[keyHint]} words={t("Focus the card, then press {keys} to inspect")} />
          </div>
        )}
      </div>
      {entries.length > 0 || side ? (
        <div className="inspect-side">
          {notes ? <StateNotes face={face} /> : null}
          <Printed face={face} />
          <Glossary entries={entries} />
          {flavoured ? <Flavour defId={face.defId} easter={face.flagged === true} /> : null}
          <LocLine face={face} />
        </div>
      ) : null}
      <References face={face} />
    </div>,
    document.body,
  );
}
