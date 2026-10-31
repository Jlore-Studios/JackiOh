// The bottom sheet a touch long-press opens (B24, B25) over a scrim: focus moves to Close on open and
// back on close; Escape, the scrim, Close and a swipe down (#2121, R2121000) all close it. Here a
// reference in the text is a control (R279). Beside the face: its states in words (StateNotes.tsx:
// tuned R386, Brittle R385, enchantments E39, a Unit R383; SPEC §10.8), its printed text where it
// differs (§10.10), the glossary, flavour (R660) and lines of code (E36). #1017: a face that prints
// less than its whole text has it under it, and a Summit its next Ascent line (R280).

import { useRef } from "react";
import type { ReactElement } from "react";
import { createPortal } from "react-dom";
import { t, useLocale } from "../../i18n/i18n.ts";
import { CardFace } from "../CardFace.tsx";
import { useOwnCopy } from "../fullArt.ts";
import type { FaceModel } from "../model.ts";
import { glossaryFor } from "../rules.ts";
import { ascentLevelOf } from "../shortText.ts";
import { Extended } from "./Extended.tsx";
import { Flavour } from "./Flavour.tsx";
import { Glossary } from "./Glossary.tsx";
import { InspectNote } from "./InspectNote.tsx";
import { Printed } from "./Printed.tsx";
import { LocLine, StateNotes } from "./StateNotes.tsx";
import { RefsInteractive } from "../refContext.tsx";
import { OVERLAY_ROOT_PROPS, useModalOverlay } from "./store.ts";
import { SheetGrip, useSheetSwipe } from "./useSheetSwipe.tsx";
import { INSPECT_CLOSE, INSPECT_FACE, INSPECT_SCRIM, INSPECT_SHEET } from "./testids.ts";
import "./inspect.css";

type InspectSheetProps = { face: FaceModel; onClose: () => void; note?: string };

export function InspectSheet({ face, onClose, note }: InspectSheetProps): ReactElement {
  useLocale();
  const closeButton = useRef<HTMLButtonElement>(null);
  const modal = useModalOverlay(onClose, closeButton);
  const swipe = useSheetSwipe(modal.close);
  const ascentLevel = ascentLevelOf(face.values);
  // R1655000: the viewer's own card's sheet wears its cosmetics, as it prints its full art.
  const own = useOwnCopy();

  return createPortal(
    <div className="inspect-layer inspect-layer--sheet" {...OVERLAY_ROOT_PROPS}>
      <div className="inspect-scrim" ref={swipe.scrimRef} data-testid={INSPECT_SCRIM} aria-hidden="true" {...modal.dismissProps} />
      <div
        className="inspect-sheet"
        ref={swipe.sheetRef}
        data-testid={INSPECT_SHEET}
        role="dialog"
        aria-modal="true"
        aria-label={face.name}
        {...swipe.handlers}
      >
        <SheetGrip />
        <div className="inspect-sheet-body">
          <RefsInteractive>
            <div className="inspect-face inspect-face--sheet" data-testid={INSPECT_FACE}>
              <CardFace face={face} layout="full" ascentLevel={ascentLevel} worn={own} />
              <InspectNote note={note} />
            </div>
            <Extended face={face} ascentLevel={ascentLevel} />
            <StateNotes face={face} />
            <Printed face={face} />
          </RefsInteractive>
          <Glossary entries={glossaryFor(face)} />
          {face.known ? <Flavour defId={face.defId} easter={face.flagged === true} /> : null}
          <LocLine face={face} />
        </div>
        <button
          ref={closeButton}
          type="button"
          className="inspect-close"
          data-testid={INSPECT_CLOSE}
          {...modal.dismissProps}
        >
          {t("Close")}
        </button>
      </div>
    </div>,
    document.body,
  );
}
