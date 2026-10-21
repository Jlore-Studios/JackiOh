// C+ #62 KY's Papaya's curve as an equation (SPEC §8.7 row 62, R831003): the log names the curve a
// `curveDrawn` draws. Presentation only (CLAUDE.md rule 7): it reads the event's public cells and
// decides nothing.
//
// The grid is the engine's (crates/engine/src/subsystems/papaya.rs, R422), from the caster's seat:
// x the lane less one, y 0 the caster's backrow, 1 their units, 2 the opponent's units, 3 the
// opponent's backrow. The event names every cell the curve meets, the picked ones among them, so the
// lowest-degree polynomial through the named cells is the curve itself; it is worked out here by
// Lagrange interpolation in exact fractions, never floating point.
//
// It prints as "y = −¹⁄₂x² + ⁵⁄₂x": terms by falling power, a zero term left out, a coefficient of
// one left out before x, a fraction as superscript digits, a fraction slash and subscript digits,
// the minus sign (U+2212) for a negative, and "y = 0" when every term is zero.

import type { PlayerId, Row, ZoneRef } from "@jackioh/shared";

import { LANES } from "./contract.ts";

/** The grid's rows from the caster's seat, by y (R422). */
const GRID_ROWS: readonly { readonly enemy: boolean; readonly row: Row }[] = [
  { enemy: false, row: "backrow" },
  { enemy: false, row: "units" },
  { enemy: true, row: "units" },
  { enemy: true, row: "backrow" },
];

const SUPERSCRIPT_DIGITS = "⁰¹²³⁴⁵⁶⁷⁸⁹";
const SUBSCRIPT_DIGITS = "₀₁₂₃₄₅₆₇₈₉";
const MINUS = "−";
const FRACTION_SLASH = "⁄";

/** A reduced fraction: an integer over a positive integer. */
type Fraction = { num: number; den: number };

function gcd(a: number, b: number): number {
  return b === 0 ? Math.abs(a) : gcd(b, a % b);
}

function fraction(num: number, den: number): Fraction {
  if (num === 0) return { num: 0, den: 1 };
  const divisor = gcd(num, den) * Math.sign(den);
  return { num: num / divisor, den: den / divisor };
}

function sum(a: Fraction, b: Fraction): Fraction {
  return fraction(a.num * b.den + b.num * a.den, a.den * b.den);
}

/** The polynomial through the points (Lagrange), its coefficients by power, the constant first. */
function coefficients(points: readonly { x: number; y: number }[]): Fraction[] {
  const out: Fraction[] = points.map(() => fraction(0, 1));
  points.forEach((pi, i) => {
    // The basis polynomial's numerator, the product of (x − xj), and its denominator, of (xi − xj).
    let basis = [1];
    let den = 1;
    points.forEach((pj, j) => {
      if (j === i) return;
      const next = new Array<number>(basis.length + 1).fill(0);
      basis.forEach((c, k) => {
        next[k + 1] = (next[k + 1] ?? 0) + c;
        next[k] = (next[k] ?? 0) - pj.x * c;
      });
      basis = next;
      den *= pi.x - pj.x;
    });
    basis.forEach((c, k) => {
      out[k] = sum(out[k] ?? fraction(0, 1), fraction(pi.y * c, den));
    });
  });
  return out;
}

function digits(n: number, set: string): string {
  return [...String(n)].map((digit) => set[Number(digit)] ?? digit).join("");
}

/** A coefficient's size: a whole number, or a fraction as ¹⁄₂. */
function magnitude(f: Fraction): string {
  const num = Math.abs(f.num);
  return f.den === 1 ? String(num) : `${digits(num, SUPERSCRIPT_DIGITS)}${FRACTION_SLASH}${digits(f.den, SUBSCRIPT_DIGITS)}`;
}

/** One term without its sign: "¹⁄₂x²", "x", "3". */
function term(f: Fraction, power: number): string {
  if (power === 0) return magnitude(f);
  const coefficient = Math.abs(f.num) === 1 && f.den === 1 ? "" : magnitude(f);
  return `${coefficient}x${power === 1 ? "" : digits(power, SUPERSCRIPT_DIGITS)}`;
}

/** The sign before a term: "−" or nothing for the first, " − " or " + " after it. */
function signed(f: Fraction, first: boolean): string {
  if (first) return f.num < 0 ? MINUS : "";
  return f.num < 0 ? ` ${MINUS} ` : " + ";
}

function isZoneRef(value: unknown): value is ZoneRef {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return false;
  const cell = value as Record<string, unknown>;
  return (
    (cell.player === "p1" || cell.player === "p2") &&
    (cell.row === "units" || cell.row === "backrow") &&
    typeof cell.lane === "number" &&
    LANES.includes(cell.lane)
  );
}

/**
 * The equation of the curve a `curveDrawn` names (R831003), read from `caster`'s seat; null when the
 * cells make no curve (malformed, none, too many, a zone off the grid, or two in one lane).
 */
export function curveEquation(cells: unknown, caster: PlayerId): string | null {
  // Downloaded replay views are shallow-cast, so validate and bound the interpolation before doing any work.
  if (!Array.isArray(cells) || cells.length === 0 || cells.length > LANES.length || !cells.every(isZoneRef)) return null;
  const points = cells.map((cell) => ({
    x: cell.lane - 1,
    y: GRID_ROWS.findIndex((grid) => grid.enemy === (cell.player !== caster) && grid.row === cell.row),
  }));
  if (points.some((point) => point.y < 0)) return null;
  if (new Set(points.map((point) => point.x)).size !== points.length) return null;
  const terms = coefficients(points)
    .map((f, power) => ({ f, power }))
    .filter(({ f }) => f.num !== 0)
    .reverse();
  if (terms.length === 0) return "y = 0";
  return `y = ${terms.map(({ f, power }, i) => signed(f, i === 0) + term(f, power)).join("")}`;
}
