// The deck builder's detail view (B29): the printed faces one at a time — base, Radiant, then each
// card the text names, paged like Marvel Snap's related cards (issue #37) — a meta line (its lines of
// code last, E36), the glossary of both faces, the card's voice lines, and the caller's meta and
// actions above Close. It is a centred modal dialog over a scrim, closed by Close, the scrim or
// Escape (B25), and it takes the one inspect slot: opening it closes any hover preview or sheet, and
// closeInspect() closes it.
//
// Both faces and the reading-size rules are where a reference in a card's text is a control (R279):
// hovering, focusing or tapping a name shows the card it names. The Radiant face and the Radiant
// line mark what the base face does not have in gold (R277).
//
// Under the faces, the rules text is printed again at reading size, both faces' together, so the
// Radiant changes read without paging; inspect.css shows this block on narrow screens and short ones,
// and on any screen for a card long enough to shrink hard. On a wide screen up to 900 px tall (a
// 1280x720 desktop, a phone on its side) the face stands at the height the screen allows, with the
// rest of the dialog in a column beside it. The actions row (the caller's actions and Close) is
// pinned under the scrolling body, so it is visible the moment the dialog opens, which is also where
// focus lands.
//
// Last in the column, the card's History (R388, patches/CardHistory.tsx): collapsed under a
// "History" control, it loads the card's patch history when opened and lists each patch that changed
// the card, newest first, with its faces as that patch left them and what changed marked. The
// Patch notes page opens the detail with it already open (`historyOpen`).

import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { ReactElement, ReactNode } from "react";
import { createPortal } from "react-dom";
import type { CardDef } from "@jackioh/shared";
import { INSPECT_VOICE_DELAY_MS, VOICE_PRIORITY } from "../../audio/constants.ts";
import { getAudioEngine } from "../../audio/engine.ts";
import type { VoiceLineKind } from "../../audio/types.ts";
import { VOICE_LINES, lineFor } from "../../audio/voiceData.ts";
import { CardFace } from "../CardFace.tsx";
import { textTier } from "../fit.ts";
import { faceModel, locWords, type FaceModel } from "../model.ts";
import { useDefResolver } from "../refContext.tsx";
import { namedCards } from "./References.tsx";
import { CAROUSEL_SWIPE_PX } from "./constants.ts";
import { glossaryFor } from "../rules.ts";
import { RulesText } from "../RulesText.tsx";
import { RefsInteractive } from "../refContext.tsx";
import { CardHistory } from "../../patches/CardHistory.tsx";
import { Glossary, mergeGlossary } from "./Glossary.tsx";
import { closeInspect, OVERLAY_ROOT_PROPS, registerDetail, useModalOverlay } from "./store.ts";
import {
  INSPECT_CLOSE,
  INSPECT_CAROUSEL,
  INSPECT_CAROUSEL_NEXT,
  INSPECT_CAROUSEL_POSITION,
  INSPECT_CAROUSEL_PREVIOUS,
  INSPECT_DETAIL,
  INSPECT_FACE_BASE,
  INSPECT_FACE_RADIANT,
  INSPECT_SCRIM,
} from "./testids.ts";
import "./inspect.css";

const VOICE_LINE_BUTTONS: readonly { kind: VoiceLineKind; label: string }[] = [
  { kind: "play", label: "Play voice line" },
  { kind: "death", label: "Death voice line" },
  { kind: "cast", label: "Cast voice line" },
];

export type CardDetailProps = {
  def: CardDef;
  onClose: () => void;
  actions?: ReactNode;
  meta?: ReactNode;
  /** R388: the History section starts open (the Patch notes page); collapsed when absent. */
  historyOpen?: boolean;
};

/**
 * `#<index> · <set> · <rarity> · <type>`, then ` · <tags>` when there are any, then ` · N lines of
 * code` (E36) when the card's script was counted. A face with a type of its own (B2.7, Classic+ #22
 * Blood Moon's Radiant Field Trap) says so after the type: "Trap (Radiant: Field Trap)".
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

type DetailFace = {
  key: string;
  caption: string;
  testId?: string;
  relatedId?: string;
  face: FaceModel;
};

/**
 * What the detail pages through (issue #37): the card's two printed faces, then each card either
 * face's text names (R279's `refs`), in the order the text names them, on the face it names: "a
 * Radiant Rush Token" is the token's Radiant face (`namedCards`, as the hover preview reads them).
 * Only what the card names, never the cards that name it, so a token's detail stays its own.
 */
function detailFaces(base: FaceModel, radiant: FaceModel, resolve: ((id: string) => CardDef | undefined) | null): DetailFace[] {
  const faces: DetailFace[] = [
    { key: "base", caption: "Base", testId: INSPECT_FACE_BASE, face: base },
    { key: "radiant", caption: "Radiant", testId: INSPECT_FACE_RADIANT, face: radiant },
  ];
  if (resolve === null) return faces;
  const seen = new Set<string>();
  for (const named of [...namedCards(base, resolve), ...namedCards(radiant, resolve)]) {
    const key = `related:${named.def.id}:${named.radiant ? "radiant" : "base"}`;
    if (seen.has(key)) continue;
    seen.add(key);
    faces.push({
      key,
      caption: named.radiant ? `Radiant ${named.def.name}` : named.def.name,
      relatedId: named.def.id,
      face: faceModel({ defId: named.def.id, def: named.def, radiant: named.radiant }),
    });
  }
  return faces;
}

