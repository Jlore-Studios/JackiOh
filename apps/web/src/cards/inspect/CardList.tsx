// Lists display public graveyards/exile piles or the viewer's library (§10.8; R310, R313) in caller
// order, using inspect's sole slot (B23). Entries are faces/counts or unopened backs (R312).
// B5 E11: the board supplies `play` only for `legalActions`; this client reports it and closes the
// sheet (CLAUDE.md rule 7).

import { useLayoutEffect, useRef, useState } from "react";
import type { ReactElement } from "react";
import { createPortal } from "react-dom";
import { CardBack } from "../CardBack.tsx";
import { CardFace } from "../CardFace.tsx";
import { FACE_ASPECT } from "../constants.ts";
import type { FaceModel } from "../model.ts";
import { glossaryFor } from "../rules.ts";
import {
  LIST_PREVIEW_COLUMNS,
  LIST_PREVIEW_FACE_HEIGHT_PX,
  LIST_PREVIEW_GAP_PX,
  LIST_PREVIEW_HEADER_PX,
  LIST_PREVIEW_MAX,
  LIST_PREVIEW_PADDING_PX,
} from "./constants.ts";
import { Glossary } from "./Glossary.tsx";
import { Printed } from "./Printed.tsx";
import { placePreview, type PreviewPrefer, type Rect } from "./placement.ts";
import { OVERLAY_ROOT_PROPS, useModalOverlay } from "./store.ts";
import {
  INSPECT_CLOSE,
  INSPECT_FACE,
  INSPECT_LIST_BACK,
  INSPECT_LIST_CARD,
  INSPECT_LIST_COUNT,
  INSPECT_LIST_DETAIL,
  INSPECT_LIST_HOVER,
  INSPECT_LIST_MORE,
  INSPECT_LIST_SHEET,
  INSPECT_SCRIM,
} from "./testids.ts";
import "./inspect.css";
import "./list-play.css";

/** One face in the list, or a card back (`face: null`, R312). `key` is stable for what it shows; `count` is 1 when absent. */
export type CardListEntry = { key: string; face: FaceModel | null; count?: number; play?: CardListPlay };

/** B5 E11: a card the viewer may play from this pile now. `testId` is the button's (`pile-play-<id>`, game/contract.ts). */
export type CardListPlay = { testId: string; glow: boolean; onPlay: () => void };

const PLAY_WORD = "Play";

export type CardListProps = {
  /** What the list is, in words: "Your graveyard". */
  title: string;
  entries: readonly CardListEntry[];
  /** What the order of the entries is, in words; a pile's is "Newest first". */
  order?: string;
};

/** A pile's entries come newest first (the board's `Pile`). */
const DEFAULT_ORDER = "Newest first";

/** What a card back in the list is called (R312). */
const UNKNOWN_NAME = "Unknown card";

function cardsWord(count: number): string {
  return count === 1 ? "1 card" : `${String(count)} cards`;
}

function countOf(entry: CardListEntry): number {
  return entry.count ?? 1;
}

function totalOf(entries: readonly CardListEntry[]): number {
  return entries.reduce((sum, entry) => sum + countOf(entry), 0);
}

function nameOf(entry: CardListEntry): string {
  return entry.face?.name ?? UNKNOWN_NAME;
}

function entryLabel(entry: CardListEntry): string {
  const count = countOf(entry);
  return count > 1 ? `${String(count)} × ${nameOf(entry)}` : nameOf(entry);
}

function EntryFace({ entry }: { entry: CardListEntry }): ReactElement {
  const count = countOf(entry);
  return (
    <>
      {entry.face === null ? <CardBack /> : <CardFace face={entry.face} layout="full" />}
      {count > 1 ? (
        <span className="inspect-list-badge" aria-hidden="true">
          ×{count}
        </span>
      ) : null}
    </>
  );
}

/** B5 E11: "Play" under a face the viewer may play from the pile. */
function PlayButton({ entry, play, onClose }: { entry: CardListEntry; play: CardListPlay; onClose: () => void }): ReactElement {
  return (
    <button
      type="button"
      className="inspect-list-play"
      data-testid={play.testId}
      data-legal="true"
      data-glow={play.glow ? "ready" : undefined}
      aria-label={`${PLAY_WORD} ${nameOf(entry)}`}
      onClick={() => {
        play.onPlay();
        onClose();
      }}
    >
      {PLAY_WORD}
    </button>
  );
}

