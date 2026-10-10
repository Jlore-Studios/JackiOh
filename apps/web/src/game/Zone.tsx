// Field zone view (SPEC §3; BUILD M5-T1).
//
// It reads `locks[row][laneIndex(lane)]` for `data-locked` (BUILD M5-T4); play legality comes from
// the engine's `legalActions`.
//
// DragLayer hit-tests zones by testid; no drag handler belongs here.

import type { ReactElement } from "react";

import type { PlayerView, Row } from "@jackioh/shared";

/**
 * Missing flags from old fixtures or servers read as "not set", not a board crash.
 */
function flagAt(flags: { units: boolean[]; backrow: boolean[] } | undefined, row: Row, lane: number): boolean {
  if (flags === undefined) return false;
  const i = laneIndex(lane);
  return (row === "units" ? flags.units[i] : flags.backrow[i]) === true;
}

import Backrow from "./Backrow.tsx";
import Card, { cx, isLegal, isSelected, legalAttr, type Pops } from "./Card.tsx";
import {
  laneIndex,
  sideView,
  testid,
  touchHoldMode,
  type AnimatingMap,
  type ClickTarget,
  type Highlight,
  type Side,
} from "./contract.ts";
import { glowAttr } from "./glow.ts";
import LockIcon from "./LockIcon.tsx";

export type ZoneProps = {
  view: PlayerView;
  side: Side;
  row: Row;
  lane: number;
  highlight?: Highlight;
  animating?: AnimatingMap;
  onClick?: (target: ClickTarget) => void;
  pops?: ReadonlyMap<string, Pops>;
};

export default function Zone(props: ZoneProps): ReactElement {
  const { view, side, row, lane } = props;
  const seat = sideView(view, side);
  const locked = flagAt(seat.locks, row, lane);
  // R64: a zone held for a dying Reborn unit takes no summon either, so it is drawn as held —
  // but it is not Locked, and only a lock gets `data-locked` (BUILD M5-T4 `locked`).
  const reserved = flagAt(seat.reserved, row, lane);

  const testId = testid.zone(side, row, lane);
  const legal = isLegal(props.highlight, testId);
  const selected = isSelected(props.highlight, testId);
  const target: ClickTarget = { on: "zone", side, row, lane };

  // Lanes are 1-based (`contract.ts`, matching the engine); the view's arrays are 0-based.
  const unit = row === "units" ? (seat.units[laneIndex(lane)] ?? null) : null;
  const backrow = row === "backrow" ? (seat.backrow[laneIndex(lane)] ?? null) : null;

  return (
    <div
      className={cx("zone", `zone-${row}`, locked && "zone-locked", reserved && "zone-reserved")}
      data-testid={testId}
      data-side={side}
      data-row={row}
      data-lane={lane}
      data-locked={locked ? "true" : undefined}
      data-reserved={reserved ? "true" : undefined}
      data-legal={legalAttr(legal)}
      data-glow={glowAttr(props.highlight, testId)}
      data-selected={selected ? "true" : undefined}
      data-animating={props.animating?.get(testId)}
      aria-disabled={legal ? undefined : "true"}
      aria-label={`${side} ${row} lane ${lane}`}
      tabIndex={legal ? 0 : undefined}
      onClick={() => {
        if (!legal) return;
        props.onClick?.(target);
      }}
      onKeyDown={(event) => {
        if (event.key !== "Enter" && event.key !== " ") return;
        event.preventDefault();
        if (!legal) return;
        props.onClick?.(target);
      }}
    >
      {locked && <LockIcon />}
      {reserved && !locked && <span className="lock-icon reserved-icon" aria-label="Reserved zone" title="Held for a Reborn unit" />}
      {unit !== null && (
        <Card
          testId={testid.card(unit.instanceId)}
          card={unit}
          unit={unit}
          switchTarget
          target={{ on: "unit", instanceId: unit.instanceId, side, lane }}
          touchHold={touchHoldMode(view)}
          highlight={props.highlight}
          animating={props.animating}
          onClick={props.onClick}
          pops={props.pops?.get(testid.card(unit.instanceId))}
        />
      )}
      {row === "backrow" && (
        <Backrow
          entry={backrow}
          side={side}
          lane={lane}
          touchHold={touchHoldMode(view)}
          highlight={props.highlight}
          animating={props.animating}
          onClick={props.onClick}
          pops={props.pops}
        />
      )}
    </div>
  );
}
