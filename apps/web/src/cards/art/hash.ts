// The procedural card art's number helpers (docs/polish/6-cards.md, Surface A): exact integer
// arithmetic or plain rounding, never `Math.random` or the clock, so one id draws one picture everywhere.

const FNV_OFFSET_BASIS = 0x811c9dc5;
const FNV_PRIME = 0x01000193;
/** Coordinates and opacities in an `ArtSpec` are rounded to this many decimal places. */
const ART_DECIMALS = 2;
const ROUNDING = 10 ** ART_DECIMALS;

/** FNV-1a over the UTF-8 bytes of `id`, as a 32-bit unsigned integer. */
export function hashId(id: string): number {
  let hash = FNV_OFFSET_BASIS;
  const mix = (byte: number): void => {
    hash ^= byte;
    hash = Math.imul(hash, FNV_PRIME);
  };
  for (const char of id) {
    const code = char.codePointAt(0) ?? 0;
    if (code < 0x80) {
      mix(code);
    } else if (code < 0x800) {
      mix(0xc0 | (code >> 6));
      mix(0x80 | (code & 0x3f));
    } else if (code < 0x10000) {
      mix(0xe0 | (code >> 12));
      mix(0x80 | ((code >> 6) & 0x3f));
      mix(0x80 | (code & 0x3f));
    } else {
      mix(0xf0 | (code >> 18));
      mix(0x80 | ((code >> 12) & 0x3f));
      mix(0x80 | ((code >> 6) & 0x3f));
      mix(0x80 | (code & 0x3f));
    }
  }
  return hash >>> 0;
}

/** mulberry32: a 32-bit seeded generator returning floats in [0, 1), the effects layer's own. */
export { createRng as seededRandom } from "../../fx/rng.ts";

/** Rounds to `ART_DECIMALS` places and never returns `-0`, so specs compare and print cleanly. */
export function round2(value: number): number {
  const rounded = Math.round(value * ROUNDING) / ROUNDING;
  return rounded === 0 ? 0 : rounded;
}

/** Below this many hundredths a rounded value's digits are an exact integer, so `fmt` writes them itself. */
const FMT_EXACT_LIMIT = 1e15;
const DECIMAL_BASE = 10;

/**
 * A number as it is written into SVG: rounded, with no exponent and no `-0`. The same string as
 * `String(round2(value))`, written from the whole count of hundredths, which is several times faster than
 * printing a float (every picture's every coordinate passes here); a value too large for that prints as before.
 */
export function fmt(value: number): string {
  const units = Math.round(value * ROUNDING);
  if (units === 0) return "0";
  const size = units < 0 ? -units : units;
  if (!(size < FMT_EXACT_LIMIT)) return String(round2(value));
  let fraction = size % ROUNDING;
  const whole = `${units < 0 ? "-" : ""}${(size - fraction) / ROUNDING}`;
  if (fraction === 0) return whole;
  let places = ART_DECIMALS;
  while (fraction % DECIMAL_BASE === 0) {
    fraction /= DECIMAL_BASE;
    places -= 1;
  }
  return `${whole}.${String(fraction).padStart(places, "0")}`;
}
