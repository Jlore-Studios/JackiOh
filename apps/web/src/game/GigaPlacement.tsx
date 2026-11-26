// #4995: a playable GIGA card in hand lights all four zones of each offered placement, not
// only its anchor (SPEC §3.2). One green rounded oblong per offered anchor, measured from those
// zones' boxes exactly as `LanePick.tsx` measures its lanes, so it follows the layout and the
// second seat's flipped board. It takes no hit (gigaPlacement.css): the anchor zone under it
// takes the click, the focus and the drop. Client only (CLAUDE.md rule 7): the anchors come
// from `Highlight.glow`, never from re-deciding legality.

import { useContext, useEffect, useLayoutEffect, useState, type ReactElement } from "react";

import type { PlayerView, Row } from "@jackioh/shared";

import { CatalogContext } from "./catalog.ts";
import { type Highlight } from "./contract.ts";
import { placeOf } from "./drag/targets.ts";
import { anchorOfTestid, gigaFootprintTestids, isGigaHandCard, type GigaAnchor } from "./giga.ts";
import { LANE_PICK_OUTSET_PX, measureBoxes, type Box } from "./LanePick.tsx";
import "./gigaPlacement.css";

const HAND_CARD = "hand-card-";
const ZONE = '[data-testid^="zone-"]';

/** The oblong over the 2×2 block a GIGA played at this anchor would take. */
export function gigaOblongTestid(row: Row, lane: number): string {
  return `giga-placement-${row}-${lane}`;
}

function keyOf(anchor: GigaAnchor): string {
  return `${anchor.row}:${anchor.lane}`;
}

/** The oblongs over every anchor whose four zones are all on the page. */
function boxesOf(anchors: readonly GigaAnchor[]): ReadonlyMap<string, Box> {
  const out = new Map<string, Box>();
  for (const anchor of anchors) {
    const box = unionOf(gigaFootprintTestids("you", anchor));
    if (box !== null) out.set(keyOf(anchor), box);
  }
  return out;
}

/** The union box of the anchor's four zones, pushed out by the oblong's outset. */
function unionOf(ids: readonly string[]): Box | null {
  const boxes = measureBoxes(ids);
  if (boxes.size < ids.length) return null;
  const rects = [...boxes.values()];
  const left = Math.min(...rects.map((rect) => rect.left)) - LANE_PICK_OUTSET_PX;
  const top = Math.min(...rects.map((rect) => rect.top)) - LANE_PICK_OUTSET_PX;
  const right = Math.max(...rects.map((rect) => rect.left + rect.width)) + LANE_PICK_OUTSET_PX;
  const bottom = Math.max(...rects.map((rect) => rect.top + rect.height)) + LANE_PICK_OUTSET_PX;
  return { left, top, width: right - left, height: bottom - top };
}

export default function GigaPlacement(props: { view: PlayerView; highlight?: Highlight }): ReactElement | null {
  const { view, highlight } = props;
  const lookup = useContext(CatalogContext);

  // Graveyard "Play" selections (`pile-play-<id>`) get no oblongs: only a card picked up from the hand does.
  const selected = [...(highlight?.selected ?? [])].filter((id) => id.startsWith(HAND_CARD));
  const gigaId = selected.find((id) => isGigaHandCard(view, lookup, id.slice(HAND_CARD.length)));

  const anchors: GigaAnchor[] =
    gigaId === undefined
      ? []
      : [...(highlight?.glow ?? [])]
          .map((id) => anchorOfTestid(id))
          .filter((anchor): anchor is GigaAnchor => anchor !== null);
  const anchorsKey = anchors.map(keyOf).join(",");

  const [hovered, setHovered] = useState<string | null>(null);
  const [focused, setFocused] = useState<string | null>(null);

  // The board can move under the oblongs without this component rendering (a resize, an
  // orientation change, a scroll), so those events re-render for a fresh measure below.
  // The measure itself runs during the render: the zones stand in the committed DOM, so the
  // boxes are current on the render that draws them, with no state to cascade.
  const [, bumpLayout] = useState(0);
  useLayoutEffect(() => {
    const update = (): void => bumpLayout((version) => version + 1);
    window.addEventListener("resize", update);
    window.addEventListener("scroll", update, true);
    return () => {
      window.removeEventListener("resize", update);
      window.removeEventListener("scroll", update, true);
    };
  }, []);
  const boxes = boxesOf(anchors);

  // Hover and focus are read off the page's zones, as `LanePick.tsx` reads them. Only the anchor
  // under the pointer or focus lights its block alone; the rest dim beside it.
  useEffect(() => {
    if (gigaId === undefined || anchorsKey === "") return undefined;
    const lit = new Set(anchorsKey.split(","));
    const anchorAt = (target: EventTarget | null): string | null => {
      if (!(target instanceof Element)) return null;
      const zone = target.closest(ZONE);
      if (zone === null || placeOf(zone) === null) return null;
      const id = zone.getAttribute("data-testid") ?? "";
      const anchor = anchorOfTestid(id);
      if (anchor === null) return null;
      const key = keyOf(anchor);
      return lit.has(key) ? key : null;
    };
    const onOver = (event: PointerEvent): void => setHovered(anchorAt(event.target));
    const onOut = (event: PointerEvent): void => {
      if (event.relatedTarget === null) setHovered(null);
    };
    const onFocusIn = (event: FocusEvent): void => setFocused(anchorAt(event.target));
    const onFocusOut = (): void => setFocused(null);
    document.addEventListener("pointerover", onOver);
    document.addEventListener("pointerout", onOut);
    document.addEventListener("focusin", onFocusIn);
    document.addEventListener("focusout", onFocusOut);
    return () => {
      document.removeEventListener("pointerover", onOver);
      document.removeEventListener("pointerout", onOut);
      document.removeEventListener("focusin", onFocusIn);
      document.removeEventListener("focusout", onFocusOut);
      setHovered(null);
      setFocused(null);
    };
  }, [anchorsKey, gigaId]);

  if (gigaId === undefined || boxes.size === 0) return null;
  const active = hovered ?? focused;
  return (
    <div className="giga-placement" data-testid="giga-placement">
      {anchors.map((anchor) => {
        const box = boxes.get(keyOf(anchor));
        if (box === undefined) return null;
        const key = keyOf(anchor);
        return (
          <div
            key={key}
            className="giga-placement-oblong"
            data-testid={gigaOblongTestid(anchor.row, anchor.lane)}
            data-active={active === key ? "true" : undefined}
            data-dim={active !== null && active !== key ? "true" : undefined}
            role="img"
            aria-label={`GIGA placement: ${anchor.row} lanes ${anchor.lane} and ${anchor.lane + 1}`}
            style={{ left: box.left, top: box.top, width: box.width, height: box.height }}
          />
        );
      })}
    </div>
  );
}
