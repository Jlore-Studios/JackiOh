// #2757: a Choose one of lanes (Showdown, Meditative #98), answered on the board. Each lane on offer is lit by
// one rounded oblong over its four zones, the far backrow to the near one, measured from those zones' boxes so
// it follows the board's layout on every screen and when the board flips for the second seat. It takes no hit
// (lanePick.css): the zone under it takes the click, and `Prompt.tsx`'s R514 listeners answer it. A lane is
// named by the card's own words, which are its accessible name and, on hover or focus, its tooltip. The arrow
// keys walk the lit lanes (`neighbourZone`, called from Prompt.tsx's key handler). The oblong breathes, and under
// Reduce Motion it stands still.

import { useEffect, useLayoutEffect, useState, type ReactElement } from "react";

import { laneZoneTestids } from "./actions.ts";
import { byTestid, testid } from "./contract.ts";
import { placeOf } from "./drag/targets.ts";
import "./lanePick.css";

/** One lane the player may choose: its number, the option it answers with, and the words the card gives it. */
export type LaneChoice = { lane: number; key: string; label: string; detail?: string };

type Box = { left: number; top: number; width: number; height: number };

/** How far an oblong stands out from the zones it covers, so its ring is not drawn on their edges. */
export const LANE_PICK_OUTSET_PX = 4;
export const LANE_TIP_TESTID = "lane-pick-tip";
const LANE_TIP_WIDTH_PX = 220;
const LANE_TIP_MARGIN_PX = 8;
const ZONE = '[data-testid^="zone-"]';

export function laneOblongTestid(lane: number): string {
  return `lane-pick-${String(lane)}`;
}

/** The box that spans a lane's four zones, or null while one of them is not on the page. */
function measure(lanes: readonly number[]): Map<number, Box> {
  const out = new Map<number, Box>();
  for (const lane of lanes) {
    const ids = laneZoneTestids(lane);
    const rects: DOMRect[] = [];
    for (const id of ids) {
      const zone = byTestid(id);
      if (zone !== null) rects.push(zone.getBoundingClientRect());
    }
    if (rects.length < ids.length) continue;
    const left = Math.min(...rects.map((rect) => rect.left)) - LANE_PICK_OUTSET_PX;
    const top = Math.min(...rects.map((rect) => rect.top)) - LANE_PICK_OUTSET_PX;
    const right = Math.max(...rects.map((rect) => rect.right)) + LANE_PICK_OUTSET_PX;
    const bottom = Math.max(...rects.map((rect) => rect.bottom)) + LANE_PICK_OUTSET_PX;
    out.set(lane, { left, top, width: right - left, height: bottom - top });
  }
  return out;
}

function same(a: ReadonlyMap<number, Box>, b: ReadonlyMap<number, Box>): boolean {
  if (a.size !== b.size) return false;
  for (const [lane, box] of a) {
    const other = b.get(lane);
    if (other === undefined || other.left !== box.left || other.top !== box.top) return false;
    if (other.width !== box.width || other.height !== box.height) return false;
  }
  return true;
}

/**
 * The arrows' step: the zone, in the lit lane next to the one `from` stands in (`step` 1 is right), on the same
 * side and row, or null when there is none (no wrap) or `from` is no zone of a lit lane. Prompt.tsx calls it from
 * R514's one key handler, so the board gains no key handler (R2126000).
 */
export function neighbourZone(lanes: readonly LaneChoice[], from: EventTarget | null, step: 1 | -1): HTMLElement | null {
  const zone = from instanceof Element ? from.closest(ZONE) : null;
  const place = zone === null ? null : placeOf(zone);
  if (place === null) return null;
  const lit = lanes.map((choice) => choice.lane).sort((a, b) => a - b);
  const next = lit[lit.indexOf(place.lane) + step];
  if (next === undefined || !lit.includes(place.lane)) return null;
  const to = byTestid(testid.zone(place.side, place.row, next));
  return to instanceof HTMLElement ? to : null;
}