/**
 * The faces one at a time, as Marvel Snap pages a card's related cards: ‹ and ›, the arrow keys, or a
 * horizontal swipe across the face (a touch or pen drag of at least CAROUSEL_SWIPE_PX) move by one,
 * wrapping, and "n of N" says where. Every card has at least its base face and its Radiant face.
 */
function RelatedCardCarousel({ def, base, radiant }: { def: CardDef; base: FaceModel; radiant: FaceModel }): ReactElement {
  const resolve = useDefResolver();
  const faces = detailFaces(base, radiant, resolve);
  const [active, setActive] = useState(0);
  const swipeFrom = useRef<number | null>(null);
  useEffect(() => setActive(0), [def.id]);
  const index = Math.min(active, faces.length - 1);
  const step = (by: number): void => setActive((current) => (current + by + faces.length) % faces.length);
  const paged = faces.length > 1;

  return (
    <div
      className="inspect-detail-carousel"
      data-testid={INSPECT_CAROUSEL}
      role="group"
      aria-roledescription="carousel"
      aria-label={`${def.name}: faces and related cards`}
      onKeyDown={(event) => {
        if (!paged || (event.key !== "ArrowLeft" && event.key !== "ArrowRight")) return;
        event.preventDefault();
        step(event.key === "ArrowLeft" ? -1 : 1);
      }}
    >
      <div
        className="inspect-detail-faces"
        onPointerDown={(event) => {
          // A swipe is a finger's or a pen's; a mouse has the buttons and the arrow keys, and its drag
          // stays a text selection.
          swipeFrom.current = event.pointerType === "mouse" ? null : event.clientX;
        }}
        onPointerUp={(event) => {
          const from = swipeFrom.current;
          swipeFrom.current = null;
          if (!paged || from === null) return;
          const moved = event.clientX - from;
          if (Math.abs(moved) >= CAROUSEL_SWIPE_PX) step(moved < 0 ? 1 : -1);
        }}
        onPointerCancel={() => {
          swipeFrom.current = null;
        }}
      >
        {faces.map((entry, entryIndex) => (
          <figure
            key={entry.key}
            className={`inspect-detail-face${entry.face.radiant ? " inspect-detail-face--radiant" : ""}`}
            hidden={entryIndex !== index}
            data-related-id={entry.relatedId}
          >
            <div className="inspect-face inspect-face--detail" {...(entry.testId === undefined ? {} : { "data-testid": entry.testId })}>
              <CardFace face={entry.face} layout="full" />
            </div>
            <figcaption className="inspect-detail-caption">{entry.caption}</figcaption>
          </figure>
        ))}
      </div>
      {paged && (
        <div className="inspect-carousel-controls">
          <button type="button" data-testid={INSPECT_CAROUSEL_PREVIOUS} aria-label="Previous face" onClick={() => step(-1)}>
            ‹
          </button>
          <span data-testid={INSPECT_CAROUSEL_POSITION} aria-live="polite">
            {index + 1} of {faces.length}
          </span>
          <button type="button" data-testid={INSPECT_CAROUSEL_NEXT} aria-label="Next face" onClick={() => step(1)}>
            ›
          </button>
        </div>
      )}
    </div>
  );
}

/** Every authored line this card has, each played only after the click has unlocked Web Audio. */
function VoiceLineButtons({ defId }: { defId: string }): ReactElement | null {
  const lines = VOICE_LINE_BUTTONS.filter(({ kind }) => lineFor(VOICE_LINES, defId, kind) !== null);
  if (lines.length === 0) return null;
  return (
    <span className="inspect-voice-lines" aria-label="Voice lines">
      {lines.map(({ kind, label }) => (
        <button
          key={kind}
          type="button"
          className="inspect-voice-line"
          data-testid={`inspect-voice-${kind}`}
          onClick={() => {
            const audio = getAudioEngine();
            audio.unlock();
            audio.playVoice(defId, kind, INSPECT_VOICE_DELAY_MS, VOICE_PRIORITY.summon);
          }}
        >
          {label}
        </button>
      ))}
    </span>
  );
}

/**
 * The rules at reading size: the base text, then the Radiant face's whole text with what the base
 * face does not have marked (R277). A Radiant text equal to the base one (a card whose Radiant face
 * changes only its stats) is not printed twice.
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

export function CardDetail({ def, onClose, actions, meta, historyOpen = false }: CardDetailProps): ReactElement {
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
            <RelatedCardCarousel def={def} base={base} radiant={radiant} />
            {/* Everything but the faces, as one column: under the faces on a tall screen, beside
                them on a wide, short one such as a 1280x720 desktop (inspect.css). */}
            <div className="inspect-detail-info">
              <p className="inspect-meta" data-loc={def.loc}>
                {detailMetaLine(def)}
              </p>
              <DetailRules base={base} radiant={radiant} />
              <Glossary entries={glossary} />
              {meta === undefined || meta === null ? null : <div className="inspect-detail-meta">{meta}</div>}
              <CardHistory key={def.id} cardId={def.id} initiallyOpen={historyOpen} />
            </div>
          </RefsInteractive>
        </div>
        <div className="inspect-actions">
          <VoiceLineButtons defId={def.id} />
          {actions}
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
