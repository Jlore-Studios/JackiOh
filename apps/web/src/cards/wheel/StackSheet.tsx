// A Stack pile opened as a wheel (issue #124, §3.2): the pile's top card with its face, every
// dormant card under it as a back (the view names their number only, never which they are), so
// what is above and below what is obvious. The dialog borrows the inspect layer's scrim and modal
// behaviour (focus trap, scrim and Escape dismiss, focus restore).

import { useRef, type ReactElement } from "react";
import { createPortal } from "react-dom";

import type { FaceModel } from "../model.ts";
import { INSPECT_CLOSE, INSPECT_SCRIM } from "../inspect/testids.ts";
import { OVERLAY_ROOT_PROPS, useModalOverlay } from "../inspect/store.ts";
import CardWheel, { type WheelItem } from "./CardWheel.tsx";

import "./wheel.css";

/** Test ids of the stack sheet. */
export const stackTestid = {
  sheet: "stack-sheet",
  title: "stack-title",
  count: "stack-count",
} as const;

/** What the sheet calls a card it may not name (R312's word for a back). */
export const STACK_UNKNOWN_NAME = "Unknown card";

/** What every Stack pile's sheet is called (§3.2). */
export const STACK_TITLE = "Stack pile";

/** The words under each place on the wheel. */
export const STACK_NOTES = {
  top: "Top of pile",
  buried: "Buried",
  bottom: "Bottom of pile",
} as const;

export type StackSheetProps = {
  /** What the pile is, in words: "Stack pile". */
  title: string;
  /** The pile's top card, or null for a back on top (a face-down pile names nothing). */
  top: FaceModel | null;
  /** What a screen reader calls the top card. */
  topName: string;
  /** How many dormant cards lie under it. */
  buried: number;
  onClose: () => void;
};

/** The wheel's items for a pile: its top, then one back per buried card. */
export function stackItems(top: FaceModel | null, topName: string, buried: number): WheelItem[] {
  const items: WheelItem[] = [{ key: "top", face: top, name: topName, note: STACK_NOTES.top }];
  for (let k = 0; k < buried; k += 1) {
    const last = k === buried - 1;
    items.push({
      key: `buried-${String(k)}`,
      face: null,
      name: STACK_UNKNOWN_NAME,
      note: last ? STACK_NOTES.bottom : STACK_NOTES.buried,
    });
  }
  return items;
}

export default function StackSheet({ title, top, topName, buried, onClose }: StackSheetProps): ReactElement {
  const closeButton = useRef<HTMLButtonElement>(null);
  const modal = useModalOverlay(onClose, closeButton);
  const items = stackItems(top, topName, buried);
  return createPortal(
    <div className="inspect-layer inspect-layer--list" {...OVERLAY_ROOT_PROPS}>
      <div className="inspect-scrim" data-testid={INSPECT_SCRIM} aria-hidden="true" {...modal.dismissProps} />
      <div className="inspect-list-sheet stack-sheet" data-testid={stackTestid.sheet} role="dialog" aria-modal="true" aria-label={title}>
        <header className="stack-sheet-head">
          <h2 className="stack-sheet-title" data-testid={stackTestid.title}>
            {title}
          </h2>
          <span className="inspect-list-count" data-testid={stackTestid.count} data-count={items.length}>
            {items.length === 1 ? "1 card" : `${String(items.length)} cards`}
          </span>
        </header>
        <CardWheel items={items} ariaLabel={title} />
        <button ref={closeButton} type="button" className="inspect-close" data-testid={INSPECT_CLOSE} {...modal.dismissProps}>
          Close
        </button>
      </div>
    </div>,
    document.body,
  );
}
