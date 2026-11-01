// Field zone view (SPEC §3; BUILD M5-T1). Play legality comes from the engine's `legalActions`.
// DragLayer hit-tests zones by testid; no drag handler belongs here.

import { useContext, type ReactElement } from "react";

import type { PlayerView, Row } from "@jackioh/shared";

/** Missing flags from old fixtures or servers read as "not set", not a board crash. */
function flagAt(flags: { units: boolean[]; backrow: boolean[] } | undefined, row: Row, lane: number): boolean {
  if (flags === undefined) return false;
  const i = laneIndex(lane);
  return (row === "units" ? flags.units[i] : flags.backrow[i]) === true;
}

import Backrow from "./Backrow.tsx";
import Card, { cx, isLegal, isSelected, legalAttr, type Pops } from "./Card.tsx";
import { CatalogContext, type CardLookup } from "./catalog.ts";
import {
  laneIndex,
  sideView,
  testid,
  type AnimatingMap,
  type ClickTarget,
  type Highlight,
  type Side,
} from "./contract.ts";
import { listedFace } from "./faces.ts";
import { glowAttr } from "./glow.ts";
import LockIcon from "./LockIcon.tsx";
import { t } from "../i18n/i18n.ts";

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

function zoneLabel(side: Side, row: Row, lane: number): string {
  if (side === "you") return row === "units" ? t("Your units, lane {lane}", { lane }) : t("Your backrow, lane {lane}", { lane });
  return row === "units" ? t("Enemy units, lane {lane}", { lane }) : t("Enemy backrow, lane {lane}", { lane });
}

/**
 * What a held zone's icon says it waits for. B3.1 rule 6, R383: a backrow zone is held for the animated
 * card on its side that goes back to it (`UnitView.animated.home`), named as its face names it (it
 * stands on the field, so its name is public, CLAUDE.md rule 7); any other hold is R64's, for Reborn.
 */
function heldFor(lookup: CardLookup | null, view: PlayerView, side: Side, row: Row, lane: number): string {
  const returning = row === "backrow" ? (sideView(view, side).units.find((unit) => unit?.animated?.home === lane) ?? null) : null;
  const face = returning === null ? null : listedFace(lookup, view, returning);
  return face === null ? t("Held for a Reborn unit") : t("Held for {name} to return", { name: face.name });
}

export default function Zone(props: ZoneProps): ReactElement {
  const lookup = useContext(CatalogContext);
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
  // WCAG 4.1.2 (#757): a zone the player may pick is a button, unless a card in it takes focus of
  // its own, which a button may not hold; otherwise it is a named group of what it holds.
  const role = legal && unit === null && backrow === null ? "button" : "group";

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
      role={role}
      aria-disabled={legal ? undefined : "true"}
      aria-label={zoneLabel(side, row, lane)}
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
      {reserved && !locked && <span className="lock-icon reserved-icon" aria-label={t("Reserved zone")} title={heldFor(lookup, view, side, row, lane)} />}
      {unit !== null && (
        <Card
          testId={testid.card(unit.instanceId)}
          card={unit}
          unit={unit}
          switchTarget
          target={{ on: "unit", instanceId: unit.instanceId, side, lane }}
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
          seat={seat.player}
          lane={lane}
          highlight={props.highlight}
          animating={props.animating}
          onClick={props.onClick}
          pops={props.pops}
        />
      )}
    </div>
  );
}
