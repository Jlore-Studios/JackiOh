/**
 * A patch's version from the name an issue gave it (docs/issues-and-patches.md, Version numbers;
 * R650).
 *
 * - `vA.B.Y` is a **micro patch**: it ships as the newest version in `patches.json` (the last entry,
 *   R388's order) with the next letter after it, so a micro patch made after v0.2.5 is v0.2.5b, and
 *   one after v0.2.7c is v0.2.7d. The newest version must be an `A.B` one: a `v0.2.Y` cannot follow
 *   a v0.3.1.
 * - Any other name is the designer's and is used as given (`vA.B.X` must be replaced first).
 *
 * This is the one place a version string is read, and only here, when a patch ships (a fragment
 * keeps its `Y` until `patches ship` promotes it, R646): R105 still holds everywhere else, where
 * a version is compared for equality and never parsed or ordered.
 */

const MICRO = /^v(\d+)\.(\d+)\.Y$/;
const PLACEHOLDER = /^v\d+\.\d+\.X$/;
/** The newest version: `vA.B.C`, optionally followed by one letter (b … z). */
const SHIPPED = /^v(\d+)\.(\d+)\.(\d+)([a-z]?)$/;

export function resolveVersion(asked: string, shipped: readonly string[]): string {
  if (PLACEHOLDER.test(asked)) {
    throw new Error(`${asked}: the designer picks the X before the patch is made`);
  }
  const micro = MICRO.exec(asked);
  if (micro === null) return asked;
  const newest = shipped[shipped.length - 1];
  const parts = newest === undefined ? null : SHIPPED.exec(newest);
  if (newest === undefined || parts === null) {
    throw new Error(`${asked}: the newest version (${newest ?? "none"}) has no vA.B.C form to follow`);
  }
  if (parts[1] !== micro[1] || parts[2] !== micro[2]) {
    throw new Error(`${asked}: the newest version is ${newest}, not a v${micro[1]}.${micro[2]} one`);
  }
  const letter = parts[4] ?? "";
  if (letter === "z") throw new Error(`${asked}: ${newest} has no letter left after z`);
  const next = letter === "" ? "b" : String.fromCharCode(letter.charCodeAt(0) + 1);
  return `v${parts[1]}.${parts[2]}.${parts[3]}${next}`;
}
