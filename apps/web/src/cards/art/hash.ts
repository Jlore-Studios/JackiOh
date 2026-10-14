// The deterministic number helpers behind the procedural card art (docs/polish/6-cards.md,
// Surface A). The art is a pure function of the card id, so everything here is exact integer
// arithmetic or plain rounding: one id draws the same picture on every client, in every test and
// after every reload. Nothing here reads `Math.random` or the clock.

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

/** A number as it is written into SVG: rounded, with no exponent and no `-0`. */
export function fmt(value: number): string {
  return String(round2(value));
}
