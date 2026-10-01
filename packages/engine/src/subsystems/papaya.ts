// Classic+ #62 KY's Papaya's curve targeting (SPEC §8.7 row 62, R422; docs/classic-sets.md B5 E32).
//
// The grid is the board's 20 cells from the caster's seat: x the lane less one on both sides (§3.1),
// y 0 the caster's backrow, 1 their units, 2 the opponent's units, 3 the opponent's backrow.
//
// The player picks 1 to `PAPAYA_MAX_CELLS` cells in different lanes; the curve is the lowest-degree
// polynomial through them, which Lagrange interpolation is (degree below the number of points, so
// three collinear cells give their line). It is evaluated in exact rationals — integer numerator over
// positive integer denominator, reduced by their gcd — so "y = p(x) exactly" is an integer test. The
// curve is read at every lane: a cell is on it when p(x) is a whole row 0 to 3, so one cell's
// constant crosses its row, a line may meet a third cell, a cubic may hit the fifth lane, and between
// lanes it touches nothing (R422).
//
// The cells are asked one `cell` prompt at a time (E18, `chooseCell`): every cell of the unused lanes
// and, after the first, "done" — at most 21 answers, so `legalActions`, the fuzz suite and the AI
// reach every curve. The cells so far ride the prompt's resume data as plain points (R113). Options
// are cells, never cards (R177). Then the board is read once and the top of the pile at each cell on
// the curve is exiled — face-down cards too (openly, once in exile, R97), tokens ceasing to exist
// (R11), a dormant card beneath resuming and not exiled again (§3.2, R13); the Radiant face only the
// enemy's rows 2 and 3.
//
// A card script: `cry: () => papayaBegin()`, `resume: { [PAPAYA_STEP]: papayaAnswered }`.

import type { PlayerId, Row, ZoneRef } from "@jackioh/shared";
import { opponentOf } from "@jackioh/shared";
import { PAPAYA_MAX_CELLS, PAPAYA_ROWS, UNIT_ZONES } from "../config";
import { chooseCell, chosenCells, exile } from "../effects";
import type { Effect, EffectContext } from "../script";
import { zoneContents, type ZoneSlot } from "../zones";

/** R422: a cell of the grid from the caster's seat — x the lane less one, y the row. */
export type PapayaPoint = { readonly x: number; readonly y: number };

/** R422: the lanes the curve is read at (§3.1: both rows have as many). */
export const PAPAYA_LANES = UNIT_ZONES;

/** R422: each row number, from the caster's hero outward. */
const GRID_ROWS: readonly { readonly enemy: boolean; readonly row: Row }[] = [
  { enemy: false, row: "backrow" },
  { enemy: false, row: "units" },
  { enemy: true, row: "units" },
  { enemy: true, row: "backrow" },
];

function isCell(point: PapayaPoint): boolean {
  const { x, y } = point;
  return Number.isInteger(x) && Number.isInteger(y) && x >= 0 && x < PAPAYA_LANES && y >= 0 && y < PAPAYA_ROWS;
}

/** R422: the zone a cell is, from `caster`'s seat. */
export function zoneOfPoint(caster: PlayerId, point: PapayaPoint): ZoneSlot {
  const grid = GRID_ROWS[point.y];
  if (grid === undefined || !isCell(point)) throw new Error(`R422: (${point.x}, ${point.y}) is not a cell of the grid`);
  return { player: grid.enemy ? opponentOf(caster) : caster, row: grid.row, lane: point.x + 1 };
}

/** R422: the cell a zone is, from `caster`'s seat. */
export function pointOfZone(caster: PlayerId, zone: ZoneRef): PapayaPoint {
  const enemy = zone.player !== caster;
  return { x: zone.lane - 1, y: GRID_ROWS.findIndex((grid) => grid.enemy === enemy && grid.row === zone.row) };
}

/** A reduced fraction: integer numerator over a positive integer denominator. */
export type Rational = { readonly num: number; readonly den: number };

function gcd(a: number, b: number): number {
  return b === 0 ? Math.abs(a) : gcd(b, a % b);
}

function rational(num: number, den: number): Rational {
  if (num === 0) return { num: 0, den: 1 };
  const divisor = gcd(num, den) * Math.sign(den);
  return { num: num / divisor, den: den / divisor };
}

/** R422: the curve through the cells (Lagrange), read at x exactly. */
export function curveAt(points: readonly PapayaPoint[], x: number): Rational {
  const lanes = new Set(points.map((point) => point.x));
  if (points.length === 0 || points.length > PAPAYA_MAX_CELLS || lanes.size !== points.length || !points.every(isCell)) {
    throw new Error(`R422: a curve goes through 1 to ${PAPAYA_MAX_CELLS} grid cells in different lanes`);
  }
  let sum: Rational = { num: 0, den: 1 };
  for (const pi of points) {
    let term: Rational = { num: pi.y, den: 1 };
    for (const pj of points) {
      if (pj !== pi) term = rational(term.num * (x - pj.x), term.den * (pi.x - pj.x));
    }
    sum = rational(sum.num * term.den + term.num * sum.den, sum.den * term.den);
  }
  return sum;
}

/** R422: every cell on the curve, one per lane at most, in lane order. */
export function cellsOnCurve(points: readonly PapayaPoint[]): PapayaPoint[] {
  return Array.from({ length: PAPAYA_LANES }, (_, x) => ({ x, value: curveAt(points, x) }))
    .filter(({ value }) => value.den === 1 && value.num >= 0 && value.num < PAPAYA_ROWS)
    .map(({ x, value }) => ({ x, y: value.num }));
}

/** R422: the ids of the cards the curve exiles — the top of each pile on it, the enemy's rows only when `enemyOnly`. */
export function cardsOnCurve(
  ctx: Pick<EffectContext, "state" | "controller">,
  points: readonly PapayaPoint[],
  enemyOnly: boolean,
): string[] {
  if (points.length === 0) return [];
  return cellsOnCurve(points).flatMap((cell) => {
    if (enemyOnly && GRID_ROWS[cell.y]?.enemy !== true) return [];
    const top = zoneContents(ctx.state, zoneOfPoint(ctx.controller, cell))[0];
    return top === undefined ? [] : [top.id];
  });
}

/** R422: the resume step each cell prompt's answer re-enters. */
export const PAPAYA_STEP = "papayaCell";
/** R113: where the cells picked so far ride the resume data. */
export const PAPAYA_POINTS_KEY = "papayaPoints";

function askCell(points: readonly PapayaPoint[]): Effect {
  return chooseCell({
    step: PAPAYA_STEP,
    done: points.length > 0,
    cells: { exceptLanes: points.map((point) => point.x + 1) },
    prompt: points.length === 0 ? "KY's Papaya: choose a cell for the curve" : "KY's Papaya: choose another cell, or done",
    data: { [PAPAYA_POINTS_KEY]: points },
  });
}

/** R422: the card's on-resolve hook — the first cell prompt. */
export function papayaBegin(): Effect[] {
  return [askCell([])];
}

/** R422: an answer — the next prompt while cells are being added, else the curve's exiles. */
export function papayaAnswered(ctx: EffectContext): Effect[] {
  const held = (ctx.data[PAPAYA_POINTS_KEY] ?? []) as PapayaPoint[];
  const picked = chosenCells(ctx).map((zone) => pointOfZone(ctx.controller, zone));
  const points = [...held, ...picked];
  if (picked.length > 0 && points.length < PAPAYA_MAX_CELLS) return [askCell(points)];
  return cardsOnCurve(ctx, points, ctx.radiant).map((instanceId) => exile({ target: { of: "instance", instanceId } }));
}
