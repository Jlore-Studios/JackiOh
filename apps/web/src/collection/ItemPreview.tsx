// R2730000 (#2730): the Collection's fullscreen preview of one held or inspected item, the
// only place a held token is spent. A hold (LONG_PRESS_MS, any pointer) or the inspect key opens
// it on any account cosmetic tile, card-cosmetic strip item or owned-cards grid card; a plain tap
// or click still equips or opens, as today, and the hold's click is swallowed. This decides
// nothing (CLAUDE.md rule 7): the preview draws the server's answers and a token's offer sends
// intent through `net/api.ts`. R2127000: Back closes it as its ✕ does. R3221000: it is a `dialog`
// drawn through `Dialog.tsx`, whose keys, scrim and ✕ it takes.
//
// The client never enforces rules and never sees hidden information (CLAUDE.md rule 7).

import { useRef, type KeyboardEvent, type MouseEvent, type PointerEvent, type ReactElement } from "react";

import { isInspectKey, useInspectTrigger } from "../cards/index.ts";
import { Dialog } from "../game/Dialog.tsx";
import { t } from "../i18n/i18n.ts";
import { collectionTestid as T } from "./collection.ts";

import "./collection.css";

export type PreviewHold = {
  onPointerDown: (event: PointerEvent<HTMLElement>) => void;
  onPointerMove: (event: PointerEvent<HTMLElement>) => void;
  onPointerUp: (event: PointerEvent<HTMLElement>) => void;
  onPointerCancel: (event: PointerEvent<HTMLElement>) => void;
  onContextMenu: (event: MouseEvent<HTMLElement>) => void;
  onClickCapture: (event: MouseEvent<HTMLElement>) => void;
  onKeyDown: (event: KeyboardEvent<HTMLElement>) => void;
};

const NO_OVERLAY = (): null => null;

/** R2730000: a hold (LONG_PRESS_MS, any pointer) or the inspect key calls `onPreview`; the hold's click is swallowed. */
export function usePreviewHold(key: string, onPreview: (pointerId?: number | null) => void): PreviewHold {
  const pressed = useRef<HTMLElement | null>(null);
  const { handlers } = useInspectTrigger(
    { key: `collection-preview-${key}`, render: NO_OVERLAY },
    { hover: false, holdAnyPointer: true, onLongPress: (pointerId) => { pressed.current?.focus({ preventScroll: true }); onPreview(pointerId); } },
  );
  return {
    onPointerDown: (event) => { pressed.current = event.currentTarget; handlers.onPointerDown(event); },
    onPointerMove: handlers.onPointerMove,
    onPointerUp: handlers.onPointerUp,
    onPointerCancel: handlers.onPointerCancel,
    onContextMenu: handlers.onContextMenu,
    onClickCapture: handlers.onClickCapture,
    onKeyDown: (event) => { if (!isInspectKey(event)) return; event.preventDefault(); onPreview(); },
  };
}

/**
 * R2730000: one item fullscreen, modal, focus trapped: its picture large, its name, its rarity,
 * whether it is owned or locked, the token's offer where one covers it, and the ✕. R3016000: `controls`
 * sit under the picture and its name, outside the picture's `aria-hidden` (a grid card's Radiant toggle).
 */
export default function ItemPreview({
  id,
  name,
  rarity,
  owned,
  picture,
  controls = null,
  offer,
  onClose,
}: {
  id: string;
  name: string;
  rarity: string;
  owned: boolean;
  picture: ReactElement;
  controls?: ReactElement | null;
  offer: ReactElement | null;
  onClose: () => void;
}): ReactElement {
  // R3221000: the ✕ takes the focus as the preview opens, and the focus goes back to the item that opened
  // it as it closes (Dialog.tsx); Escape, Back and the scrim close it as the ✕ does.
  return (
    <Dialog
      role="dialog"
      testid={T.preview}
      data={{ "data-item": id, "data-owned": String(owned) }}
      scrimTestid={T.previewScrim}
      closeTestid={T.previewClose}
      className="collection-preview__panel"
      header={
        <div className="collection-preview__picture" aria-hidden="true">
          {picture}
        </div>
      }
      title={name}
      onClose={onClose}
      actions={[]}
    >
      {controls}
      <p className="collection-preview__meta">{t(rarity)}</p>
      <p className="collection-preview__meta" data-testid={T.previewStatus}>
        {owned ? t("Owned") : t("Locked")}
      </p>
      {offer}
    </Dialog>
  );
}