/** The preview's size before layout, from the numbers inspect.css uses. */
function estimatedSize(shown: number, more: boolean): { width: number; height: number } {
  const columns = Math.max(1, Math.min(LIST_PREVIEW_COLUMNS, shown));
  const rows = Math.max(1, Math.ceil(shown / columns));
  const faceWidth = LIST_PREVIEW_FACE_HEIGHT_PX * FACE_ASPECT;
  const width = columns * faceWidth + (columns - 1) * LIST_PREVIEW_GAP_PX + 2 * LIST_PREVIEW_PADDING_PX;
  const height =
    LIST_PREVIEW_HEADER_PX * (more ? 2 : 1) +
    rows * LIST_PREVIEW_FACE_HEIGHT_PX +
    rows * LIST_PREVIEW_GAP_PX +
    2 * LIST_PREVIEW_PADDING_PX;
  return { width, height };
}

function viewportSize(): { width: number; height: number } {
  return { width: window.innerWidth, height: window.innerHeight };
}

export function CardListPreview({
  title,
  entries,
  order = DEFAULT_ORDER,
  anchor,
  prefer = "beside",
}: CardListProps & { anchor: Rect; prefer?: PreviewPrefer }): ReactElement {
  const ref = useRef<HTMLDivElement>(null);
  const shown = entries.slice(0, LIST_PREVIEW_MAX);
  const total = totalOf(entries);
  // The cards the faces left out, not the faces: a grouped entry stands for several.
  const more = total - totalOf(shown);
  const placed = placePreview(anchor, viewportSize(), estimatedSize(shown.length, more > 0), prefer);

  // Once laid out, place it again by its real size. jsdom has no layout and keeps the estimate.
  useLayoutEffect(() => {
    const element = ref.current;
    if (element === null) return;
    const width = element.offsetWidth;
    const height = element.offsetHeight;
    if (width === 0 || height === 0) return;
    const measured = placePreview(anchor, viewportSize(), { width, height }, prefer);
    element.style.left = `${String(measured.left)}px`;
    element.style.top = `${String(measured.top)}px`;
    element.dataset.side = measured.side;
  });

  return createPortal(
    <div
      ref={ref}
      className="inspect-list-hover"
      data-testid={INSPECT_LIST_HOVER}
      data-side={placed.side}
      aria-hidden="true"
      style={{ position: "fixed", left: placed.left, top: placed.top, pointerEvents: "none" }}
      {...OVERLAY_ROOT_PROPS}
    >
      <span className="inspect-list-head">
        <span className="inspect-list-title">{title}</span>
        <span className="inspect-list-count" data-testid={INSPECT_LIST_COUNT} data-count={total}>
          {cardsWord(total)}
        </span>
        <span className="inspect-list-order">{order}</span>
      </span>
      <span className="inspect-list-grid">
        {shown.map((entry) => (
          <span
            key={entry.key}
            className={entry.face === null ? "inspect-list-face inspect-list-face--unknown" : "inspect-list-face"}
            data-testid={INSPECT_LIST_CARD}
            data-def-name={entry.face?.name ?? ""}
            data-count={countOf(entry)}
            data-unknown={entry.face === null ? "true" : undefined}
          >
            <EntryFace entry={entry} />
          </span>
        ))}
      </span>
      {more > 0 ? (
        <span className="inspect-list-more" data-testid={INSPECT_LIST_MORE}>
          +{more} more · click to see them all
        </span>
      ) : null}
    </div>,
    document.body,
  );
}

/** B5 E11: a grid face opens large, with "Play" when permitted. */
function FaceTile({
  entry,
  face,
  onOpen,
  onClose,
}: {
  entry: CardListEntry;
  face: FaceModel;
  onOpen: () => void;
  onClose: () => void;
}): ReactElement {
  const button = (
    <button
      type="button"
      className="inspect-list-face inspect-list-face--button"
      data-testid={INSPECT_LIST_CARD}
      data-def-name={face.name}
      data-count={countOf(entry)}
      data-entry-key={entry.key}
      aria-label={`${entryLabel(entry)}: show it large`}
      onClick={onOpen}
    >
      <EntryFace entry={entry} />
    </button>
  );
  if (entry.play === undefined) return button;
  return (
    <span className="inspect-list-playable" data-playable="true">
      {button}
      <PlayButton entry={entry} play={entry.play} onClose={onClose} />
    </span>
  );
}

