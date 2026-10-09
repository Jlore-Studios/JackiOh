// The deck builder's detail view (B29): both faces side by side, a meta line (lines of code last,
// E36), the glossary, and the caller's meta and actions above Close. A modal dialog over a scrim,
// closed by Close, the scrim or Escape (B25); it takes the one inspect slot, and closeInspect() closes it.
//
// A reference in a card's text is a control (R279); the Radiant face and line mark in gold what the
// base face lacks (R277). The rules text is printed again at reading size, since two faces on a
// 390 px phone print a long card at 5 px: inspect.css shows it on narrow, short and long-text
// screens, and on a wide short one puts the info column beside the faces. The actions row is pinned
// under the scrolling body, where focus lands.
//
// After the glossary: the flavour line and artist credit (R660, Flavour.tsx). Last, the card's
// History (R388, patches/CardHistory.tsx), collapsed unless `historyOpen` (the Patch notes page).

import { useLayoutEffect, useRef } from "react";
import type { ReactElement, ReactNode } from "react";
import { createPortal } from "react-dom";
import type { CardDef } from "@jackioh/shared";
import { CardFace } from "../CardFace.tsx";
import { textTier } from "../fit.ts";
import { faceModel, locWords, type FaceModel } from "../model.ts";
import { glossaryFor } from "../rules.ts";
import { RulesText } from "../RulesText.tsx";
import { RefsInteractive } from "../refContext.tsx";
import { CardHistory } from "../../patches/CardHistory.tsx";
import { Flavour } from "./Flavour.tsx";
import { Glossary, mergeGlossary } from "./Glossary.tsx";
import { CardStatsBlock } from "../../stats/CardStatsBlock.tsx";
import { VoicePreview } from "./VoicePreview.tsx";
import { closeInspect, OVERLAY_ROOT_PROPS, registerDetail, useModalOverlay } from "./store.ts";
import {
  INSPECT_CLOSE,
  INSPECT_DETAIL,
  INSPECT_FACE_BASE,
  INSPECT_FACE_RADIANT,
  INSPECT_SCRIM,
} from "./testids.ts";
import "./inspect.css";

export type CardDetailProps = {
  def: CardDef;
  onClose: () => void;
  actions?: ReactNode;
  meta?: ReactNode;
  /** R388: the History section starts open (the Patch notes page); collapsed when absent. */
  historyOpen?: boolean;
  /** SPEC §9.11, R654: render the compact card statistics block (deckbuilder and almanac). */
  showStats?: boolean;
};

/**
 * `#<index> · <set> · <rarity> · <type>`, then tags and ` · N lines of code` (E36) when present. A
 * Radiant face with a type of its own (B2.7, Classic+ #22) says so: "Trap (Radiant: Field Trap)".
 */
export function detailMetaLine(def: CardDef): string {
  const radiantType = def.radiant.type;
  const type = radiantType !== undefined && radiantType !== def.type ? `${def.type} (Radiant: ${radiantType})` : def.type;
  const parts = [`#${def.index}`, def.set, def.rarity, type];
  if (def.tags.length > 0) parts.push(def.tags.join(", "));
  if (def.loc !== undefined) parts.push(locWords(def.loc));
  return parts.join(" · ");
}

/** Text tiers whose printed face shrinks far enough to want the reading-size copy on any screen. */
const LONG_TEXT_TIERS: ReadonlySet<string> = new Set(["xl", "xxl"]);

/**
 * The rules at reading size: the base text, then the Radiant face's whole text with what the base
 * lacks marked (R277); a Radiant text equal to the base one is not printed twice.
 */