export default function LanePick(props: { lanes: readonly LaneChoice[]; readOnly: boolean }): ReactElement {
  const { lanes, readOnly } = props;
  const offered = lanes.map((choice) => choice.lane).sort((a, b) => a - b);
  const offeredKey = offered.join(",");
  const [boxes, setBoxes] = useState<ReadonlyMap<number, Box>>(() => new Map());
  const [hovered, setHovered] = useState<number | null>(null);
  const [focused, setFocused] = useState<number | null>(null);

  // The board can move under the oblongs without this component rendering (a resize, an orientation change,
  // a scroll), so it measures after every render and on those events, and sets state only when a box moved.
  useLayoutEffect(() => {
    const next = measure(offered);
    setBoxes((prev) => (same(prev, next) ? prev : next));
  });
  useEffect(() => {
    const lit = offeredKey === "" ? [] : offeredKey.split(",").map(Number);
    const update = (): void => {
      const next = measure(lit);
      setBoxes((prev) => (same(prev, next) ? prev : next));
    };
    window.addEventListener("resize", update);
    window.addEventListener("scroll", update, true);
    return () => {
      window.removeEventListener("resize", update);
      window.removeEventListener("scroll", update, true);
    };
  }, [offeredKey]);

  // Hover and focus are read off the page's zones, as R514's answer is (Prompt.tsx). A replay draws the lanes
  // and answers nothing, so it listens to nothing.
  useEffect(() => {
    if (readOnly) return undefined;
    const lit = offeredKey === "" ? [] : offeredKey.split(",").map(Number);
    const laneAt = (target: EventTarget | null): number | null => {
      if (!(target instanceof Element)) return null;
      const zone = target.closest(ZONE);
      const lane = zone === null ? null : placeOf(zone)?.lane;
      return lane !== null && lane !== undefined && lit.includes(lane) ? lane : null;
    };
    const onOver = (event: PointerEvent): void => setHovered(laneAt(event.target));
    const onOut = (event: PointerEvent): void => {
      if (event.relatedTarget === null) setHovered(null);
    };
    const onFocusIn = (event: FocusEvent): void => setFocused(laneAt(event.target));
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
  }, [offeredKey, readOnly]);

  const active = hovered ?? focused;
  const tipFor = active === null ? undefined : lanes.find((choice) => choice.lane === active);
  const tipBox = active === null ? undefined : boxes.get(active);

  return (
    <div className="lane-pick" data-testid="lane-pick">
      {lanes.map((choice) => {
        const box = boxes.get(choice.lane);
        if (box === undefined) return null;
        const name = choice.detail === undefined ? choice.label : `${choice.label}: ${choice.detail}`;
        return (
          <div
            key={choice.lane}
            className="lane-pick-oblong"
            data-testid={laneOblongTestid(choice.lane)}
            data-lane={choice.lane}
            data-active={active === choice.lane ? "true" : undefined}
            role="img"
            aria-label={name}
            style={{ left: box.left, top: box.top, width: box.width, height: box.height }}
          />
        );
      })}
      {tipFor === undefined || tipBox === undefined ? null : (
        <p
          className="lane-pick-tip"
          data-testid={LANE_TIP_TESTID}
          role="tooltip"
          style={{
            width: LANE_TIP_WIDTH_PX,
            top: tipBox.top + tipBox.height / 2,
            left: Math.max(
              LANE_TIP_MARGIN_PX,
              Math.min(tipBox.left + tipBox.width / 2 - LANE_TIP_WIDTH_PX / 2, window.innerWidth - LANE_TIP_WIDTH_PX - LANE_TIP_MARGIN_PX),
            ),
          }}
        >
          <span className="lane-pick-tip-label">{tipFor.label}</span>
          {tipFor.detail === undefined ? null : <span className="lane-pick-tip-detail">{tipFor.detail}</span>}
        </p>
      )}
    </div>
  );
}