export function CardListSheet({
  title,
  entries,
  order = DEFAULT_ORDER,
  onClose,
}: CardListProps & { onClose: () => void }): ReactElement {
  const closeButton = useRef<HTMLButtonElement>(null);
  const backButton = useRef<HTMLButtonElement>(null);
  const grid = useRef<HTMLUListElement>(null);
  const modal = useModalOverlay(onClose, closeButton);
  const [open, setOpen] = useState<string | null>(null);
  // A back opens nothing: it has no face to show large.
  const openedEntry = open === null ? undefined : entries.find((entry) => entry.key === open);
  const opened: FaceModel | undefined = openedEntry?.face ?? undefined;
  // B5 E11: the face opened large keeps its "Play" while it is listed.
  const openedPlay = opened === undefined ? undefined : openedEntry?.play;
  const total = totalOf(entries);
  /** The face last opened, so Back refocuses it rather than dropping focus on <body>. */
  const returnTo = useRef<string | null>(null);

  // Changing views removes the focused button, so move focus to Back or its returning tile.
  useLayoutEffect(() => {
    if (opened !== undefined) {
      backButton.current?.focus({ preventScroll: true });
      return;
    }
    const key = returnTo.current;
    if (key === null) return;
    returnTo.current = null;
    const tile = [...(grid.current?.querySelectorAll<HTMLButtonElement>("[data-entry-key]") ?? [])].find(
      (element) => element.dataset.entryKey === key,
    );
    tile?.focus();
  }, [opened]);

  return createPortal(
    <div className="inspect-layer inspect-layer--list" {...OVERLAY_ROOT_PROPS}>
      <div className="inspect-scrim" data-testid={INSPECT_SCRIM} aria-hidden="true" {...modal.dismissProps} />
      <div className="inspect-list-sheet" data-testid={INSPECT_LIST_SHEET} role="dialog" aria-modal="true" aria-label={title}>
        <header className="inspect-list-head">
          <h2 className="inspect-list-title">{title}</h2>
          <span className="inspect-list-count" data-testid={INSPECT_LIST_COUNT} data-count={total}>
            {cardsWord(total)}
          </span>
          <span className="inspect-list-order">{order}</span>
        </header>

        {opened === undefined ? (
          <ul ref={grid} className="inspect-list-grid inspect-list-grid--sheet">
            {entries.map((entry) => (
              <li key={entry.key} className="inspect-list-item">
                {entry.face === null ? (
                  <span
                    className="inspect-list-face inspect-list-face--unknown"
                    data-testid={INSPECT_LIST_CARD}
                    data-def-name=""
                    data-count={countOf(entry)}
                    data-unknown="true"
                    role="img"
                    aria-label={entryLabel(entry)}
                  >
                    <EntryFace entry={entry} />
                  </span>
                ) : (
                  <FaceTile entry={entry} face={entry.face} onOpen={() => setOpen(entry.key)} onClose={onClose} />
                )}
              </li>
            ))}
          </ul>
        ) : (
          <div className="inspect-list-detail" data-testid={INSPECT_LIST_DETAIL}>
            <div className="inspect-face inspect-face--list" data-testid={INSPECT_FACE}>
              <CardFace face={opened} layout="full" />
            </div>
            <Printed face={opened} />
            <Glossary entries={glossaryFor(opened)} />
          </div>
        )}

        <div className="inspect-list-actions">
          {openedPlay !== undefined && openedEntry !== undefined ? (
            <PlayButton entry={openedEntry} play={openedPlay} onClose={onClose} />
          ) : null}
          {opened !== undefined ? (
            <button
              ref={backButton}
              type="button"
              className="inspect-close inspect-list-back"
              data-testid={INSPECT_LIST_BACK}
              onClick={() => {
                returnTo.current = open;
                setOpen(null);
              }}
            >
              All cards
            </button>
          ) : null}
          <button ref={closeButton} type="button" className="inspect-close" data-testid={INSPECT_CLOSE} {...modal.dismissProps}>
            Close
          </button>
        </div>
      </div>
    </div>,
    document.body,
  );
}
