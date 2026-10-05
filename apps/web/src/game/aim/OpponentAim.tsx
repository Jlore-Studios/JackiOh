// R660: the opponent's aim, drawn on this board — Hearthstone's targeting arrow in the opponent's
// colour, from the source the opponent is aiming with to the target it is over, gone when the aim
// ends. It draws only what the relay names, and the relay names only public handles: a hand card is
// the card back at that position, a field card its zone, a hero its portrait (`aim.ts`).
//
// While the opponent is over nothing yet, only the source is marked. Under Reduce Motion the arrow
// stands still (opponentAim.css). It never takes a hit, so the board under it stays clickable.

import { useLayoutEffect, useState, type ReactElement } from "react";

import { aimKey, type Aim, type PlayerView } from "@jackioh/shared";

import { aimEndElement } from "./aim.ts";
import "./opponentAim.css";

type Point = { x: number; y: number };
type Geometry = { from: Point; to: Point | null };

/** How far the arrow bows from a straight line, as a share of its length, and at most. */
const BEND_SHARE = 0.2;
const BEND_MAX_PX = 80;
const HEAD_PX = 24;
const HEAD_HALF_WIDTH_PX = 14;
const ORIGIN_RADIUS_PX = 9;

function centreOf(element: Element | null): Point | null {
  if (element === null) return null;
  const rect = element.getBoundingClientRect();
  return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
}

function round(n: number): number {
  return Math.round(n * 10) / 10;
}

function measure(view: PlayerView, aim: Aim): Geometry | null {
  const from = centreOf(aimEndElement(document, view, aim.source));
  if (from === null) return null;
  return { from, to: aim.target === null ? null : centreOf(aimEndElement(document, view, aim.target)) };
}

export default function OpponentAim(props: { view: PlayerView; aim: Aim | null }): ReactElement | null {
  const { view, aim } = props;
  const [geometry, setGeometry] = useState<Geometry | null>(null);

  useLayoutEffect(() => {
    if (aim === null) {
      setGeometry(null);
      return undefined;
    }
    const update = (): void => setGeometry(measure(view, aim));
    update();
    window.addEventListener("resize", update);
    window.addEventListener("scroll", update, true);
    return () => {
      window.removeEventListener("resize", update);
      window.removeEventListener("scroll", update, true);
    };
  }, [view, aim]);

  if (aim === null || geometry === null) return null;
  const { from, to } = geometry;

  return (
    <svg
      className="opponent-aim"
      data-testid="opponent-aim"
      data-aim={aimKey(aim)}
      data-targeting={to === null ? "false" : "true"}
      width="100%"
      height="100%"
      aria-hidden="true"
      focusable="false"
    >
      {to === null ? null : <Arrow from={from} to={to} />}
      <circle className="opponent-aim-origin" cx={round(from.x)} cy={round(from.y)} r={ORIGIN_RADIUS_PX} />
    </svg>
  );
}

function Arrow({ from, to }: { from: Point; to: Point }): ReactElement {
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  const length = Math.hypot(dx, dy) || 1;
  const bend = Math.min(BEND_MAX_PX, length * BEND_SHARE);
  // Bow the shaft upward on screen, whichever way it points.
  let nx = -dy / length;
  let ny = dx / length;
  if (ny > 0) {
    nx = -nx;
    ny = -ny;
  }
  const cx = (from.x + to.x) / 2 + nx * bend;
  const cy = (from.y + to.y) / 2 + ny * bend;
  const tx = to.x - cx;
  const ty = to.y - cy;
  const tangent = Math.hypot(tx, ty) || 1;
  const ux = tx / tangent;
  const uy = ty / tangent;
  const head = Math.min(HEAD_PX, length);
  const baseX = to.x - ux * head;
  const baseY = to.y - uy * head;
  const shaft = `M ${round(from.x)} ${round(from.y)} Q ${round(cx)} ${round(cy)} ${round(baseX)} ${round(baseY)}`;
  const tip = [
    `${round(to.x)},${round(to.y)}`,
    `${round(baseX - uy * HEAD_HALF_WIDTH_PX)},${round(baseY + ux * HEAD_HALF_WIDTH_PX)}`,
    `${round(baseX + uy * HEAD_HALF_WIDTH_PX)},${round(baseY - ux * HEAD_HALF_WIDTH_PX)}`,
  ].join(" ");
  return (
    <g className="opponent-aim-arrow" data-testid="opponent-aim-arrow">
      <path className="opponent-aim-shaft" d={shaft} />
      <path className="opponent-aim-pulse" d={shaft} />
      <polygon className="opponent-aim-head" points={tip} />
    </g>
  );
}