function DetailRules({ base, radiant }: { base: FaceModel; radiant: FaceModel }): ReactElement | null {
  const radiantLine = radiant.text.full === base.text.full ? null : radiant.text;
  if (base.text.full === "" && radiantLine === null) return null;
  const long = LONG_TEXT_TIERS.has(textTier(base.text.full)) || LONG_TEXT_TIERS.has(textTier(radiant.text.full));
  return (
    <div className="inspect-rules" data-long={long ? "true" : "false"}>
      {base.text.full === "" ? null : (
        <p className="inspect-rules-line">
          <span className="inspect-rules-label">Base</span>
          <span className="inspect-rules-text">
            <RulesText text={base.text.full} refs={base.refs} />
          </span>
        </p>
      )}
      {radiantLine === null ? null : (
        <p className="inspect-rules-line inspect-rules-line--radiant">
          <span className="inspect-rules-label">Radiant</span>
          <span className="inspect-rules-text">
            <RulesText text={radiantLine.full} marks={radiantLine.marks} refs={radiant.refs} />
          </span>
        </p>
      )}
    </div>
  );
}

export function CardDetail({ def, onClose, actions, meta, historyOpen = false, showStats = false }: CardDetailProps): ReactElement {
  const closeButton = useRef<HTMLButtonElement>(null);
  const modal = useModalOverlay(onClose, closeButton);
  const onCloseRef = useRef(onClose);

  useLayoutEffect(() => {
    onCloseRef.current = onClose;
  });

  // Take the inspect slot: close what is open, then let closeInspect() reach this dialog.
  useLayoutEffect(() => {
    closeInspect();
    return registerDetail(`inspect-detail:${def.id}`, () => onCloseRef.current());
  }, [def.id]);

  const base = faceModel({ defId: def.id, def, radiant: false });
  const radiant = faceModel({ defId: def.id, def, radiant: true });
  const glossary = mergeGlossary(glossaryFor(base), glossaryFor(radiant));

  return createPortal(
    <div className="inspect-layer inspect-layer--detail" {...OVERLAY_ROOT_PROPS}>
      <div className="inspect-scrim" data-testid={INSPECT_SCRIM} aria-hidden="true" {...modal.dismissProps} />
      <div
        className="inspect-detail"
        data-testid={INSPECT_DETAIL}
        data-card={def.id}
        role="dialog"
        aria-modal="true"
        aria-label={def.name}
      >
        {/* The faces and everything about them scroll; the actions row is pinned under them, so
            Add and Close are on screen as the dialog opens, whatever the card's length. */}
        <div className="inspect-detail-body">
          <RefsInteractive>
            <div className="inspect-detail-faces">
              <figure className="inspect-detail-face">
                <div className="inspect-face inspect-face--detail" data-testid={INSPECT_FACE_BASE}>
                  <CardFace face={base} layout="full" />
                </div>
                <figcaption className="inspect-detail-caption">Base</figcaption>
              </figure>
              <figure className="inspect-detail-face inspect-detail-face--radiant">
                <div className="inspect-face inspect-face--detail" data-testid={INSPECT_FACE_RADIANT}>
                  <CardFace face={radiant} layout="full" />
                </div>
                <figcaption className="inspect-detail-caption">Radiant</figcaption>
              </figure>
            </div>
            {/* Everything but the faces, as one column: under the faces on a tall screen, beside
                them on a wide, short one such as a 1280x720 desktop (inspect.css). */}
            <div className="inspect-detail-info">
              <p className="inspect-meta" data-loc={def.loc}>
                {detailMetaLine(def)}
              </p>
              <DetailRules base={base} radiant={radiant} />
              <Glossary entries={glossary} />
              <Flavour defId={def.id} />
              {showStats ? <CardStatsBlock key={`stats-${def.id}`} cardId={def.id} /> : null}
              {meta === undefined || meta === null ? null : <div className="inspect-detail-meta">{meta}</div>}
              <CardHistory key={def.id} cardId={def.id} initiallyOpen={historyOpen} />
            </div>
          </RefsInteractive>
        </div>
        <div className="inspect-actions">
          {actions}
          <VoicePreview defId={def.id} />
          <button
            ref={closeButton}
            type="button"
            className="inspect-close"
            data-testid={INSPECT_CLOSE}
            {...modal.dismissProps}
          >
            Close
          </button>
        </div>
      </div>
    </div>,
    document.body,
  );
}
